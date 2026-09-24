use std::{
    collections::HashMap,
    env,
    process::ExitCode,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Number, Value};
use tokio::time::{sleep, timeout};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const APIFY_MAX_RETRIES: usize = 2;
const PROPERTY_RESULT_CHARGE_EVENT: &str = "property-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const DEFAULT_LOCATION: &str = "1276003001";
const DEFAULT_TYPE: &str = "apartment-rent";
const MAX_LOCATION_LENGTH: usize = 120;
const MAX_PER_PAGE: i64 = 50;
const PROPERTY_TYPES: [&str; 4] = ["apartment-rent", "apartment-buy", "house-rent", "house-buy"];

#[derive(Clone)]
struct Config {
    apify_api_base: String,
    apify_token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_base: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env("SCRAPPA_API_KEY").context(
            "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.",
        )?;
        Ok(Self {
            apify_api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_base: env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
            scrappa_api_key,
        })
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

#[derive(Debug, Clone, PartialEq)]
struct SearchParams {
    location: String,
    property_type: String,
    price_min: Option<Number>,
    price_max: Option<Number>,
    rooms_min: Option<Number>,
    rooms_max: Option<Number>,
    size_min: Option<Number>,
    size_max: Option<Number>,
    page: i64,
    per_page: i64,
}

impl SearchParams {
    fn query_pairs(&self) -> Vec<(String, String)> {
        let mut params = vec![
            ("location".to_owned(), self.location.clone()),
            ("type".to_owned(), self.property_type.clone()),
        ];
        push_number_pair(&mut params, "price_min", &self.price_min);
        push_number_pair(&mut params, "price_max", &self.price_max);
        push_number_pair(&mut params, "rooms_min", &self.rooms_min);
        push_number_pair(&mut params, "rooms_max", &self.rooms_max);
        push_number_pair(&mut params, "size_min", &self.size_min);
        push_number_pair(&mut params, "size_max", &self.size_max);
        params.push(("page".to_owned(), self.page.to_string()));
        params.push(("per_page".to_owned(), self.per_page.to_string()));
        params
    }

    fn describe(&self) -> String {
        let filters = [
            describe_range("price", &self.price_min, &self.price_max),
            describe_range("rooms", &self.rooms_min, &self.rooms_max),
            describe_range("size", &self.size_min, &self.size_max),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let filter_text = if filters.is_empty() {
            String::new()
        } else {
            format!(", {}", filters.join(", "))
        };
        format!(
            "{} properties in {} (page {}, per_page {}{})",
            self.property_type, self.location, self.page, self.per_page, filter_text
        )
    }

    fn as_value(&self) -> Value {
        let mut params = Map::new();
        params.insert("location".to_owned(), Value::String(self.location.clone()));
        params.insert("type".to_owned(), Value::String(self.property_type.clone()));
        insert_optional_number(&mut params, "price_min", &self.price_min);
        insert_optional_number(&mut params, "price_max", &self.price_max);
        insert_optional_number(&mut params, "rooms_min", &self.rooms_min);
        insert_optional_number(&mut params, "rooms_max", &self.rooms_max);
        insert_optional_number(&mut params, "size_min", &self.size_min);
        insert_optional_number(&mut params, "size_max", &self.size_max);
        params.insert("page".to_owned(), json!(self.page));
        params.insert("per_page".to_owned(), json!(self.per_page));
        Value::Object(params)
    }
}

fn normalize_search_input(input: Option<&Value>) -> Result<SearchParams> {
    let input_object = input.and_then(Value::as_object);
    let has_known_input = input_object.is_some_and(|object| {
        [
            "location",
            "type",
            "price_min",
            "price_max",
            "rooms_min",
            "rooms_max",
            "size_min",
            "size_max",
            "per_page",
            "property_type",
            "page",
            "limit",
        ]
        .iter()
        .any(|key| object.contains_key(*key))
    });

    let mut normalized = json!({
        "location": DEFAULT_LOCATION,
        "type": DEFAULT_TYPE,
        "page": 1,
        "per_page": 20,
    });
    if !has_known_input {
        return build_search_params(&normalized);
    }

    let object = input_object.expect("known input requires an object");
    let fields = [
        ("location", None),
        ("type", Some("property_type")),
        ("price_min", None),
        ("price_max", None),
        ("rooms_min", None),
        ("rooms_max", None),
        ("size_min", None),
        ("size_max", None),
        ("page", None),
        ("per_page", Some("limit")),
    ];
    let normalized_object = normalized
        .as_object_mut()
        .expect("default input is a JSON object");
    for (field, alias) in fields {
        let value = object.get(field).or_else(|| {
            alias.and_then(|alias| {
                (!object.contains_key(field))
                    .then(|| object.get(alias))
                    .flatten()
            })
        });
        if let Some(value) = value {
            normalized_object.insert(field.to_owned(), trim_input_value(value));
        }
    }

    build_search_params(&normalized)
}

fn trim_input_value(value: &Value) -> Value {
    match value {
        Value::String(value) => Value::String(value.trim().to_owned()),
        _ => value.clone(),
    }
}

fn build_search_params(input: &Value) -> Result<SearchParams> {
    let location = clean_required_string(input.get("location"), "location", MAX_LOCATION_LENGTH)?;
    let property_type = clean_enum_string(input.get("type"), "type", &PROPERTY_TYPES)?;
    let page = clean_integer(input.get("page"), "page", 1, 10_000)?;
    let per_page = clean_integer(input.get("per_page"), "per_page", 1, MAX_PER_PAGE)?;
    let price_min = optional_integer(input.get("price_min"), "price_min", 0, 100_000_000)?;
    let price_max = optional_integer(input.get("price_max"), "price_max", 0, 100_000_000)?;
    let rooms_min = optional_number(input.get("rooms_min"), "rooms_min", 0.0, 100.0)?;
    let rooms_max = optional_number(input.get("rooms_max"), "rooms_max", 0.0, 100.0)?;
    let size_min = optional_integer(input.get("size_min"), "size_min", 0, 1_000_000)?;
    let size_max = optional_integer(input.get("size_max"), "size_max", 0, 1_000_000)?;
    validate_min_max("price_min", &price_min, "price_max", &price_max)?;
    validate_min_max("rooms_min", &rooms_min, "rooms_max", &rooms_max)?;
    validate_min_max("size_min", &size_min, "size_max", &size_max)?;

    Ok(SearchParams {
        location,
        property_type,
        price_min,
        price_max,
        rooms_min,
        rooms_max,
        size_min,
        size_max,
        page,
        per_page,
    })
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    let Some(Value::String(value)) = value else {
        bail!("{field} must be a string");
    };
    let value = value.trim();
    if value.is_empty() {
        bail!("{field} is required");
    }
    if value.chars().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(value.to_owned())
}

fn clean_enum_string(value: Option<&Value>, field: &str, allowed: &[&str]) -> Result<String> {
    let value = clean_required_string(value, field, 40)?;
    if !allowed.contains(&value.as_str()) {
        bail!("{field} must be one of: {}", allowed.join(", "));
    }
    Ok(value)
}

fn optional_integer(
    value: Option<&Value>,
    field: &str,
    min: i64,
    max: i64,
) -> Result<Option<Number>> {
    let Some(value) = nonempty_optional_value(value) else {
        return Ok(None);
    };
    let number = clean_number_value(value, field, true)?;
    if number.fract() != 0.0 {
        bail!("{field} must be an integer");
    }
    if number < min as f64 || number > max as f64 {
        bail!("{field} must be between {min} and {max}");
    }
    Ok(Some(Number::from(number as i64)))
}

fn optional_number(
    value: Option<&Value>,
    field: &str,
    min: f64,
    max: f64,
) -> Result<Option<Number>> {
    let Some(value) = nonempty_optional_value(value) else {
        return Ok(None);
    };
    let number = clean_number_value(value, field, false)?;
    if number < min || number > max {
        bail!(
            "{field} must be between {} and {}",
            format_number(min),
            format_number(max)
        );
    }
    let number = if number.fract() == 0.0 {
        Number::from(number as i64)
    } else {
        Number::from_f64(number).ok_or_else(|| anyhow!("{field} must be a number"))?
    };
    Ok(Some(number))
}

fn nonempty_optional_value(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| !value.is_null() && value.as_str() != Some(""))
}

fn clean_integer(value: Option<&Value>, field: &str, min: i64, max: i64) -> Result<i64> {
    let Some(value) = value else {
        bail!("{field} must be an integer");
    };
    let number = clean_number_value(value, field, true)?;
    if number.fract() != 0.0 {
        bail!("{field} must be an integer");
    }
    if number < min as f64 || number > max as f64 {
        bail!("{field} must be between {min} and {max}");
    }
    Ok(number as i64)
}

fn clean_number_value(value: &Value, field: &str, integer: bool) -> Result<f64> {
    let number = match value {
        Value::Number(number) => number.as_f64(),
        Value::String(value) => {
            let value = value.trim();
            let valid = if integer {
                valid_integer_string(value)
            } else {
                valid_decimal_string(value)
            };
            if valid {
                value.parse::<f64>().ok()
            } else {
                None
            }
        }
        _ => None,
    };
    let Some(number) = number.filter(|number| number.is_finite()) else {
        bail!(
            "{field} must be a {}",
            if integer { "integer" } else { "number" }
        );
    };
    Ok(number)
}

fn valid_integer_string(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_decimal_string(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    if value.is_empty() {
        return false;
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fractional = parts.next();
    !whole.is_empty()
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && parts.next().is_none()
        && fractional.is_none_or(|digits| {
            !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn validate_min_max(
    min_field: &str,
    min: &Option<Number>,
    max_field: &str,
    max: &Option<Number>,
) -> Result<()> {
    if let (Some(min), Some(max)) = (min, max) {
        if min.as_f64().unwrap_or_default() > max.as_f64().unwrap_or_default() {
            bail!("{min_field} must be less than or equal to {max_field}");
        }
    }
    Ok(())
}

fn push_number_pair(params: &mut Vec<(String, String)>, key: &str, value: &Option<Number>) {
    if let Some(value) = value {
        params.push((
            key.to_owned(),
            format_number(value.as_f64().unwrap_or_default()),
        ));
    }
}

fn insert_optional_number(params: &mut Map<String, Value>, key: &str, value: &Option<Number>) {
    params.insert(
        key.to_owned(),
        value.clone().map(Value::Number).unwrap_or(Value::Null),
    );
}

fn describe_range(label: &str, min: &Option<Number>, max: &Option<Number>) -> Option<String> {
    let min = min.as_ref().and_then(Number::as_f64);
    let max = max.as_ref().and_then(Number::as_f64);
    match (min, max) {
        (None, None) => None,
        (Some(min), Some(max)) => Some(format!(
            "{label} {}-{}",
            format_number(min),
            format_number(max)
        )),
        (Some(min), None) => Some(format!("{label} >= {}", format_number(min))),
        (None, Some(max)) => Some(format!("{label} <= {}", format_number(max))),
    }
}

fn format_number(number: f64) -> String {
    if number.fract() == 0.0 {
        format!("{number:.0}")
    } else {
        number.to_string()
    }
}

#[derive(Debug)]
enum ScrappaError {
    Api { status: u16, message: String },
    Timeout,
    Request(String),
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Api { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::Timeout => write!(
                formatter,
                "Scrappa API request timed out after {}ms",
                SCRAPPA_REQUEST_TIMEOUT.as_millis()
            ),
            Self::Request(message) => formatter.write_str(message),
        }
    }
}

struct ScrappaClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    fn new(http: Client, base_url: String, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    async fn search(&self, params: &SearchParams) -> std::result::Result<Value, ScrappaError> {
        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send_search(params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt >= SCRAPPA_MAX_ATTEMPTS || !is_retryable_scrappa_error(&error) {
                        return Err(error);
                    }
                    let delay = scrappa_retry_delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {}ms.",
                        attempt + 1,
                        delay.as_millis()
                    );
                    sleep(delay).await;
                }
            }
        }
        unreachable!("the retry loop returns on success or final failure")
    }

    async fn send_search(&self, params: &SearchParams) -> std::result::Result<Value, ScrappaError> {
        let mut url = endpoint_url(&self.base_url, &["immobilienscout24", "search"])
            .map_err(|error| ScrappaError::Request(error.to_string()))?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params.query_pairs() {
                query.append_pair(&key, &value);
            }
        }
        eprintln!("[Scrappa] GET {url}");
        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .header(
                header::USER_AGENT,
                "thescrappa-immobilienscout24-search-scraper/1.0",
            )
            .send()
            .await
            .map_err(scrappa_transport_error)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let fallback = response
                .status()
                .canonical_reason()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("HTTP {status}"));
            let body = match response.text().await {
                Ok(body) => body,
                Err(error) if error.is_timeout() => return Err(ScrappaError::Timeout),
                Err(_) => String::new(),
            };
            return Err(ScrappaError::Api {
                status,
                message: scrappa_error_message(&body, &fallback),
            });
        }

        response
            .json::<Value>()
            .await
            .map_err(scrappa_transport_error)
    }
}

fn scrappa_transport_error(error: reqwest::Error) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout
    } else {
        ScrappaError::Request(error.to_string())
    }
}

fn is_retryable_scrappa_error(error: &ScrappaError) -> bool {
    match error {
        ScrappaError::Timeout => true,
        ScrappaError::Api { status, .. } => {
            matches!(*status, 408 | 429 | 500 | 502 | 503 | 504)
        }
        ScrappaError::Request(_) => false,
    }
}

fn scrappa_retry_delay(failed_attempt: usize) -> Duration {
    let jitter_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64;
    let base_ms = 1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt as u32));
    Duration::from_millis((base_ms + jitter_ms).min(10_000))
}

fn scrappa_error_message(body: &str, fallback: &str) -> String {
    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let message = error_data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(fallback);
        let mut result = message.to_owned();
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let messages = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .map(value_to_js_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {messages}")
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                result.push_str(" - ");
                result.push_str(&details);
            }
        }
        return result;
    }

    let body = body.trim();
    if body.is_empty() {
        fallback.to_owned()
    } else {
        body.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(500)
            .collect()
    }
}

fn value_to_js_string(value: &Value) -> String {
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
                    value_to_js_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn is_handled_empty_search_error(error: &ScrappaError) -> bool {
    let ScrappaError::Api { status, message } = error else {
        return false;
    };
    if *status == 502 {
        return true;
    }
    if *status != 400 {
        return false;
    }
    let message = message.to_ascii_lowercase();
    message.contains("invalid_location")
        || message
            .split_once("location ")
            .is_some_and(|(_, rest)| rest.contains("not found"))
}

struct ApifyClient {
    http: Client,
    base_url: String,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    fn new(http: Client, config: &Config) -> Self {
        Self {
            http,
            base_url: config.apify_api_base.clone(),
            token: config.apify_token.clone(),
            actor_run_id: config.actor_run_id.clone(),
            key_value_store_id: config.key_value_store_id.clone(),
            dataset_id: config.dataset_id.clone(),
            input_key: config.input_key.clone(),
        }
    }

    fn endpoint(&self, path: &[&str]) -> Result<Url> {
        endpoint_url(&self.base_url, path)
    }

    async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
                .context("Failed to retrieve actor input from Apify API")?;
            if response.status() == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                sleep(delay).await;
                continue;
            }
            let response = require_apify_success(response, "input retrieval").await?;
            return response
                .json::<Value>()
                .await
                .context("Apify input record was not valid JSON")
                .map(Some);
        }
        unreachable!("bounded Apify retry loop returns a response")
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
                .context("Apify run pricing request failed")?;
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                sleep(delay).await;
                continue;
            }
            return require_apify_success(response, "run pricing request")
                .await?
                .json::<Value>()
                .await
                .context("Apify run pricing response was not valid JSON");
        }
        unreachable!("bounded Apify retry loop returns a response")
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Failed to publish dataset items to Apify API")?;
        require_apify_success(response, "dataset item publication")
            .await?
            .bytes()
            .await
            .context("Could not finish Apify dataset item publication")?;
        Ok(())
    }

    async fn charge_event(&self, event_name: &str, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id, "charge"])?;
        let idempotency_key = format!(
            "{}-{event_name}-{}-{}",
            self.actor_run_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            std::process::id()
        );
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let response = self
                .http
                .post(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .header("idempotency-key", &idempotency_key)
                .json(&json!({ "eventName": event_name, "count": count }))
                .send()
                .await
                .context("Apify event charge request failed")?;
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                sleep(delay).await;
                continue;
            }
            require_apify_success(response, "event charge")
                .await?
                .bytes()
                .await?;
            return Ok(());
        }
        unreachable!("bounded Apify retry loop returns a response")
    }

    async fn set_terminal_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let body = json!({
            "runId": self.actor_run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true,
        });
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let response = self
                .http
                .put(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .json(&body)
                .send()
                .await
                .context("Apify status message update failed")?;
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                sleep(delay).await;
                continue;
            }
            require_apify_success(response, "status message update")
                .await?
                .bytes()
                .await
                .context("Could not finish Apify status message update")?;
            return Ok(());
        }
        unreachable!("bounded Apify retry loop returns a response")
    }

    async fn put_record(&self, key: &str, value: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            key,
        ])?;
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let response = self
                .http
                .put(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .json(value)
                .send()
                .await
                .with_context(|| format!("Failed to write {key} record to Apify API"))?;
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                sleep(delay).await;
                continue;
            }
            require_apify_success(response, &format!("{key} record publication"))
                .await?
                .bytes()
                .await
                .with_context(|| format!("Could not finish {key} record publication"))?;
            return Ok(());
        }
        unreachable!("bounded Apify retry loop returns a response")
    }
}

fn apify_retry_delay(status: StatusCode, retry_count: usize) -> Option<Duration> {
    if retry_count >= APIFY_MAX_RETRIES
        || (status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error())
    {
        return None;
    }
    Some(Duration::from_secs((retry_count + 1) as u64))
}

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

fn endpoint_url(base: &str, segments: &[&str]) -> Result<Url> {
    let mut url = Url::parse(&format!("{}/", base.trim_end_matches('/')))
        .with_context(|| format!("Invalid API base URL: {base}"))?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow!("API base URL cannot contain query or fragment: {base}"))?;
        path.pop_if_empty();
        path.extend(segments.iter().copied());
    }
    Ok(url)
}

struct ChargePricing {
    is_pay_per_event: bool,
    max_total_charge: f64,
    spent: f64,
    event_prices: HashMap<String, f64>,
    configured_events: Vec<String>,
}

impl ChargePricing {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run.get("data").unwrap_or(run);
        let pricing_info = data.get("pricingInfo");
        if pricing_info
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(Self {
                is_pay_per_event: false,
                max_total_charge: f64::INFINITY,
                spent: 0.0,
                event_prices: HashMap::new(),
                configured_events: Vec::new(),
            });
        }

        let configured = pricing_info
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        let mut configured_events = Vec::new();
        for (name, event) in configured {
            configured_events.push(name.clone());
            if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
                if !price.is_finite() || price < 0.0 {
                    bail!("Apify run returned invalid charging values");
                }
                event_prices.insert(name.clone(), price);
            }
        }

        let max_total_charge = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|amount| amount.is_finite() && *amount != 0.0)
            .unwrap_or(f64::INFINITY);
        let counts = data.get("chargedEventCounts").and_then(Value::as_object);
        let mut spent = 0.0;
        if let Some(counts) = counts {
            for (event_name, count) in counts {
                let count = count
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
                if let Some(price) = event_prices.get(event_name) {
                    spent += price * count as f64;
                }
            }
        }
        if !spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        Ok(Self {
            is_pay_per_event: true,
            max_total_charge,
            spent: round_to_six_decimals(spent),
            event_prices,
            configured_events,
        })
    }

    fn plan_dataset_push(&self, requested: usize) -> PushPlan {
        if !self.is_pay_per_event {
            return PushPlan {
                items_to_push: requested,
                custom_event_charge_count: 0,
                limit_reached: false,
                charged_count: 0,
                should_charge_custom_event: false,
            };
        }

        let custom_price = self
            .event_prices
            .get(PROPERTY_RESULT_CHARGE_EVENT)
            .copied()
            .unwrap_or(0.0);
        let default_item_price = self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0.0);
        let item_price = custom_price + default_item_price;
        let remaining_count = self.available_count(item_price);
        let items_to_push = if remaining_count >= requested {
            requested
        } else if remaining_count == 0 && self.spent <= self.max_total_charge {
            usize::from(requested > 0)
        } else {
            remaining_count
        };
        let items_to_push = items_to_push.min(requested.max(usize::from(requested > 0)));
        if items_to_push == 0 {
            return PushPlan {
                items_to_push,
                custom_event_charge_count: 0,
                limit_reached: requested > 0,
                charged_count: 0,
                should_charge_custom_event: false,
            };
        }

        let mut spent = self.spent;
        let custom_count =
            self.charge_count(PROPERTY_RESULT_CHARGE_EVENT, items_to_push, &mut spent);
        let default_count =
            self.charge_count(DEFAULT_DATASET_ITEM_EVENT, items_to_push, &mut spent);
        let limit_reached = self.event_limit_reached(PROPERTY_RESULT_CHARGE_EVENT, spent)
            || self.event_limit_reached(DEFAULT_DATASET_ITEM_EVENT, spent);

        PushPlan {
            items_to_push,
            custom_event_charge_count: custom_count,
            limit_reached,
            charged_count: custom_count + default_count,
            should_charge_custom_event: self
                .configured_events
                .iter()
                .any(|event| event == PROPERTY_RESULT_CHARGE_EVENT),
        }
    }

    fn charge_count(&self, event_name: &str, count: usize, spent: &mut f64) -> usize {
        let price = self.event_prices.get(event_name).copied().unwrap_or(0.0);
        let available = self.available_count_for_price(price, *spent);
        let charged = if count <= available {
            count
        } else if *spent <= self.max_total_charge {
            available.saturating_add(1)
        } else {
            0
        };
        *spent = round_to_six_decimals(*spent + price * charged as f64);
        charged
    }

    fn event_limit_reached(&self, event_name: &str, spent: f64) -> bool {
        self.event_prices
            .get(event_name)
            .is_some_and(|price| *price > 0.0 && self.available_count_for_price(*price, spent) == 0)
    }

    fn available_count(&self, item_price: f64) -> usize {
        self.available_count_for_price(item_price, self.spent)
    }

    fn available_count_for_price(&self, price: f64, spent: f64) -> usize {
        if price <= 0.0 {
            return usize::MAX;
        }
        if self.max_total_charge.is_infinite() {
            return usize::MAX;
        }
        let amount = ((self.max_total_charge - spent) / price * 10_000.0).round() / 10_000.0;
        if amount <= 0.0 || !amount.is_finite() {
            0
        } else {
            amount.floor().min(usize::MAX as f64) as usize
        }
    }
}

fn round_to_six_decimals(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

#[derive(Debug, Clone, Copy)]
struct PushPlan {
    items_to_push: usize,
    custom_event_charge_count: usize,
    limit_reached: bool,
    charged_count: usize,
    should_charge_custom_event: bool,
}

fn get_listings(response: &Value) -> Vec<Value> {
    let top_level = response.get("results").and_then(Value::as_array);
    let wrapped = response.pointer("/data/results").and_then(Value::as_array);
    if let Some(results) = top_level {
        if !results.is_empty() || wrapped.is_none() {
            return results.clone();
        }
    }
    if let Some(results) = wrapped {
        return results.clone();
    }
    eprintln!(
        "Unexpected ImmobilienScout24 response shape: expected \"results\" or \"data.results\" array."
    );
    Vec::new()
}

fn get_number(value: Option<&Value>) -> Option<Number> {
    let value = value?;
    if let Some(number) = value.as_number() {
        return number
            .as_f64()
            .filter(|number| number.is_finite())
            .map(|_| number.clone());
    }
    let text = value.as_str()?.trim();
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u64>()
        .ok()
        .map(Number::from)
        .or_else(|| text.parse::<f64>().ok().and_then(Number::from_f64))
}

fn get_total_results(response: &Value) -> Option<Number> {
    get_number(response.get("total_results"))
        .or_else(|| get_number(response.pointer("/data/total_results")))
}

fn get_response_page(response: &Value) -> Option<Number> {
    get_number(response.get("page")).or_else(|| get_number(response.pointer("/data/page")))
}

fn get_total_pages(response: &Value) -> Option<Number> {
    get_number(response.get("total_pages"))
        .or_else(|| get_number(response.pointer("/data/total_pages")))
}

fn dataset_item(listing: &Value, params: &SearchParams) -> Value {
    let mut item = listing.as_object().cloned().unwrap_or_default();
    for (field, source) in [
        ("id", "id"),
        ("online_id", "online_id"),
        ("title", "title"),
        ("price", "price"),
        ("price_formatted", "price_formatted"),
        ("rooms", "rooms"),
        ("rooms_max", "rooms_max"),
        ("size_m2", "size_m2"),
        ("size_m2_max", "size_m2_max"),
        ("address", "address"),
        ("image_url", "image_url"),
        ("url", "url"),
        ("is_private", "is_private"),
        ("published", "published"),
    ] {
        item.insert(
            field.to_owned(),
            listing.get(source).cloned().unwrap_or(Value::Null),
        );
    }
    item.insert(
        "latitude".to_owned(),
        listing.get("lat").cloned().unwrap_or(Value::Null),
    );
    item.insert(
        "longitude".to_owned(),
        listing.get("lon").cloned().unwrap_or(Value::Null),
    );
    let params = params.as_value();
    for field in [
        "location",
        "type",
        "price_min",
        "price_max",
        "rooms_min",
        "rooms_max",
        "size_min",
        "size_max",
        "page",
        "per_page",
    ] {
        item.insert(
            format!("request_{field}"),
            params.get(field).cloned().unwrap_or(Value::Null),
        );
    }
    Value::Object(item)
}

fn limited_response(response: &Value, limit: usize) -> Value {
    let mut limited = response.clone();
    if let Some(results) = response.get("results").and_then(Value::as_array) {
        if let Some(object) = limited.as_object_mut() {
            object.insert(
                "results".to_owned(),
                Value::Array(results.iter().take(limit).cloned().collect()),
            );
        }
    }
    if let Some(results) = response.pointer("/data/results").and_then(Value::as_array) {
        if let Some(data) = limited.get_mut("data").and_then(Value::as_object_mut) {
            data.insert(
                "results".to_owned(),
                Value::Array(results.iter().take(limit).cloned().collect()),
            );
        }
    }
    limited
}

async fn search_immobilienscout24(client: &ScrappaClient, params: &SearchParams) -> Result<Value> {
    match client.search(params).await {
        Ok(response) => Ok(response),
        Err(error) if is_handled_empty_search_error(&error) => {
            let (status, message) = match &error {
                ScrappaError::Api { status, message } => (Some(*status), message.clone()),
                _ => unreachable!("only API errors are mapped as empty search results"),
            };
            eprintln!(
                "Scrappa ImmobilienScout24 search returned no usable result; saving a clean zero-result output. {}",
                json!({
                    "status": status,
                    "message": message,
                    "request_location": params.location,
                    "request_type": params.property_type,
                })
            );
            Ok(json!({
                "success": false,
                "total_results": 0,
                "page": params.page,
                "total_pages": 0,
                "results": [],
                "error": {
                    "message": message,
                    "status": status,
                },
            }))
        }
        Err(error) => {
            let message = match error {
                ScrappaError::Timeout => format!(
                    "{}. The ImmobilienScout24 request exceeded the {}s Scrappa API timeout. Try a smaller page size or run the request again.",
                    ScrappaError::Timeout,
                    SCRAPPA_REQUEST_TIMEOUT.as_secs()
                ),
                error => error.to_string(),
            };
            Err(anyhow!(message))
        }
    }
}

async fn run_actor() -> Result<()> {
    let config = Config::from_env()?;
    let apify_http = Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .context("Could not create Apify HTTP client")?;
    let scrappa_http = Client::builder()
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .build()
        .context("Could not create Scrappa HTTP client")?;
    let apify = ApifyClient::new(apify_http, &config);
    let scrappa = ScrappaClient::new(
        scrappa_http,
        config.scrappa_api_base.clone(),
        config.scrappa_api_key.clone(),
    );

    let run = apify.get_run().await?;
    let pricing = ChargePricing::from_run(&run)?;
    let input = apify.get_input().await?;
    let params = normalize_search_input(input.as_ref())?;
    println!("Searching ImmobilienScout24 for {}", params.describe());

    let response = search_immobilienscout24(&scrappa, &params).await?;
    let raw_listings = get_listings(&response);
    let requested_limit = params.per_page as usize;
    let listings = raw_listings
        .iter()
        .take(requested_limit)
        .map(|listing| dataset_item(listing, &params))
        .collect::<Vec<_>>();

    if !listings.is_empty() {
        if pricing.is_pay_per_event {
            let plan = pricing.plan_dataset_push(listings.len());
            apify
                .push_dataset_items(&listings[..plan.items_to_push])
                .await?;
            if plan.should_charge_custom_event {
                apify
                    .charge_event(PROPERTY_RESULT_CHARGE_EVENT, plan.custom_event_charge_count)
                    .await?;
            }
            if plan.limit_reached && plan.charged_count < listings.len() {
                let message = format!(
                    "Charge limit reached after saving {} of {} ImmobilienScout24 property result(s); OUTPUT was not written.",
                    plan.charged_count,
                    listings.len()
                );
                println!("[Status message]: {message}");
                match timeout(
                    Duration::from_secs(1),
                    apify.set_terminal_status_message(&message),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        eprintln!("Could not set final Apify status message: {error}")
                    }
                    Err(_) => eprintln!("Setting status message timed out after 1s"),
                }
                println!(
                    "{message} {}",
                    json!({
                        "event": PROPERTY_RESULT_CHARGE_EVENT,
                        "charged_count": plan.charged_count,
                        "requested_count": listings.len(),
                    })
                );
                return Ok(());
            }
        } else {
            apify.push_dataset_items(&listings).await?;
        }

        println!(
            "Found {} ImmobilienScout24 property result(s)",
            listings.len()
        );
        if raw_listings.len() > listings.len() {
            println!(
                "Scrappa returned {} result(s); saved the requested limit of {}.",
                raw_listings.len(),
                listings.len()
            );
        }
    } else {
        println!("No ImmobilienScout24 property results found for this request");
    }

    apify
        .put_record("OUTPUT", &limited_response(&response, listings.len()))
        .await?;
    println!("ImmobilienScout24 property search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "listings": listings.len(),
            "total_results": get_total_results(&response),
            "page": get_response_page(&response),
            "total_pages": get_total_pages(&response),
            "request_location": params.location,
            "request_type": params.property_type,
        })
    );
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run_actor().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
