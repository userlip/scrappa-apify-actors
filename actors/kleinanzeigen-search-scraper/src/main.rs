use std::{
    env,
    error::Error as StdError,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Number, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const LISTING_RESULT_CHARGE_EVENT: &str = "listing-result";
const MAX_QUERY_LENGTH: usize = 500;
const MAX_FILTER_LENGTH: usize = 100;
const MAX_PAGE: i64 = 100;
const MAX_BATCH_SEARCHES: usize = 25;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

struct Config {
    apify_api_base: Url,
    scrappa_api_base: Url,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    scrappa_api_key: Option<String>,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key: env::var("SCRAPPA_API_KEY")
                .ok()
                .filter(|api_key| !api_key.is_empty()),
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
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
        .extend(segments);
    Ok(url)
}

async fn response_text(response: Response, operation: &str) -> Result<String> {
    response
        .text()
        .await
        .with_context(|| format!("{operation} response could not be read"))
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response_text(response, operation).await?;
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
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

    serde_json::from_str(&body).with_context(|| format!("{operation} returned invalid JSON"))
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

struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

impl ApifyClient<'_> {
    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
    }

    fn authenticated(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
    }

    async fn get_input(&self) -> Result<Value> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .authenticated(self.http.get(url))
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(json!({}));
        }
        response_json(response, "Apify INPUT request").await
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .authenticated(self.http.get(url))
            .send()
            .await
            .context("Apify run pricing request failed")?;
        response_json(response, "Apify run pricing request").await
    }

    async fn charge_event(&self, count: usize, idempotency_key: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        let response = self
            .authenticated(self.http.post(url))
            .header("idempotency-key", idempotency_key)
            .json(&json!({
                "eventName": LISTING_RESULT_CHARGE_EVENT,
                "count": count,
            }))
            .send()
            .await
            .context("Apify listing-result charge request failed")?;
        ensure_success(response, "Apify listing-result charge").await
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .authenticated(self.http.post(url))
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }

    async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .authenticated(self.http.put(url))
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }

    async fn set_terminal_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .authenticated(self.http.put(url))
            .json(&json!({
                "runId": self.config.actor_run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true,
            }))
            .send()
            .await
            .context("Apify run status update failed")?;
        ensure_success(response, "Apify run status update").await
    }
}

#[derive(Clone, Debug)]
struct SearchPlanItem {
    index: usize,
    params: Map<String, Value>,
}

fn decode_input_string(value: &str) -> String {
    let bytes = value.as_bytes();
    if !bytes.contains(&b'%') {
        return value.to_owned();
    }

    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        if index + 2 >= bytes.len() {
            return value.to_owned();
        }
        let Some(high) = hex_value(bytes[index + 1]) else {
            return value.to_owned();
        };
        let Some(low) = hex_value(bytes[index + 2]) else {
            return value.to_owned();
        };
        decoded.push((high << 4) | low);
        index += 3;
    }
    String::from_utf8(decoded).unwrap_or_else(|_| value.to_owned())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
    let cleaned = decode_input_string(value)
        .trim_matches(is_javascript_whitespace)
        .to_owned();
    if cleaned.is_empty() {
        return Ok(None);
    }
    if cleaned.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(cleaned))
}

fn is_javascript_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_integer(value: Option<&Value>, field: &str, min: i64, max: u64) -> Result<Option<Value>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }

    let integer = if let Some(text) = value.as_str() {
        let text = text.trim();
        let digits = text.strip_prefix('-').unwrap_or(text);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            bail!("{field} must be an integer");
        }
        text.parse::<i128>()
            .map_err(|_| anyhow!("{field} must be an integer"))?
    } else if let Some(number) = value.as_number() {
        number
            .as_i64()
            .map(i128::from)
            .or_else(|| number.as_u64().map(i128::from))
            .or_else(|| {
                number
                    .as_f64()
                    .filter(|number| number.is_finite() && number.fract() == 0.0)
                    .map(|number| number as i128)
            })
            .ok_or_else(|| anyhow!("{field} must be an integer"))?
    } else {
        bail!("{field} must be an integer");
    };

    if integer < i128::from(min) || integer > i128::from(max) {
        bail!("{field} must be between {min} and {max}");
    }
    if integer < 0 {
        Ok(Some(Value::Number(Number::from(integer as i64))))
    } else {
        Ok(Some(Value::Number(Number::from(integer as u64))))
    }
}

fn build_single_search_params(input: &Map<String, Value>) -> Result<Map<String, Value>> {
    let price_min = clean_integer(input.get("price_min"), "price_min", 0, MAX_SAFE_INTEGER)?;
    let price_max = clean_integer(input.get("price_max"), "price_max", 0, MAX_SAFE_INTEGER)?;
    if let (Some(price_min), Some(price_max)) = (&price_min, &price_max) {
        if price_max.as_u64().unwrap_or(0) < price_min.as_u64().unwrap_or(0) {
            bail!("price_max cannot be less than price_min");
        }
    }

    let query = clean_required_string(input.get("query"), "query", MAX_QUERY_LENGTH)?;
    let page =
        clean_integer(input.get("page"), "page", 1, MAX_PAGE as u64)?.unwrap_or_else(|| json!(1));
    let location = clean_string(input.get("location"), "location", MAX_FILTER_LENGTH)?;
    let category = clean_string(input.get("category"), "category", MAX_FILTER_LENGTH)?;

    let mut params = Map::new();
    params.insert("query".to_owned(), Value::String(query));
    params.insert("page".to_owned(), page);
    if let Some(location) = location {
        params.insert("location".to_owned(), Value::String(location));
    }
    if let Some(category) = category {
        params.insert("category".to_owned(), Value::String(category));
    }
    if let Some(price_min) = price_min {
        params.insert("price_min".to_owned(), price_min);
    }
    if let Some(price_max) = price_max {
        params.insert("price_max".to_owned(), price_max);
    }
    Ok(params)
}

fn build_search_plan(input: &Value) -> Result<Vec<SearchPlanItem>> {
    let Some(input) = input.as_object() else {
        bail!("query is required");
    };
    let raw_searches = match input.get("searches") {
        None | Some(Value::Null) => vec![input],
        Some(Value::Array(searches)) => {
            if searches.is_empty() {
                bail!("searches must contain at least one search");
            }
            if searches.len() > MAX_BATCH_SEARCHES {
                bail!("searches cannot contain more than {MAX_BATCH_SEARCHES} search objects");
            }
            searches
                .iter()
                .enumerate()
                .map(|(index, search)| {
                    search
                        .as_object()
                        .ok_or_else(|| anyhow!("searches[{index}] must be an object"))
                })
                .collect::<Result<Vec<_>>>()?
        }
        Some(_) => bail!("searches must be an array"),
    };

    raw_searches
        .into_iter()
        .enumerate()
        .map(|(index, search)| {
            Ok(SearchPlanItem {
                index,
                params: build_single_search_params(search)?,
            })
        })
        .collect()
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(number) => number.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(number) => number
            .as_f64()
            .filter(|value| value.fract() == 0.0 && value.abs() < 1e21)
            .map(|value| {
                if value == 0.0 {
                    "0".to_owned()
                } else {
                    format!("{value:.0}")
                }
            })
            .unwrap_or_else(|| number.to_string()),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn describe_search_request(searches: &[SearchPlanItem]) -> String {
    if searches.len() != 1 {
        return format!("{} Kleinanzeigen searches", searches.len());
    }
    let params = searches.first().map(|search| &search.params);
    let query = params
        .and_then(|params| params.get("query"))
        .map(js_string)
        .unwrap_or_default();
    let location = params
        .and_then(|params| params.get("location"))
        .filter(|value| js_truthy(value))
        .map(|value| format!(" in {}", js_string(value)))
        .unwrap_or_default();
    let category = params
        .and_then(|params| params.get("category"))
        .filter(|value| js_truthy(value))
        .map(|value| format!(", category {}", js_string(value)))
        .unwrap_or_default();
    let page = params
        .and_then(|params| params.get("page"))
        .map(js_string)
        .unwrap_or_else(|| "1".to_owned());
    format!("\"{query}\"{location} (page {page}{category})")
}

const LISTING_SOURCES: [&str; 7] = [
    "data",
    "listings",
    "results",
    "items",
    "data.listings",
    "data.results",
    "data.items",
];

fn listing_array<'a>(response: &'a Value, source: &str) -> Option<&'a Vec<Value>> {
    match source {
        "data" => response.get("data")?.as_array(),
        "listings" | "results" | "items" => response.get(source)?.as_array(),
        "data.listings" | "data.results" | "data.items" => response
            .get("data")?
            .get(source.strip_prefix("data.")?)?
            .as_array(),
        _ => None,
    }
}

fn select_listings(response: &Value) -> (Vec<Value>, Option<&'static str>) {
    let candidates = LISTING_SOURCES
        .iter()
        .filter_map(|source| listing_array(response, source).map(|listings| (*source, listings)))
        .collect::<Vec<_>>();

    if let Some((source, listings)) = candidates.iter().find(|(_, listings)| !listings.is_empty()) {
        return (listings.to_vec(), Some(*source));
    }
    if let Some((source, listings)) = candidates.first() {
        return (listings.to_vec(), Some(*source));
    }
    eprintln!("Unexpected Kleinanzeigen response shape: expected \"data\", \"listings\", \"results\", or \"items\" array.");
    (Vec::new(), None)
}

fn value_or_null(object: &Value, key: &str) -> Value {
    object
        .get(key)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn build_dataset_item(listing: &Value, params: &Map<String, Value>, response: &Value) -> Value {
    let mut item = listing.as_object().cloned().unwrap_or_default();
    for field in [
        "id",
        "title",
        "url",
        "price",
        "price_numeric",
        "location",
        "description",
        "has_shipping",
    ] {
        item.insert(field.to_owned(), value_or_null(listing, field));
    }
    item.insert(
        "image_url".to_owned(),
        listing
            .get("image_url")
            .filter(|value| !value.is_null())
            .or_else(|| listing.get("image").filter(|value| !value.is_null()))
            .cloned()
            .unwrap_or(Value::Null),
    );
    for (request_field, output_field) in [
        ("query", "request_query"),
        ("page", "request_page"),
        ("location", "request_location"),
        ("category", "request_category"),
        ("price_min", "request_price_min"),
        ("price_max", "request_price_max"),
    ] {
        item.insert(
            output_field.to_owned(),
            params.get(request_field).cloned().unwrap_or(Value::Null),
        );
    }
    item.insert(
        "results_count".to_owned(),
        response
            .pointer("/meta/results_count")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    );
    Value::Object(item)
}

fn limit_search_response(response: &Value, limit: usize, selected_source: Option<&str>) -> Value {
    let mut limited = response.as_object().cloned().unwrap_or_default();
    match response.get("data") {
        Some(Value::Array(listings)) => {
            if selected_source == Some("data") {
                limited.insert(
                    "data".to_owned(),
                    Value::Array(listings.iter().take(limit).cloned().collect()),
                );
            } else {
                limited.remove("data");
            }
        }
        Some(Value::Object(data)) => {
            let mut limited_data = data.clone();
            for source in ["listings", "results", "items"] {
                if let Some(Value::Array(listings)) = data.get(source) {
                    let nested_source = format!("data.{source}");
                    if selected_source == Some(nested_source.as_str()) {
                        limited_data.insert(
                            source.to_owned(),
                            Value::Array(listings.iter().take(limit).cloned().collect()),
                        );
                    } else {
                        limited_data.remove(source);
                    }
                }
            }
            limited.insert("data".to_owned(), Value::Object(limited_data));
        }
        _ => {}
    }
    for source in ["listings", "results", "items"] {
        if let Some(Value::Array(listings)) = response.get(source) {
            if selected_source == Some(source) {
                limited.insert(
                    source.to_owned(),
                    Value::Array(listings.iter().take(limit).cloned().collect()),
                );
            } else {
                limited.remove(source);
            }
        }
    }
    Value::Object(limited)
}

#[derive(Debug)]
struct ScrappaApiError {
    status: u16,
    message: String,
}

impl fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.message
        )
    }
}

impl StdError for ScrappaApiError {}

#[derive(Debug)]
struct ScrappaTimeoutError {
    timeout: Duration,
}

impl fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            self.timeout.as_millis()
        )
    }
}

impl StdError for ScrappaTimeoutError {}

fn scrub_body(body: &str) -> String {
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn scrappa_error(status: u16, body: &str) -> ScrappaApiError {
    let fallback = StatusCode::from_u16(status)
        .ok()
        .and_then(|status| status.canonical_reason())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {status}"));
    let parsed = match serde_json::from_str::<Value>(body) {
        Ok(parsed) => parsed,
        Err(_) => {
            return ScrappaApiError {
                status,
                message: if body.is_empty() {
                    fallback
                } else {
                    scrub_body(body)
                },
            };
        }
    };
    let Some(object) = parsed.as_object() else {
        return ScrappaApiError {
            status,
            message: if parsed.is_null() {
                scrub_body(body)
            } else {
                fallback
            },
        };
    };

    let mut message = object
        .get("message")
        .filter(|value| !value.is_null())
        .map(js_string)
        .unwrap_or(fallback);
    if let Some(errors) = object.get("errors").filter(|value| js_truthy(value)) {
        let Some(errors) = errors.as_object() else {
            return ScrappaApiError {
                status,
                message: scrub_body(body),
            };
        };
        let mut details = Vec::with_capacity(errors.len());
        for (field, messages) in errors {
            let Some(messages) = messages.as_array() else {
                return ScrappaApiError {
                    status,
                    message: scrub_body(body),
                };
            };
            details.push(format!(
                "{field}: {}",
                messages
                    .iter()
                    .map(js_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details.join("; "));
        }
    }
    ScrappaApiError { status, message }
}

fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    for cause in error.chain() {
        if let Some(api_error) = cause.downcast_ref::<ScrappaApiError>() {
            return matches!(api_error.status, 408 | 429 | 500 | 502 | 503 | 504);
        }
        if cause.downcast_ref::<ScrappaTimeoutError>().is_some() {
            return true;
        }
        if let Some(request_error) = cause.downcast_ref::<reqwest::Error>() {
            return request_error.is_timeout() || request_error.is_connect();
        }
    }
    false
}

fn retry_delay(attempt: usize, jitter_ms: u64) -> Duration {
    let base_ms = 1_000_u64.saturating_mul(2_u64.saturating_pow(attempt as u32));
    Duration::from_millis(base_ms.saturating_add(jitter_ms).min(10_000))
}

fn retry_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::from(duration.subsec_nanos() % 1_000_000_000) / 1_000_000)
        .unwrap_or(0)
}

struct ScrappaClient<'a> {
    http: &'a Client,
    config: &'a Config,
    api_key: &'a str,
    timeout: Duration,
}

impl<'a> ScrappaClient<'a> {
    fn new(http: &'a Client, config: &'a Config, api_key: &'a str) -> Self {
        Self {
            http,
            config,
            api_key,
            timeout: SCRAPPA_REQUEST_TIMEOUT,
        }
    }

    async fn get(&self, params: &Map<String, Value>) -> Result<Value> {
        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send(params).await {
                Ok(response) => return Ok(response),
                Err(error)
                    if attempt < SCRAPPA_MAX_ATTEMPTS && is_retryable_scrappa_error(&error) =>
                {
                    let delay = retry_delay(attempt, retry_jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        SCRAPPA_MAX_ATTEMPTS,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("the retry loop always returns or continues for a bounded number of attempts")
    }

    async fn send(&self, params: &Map<String, Value>) -> Result<Value> {
        let mut url = endpoint_url(&self.config.scrappa_api_base, &["kleinanzeigen", "search"])?;
        for (key, value) in params {
            if value.is_null() || value.as_str() == Some("") || value == &Value::Bool(false) {
                continue;
            }
            let value = if value == &Value::Bool(true) {
                "1".to_owned()
            } else {
                js_string(value)
            };
            url.query_pairs_mut().append_pair(key, &value);
        }

        let response = self
            .http
            .get(url)
            .timeout(self.timeout)
            .header("X-API-Key", self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(
                header::USER_AGENT,
                "thescrappa-kleinanzeigen-search-scraper/1.0",
            )
            .send()
            .await
            .map_err(|error| scrappa_transport_error(error, self.timeout))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| scrappa_transport_error(error, self.timeout))?;
        if !status.is_success() {
            return Err(scrappa_error(status.as_u16(), &body).into());
        }
        serde_json::from_str(&body)
            .map_err(|error| anyhow!("Scrappa API response was not valid JSON: {error}"))
    }
}

fn scrappa_transport_error(error: reqwest::Error, timeout: Duration) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaTimeoutError { timeout }.into()
    } else {
        error.into()
    }
}

#[derive(Default)]
struct DatasetBudget {
    initial_listing_results: Option<u64>,
    saved_listing_results: u64,
    charge_attempts: u64,
}

fn affordable_listing_count(
    run: &Value,
    requested: usize,
    budget: &mut DatasetBudget,
) -> Result<usize> {
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

    let event_prices = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let listing_result_price = event_prices
        .get(LISTING_RESULT_CHARGE_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the listing-result event price"))?;
    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
    if !listing_result_price.is_finite()
        || listing_result_price < 0.0
        || !max_charge.is_finite()
        || max_charge < 0.0
    {
        bail!("Apify run returned invalid charging values");
    }

    let charged_counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let current_listing_results = charged_counts
        .get(LISTING_RESULT_CHARGE_EVENT)
        .map(|count| {
            count.as_u64().ok_or_else(|| {
                anyhow!("Invalid charged event count for {LISTING_RESULT_CHARGE_EVENT}")
            })
        })
        .transpose()?
        .unwrap_or(0);
    let initial_listing_results = *budget
        .initial_listing_results
        .get_or_insert(current_listing_results);
    let local_listing_results = initial_listing_results
        .checked_add(budget.saved_listing_results)
        .ok_or_else(|| anyhow!("Listing result count overflowed"))?;

    let mut spent = 0.0;
    let mut saw_listing_result_count = false;
    for (event_name, count) in charged_counts {
        let mut count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if event_name == LISTING_RESULT_CHARGE_EVENT {
            saw_listing_result_count = true;
            count = count.max(local_listing_results);
        }
        if count == 0 {
            continue;
        }

        let price = event_prices
            .get(event_name)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        spent += price * count as f64;
    }
    if !saw_listing_result_count && local_listing_results > 0 {
        spent += listing_result_price * local_listing_results as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if listing_result_price == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * listing_result_price <= max_charge + tolerance)
        .count())
}

fn is_pay_per_event(run: &Value) -> Result<bool> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    Ok(data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        == Some("PAY_PER_EVENT"))
}

fn next_charge_idempotency_key(run_id: &str, budget: &mut DatasetBudget) -> String {
    budget.charge_attempts = budget.charge_attempts.saturating_add(1);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!(
        "{run_id}-{LISTING_RESULT_CHARGE_EVENT}-{}-{timestamp}",
        budget.charge_attempts
    )
}

struct PushChargedListingsResult {
    saved_count: usize,
    charge_limit_reached: bool,
}

async fn push_charged_listings(
    apify: &ApifyClient<'_>,
    items: &[Value],
    budget: &mut DatasetBudget,
) -> Result<PushChargedListingsResult> {
    if items.is_empty() {
        return Ok(PushChargedListingsResult {
            saved_count: 0,
            charge_limit_reached: false,
        });
    }

    let run = apify.get_run().await?;
    if !is_pay_per_event(&run)? {
        apify.push_dataset_items(items).await?;
        return Ok(PushChargedListingsResult {
            saved_count: items.len(),
            charge_limit_reached: false,
        });
    }

    let saved_count = affordable_listing_count(&run, items.len(), budget)?;
    if saved_count == 0 {
        return Ok(PushChargedListingsResult {
            saved_count: 0,
            charge_limit_reached: true,
        });
    }
    let idempotency_key = next_charge_idempotency_key(&apify.config.actor_run_id, budget);
    apify.charge_event(saved_count, &idempotency_key).await?;
    apify.push_dataset_items(&items[..saved_count]).await?;
    budget.saved_listing_results = budget
        .saved_listing_results
        .checked_add(saved_count as u64)
        .ok_or_else(|| anyhow!("Listing result count overflowed"))?;

    Ok(PushChargedListingsResult {
        saved_count,
        charge_limit_reached: saved_count < items.len(),
    })
}

#[derive(Debug)]
struct ActorOutput {
    #[cfg(test)]
    value: Value,
    status_message: Option<String>,
}

async fn run_actor(http: &Client, config: &Config) -> Result<ActorOutput> {
    let api_key = config.scrappa_api_key.as_deref().ok_or_else(|| {
        anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
    })?;
    let apify = ApifyClient { http, config };
    let input = apify.get_input().await?;
    let searches = build_search_plan(&input)?;
    println!(
        "Searching Kleinanzeigen for {}",
        describe_search_request(&searches)
    );

    let scrappa = ScrappaClient::new(http, config, api_key);
    let mut responses = Vec::new();
    let mut saved_listings = 0;
    let mut status_message = None;
    let mut dataset_budget = DatasetBudget::default();

    for search in &searches {
        let run = apify.get_run().await?;
        if is_pay_per_event(&run)? && affordable_listing_count(&run, 1, &mut dataset_budget)? == 0 {
            let query = search
                .params
                .get("query")
                .map(js_string)
                .unwrap_or_default();
            let page = search.params.get("page").map(js_string).unwrap_or_default();
            let message = format!(
                "Charge limit reached before fetching Kleinanzeigen query {query} on page {page}."
            );
            println!(
                "{message} {}",
                json!({
                    "event": LISTING_RESULT_CHARGE_EVENT,
                    "query": search.params.get("query"),
                    "page": search.params.get("page"),
                })
            );
            status_message = Some(message);
            break;
        }

        let query = search
            .params
            .get("query")
            .map(js_string)
            .unwrap_or_default();
        let page = search.params.get("page").map(js_string).unwrap_or_default();
        println!("Fetching Kleinanzeigen query {query} on page {page}");

        let response = scrappa.get(&search.params).await?;
        let (listings, source) = select_listings(&response);
        let items = listings
            .iter()
            .map(|listing| build_dataset_item(listing, &search.params, &response))
            .collect::<Vec<_>>();
        let push_result = push_charged_listings(&apify, &items, &mut dataset_budget).await?;
        saved_listings += push_result.saved_count;
        responses.push(json!({
            "index": search.index,
            "request": search.params,
            "listings_saved": push_result.saved_count,
            "response": limit_search_response(&response, push_result.saved_count, source),
        }));

        println!(
            "Found {} listing(s); saved {}",
            items.len(),
            push_result.saved_count
        );
        if push_result.charge_limit_reached || push_result.saved_count < items.len() {
            let message = format!(
                "Charge limit reached after saving {} of {} Kleinanzeigen listing result(s) for query {}.",
                push_result.saved_count,
                items.len(),
                search
                    .params
                    .get("query")
                    .map(js_string)
                    .unwrap_or_default()
            );
            println!(
                "{message} {}",
                json!({
                    "event": LISTING_RESULT_CHARGE_EVENT,
                    "charged_count": push_result.saved_count,
                    "requested_count": items.len(),
                    "saved_count": push_result.saved_count,
                    "query": search.params.get("query"),
                    "page": search.params.get("page"),
                })
            );
            status_message = Some(message);
            break;
        }
    }

    let output = json!({
        "searches_requested": searches.len(),
        "searches_completed": responses.len(),
        "listings_extracted": saved_listings,
        "status_message": status_message,
        "responses": responses,
    });
    apify.put_output(&output).await?;
    println!("Kleinanzeigen search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "searches_requested": output["searches_requested"],
            "searches_completed": output["searches_completed"],
            "listings_extracted": output["listings_extracted"],
        })
    );
    Ok(ActorOutput {
        #[cfg(test)]
        value: output,
        status_message,
    })
}

fn actor_failure_message(error: &anyhow::Error) -> String {
    let raw_message = error.to_string();
    if error
        .chain()
        .any(|cause| cause.downcast_ref::<ScrappaTimeoutError>().is_some())
    {
        format!(
            "{raw_message}. The Kleinanzeigen search request exceeded the {}s Scrappa API timeout. Try fewer searches, narrower filters, or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        raw_message
    }
}

#[tokio::main]
async fn main() {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            std::process::exit(1);
        }
    };
    let http = match Client::builder().build() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            std::process::exit(1);
        }
    };

    match run_actor(&http, &config).await {
        Ok(output) => {
            if let Some(status_message) = output.status_message {
                let apify = ApifyClient {
                    http: &http,
                    config: &config,
                };
                if let Err(error) = apify.set_terminal_status_message(&status_message).await {
                    eprintln!("Actor failed: {error}");
                    std::process::exit(1);
                }
            }
        }
        Err(error) => {
            let message = actor_failure_message(&error);
            eprintln!("Actor failed: {message}");
            let apify = ApifyClient {
                http: &http,
                config: &config,
            };
            if let Err(status_error) = apify.set_terminal_status_message(&message).await {
                eprintln!("Could not set the terminal Actor status message: {status_error}");
            }
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        io,
        sync::{Arc, Mutex},
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        task::JoinHandle,
    };

    #[derive(Clone)]
    struct MockResponse {
        status: u16,
        body: String,
    }

    impl MockResponse {
        fn json(status: u16, body: Value) -> Self {
            Self {
                status,
                body: body.to_string(),
            }
        }

        fn text(status: u16, body: &str) -> Self {
            Self {
                status,
                body: body.to_owned(),
            }
        }
    }

    #[derive(Clone, Debug)]
    struct RecordedRequest {
        method: String,
        target: String,
        headers: HashMap<String, String>,
        body: String,
    }

    struct MockServer {
        base_url: String,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        handle: JoinHandle<()>,
    }

    impl MockServer {
        async fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let recorded_requests = Arc::clone(&requests);
            let handle = tokio::spawn(async move {
                for response in responses {
                    let Ok((mut stream, _)) = listener.accept().await else {
                        break;
                    };
                    let Ok(Some(request)) = read_request(&mut stream).await else {
                        break;
                    };
                    recorded_requests.lock().unwrap().push(request);
                    let reason = StatusCode::from_u16(response.status)
                        .ok()
                        .and_then(|status| status.canonical_reason())
                        .unwrap_or("Unknown");
                    let response_text = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    let _ = stream.write_all(response_text.as_bytes()).await;
                    let _ = stream.shutdown().await;
                }
            });
            Self {
                base_url: format!("http://{address}"),
                requests,
                handle,
            }
        }

        fn finish(self) -> Vec<RecordedRequest> {
            self.handle.abort();
            let requests = self.requests.lock().unwrap().clone();
            requests
        }
    }

    async fn read_request(stream: &mut TcpStream) -> io::Result<Option<RecordedRequest>> {
        let mut bytes = Vec::new();
        let mut buffer = [0; 1024];
        let header_end = loop {
            let read = stream.read(&mut buffer).await?;
            if read == 0 {
                return Ok(None);
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                break position + 4;
            }
        };
        let header_text = String::from_utf8_lossy(&bytes[..header_end]);
        let mut lines = header_text.split("\r\n");
        let Some(request_line) = lines.next() else {
            return Ok(None);
        };
        let mut request_parts = request_line.split_whitespace();
        let Some(method) = request_parts.next().map(str::to_owned) else {
            return Ok(None);
        };
        let Some(target) = request_parts.next().map(str::to_owned) else {
            return Ok(None);
        };
        let mut headers = HashMap::new();
        let mut content_length = 0;
        for line in lines.filter(|line| !line.is_empty()) {
            if let Some((name, value)) = line.split_once(':') {
                let name = name.trim().to_ascii_lowercase();
                let value = value.trim().to_owned();
                if name == "content-length" {
                    content_length = value.parse::<usize>().unwrap_or(0);
                }
                headers.insert(name, value);
            }
        }
        while bytes.len() < header_end + content_length {
            let read = stream.read(&mut buffer).await?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
        }
        let body = String::from_utf8_lossy(
            &bytes[header_end..bytes.len().min(header_end + content_length)],
        )
        .to_string();
        Ok(Some(RecordedRequest {
            method,
            target,
            headers,
            body,
        }))
    }

    fn test_config(base_url: &str) -> Config {
        Config {
            apify_api_base: Url::parse(base_url).unwrap(),
            scrappa_api_base: Url::parse(&format!("{base_url}/api")).unwrap(),
            apify_token: "test-token".to_owned(),
            key_value_store_id: "test-store".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: Some("scrappa-test-key".to_owned()),
        }
    }

    fn input_response(input: Value) -> MockResponse {
        MockResponse::json(200, input)
    }

    fn flat_pricing() -> MockResponse {
        MockResponse::json(
            200,
            json!({"data": {"pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM"}}}),
        )
    }

    fn ppe_pricing(max_charge: f64, charged: u64) -> MockResponse {
        MockResponse::json(
            200,
            json!({"data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "listing-result": {"eventPriceUsd": 0.1},
                        "apify-actor-start": {"eventPriceUsd": 0.05}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": {
                    "listing-result": charged,
                    "apify-actor-start": 0
                }
            }}),
        )
    }

    fn listing_response(count: usize) -> MockResponse {
        MockResponse::json(
            200,
            json!({
                "data": (0..count).map(|index| json!({
                    "id": format!("listing-{index}"),
                    "title": format!("Listing {index}"),
                    "image": "https://img.kleinanzeigen.de/example.jpg"
                })).collect::<Vec<_>>(),
                "meta": {"results_count": count}
            }),
        )
    }

    fn requests_to<'a>(requests: &'a [RecordedRequest], target: &str) -> Vec<&'a RecordedRequest> {
        requests
            .iter()
            .filter(|request| request.target.starts_with(target))
            .collect()
    }

    fn query_values(target: &str) -> Vec<(String, String)> {
        Url::parse(&format!("http://localhost{target}"))
            .unwrap()
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect()
    }

    #[test]
    fn normalizes_single_and_batch_searches() {
        let plan = build_search_plan(&json!({
            "query": " e-bike%20fully ",
            "page": "2",
            "location": " Berlin ",
            "category": " elektronik ",
            "price_min": "50",
            "price_max": 500
        }))
        .unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].index, 0);
        assert_eq!(plan[0].params["query"], "e-bike fully");
        assert_eq!(plan[0].params["page"], 2);
        assert_eq!(plan[0].params["price_min"], 50);
        assert_eq!(plan[0].params["price_max"], 500);
        assert_eq!(
            describe_search_request(&plan),
            "\"e-bike fully\" in Berlin (page 2, category elektronik)"
        );
        assert_eq!(
            plan[0]
                .params
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "query",
                "page",
                "location",
                "category",
                "price_min",
                "price_max"
            ]
        );

        let batch = build_search_plan(&json!({
            "query": "ignored",
            "searches": [
                {"query": "iphone", "location": "Berlin"},
                {"query": "fahrrad", "location": "Hamburg", "price_max": "500"}
            ]
        }))
        .unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].params["query"], "iphone");
        assert_eq!(batch[1].params["query"], "fahrrad");
        assert_eq!(batch[1].params["page"], 1);
        assert_eq!(batch[1].params["price_max"], 500);
        assert_eq!(describe_search_request(&batch), "2 Kleinanzeigen searches");
    }

    #[test]
    fn validates_inputs_and_decodes_percent_encoding_like_uri_component() {
        assert!(build_search_plan(&json!({}))
            .unwrap_err()
            .to_string()
            .contains("query is required"));
        assert!(build_search_plan(&json!({"query": ""}))
            .unwrap_err()
            .to_string()
            .contains("query is required"));
        assert!(build_search_plan(&json!({"query": "iphone", "page": 101}))
            .unwrap_err()
            .to_string()
            .contains("page must be between 1 and 100"));
        assert!(
            build_search_plan(&json!({"query": "iphone", "location": 123}))
                .unwrap_err()
                .to_string()
                .contains("location must be a string")
        );
        assert!(
            build_search_plan(&json!({"query": "iphone", "price_min": "10.5"}))
                .unwrap_err()
                .to_string()
                .contains("price_min must be an integer")
        );
        assert!(
            build_search_plan(&json!({"query": "iphone", "price_min": 500, "price_max": 50}))
                .unwrap_err()
                .to_string()
                .contains("price_max cannot be less than price_min")
        );
        assert!(build_search_plan(&json!({"query": "😀".repeat(251)}))
            .unwrap_err()
            .to_string()
            .contains("query must be 500 characters or fewer"));
        assert_eq!(
            build_search_plan(&json!({"query": "\u{FEFF}iphone\u{FEFF}"})).unwrap()[0].params
                ["query"],
            "iphone"
        );
        assert!(
            build_search_plan(&json!({"query": "iphone", "searches": []}))
                .unwrap_err()
                .to_string()
                .contains("searches must contain at least one search")
        );
        assert!(build_search_plan(&json!({"searches": ["iphone"]}))
            .unwrap_err()
            .to_string()
            .contains("searches[0] must be an object"));
        assert!(build_search_plan(
            &json!({"searches": (0..26).map(|_| json!({"query": "x"})).collect::<Vec<_>>()})
        )
        .unwrap_err()
        .to_string()
        .contains("searches cannot contain more than 25"));
        assert!(
            build_search_plan(&json!({"query": "iphone", "searches": "nope"}))
                .unwrap_err()
                .to_string()
                .contains("searches must be an array")
        );
        assert_eq!(decode_input_string("100% baumwolle"), "100% baumwolle");
        assert_eq!(decode_input_string("shoe%2"), "shoe%2");
        assert_eq!(decode_input_string("invalid%FF"), "invalid%FF");
        assert_eq!(decode_input_string("a+b%20c"), "a+b c");
    }

    #[test]
    fn selects_and_limits_listing_response_shapes() {
        for (response, expected_source) in [
            (json!({"data": [{"id": "data"}]}), "data"),
            (json!({"listings": [{"id": "listings"}]}), "listings"),
            (json!({"results": [{"id": "results"}]}), "results"),
            (json!({"items": [{"id": "items"}]}), "items"),
            (
                json!({"data": {"listings": [{"id": "nested-listings"}]}}),
                "data.listings",
            ),
            (
                json!({"data": {"results": [{"id": "nested-results"}]}}),
                "data.results",
            ),
            (
                json!({"data": {"items": [{"id": "nested-items"}]}}),
                "data.items",
            ),
        ] {
            let (_, source) = select_listings(&response);
            assert_eq!(source, Some(expected_source));
        }
        assert_eq!(select_listings(&json!({})), (Vec::new(), None));

        let (listings, source) = select_listings(&json!({
            "data": [],
            "listings": [],
            "results": [{"id": "result"}],
            "items": [{"id": "item"}],
            "data_extra": [{"id": "ignored"}]
        }));
        assert_eq!(source, Some("results"));
        assert_eq!(listings, vec![json!({"id": "result"})]);

        let (nested, source) = select_listings(&json!({
            "listings": [],
            "data": {"listings": [{"id": "nested"}]}
        }));
        assert_eq!(source, Some("data.listings"));
        assert_eq!(nested, vec![json!({"id": "nested"})]);

        let params = build_search_plan(&json!({
            "query": "iphone", "page": 2, "location": "Berlin", "price_min": 500
        }))
        .unwrap()
        .remove(0)
        .params;
        let response = json!({
            "data": {
                "cursor": "next-page",
                "listings": [{"id": "a"}, {"id": "b"}],
                "results": [{"id": "ignored"}]
            },
            "listings": [{"id": "top"}],
            "meta": {"results_count": 26}
        });
        let item = build_dataset_item(
            &json!({"id": "listing-1", "image": "image.jpg", "extra": true}),
            &params,
            &response,
        );
        assert_eq!(item["id"], "listing-1");
        assert_eq!(item["image_url"], "image.jpg");
        assert_eq!(item["request_query"], "iphone");
        assert_eq!(item["request_page"], 2);
        assert_eq!(item["request_location"], "Berlin");
        assert_eq!(item["request_category"], Value::Null);
        assert_eq!(item["request_price_min"], 500);
        assert_eq!(item["results_count"], 26);
        assert_eq!(item["extra"], true);
        assert_eq!(
            limit_search_response(&response, 1, Some("data.listings")),
            json!({
                "data": {"cursor": "next-page", "listings": [{"id": "a"}]},
                "meta": {"results_count": 26}
            })
        );
    }

    #[test]
    fn preserves_retryable_statuses_timeout_deadline_and_error_text() {
        for status in [408, 429, 500, 502, 503, 504] {
            let error: anyhow::Error = scrappa_error(status, "busy").into();
            assert!(is_retryable_scrappa_error(&error));
        }
        for status in [400, 401, 403, 404] {
            let error: anyhow::Error = scrappa_error(status, "bad request").into();
            assert!(!is_retryable_scrappa_error(&error));
        }
        assert_eq!(retry_delay(1, 999), Duration::from_millis(2_999));
        assert_eq!(retry_delay(2, 999), Duration::from_millis(4_999));
        assert_eq!(retry_delay(5, 999), Duration::from_millis(10_000));
        assert!(is_retryable_scrappa_error(&anyhow!(ScrappaTimeoutError {
            timeout: SCRAPPA_REQUEST_TIMEOUT
        })));
        assert_eq!(
            scrappa_error(
                422,
                r#"{"message":"Invalid input","errors":{"query":["is required","is too long"]}}"#
            )
            .to_string(),
            "Scrappa API error (422): Invalid input - query: is required, is too long"
        );
        assert_eq!(
            scrappa_error(503, " unavailable \n now ").to_string(),
            "Scrappa API error (503): unavailable now"
        );
        assert_eq!(
            scrappa_error(400, "").to_string(),
            "Scrappa API error (400): Bad Request"
        );
        assert_eq!(
            scrappa_error(503, "[]").to_string(),
            "Scrappa API error (503): Service Unavailable"
        );
        assert_eq!(
            scrappa_error(503, "null").to_string(),
            "Scrappa API error (503): null"
        );
        let error: anyhow::Error = ScrappaTimeoutError {
            timeout: SCRAPPA_REQUEST_TIMEOUT,
        }
        .into();
        let message = actor_failure_message(&error);
        assert!(message.contains("timed out after 90000ms"));
        assert!(message.contains("exceeded the 90s Scrappa API timeout"));
        assert!(message.contains("Try fewer searches, narrower filters"));
    }

    #[test]
    fn limits_ppe_to_budget_after_all_charged_events() {
        let run = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "listing-result": {"eventPriceUsd": 0.1},
                "apify-actor-start": {"eventPriceUsd": 0.05}
            }}},
            "options": {"maxTotalChargeUsd": 0.25},
            "chargedEventCounts": {"listing-result": 1, "apify-actor-start": 1}
        }});
        let mut budget = DatasetBudget::default();
        assert_eq!(affordable_listing_count(&run, 5, &mut budget).unwrap(), 1);
        budget.saved_listing_results = 2;
        assert_eq!(affordable_listing_count(&run, 5, &mut budget).unwrap(), 0);
    }

    #[tokio::test]
    async fn non_ppe_run_preserves_pagination_auth_and_dataset_kv_output() {
        let server = MockServer::start(vec![
            input_response(json!({"query": "iphone case", "page": 3, "location": "Berlin"})),
            flat_pricing(),
            listing_response(2),
            flat_pricing(),
            MockResponse::json(201, json!({})),
            MockResponse::json(200, json!({})),
        ])
        .await;
        let config = test_config(&server.base_url);
        let client = Client::new();
        let output = run_actor(&client, &config).await.unwrap();
        assert_eq!(output.value["searches_requested"], 1);
        assert_eq!(output.value["searches_completed"], 1);
        assert_eq!(output.value["listings_extracted"], 2);
        assert_eq!(
            output.value["responses"][0]["response"]["data"]
                .as_array()
                .unwrap()
                .len(),
            2
        );

        let requests = server.finish();
        assert_eq!(
            requests.first().unwrap().target,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        let scrappa = requests_to(&requests, "/api/kleinanzeigen/search");
        assert_eq!(scrappa.len(), 1);
        assert_eq!(
            scrappa[0].headers.get("x-api-key").unwrap(),
            "scrappa-test-key"
        );
        assert_eq!(
            scrappa[0].headers.get("user-agent").unwrap(),
            "thescrappa-kleinanzeigen-search-scraper/1.0"
        );
        let query = query_values(&scrappa[0].target);
        assert!(query.contains(&("query".to_owned(), "iphone case".to_owned())));
        assert!(query.contains(&("page".to_owned(), "3".to_owned())));
        assert!(query.contains(&("location".to_owned(), "Berlin".to_owned())));
        let dataset = requests_to(&requests, "/v2/datasets/test-dataset/items");
        assert_eq!(dataset.len(), 1);
        let rows = serde_json::from_str::<Value>(&dataset[0].body).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 2);
        assert_eq!(rows[0]["request_page"], 3);
        assert_eq!(
            rows[0]["image_url"],
            "https://img.kleinanzeigen.de/example.jpg"
        );
        let output_write = requests_to(&requests, "/v2/key-value-stores/test-store/records/OUTPUT");
        assert_eq!(output_write.len(), 1);
        assert_eq!(
            serde_json::from_str::<Value>(&output_write[0].body).unwrap(),
            output.value
        );
        assert!(requests
            .iter()
            .filter(|request| !request.target.starts_with("/api/"))
            .all(
                |request| request.headers.get("authorization").map(String::as_str)
                    == Some("Bearer test-token")
            ));
        assert!(requests_to(&requests, "/v2/actor-runs/test-run/charge").is_empty());
    }

    #[tokio::test]
    async fn ppe_charges_only_affordable_rows_before_writing_dataset_and_terminal_status() {
        let server = MockServer::start(vec![
            input_response(json!({"query": "iphone"})),
            ppe_pricing(0.25, 0),
            listing_response(3),
            ppe_pricing(0.25, 0),
            MockResponse::json(201, json!({})),
            MockResponse::json(201, json!({})),
            MockResponse::json(200, json!({})),
            MockResponse::json(200, json!({})),
        ])
        .await;
        let config = test_config(&server.base_url);
        let client = Client::new();
        let output = run_actor(&client, &config).await.unwrap();
        assert_eq!(output.value["listings_extracted"], 2);
        assert!(output
            .status_message
            .as_deref()
            .unwrap()
            .contains("saving 2 of 3"));
        assert_eq!(
            output.value["responses"][0]["response"]["data"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        ApifyClient {
            http: &client,
            config: &config,
        }
        .set_terminal_status_message(output.status_message.as_deref().unwrap())
        .await
        .unwrap();

        let requests = server.finish();
        let charge = requests_to(&requests, "/v2/actor-runs/test-run/charge");
        assert_eq!(charge.len(), 1);
        assert_eq!(
            serde_json::from_str::<Value>(&charge[0].body).unwrap(),
            json!({"eventName": "listing-result", "count": 2})
        );
        let key = charge[0].headers.get("idempotency-key").unwrap();
        assert!(key.starts_with("test-run-listing-result-1-"));
        let charge_position = requests
            .iter()
            .position(|request| request.target.starts_with("/v2/actor-runs/test-run/charge"))
            .unwrap();
        let dataset_position = requests
            .iter()
            .position(|request| {
                request
                    .target
                    .starts_with("/v2/datasets/test-dataset/items")
            })
            .unwrap();
        assert!(charge_position < dataset_position);
        let dataset = requests_to(&requests, "/v2/datasets/test-dataset/items");
        assert_eq!(
            serde_json::from_str::<Value>(&dataset[0].body)
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let terminal = requests_to(&requests, "/v2/actor-runs/test-run");
        assert_eq!(terminal.last().unwrap().method, "PUT");
        let terminal_body = serde_json::from_str::<Value>(&terminal.last().unwrap().body).unwrap();
        assert_eq!(
            terminal_body["statusMessage"],
            output.status_message.unwrap()
        );
        assert_eq!(terminal_body["isStatusMessageTerminal"], true);
    }

    #[tokio::test]
    async fn zero_ppe_budget_skips_fetch_but_writes_output_and_terminal_message() {
        let server = MockServer::start(vec![
            input_response(json!({"query": "iphone", "page": 4})),
            ppe_pricing(0.05, 0),
            MockResponse::json(200, json!({})),
            MockResponse::json(200, json!({})),
        ])
        .await;
        let config = test_config(&server.base_url);
        let client = Client::new();
        let output = run_actor(&client, &config).await.unwrap();
        assert_eq!(output.value["searches_completed"], 0);
        assert_eq!(output.value["listings_extracted"], 0);
        assert!(output
            .status_message
            .as_deref()
            .unwrap()
            .contains("before fetching"));
        ApifyClient {
            http: &client,
            config: &config,
        }
        .set_terminal_status_message(output.status_message.as_deref().unwrap())
        .await
        .unwrap();

        let requests = server.finish();
        assert!(requests_to(&requests, "/api/kleinanzeigen/search").is_empty());
        assert!(requests_to(&requests, "/v2/actor-runs/test-run/charge").is_empty());
        assert_eq!(
            requests_to(&requests, "/v2/key-value-stores/test-store/records/OUTPUT").len(),
            1
        );
    }

    #[tokio::test]
    async fn retries_transient_scrappa_errors_but_auth_errors_fail_without_output() {
        let server = MockServer::start(vec![
            input_response(json!({"query": "iphone"})),
            flat_pricing(),
            MockResponse::json(503, json!({"message": "temporarily unavailable"})),
            listing_response(0),
            flat_pricing(),
            MockResponse::json(200, json!({})),
        ])
        .await;
        let config = test_config(&server.base_url);
        let output = run_actor(&Client::new(), &config).await.unwrap();
        assert_eq!(output.value["searches_completed"], 1);
        let requests = server.finish();
        let scrappa = requests_to(&requests, "/api/kleinanzeigen/search");
        assert_eq!(scrappa.len(), 2);
        assert_eq!(scrappa[0].target, scrappa[1].target);
        assert_eq!(
            requests_to(&requests, "/v2/datasets/test-dataset/items").len(),
            0
        );

        let server = MockServer::start(vec![
            input_response(json!({"query": "iphone"})),
            flat_pricing(),
            MockResponse::json(403, json!({"message": "forbidden"})),
        ])
        .await;
        let config = test_config(&server.base_url);
        let error = run_actor(&Client::new(), &config).await.unwrap_err();
        assert!(error
            .to_string()
            .contains("Scrappa API error (403): forbidden"));
        let requests = server.finish();
        assert_eq!(requests_to(&requests, "/api/kleinanzeigen/search").len(), 1);
        assert!(
            requests_to(&requests, "/v2/key-value-stores/test-store/records/OUTPUT").is_empty()
        );
    }

    #[tokio::test]
    async fn scrappa_request_deadline_is_enforced() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let _ = read_request(&mut stream).await;
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
        let config = test_config(&format!("http://{address}"));
        let client = Client::new();
        let mut scrappa = ScrappaClient::new(&client, &config, "scrappa-test-key");
        scrappa.timeout = Duration::from_millis(5);
        let params = build_search_plan(&json!({"query": "iphone"}))
            .unwrap()
            .remove(0)
            .params;
        let error = scrappa.send(&params).await.unwrap_err();
        assert!(error
            .downcast_ref::<ScrappaTimeoutError>()
            .is_some_and(|error| error.timeout == Duration::from_millis(5)));
        assert!(is_retryable_scrappa_error(&error));
        server.abort();
    }

    #[tokio::test]
    async fn missing_scrappa_key_fails_before_loading_input() {
        let server = MockServer::start(Vec::new()).await;
        let mut config = test_config(&server.base_url);
        config.scrappa_api_key = None;
        let error = run_actor(&Client::new(), &config).await.unwrap_err();
        assert!(error.to_string().contains(
            "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."
        ));
        assert!(server.finish().is_empty());
    }

    #[tokio::test]
    async fn dataset_failure_after_ppe_charge_does_not_write_output() {
        let server = MockServer::start(vec![
            input_response(json!({"query": "iphone"})),
            ppe_pricing(1.0, 0),
            listing_response(1),
            ppe_pricing(1.0, 0),
            MockResponse::json(201, json!({})),
            MockResponse::text(500, "dataset unavailable"),
        ])
        .await;
        let config = test_config(&server.base_url);
        let error = run_actor(&Client::new(), &config).await.unwrap_err();
        assert!(error
            .to_string()
            .contains("Apify dataset write failed with 500"));
        let requests = server.finish();
        assert_eq!(
            requests_to(&requests, "/v2/actor-runs/test-run/charge").len(),
            1
        );
        assert_eq!(
            requests_to(&requests, "/v2/datasets/test-dataset/items").len(),
            1
        );
        assert!(
            requests_to(&requests, "/v2/key-value-stores/test-store/records/OUTPUT").is_empty()
        );
    }
}
