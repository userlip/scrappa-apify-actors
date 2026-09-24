use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::{json, Map, Number, Value};
use std::{env, time::Duration};
use tokio::time::timeout;
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const PAGE_SIZE: usize = 50;
const OUTPUT_KEY: &str = "OUTPUT";

#[derive(Clone, Debug)]
pub struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    input_key: String,
    actor_run_id: String,
    apify_token: String,
    scrappa_api_key: String,
}

impl ActorConfig {
    pub fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        if scrappa_api_key.trim().is_empty() {
            bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
        }

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
struct FollowingParams {
    unique_id: Option<String>,
    user_id: Option<String>,
    count: Option<f64>,
    time: Option<Value>,
}

impl FollowingParams {
    fn requested_count(&self) -> f64 {
        self.count.unwrap_or(10.0)
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

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments.iter().copied());
    Ok(url)
}

fn warn_for_non_string(value: Option<&Value>, field: &str, warn: &mut impl FnMut(String)) {
    if let Some(value) = value {
        if !value.is_null() && value.as_str() != Some("") {
            warn(format!(
                "{field} must be a string, got {}.",
                js_typeof(value)
            ));
        }
    }
}

fn js_typeof(value: &Value) -> &'static str {
    match value {
        Value::Null => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) | Value::Object(_) => "object",
    }
}

fn resolve_lookup(
    input: &Value,
    warn: &mut impl FnMut(String),
) -> Result<(Option<String>, Option<String>)> {
    if let Some(value) = input.get("profile") {
        if let Some(profile) = value.as_str() {
            let profile = profile.trim();
            if !profile.is_empty() {
                if profile.bytes().all(|byte| byte.is_ascii_digit()) {
                    let user_id = normalize_user_id(profile)?;
                    if !user_id.is_empty() {
                        return Ok((None, Some(user_id)));
                    }
                } else {
                    let unique_id = normalize_unique_id(profile)?;
                    if !unique_id.is_empty() {
                        return Ok((Some(unique_id), None));
                    }
                }
            }
        } else {
            warn_for_non_string(Some(value), "profile", warn);
        }
    }

    if let Some(value) = input.get("unique_id") {
        if let Some(unique_id) = value.as_str() {
            let unique_id = normalize_unique_id(unique_id)?;
            if !unique_id.is_empty() {
                return Ok((Some(unique_id), None));
            }
        } else {
            warn_for_non_string(Some(value), "unique_id", warn);
        }
    }

    if let Some(value) = input.get("user_id") {
        if let Some(user_id) = value.as_str() {
            let user_id = normalize_user_id(user_id)?;
            if !user_id.is_empty() {
                return Ok((None, Some(user_id)));
            }
        } else {
            warn_for_non_string(Some(value), "user_id", warn);
        }
    }

    Ok((None, None))
}

fn is_tiktok_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("tiktok.com") || host.to_ascii_lowercase().ends_with(".tiktok.com")
}

fn is_url_like(value: &str) -> bool {
    let has_scheme = value.split_once("://").is_some_and(|(scheme, _)| {
        let mut chars = scheme.chars();
        chars
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
            && chars.all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
            })
    });
    if has_scheme || value.starts_with("//") {
        return true;
    }

    let lower = value.to_ascii_lowercase();
    lower.match_indices("tiktok.com").any(|(index, host)| {
        let before_is_boundary = index == 0 || lower.as_bytes()[index - 1] == b'.';
        let after_index = index + host.len();
        let after_is_boundary = after_index == lower.len() || lower.as_bytes()[after_index] == b'/';
        before_is_boundary && after_is_boundary
    })
}

fn normalize_unique_id(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    if is_url_like(trimmed) {
        let parsed = Url::parse(trimmed)
            .map_err(|_| anyhow!("A valid TikTok profile URL or username is required"))?;
        if !is_tiktok_host(parsed.host_str().unwrap_or_default()) {
            bail!("TikTok profile URL must be on tiktok.com");
        }
        if parsed.scheme() != "https" {
            bail!("TikTok profile URL must use HTTPS");
        }

        let path = parsed.path();
        let Some(username) = path
            .strip_prefix("/@")
            .map(|tail| tail.strip_suffix('/').unwrap_or(tail))
            .filter(|username| !username.is_empty() && !username.contains('/'))
        else {
            bail!("TikTok profile URL must use the format https://www.tiktok.com/@username");
        };
        return normalize_username(username);
    }

    normalize_username(trimmed)
}

fn normalize_username(value: &str) -> Result<String> {
    let username = value.strip_prefix('@').unwrap_or(value);
    if username.len() < 2
        || username.len() > 255
        || !username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'))
    {
        bail!("TikTok username must be 2 to 255 characters and contain only letters, numbers, dots, or underscores");
    }

    Ok(format!("@{username}"))
}

fn normalize_user_id(value: &str) -> Result<String> {
    let user_id = value.trim();
    if user_id.is_empty() {
        return Ok(String::new());
    }
    if !user_id.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("TikTok user_id must contain digits only");
    }
    if user_id.len() > 30 {
        bail!("TikTok numeric user ID must be 30 digits or fewer");
    }
    Ok(user_id.to_owned())
}

fn build_params(input: &Value, mut warn: impl FnMut(String)) -> Result<FollowingParams> {
    let (unique_id, user_id) = resolve_lookup(input, &mut warn)?;

    let count = input
        .get("count")
        .and_then(Value::as_number)
        .and_then(|number| {
            let count = number.as_f64()?;
            (count.is_finite() && count.fract() == 0.0 && count >= 1.0).then_some(count)
        });
    if input.get("count").is_some()
        && input
            .get("count")
            .and_then(Value::as_number)
            .and_then(Number::as_f64)
            .is_none_or(|count| !count.is_finite() || count.fract() != 0.0 || count < 1.0)
    {
        warn(format!(
            "count must be a positive integer, got {}. Using Scrappa default.",
            input.get("count").map(js_string).unwrap_or_default()
        ));
    }

    let time_field = if input.get("time").is_some() {
        "time"
    } else {
        "cursor"
    };
    let pagination_value = input
        .get("time")
        .filter(|value| !value.is_null())
        .or_else(|| input.get("cursor").filter(|value| !value.is_null()));

    let time = match pagination_value {
        Some(Value::Number(number)) => {
            let value = number.as_f64().unwrap_or_default();
            if value.is_finite() && value.fract() == 0.0 && value >= 0.0 {
                Some(Value::Number(number.clone()))
            } else {
                warn(format!(
                    "{time_field} must be a non-negative integer, got {}. Starting from the first page.",
                    js_string(pagination_value.unwrap())
                ));
                None
            }
        }
        Some(Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                None
            } else if value.bytes().all(|byte| byte.is_ascii_digit()) {
                Some(Value::String(value.to_owned()))
            } else {
                warn(format!(
                    "{time_field} must contain digits only. Starting from the first page."
                ));
                None
            }
        }
        Some(Value::Null) | None => None,
        Some(value) if value.as_str() == Some("") => None,
        Some(value) => {
            warn(format!(
                "{time_field} must be a non-negative integer or digit string, got {}. Starting from the first page.",
                js_typeof(value)
            ));
            None
        }
    };

    if unique_id.is_none() && user_id.is_none() {
        bail!("TikTok unique_id or user_id is required");
    }

    Ok(FollowingParams {
        unique_id,
        user_id,
        count,
        time,
    })
}

fn format_lookup(input: &Value) -> String {
    let mut ignored_warning = |_| {};
    match resolve_lookup(input, &mut ignored_warning) {
        Ok((Some(unique_id), _)) => unique_id,
        Ok((_, Some(user_id))) => format!("user_id:{user_id}"),
        _ => "unknown TikTok profile".to_owned(),
    }
}

fn js_number_string(number: &Number) -> String {
    let Some(value) = number.as_f64() else {
        return number.to_string();
    };
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.fract() == 0.0 && value.abs() < 1e21 {
        return format!("{value:.0}");
    }
    number.to_string()
}

fn requested_count_json(count: f64) -> Value {
    if count < u64::MAX as f64 {
        return Value::Number(Number::from(count as u64));
    }
    Number::from_f64(count)
        .map(Value::Number)
        .unwrap_or_else(|| Value::Number(Number::from(10)))
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(number) => js_number_string(number),
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

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_strict_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Number(left), Value::Number(right)) => left.as_f64() == right.as_f64(),
        (Value::String(left), Value::String(right)) => left == right,
        _ => false,
    }
}

fn extract_string_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) => {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_owned())
        }
        Value::Number(value) if value.as_f64().is_some_and(|number| number.is_finite()) => {
            Some(js_number_string(value))
        }
        _ => None,
    }
}

fn extract_profile_user_id(data: Option<&Value>) -> Option<String> {
    let profile = match data? {
        Value::Array(values) => values.first()?,
        value => value,
    };
    if !profile.is_object() {
        return None;
    }

    extract_string_value(profile.get("user_id"))
        .or_else(|| extract_string_value(profile.get("id")))
        .or_else(|| {
            let user = profile.get("user")?;
            extract_string_value(user.get("user_id"))
                .or_else(|| extract_string_value(user.get("id")))
        })
}

fn following_items(data: Option<&Value>) -> &[Value] {
    match data {
        Some(Value::Array(values)) => values,
        Some(Value::Object(data)) => ["following", "followings", "users", "user_list"]
            .iter()
            .find_map(|field| data.get(*field).and_then(Value::as_array))
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        _ => &[],
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Pagination {
    has_next_page: bool,
    next_time: Option<Value>,
}

fn extract_pagination(data: Option<&Value>) -> Pagination {
    let Some(data) = data.and_then(Value::as_object) else {
        return Pagination {
            has_next_page: false,
            next_time: None,
        };
    };

    let has_more = data
        .get("hasMore")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("has_more").filter(|value| !value.is_null()));
    let next_time = data
        .get("time")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("min_time").filter(|value| !value.is_null()))
        .or_else(|| data.get("max_time").filter(|value| !value.is_null()));

    Pagination {
        has_next_page: has_more.is_some_and(js_truthy),
        next_time: next_time.cloned(),
    }
}

fn dataset_item(user: &Value, unique_id: Option<&str>, user_id: Option<&str>) -> Value {
    let mut item = match user {
        Value::Object(fields) => fields.clone(),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        Value::String(value) => value
            .chars()
            .enumerate()
            .map(|(index, character)| (index.to_string(), Value::String(character.to_string())))
            .collect(),
        _ => Map::new(),
    };
    item.insert(
        "lookup_unique_id".to_owned(),
        unique_id.map_or(Value::Null, |value| Value::String(value.to_owned())),
    );
    item.insert(
        "lookup_user_id".to_owned(),
        user_id.map_or(Value::Null, |value| Value::String(value.to_owned())),
    );
    Value::Object(item)
}

fn following_url(
    base_url: &Url,
    params: &FollowingParams,
    count: usize,
    time: Option<&Value>,
) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["tiktok", "user", "following"])?;
    {
        let mut query = url.query_pairs_mut();
        if let Some(unique_id) = &params.unique_id {
            query.append_pair("unique_id", unique_id);
        }
        if let Some(user_id) = &params.user_id {
            query.append_pair("user_id", user_id);
        }
        query.append_pair("count", &count.to_string());
        if let Some(time) = time.filter(|value| {
            !value.is_null() && value.as_str() != Some("") && !matches!(value, Value::Bool(false))
        }) {
            let time = match time {
                Value::Bool(true) => "1".to_owned(),
                _ => js_string(time),
            };
            query.append_pair("time", &time);
        }
    }
    Ok(url)
}

fn profile_url(base_url: &Url, unique_id: &str) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["tiktok", "user", "profile"])?;
    url.query_pairs_mut().append_pair("unique_id", unique_id);
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

async fn get_input(client: &Client, config: &ActorConfig) -> Result<Option<Value>> {
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
        .header(reqwest::header::ACCEPT, "application/json")
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify INPUT request failed")?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response_json(response, "Apify INPUT request")
        .await
        .map(Some)
}

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() || error.to_string().contains("aborted") {
        anyhow!(
            "Scrappa API request timed out after {}s",
            REQUEST_TIMEOUT.as_secs()
        )
    } else {
        anyhow::Error::new(error)
    }
}

fn response_error_message(body: &str, fallback: &str) -> String {
    let Ok(error_data) = serde_json::from_str::<Value>(body) else {
        return body
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(500)
            .collect();
    };

    let mut message = error_data
        .get("message")
        .filter(|value| !value.is_null())
        .map(js_string)
        .unwrap_or_else(|| fallback.to_owned());
    if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .filter_map(|(field, messages)| {
                let messages = messages.as_array()?;
                let messages = messages
                    .iter()
                    .map(|message| {
                        if message.is_null() {
                            String::new()
                        } else {
                            js_string(message)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(format!("{field}: {messages}"))
            })
            .collect::<Vec<_>>()
            .join("; ");
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details);
        }
    }
    message
}

async fn fetch_scrappa_json(client: &Client, url: &Url, api_key: &str) -> Result<Value> {
    let request = async {
        let response = client
            .get(url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .header("X-API-Key", api_key)
            .send()
            .await
            .map_err(scrappa_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let fallback = status.canonical_reason().unwrap_or("Unknown error");
            let body = response.text().await.map_err(scrappa_request_error)?;
            let message = if body.is_empty() {
                fallback.to_owned()
            } else {
                response_error_message(&body, fallback)
            };
            bail!("Scrappa API error ({}): {message}", status.as_u16());
        }
        response
            .json()
            .await
            .context("Scrappa API response was not valid JSON")
    };

    timeout(REQUEST_TIMEOUT, request).await.map_err(|_| {
        anyhow!(
            "Scrappa API request timed out after {}s",
            REQUEST_TIMEOUT.as_secs()
        )
    })?
}

fn check_scrappa_code(response: &Value, endpoint: &str) -> Result<()> {
    let Some(code) = response.get("code") else {
        return Ok(());
    };
    if code.as_f64() == Some(0.0) {
        return Ok(());
    }
    let message = response
        .get("msg")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or_else(|| "Unknown error".to_owned());
    bail!(
        "Scrappa TikTok {endpoint} API returned code {}: {message}",
        js_string(code)
    );
}

fn apify_run_url(base_url: &Url, run_id: &str) -> Result<Url> {
    endpoint_url(base_url, &["v2", "actor-runs", run_id])
}

async fn run_dataset_capacity(
    client: &Client,
    config: &ActorConfig,
    requested: usize,
) -> Result<usize> {
    let response = client
        .get(apify_run_url(
            &config.apify_api_base_url,
            &config.actor_run_id,
        )?)
        .timeout(REQUEST_TIMEOUT)
        .header(reqwest::header::ACCEPT, "application/json")
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify run pricing request failed")?;
    let run = response_json(response, "Apify run pricing request").await?;
    affordable_dataset_items(&run, requested)
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
        .get("apify-default-dataset-item")
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

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    items: &[Value],
) -> Result<usize> {
    if items.is_empty() {
        return Ok(0);
    }

    let capacity = run_dataset_capacity(client, config, items.len()).await?;
    let items = &items[..items.len().min(capacity)];
    if items.is_empty() {
        return Ok(0);
    }

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
    ensure_success(response, "Apify dataset write").await?;
    Ok(items.len())
}

async fn set_output(client: &Client, config: &ActorConfig, output: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            OUTPUT_KEY,
        ],
    )?;
    let body = serde_json::to_vec(output).context("Could not encode OUTPUT value")?;
    let response = client
        .put(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .bearer_auth(&config.apify_token)
        .body(body)
        .send()
        .await
        .context("Apify OUTPUT write failed")?;
    ensure_success(response, "Apify OUTPUT write").await
}

async fn ensure_success(response: Response, operation: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
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

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config)
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("TikTok unique_id or user_id is required"))?;
    let mut params = build_params(&input, |warning| eprintln!("Warning: {warning}"))?;
    let requested_count = params.requested_count();
    let original_lookup_unique_id = params.unique_id.clone();

    println!("Fetching TikTok following for: {}", format_lookup(&input));
    if let Some(unique_id) = params
        .unique_id
        .clone()
        .filter(|_| params.user_id.is_none())
    {
        println!("Resolving TikTok username {unique_id} to numeric user_id");
        let response = fetch_scrappa_json(
            client,
            &profile_url(&config.scrappa_api_base_url, &unique_id)?,
            &config.scrappa_api_key,
        )
        .await?;
        check_scrappa_code(&response, "Profile")?;
        let user_id = extract_profile_user_id(response.get("data"))
            .ok_or_else(|| anyhow!("Could not resolve {unique_id} to a TikTok user_id"))?;
        params.user_id = Some(user_id);
        params.unique_id = None;
    }

    let mut latest_response: Option<Value> = None;
    let mut latest_pagination = Pagination {
        has_next_page: false,
        next_time: None,
    };
    let mut next_time = params.time.clone();
    let mut following_extracted = 0usize;
    let mut pages_fetched = 0usize;

    while (following_extracted as f64) < requested_count {
        let current_time = next_time.clone();
        let remaining_count =
            (requested_count - following_extracted as f64).min(usize::MAX as f64) as usize;
        let page_count = PAGE_SIZE.min(remaining_count);
        let url = following_url(
            &config.scrappa_api_base_url,
            &params,
            page_count,
            current_time.as_ref(),
        )?;

        println!(
            "Fetching TikTok following page {} ({page_count} requested)",
            pages_fetched + 1
        );
        let response = fetch_scrappa_json(client, &url, &config.scrappa_api_key).await?;
        latest_response = Some(response.clone());
        pages_fetched += 1;

        check_scrappa_code(&response, "Following")?;
        let data = response.get("data");
        let following = following_items(data);
        latest_pagination = extract_pagination(data);

        if !following.is_empty() {
            let items = following
                .iter()
                .take(remaining_count)
                .map(|user| {
                    dataset_item(
                        user,
                        original_lookup_unique_id.as_deref(),
                        params.user_id.as_deref(),
                    )
                })
                .collect::<Vec<_>>();
            let saved = push_dataset_items(client, config, &items).await?;
            following_extracted += saved;
            println!("Found {saved} followed accounts on page {pages_fetched}");

            if saved < items.len() {
                println!(
                    "PPE budget allowed {saved} of {} dataset items on page {pages_fetched}",
                    items.len()
                );
                break;
            }
        }

        if following.is_empty()
            || !latest_pagination.has_next_page
            || latest_pagination.next_time.is_none()
            || current_time.as_ref().is_some_and(|current| {
                js_strict_equal(latest_pagination.next_time.as_ref().unwrap(), current)
            })
        {
            break;
        }

        next_time = latest_pagination.next_time.clone();
    }

    if following_extracted == 0 {
        println!("No followed accounts found for the given TikTok lookup");
    } else {
        println!("Found {following_extracted} followed accounts");
    }

    let summary = json!({
        "following_extracted": following_extracted,
        "requested_count": requested_count_json(requested_count),
        "pages_fetched": pages_fetched,
        "has_next_page": latest_pagination.has_next_page,
        "next_time": latest_pagination.next_time,
        "processed_time": latest_response
            .as_ref()
            .and_then(|response| response.get("processed_time"))
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    });
    let output = if requested_count <= PAGE_SIZE as f64 {
        latest_response.as_ref().unwrap_or(&summary)
    } else {
        &summary
    };
    set_output(client, config, output).await?;

    println!("TikTok following extraction completed successfully");
    println!("Results summary: {}", summary);
    Ok(())
}

pub async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("Could not create HTTP client")?;
    run_actor(&client, &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    fn base_url() -> Url {
        Url::parse("https://scrappa.co/api").unwrap()
    }

    fn test_config(server_url: Url, api_key: &str) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: server_url.clone(),
            scrappa_api_base_url: server_url,
            default_key_value_store_id: "store-id".to_owned(),
            default_dataset_id: "dataset-id".to_owned(),
            input_key: "INPUT".to_owned(),
            actor_run_id: "run-id".to_owned(),
            apify_token: "test-apify-token".to_owned(),
            scrappa_api_key: api_key.to_owned(),
        }
    }

    #[test]
    fn normalizes_profile_inputs_and_keeps_lookup_precedence() {
        let params = build_params(
            &json!({ "profile": "https://www.tiktok.com/@TikTok?lang=en", "unique_id": "other", "user_id": "bad" }),
            |_| {},
        )
        .unwrap();
        assert_eq!(
            params,
            FollowingParams {
                unique_id: Some("@TikTok".to_owned()),
                user_id: None,
                count: None,
                time: None,
            }
        );

        let params = build_params(&json!({ "profile": "107955" }), |_| {}).unwrap();
        assert_eq!(params.user_id.as_deref(), Some("107955"));
        assert_eq!(params.unique_id, None);

        let params = build_params(&json!({ "unique_id": "107955" }), |_| {}).unwrap();
        assert_eq!(params.unique_id.as_deref(), Some("@107955"));
    }

    #[test]
    fn validates_urls_usernames_and_user_ids() {
        assert_eq!(
            build_params(&json!({ "profile": "@tiktok" }), |_| {})
                .unwrap()
                .unique_id
                .as_deref(),
            Some("@tiktok")
        );
        assert!(
            build_params(&json!({ "profile": "https://example.com/@tiktok" }), |_| {})
                .unwrap_err()
                .to_string()
                .contains("must be on tiktok.com")
        );
        assert!(
            build_params(&json!({ "profile": "http://tiktok.com/@tiktok" }), |_| {})
                .unwrap_err()
                .to_string()
                .contains("must use HTTPS")
        );
        assert!(build_params(&json!({ "unique_id": "@tik-tok" }), |_| {})
            .unwrap_err()
            .to_string()
            .contains("TikTok username must be"));
        assert!(build_params(&json!({ "user_id": "12x" }), |_| {})
            .unwrap_err()
            .to_string()
            .contains("user_id must contain digits only"));
    }

    #[test]
    fn retains_count_defaulting_and_cursor_compatibility() {
        let mut warnings = Vec::new();
        let params = build_params(
            &json!({ "profile": "tiktok", "count": 0, "time": null, "cursor": "123" }),
            |warning| warnings.push(warning),
        )
        .unwrap();
        assert_eq!(params.requested_count(), 10.0);
        assert_eq!(params.time, Some(Value::String("123".to_owned())));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("count must be a positive integer"));

        let params = build_params(
            &json!({ "profile": "@tiktok", "count": 100000, "time": 0 }),
            |_| {},
        )
        .unwrap();
        assert_eq!(params.requested_count(), 100000.0);
        assert_eq!(params.time, Some(json!(0)));

        let params = build_params(
            &json!({ "profile": "@tiktok", "time": "  ", "cursor": "123" }),
            |_| {},
        )
        .unwrap();
        assert_eq!(params.time, None);
    }

    #[test]
    fn extracts_profile_ids_following_aliases_and_pagination() {
        assert_eq!(
            extract_profile_user_id(Some(&json!([{ "user_id": " 123 " }]))).as_deref(),
            Some("123")
        );
        assert_eq!(
            extract_profile_user_id(Some(&json!({ "user": { "id": 456 } }))).as_deref(),
            Some("456")
        );
        assert_eq!(
            following_items(Some(&json!({ "following": [{ "id": 1 }] }))).len(),
            1
        );
        assert_eq!(
            following_items(Some(&json!({ "followings": [{ "id": 2 }] }))).len(),
            1
        );
        assert_eq!(
            following_items(Some(&json!({ "users": [{ "id": 3 }] }))).len(),
            1
        );
        assert_eq!(
            following_items(Some(&json!({ "user_list": [{ "id": 4 }] }))).len(),
            1
        );

        let pagination = extract_pagination(Some(&json!({
            "hasMore": null,
            "has_more": true,
            "time": null,
            "min_time": "1711111111",
            "max_time": "later"
        })));
        assert_eq!(
            pagination,
            Pagination {
                has_next_page: true,
                next_time: Some(Value::String("1711111111".to_owned())),
            }
        );
        assert_eq!(
            extract_pagination(Some(&json!([{ "user_id": "1" }]))),
            Pagination {
                has_next_page: false,
                next_time: None,
            }
        );
    }

    #[test]
    fn builds_encoded_api_urls_with_cursor_markers() {
        let params = FollowingParams {
            unique_id: Some("@tiktok".to_owned()),
            user_id: None,
            count: None,
            time: None,
        };
        assert_eq!(
            following_url(&base_url(), &params, 50, Some(&json!(0)))
                .unwrap()
                .as_str(),
            "https://scrappa.co/api/tiktok/user/following?unique_id=%40tiktok&count=50&time=0"
        );
        assert_eq!(
            profile_url(&base_url(), "@a & b").unwrap().as_str(),
            "https://scrappa.co/api/tiktok/user/profile?unique_id=%40a+%26+b"
        );
    }

    #[test]
    fn calculates_ppe_capacity_from_existing_run_charges() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-default-dataset-item": { "eventPriceUsd": 0.0001 },
                            "other-event": { "eventPriceUsd": 0.0001 }
                        }
                    }
                },
                "chargedEventCounts": { "other-event": 1 },
                "options": { "maxTotalChargeUsd": 0.00025 }
            }
        });
        assert_eq!(affordable_dataset_items(&run, 5).unwrap(), 1);
        assert_eq!(
            affordable_dataset_items(
                &json!({
                    "data": {
                        "pricingInfo": {
                            "pricingModel": "PAY_PER_EVENT",
                            "pricingPerEvent": {
                                "actorChargeEvents": {
                                    "apify-default-dataset-item": { "eventPriceUsd": 0 }
                                }
                            }
                        },
                        "chargedEventCounts": {},
                        "options": { "maxTotalChargeUsd": 0 }
                    }
                }),
                5
            )
            .unwrap(),
            5
        );
    }

    fn mock_server(
        responses: Vec<(&'static str, &'static str)>,
    ) -> (Url, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                requests.push(read_request(&mut stream));
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
            requests
        });
        (
            Url::parse(&format!("http://{address}/api")).unwrap(),
            server,
        )
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut reader = BufReader::new(stream);
        let mut request = String::new();
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        request.push_str(&line);
        let mut content_length = 0usize;
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.trim().parse().unwrap();
                }
            }
            request.push_str(&line);
        }
        request.push_str("\r\n");
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        request.push_str(&String::from_utf8(body).unwrap());
        request
    }

    const RUN_NORMAL_BUDGET: &str = r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001},"other-event":{"eventPriceUsd":0.0001}}}},"chargedEventCounts":{"other-event":1},"options":{"maxTotalChargeUsd":1.0}}}"#;
    const RUN_ONE_ITEM_LEFT: &str = r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001},"other-event":{"eventPriceUsd":0.0001}}}},"chargedEventCounts":{"other-event":1},"options":{"maxTotalChargeUsd":0.00025}}}"#;

    #[tokio::test]
    async fn resolves_profiles_paginates_and_writes_dataset_and_summary() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", r#"{"profile":"tiktok","count":55}"#),
            ("200 OK", r#"{"code":0,"data":{"user_id":"107955"}}"#),
            (
                "200 OK",
                r#"{"code":0,"data":{"following":[{"user_id":"1","unique_id":"one"},{"user_id":"2","extra":true}],"hasMore":true,"time":1711111111},"processed_time":10}"#,
            ),
            ("200 OK", RUN_NORMAL_BUDGET),
            ("201 Created", ""),
            (
                "200 OK",
                r#"{"code":0,"data":{"users":[{"user_id":"3"}],"has_more":false,"time":0},"processed_time":20}"#,
            ),
            ("200 OK", RUN_NORMAL_BUDGET),
            ("201 Created", ""),
            ("201 Created", ""),
        ]);
        let config = test_config(base_url, "test-scrappa-key");
        let client = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();

        run_actor(&client, &config).await.unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 9);
        assert!(requests[0].starts_with("GET /api/v2/key-value-stores/store-id/records/INPUT "));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer test-apify-token"));
        assert!(requests[1].contains("GET /api/tiktok/user/profile?unique_id=%40tiktok "));
        assert!(requests[1]
            .to_ascii_lowercase()
            .contains("x-api-key: test-scrappa-key"));
        assert!(requests[2].contains("GET /api/tiktok/user/following?user_id=107955&count=50 "));
        assert!(requests[3].contains("GET /api/v2/actor-runs/run-id "));
        assert!(requests[4].contains("POST /api/v2/datasets/dataset-id/items "));
        assert!(requests[4].contains("\"lookup_unique_id\":\"@tiktok\""));
        assert!(requests[4].contains("\"lookup_user_id\":\"107955\""));
        assert!(requests[5]
            .contains("GET /api/tiktok/user/following?user_id=107955&count=50&time=1711111111 "));
        assert!(requests[6].contains("GET /api/v2/actor-runs/run-id "));
        assert!(requests[7].contains("POST /api/v2/datasets/dataset-id/items "));
        assert!(requests[8].starts_with("PUT /api/v2/key-value-stores/store-id/records/OUTPUT "));
        assert!(requests[8].contains("\"following_extracted\":3"));
        assert!(requests[8].contains("\"requested_count\":55"));
        assert!(requests[8].contains("\"next_time\":0"));
    }

    #[tokio::test]
    async fn stops_at_the_ppe_cap_and_keeps_small_run_output_response() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", r#"{"profile":"107955","count":10}"#),
            (
                "200 OK",
                r#"{"code":0,"data":{"following":[{"user_id":"1"},{"user_id":"2"}],"hasMore":true,"time":12},"processed_time":30}"#,
            ),
            ("200 OK", RUN_ONE_ITEM_LEFT),
            ("201 Created", ""),
            ("201 Created", ""),
        ]);
        let config = test_config(base_url, "test-scrappa-key");
        let client = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();

        run_actor(&client, &config).await.unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 5);
        assert!(requests[1].contains("GET /api/tiktok/user/following?user_id=107955&count=10 "));
        assert!(requests[3].contains("\"user_id\":\"1\""));
        assert!(requests[3].contains("\"lookup_unique_id\":null"));
        assert!(requests[3].contains("\"lookup_user_id\":\"107955\""));
        assert!(requests[4].contains("\"following\":[{\"user_id\":\"1\"},{\"user_id\":\"2\"}]"));
        assert!(!requests[4].contains("\"following_extracted\""));
    }

    #[tokio::test]
    async fn reports_scrappa_http_errors_without_retrying() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", r#"{"profile":"107955"}"#),
            (
                "429 Too Many Requests",
                r#"{"message":"Slow down","errors":{"profile":["blocked"]}}"#,
            ),
        ]);
        let config = test_config(base_url, "test-scrappa-key");
        let client = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();

        let error = run_actor(&client, &config).await.unwrap_err().to_string();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 2);
        assert!(error.contains("Scrappa API error (429): Slow down - profile: blocked"));
    }
}
