use std::{collections::HashMap, env, fmt, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response};
use serde_json::{json, Map, Value};
use tokio::time::{sleep, timeout};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const SCRAPPA_MAX_ATTEMPTS: u32 = 3;
const MAX_QUERIES_PER_RUN: usize = 25;
const FINANCE_SEARCH_RESULT_EVENT: &str = "finance-search-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const SCRAPPA_USER_AGENT: &str = "thescrappa-google-finance-search-scraper/1.0";

struct Config {
    apify_api_base: Url,
    scrappa_api_base: Url,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
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

struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

impl ApifyClient<'_> {
    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
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
            .http
            .get(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        response_json(response, "Apify INPUT request").await
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .get(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        response_json(response, "Apify run pricing request").await
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }

    async fn charge_event(
        &self,
        event_name: &str,
        count: usize,
        idempotency_key: &str,
    ) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        let response = self
            .http
            .post(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .header("idempotency-key", idempotency_key)
            .json(&json!({"eventName": event_name, "count": count}))
            .send()
            .await
            .context("Apify finance search charge request failed")?;
        ensure_success(response, "Apify finance search charge request").await
    }

    async fn update_status(&self, status_message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .put(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(&json!({
                "runId": self.config.actor_run_id,
                "statusMessage": status_message,
                "isStatusMessageTerminal": true,
            }))
            .send()
            .await
            .context("Apify run status update failed")?;
        ensure_success(response, "Apify run status update").await
    }
}

#[derive(Default)]
struct ChargingManager {
    is_pay_per_event: bool,
    event_prices: HashMap<String, f64>,
    charged_counts: HashMap<String, u64>,
    max_total_charge_usd: f64,
}

impl ChargingManager {
    fn from_run(run_response: &Value) -> Result<Self> {
        let run = run_response
            .get("data")
            .filter(|data| data.is_object())
            .unwrap_or(run_response);
        let pricing = run.get("pricingInfo");
        let is_pay_per_event = pricing
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self::default());
        }

        let event_definitions = pricing
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (event_name, definition) in event_definitions {
            if let Some(price) = definition.get("eventPriceUsd").and_then(Value::as_f64) {
                if !price.is_finite() || price < 0.0 {
                    bail!("Apify run returned an invalid price for event {event_name}");
                }
                event_prices.insert(event_name.clone(), price);
            }
        }

        let max_total_charge_usd = match run.pointer("/options/maxTotalChargeUsd") {
            None | Some(Value::Null) => f64::INFINITY,
            Some(value) => {
                let limit = value
                    .as_f64()
                    .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
                if !limit.is_finite() || limit < 0.0 {
                    bail!("Apify run returned an invalid spending limit");
                }
                limit
            }
        };

        let mut charged_counts = HashMap::new();
        if let Some(counts) = run.get("chargedEventCounts").and_then(Value::as_object) {
            for (event_name, count) in counts {
                let count = count.as_u64().ok_or_else(|| {
                    anyhow!("Apify run returned an invalid charged count for {event_name}")
                })?;
                charged_counts.insert(event_name.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event,
            event_prices,
            charged_counts,
            max_total_charge_usd,
        })
    }

    fn max_items_within_budget(&self, requested: usize) -> Result<usize> {
        if !self.is_pay_per_event {
            return Ok(requested);
        }

        let event_price = self
            .event_prices
            .get(FINANCE_SEARCH_RESULT_EVENT)
            .copied()
            .ok_or_else(|| {
                anyhow!("Apify run did not provide a price for {FINANCE_SEARCH_RESULT_EVENT}")
            })?;
        let dataset_item_price = self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0.0);
        let item_price = event_price + dataset_item_price;
        if !item_price.is_finite() {
            bail!("Apify run returned invalid charging values");
        }
        if item_price == 0.0 || self.max_total_charge_usd.is_infinite() {
            return Ok(requested);
        }

        let charged_total =
            self.charged_counts
                .iter()
                .try_fold(0.0, |total, (event_name, count)| {
                    let price = self.event_prices.get(event_name).copied().unwrap_or(0.0);
                    let next_total = total + price * (*count as f64);
                    if next_total.is_finite() {
                        Ok(next_total)
                    } else {
                        Err(anyhow!("Apify run returned invalid charged totals"))
                    }
                })?;
        let charged_total = (charged_total * 1_000_000.0).round() / 1_000_000.0;
        let remaining = self.max_total_charge_usd - charged_total;
        if remaining <= 0.0 {
            return Ok(0);
        }

        // Match the SDK's four-decimal rounding before flooring to avoid float noise at the cap.
        let affordable = ((remaining / item_price) * 10_000.0).round() / 10_000.0;
        Ok(requested.min(affordable.floor() as usize))
    }

    fn record_saved_items(&mut self, count: usize) -> Result<()> {
        if !self.is_pay_per_event || count == 0 {
            return Ok(());
        }
        increment_count(&mut self.charged_counts, FINANCE_SEARCH_RESULT_EVENT, count)?;
        if self.event_prices.contains_key(DEFAULT_DATASET_ITEM_EVENT) {
            increment_count(&mut self.charged_counts, DEFAULT_DATASET_ITEM_EVENT, count)?;
        }
        Ok(())
    }
}

fn increment_count(
    counts: &mut HashMap<String, u64>,
    event_name: &str,
    count: usize,
) -> Result<()> {
    let count = u64::try_from(count).context("Dataset row count is too large")?;
    let current = counts.entry(event_name.to_owned()).or_default();
    *current = current
        .checked_add(count)
        .ok_or_else(|| anyhow!("Charged event count overflowed"))?;
    Ok(())
}

#[derive(Debug)]
enum ScrappaFailure {
    Timeout,
    Http { status: u16, details: String },
    Network(String),
}

impl fmt::Display for ScrappaFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(formatter, "Scrappa API request timed out after 30000ms"),
            Self::Http { status, details } => {
                write!(formatter, "Scrappa API error ({status}): {details}")
            }
            Self::Network(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ScrappaFailure {}

impl ScrappaFailure {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout | Self::Network(_) => true,
            Self::Http { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
        }
    }
}

struct ScrappaClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

impl ScrappaClient<'_> {
    async fn get_search(&self, params: &GoogleFinanceSearchRequest) -> Result<Value> {
        let mut last_error = None;
        for failed_attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send_search(params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let retryable = is_retryable_scrappa_error(&error);
                    if !retryable || failed_attempt == SCRAPPA_MAX_ATTEMPTS {
                        return Err(error);
                    }
                    let delay_ms = get_retry_delay_ms(failed_attempt, retry_jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        failed_attempt + 1,
                        SCRAPPA_MAX_ATTEMPTS,
                        delay_ms
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                    last_error = Some(error);
                }
            }
        }
        Err(last_error.expect("at least one Scrappa attempt is made"))
    }

    async fn send_search(&self, params: &GoogleFinanceSearchRequest) -> Result<Value> {
        let operation = async {
            let mut url =
                endpoint_url(&self.config.scrappa_api_base, &["google-finance", "search"])?;
            {
                let mut query = url.query_pairs_mut();
                query.append_pair("q", &params.q);
                if let Some(hl) = &params.hl {
                    query.append_pair("hl", hl);
                }
                if let Some(gl) = &params.gl {
                    query.append_pair("gl", gl);
                }
            }

            let response = self
                .http
                .get(url)
                .timeout(SCRAPPA_REQUEST_TIMEOUT)
                .header("X-API-Key", &self.config.scrappa_api_key)
                .header(header::ACCEPT, "application/json")
                .header(header::USER_AGENT, SCRAPPA_USER_AGENT)
                .send()
                .await
                .map_err(scrappa_transport_error)?;

            let status = response.status();
            if !status.is_success() {
                let fallback = status.canonical_reason().unwrap_or("HTTP error");
                let body = match response.text().await {
                    Ok(body) => body,
                    Err(error) if error.is_timeout() => {
                        return Err(anyhow!(ScrappaFailure::Timeout));
                    }
                    Err(_) => String::new(),
                };
                let details = parse_scrappa_error_body(&body, fallback);
                return Err(anyhow!(ScrappaFailure::Http {
                    status: status.as_u16(),
                    details,
                }));
            }

            let body = response.text().await.map_err(scrappa_transport_error)?;
            serde_json::from_str(&body).context("Scrappa API response was not valid JSON")
        };

        match timeout(SCRAPPA_REQUEST_TIMEOUT, operation).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!(ScrappaFailure::Timeout)),
        }
    }
}

fn scrappa_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        return anyhow!(ScrappaFailure::Timeout);
    }
    if error.is_connect() || looks_like_network_error(&error.to_string()) {
        return anyhow!(ScrappaFailure::Network(error.to_string()));
    }
    anyhow::Error::new(error)
}

fn looks_like_network_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    [
        "fetch failed",
        "failed to fetch",
        "network",
        "terminated",
        "reset",
        "econnrefused",
        "econnreset",
        "socket hang up",
        "chunk",
    ]
    .iter()
    .any(|fragment| message.contains(fragment))
}

fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaFailure>()
        .is_some_and(ScrappaFailure::is_retryable)
}

fn describe_transient_failure(error: &anyhow::Error) -> String {
    match error.downcast_ref::<ScrappaFailure>() {
        Some(ScrappaFailure::Http { status, .. }) => {
            format!("Scrappa upstream returned {status} after retries")
        }
        Some(ScrappaFailure::Timeout) => "Scrappa API request timed out after 30000ms".to_owned(),
        Some(ScrappaFailure::Network(message)) => message.clone(),
        None => error.to_string(),
    }
}

fn get_retry_delay_ms(failed_attempt: u32, jitter_ms: u64) -> u64 {
    (1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt)) + jitter_ms).min(10_000)
}

fn retry_jitter_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::from(duration.subsec_micros() % 1000))
        .unwrap_or(0)
}

fn parse_scrappa_error_body(body: &str, fallback: &str) -> String {
    if let Ok(data) = serde_json::from_str::<Value>(body) {
        if let Some(object) = data.as_object() {
            let mut message = object
                .get("message")
                .filter(|value| !value.is_null())
                .map(js_string)
                .unwrap_or_else(|| fallback.to_owned());
            if let Some(errors) = object.get("errors").and_then(Value::as_object) {
                let details = errors
                    .iter()
                    .filter_map(|(field, messages)| {
                        let messages = messages.as_array()?;
                        Some(format!(
                            "{field}: {}",
                            messages
                                .iter()
                                .map(js_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
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
    }
    if body.is_empty() {
        return fallback.to_owned();
    }
    collapse_whitespace(body, 500)
}

fn collapse_whitespace(value: &str, max_utf16_units: usize) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut result = String::new();
    let mut units = 0;
    for character in collapsed.chars() {
        let width = character.len_utf16();
        if units + width > max_utf16_units {
            break;
        }
        result.push(character);
        units += width;
    }
    result
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct GoogleFinanceSearchRequest {
    q: String,
    hl: Option<String>,
    gl: Option<String>,
}

fn build_search_requests(input: &Value) -> Result<Vec<GoogleFinanceSearchRequest>> {
    let input = input.as_object();
    let get = |field: &str| input.and_then(|input| input.get(field));
    let hl = clean_language_code(get("hl"))?;
    let gl = clean_country_code(get("gl"))?;
    let queries = match get("queries") {
        Some(value) if !value.is_null() => {
            let queries = value
                .as_array()
                .ok_or_else(|| anyhow!("queries must be an array of strings"))?;
            if queries.len() > MAX_QUERIES_PER_RUN {
                bail!("queries can include at most {MAX_QUERIES_PER_RUN} items per run");
            }
            let queries = queries
                .iter()
                .enumerate()
                .map(|(index, query)| {
                    clean_required_string(Some(query), &format!("queries[{index}]"), 255)
                })
                .collect::<Result<Vec<_>>>()?;
            if queries.is_empty() {
                bail!("queries must include at least one query");
            }
            queries
        }
        _ => vec![clean_required_string(get("q"), "q", 255)?],
    };

    Ok(queries
        .into_iter()
        .map(|q| GoogleFinanceSearchRequest {
            q,
            hl: hl.clone(),
            gl: gl.clone(),
        })
        .collect())
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .ok_or_else(|| anyhow!("{field} must be a string"))?;
    if value.is_empty() {
        return Ok(None);
    }
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(trimmed.to_owned()))
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_language_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(value) = clean_string(value, "hl", 10)? else {
        return Ok(None);
    };
    let mut parts = value.split('-');
    let language = parts.next().unwrap_or_default();
    let region = parts.next();
    if !language.is_ascii()
        || language.len() != 2
        || !language.bytes().all(|byte| byte.is_ascii_alphabetic())
        || region.is_some_and(|region| {
            !region.is_ascii()
                || region.len() != 2
                || !region.bytes().all(|byte| byte.is_ascii_alphabetic())
        })
        || parts.next().is_some()
    {
        bail!("hl must be a two-letter language code with an optional two-letter region");
    }
    Ok(Some(value.to_ascii_lowercase()))
}

fn clean_country_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(value) = clean_string(value, "gl", 10)? else {
        return Ok(None);
    };
    if !value.is_ascii()
        || value.len() != 2
        || !value.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        bail!("gl must be a two-letter country code");
    }
    Ok(Some(value.to_ascii_lowercase()))
}

fn describe_search_request(params: &GoogleFinanceSearchRequest) -> String {
    let filters = [
        params.hl.as_ref().map(|value| format!("hl={value}")),
        params.gl.as_ref().map(|value| format!("gl={value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    if filters.is_empty() {
        format!("\"{}\"", params.q)
    } else {
        format!("\"{}\" ({})", params.q, filters.join(", "))
    }
}

fn as_record(value: &Value) -> &Map<String, Value> {
    value.as_object().unwrap_or_else(|| empty_record())
}

fn empty_record() -> &'static Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Map::new)
}

fn first_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().flatten().find_map(|value| {
        value
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
    })
}

fn first_number(values: &[Option<&Value>]) -> Option<f64> {
    values.iter().flatten().find_map(|value| match value {
        Value::Number(number) => number.as_f64().filter(|number| number.is_finite()),
        Value::String(value) => {
            let cleaned = value.replace(',', "");
            let cleaned = cleaned.trim();
            if cleaned.is_empty() {
                return None;
            }
            parse_js_number(cleaned).filter(|number| number.is_finite())
        }
        _ => None,
    })
}

fn parse_js_number(value: &str) -> Option<f64> {
    let (radix, digits) = if let Some(value) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        (16, value)
    } else if let Some(value) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        (2, value)
    } else if let Some(value) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        (8, value)
    } else {
        return value.parse().ok();
    };
    u64::from_str_radix(digits, radix)
        .ok()
        .map(|number| number as f64)
}

fn build_google_finance_url(record: &Map<String, Value>) -> Option<String> {
    if let Some(link) = first_string(&[
        record.get("link"),
        record.get("url"),
        record.get("google_finance_url"),
    ]) {
        return Some(link);
    }
    if let Some(stock) = first_string(&[record.get("stock")]) {
        return Some(format!(
            "https://www.google.com/finance/quote/{}",
            encode_uri_component(&stock)
        ));
    }
    let symbol = first_string(&[record.get("symbol")])?;
    let exchange = first_string(&[record.get("exchange")])?;
    Some(format!(
        "https://www.google.com/finance/quote/{}",
        encode_uri_component(&format!("{symbol}:{exchange}"))
    ))
}

fn encode_uri_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
        {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn extract_search_results(response: &Value) -> &[Value] {
    for key in ["results", "search_results", "items"] {
        if let Some(results) = response
            .get(key)
            .and_then(Value::as_array)
            .filter(|results| !results.is_empty())
        {
            return results;
        }
    }
    if let Some(data) = response.get("data").and_then(Value::as_object) {
        for key in ["results", "search_results", "items"] {
            if let Some(results) = data
                .get(key)
                .and_then(Value::as_array)
                .filter(|results| !results.is_empty())
            {
                return results;
            }
        }
    }
    &[]
}

fn build_dataset_items(response: &Value, params: &GoogleFinanceSearchRequest) -> Vec<Value> {
    extract_search_results(response)
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let record = as_record(result);
            let price_movement = record.get("price_movement").and_then(Value::as_object);
            let google_finance_url = build_google_finance_url(record);
            json!({
                "query": params.q,
                "position": index + 1,
                "name": first_string(&[record.get("name"), record.get("title")]),
                "symbol": first_string(&[record.get("symbol")]),
                "exchange": first_string(&[record.get("exchange")]),
                "stock": first_string(&[record.get("stock")]),
                "type": first_string(&[record.get("type"), record.get("instrument_type"), record.get("asset_type")]),
                "currency": first_string(&[record.get("currency")]),
                "price": first_number(&[record.get("price"), record.get("extracted_price"), record.get("current_price")]),
                "price_change": first_number(&[record.get("price_change"), record.get("change"), price_movement.and_then(|movement| movement.get("value"))]),
                "percent_change": first_number(&[record.get("percent_change"), record.get("change_percent"), price_movement.and_then(|movement| movement.get("percentage"))]),
                "link": first_string(&[record.get("link"), record.get("url")]).or_else(|| google_finance_url.clone()),
                "google_finance_url": google_finance_url,
                "market": first_string(&[record.get("market"), record.get("region")]),
                "request_hl": params.hl,
                "request_gl": params.gl,
                "raw_result": Value::Object(record.clone()),
            })
        })
        .collect()
}

fn count_search_results(response: &Value) -> usize {
    extract_search_results(response).len()
}

struct PushSearchItemsResult {
    pushed: bool,
    status_message: Option<String>,
}

async fn push_search_items(
    apify: &ApifyClient<'_>,
    charging: &mut ChargingManager,
    items: &[Value],
    charge_sequence: usize,
) -> Result<PushSearchItemsResult> {
    if items.is_empty() {
        return Ok(PushSearchItemsResult {
            pushed: true,
            status_message: None,
        });
    }

    let limit = charging.max_items_within_budget(items.len())?;
    let saved_items = &items[..limit];
    if !saved_items.is_empty() {
        apify.push_dataset_items(saved_items).await?;
        if charging.is_pay_per_event {
            let idempotency_key = format!(
                "finance-search-{}-{charge_sequence}",
                apify.config.actor_run_id
            );
            apify
                .charge_event(
                    FINANCE_SEARCH_RESULT_EVENT,
                    saved_items.len(),
                    &idempotency_key,
                )
                .await?;
            charging.record_saved_items(saved_items.len())?;
        }
    }

    if limit < items.len() {
        return Ok(PushSearchItemsResult {
            pushed: false,
            status_message: Some(
                "Charge limit reached before saving all Google Finance search results.".to_owned(),
            ),
        });
    }
    Ok(PushSearchItemsResult {
        pushed: true,
        status_message: None,
    })
}

fn build_transient_failure_status_message(
    failure_message: &str,
    total_results_written: usize,
    total_queries: usize,
) -> String {
    if total_results_written > 0 {
        format!(
            "{failure_message}; {total_results_written} Google Finance search results were already written and may have been charged. Remaining queries were not completed. Try the run again later for the unfinished queries."
        )
    } else if total_queries > 1 {
        format!(
            "{failure_message}; no Google Finance search results were written or charged. Remaining batch queries were not completed. Try the run again later."
        )
    } else {
        format!(
            "{failure_message}; no Google Finance search results were written or charged. Try the run again later."
        )
    }
}

async fn run_actor() -> Result<()> {
    let config = Config::from_env()?;
    let http = Client::builder()
        .build()
        .context("Failed to create HTTP client")?;
    let apify = ApifyClient {
        http: &http,
        config: &config,
    };
    let mut total_results = 0;
    let mut total_queries = 0;
    let mut zero_result_queries = 0;
    let mut charge_sequence = 0;

    let execution = async {
        let run = apify.get_run().await?;
        let mut charging = ChargingManager::from_run(&run)?;
        let input = apify.get_input().await?;
        if input.is_null() {
            bail!("Input is required");
        }
        let requests = build_search_requests(&input)?;
        total_queries = requests.len();
        let scrappa = ScrappaClient {
            http: &http,
            config: &config,
        };

        for params in requests {
            eprintln!(
                "Searching Google Finance for {}",
                describe_search_request(&params)
            );
            let response = scrappa.get_search(&params).await?;
            let dataset_items = build_dataset_items(&response, &params);
            let result_count = count_search_results(&response);
            if dataset_items.is_empty() {
                zero_result_queries += 1;
                eprintln!(
                    "No Google Finance search results found for {}",
                    describe_search_request(&params)
                );
                continue;
            }

            charge_sequence += 1;
            let push_result = push_search_items(&apify, &mut charging, &dataset_items, charge_sequence).await?;
            if !push_result.pushed {
                return Ok(push_result.status_message);
            }
            total_results += dataset_items.len();
            eprintln!(
                "Saved {} Google Finance search results for {} (raw count: {})",
                dataset_items.len(),
                describe_search_request(&params),
                result_count
            );
        }

        eprintln!("Google Finance search scraping completed successfully");
        eprintln!(
            "Results summary: {}",
            json!({"queries": total_queries, "total_results": total_results, "zero_result_queries": zero_result_queries})
        );
        Ok(None)
    }
    .await;

    match execution {
        Ok(Some(status_message)) => {
            eprintln!("{status_message}");
            if let Err(error) = apify.update_status(&status_message).await {
                eprintln!("Failed to update Actor status message: {error:#}");
            }
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(error) if is_retryable_scrappa_error(&error) => {
            let status_message = build_transient_failure_status_message(
                &describe_transient_failure(&error),
                total_results,
                total_queries,
            );
            eprintln!("{status_message}");
            if let Err(status_error) = apify.update_status(&status_message).await {
                eprintln!("Failed to update Actor status message: {status_error:#}");
            }
            if total_results > 0 || total_queries > 1 {
                Err(anyhow!(status_message))
            } else {
                Ok(())
            }
        }
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            if let Err(status_error) = apify.update_status(&message).await {
                eprintln!("Failed to update Actor status message: {status_error:#}");
            }
            Err(anyhow!(message))
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run_actor().await {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_single_and_batch_requests_with_compatibility_priority() {
        let single = build_search_requests(&json!({"q":" AAPL ","hl":"EN","gl":"US"})).unwrap();
        assert_eq!(
            single,
            vec![GoogleFinanceSearchRequest {
                q: "AAPL".to_owned(),
                hl: Some("en".to_owned()),
                gl: Some("us".to_owned()),
            }]
        );

        let batch = build_search_requests(&json!({
            "q": "ignored",
            "queries": [" Tesla ", "MSFT"],
            "hl": "en"
        }))
        .unwrap();
        assert_eq!(
            batch
                .iter()
                .map(|query| query.q.as_str())
                .collect::<Vec<_>>(),
            ["Tesla", "MSFT"]
        );
        assert!(batch.iter().all(|query| query.gl.is_none()));
    }

    #[test]
    fn rejects_invalid_queries_and_locale_codes() {
        assert_eq!(
            build_search_requests(&json!({})).unwrap_err().to_string(),
            "q is required"
        );
        assert_eq!(
            build_search_requests(&json!({"queries":[]}))
                .unwrap_err()
                .to_string(),
            "queries must include at least one query"
        );
        assert_eq!(
            build_search_requests(&json!({"queries":["AAPL",123]}))
                .unwrap_err()
                .to_string(),
            "queries[1] must be a string"
        );
        assert_eq!(
            build_search_requests(&json!({"q":"AAPL","hl":"english"}))
                .unwrap_err()
                .to_string(),
            "hl must be a two-letter language code with an optional two-letter region"
        );
        assert_eq!(
            build_search_requests(&json!({"q":"AAPL","gl":"usa"}))
                .unwrap_err()
                .to_string(),
            "gl must be a two-letter country code"
        );
        assert!(build_search_requests(&json!({"q":"x".repeat(256)}))
            .unwrap_err()
            .to_string()
            .contains("255 characters or fewer"));
    }

    #[test]
    fn enforces_batch_cap_and_javascript_utf16_query_length() {
        let too_many = (0..=MAX_QUERIES_PER_RUN)
            .map(|index| json!(format!("query-{index}")))
            .collect::<Vec<_>>();
        assert!(build_search_requests(&json!({"queries":too_many}))
            .unwrap_err()
            .to_string()
            .contains("at most 25"));
        assert!(build_search_requests(&json!({"q":"😀".repeat(128)}))
            .unwrap_err()
            .to_string()
            .contains("255 characters or fewer"));
    }

    #[test]
    fn builds_finance_result_fields_and_google_url() {
        let params = GoogleFinanceSearchRequest {
            q: "AAPL".to_owned(),
            hl: Some("en".to_owned()),
            gl: Some("us".to_owned()),
        };
        let response = json!({"results":[{
            "stock":"AAPL:NASDAQ", "name":"Apple Inc", "symbol":"AAPL", "exchange":"NASDAQ",
            "type":"Stock", "currency":"USD", "price":"1,176.85",
            "price_movement":{"value":"2.34","percentage":"1.33"}
        }]});
        let items = build_dataset_items(&response, &params);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["query"], "AAPL");
        assert_eq!(items[0]["position"], 1);
        assert_eq!(items[0]["name"], "Apple Inc");
        assert_eq!(items[0]["price"], 1176.85);
        assert_eq!(items[0]["price_change"], 2.34);
        assert_eq!(items[0]["percent_change"], 1.33);
        assert_eq!(
            items[0]["google_finance_url"],
            "https://www.google.com/finance/quote/AAPL%3ANASDAQ"
        );
        assert_eq!(items[0]["raw_result"], response["results"][0]);
    }

    #[test]
    fn finds_alternate_wrappers_and_uses_fallback_fields() {
        let params = GoogleFinanceSearchRequest {
            q: "Tesla".to_owned(),
            hl: None,
            gl: None,
        };
        let response = json!({"results":[],"data":{"search_results":[{
            "title":"Tesla Inc", "symbol":"TSLA", "exchange":"NASDAQ", "change":"4.5",
            "change_percent":"1.25", "url":"https://example.test/tesla", "region":"US"
        }]}});
        assert_eq!(count_search_results(&response), 1);
        let item = &build_dataset_items(&response, &params)[0];
        assert_eq!(item["name"], "Tesla Inc");
        assert_eq!(item["price_change"], 4.5);
        assert_eq!(item["percent_change"], 1.25);
        assert_eq!(item["link"], "https://example.test/tesla");
        assert_eq!(item["market"], "US");
        assert!(item["request_hl"].is_null());
    }

    #[test]
    fn empty_or_unrecognized_search_payloads_produce_no_items() {
        let params = GoogleFinanceSearchRequest {
            q: "missing".to_owned(),
            hl: None,
            gl: None,
        };
        assert_eq!(count_search_results(&json!({"results":[]})), 0);
        assert!(build_dataset_items(&json!({}), &params).is_empty());
    }

    #[test]
    fn parses_scrappa_error_messages_and_fallback_text() {
        assert_eq!(
            parse_scrappa_error_body(
                r#"{"message":"Invalid input","errors":{"q":["required","too short"]}}"#,
                "Bad Request"
            ),
            "Invalid input - q: required, too short"
        );
        assert_eq!(
            parse_scrappa_error_body(" upstream   is down ", "Service Unavailable"),
            "upstream is down"
        );
        assert_eq!(parse_scrappa_error_body("", "Not Found"), "Not Found");
    }

    #[test]
    fn retries_only_transient_scrappa_errors_with_bounded_backoff() {
        for status in [408, 429, 500, 502, 503, 504] {
            let error = anyhow!(ScrappaFailure::Http {
                status,
                details: "failure".to_owned()
            });
            assert!(is_retryable_scrappa_error(&error));
        }
        let not_found = anyhow!(ScrappaFailure::Http {
            status: 404,
            details: "not found".to_owned()
        });
        assert!(!is_retryable_scrappa_error(&not_found));
        assert!(looks_like_network_error("read ECONNRESET"));
        assert!(!looks_like_network_error("invalid JSON response"));
        assert_eq!(get_retry_delay_ms(1, 0), 2000);
        assert_eq!(get_retry_delay_ms(20, 500), 10000);
    }

    #[test]
    fn accounts_for_existing_and_local_pay_per_event_charges() {
        let run = json!({"data":{
            "pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{
                "finance-search-result":{"eventPriceUsd":0.05},
                "apify-default-dataset-item":{"eventPriceUsd":0.01},
                "other":{"eventPriceUsd":0.01}
            }}},
            "chargedEventCounts":{"other":2},
            "options":{"maxTotalChargeUsd":0.13}
        }});
        let mut charging = ChargingManager::from_run(&run).unwrap();
        assert_eq!(charging.max_items_within_budget(10).unwrap(), 1);
        charging.record_saved_items(1).unwrap();
        assert_eq!(charging.max_items_within_budget(10).unwrap(), 0);
    }

    #[test]
    fn free_pricing_keeps_all_results_and_zero_price_events_are_unlimited() {
        let free =
            ChargingManager::from_run(&json!({"data":{"pricingInfo":{"pricingModel":"FREE"}}}))
                .unwrap();
        assert_eq!(free.max_items_within_budget(10).unwrap(), 10);
        let no_charge = ChargingManager::from_run(&json!({"data":{
            "pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{
                "finance-search-result":{"eventPriceUsd":0.0}
            }}},
            "chargedEventCounts":{},"options":{"maxTotalChargeUsd":0.0}
        }}))
        .unwrap();
        assert_eq!(no_charge.max_items_within_budget(10).unwrap(), 10);
    }

    #[test]
    fn reports_partial_results_and_batch_failures_clearly() {
        assert_eq!(
            build_transient_failure_status_message("Scrappa upstream returned 503 after retries", 0, 1),
            "Scrappa upstream returned 503 after retries; no Google Finance search results were written or charged. Try the run again later."
        );
        assert!(
            build_transient_failure_status_message("failure", 7, 2).contains(
                "7 Google Finance search results were already written and may have been charged"
            )
        );
        assert!(build_transient_failure_status_message("failure", 0, 2).contains("no Google Finance search results were written or charged. Remaining batch queries were not completed"));
    }
}
