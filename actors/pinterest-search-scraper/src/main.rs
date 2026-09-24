use std::{
    collections::HashSet,
    env,
    process::ExitCode,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use percent_encoding::percent_decode_str;
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Map, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_RETRY_MIN_DELAY: Duration = Duration::from_millis(500);
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const APIFY_MAX_RETRIES: usize = 8;
const MAX_DATASET_REQUEST_BYTES: usize = 4 * 1024 * 1024;
const PIN_RESULT_CHARGE_EVENT: &str = "pin-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
static CHARGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Config {
    apify_api_base: Url,
    apify_token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_base: Url,
    scrappa_api_key: Option<String>,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base: apify_api_base_from_env()?,
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            scrappa_api_key: env::var("SCRAPPA_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
        })
    }
}

fn apify_api_base_from_env() -> Result<Url> {
    let raw_url = env::var("APIFY_API_BASE_URL")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("APIFY_API_PUBLIC_BASE_URL")
                .ok()
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| APIFY_API_DEFAULT.to_owned());
    Url::parse(&raw_url).context("APIFY_API_BASE_URL must be a valid absolute URL")
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

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env_or_default(name, default);
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
    http: Client,
    base_url: Url,
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

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.base_url, segments)
    }

    async fn send_with_retry<F>(&self, operation: &str, mut build_request: F) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        let mut retry_count = 0;
        loop {
            match build_request().timeout(APIFY_REQUEST_TIMEOUT).send().await {
                Ok(response) => {
                    if retry_count < APIFY_MAX_RETRIES && retryable_apify_status(response.status())
                    {
                        let delay = apify_retry_delay(retry_count);
                        drop(response);
                        tokio::time::sleep(delay).await;
                        retry_count += 1;
                        continue;
                    }
                    return Ok(response);
                }
                Err(error)
                    if retry_count < APIFY_MAX_RETRIES
                        && (error.is_timeout() || error.is_connect() || error.is_request()) =>
                {
                    tokio::time::sleep(apify_retry_delay(retry_count)).await;
                    retry_count += 1;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify {operation} request failed"));
                }
            }
        }
    }

    async fn get_actor_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let response = self
            .send_with_retry("run pricing", || {
                self.http
                    .get(url.clone())
                    .bearer_auth(&self.token)
                    .header(header::ACCEPT, "application/json")
            })
            .await?;
        response_json(response, "run pricing request").await
    }

    async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .send_with_retry("input retrieval", || {
                self.http
                    .get(url.clone())
                    .bearer_auth(&self.token)
                    .header(header::ACCEPT, "application/json")
            })
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let response = require_apify_success(response, "input retrieval").await?;
        let body = response
            .bytes()
            .await
            .context("Failed to read actor input from Apify API")?;
        serde_json::from_slice(&body)
            .context("Apify input record was not valid JSON")
            .map(Some)
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        for chunk in dataset_item_chunks(items)? {
            let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
            let response = self
                .send_with_retry("dataset write", || {
                    self.http
                        .post(url.clone())
                        .bearer_auth(&self.token)
                        .header(header::ACCEPT, "application/json")
                        .json(&chunk)
                })
                .await?;
            require_apify_success(response, "dataset item publication").await?;
        }
        Ok(())
    }

    async fn charge_event(&self, event_name: &str, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }

        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id, "charge"])?;
        let idempotency_key = self.new_idempotency_key();
        let response = self
            .send_with_retry("event charge", || {
                self.http
                    .post(url.clone())
                    .bearer_auth(&self.token)
                    .header(header::ACCEPT, "application/json")
                    .header("idempotency-key", &idempotency_key)
                    .json(&json!({ "eventName": event_name, "count": count }))
            })
            .await?;
        require_apify_success(response, "event charge").await?;
        Ok(())
    }

    async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .send_with_retry("OUTPUT write", || {
                self.http
                    .put(url.clone())
                    .bearer_auth(&self.token)
                    .header(header::ACCEPT, "application/json")
                    .json(output)
            })
            .await?;
        require_apify_success(response, "OUTPUT record publication").await?;
        Ok(())
    }

    async fn set_status_message(&self, message: &str, level: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let response = self
            .send_with_retry("status message update", || {
                self.http
                    .put(url.clone())
                    .bearer_auth(&self.token)
                    .header(header::ACCEPT, "application/json")
                    .json(&json!({
                        "statusMessage": message,
                        "isStatusMessageTerminal": true,
                        "level": level
                    }))
            })
            .await?;
        require_apify_success(response, "status message update").await?;
        Ok(())
    }

    fn new_idempotency_key(&self) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let sequence = CHARGE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        format!(
            "pinterest-search-{}-{timestamp}-{sequence}",
            self.actor_run_id
        )
    }
}

fn retryable_apify_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn apify_retry_delay(retry_count: usize) -> Duration {
    let exponent = u32::try_from(retry_count).unwrap_or(u32::MAX);
    APIFY_RETRY_MIN_DELAY.saturating_mul(2_u32.saturating_pow(exponent))
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let response = require_apify_success(response, operation).await?;
    response
        .json::<Value>()
        .await
        .with_context(|| format!("Apify {operation} response was not valid JSON"))
}

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

fn dataset_item_chunks(items: &[Value]) -> Result<Vec<Vec<Value>>> {
    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_size = 2;

    for item in items {
        let item_size = serde_json::to_vec(item)
            .context("Failed to encode dataset item")?
            .len();
        if item_size + 2 > MAX_DATASET_REQUEST_BYTES {
            bail!("Pinterest dataset item exceeds the Apify dataset request size limit");
        }

        let added_size = item_size + usize::from(!current.is_empty());
        if !current.is_empty() && current_size + added_size > MAX_DATASET_REQUEST_BYTES {
            chunks.push(std::mem::take(&mut current));
            current_size = 2;
        }
        current_size += item_size + usize::from(!current.is_empty());
        current.push(item.clone());
    }

    if !current.is_empty() {
        chunks.push(current);
    }
    Ok(chunks)
}

#[derive(Debug)]
struct ScrappaApiError {
    status: u16,
    message: String,
}

impl std::fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.message
        )
    }
}

impl std::error::Error for ScrappaApiError {}

#[derive(Debug)]
struct ScrappaTimeoutError;

impl std::fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            SCRAPPA_REQUEST_TIMEOUT.as_millis()
        )
    }
}

impl std::error::Error for ScrappaTimeoutError {}

struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
}

impl ScrappaClient {
    fn new(base_url: Url, api_key: String) -> Result<Self> {
        let http = Client::builder()
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .build()
            .context("Failed to create Scrappa HTTP client")?;
        Ok(Self {
            http,
            base_url,
            api_key,
        })
    }

    async fn pinterest_search(&self, params: &PinterestSearchParams) -> Result<Value> {
        let mut url = endpoint_url(&self.base_url, &["pinterest", "search"])?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("query", &params.query);
            query.append_pair("limit", &params.limit.to_string());
            if let Some(bookmark) = &params.bookmark {
                query.append_pair("bookmark", bookmark);
            }
        }

        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send_search(&url).await {
                Ok(response) => return Ok(response),
                Err(error)
                    if attempt < SCRAPPA_MAX_ATTEMPTS && is_retryable_scrappa_error(&error) =>
                {
                    let delay = scrappa_retry_delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {}ms.",
                        error,
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("Scrappa attempts always return or error")
    }

    async fn send_search(&self, url: &Url) -> Result<Value> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(
                header::USER_AGENT,
                "thescrappa-pinterest-search-scraper/1.0",
            )
            .send()
            .await
            .map_err(scrappa_transport_error)?;

        let status = response.status();
        let body = response.text().await.map_err(scrappa_transport_error)?;
        if !status.is_success() {
            return Err(ScrappaApiError {
                status: status.as_u16(),
                message: scrappa_api_error_message(status, &body),
            }
            .into());
        }

        serde_json::from_str(&body).context("Scrappa API response was not valid JSON")
    }
}

fn scrappa_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow::Error::new(ScrappaTimeoutError)
    } else {
        anyhow::Error::new(error)
    }
}

fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if let Some(error) = error.downcast_ref::<ScrappaApiError>() {
        return matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504);
    }
    error
        .downcast_ref::<reqwest::Error>()
        .is_some_and(|error| error.is_timeout() || error.is_connect())
}

fn scrappa_retry_delay(failed_attempt: usize) -> Duration {
    let exponent = u32::try_from(failed_attempt).unwrap_or(u32::MAX);
    let base_ms = 1000_u64.saturating_mul(2_u64.saturating_pow(exponent));
    let jitter_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64;
    Duration::from_millis(base_ms.saturating_add(jitter_ms).min(10_000))
}

fn scrappa_api_error_message(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    if body.is_empty() {
        return fallback;
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        if let Some(object) = error_data.as_object() {
            let mut message = object
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or(&fallback)
                .to_owned();
            if let Some(errors) = object.get("errors").and_then(Value::as_object) {
                let details = errors
                    .iter()
                    .filter_map(|(field, messages)| {
                        messages.as_array().map(|messages| {
                            let messages = messages
                                .iter()
                                .map(|message| match message {
                                    Value::String(message) => message.clone(),
                                    value => value.to_string(),
                                })
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!("{field}: {messages}")
                        })
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

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 250;
const MAX_QUERIES_PER_RUN: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PinterestSearchParams {
    query: String,
    limit: usize,
    bookmark: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PinterestSearchPlan {
    queries: Vec<String>,
    limit: usize,
    bookmark: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PinterestSearchFetchParams {
    params: PinterestSearchParams,
    requested_limit: usize,
    fetch_limit: usize,
}

fn decode_input_string(value: &str) -> String {
    if !value.contains('%') {
        return value.to_owned();
    }
    if !has_valid_percent_escapes(value) {
        return value.to_owned();
    }
    percent_decode_str(value)
        .decode_utf8()
        .map(|decoded| decoded.into_owned())
        .unwrap_or_else(|_| value.to_owned())
}

fn has_valid_percent_escapes(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };

    let trimmed = decode_input_string(value).trim().to_owned();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(trimmed))
}

fn clean_integer(
    value: Option<&Value>,
    field: &str,
    min: usize,
    max: usize,
) -> Result<Option<usize>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }

    let normalized = match value {
        Value::String(value) => {
            let value = value.trim();
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                bail!("{field} must be an integer");
            }
            value
                .parse::<usize>()
                .with_context(|| format!("{field} must be an integer"))?
        }
        Value::Number(value) => {
            let value = value.as_f64().filter(|value| value.is_finite());
            let Some(value) = value.filter(|value| value.fract() == 0.0) else {
                bail!("{field} must be an integer");
            };
            if value < 0.0 || value > usize::MAX as f64 {
                bail!("{field} must be an integer");
            }
            value as usize
        }
        _ => bail!("{field} must be an integer"),
    };

    if normalized < min || normalized > max {
        bail!("{field} must be between {min} and {max}");
    }
    Ok(Some(normalized))
}

fn build_pinterest_search_plan(input: &Map<String, Value>) -> Result<PinterestSearchPlan> {
    let mut queries = Vec::new();
    if let Some(query) = clean_string(input.get("query"), "query", 200)? {
        queries.push(query);
    }

    if let Some(value) = input.get("queries") {
        if !value.is_null() && value.as_str() != Some("") {
            let Some(values) = value.as_array() else {
                bail!("queries must be an array of strings");
            };
            for (index, value) in values.iter().enumerate() {
                if let Some(query) = clean_string(Some(value), &format!("queries[{index}]"), 200)? {
                    queries.push(query);
                }
            }
        }
    }

    let mut seen = HashSet::new();
    queries.retain(|query| seen.insert(query.clone()));
    if queries.is_empty() {
        bail!("Provide at least one Pinterest search query using queries or query");
    }
    if queries.len() > MAX_QUERIES_PER_RUN {
        bail!("queries cannot contain more than {MAX_QUERIES_PER_RUN} values per run");
    }

    let limit = clean_integer(input.get("limit"), "limit", 1, MAX_LIMIT)?.unwrap_or(DEFAULT_LIMIT);
    let bookmark = clean_string(input.get("bookmark"), "bookmark", 2000)?;
    Ok(PinterestSearchPlan {
        queries,
        limit,
        bookmark,
    })
}

fn describe_pinterest_search_request(plan: &PinterestSearchPlan) -> String {
    let query_label = if plan.queries.len() == 1 {
        format!("\"{}\"", plan.queries[0])
    } else {
        format!("{} queries", plan.queries.len())
    };
    let bookmark_label = if plan.bookmark.is_some() {
        ", with bookmark"
    } else {
        ""
    };
    format!("{query_label} ({} pins/query{bookmark_label})", plan.limit)
}

fn cap_pinterest_search_params(
    params: &PinterestSearchParams,
    chargeable_pin_capacity: usize,
) -> PinterestSearchFetchParams {
    let requested_limit = params.limit;
    let fetch_limit = requested_limit.min(chargeable_pin_capacity);
    let mut params = params.clone();
    params.limit = fetch_limit;
    PinterestSearchFetchParams {
        params,
        requested_limit,
        fetch_limit,
    }
}

struct ChargeBudget {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: Map<String, Value>,
    charged_event_counts: Map<String, Value>,
}

impl ChargeBudget {
    fn from_actor_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_info = data.get("pricingInfo");
        let is_pay_per_event = pricing_info
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self {
                is_pay_per_event: false,
                max_total_charge_usd: f64::INFINITY,
                event_prices: Map::new(),
                charged_event_counts: Map::new(),
            });
        }

        let event_prices = pricing_info
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?
            .iter()
            .map(|(name, event)| {
                let price = event
                    .get("eventPriceUsd")
                    .and_then(Value::as_f64)
                    .filter(|price| price.is_finite() && *price >= 0.0)
                    .ok_or_else(|| anyhow!("Apify run did not provide a valid price for {name}"))?;
                Ok((name.clone(), json!(price)))
            })
            .collect::<Result<Map<String, Value>>>()?;

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|amount| amount.is_finite() && *amount >= 0.0)
            .filter(|amount| *amount != 0.0)
            .unwrap_or(f64::INFINITY);
        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (name, count) in &charged_event_counts {
            if count.as_u64().is_none() {
                bail!("Invalid charged event count for {name}");
            }
        }

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        })
    }

    fn is_pay_per_event(&self) -> bool {
        self.is_pay_per_event
    }

    fn price(&self, event_name: &str) -> Option<f64> {
        self.event_prices.get(event_name).and_then(Value::as_f64)
    }

    fn charged_count(&self, event_name: &str) -> u64 {
        self.charged_event_counts
            .get(event_name)
            .and_then(Value::as_u64)
            .unwrap_or(0)
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_event_counts
            .iter()
            .map(|(event_name, count)| {
                self.price(event_name).unwrap_or(0.0) * count.as_u64().unwrap_or(0) as f64
            })
            .sum::<f64>();
        if total.is_finite() {
            format!("{total:.6}").parse().unwrap_or(total)
        } else {
            total
        }
    }

    fn max_event_charge_count_within_limit(&self, event_name: &str) -> usize {
        let Some(price) = self.price(event_name) else {
            return usize::MAX;
        };
        if price == 0.0 {
            return usize::MAX;
        }
        self.max_charge_count_by_price(price)
    }

    fn max_charge_count_by_price(&self, price: f64) -> usize {
        if price <= 0.0 || self.max_total_charge_usd.is_infinite() {
            return usize::MAX;
        }
        let unrounded = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !unrounded.is_finite() {
            return if unrounded.is_sign_positive() {
                usize::MAX
            } else {
                0
            };
        }
        let rounded = format!("{unrounded:.4}")
            .parse::<f64>()
            .unwrap_or(unrounded);
        if rounded <= 0.0 {
            0
        } else {
            rounded.floor().min(usize::MAX as f64) as usize
        }
    }

    fn chargeable_pin_capacity(&self) -> usize {
        self.max_event_charge_count_within_limit(PIN_RESULT_CHARGE_EVENT)
    }

    fn pushable_pin_count(&self, requested_count: usize) -> usize {
        if !self.is_pay_per_event || requested_count == 0 {
            return requested_count;
        }

        let item_price = self.price(PIN_RESULT_CHARGE_EVENT).unwrap_or(0.0)
            + self.price(DEFAULT_DATASET_ITEM_EVENT).unwrap_or(0.0);
        let max_charged_count = if item_price > 0.0 {
            self.max_charge_count_by_price(item_price)
        } else {
            usize::MAX
        };
        if max_charged_count >= requested_count {
            return requested_count;
        }
        if max_charged_count == 0 && self.total_charged_amount() <= self.max_total_charge_usd {
            return 1;
        }
        max_charged_count
    }

    fn prepare_event_charge(&mut self, event_name: &str, requested_count: usize) -> usize {
        if !self.is_pay_per_event {
            return 0;
        }

        let max_event_count = self.max_event_charge_count_within_limit(event_name);
        let charged_count = if requested_count <= max_event_count {
            requested_count
        } else if self.total_charged_amount() <= self.max_total_charge_usd {
            max_event_count.saturating_add(1)
        } else {
            0
        };
        if charged_count > 0 {
            let new_count = self
                .charged_count(event_name)
                .saturating_add(charged_count as u64);
            self.charged_event_counts
                .insert(event_name.to_owned(), json!(new_count));
        }
        charged_count
    }

    fn event_charge_limit_reached(&self, event_name: &str) -> bool {
        self.max_event_charge_count_within_limit(event_name) == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PinterestPinsSource {
    Pins,
    DataPins,
    Results,
    DataResults,
}

#[derive(Debug)]
struct PinterestPinsSelection<'a> {
    pins: &'a [Value],
    source: Option<PinterestPinsSource>,
}

fn select_pinterest_pins(response: &Value) -> PinterestPinsSelection<'_> {
    if let Some(pins) = response.get("pins").and_then(Value::as_array) {
        return PinterestPinsSelection {
            pins,
            source: Some(PinterestPinsSource::Pins),
        };
    }
    if let Some(pins) = response.pointer("/data/pins").and_then(Value::as_array) {
        return PinterestPinsSelection {
            pins,
            source: Some(PinterestPinsSource::DataPins),
        };
    }
    if let Some(pins) = response.get("results").and_then(Value::as_array) {
        return PinterestPinsSelection {
            pins,
            source: Some(PinterestPinsSource::Results),
        };
    }
    if let Some(pins) = response.pointer("/data/results").and_then(Value::as_array) {
        return PinterestPinsSelection {
            pins,
            source: Some(PinterestPinsSource::DataResults),
        };
    }
    PinterestPinsSelection {
        pins: &[],
        source: None,
    }
}

fn pinterest_next_bookmark(response: &Value) -> Value {
    response
        .get("nextBookmark")
        .filter(|bookmark| !bookmark.is_null())
        .or_else(|| {
            response
                .get("bookmark")
                .filter(|bookmark| !bookmark.is_null())
        })
        .cloned()
        .unwrap_or(Value::Null)
}

fn limit_pinterest_search_response(
    response: &Value,
    limit: usize,
    selected_source: Option<PinterestPinsSource>,
) -> Value {
    let Some(response) = response.as_object() else {
        return json!({});
    };
    let source =
        selected_source.or_else(|| select_pinterest_pins(&Value::Object(response.clone())).source);
    let mut limited = response.clone();

    if source == Some(PinterestPinsSource::Pins) {
        if let Some(pins) = response.get("pins").and_then(Value::as_array) {
            limited.insert(
                "pins".to_owned(),
                json!(pins.iter().take(limit).cloned().collect::<Vec<_>>()),
            );
        }
    } else {
        limited.remove("pins");
    }
    if source == Some(PinterestPinsSource::Results) {
        if let Some(results) = response.get("results").and_then(Value::as_array) {
            limited.insert(
                "results".to_owned(),
                json!(results.iter().take(limit).cloned().collect::<Vec<_>>()),
            );
        }
    } else {
        limited.remove("results");
    }

    if let Some(data) = response.get("data").and_then(Value::as_object) {
        let mut limited_data = data.clone();
        if source == Some(PinterestPinsSource::DataPins) {
            if let Some(pins) = data.get("pins").and_then(Value::as_array) {
                limited_data.insert(
                    "pins".to_owned(),
                    json!(pins.iter().take(limit).cloned().collect::<Vec<_>>()),
                );
            }
        } else {
            limited_data.remove("pins");
        }
        if source == Some(PinterestPinsSource::DataResults) {
            if let Some(results) = data.get("results").and_then(Value::as_array) {
                limited_data.insert(
                    "results".to_owned(),
                    json!(results.iter().take(limit).cloned().collect::<Vec<_>>()),
                );
            }
        } else {
            limited_data.remove("results");
        }
        limited.insert("data".to_owned(), Value::Object(limited_data));
    }
    Value::Object(limited)
}

fn first_image_url(pin: &Map<String, Value>) -> Option<Value> {
    for field in ["image_url", "image"] {
        if let Some(Value::String(value)) = pin.get(field) {
            if !value.is_empty() {
                return Some(Value::String(value.clone()));
            }
        }
    }

    if let Some(images) = pin.get("images") {
        if let Some(images) = images.as_array() {
            for image in images {
                if let Some(image) = image.as_str() {
                    return Some(json!(image));
                }
                if let Some(url) = image.get("url").and_then(Value::as_str) {
                    return Some(json!(url));
                }
            }
        }
        if let Some(images) = images.as_object() {
            for key in ["orig", "original", "736x", "564x", "236x"] {
                let Some(value) = images.get(key) else {
                    continue;
                };
                if let Some(value) = value.as_str().filter(|value| !value.is_empty()) {
                    return Some(json!(value));
                }
                if let Some(url) = value.get("url").and_then(Value::as_str) {
                    return Some(json!(url));
                }
            }
        }
    }
    None
}

fn object_value<'a>(value: Option<&'a Value>, keys: &[&str]) -> Option<&'a Value> {
    let object = value?.as_object()?;
    keys.iter().find_map(|key| {
        object
            .get(*key)
            .filter(|value| !value.is_null() && value.as_str() != Some(""))
    })
}

fn javascript_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Number(value)) => value.as_f64().is_some_and(|value| value != 0.0),
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Array(_)) | Some(Value::Object(_)) | Some(Value::Bool(true)) => true,
    }
}

fn nullish(value: Option<&Value>) -> Value {
    value
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn pinterest_dataset_item(
    pin: &Value,
    params: &PinterestSearchParams,
    response: &Value,
) -> Result<Value> {
    let Some(pin) = pin.as_object() else {
        bail!("Pinterest pin result must be an object");
    };
    let mut item = pin.clone();
    let image_url = first_image_url(pin).unwrap_or(Value::Null);
    let pinner_id = object_value(pin.get("pinner"), &["id", "user_id"])
        .cloned()
        .unwrap_or(Value::Null);
    let pinner_username = object_value(pin.get("pinner"), &["username", "userName", "name"])
        .cloned()
        .unwrap_or(Value::Null);
    let board_id = object_value(pin.get("board"), &["id", "board_id"])
        .cloned()
        .unwrap_or(Value::Null);
    let board_name = object_value(pin.get("board"), &["name", "title"])
        .cloned()
        .unwrap_or(Value::Null);
    let response_pins_length = select_pinterest_pins(response).pins.len();
    let results_count = response
        .get("results_count")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or_else(|| json!(response_pins_length));

    let link = pin
        .get("link")
        .filter(|value| !value.is_null())
        .or_else(|| pin.get("url").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null);
    item.insert("id".to_owned(), nullish(pin.get("id")));
    item.insert("title".to_owned(), nullish(pin.get("title")));
    item.insert("description".to_owned(), nullish(pin.get("description")));
    item.insert("image_url".to_owned(), image_url);
    item.insert("link".to_owned(), link);
    item.insert("domain".to_owned(), nullish(pin.get("domain")));
    item.insert("pinner_id".to_owned(), pinner_id);
    item.insert("pinner_username".to_owned(), pinner_username);
    item.insert("board_id".to_owned(), board_id);
    item.insert("board_name".to_owned(), board_name);
    item.insert(
        "has_video".to_owned(),
        json!(javascript_truthy(pin.get("video"))),
    );
    item.insert("repin_count".to_owned(), nullish(pin.get("repin_count")));
    item.insert(
        "comment_count".to_owned(),
        nullish(pin.get("comment_count")),
    );
    item.insert("like_count".to_owned(), nullish(pin.get("like_count")));
    item.insert("save_count".to_owned(), nullish(pin.get("save_count")));
    item.insert("request_query".to_owned(), json!(params.query));
    item.insert("request_limit".to_owned(), json!(params.limit));
    item.insert(
        "request_bookmark".to_owned(),
        params
            .bookmark
            .as_ref()
            .map_or(Value::Null, |value| json!(value)),
    );
    item.insert("count".to_owned(), nullish(response.get("count")));
    item.insert("results_count".to_owned(), results_count);
    item.insert("nextBookmark".to_owned(), pinterest_next_bookmark(response));
    Ok(Value::Object(item))
}

#[derive(Debug)]
struct PinterestChargedSaveResult {
    saved_count: usize,
    charge_limit_reached: bool,
}

fn pinterest_charged_save_result(
    charged_count: usize,
    event_charge_limit_reached: bool,
    requested_count: usize,
) -> PinterestChargedSaveResult {
    let saved_count = charged_count.min(requested_count);
    PinterestChargedSaveResult {
        saved_count,
        charge_limit_reached: event_charge_limit_reached || saved_count < requested_count,
    }
}

#[derive(Debug)]
struct PushChargedItemsResult {
    saved_count: usize,
    status_message: Option<String>,
}

async fn push_charged_pins(
    apify: &ApifyClient,
    budget: &mut ChargeBudget,
    items: &[Value],
    query: &str,
) -> Result<PushChargedItemsResult> {
    if items.is_empty() {
        return Ok(PushChargedItemsResult {
            saved_count: 0,
            status_message: None,
        });
    }

    if !budget.is_pay_per_event() {
        apify.push_dataset_items(items).await?;
        return Ok(PushChargedItemsResult {
            saved_count: items.len(),
            status_message: None,
        });
    }

    let pushed_count = budget.pushable_pin_count(items.len());
    let pushed_items = &items[..pushed_count];
    if !pushed_items.is_empty() {
        apify.push_dataset_items(pushed_items).await?;
    }

    let pin_charged_count = budget.prepare_event_charge(PIN_RESULT_CHARGE_EVENT, pushed_count);
    let dataset_charged_count =
        budget.prepare_event_charge(DEFAULT_DATASET_ITEM_EVENT, pushed_count);
    if budget.event_prices.contains_key(PIN_RESULT_CHARGE_EVENT) && pin_charged_count > 0 {
        apify
            .charge_event(PIN_RESULT_CHARGE_EVENT, pin_charged_count)
            .await?;
    }

    let charge_limit_reached = if pushed_count == 0 {
        true
    } else {
        budget.event_charge_limit_reached(PIN_RESULT_CHARGE_EVENT)
            || budget.event_charge_limit_reached(DEFAULT_DATASET_ITEM_EVENT)
    };
    let charge_result = pinterest_charged_save_result(
        pin_charged_count.saturating_add(dataset_charged_count),
        charge_limit_reached,
        items.len(),
    );

    let status_message = if charge_result.charge_limit_reached {
        let message = format!(
            "Charge limit reached after saving {} of {} Pinterest pin result(s) for \"{query}\".",
            charge_result.saved_count,
            items.len()
        );
        eprintln!(
            "{message} {}",
            json!({
                "event": PIN_RESULT_CHARGE_EVENT,
                "charged_count": charge_result.saved_count,
                "requested_count": items.len(),
                "saved_count": charge_result.saved_count,
                "query": query,
            })
        );
        Some(message)
    } else {
        None
    };

    Ok(PushChargedItemsResult {
        saved_count: charge_result.saved_count,
        status_message,
    })
}

async fn execute_actor(
    config: &Config,
    apify: &ApifyClient,
    budget: &mut ChargeBudget,
) -> Result<Option<String>> {
    let api_key = config.scrappa_api_key.as_ref().ok_or_else(|| {
        anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
    })?;
    let input = apify.get_input().await?.unwrap_or_else(|| json!({}));
    let input = input.as_object().cloned().unwrap_or_default();
    let plan = build_pinterest_search_plan(&input)?;
    println!(
        "Searching Pinterest for {}",
        describe_pinterest_search_request(&plan)
    );

    let scrappa = ScrappaClient::new(config.scrappa_api_base.clone(), api_key.clone())?;
    let mut responses = Vec::new();
    let mut query_summaries = Vec::new();
    let mut searches_fetched = 0usize;
    let mut extracted_pins = 0usize;
    let mut saved_pins = 0usize;
    let mut status_message = None;

    for query in &plan.queries {
        let chargeable_pin_capacity = budget.chargeable_pin_capacity();
        if chargeable_pin_capacity == 0 {
            let message =
                format!("Charge limit reached before fetching Pinterest pins for \"{query}\".");
            eprintln!(
                "{message} {}",
                json!({"event": PIN_RESULT_CHARGE_EVENT, "query": query})
            );
            status_message = Some(message);
            break;
        }

        let params = PinterestSearchParams {
            query: query.clone(),
            limit: plan.limit,
            bookmark: plan.bookmark.clone(),
        };
        let fetch_params = cap_pinterest_search_params(&params, chargeable_pin_capacity);
        println!(
            "Fetching Pinterest pins for \"{}\" with limit {}",
            query, fetch_params.fetch_limit
        );
        let response = scrappa.pinterest_search(&fetch_params.params).await?;
        searches_fetched += 1;

        let selection = select_pinterest_pins(&response);
        let pins = selection.pins;
        let source = selection.source;
        extracted_pins += pins.len();
        let items = pins
            .iter()
            .map(|pin| pinterest_dataset_item(pin, &fetch_params.params, &response))
            .collect::<Result<Vec<_>>>()?;
        let push_result = push_charged_pins(apify, budget, &items, query).await?;
        saved_pins += push_result.saved_count;
        responses.push(limit_pinterest_search_response(
            &response,
            push_result.saved_count,
            source,
        ));
        query_summaries.push(json!({
            "query": query,
            "requested_limit": fetch_params.requested_limit,
            "fetch_limit": fetch_params.fetch_limit,
            "request_bookmark": fetch_params.params.bookmark,
            "count": nullish(response.get("count")),
            "results_count": response.get("results_count")
                .filter(|value| !value.is_null())
                .cloned()
                .unwrap_or_else(|| json!(pins.len())),
            "pins_extracted": pins.len(),
            "pins_saved": push_result.saved_count,
            "nextBookmark": pinterest_next_bookmark(&response),
        }));
        println!(
            "Found {} Pinterest pin result(s) for \"{}\"; saved {}",
            pins.len(),
            query,
            push_result.saved_count
        );
        if push_result.status_message.is_some() {
            status_message = push_result.status_message;
            break;
        }
    }

    let output = json!({
        "request": {
            "queries": plan.queries,
            "limit": plan.limit,
            "bookmark": plan.bookmark,
        },
        "searches_fetched": searches_fetched,
        "responses_saved": responses.len(),
        "pins_extracted": extracted_pins,
        "pins_saved": saved_pins,
        "status_message": status_message,
        "query_summaries": query_summaries,
        "responses": responses,
    });
    apify.put_output(&output).await?;

    println!("Pinterest search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "searches_fetched": searches_fetched,
            "responses_saved": responses.len(),
            "pins_extracted": extracted_pins,
            "pins_saved": saved_pins,
            "queries": plan.queries.len(),
        })
    );
    Ok(status_message)
}

fn actor_failure_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{message}. The Pinterest search request exceeded the {}s Scrappa API timeout. Try fewer queries, a lower limit, or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let apify = ApifyClient::new(Client::new(), &config);
    let run = apify.get_actor_run().await?;
    let mut budget = ChargeBudget::from_actor_run(&run)?;
    let result = execute_actor(&config, &apify, &mut budget).await;

    match result {
        Ok(Some(message)) => {
            match tokio::time::timeout(
                Duration::from_secs(1),
                apify.set_status_message(&message, "INFO"),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    eprintln!("Warning: failed to set the final status message: {error}")
                }
                Err(_) => eprintln!("Warning: setting the final status message timed out"),
            }
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(error) => {
            let message = actor_failure_message(&error);
            match tokio::time::timeout(
                Duration::from_secs(1),
                apify.set_status_message(&message, "ERROR"),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(status_error)) => {
                    eprintln!("Warning: failed to set the failure status message: {status_error}")
                }
                Err(_) => eprintln!("Warning: setting the failure status message timed out"),
            }
            Err(anyhow!(message))
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn mock_http_response(status: u16, body: &str) -> (Url, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let body = body.to_owned();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            let mut headers_end = None;
            let mut content_length = 0usize;
            loop {
                let count = stream.read(&mut buffer).await.unwrap();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
                if headers_end.is_none() {
                    if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                        headers_end = Some(end + 4);
                        let headers = String::from_utf8_lossy(&request[..end]);
                        content_length = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                    }
                }
                if headers_end.is_some_and(|end| request.len() >= end + content_length) {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&request).into_owned();
            let response = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            request
        });
        (Url::parse(&format!("http://{address}")).unwrap(), task)
    }

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn ppe_run(max_total: f64, charged_event_counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "pin-result": {"eventPriceUsd": 0.002},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.001},
                        "actor-start": {"eventPriceUsd": 0.001}
                    }}
                },
                "chargedEventCounts": charged_event_counts,
                "options": {"maxTotalChargeUsd": max_total}
            }
        })
    }

    fn apify_client(base_url: Url) -> ApifyClient {
        let config = Config {
            apify_api_base: base_url,
            apify_token: "test-token".to_owned(),
            actor_run_id: "run-1".to_owned(),
            key_value_store_id: "store-1".to_owned(),
            dataset_id: "dataset-1".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_base: Url::parse(SCRAPPA_API_DEFAULT).unwrap(),
            scrappa_api_key: Some("test-key".to_owned()),
        };
        ApifyClient::new(Client::new(), &config)
    }

    #[test]
    fn builds_batch_input_and_keeps_query_order() {
        let plan = build_pinterest_search_plan(&object(json!({
            "query": " home%20decor ",
            "queries": ["home decor", " Home Decor ", "kitchen ideas"],
            "limit": "25",
            "bookmark": "abc123"
        })))
        .unwrap();

        assert_eq!(plan.queries, ["home decor", "Home Decor", "kitchen ideas"]);
        assert_eq!(plan.limit, 25);
        assert_eq!(plan.bookmark.as_deref(), Some("abc123"));
        assert_eq!(
            describe_pinterest_search_request(&plan),
            "3 queries (25 pins/query, with bookmark)"
        );
    }

    #[test]
    fn decodes_uri_components_only_when_the_entire_escape_sequence_is_valid() {
        assert_eq!(decode_input_string("home%20decor"), "home decor");
        assert_eq!(decode_input_string("plus+sign"), "plus+sign");
        assert_eq!(decode_input_string("100% ready"), "100% ready");
        assert_eq!(decode_input_string("home%20decor%ZZ"), "home%20decor%ZZ");
        assert_eq!(decode_input_string("%E0%A4%A"), "%E0%A4%A");
    }

    #[test]
    fn validates_query_and_pagination_input() {
        assert!(build_pinterest_search_plan(&Map::new())
            .unwrap_err()
            .to_string()
            .contains("Provide at least one Pinterest search query"));
        assert!(
            build_pinterest_search_plan(&object(json!({"queries": "home decor"})))
                .unwrap_err()
                .to_string()
                .contains("queries must be an array of strings")
        );
        assert!(
            build_pinterest_search_plan(&object(json!({"query": "decor", "limit": 251})))
                .unwrap_err()
                .to_string()
                .contains("limit must be between 1 and 250")
        );
        assert!(
            build_pinterest_search_plan(&object(json!({"query": "decor", "bookmark": 12})))
                .unwrap_err()
                .to_string()
                .contains("bookmark must be a string")
        );
        assert!(build_pinterest_search_plan(&object(json!({"query": "x".repeat(201)}))).is_err());
    }

    #[test]
    fn caps_upstream_limit_to_the_remaining_pin_event_capacity() {
        let params = PinterestSearchParams {
            query: "home decor".to_owned(),
            limit: 250,
            bookmark: Some("next/page".to_owned()),
        };
        let fetch = cap_pinterest_search_params(&params, 3);
        assert_eq!(fetch.requested_limit, 250);
        assert_eq!(fetch.fetch_limit, 3);
        assert_eq!(fetch.params.limit, 3);
        assert_eq!(fetch.params.bookmark.as_deref(), Some("next/page"));
    }

    #[test]
    fn selects_response_shapes_in_the_same_priority_order() {
        assert_eq!(
            select_pinterest_pins(&json!({"pins": [], "data": {"pins": [{"id": 2}]}})).source,
            Some(PinterestPinsSource::Pins)
        );
        assert_eq!(
            select_pinterest_pins(&json!({"data": {"pins": [{"id": 2}]}, "results": [{"id": 3}]}))
                .source,
            Some(PinterestPinsSource::DataPins)
        );
        assert_eq!(
            select_pinterest_pins(
                &json!({"results": [{"id": 3}], "data": {"results": [{"id": 4}]}})
            )
            .source,
            Some(PinterestPinsSource::Results)
        );
        assert_eq!(
            select_pinterest_pins(&json!({"data": {"results": [{"id": 4}]}})).source,
            Some(PinterestPinsSource::DataResults)
        );
        assert!(select_pinterest_pins(&json!({})).pins.is_empty());
    }

    #[test]
    fn normalizes_pin_fields_and_limits_only_the_selected_response_array() {
        let response = json!({
            "query": "home decor",
            "count": 25,
            "pins": [{"id": "123"}, {"id": "456"}],
            "results": [{"id": "other"}]
        });
        let params = PinterestSearchParams {
            query: "home decor".to_owned(),
            limit: 25,
            bookmark: Some("abc".to_owned()),
        };
        let pin = json!({
            "id": "123",
            "title": "Storage",
            "images": {"orig": {"url": "https://example.com/pin.jpg"}},
            "link": "https://example.com/storage",
            "pinner": {"id": "u1", "username": "homeideas"},
            "board": {"id": "b1", "name": "Home"},
            "video": {"duration": 12},
            "repin_count": 5,
            "unknown_field": true
        });

        let item = pinterest_dataset_item(&pin, &params, &response).unwrap();
        assert_eq!(item["id"], "123");
        assert_eq!(item["image_url"], "https://example.com/pin.jpg");
        assert_eq!(item["link"], "https://example.com/storage");
        assert_eq!(item["pinner_id"], "u1");
        assert_eq!(item["pinner_username"], "homeideas");
        assert_eq!(item["board_id"], "b1");
        assert_eq!(item["board_name"], "Home");
        assert_eq!(item["has_video"], true);
        assert_eq!(item["request_query"], "home decor");
        assert_eq!(item["request_bookmark"], "abc");
        assert_eq!(item["results_count"], 2);
        assert_eq!(item["unknown_field"], true);

        let limited =
            limit_pinterest_search_response(&response, 1, Some(PinterestPinsSource::Pins));
        assert_eq!(limited["pins"], json!([{"id": "123"}]));
        assert!(limited.get("results").is_none());
    }

    #[test]
    fn preserves_empty_primary_results_and_bookmark_fallback() {
        assert!(select_pinterest_pins(&json!({
            "pins": [],
            "data": {"pins": [{"id": "fallback"}]}
        }))
        .pins
        .is_empty());
        assert_eq!(
            pinterest_next_bookmark(&json!({"bookmark": "fallback-token"})),
            json!("fallback-token")
        );
        assert_eq!(
            pinterest_next_bookmark(&json!({"nextBookmark": "", "bookmark": "fallback"})),
            json!("")
        );
    }

    #[test]
    fn caps_ppe_results_using_combined_named_and_dataset_event_costs() {
        let mut budget =
            ChargeBudget::from_actor_run(&ppe_run(0.01, json!({"actor-start": 1}))).unwrap();
        assert_eq!(budget.chargeable_pin_capacity(), 4);
        assert_eq!(budget.pushable_pin_count(10), 3);

        let named = budget.prepare_event_charge(PIN_RESULT_CHARGE_EVENT, 3);
        let dataset = budget.prepare_event_charge(DEFAULT_DATASET_ITEM_EVENT, 3);
        assert_eq!((named, dataset), (3, 3));
        assert_eq!(budget.total_charged_amount(), 0.01);
        assert!(budget.event_charge_limit_reached(PIN_RESULT_CHARGE_EVENT));
        assert_eq!(
            pinterest_charged_save_result(named + dataset, true, 3).saved_count,
            3
        );
    }

    #[test]
    fn treats_unpriced_pin_events_as_free_but_keeps_dataset_item_budgeting() {
        let mut run = ppe_run(0.01, json!({}));
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove(PIN_RESULT_CHARGE_EVENT);
        let budget = ChargeBudget::from_actor_run(&run).unwrap();
        assert_eq!(budget.chargeable_pin_capacity(), usize::MAX);
        assert_eq!(budget.pushable_pin_count(250), 10);

        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove(DEFAULT_DATASET_ITEM_EVENT);
        let unpriced_budget = ChargeBudget::from_actor_run(&run).unwrap();
        assert_eq!(unpriced_budget.pushable_pin_count(250), 250);

        let free_budget = ChargeBudget::from_actor_run(&json!({"data": {
            "pricingInfo": {"pricingModel": "FREE"}
        }}))
        .unwrap();
        assert!(!free_budget.is_pay_per_event());
        assert_eq!(free_budget.pushable_pin_count(250), 250);
    }

    #[test]
    fn formats_scrappa_errors_and_selects_only_transient_retry_statuses() {
        assert_eq!(
            scrappa_api_error_message(
                StatusCode::BAD_REQUEST,
                r#"{"message":"Invalid input","errors":{"query":["is required","is too short"]}}"#
            ),
            "Invalid input - query: is required, is too short"
        );
        assert_eq!(
            scrappa_api_error_message(StatusCode::BAD_REQUEST, " bad   request\nbody "),
            "bad request body"
        );
        for status in [408, 429, 500, 502, 503, 504] {
            let error = ScrappaApiError {
                status,
                message: "retry".to_owned(),
            };
            assert!(is_retryable_scrappa_error(&anyhow::Error::new(error)));
        }
        assert!(!is_retryable_scrappa_error(&anyhow::Error::new(
            ScrappaApiError {
                status: 400,
                message: "bad input".to_owned(),
            }
        )));
        assert_eq!(scrappa_retry_delay(1).as_millis() / 1000, 2);
        assert!(scrappa_retry_delay(2).as_millis() >= 4000);
    }

    #[test]
    fn preserves_apify_client_retry_statuses_deadline_and_backoff() {
        assert_eq!(APIFY_MAX_RETRIES, 8);
        assert_eq!(APIFY_REQUEST_TIMEOUT.as_secs(), 360);
        assert_eq!(apify_retry_delay(0), Duration::from_millis(500));
        assert_eq!(apify_retry_delay(1), Duration::from_secs(1));
        assert!(retryable_apify_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(retryable_apify_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(!retryable_apify_status(StatusCode::REQUEST_TIMEOUT));
        assert!(!retryable_apify_status(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn chunks_dataset_items_without_exceeding_the_request_limit() {
        let item = json!({"payload": "x".repeat(1024 * 1024)});
        let chunks = dataset_item_chunks(&vec![item.clone(); 5]).unwrap();
        assert_eq!(chunks.len(), 2);
        assert!(chunks.iter().all(|chunk| {
            serde_json::to_vec(chunk).unwrap().len() <= MAX_DATASET_REQUEST_BYTES
        }));
    }

    #[test]
    fn actor_manifest_keeps_input_prefill_and_run_resources() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let actor: Value = serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();
        assert_eq!(
            schema["properties"]["queries"]["prefill"],
            json!(["home decor", "kitchen ideas"])
        );
        assert_eq!(schema["properties"]["query"]["prefill"], "home decor");
        assert_eq!(schema["properties"]["limit"]["maximum"], 250);
        assert_eq!(actor["defaultMemoryMbytes"], 256);
        assert_eq!(actor["defaultRunOptions"]["timeoutSecs"], 360);
        assert!(actor.get("meta").is_none());
    }

    #[tokio::test]
    async fn scrappa_request_keeps_auth_headers_and_pagination_query() {
        let (base_url, task) = mock_http_response(200, r#"{"pins":[]}"#).await;
        let base_url = endpoint_url(&base_url, &["api"]).unwrap();
        let client = ScrappaClient::new(base_url, "scrappa-secret".to_owned()).unwrap();
        let params = PinterestSearchParams {
            query: "home decor".to_owned(),
            limit: 3,
            bookmark: Some("next/page".to_owned()),
        };
        assert_eq!(
            client.pinterest_search(&params).await.unwrap(),
            json!({"pins": []})
        );
        let request = task.await.unwrap();
        let request = request.to_ascii_lowercase();
        assert!(request.starts_with(
            "get /api/pinterest/search?query=home+decor&limit=3&bookmark=next%2fpage http/1.1"
        ));
        assert!(request.contains("x-api-key: scrappa-secret"));
        assert!(request.contains("user-agent: thescrappa-pinterest-search-scraper/1.0"));
        assert!(request.contains("accept: application/json"));
    }

    #[tokio::test]
    async fn apify_input_and_output_use_run_storage_and_bearer_auth() {
        let (base_url, task) = mock_http_response(200, r#"{"query":"home decor"}"#).await;
        let apify = apify_client(base_url);
        let input = apify.get_input().await.unwrap().unwrap();
        assert_eq!(input["query"], "home decor");
        let request = task.await.unwrap().to_ascii_lowercase();
        assert!(request.starts_with("get /v2/key-value-stores/store-1/records/input http/1.1"));
        assert!(request.contains("authorization: bearer test-token"));

        let (base_url, task) = mock_http_response(201, "{}").await;
        let apify = apify_client(base_url);
        apify.put_output(&json!({"pins_saved": 1})).await.unwrap();
        let request = task.await.unwrap().to_ascii_lowercase();
        assert!(request.starts_with("put /v2/key-value-stores/store-1/records/output http/1.1"));
        assert!(request.contains("\"pins_saved\":1"));
        assert!(request.contains("authorization: bearer test-token"));
    }

    #[tokio::test]
    async fn custom_result_charge_uses_apify_run_charge_endpoint() {
        let (base_url, task) = mock_http_response(201, "{}").await;
        let apify = apify_client(base_url);
        apify
            .charge_event(PIN_RESULT_CHARGE_EVENT, 2)
            .await
            .unwrap();
        let request = task.await.unwrap().to_ascii_lowercase();
        assert!(request.starts_with("post /v2/actor-runs/run-1/charge http/1.1"));
        assert!(request.contains("\"eventname\":\"pin-result\""));
        assert!(request.contains("\"count\":2"));
        assert!(request.contains("idempotency-key: pinterest-search-run-1-"));
    }

    #[tokio::test]
    async fn terminal_status_updates_keep_apify_level_and_terminal_flag() {
        let (base_url, task) = mock_http_response(200, "{}").await;
        let apify = apify_client(base_url);
        apify
            .set_status_message("Actor failed", "ERROR")
            .await
            .unwrap();
        let request = task.await.unwrap().to_ascii_lowercase();
        assert!(request.starts_with("put /v2/actor-runs/run-1 http/1.1"));
        assert!(request.contains("\"statusmessage\":\"actor failed\""));
        assert!(request.contains("\"isstatusmessageterminal\":true"));
        assert!(request.contains("\"level\":\"error\""));
        assert!(!request.contains("\"runid\""));
    }
}
