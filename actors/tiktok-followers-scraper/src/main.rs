use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::{json, Map, Number, Value};
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    input_key: String,
    actor_run_id: String,
    apify_token: String,
    scrappa_api_key: String,
    scrappa_request_timeout: Duration,
}

impl ActorConfig {
    fn from_env(scrappa_api_key: String) -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
            scrappa_request_timeout: SCRAPPA_REQUEST_TIMEOUT,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.trim().is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn scrappa_api_key(value: Option<&str>) -> Result<String> {
    value
        .filter(|key| !key.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            anyhow!(
                "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."
            )
        })
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.set_query(None);
    url.set_fragment(None);
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments.iter().copied());
    Ok(url)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = response.text().await.unwrap_or_default();
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "{operation} failed with {} {reason}{detail}",
            status.as_u16()
        );
    }

    response
        .json()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

async fn get_input(client: &Client, config: &ActorConfig) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify INPUT request failed")?;
    response_json(response, "Apify INPUT request").await
}

async fn get_run(client: &Client, config: &ActorConfig) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify run pricing request failed")?;
    response_json(response, "Apify run pricing request").await
}

fn js_number_string(number: &Number) -> String {
    if let Some(value) = number.as_f64() {
        if value.is_finite() && value.fract() == 0.0 && value >= 0.0 && value <= u64::MAX as f64 {
            return format!("{value:.0}");
        }
    }
    number.to_string()
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => js_number_string(value),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    js_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn js_typeof(value: &Value) -> &'static str {
    match value {
        Value::Null | Value::Array(_) | Value::Object(_) => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
    }
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TikTokLookup {
    UniqueId(String),
    UserId(String),
}

impl TikTokLookup {
    fn log_value(&self) -> String {
        match self {
            Self::UniqueId(value) => value.clone(),
            Self::UserId(value) => format!("user_id:{value}"),
        }
    }

    fn unique_id(&self) -> Option<&str> {
        match self {
            Self::UniqueId(value) => Some(value),
            Self::UserId(_) => None,
        }
    }

    fn user_id(&self) -> Option<&str> {
        match self {
            Self::UserId(value) => Some(value),
            Self::UniqueId(_) => None,
        }
    }
}

#[derive(Debug)]
struct TikTokFollowersParams {
    lookup: TikTokLookup,
    count: Option<String>,
    time: Option<String>,
}

fn normalize_tiktok_user_id(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("TikTok user_id must contain digits only");
    }
    if value.len() > 30 {
        bail!("TikTok numeric user ID must be 30 digits or fewer");
    }
    Ok(value.to_owned())
}

fn normalize_tiktok_username(value: &str) -> Result<String> {
    let username = value.strip_prefix('@').unwrap_or(value);
    let valid = (2..=255).contains(&username.len())
        && username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'_');
    if !valid {
        bail!(
            "TikTok username must be 2 to 255 characters and contain only letters, numbers, dots, or underscores"
        );
    }
    Ok(format!("@{username}"))
}

fn looks_like_tiktok_url(value: &str) -> bool {
    let has_scheme = value.find("://").is_some_and(|scheme_end| {
        let scheme = &value[..scheme_end];
        let mut bytes = scheme.bytes();
        bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
            && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'))
    });

    let lowercased = value.to_ascii_lowercase();
    let contains_tiktok_host = lowercased.match_indices("tiktok.com").any(|(index, _)| {
        let before = index == 0 || lowercased.as_bytes().get(index - 1) == Some(&b'.');
        let after_index = index + "tiktok.com".len();
        let after = after_index == lowercased.len()
            || lowercased.as_bytes().get(after_index) == Some(&b'/');
        before && after
    });

    has_scheme || value.starts_with("//") || contains_tiktok_host
}

fn normalize_tiktok_unique_id(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    if looks_like_tiktok_url(trimmed) {
        let parsed = Url::parse(trimmed);
        let Ok(parsed) = parsed else {
            bail!("A valid TikTok profile URL or username is required");
        };

        let hostname = parsed.host_str().unwrap_or_default();
        if !(hostname.eq_ignore_ascii_case("tiktok.com")
            || hostname.to_ascii_lowercase().ends_with(".tiktok.com"))
        {
            bail!("TikTok profile URL must be on tiktok.com");
        }
        if parsed.scheme() != "https" {
            bail!("TikTok profile URL must use HTTPS");
        }

        let path = parsed.path();
        let username = path
            .strip_prefix('/')
            .and_then(|path| path.strip_suffix('/').or(Some(path)))
            .filter(|path| path.starts_with('@'))
            .filter(|path| !path[1..].is_empty() && !path[1..].contains('/'))
            .ok_or_else(|| {
                anyhow!("TikTok profile URL must use the format https://www.tiktok.com/@username")
            })?;
        return normalize_tiktok_username(username);
    }

    normalize_tiktok_username(trimmed)
}

fn nonempty_nonnull(value: Option<&Value>) -> bool {
    value.is_some_and(|value| !value.is_null() && value != &Value::String(String::new()))
}

fn resolve_tiktok_lookup(
    input: &Value,
    warn: &mut impl FnMut(String),
) -> Result<Option<TikTokLookup>> {
    let profile = input.get("profile");
    if let Some(Value::String(profile)) = profile {
        let profile = profile.trim();
        if !profile.is_empty() {
            if profile.bytes().all(|byte| byte.is_ascii_digit()) {
                return normalize_tiktok_user_id(profile)
                    .map(TikTokLookup::UserId)
                    .map(Some);
            }
            return normalize_tiktok_unique_id(profile)
                .map(TikTokLookup::UniqueId)
                .map(Some);
        }
    } else if nonempty_nonnull(profile) {
        warn(format!(
            "profile must be a string, got {}.",
            js_typeof(profile.unwrap())
        ));
    }

    let unique_id = input.get("unique_id");
    if let Some(Value::String(unique_id)) = unique_id {
        let unique_id = normalize_tiktok_unique_id(unique_id)?;
        if !unique_id.is_empty() {
            return Ok(Some(TikTokLookup::UniqueId(unique_id)));
        }
    } else if nonempty_nonnull(unique_id) {
        warn(format!(
            "unique_id must be a string, got {}.",
            js_typeof(unique_id.unwrap())
        ));
    }

    let user_id = input.get("user_id");
    if let Some(Value::String(user_id)) = user_id {
        let user_id = normalize_tiktok_user_id(user_id)?;
        if !user_id.is_empty() {
            return Ok(Some(TikTokLookup::UserId(user_id)));
        }
    } else if nonempty_nonnull(user_id) {
        warn(format!(
            "user_id must be a string, got {}.",
            js_typeof(user_id.unwrap())
        ));
    }

    Ok(None)
}

fn integer_query_value(value: &Value, minimum: f64, maximum: Option<f64>) -> Option<String> {
    let number = value.as_number()?.as_f64()?;
    if !number.is_finite() || number.fract() != 0.0 || number < minimum {
        return None;
    }
    if maximum.is_some_and(|maximum| number > maximum) {
        return None;
    }
    Some(format!("{number:.0}"))
}

fn build_tiktok_followers_params(
    input: &Value,
    warn: &mut impl FnMut(String),
) -> Result<TikTokFollowersParams> {
    let lookup = resolve_tiktok_lookup(input, warn)?;

    let count = input.get("count").and_then(|count| {
        integer_query_value(count, 1.0, Some(50.0)).or_else(|| {
            warn(format!(
                "count must be an integer between 1 and 50, got {}. Using Scrappa default.",
                js_string(count)
            ));
            None
        })
    });

    let pagination_field = if input.get("time").is_some() {
        "time"
    } else {
        "cursor"
    };
    let pagination_value = input
        .get("time")
        .filter(|value| !value.is_null())
        .or_else(|| input.get("cursor"));
    let time = match pagination_value {
        Some(Value::Number(_)) => integer_query_value(
            pagination_value.unwrap(),
            0.0,
            None,
        )
        .or_else(|| {
            warn(format!(
                "{pagination_field} must be a non-negative integer, got {}. Starting from the first page.",
                js_string(pagination_value.unwrap())
            ));
            None
        }),
        Some(Value::String(value)) if !value.trim().is_empty() => {
            let value = value.trim();
            if value.bytes().all(|byte| byte.is_ascii_digit()) {
                Some(value.to_owned())
            } else {
                warn(format!(
                    "{pagination_field} must contain digits only. Starting from the first page."
                ));
                None
            }
        }
        Some(value) if !value.is_null() && value != &Value::String(String::new()) => {
            warn(format!(
                "{pagination_field} must be a non-negative integer or digit string, got {}. Starting from the first page.",
                js_typeof(value)
            ));
            None
        }
        _ => None,
    };

    let lookup = lookup.ok_or_else(|| anyhow!("TikTok unique_id or user_id is required"))?;
    Ok(TikTokFollowersParams {
        lookup,
        count,
        time,
    })
}

fn check_scrappa_code(response: &Value, api_name: &str) -> Result<()> {
    if let Some(code) = response.get("code") {
        if code.as_f64() != Some(0.0) {
            let message = response
                .get("msg")
                .filter(|message| !message.is_null())
                .map(js_string)
                .unwrap_or_else(|| "Unknown error".to_owned());
            bail!(
                "Scrappa TikTok {api_name} API returned code {}: {message}",
                js_string(code)
            );
        }
    }
    Ok(())
}

fn request_timeout_message(timeout: Duration) -> String {
    format!(
        "Scrappa API request timed out after {}ms",
        timeout.as_millis()
    )
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn joined_error_messages(value: &Value) -> Option<String> {
    let messages = value.as_array()?;
    Some(
        messages
            .iter()
            .map(|message| {
                if message.is_null() {
                    String::new()
                } else {
                    js_string(message)
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
    )
}

fn format_scrappa_error_body(body: &str, fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        if !error_data.is_null() {
            let mut message = error_data
                .get("message")
                .filter(|message| !message.is_null())
                .map(js_string)
                .unwrap_or_else(|| fallback.to_owned());

            if let Some(errors) = error_data.get("errors").filter(|errors| js_truthy(errors)) {
                let details = match errors {
                    Value::Object(errors) => errors
                        .iter()
                        .filter_map(|(field, messages)| {
                            joined_error_messages(messages)
                                .map(|messages| format!("{field}: {messages}"))
                        })
                        .collect::<Vec<_>>(),
                    Value::Array(errors) => errors
                        .iter()
                        .enumerate()
                        .filter_map(|(index, messages)| {
                            joined_error_messages(messages)
                                .map(|messages| format!("{index}: {messages}"))
                        })
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                };
                if !details.is_empty() {
                    message.push_str(" - ");
                    message.push_str(&details.join("; "));
                }
            }
            return message;
        }
    }

    collapse_whitespace(body).chars().take(500).collect()
}

async fn get_scrappa_json(
    client: &Client,
    config: &ActorConfig,
    endpoint: &[&str],
    params: &[(&str, String)],
) -> Result<Value> {
    let mut url = endpoint_url(&config.scrappa_api_base_url, endpoint)?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            if !value.is_empty() {
                query.append_pair(key, value);
            }
        }
    }

    let response = client
        .get(url)
        .timeout(config.scrappa_request_timeout)
        .header("X-API-Key", &config.scrappa_api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "{}",
                    request_timeout_message(config.scrappa_request_timeout)
                )
            } else {
                anyhow!("Scrappa API request failed: {error}")
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        let fallback = status
            .canonical_reason()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
        let body = response.text().await.map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "{}",
                    request_timeout_message(config.scrappa_request_timeout)
                )
            } else {
                anyhow!(fallback.clone())
            }
        })?;
        bail!(
            "Scrappa API error ({}): {}",
            status.as_u16(),
            format_scrappa_error_body(&body, &fallback)
        );
    }

    let body = response.text().await.map_err(|error| {
        if error.is_timeout() {
            anyhow!(
                "{}",
                request_timeout_message(config.scrappa_request_timeout)
            )
        } else {
            anyhow!("Scrappa API response could not be read: {error}")
        }
    })?;
    serde_json::from_str(&body).context("Scrappa API response was not valid JSON")
}

async fn resolve_tiktok_user_id(
    client: &Client,
    config: &ActorConfig,
    params: &TikTokFollowersParams,
) -> Result<String> {
    if let Some(user_id) = params.lookup.user_id() {
        return Ok(user_id.to_owned());
    }

    let unique_id = params
        .lookup
        .unique_id()
        .ok_or_else(|| anyhow!("TikTok unique_id or user_id is required"))?;
    println!("Resolving TikTok user_id for {unique_id}");

    let response = get_scrappa_json(
        client,
        config,
        &["tiktok", "user", "profile"],
        &[("unique_id", unique_id.to_owned())],
    )
    .await?;
    check_scrappa_code(&response, "Profile")?;

    let data = response.get("data");
    let profile = match data {
        Some(Value::Array(profiles)) => profiles.first(),
        Some(value) => Some(value),
        None => None,
    };
    let user_id = profile
        .and_then(|profile| {
            profile
                .get("user_id")
                .filter(|user_id| !user_id.is_null())
                .or_else(|| {
                    profile
                        .get("user")
                        .and_then(|user| user.get("id"))
                        .filter(|user_id| !user_id.is_null())
                })
        })
        .and_then(|user_id| match user_id {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(js_number_string(value)),
            _ => None,
        })
        .map(|user_id| user_id.trim().to_owned())
        .filter(|user_id| !user_id.is_empty())
        .ok_or_else(|| anyhow!("Could not resolve TikTok user_id for {unique_id}"))?;

    Ok(user_id)
}

fn extract_followers(data: Option<&Value>) -> Vec<Value> {
    let Some(data) = data.filter(|data| !data.is_null()) else {
        return Vec::new();
    };

    if let Value::Array(followers) = data {
        return followers.clone();
    }

    for field in ["followers", "users", "user_list"] {
        if let Some(Value::Array(followers)) = data.get(field) {
            return followers.clone();
        }
    }
    Vec::new()
}

fn extract_pagination(data: Option<&Value>) -> (bool, Value) {
    let Some(data) = data.filter(|data| !data.is_null() && !data.is_array()) else {
        return (false, Value::Null);
    };

    let has_more = data
        .get("hasMore")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("has_more"))
        .is_some_and(js_truthy);
    let next_time = data
        .get("time")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("min_time").filter(|value| !value.is_null()))
        .or_else(|| data.get("max_time").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null);
    (has_more, next_time)
}

fn follower_item(follower: &Value, lookup_unique_id: Option<&str>, lookup_user_id: &str) -> Value {
    let mut item = match follower {
        Value::Object(follower) => follower.clone(),
        Value::Array(follower) => follower
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        Value::String(follower) => follower
            .chars()
            .enumerate()
            .map(|(index, value)| (index.to_string(), Value::String(value.to_string())))
            .collect(),
        _ => Map::new(),
    };
    item.insert(
        "lookup_unique_id".to_owned(),
        lookup_unique_id
            .map(|value| Value::String(value.to_owned()))
            .unwrap_or(Value::Null),
    );
    item.insert(
        "lookup_user_id".to_owned(),
        Value::String(lookup_user_id.to_owned()),
    );
    Value::Object(item)
}

fn affordable_dataset_items(run: &Value, requested: usize) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        bail!("Apify run is not configured for pay-per-event pricing");
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = events
        .get(APIFY_DEFAULT_DATASET_ITEM_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
    if !item_price.is_finite() || item_price < 0.0 || !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    for (event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if count == 0 {
            continue;
        }
        let price = events
            .get(event_name)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .count())
}

async fn run_dataset_capacity(
    client: &Client,
    config: &ActorConfig,
    requested: usize,
) -> Result<usize> {
    let run = get_run(client, config).await?;
    affordable_dataset_items(&run, requested)
}

async fn push_dataset_data(client: &Client, config: &ActorConfig, items: &[Value]) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(items)
        .send()
        .await
        .context("Apify dataset write failed")?;
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = response.text().await.unwrap_or_default();
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "Apify dataset write failed with {} {reason}{detail}",
            status.as_u16()
        );
    }
    Ok(())
}

async fn store_output(client: &Client, config: &ActorConfig, output: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            "OUTPUT",
        ],
    )?;
    let response = client
        .put(url)
        .bearer_auth(&config.apify_token)
        .json(output)
        .send()
        .await
        .context("Apify OUTPUT storage write failed")?;
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = response.text().await.unwrap_or_default();
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "Apify OUTPUT storage write failed with {} {reason}{detail}",
            status.as_u16()
        );
    }
    Ok(())
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    if !js_truthy(&input) {
        bail!("TikTok unique_id or user_id is required");
    }

    let params = build_tiktok_followers_params(&input, &mut |warning| eprintln!("{warning}"))?;
    println!(
        "Fetching TikTok followers for: {}",
        params.lookup.log_value()
    );

    let lookup_user_id = resolve_tiktok_user_id(client, config, &params).await?;
    let mut request_params = vec![("user_id", lookup_user_id.clone())];
    if let Some(count) = &params.count {
        request_params.push(("count", count.clone()));
    }
    if let Some(time) = &params.time {
        request_params.push(("time", time.clone()));
    }

    let response = get_scrappa_json(
        client,
        config,
        &["tiktok", "user", "followers"],
        &request_params,
    )
    .await?;
    check_scrappa_code(&response, "Followers")?;

    let data = response.get("data");
    let followers = extract_followers(data);
    let (has_next_page, next_time) = extract_pagination(data);
    if followers.is_empty() {
        println!("No followers found for the given TikTok lookup");
    } else {
        println!("Found {} followers", followers.len());
        let max_saved_rows = run_dataset_capacity(client, config, followers.len()).await?;
        if max_saved_rows < followers.len() {
            println!(
                "PPE spending limit allows saving {max_saved_rows} of {} follower rows",
                followers.len()
            );
        }
        let items = followers
            .iter()
            .take(max_saved_rows)
            .map(|follower| follower_item(follower, params.lookup.unique_id(), &lookup_user_id))
            .collect::<Vec<_>>();
        if !items.is_empty() {
            push_dataset_data(client, config, &items).await?;
        }
    }

    store_output(client, config, &response).await?;
    let summary = json!({
        "followers_extracted": followers.len(),
        "has_next_page": has_next_page,
        "next_time": next_time,
        "processed_time": response.get("processed_time").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
    });
    println!("TikTok followers extraction completed successfully");
    println!("Results summary: {}", summary);
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let api_key = scrappa_api_key(env::var("SCRAPPA_API_KEY").ok().as_deref())?;
    let config = ActorConfig::from_env(api_key)?;
    let client = Client::builder()
        .build()
        .context("Could not create HTTP client")?;
    run_actor(&client, &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver},
            Arc,
        },
        thread::{self, JoinHandle},
    };

    struct MockResponse {
        status: u16,
        body: String,
        delay: Duration,
    }

    struct MockServer {
        base_url: Url,
        requests: Receiver<String>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = std::time::Instant::now() + Duration::from_secs(10);
                for response in responses {
                    let (mut stream, _) = loop {
                        if stopped.load(Ordering::Relaxed) || std::time::Instant::now() >= deadline
                        {
                            return;
                        }
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(_) => return,
                        }
                    };
                    let request = read_request(&mut stream).unwrap_or_default();
                    if sender.send(request).is_err() {
                        return;
                    }
                    thread::sleep(response.delay);
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
                        504 => "Gateway Timeout",
                        _ => "Mock Response",
                    };
                    let message = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    if stream.write_all(message.as_bytes()).is_err() {
                        return;
                    }
                }
            });
            Self {
                base_url: Url::parse(&format!("http://{address}")).unwrap(),
                requests,
                stop,
                thread: Some(thread),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.try_iter().collect()
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
            delay: Duration::ZERO,
        }
    }

    fn json_response(status: u16, body: &Value) -> MockResponse {
        response(status, &body.to_string())
    }

    fn test_config(apify_api_base_url: &Url, scrappa_api_base_url: &Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: apify_api_base_url.clone(),
            scrappa_api_base_url: scrappa_api_base_url.clone(),
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            actor_run_id: "test-run".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
            scrappa_request_timeout: SCRAPPA_REQUEST_TIMEOUT,
        }
    }

    fn run_pricing(max_charge: f64, item_price: f64, charged_counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": item_price},
                            "other-event": {"eventPriceUsd": 0.0001}
                        }
                    }
                },
                "chargedEventCounts": charged_counts,
                "options": {"maxTotalChargeUsd": max_charge}
            }
        })
    }

    fn request_parts(request: &str) -> (&str, &str, &str) {
        let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
        let mut parts = headers
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace();
        (
            parts.next().unwrap_or_default(),
            parts.next().unwrap_or_default(),
            body,
        )
    }

    fn has_header(request: &str, name: &str, expected: &str) -> bool {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .skip(1)
            .any(|line| {
                line.split_once(':').is_some_and(|(key, value)| {
                    key.eq_ignore_ascii_case(name) && value.trim().eq_ignore_ascii_case(expected)
                })
            })
    }

    fn query_pairs(request_path: &str) -> Vec<(String, String)> {
        let url = Url::parse(&format!("http://127.0.0.1{request_path}")).unwrap();
        url.query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect()
    }

    #[test]
    fn builds_username_url_and_numeric_lookup_params() {
        let mut warnings = Vec::new();
        let params = build_tiktok_followers_params(
            &json!({"profile":"@tiktok","count":10,"time":" 0 "}),
            &mut |warning| warnings.push(warning),
        )
        .unwrap();
        assert_eq!(params.lookup, TikTokLookup::UniqueId("@tiktok".to_owned()));
        assert_eq!(params.count.as_deref(), Some("10"));
        assert_eq!(params.time.as_deref(), Some("0"));
        assert!(warnings.is_empty());

        let url = normalize_tiktok_unique_id("https://www.tiktok.com/@tiktok?lang=en").unwrap();
        assert_eq!(url, "@tiktok");

        let numeric =
            build_tiktok_followers_params(&json!({"profile":"107955"}), &mut |_| {}).unwrap();
        assert_eq!(numeric.lookup, TikTokLookup::UserId("107955".to_owned()));

        let explicit = build_tiktok_followers_params(
            &json!({"unique_id":"tiktok","user_id":"abc"}),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(
            explicit.lookup,
            TikTokLookup::UniqueId("@tiktok".to_owned())
        );

        let by_id =
            build_tiktok_followers_params(&json!({"user_id":" 107955 ","count":25}), &mut |_| {})
                .unwrap();
        assert_eq!(by_id.lookup, TikTokLookup::UserId("107955".to_owned()));
    }

    #[test]
    fn preserves_cursor_alias_and_omits_invalid_optional_values() {
        let mut warnings = Vec::new();
        let params = build_tiktok_followers_params(
            &json!({"profile":"@tiktok","count":0,"time":"abc","cursor":"123"}),
            &mut |warning| warnings.push(warning),
        )
        .unwrap();
        assert_eq!(params.lookup, TikTokLookup::UniqueId("@tiktok".to_owned()));
        assert_eq!(params.count, None);
        assert_eq!(params.time, None);
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("count must be an integer between 1 and 50")));
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("time must contain digits only")));

        let cursor = build_tiktok_followers_params(
            &json!({"profile":"@tiktok","cursor":"123"}),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(cursor.time.as_deref(), Some("123"));

        let empty =
            build_tiktok_followers_params(&json!({"profile":"@tiktok","time":"   "}), &mut |_| {})
                .unwrap();
        assert_eq!(empty.time, None);

        assert!(
            build_tiktok_followers_params(&json!({"profile":" "}), &mut |_| {})
                .unwrap_err()
                .to_string()
                .contains("TikTok unique_id or user_id is required")
        );
    }

    #[test]
    fn rejects_invalid_tiktok_urls_usernames_and_ids() {
        assert!(normalize_tiktok_unique_id("https://example.com/@tiktok")
            .unwrap_err()
            .to_string()
            .contains("must be on tiktok.com"));
        assert!(normalize_tiktok_unique_id("http://www.tiktok.com/@tiktok")
            .unwrap_err()
            .to_string()
            .contains("must use HTTPS"));
        assert!(normalize_tiktok_unique_id("https://www.tiktok.com/@")
            .unwrap_err()
            .to_string()
            .contains("must use the format https://www.tiktok.com/@username"));
        assert!(normalize_tiktok_unique_id("@tik-tok")
            .unwrap_err()
            .to_string()
            .contains("TikTok username must be"));
        assert!(normalize_tiktok_user_id("1234567890123456789012345678901")
            .unwrap_err()
            .to_string()
            .contains("30 digits or fewer"));
        assert!(normalize_tiktok_user_id("10x")
            .unwrap_err()
            .to_string()
            .contains("digits only"));
    }

    #[test]
    fn extracts_follower_arrays_and_pagination_fallbacks() {
        let followers = vec![json!({"user_id":"1"})];
        assert_eq!(extract_followers(Some(&json!(followers))), followers);
        assert_eq!(
            extract_followers(Some(&json!({"followers":[{"user_id":"2"}]}))),
            vec![json!({"user_id":"2"})]
        );
        assert_eq!(
            extract_followers(Some(&json!({"users":[{"user_id":"3"}]}))),
            vec![json!({"user_id":"3"})]
        );
        assert_eq!(
            extract_followers(Some(&json!({"user_list":[{"user_id":"4"}]}))),
            vec![json!({"user_id":"4"})]
        );
        assert!(extract_followers(Some(&Value::Null)).is_empty());

        assert_eq!(
            extract_pagination(Some(&json!({"hasMore":true,"time":1711111111}))),
            (true, json!(1711111111))
        );
        assert_eq!(
            extract_pagination(Some(&json!({"has_more":false,"time":"0"}))),
            (false, json!("0"))
        );
        assert_eq!(
            extract_pagination(Some(
                &json!({"has_more":true,"min_time":"1","max_time":"2"})
            )),
            (true, json!("1"))
        );
        assert_eq!(
            extract_pagination(Some(&json!({"hasMore":true,"max_time":2}))),
            (true, json!(2))
        );
        assert_eq!(extract_pagination(Some(&json!([]))), (false, Value::Null));
    }

    #[test]
    fn adds_lookup_fields_with_javascript_object_spread_behavior() {
        assert_eq!(
            follower_item(&json!({"user_id":"1"}), Some("@tiktok"), "107955"),
            json!({
                "user_id":"1",
                "lookup_unique_id":"@tiktok",
                "lookup_user_id":"107955"
            })
        );
        assert_eq!(
            follower_item(&json!({"user_id":"1"}), None, "107955")["lookup_unique_id"],
            Value::Null
        );
        assert_eq!(
            follower_item(&json!("ab"), None, "107955"),
            json!({"0":"a","1":"b","lookup_unique_id":null,"lookup_user_id":"107955"})
        );
    }

    #[test]
    fn calculates_ppe_capacity_from_remaining_run_budget() {
        let run = run_pricing(0.0002, 0.0001, json!({"other-event":1}));
        assert_eq!(affordable_dataset_items(&run, 5).unwrap(), 1);

        let no_budget = run_pricing(0.0, 0.0001, json!({}));
        assert_eq!(affordable_dataset_items(&no_budget, 5).unwrap(), 0);

        let no_charge = run_pricing(0.0, 0.0, json!({}));
        assert_eq!(affordable_dataset_items(&no_charge, 5).unwrap(), 5);

        let invalid = json!({"data":{"pricingInfo":{"pricingModel":"PAY_PER_RESULT"}}});
        assert!(affordable_dataset_items(&invalid, 1)
            .unwrap_err()
            .to_string()
            .contains("not configured for pay-per-event pricing"));
    }

    #[test]
    fn actor_schema_keeps_prefill_and_input_defaults() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["properties"]["profile"]["prefill"], "@tiktok");
        assert_eq!(schema["properties"]["count"]["default"], 10);
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("profile")));
    }

    #[tokio::test]
    async fn resolves_tiktok_user_id_and_runs_authenticated_budgeted_output_flow() {
        let followers_response = json!({
            "code": 0,
            "data": {
                "followers": [
                    {"user_id":"1","unique_id":"first","nickname":"First"},
                    {"user_id":"2","unique_id":"second","nickname":"Second"}
                ],
                "hasMore": true,
                "time": "1711111111"
            },
            "processed_time": 0.5
        });
        let input = json!({"profile":"@tiktok","count":2,"cursor":"9"}).to_string();
        let pricing = run_pricing(0.0002, 0.0001, json!({"other-event":1}));
        let server = MockServer::start(vec![
            response(200, &input),
            response(200, r#"{"code":0,"data":{"user_id":"107955"}}"#),
            json_response(200, &followers_response),
            json_response(200, &pricing),
            response(201, ""),
            response(201, ""),
        ]);
        let mut scrappa_base_url = server.base_url.clone();
        scrappa_base_url.set_path("/api");
        let config = test_config(&server.base_url, &scrappa_base_url);

        run_actor(&Client::builder().build().unwrap(), &config)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 6);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert!(has_header(
            &requests[0],
            "authorization",
            "Bearer test-token-not-a-real-credential"
        ));
        assert_eq!(
            request_parts(&requests[1]).1,
            "/api/tiktok/user/profile?unique_id=%40tiktok"
        );
        assert!(has_header(&requests[1], "x-api-key", "test-scrappa-key"));
        assert!(has_header(&requests[1], "accept", "application/json"));
        let query = query_pairs(request_parts(&requests[2]).1);
        assert_eq!(
            query,
            vec![
                ("user_id".to_owned(), "107955".to_owned()),
                ("count".to_owned(), "2".to_owned()),
                ("time".to_owned(), "9".to_owned())
            ]
        );

        let (method, path, body) = request_parts(&requests[4]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{
                "user_id":"1",
                "unique_id":"first",
                "nickname":"First",
                "lookup_unique_id":"@tiktok",
                "lookup_user_id":"107955"
            }])
        );

        let (method, path, body) = request_parts(&requests[5]);
        assert_eq!(
            (method, path),
            ("PUT", "/v2/key-value-stores/test-store/records/OUTPUT")
        );
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            followers_response
        );
    }

    #[tokio::test]
    async fn uses_nested_profile_id_and_does_not_resolve_explicit_user_ids() {
        let server = MockServer::start(vec![response(
            200,
            r#"{"code":0,"data":{"user":{"id":107955}}}"#,
        )]);
        let mut scrappa_base_url = server.base_url.clone();
        scrappa_base_url.set_path("/api");
        let config = test_config(&server.base_url, &scrappa_base_url);
        let params =
            build_tiktok_followers_params(&json!({"profile":"@tiktok"}), &mut |_| {}).unwrap();
        assert_eq!(
            resolve_tiktok_user_id(&Client::new(), &config, &params)
                .await
                .unwrap(),
            "107955"
        );
        assert_eq!(server.requests().len(), 1);

        let no_request_server = MockServer::start(vec![]);
        let mut scrappa_base_url = no_request_server.base_url.clone();
        scrappa_base_url.set_path("/api");
        let config = test_config(&no_request_server.base_url, &scrappa_base_url);
        let params =
            build_tiktok_followers_params(&json!({"profile":"107955"}), &mut |_| {}).unwrap();
        assert_eq!(
            resolve_tiktok_user_id(&Client::new(), &config, &params)
                .await
                .unwrap(),
            "107955"
        );
        assert!(no_request_server.requests().is_empty());
    }

    #[tokio::test]
    async fn reports_nonzero_scrappa_codes_and_does_not_resolve_missing_ids() {
        let server = MockServer::start(vec![response(200, r#"{"code":-1,"msg":"not found"}"#)]);
        let mut scrappa_base_url = server.base_url.clone();
        scrappa_base_url.set_path("/api");
        let config = test_config(&server.base_url, &scrappa_base_url);
        let params =
            build_tiktok_followers_params(&json!({"profile":"@missing"}), &mut |_| {}).unwrap();
        let error = resolve_tiktok_user_id(&Client::new(), &config, &params)
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Scrappa TikTok Profile API returned code -1: not found"));
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn reports_upstream_http_errors_without_retrying() {
        let server = MockServer::start(vec![response(
            429,
            r#"{"message":"rate limited","errors":{"profile":["try later"]}}"#,
        )]);
        let mut scrappa_base_url = server.base_url.clone();
        scrappa_base_url.set_path("/api");
        let config = test_config(&server.base_url, &scrappa_base_url);
        let error = get_scrappa_json(
            &Client::new(),
            &config,
            &["tiktok", "user", "followers"],
            &[("user_id", "107955".to_owned())],
        )
        .await
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Scrappa API error (429): rate limited - profile: try later"
        );
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn request_deadline_is_sixty_seconds_and_timeout_is_not_retried() {
        let server = MockServer::start(vec![MockResponse {
            status: 200,
            body: r#"{"code":0}"#.to_owned(),
            delay: Duration::from_millis(100),
        }]);
        let mut scrappa_base_url = server.base_url.clone();
        scrappa_base_url.set_path("/api");
        let mut config = test_config(&server.base_url, &scrappa_base_url);
        config.scrappa_request_timeout = Duration::from_millis(20);
        let error = get_scrappa_json(
            &Client::new(),
            &config,
            &["tiktok", "user", "followers"],
            &[("user_id", "107955".to_owned())],
        )
        .await
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Scrappa API request timed out after 20ms"
        );
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn stores_output_and_skips_dataset_and_pricing_requests_when_no_followers_exist() {
        let response_body = json!({"code":0,"data":{"followers":[],"has_more":false}});
        let server = MockServer::start(vec![
            response(200, r#"{"profile":"107955"}"#),
            json_response(200, &response_body),
            response(201, ""),
        ]);
        let mut scrappa_base_url = server.base_url.clone();
        scrappa_base_url.set_path("/api");
        let config = test_config(&server.base_url, &scrappa_base_url);

        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(
            request_parts(&requests[2]).1,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
        assert!(!requests.iter().any(|request| {
            let (method, path, _) = request_parts(request);
            method == "POST" && path == "/v2/datasets/test-dataset/items"
        }));
        assert!(!requests
            .iter()
            .any(|request| { request_parts(request).1 == "/v2/actor-runs/test-run" }));
    }

    #[test]
    fn requires_a_scrappa_api_key() {
        assert_eq!(scrappa_api_key(Some("test-key")).unwrap(), "test-key");
        assert!(scrappa_api_key(None)
            .unwrap_err()
            .to_string()
            .contains("SCRAPPA_API_KEY environment variable is not set"));
        assert!(scrappa_api_key(Some("")).is_err());
    }
}
