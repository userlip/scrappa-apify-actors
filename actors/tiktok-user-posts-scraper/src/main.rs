use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Method, Response, StatusCode};
use serde_json::{json, Map, Value};
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const SCRAPPA_ENDPOINT: [&str; 3] = ["tiktok", "user", "posts"];
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: u32 = 8;
const APIFY_RETRY_DELAY: Duration = Duration::from_millis(500);
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
    scrappa_api_key: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env(
            "SCRAPPA_API_KEY",
            "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.",
        )?;
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env(
                "ACTOR_DEFAULT_KEY_VALUE_STORE_ID",
                "ACTOR_DEFAULT_KEY_VALUE_STORE_ID environment variable is not set",
            )?,
            default_dataset_id: required_env(
                "ACTOR_DEFAULT_DATASET_ID",
                "ACTOR_DEFAULT_DATASET_ID environment variable is not set",
            )?,
            actor_run_id: required_env(
                "ACTOR_RUN_ID",
                "ACTOR_RUN_ID environment variable is not set",
            )?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env(
                "APIFY_TOKEN",
                "APIFY_TOKEN environment variable is not set",
            )?,
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str, missing_message: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{missing_message}"))
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

struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    fn new(base_url: Url, token: String) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url,
            token,
        })
    }

    fn resource_url(&self, segments: &[&str]) -> Result<Url> {
        let mut path = vec!["v2"];
        path.extend_from_slice(segments);
        endpoint_url(&self.base_url, &path)
    }

    async fn request(
        &self,
        method: Method,
        url: Url,
        body: Option<&Value>,
        operation: &str,
    ) -> Result<Response> {
        for attempt in 0..=APIFY_MAX_RETRIES {
            let mut request = self
                .client
                .request(method.clone(), url.clone())
                .bearer_auth(&self.token)
                .header(reqwest::header::ACCEPT, "application/json");
            if let Some(body) = body {
                request = request.json(body);
            }

            match request.send().await {
                Ok(response)
                    if is_retryable_status(response.status()) && attempt < APIFY_MAX_RETRIES =>
                {
                    eprintln!("{operation} returned {}; retrying", response.status());
                }
                Ok(response) => return Ok(response),
                Err(error) if is_retryable_request(&error) && attempt < APIFY_MAX_RETRIES => {
                    eprintln!("{operation} failed; retrying: {error}");
                }
                Err(error) => return Err(anyhow!("{operation} failed: {error}")),
            }

            tokio::time::sleep(APIFY_RETRY_DELAY * 2_u32.pow(attempt)).await;
        }
        unreachable!("the final Apify request attempt always returns or fails")
    }

    async fn get_run(&self, actor_run_id: &str) -> Result<Value> {
        let url = self.resource_url(&["actor-runs", actor_run_id])?;
        let response = self
            .request(Method::GET, url, None, "Apify run pricing request")
            .await?;
        let response = successful_response(response, "fetch Actor run pricing").await?;
        response
            .json()
            .await
            .context("Actor run pricing response is not valid JSON")
    }

    async fn get_input(&self, store_id: &str, input_key: &str) -> Result<Option<Value>> {
        let url = self.resource_url(&["key-value-stores", store_id, "records", input_key])?;
        let response = self
            .request(Method::GET, url, None, "Apify INPUT request")
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "fetch Actor input").await?;
        Ok(Some(
            response
                .json()
                .await
                .context("Actor input record is not valid JSON")?,
        ))
    }

    async fn push_data(&self, dataset_id: &str, items: &[Value]) -> Result<()> {
        let url = self.resource_url(&["datasets", dataset_id, "items"])?;
        let response = self
            .request(
                Method::POST,
                url,
                Some(&Value::Array(items.to_vec())),
                "Apify dataset write",
            )
            .await?;
        successful_response(response, "store post items").await?;
        Ok(())
    }

    async fn set_output(&self, store_id: &str, output: &Value) -> Result<()> {
        let url = self.resource_url(&["key-value-stores", store_id, "records", "OUTPUT"])?;
        let response = self
            .request(Method::PUT, url, Some(output), "Apify OUTPUT write")
            .await?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_retryable_request(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request()
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.is_empty() {
        format!("HTTP {}", status.as_u16())
    } else {
        body
    };
    bail!(
        "Apify API error ({}) while trying to {operation}: {detail}",
        status.as_u16()
    );
}

#[derive(Debug, Default, PartialEq)]
struct TikTokUserPostsParams {
    unique_id: Option<String>,
    user_id: Option<String>,
    count: Option<i64>,
    cursor: Option<String>,
}

impl TikTokUserPostsParams {
    fn append_to_url(&self, url: &mut Url) {
        let mut query = url.query_pairs_mut();
        if let Some(unique_id) = &self.unique_id {
            query.append_pair("unique_id", unique_id);
        }
        if let Some(user_id) = &self.user_id {
            query.append_pair("user_id", user_id);
        }
        if let Some(count) = self.count {
            query.append_pair("count", &count.to_string());
        }
        if let Some(cursor) = &self.cursor {
            query.append_pair("cursor", cursor);
        }
    }
}

fn build_params(input: &Value) -> Result<TikTokUserPostsParams> {
    build_params_with_warning(input, |message| eprintln!("{message}"))
}

fn build_params_with_warning<F>(input: &Value, mut warn: F) -> Result<TikTokUserPostsParams>
where
    F: FnMut(String),
{
    let mut params = TikTokUserPostsParams::default();
    match input.get("profile") {
        Some(Value::String(profile)) if !js_trim(profile).is_empty() => {
            set_lookup(&mut params, normalize_profile_lookup(profile)?)
        }
        Some(value) if !value.is_null() && value.as_str() != Some("") => warn(format!(
            "profile must be a string, got {}.",
            value_type(value)
        )),
        _ => {}
    }

    if params.unique_id.is_none() && params.user_id.is_none() {
        match input.get("unique_id") {
            Some(Value::String(unique_id)) => {
                let unique_id = normalize_tiktok_unique_id(unique_id)?;
                if !unique_id.is_empty() {
                    params.unique_id = Some(unique_id);
                }
            }
            Some(value) if !value.is_null() && value.as_str() != Some("") => warn(format!(
                "unique_id must be a string, got {}.",
                value_type(value)
            )),
            _ => {}
        }
    }

    if params.unique_id.is_none() && params.user_id.is_none() {
        match input.get("user_id") {
            Some(Value::String(user_id)) => {
                let user_id = normalize_tiktok_user_id(user_id)?;
                if !user_id.is_empty() {
                    params.user_id = Some(user_id);
                }
            }
            Some(value) if !value.is_null() && value.as_str() != Some("") => warn(format!(
                "user_id must be a string, got {}.",
                value_type(value)
            )),
            _ => {}
        }
    }

    if let Some(count) = input.get("count") {
        let normalized = count.as_f64().filter(|number| {
            number.is_finite() && number.fract() == 0.0 && (1.0..=50.0).contains(number)
        });
        if let Some(count) = normalized {
            params.count = Some(count as i64);
        } else {
            warn(format!(
                "count must be an integer between 1 and 50, got {}. Using Scrappa default.",
                js_string(count)
            ));
        }
    }

    match input.get("cursor") {
        Some(Value::String(cursor)) => {
            let cursor = js_trim(cursor);
            if !cursor.is_empty() {
                params.cursor = Some(cursor.to_owned());
            }
        }
        Some(value) if !value.is_null() && value.as_str() != Some("") => warn(format!(
            "cursor must be a string, got {}. Starting from the first page.",
            value_type(value)
        )),
        _ => {}
    }

    if params.unique_id.is_none() && params.user_id.is_none() {
        bail!("TikTok unique_id or user_id is required");
    }
    Ok(params)
}

fn set_lookup(params: &mut TikTokUserPostsParams, (field, value): (&str, String)) {
    match field {
        "unique_id" => params.unique_id = Some(value),
        "user_id" => params.user_id = Some(value),
        _ => unreachable!("only TikTok lookup fields are produced"),
    }
}

fn format_lookup_for_log(input: &Value) -> Result<String> {
    if let Some(profile) = input
        .get("profile")
        .and_then(Value::as_str)
        .map(js_trim)
        .filter(|value| !value.is_empty())
    {
        let (field, value) = normalize_profile_lookup(profile)?;
        return Ok(match field {
            "unique_id" => value,
            "user_id" => format!("user_id:{value}"),
            _ => unreachable!(),
        });
    }
    if let Some(unique_id) = input
        .get("unique_id")
        .and_then(Value::as_str)
        .map(js_trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(normalize_tiktok_unique_id(unique_id)?);
    }
    if let Some(user_id) = input
        .get("user_id")
        .and_then(Value::as_str)
        .map(js_trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(format!("user_id:{}", normalize_tiktok_user_id(user_id)?));
    }
    Ok("unknown TikTok profile".to_owned())
}

fn normalize_profile_lookup(value: &str) -> Result<(&'static str, String)> {
    let trimmed = js_trim(value);
    if trimmed.bytes().all(|byte| byte.is_ascii_digit()) && !trimmed.is_empty() {
        return Ok(("user_id", normalize_tiktok_user_id(trimmed)?));
    }
    Ok(("unique_id", normalize_tiktok_unique_id(trimmed)?))
}

fn normalize_tiktok_unique_id(value: &str) -> Result<String> {
    let trimmed = js_trim(value);
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let url_like =
        has_url_scheme(trimmed) || trimmed.starts_with("//") || looks_like_tiktok_domain(trimmed);

    if let Ok(parsed) = Url::parse(trimmed) {
        let host = parsed.host_str().unwrap_or_default();
        if !(host.eq_ignore_ascii_case("tiktok.com")
            || host.to_ascii_lowercase().ends_with(".tiktok.com"))
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
            .filter(|path| path.starts_with('@') && !path[1..].contains('/'));
        let Some(username) = username else {
            bail!("TikTok profile URL must use the format https://www.tiktok.com/@username");
        };
        return normalize_tiktok_username(username);
    }

    if url_like {
        bail!("A valid TikTok profile URL or username is required");
    }
    normalize_tiktok_username(trimmed)
}

fn has_url_scheme(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once("://") else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
        })
}

fn is_js_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'..='\u{000d}'
            | '\u{0020}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

fn js_trim(value: &str) -> &str {
    value.trim_matches(is_js_whitespace)
}

fn looks_like_tiktok_domain(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.match_indices("tiktok.com").any(|(index, _)| {
        let before = index == 0 || value.as_bytes().get(index - 1) == Some(&b'.');
        let after = value.as_bytes().get(index + "tiktok.com".len());
        before && (after.is_none() || after == Some(&b'/'))
    })
}

fn normalize_tiktok_username(value: &str) -> Result<String> {
    let username = value.strip_prefix('@').unwrap_or(value);
    let valid = (2..=255).contains(&username.len())
        && username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'));
    if !valid {
        bail!("TikTok username must be 2 to 255 characters and contain only letters, numbers, dots, or underscores");
    }
    Ok(format!("@{username}"))
}

fn normalize_tiktok_user_id(value: &str) -> Result<String> {
    let user_id = js_trim(value);
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

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null | Value::Array(_) | Value::Object(_) => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
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

#[derive(Debug, PartialEq)]
enum DatasetBudget {
    Unlimited,
    Limited(usize),
}

impl DatasetBudget {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        if data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(Self::Unlimited);
        }
        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let Some(item_price) = events
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
        else {
            return Ok(Self::Unlimited);
        };
        if !item_price.is_finite() || item_price < 0.0 {
            bail!("Apify run returned an invalid dataset item price");
        }
        if item_price == 0.0 {
            return Ok(Self::Unlimited);
        }

        let max_charge = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|value| *value != 0.0)
            .unwrap_or(f64::INFINITY);
        if max_charge.is_nan() || max_charge < 0.0 {
            bail!("Apify run returned an invalid spending limit");
        }
        if max_charge.is_infinite() {
            return Ok(Self::Unlimited);
        }

        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut spent = 0.0;
        for (event_name, count) in counts {
            let Some(price) = events
                .get(&event_name)
                .and_then(|event| event.get("eventPriceUsd"))
                .and_then(Value::as_f64)
            else {
                continue;
            };
            let count = count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            if !price.is_finite() || price < 0.0 {
                bail!("Invalid price for charged event {event_name}");
            }
            spent += price * count as f64;
        }
        if !spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        let tolerance = f64::EPSILON * max_charge.max(1.0);
        let mut affordable = (((max_charge - spent + tolerance) / item_price)
            .floor()
            .max(0.0))
        .min(usize::MAX as f64) as usize;
        while affordable > 0 && spent + affordable as f64 * item_price > max_charge + tolerance {
            affordable -= 1;
        }
        Ok(Self::Limited(affordable))
    }

    fn limit(&self, requested: usize) -> usize {
        match self {
            Self::Unlimited => requested,
            Self::Limited(remaining) => requested.min(*remaining),
        }
    }
}

async fn scrappa_response(
    client: &Client,
    config: &ActorConfig,
    params: &TikTokUserPostsParams,
) -> Result<Value> {
    let mut url = endpoint_url(&config.scrappa_api_base_url, &SCRAPPA_ENDPOINT)?;
    params.append_to_url(&mut url);
    let response = client
        .get(url)
        .header("X-API-Key", &config.scrappa_api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(scrappa_request_error)?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let message = read_scrappa_error(response).await?;
        bail!("Scrappa API error ({status}): {message}");
    }
    response.json().await.map_err(scrappa_request_error)
}

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            SCRAPPA_REQUEST_TIMEOUT.as_millis()
        )
    } else {
        error.into()
    }
}

async fn read_scrappa_error(response: Response) -> Result<String> {
    let status = response.status();
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) if error.is_timeout() => return Err(scrappa_request_error(error)),
        Err(_) => return Ok(format_scrappa_error(status, "")),
    };
    Ok(format_scrappa_error(status, &body))
}

fn format_scrappa_error(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    if body.is_empty() {
        return fallback;
    }
    if let Ok(data) = serde_json::from_str::<Value>(&body) {
        let mut message = data
            .get("message")
            .filter(|message| !message.is_null())
            .map(js_string)
            .unwrap_or(fallback);
        if let Some(errors) = data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let messages = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .map(js_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {messages}")
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details);
            }
        }
        return message;
    }
    body.split(is_js_whitespace)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn extract_posts(data: Option<&Value>) -> Vec<&Value> {
    let Some(data) = data.filter(|data| js_truthy(data)) else {
        return Vec::new();
    };
    if let Some(posts) = data.as_array() {
        return posts.iter().collect();
    }
    ["posts", "videos", "aweme_list"]
        .iter()
        .find_map(|key| data.get(key).and_then(Value::as_array))
        .map(|posts| posts.iter().collect())
        .unwrap_or_default()
}

fn extract_pagination(data: Option<&Value>) -> (bool, Value) {
    let Some(data) = data
        .filter(|data| js_truthy(data))
        .filter(|data| !data.is_array())
    else {
        return (false, Value::Null);
    };
    let has_more = data
        .get("hasMore")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("has_more").filter(|value| !value.is_null()))
        .is_some_and(js_truthy);
    let cursor = data
        .get("cursor")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("max_cursor").filter(|value| !value.is_null()))
        .or_else(|| data.get("min_cursor").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null);
    (has_more, cursor)
}

fn enrich_post(post: &Value, params: &TikTokUserPostsParams) -> Value {
    let mut row = match post {
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
    row.insert(
        "lookup_unique_id".to_owned(),
        params
            .unique_id
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    row.insert(
        "lookup_user_id".to_owned(),
        params
            .user_id
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    Value::Object(row)
}

fn validate_scrappa_code(response: &Value) -> Result<()> {
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
        "Scrappa TikTok User Posts API returned code {}: {message}",
        js_string(code)
    );
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify = ApifyClient::new(
        config.apify_api_base_url.clone(),
        config.apify_token.clone(),
    )?;

    // Actor.init() loads run pricing before the input or scraper request is processed.
    let run_pricing = apify.get_run(&config.actor_run_id).await?;
    let dataset_budget = DatasetBudget::from_run(&run_pricing)?;
    let input = apify
        .get_input(&config.default_key_value_store_id, &config.input_key)
        .await?
        .filter(js_truthy)
        .ok_or_else(|| anyhow!("TikTok unique_id or user_id is required"))?;
    let params = build_params(&input)?;
    println!(
        "Fetching TikTok user posts for: {}",
        format_lookup_for_log(&input)?
    );

    let response = scrappa_response(&Client::new(), &config, &params).await?;
    validate_scrappa_code(&response)?;
    let data = response.get("data");
    let posts = extract_posts(data);
    let (has_next_page, next_cursor) = extract_pagination(data);
    let posts_saved = if posts.is_empty() {
        println!("No posts found for the given TikTok lookup");
        0
    } else {
        let rows = posts
            .iter()
            .map(|post| enrich_post(post, &params))
            .collect::<Vec<_>>();
        let allowed = dataset_budget.limit(rows.len());
        if allowed < rows.len() {
            println!(
                "Apify spend limit permits saving {allowed} of {} posts",
                rows.len()
            );
        }
        if allowed > 0 {
            apify
                .push_data(&config.default_dataset_id, &rows[..allowed])
                .await?;
        }
        println!("Found {} posts; saved {allowed}", rows.len());
        allowed
    };

    apify
        .set_output(&config.default_key_value_store_id, &response)
        .await?;
    let processed_time = response
        .get("processed_time")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);
    println!("TikTok user posts extraction completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "posts_extracted": posts.len(),
            "posts_saved": posts_saved,
            "has_next_page": has_next_page,
            "next_cursor": next_cursor,
            "processed_time": processed_time,
        })
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_pricing(model: &str, max_charge: Value, charged_counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": model,
                    "pricingPerEvent": { "actorChargeEvents": {
                        "apify-default-dataset-item": { "eventPriceUsd": 0.0003 },
                        "apify-actor-start": { "eventPriceUsd": 0.001 }
                    }}
                },
                "options": { "maxTotalChargeUsd": max_charge },
                "chargedEventCounts": charged_counts
            }
        })
    }

    #[test]
    fn builds_profile_params_and_keeps_profile_precedence() {
        let input = json!({
            "profile": " https://www.tiktok.com/@tik.tok_123/?lang=en ",
            "unique_id": "ignored",
            "user_id": "ignored",
            "count": 25,
            "cursor": "  next  "
        });
        let params = build_params(&input).unwrap();
        assert_eq!(params.unique_id.as_deref(), Some("@tik.tok_123"));
        assert_eq!(params.user_id, None);
        assert_eq!(params.count, Some(25));
        assert_eq!(params.cursor.as_deref(), Some("next"));
        assert_eq!(format_lookup_for_log(&input).unwrap(), "@tik.tok_123");
    }

    #[test]
    fn handles_numeric_profile_and_explicit_user_id() {
        assert_eq!(
            build_params(&json!({ "profile": "107955" }))
                .unwrap()
                .user_id
                .as_deref(),
            Some("107955")
        );
        assert_eq!(
            build_params(&json!({ "user_id": " 107955 " }))
                .unwrap()
                .user_id
                .as_deref(),
            Some("107955")
        );
        assert_eq!(
            build_params(&json!({ "unique_id": "tiktok", "user_id": "abc" }))
                .unwrap()
                .unique_id
                .as_deref(),
            Some("@tiktok")
        );
    }

    #[test]
    fn warns_and_omits_invalid_count_and_cursor() {
        let mut warnings = Vec::new();
        let params = build_params_with_warning(
            &json!({ "profile": "@tiktok", "count": 0, "cursor": 123 }),
            |message| warnings.push(message),
        )
        .unwrap();
        assert_eq!(params.count, None);
        assert_eq!(params.cursor, None);
        assert!(warnings[0].contains("count must be an integer between 1 and 50"));
        assert!(warnings[1].contains("cursor must be a string"));
    }

    #[test]
    fn rejects_missing_lookup_and_invalid_tiktok_profiles() {
        assert!(build_params(&json!({ "profile": " " }))
            .unwrap_err()
            .to_string()
            .contains("TikTok unique_id or user_id is required"));
        assert!(normalize_tiktok_unique_id("https://example.com/@tiktok")
            .unwrap_err()
            .to_string()
            .contains("must be on tiktok.com"));
        assert!(normalize_tiktok_unique_id("http://www.tiktok.com/@tiktok")
            .unwrap_err()
            .to_string()
            .contains("must use HTTPS"));
        assert!(normalize_tiktok_unique_id("@tik-tok")
            .unwrap_err()
            .to_string()
            .contains("TikTok username must"));
    }

    #[test]
    fn extracts_post_shapes_and_pagination_fallbacks() {
        let data = json!({
            "posts": [{ "aweme_id": "preferred" }],
            "videos": [{ "aweme_id": "1" }],
            "has_more": true,
            "max_cursor": "200"
        });
        assert_eq!(extract_posts(Some(&data))[0]["aweme_id"], "preferred");
        assert_eq!(extract_pagination(Some(&data)), (true, json!("200")));
        assert_eq!(
            extract_posts(Some(&json!({ "aweme_list": [{ "aweme_id": "3" }] })))[0]["aweme_id"],
            "3"
        );
        assert_eq!(
            extract_pagination(Some(&json!({
                "hasMore": true,
                "cursor": "100",
                "max_cursor": "200"
            }))),
            (true, json!("100"))
        );
        assert_eq!(
            extract_pagination(Some(&json!({ "has_more": false, "min_cursor": "300" }))),
            (false, json!("300"))
        );
        assert_eq!(extract_posts(Some(&json!([{ "aweme_id": "2" }]))).len(), 1);
        assert_eq!(extract_pagination(Some(&Value::Null)), (false, Value::Null));
    }

    #[test]
    fn enriches_each_post_with_both_lookup_columns() {
        let post = json!({ "aweme_id": "1" });
        let params = build_params(&json!({ "profile": "@tiktok" })).unwrap();
        let row = enrich_post(&post, &params);
        assert_eq!(row["lookup_unique_id"], "@tiktok");
        assert!(row["lookup_user_id"].is_null());
        assert_eq!(row["aweme_id"], "1");
    }

    #[test]
    fn applies_the_ppe_dataset_budget_and_leaves_other_pricing_unlimited() {
        let ppe = run_pricing(
            "PAY_PER_EVENT",
            json!(0.0019),
            json!({ "apify-actor-start": 1, "apify-default-dataset-item": 1 }),
        );
        let budget = DatasetBudget::from_run(&ppe).unwrap();
        assert_eq!(budget, DatasetBudget::Limited(2));
        assert_eq!(budget.limit(5), 2);

        let free = run_pricing("FREE", json!(0.0019), json!({}));
        assert_eq!(
            DatasetBudget::from_run(&free).unwrap(),
            DatasetBudget::Unlimited
        );
    }

    #[test]
    fn returns_scrappa_code_errors() {
        let error = validate_scrappa_code(&json!({ "code": 12, "msg": "Blocked" })).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa TikTok User Posts API returned code 12: Blocked"
        );
        assert!(validate_scrappa_code(&json!({ "code": 0 })).is_ok());
    }

    #[test]
    fn formats_scrappa_http_error_details_and_plain_text() {
        assert_eq!(
            format_scrappa_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                r#"{"message":"Invalid input","errors":{"profile":["required"]}}"#
            ),
            "Invalid input - profile: required"
        );
        assert_eq!(
            format_scrappa_error(StatusCode::BAD_GATEWAY, "  upstream\n unavailable  "),
            "upstream unavailable"
        );
    }
}
