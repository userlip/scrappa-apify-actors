use std::{
    env,
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const INTRADAY_PRICE_POINT_CHARGE_EVENT: &str = "intraday-price-point";
const ACTOR_USER_AGENT: &str = "thescrappa-google-finance-intraday-scraper/1.0";

struct Config {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key,
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

#[derive(Clone, Debug, PartialEq)]
struct IntradayRequest {
    params: Map<String, Value>,
}

fn clean_optional_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(value.to_owned()))
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_optional_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_symbol(value: Option<&Value>, index: usize) -> Result<String> {
    let field = format!("symbols[{index}].symbol");
    let symbol = clean_required_string(value, &field, 20)?.to_uppercase();
    if symbol.chars().any(char::is_whitespace) {
        bail!("{field} cannot contain spaces");
    }
    Ok(symbol)
}

fn clean_exchange(value: Option<&Value>, index: usize) -> Result<Option<String>> {
    Ok(
        clean_optional_string(value, &format!("symbols[{index}].exchange"), 40)?
            .map(|exchange| exchange.to_uppercase()),
    )
}

fn clean_language_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(language) = clean_optional_string(value, "hl", 10)? else {
        return Ok(None);
    };
    let language = language.to_ascii_lowercase();
    let valid = match language.split_once('-') {
        Some((language, country)) => {
            language.len() == 2
                && country.len() == 2
                && language.bytes().all(|byte| byte.is_ascii_lowercase())
                && country.bytes().all(|byte| byte.is_ascii_lowercase())
                && !country.contains('-')
        }
        None => language.len() == 2 && language.bytes().all(|byte| byte.is_ascii_lowercase()),
    };
    if !valid {
        bail!("hl must be a valid language code such as en, de, or zh-cn");
    }
    Ok(Some(language))
}

fn clean_country_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(country) = clean_optional_string(value, "gl", 10)? else {
        return Ok(None);
    };
    let country = country.to_ascii_lowercase();
    if country.len() != 2 || !country.bytes().all(|byte| byte.is_ascii_lowercase()) {
        bail!("gl must be a two-letter country code");
    }
    Ok(Some(country))
}

fn build_intraday_requests(input: &Value) -> Result<Vec<IntradayRequest>> {
    let Some(input) = input.as_object() else {
        bail!("Input must be an object");
    };
    let Some(symbols) = input.get("symbols").and_then(Value::as_array) else {
        bail!("symbols must be an array");
    };
    if symbols.is_empty() {
        bail!("At least one symbol is required");
    }

    let hl = clean_language_code(input.get("hl"))?;
    let gl = clean_country_code(input.get("gl"))?;
    symbols
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let Some(item) = item.as_object() else {
                bail!("symbols[{index}] must be an object");
            };
            let mut params = Map::new();
            params.insert(
                "symbol".to_owned(),
                Value::String(clean_symbol(item.get("symbol"), index)?),
            );
            if let Some(exchange) = clean_exchange(item.get("exchange"), index)? {
                params.insert("exchange".to_owned(), Value::String(exchange));
            }
            if let Some(hl) = &hl {
                params.insert("hl".to_owned(), Value::String(hl.clone()));
            }
            if let Some(gl) = &gl {
                params.insert("gl".to_owned(), Value::String(gl.clone()));
            }
            Ok(IntradayRequest { params })
        })
        .collect()
}

fn describe_intraday_request(params: &Map<String, Value>) -> String {
    let symbol = params
        .get("symbol")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let exchange = params
        .get("exchange")
        .and_then(Value::as_str)
        .filter(|exchange| !exchange.is_empty())
        .map(|exchange| format!(":{exchange}"))
        .unwrap_or_default();
    let details = ["hl", "gl"]
        .iter()
        .filter_map(|field| {
            params
                .get(*field)
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(|value| format!("{field}={value}"))
        })
        .collect::<Vec<_>>();
    if details.is_empty() {
        format!("{symbol}{exchange}")
    } else {
        format!("{symbol}{exchange} ({})", details.join(", "))
    }
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

impl Error for ScrappaApiError {}

#[derive(Debug)]
struct ScrappaTimeoutError {
    timeout_ms: u128,
}

impl fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            self.timeout_ms
        )
    }
}

impl Error for ScrappaTimeoutError {}

struct ScrappaClient<'a> {
    http: &'a Client,
    api_key: &'a str,
    base_url: &'a Url,
}

impl ScrappaClient<'_> {
    async fn get_intraday(&self, params: &Map<String, Value>) -> Result<Value> {
        let mut url = endpoint_url(self.base_url, &["google-finance", "intraday"])?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                if value.is_null() || value.as_str() == Some("") {
                    continue;
                }
                if let Some(value) = value.as_bool() {
                    if value {
                        query.append_pair(key, "1");
                    }
                } else {
                    query.append_pair(key, &js_string(value));
                }
            }
        }

        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send_intraday(&url).await {
                Ok(response) => return Ok(response),
                Err(error)
                    if attempt < SCRAPPA_MAX_ATTEMPTS && is_retryable_scrappa_error(&error) =>
                {
                    let delay_ms = retry_delay_ms(attempt, random_jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {delay_ms}ms.",
                        error,
                        attempt + 1,
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("retry loop always returns the final result")
    }

    async fn send_intraday(&self, url: &Url) -> Result<Value> {
        let response = self
            .http
            .get(url.clone())
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .header("X-API-Key", self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, ACTOR_USER_AGENT)
            .send()
            .await
            .map_err(scrappa_request_error)?;

        let status = response.status();
        let body = response.text().await.map_err(scrappa_request_error)?;
        if !status.is_success() {
            return Err(scrappa_api_error(status, &body).into());
        }
        serde_json::from_str(&body)
            .map_err(|error| anyhow!("Scrappa API response was not valid JSON: {error}"))
    }
}

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaTimeoutError {
            timeout_ms: SCRAPPA_REQUEST_TIMEOUT.as_millis(),
        }
        .into()
    } else {
        anyhow!(error)
    }
}

fn scrappa_api_error(status: StatusCode, body: &str) -> ScrappaApiError {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let Ok(data) = serde_json::from_str::<Value>(body) else {
        return ScrappaApiError {
            status: status.as_u16(),
            message: if body.is_empty() {
                fallback
            } else {
                body.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(500)
                    .collect()
            },
        };
    };
    let Some(object) = data.as_object() else {
        return ScrappaApiError {
            status: status.as_u16(),
            message: body.to_owned(),
        };
    };
    let mut message = object
        .get("message")
        .filter(|value| !value.is_null())
        .or_else(|| object.get("error").filter(|value| !value.is_null()))
        .map(js_string)
        .unwrap_or(fallback);
    if let Some(errors) = object.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .map(|(field, messages)| {
                let messages = match messages.as_array() {
                    Some(messages) => messages
                        .iter()
                        .map(js_string)
                        .collect::<Vec<_>>()
                        .join(", "),
                    None => js_string(messages),
                };
                format!("{field}: {messages}")
            })
            .collect::<Vec<_>>();
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details.join("; "));
        }
    }
    ScrappaApiError {
        status: status.as_u16(),
        message,
    }
}

fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if let Some(api_error) = error.downcast_ref::<ScrappaApiError>() {
        return matches!(api_error.status, 408 | 429 | 500 | 502 | 503 | 504);
    }
    error.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|error| error.is_timeout() || error.is_connect() || error.is_body())
    })
}

fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponent = u32::try_from(failed_attempt).unwrap_or(u32::MAX);
    let backoff = 1_000u64.saturating_mul(1u64.checked_shl(exponent).unwrap_or(u64::MAX));
    backoff.saturating_add(jitter_ms).min(10_000)
}

fn random_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
        % 1_000
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn first_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().find_map(|value| {
        value
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
    })
}

fn as_number(value: &Value) -> Option<Value> {
    if let Value::Number(number) = value {
        if number.as_i64().is_some() || number.as_u64().is_some() {
            return Some(value.clone());
        }
    }
    let number = match value {
        Value::Number(number) => number.as_f64()?,
        Value::String(value) => value.replace(',', "").trim().parse::<f64>().ok()?,
        _ => return None,
    };
    if !number.is_finite() {
        return None;
    }
    if number.fract() == 0.0 {
        if number >= i64::MIN as f64 && number <= i64::MAX as f64 {
            return Some(json!(number as i64));
        }
        if number >= 0.0 && number <= u64::MAX as f64 {
            return Some(json!(number as u64));
        }
    }
    serde_json::Number::from_f64(number).map(Value::Number)
}

fn first_number(values: &[Option<&Value>]) -> Value {
    values
        .iter()
        .find_map(|value| value.and_then(as_number))
        .unwrap_or(Value::Null)
}

fn date_to_iso(value: Option<&Value>) -> Option<String> {
    let value = value?.as_str()?.trim();
    if value.is_empty() {
        return None;
    }

    let parsed = DateTime::parse_from_str(value, "%b %e %Y, %I:%M %p UTC%:z")
        .map(|date| date.with_timezone(&Utc))
        .or_else(|_| DateTime::parse_from_rfc3339(value).map(|date| date.with_timezone(&Utc)))
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f").map(|date| date.and_utc())
        })
        .or_else(|_| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d").map(|date| {
                date.and_hms_opt(0, 0, 0)
                    .expect("midnight is a valid time")
                    .and_utc()
            })
        })
        .ok()?;
    Some(parsed.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
}

fn build_intraday_price_point_dataset_items(
    response: &Value,
    params: &Map<String, Value>,
) -> Vec<Value> {
    let Some(graph) = response.get("graph").and_then(Value::as_array) else {
        return Vec::new();
    };
    graph
        .iter()
        .filter_map(Value::as_object)
        .enumerate()
        .map(|(index, point)| {
            let mut item = point.clone();
            let date = first_string(&[point.get("date")])
                .map(Value::String)
                .unwrap_or(Value::Null);
            let date_iso = date_to_iso(point.get("date"))
                .map(Value::String)
                .unwrap_or_else(|| {
                    if let Some(date) = point
                        .get("date")
                        .and_then(Value::as_str)
                        .filter(|date| !date.trim().is_empty())
                    {
                        eprintln!("Could not parse Google Finance intraday date: {date}");
                    }
                    Value::Null
                });
            let symbol = first_string(&[response.get("symbol"), params.get("symbol")])
                .map(Value::String)
                .unwrap_or(Value::Null);
            let exchange = first_string(&[response.get("exchange"), params.get("exchange")])
                .map(Value::String)
                .unwrap_or(Value::Null);
            let currency = first_string(&[point.get("currency"), response.get("currency")])
                .map(Value::String)
                .unwrap_or(Value::Null);

            item.insert("position".to_owned(), json!(index + 1));
            item.insert("date".to_owned(), date);
            item.insert("date_iso".to_owned(), date_iso);
            item.insert("price".to_owned(), first_number(&[point.get("price")]));
            item.insert("change".to_owned(), first_number(&[point.get("change")]));
            item.insert(
                "percent_change".to_owned(),
                first_number(&[point.get("percent_change")]),
            );
            item.insert("volume".to_owned(), first_number(&[point.get("volume")]));
            item.insert("symbol".to_owned(), symbol);
            item.insert("exchange".to_owned(), exchange);
            item.insert("currency".to_owned(), currency);
            for field in ["symbol", "exchange", "hl", "gl"] {
                item.insert(
                    format!("request_{field}"),
                    params.get(field).cloned().unwrap_or(Value::Null),
                );
            }
            Value::Object(item)
        })
        .collect()
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("{operation} response could not be read"))?;
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

async fn update_terminal_status_message(
    http: &Client,
    apify_api_base_url: &Url,
    apify_token: &str,
    actor_run_id: &str,
    status_message: &str,
) -> Result<()> {
    let url = endpoint_url(apify_api_base_url, &["v2", "actor-runs", actor_run_id])?;
    let response = http
        .put(url)
        .bearer_auth(apify_token)
        .header(header::ACCEPT, "application/json")
        .json(&json!({
            "runId": actor_run_id,
            "statusMessage": status_message,
            "isStatusMessageTerminal": true,
        }))
        .send()
        .await
        .context("Apify run status update failed")?;
    ensure_success(response, "Apify run status update").await
}

async fn update_terminal_status_message_from_env(status_message: &str) -> Result<()> {
    let apify_api_base_url = base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?;
    let apify_token = required_env("APIFY_TOKEN")?;
    let actor_run_id = required_env("ACTOR_RUN_ID")?;
    let http = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("Failed to create the Apify API client for the status message")?;
    update_terminal_status_message(
        &http,
        &apify_api_base_url,
        &apify_token,
        &actor_run_id,
        status_message,
    )
    .await
}

#[derive(Default)]
struct ChargeBudget {
    initial_point_charges: Option<u64>,
    confirmed_point_charges: u64,
}

fn affordable_point_count(
    run: &Value,
    requested: usize,
    budget: &mut ChargeBudget,
) -> Result<Option<usize>> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        return Ok(None);
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let point_price = events
        .get(INTRADAY_PRICE_POINT_CHARGE_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the intraday price point event price"))?;
    if !point_price.is_finite() || point_price < 0.0 {
        bail!("Apify run returned an invalid intraday price point event price");
    }
    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .filter(|value| !value.is_null())
        .map(|value| {
            value
                .as_f64()
                .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))
        })
        .transpose()?;
    if max_charge.is_some_and(|max_charge| !max_charge.is_finite() || max_charge < 0.0) {
        bail!("Apify run returned an invalid spending limit");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let reported_point_charges = counts
        .get(INTRADAY_PRICE_POINT_CHARGE_EVENT)
        .map(|count| {
            count.as_u64().ok_or_else(|| {
                anyhow!("Invalid charged event count for {INTRADAY_PRICE_POINT_CHARGE_EVENT}")
            })
        })
        .transpose()?
        .unwrap_or(0);
    let initial_point_charges = *budget
        .initial_point_charges
        .get_or_insert(reported_point_charges);
    let locally_confirmed = initial_point_charges
        .checked_add(budget.confirmed_point_charges)
        .ok_or_else(|| anyhow!("Intraday price point charge count overflowed"))?;

    let mut spent = 0.0;
    let mut saw_point_count = false;
    for (event_name, count) in &counts {
        let mut count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if event_name == INTRADAY_PRICE_POINT_CHARGE_EVENT {
            saw_point_count = true;
            count = count.max(locally_confirmed);
        }
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
    if !saw_point_count && locally_confirmed > 0 {
        spent += point_price * locally_confirmed as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    let Some(max_charge) = max_charge else {
        return Ok(Some(requested));
    };
    if point_price == 0.0 {
        return Ok(Some(requested));
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let affordable = (1..=requested)
        .take_while(|count| spent + *count as f64 * point_price <= max_charge + tolerance)
        .count();
    Ok(Some(affordable))
}

struct DatasetPushResult {
    saved_count: usize,
    charge_limit_reached: bool,
}

struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
    charge_sequence: u64,
}

impl ApifyClient<'_> {
    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base_url, segments)
    }

    async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        response_json(response, "Apify INPUT request")
            .await
            .map(Some)
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
            .http
            .put(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }

    async fn set_terminal_status_message(&self, status_message: &str) -> Result<()> {
        update_terminal_status_message(
            self.http,
            &self.config.apify_api_base_url,
            &self.config.apify_token,
            &self.config.actor_run_id,
            status_message,
        )
        .await
    }

    async fn push_dataset_items(
        &mut self,
        items: &[Value],
        budget: &mut ChargeBudget,
    ) -> Result<DatasetPushResult> {
        if items.is_empty() {
            return Ok(DatasetPushResult {
                saved_count: 0,
                charge_limit_reached: false,
            });
        }

        let run_url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let run_response = self
            .http
            .get(run_url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = response_json(run_response, "Apify run pricing request").await?;
        let charge_count = affordable_point_count(&run, items.len(), budget)?;

        let Some(charge_count) = charge_count else {
            self.store_dataset_items(items).await?;
            return Ok(DatasetPushResult {
                saved_count: items.len(),
                charge_limit_reached: false,
            });
        };

        let charge_limit_reached = charge_count < items.len();
        if charge_count == 0 {
            return Ok(DatasetPushResult {
                saved_count: 0,
                charge_limit_reached,
            });
        }
        self.store_dataset_items(&items[..charge_count]).await?;
        self.charge_points(charge_count, budget).await?;
        Ok(DatasetPushResult {
            saved_count: charge_count,
            charge_limit_reached,
        })
    }

    async fn charge_points(&mut self, count: usize, budget: &mut ChargeBudget) -> Result<()> {
        self.charge_sequence = self
            .charge_sequence
            .checked_add(1)
            .ok_or_else(|| anyhow!("Apify charge request count overflowed"))?;
        let idempotency_key = format!(
            "{}-{}-{}",
            self.config.actor_run_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            self.charge_sequence,
        );
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .header("idempotency-key", idempotency_key)
            .json(&json!({
                "eventName": INTRADAY_PRICE_POINT_CHARGE_EVENT,
                "count": count,
            }))
            .send()
            .await
            .context("Apify intraday price point charge request failed")?;
        ensure_success(response, "Apify intraday price point charge request").await?;
        let count =
            u64::try_from(count).context("Intraday price point charge count is too large")?;
        budget.confirmed_point_charges = budget
            .confirmed_point_charges
            .checked_add(count)
            .ok_or_else(|| anyhow!("Intraday price point charge count overflowed"))?;
        Ok(())
    }

    async fn store_dataset_items(&self, items: &[Value]) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }
}

#[derive(Default)]
struct RunSummary {
    requested: usize,
    succeeded: usize,
    no_data: usize,
    failed: usize,
    graph_points: usize,
}

impl RunSummary {
    fn to_value(&self) -> Value {
        json!({
            "requested": self.requested,
            "succeeded": self.succeeded,
            "no_data": self.no_data,
            "failed": self.failed,
            "graph_points": self.graph_points,
        })
    }
}

fn is_no_data_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| error.status == StatusCode::NOT_FOUND.as_u16())
}

async fn run_actor(config: &Config) -> Result<()> {
    let apify_http = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("Failed to create the Apify API client")?;
    let scrappa_http = Client::new();
    let apify = ApifyClient {
        http: &apify_http,
        config,
        charge_sequence: 0,
    };

    let Some(input) = apify.get_input().await? else {
        bail!("Input is required");
    };
    let requests = build_intraday_requests(&input)?;
    let scrappa = ScrappaClient {
        http: &scrappa_http,
        api_key: &config.scrappa_api_key,
        base_url: &config.scrappa_api_base_url,
    };
    let mut apify = apify;
    let mut summary = RunSummary {
        requested: requests.len(),
        ..RunSummary::default()
    };
    let mut charge_budget = ChargeBudget::default();

    println!(
        "Fetching Google Finance intraday data for {} symbol{}",
        requests.len(),
        if requests.len() == 1 { "" } else { "s" }
    );

    for request in &requests {
        let description = describe_intraday_request(&request.params);
        println!("Fetching Google Finance intraday data for {description}");
        let response = match scrappa.get_intraday(&request.params).await {
            Ok(response) => response,
            Err(error) if is_no_data_error(&error) => {
                println!("No Google Finance intraday graph points found for {description}");
                summary.no_data += 1;
                continue;
            }
            Err(error) => return Err(error),
        };
        let dataset_items = build_intraday_price_point_dataset_items(&response, &request.params);
        if dataset_items.is_empty() {
            println!("No Google Finance intraday graph points found for {description}");
            summary.no_data += 1;
            continue;
        }

        let push_result = apify
            .push_dataset_items(&dataset_items, &mut charge_budget)
            .await?;
        if push_result.charge_limit_reached {
            let status_message = "Charge limit reached before saving all Google Finance intraday price points; remaining symbols were not processed.";
            println!(
                "{status_message} {}",
                json!({
                    "event": INTRADAY_PRICE_POINT_CHARGE_EVENT,
                    "charged_count": push_result.saved_count,
                    "requested_count": dataset_items.len(),
                })
            );
            apify.set_terminal_status_message(status_message).await?;
            return Ok(());
        }
        summary.succeeded += 1;
        summary.graph_points += push_result.saved_count;
    }

    apify.put_output(&summary.to_value()).await?;
    println!("Google Finance intraday scraping completed successfully");
    println!("Results summary: {}", summary.to_value());
    Ok(())
}

#[tokio::main]
async fn main() {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            if let Err(status_error) = update_terminal_status_message_from_env(&message).await {
                eprintln!("Could not set the Apify run status message: {status_error}");
            }
            std::process::exit(1);
        }
    };

    if let Err(error) = run_actor(&config).await {
        let message = if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
            format!(
                "{error}. The Google Finance intraday request exceeded the {}s Scrappa API timeout. Provide exchange codes, reduce the symbol batch, or run the request again.",
                SCRAPPA_REQUEST_TIMEOUT.as_secs()
            )
        } else {
            error.to_string()
        };
        eprintln!("Actor failed: {message}");
        if let Err(status_error) = update_terminal_status_message_from_env(&message).await {
            eprintln!("Could not set the Apify run status message: {status_error}");
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc,
        thread,
    };

    fn read_http_request(stream: &mut TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut byte = [0; 1];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }

        let headers = String::from_utf8_lossy(&request);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        let body_start = request.len();
        request.resize(body_start + content_length, 0);
        stream.read_exact(&mut request[body_start..]).unwrap();
        request
    }

    fn mock_apify_server(
        responses: Vec<(u16, &'static str, String)>,
    ) -> (Url, thread::JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            responses
                .into_iter()
                .map(|(status, reason, body)| {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_http_request(&mut stream);
                    write!(
                        stream,
                        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .unwrap();
                    request
                })
                .collect()
        });
        (Url::parse(&format!("http://{address}")).unwrap(), server)
    }

    fn test_apify_config(api_base_url: Url) -> Config {
        Config {
            apify_api_base_url: api_base_url,
            scrappa_api_base_url: Url::parse(SCRAPPA_API_DEFAULT).unwrap(),
            apify_token: "test-apify-token".to_owned(),
            key_value_store_id: "test-store".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    fn intraday_pricing_response() -> String {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "intraday-price-point": { "eventPriceUsd": 0.2 },
                        "another-event": { "eventPriceUsd": 0.1 }
                    }}
                },
                "chargedEventCounts": {
                    "intraday-price-point": 1,
                    "another-event": 1
                },
                "options": { "maxTotalChargeUsd": 0.75 }
            }
        })
        .to_string()
    }

    fn request_path(request: &[u8]) -> &str {
        std::str::from_utf8(request)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
    }

    fn request_body(request: &[u8]) -> Value {
        let body_start = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
            .unwrap();
        serde_json::from_slice(&request[body_start..]).unwrap()
    }

    #[test]
    fn builds_normalized_requests_for_symbol_batches() {
        let requests = build_intraday_requests(&json!({
            "symbols": [
                { "symbol": " aapl ", "exchange": " nasdaq " },
                { "symbol": "msft" },
            ],
            "hl": "EN",
            "gl": "US",
        }))
        .unwrap();
        let expected = vec![
            json!({ "symbol": "AAPL", "exchange": "NASDAQ", "hl": "en", "gl": "us" })
                .as_object()
                .unwrap()
                .clone(),
            json!({ "symbol": "MSFT", "hl": "en", "gl": "us" })
                .as_object()
                .unwrap()
                .clone(),
        ];
        assert_eq!(
            requests
                .into_iter()
                .map(|request| request.params)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn preserves_input_validation_messages() {
        assert_eq!(
            build_intraday_requests(&json!({})).unwrap_err().to_string(),
            "symbols must be an array"
        );
        assert_eq!(
            build_intraday_requests(&json!({ "symbols": [] }))
                .unwrap_err()
                .to_string(),
            "At least one symbol is required"
        );
        assert!(build_intraday_requests(&json!({ "symbols": ["AAPL"] }))
            .unwrap_err()
            .to_string()
            .contains("symbols[0] must be an object"));
        assert!(
            build_intraday_requests(&json!({ "symbols": [{ "symbol": "BRK B" }] }))
                .unwrap_err()
                .to_string()
                .contains("symbols[0].symbol cannot contain spaces")
        );
        assert!(build_intraday_requests(
            &json!({ "symbols": [{ "symbol": "AAPL" }], "hl": "english" })
        )
        .unwrap_err()
        .to_string()
        .contains("hl must be a valid language code"));
        assert!(build_intraday_requests(
            &json!({ "symbols": [{ "symbol": "AAPL" }], "gl": "usa" })
        )
        .unwrap_err()
        .to_string()
        .contains("gl must be a two-letter country code"));
    }

    #[test]
    fn describes_requests_for_logs() {
        let params = json!({ "symbol": "AAPL", "exchange": "NASDAQ", "hl": "en", "gl": "us" });
        assert_eq!(
            describe_intraday_request(params.as_object().unwrap()),
            "AAPL:NASDAQ (hl=en, gl=us)"
        );
    }

    #[tokio::test]
    async fn failed_dataset_write_does_not_charge_or_retry_the_append() {
        let (api_base_url, server) = mock_apify_server(vec![
            (200, "OK", intraday_pricing_response()),
            (
                503,
                "Service Unavailable",
                r#"{"error":"temporary dataset failure"}"#.to_owned(),
            ),
        ]);
        let http = Client::new();
        let config = test_apify_config(api_base_url);
        let mut apify = ApifyClient {
            http: &http,
            config: &config,
            charge_sequence: 0,
        };
        let items = [json!({ "price": 198.42 }), json!({ "price": 199.01 })];
        let mut budget = ChargeBudget::default();

        let error = match apify.push_dataset_items(&items, &mut budget).await {
            Ok(_) => panic!("failed dataset write unexpectedly succeeded"),
            Err(error) => error,
        };

        assert!(error
            .to_string()
            .contains("Apify dataset write failed with 503 Service Unavailable"));
        assert_eq!(budget.confirmed_point_charges, 0);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_path(&requests[0]), "/v2/actor-runs/test-run");
        assert_eq!(
            request_path(&requests[1]),
            "/v2/datasets/test-dataset/items"
        );
    }

    #[tokio::test]
    async fn successful_dataset_write_is_charged_after_saving_affordable_points() {
        let (api_base_url, server) = mock_apify_server(vec![
            (200, "OK", intraday_pricing_response()),
            (201, "Created", String::new()),
            (200, "OK", String::new()),
        ]);
        let http = Client::new();
        let config = test_apify_config(api_base_url);
        let mut apify = ApifyClient {
            http: &http,
            config: &config,
            charge_sequence: 0,
        };
        let items = [
            json!({ "price": 198.42 }),
            json!({ "price": 199.01 }),
            json!({ "price": 199.5 }),
        ];
        let mut budget = ChargeBudget::default();

        let result = apify.push_dataset_items(&items, &mut budget).await.unwrap();

        assert_eq!(result.saved_count, 2);
        assert!(result.charge_limit_reached);
        assert_eq!(budget.confirmed_point_charges, 2);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_path(&requests[0]), "/v2/actor-runs/test-run");
        assert_eq!(
            request_path(&requests[1]),
            "/v2/datasets/test-dataset/items"
        );
        assert_eq!(
            request_body(&requests[1]),
            json!([{ "price": 198.42 }, { "price": 199.01 }])
        );
        assert_eq!(request_path(&requests[2]), "/v2/actor-runs/test-run/charge");
        assert_eq!(
            request_body(&requests[2]),
            json!({ "eventName": "intraday-price-point", "count": 2 })
        );
    }

    #[tokio::test]
    async fn non_pay_per_event_dataset_write_skips_custom_charging() {
        let (api_base_url, server) = mock_apify_server(vec![
            (
                200,
                "OK",
                json!({ "data": { "pricingInfo": { "pricingModel": "PRICE_PER_DATASET_ITEM" } } })
                    .to_string(),
            ),
            (201, "Created", String::new()),
        ]);
        let http = Client::new();
        let config = test_apify_config(api_base_url);
        let mut apify = ApifyClient {
            http: &http,
            config: &config,
            charge_sequence: 0,
        };
        let items = [json!({ "price": 198.42 })];
        let mut budget = ChargeBudget::default();

        let result = apify.push_dataset_items(&items, &mut budget).await.unwrap();

        assert_eq!(result.saved_count, 1);
        assert!(!result.charge_limit_reached);
        assert_eq!(budget.confirmed_point_charges, 0);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_path(&requests[0]), "/v2/actor-runs/test-run");
        assert_eq!(
            request_path(&requests[1]),
            "/v2/datasets/test-dataset/items"
        );
    }

    #[tokio::test]
    async fn sends_authenticated_intraday_request_to_scrappa_api() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, request_receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut byte = [0; 1];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            request_sender
                .send(String::from_utf8(request).unwrap())
                .unwrap();

            let body = r#"{"graph":[]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let base_url = Url::parse(&format!("http://{address}/api")).unwrap();
        let http = Client::new();
        let params = json!({
            "symbol": "AAPL",
            "exchange": "NASDAQ",
            "hl": "en",
            "gl": "us",
        });
        let response = ScrappaClient {
            http: &http,
            api_key: "test-scrappa-key",
            base_url: &base_url,
        }
        .get_intraday(params.as_object().unwrap())
        .await
        .unwrap();

        assert_eq!(response["graph"], json!([]));
        let request = request_receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .to_ascii_lowercase();
        assert!(request.starts_with("get /api/google-finance/intraday?"));
        for query_param in ["symbol=aapl", "exchange=nasdaq", "hl=en", "gl=us"] {
            assert!(
                request.contains(query_param),
                "missing {query_param} in request: {request}"
            );
        }
        assert!(request.contains("x-api-key: test-scrappa-key"));
        assert!(request.contains("accept: application/json"));
        assert!(request.contains(ACTOR_USER_AGENT));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn retries_a_transient_scrappa_error() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, request_receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            for (status, reason, body) in [
                (503, "Service Unavailable", r#"{"error":"retry"}"#),
                (200, "OK", r#"{"graph":[]}"#),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut byte = [0; 1];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                }
                request_sender
                    .send(String::from_utf8(request).unwrap())
                    .unwrap();
                write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });

        let base_url = Url::parse(&format!("http://{address}/api")).unwrap();
        let http = Client::new();
        let params = json!({ "symbol": "AAPL" });
        let response = ScrappaClient {
            http: &http,
            api_key: "test-scrappa-key",
            base_url: &base_url,
        }
        .get_intraday(params.as_object().unwrap())
        .await
        .unwrap();

        assert_eq!(response["graph"], json!([]));
        for _ in 0..2 {
            let request = request_receiver
                .recv_timeout(Duration::from_secs(2))
                .unwrap();
            assert!(request.starts_with("GET /api/google-finance/intraday?symbol=AAPL"));
        }
        server.join().unwrap();
    }

    #[test]
    fn maps_graph_points_to_dataset_items_and_keeps_extra_fields() {
        let params = json!({ "symbol": "AAPL", "exchange": "NASDAQ", "hl": "en", "gl": "us" });
        let items = build_intraday_price_point_dataset_items(
            &json!({
                "symbol": "AAPL",
                "exchange": "NASDAQ",
                "currency": "USD",
                "graph": [{
                    "date": "Jun 16 2025, 09:30 AM UTC-04:00",
                    "price": "198.42",
                    "change": "1.25",
                    "percent_change": "0.63",
                    "volume": "3,482,103",
                    "vendor_field": "kept",
                }],
            }),
            params.as_object().unwrap(),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["position"], 1);
        assert_eq!(items[0]["date"], "Jun 16 2025, 09:30 AM UTC-04:00");
        assert_eq!(items[0]["date_iso"], "2025-06-16T13:30:00.000Z");
        assert_eq!(items[0]["price"], 198.42);
        assert_eq!(items[0]["change"], 1.25);
        assert_eq!(items[0]["percent_change"], 0.63);
        assert_eq!(items[0]["volume"], 3_482_103);
        assert_eq!(items[0]["currency"], "USD");
        assert_eq!(items[0]["vendor_field"], "kept");
        assert_eq!(items[0]["request_symbol"], "AAPL");
        assert_eq!(items[0]["request_exchange"], "NASDAQ");
        assert_eq!(items[0]["request_hl"], "en");
        assert_eq!(items[0]["request_gl"], "us");
        assert!(items[0].get("result_counts").is_none());
    }

    #[test]
    fn ignores_non_object_points_and_handles_missing_graph_and_invalid_dates() {
        let params = json!({ "symbol": "VOO", "exchange": "NYSEARCA" });
        assert!(build_intraday_price_point_dataset_items(
            &json!({ "currency": "USD" }),
            params.as_object().unwrap(),
        )
        .is_empty());
        let items = build_intraday_price_point_dataset_items(
            &json!({ "symbol": "VOO", "graph": [null, "invalid", { "date": "not a date", "price": "bad" }] }),
            params.as_object().unwrap(),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["position"], 1);
        assert_eq!(items[0]["date_iso"], Value::Null);
        assert_eq!(items[0]["price"], Value::Null);
        assert_eq!(items[0]["exchange"], "NYSEARCA");
    }

    #[test]
    fn formats_upstream_errors_and_retry_policy() {
        assert_eq!(
            scrappa_api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                r#"{"error":"Intraday data is temporarily unavailable. Please retry."}"#,
            )
            .to_string(),
            "Scrappa API error (503): Intraday data is temporarily unavailable. Please retry."
        );
        assert_eq!(
            scrappa_api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                r#"{"message":"Invalid request","errors":{"symbol":["The stock symbol is required."]}}"#,
            )
            .to_string(),
            "Scrappa API error (422): Invalid request - symbol: The stock symbol is required."
        );
        assert!(is_retryable_scrappa_error(
            &ScrappaTimeoutError { timeout_ms: 60_000 }.into()
        ));
        assert!(is_retryable_scrappa_error(
            &scrappa_api_error(StatusCode::TOO_MANY_REQUESTS, "rate limited").into()
        ));
        assert!(is_retryable_scrappa_error(
            &scrappa_api_error(StatusCode::BAD_GATEWAY, "upstream failed").into()
        ));
        assert!(!is_retryable_scrappa_error(
            &scrappa_api_error(StatusCode::NOT_FOUND, "no data").into()
        ));
        assert!(!is_retryable_scrappa_error(
            &scrappa_api_error(StatusCode::UNPROCESSABLE_ENTITY, "invalid").into()
        ));
        assert_eq!(retry_delay_ms(1, 0), 2_000);
        assert_eq!(retry_delay_ms(2, 500), 4_500);
        assert_eq!(retry_delay_ms(20, 0), 10_000);
    }

    #[test]
    fn respects_existing_event_charges_and_partial_spending_limits() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "intraday-price-point": { "eventPriceUsd": 0.2 },
                        "another-event": { "eventPriceUsd": 0.1 }
                    }}
                },
                "chargedEventCounts": {
                    "intraday-price-point": 1,
                    "another-event": 1
                },
                "options": { "maxTotalChargeUsd": 0.75 }
            }
        });
        let mut budget = ChargeBudget::default();
        assert_eq!(
            affordable_point_count(&run, 5, &mut budget).unwrap(),
            Some(2)
        );
        budget.confirmed_point_charges = 2;
        let mut next_run = run.clone();
        next_run["data"]["chargedEventCounts"]["intraday-price-point"] = json!(1);
        assert_eq!(
            affordable_point_count(&next_run, 2, &mut budget).unwrap(),
            Some(0)
        );
    }

    #[test]
    fn skips_event_charging_during_non_pay_per_event_pricing() {
        let run =
            json!({ "data": { "pricingInfo": { "pricingModel": "PRICE_PER_DATASET_ITEM" } } });
        assert_eq!(
            affordable_point_count(&run, 3, &mut ChargeBudget::default()).unwrap(),
            None
        );
    }
}
