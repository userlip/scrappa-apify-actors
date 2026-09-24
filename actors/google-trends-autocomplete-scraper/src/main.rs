use std::{env, error::Error, fmt, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_MAX_ATTEMPTS: u32 = 3;
const SCRAPPA_USER_AGENT: &str = "thescrappa-google-trends-autocomplete-scraper/1.0";
const SUGGESTION_RESULT_CHARGE_EVENT: &str = "suggestion-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

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
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
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

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env_or_default(name, default);
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

#[derive(Clone, Debug)]
struct PricingState {
    is_pay_per_event: bool,
    event_prices: Map<String, Value>,
    charged_event_counts: Map<String, Value>,
    max_total_charge_usd: f64,
}

impl PricingState {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let is_pay_per_event = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self {
                is_pay_per_event: false,
                event_prices: Map::new(),
                charged_event_counts: Map::new(),
                max_total_charge_usd: f64::INFINITY,
            });
        }

        let event_prices = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?
            .iter()
            .map(|(name, event)| {
                let price = event
                    .get("eventPriceUsd")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| {
                        anyhow!("Apify run did not provide the price for event {name}")
                    })?;
                if !price.is_finite() || price < 0.0 {
                    bail!("Apify run returned an invalid price for event {name}");
                }
                Ok((name.clone(), json!(price)))
            })
            .collect::<Result<Map<String, Value>>>()?;
        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .cloned()
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let max_total_charge_usd = match data.pointer("/options/maxTotalChargeUsd") {
            None | Some(Value::Null) => f64::INFINITY,
            Some(value) => {
                let max_charge = value
                    .as_f64()
                    .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
                if !max_charge.is_finite() || max_charge < 0.0 {
                    bail!("Apify run returned an invalid spending limit");
                }
                if max_charge == 0.0 {
                    f64::INFINITY
                } else {
                    max_charge
                }
            }
        };

        Ok(Self {
            is_pay_per_event,
            event_prices,
            charged_event_counts,
            max_total_charge_usd,
        })
    }

    fn affordable_suggestion_count(&self, requested: usize) -> Result<usize> {
        if requested == 0 || !self.is_pay_per_event {
            return Ok(requested);
        }

        let suggestion_price = event_price(&self.event_prices, SUGGESTION_RESULT_CHARGE_EVENT)?;
        let dataset_item_price = event_price(&self.event_prices, DEFAULT_DATASET_ITEM_EVENT)?;
        let price_per_row = suggestion_price + dataset_item_price;
        if !price_per_row.is_finite() {
            bail!("Apify run returned invalid total event prices");
        }
        if price_per_row == 0.0 || self.max_total_charge_usd.is_infinite() {
            return Ok(requested);
        }

        let total_spent =
            self.charged_event_counts
                .iter()
                .try_fold(0.0, |spent, (name, count)| {
                    let count = count
                        .as_u64()
                        .ok_or_else(|| anyhow!("Invalid charged event count for {name}"))?;
                    let event_cost = event_price(&self.event_prices, name)? * count as f64;
                    let next_spent = spent + event_cost;
                    if !next_spent.is_finite() {
                        bail!("Apify run returned invalid charged totals");
                    }
                    Ok(next_spent)
                })?;
        let remaining = self.max_total_charge_usd - total_spent;
        if remaining <= 0.0 {
            return Ok(0);
        }

        // Match the SDK's four-decimal rounding before flooring to avoid losing an
        // affordable row to floating-point noise (for example, 4.999999999999999).
        let raw_limit = remaining / price_per_row;
        let rounded_limit = (raw_limit * 10_000.0).round() / 10_000.0;
        Ok(requested.min(rounded_limit.floor() as usize))
    }
}

fn event_price(prices: &Map<String, Value>, event_name: &str) -> Result<f64> {
    prices
        .get(event_name)
        .and_then(Value::as_f64)
        .map(Ok)
        .unwrap_or_else(|| {
            if event_name == SUGGESTION_RESULT_CHARGE_EVENT {
                bail!("Apify run did not configure the {event_name} charge event")
            }
            Ok(0.0)
        })
}

struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

impl ApifyClient<'_> {
    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        let mut path = vec!["v2"];
        path.extend_from_slice(segments);
        endpoint_url(&self.config.apify_api_base, &path)
    }

    async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
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
        let input = response_json(response, "Apify INPUT request").await?;
        Ok((!input.is_null()).then_some(input))
    }

    async fn get_pricing(&self) -> Result<PricingState> {
        let url = self.endpoint(&["actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        PricingState::from_run(&response_json(response, "Apify run pricing request").await?)
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["datasets", &self.config.dataset_id, "items"])?;
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

    async fn charge_suggestions(&self, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let url = self.endpoint(&["actor-runs", &self.config.actor_run_id, "charge"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .header(
                "idempotency-key",
                format!(
                    "google-trends-autocomplete-{}-suggestions",
                    self.config.actor_run_id
                ),
            )
            .json(&json!({
                "eventName": SUGGESTION_RESULT_CHARGE_EVENT,
                "count": count,
            }))
            .send()
            .await
            .context("Apify suggestion result charge request failed")?;
        ensure_success(response, "Apify suggestion result charge request").await
    }

    async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
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

    async fn set_terminal_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .timeout(Duration::from_secs(1))
            .json(&json!({
                "runId": self.config.actor_run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true,
            }))
            .send()
            .await
            .context("Apify status message request failed")?;
        ensure_success(response, "Apify status message request").await
    }
}

#[derive(Clone, Debug)]
struct GoogleTrendsAutocompleteParams {
    query: String,
    geo: String,
    hl: String,
}

impl GoogleTrendsAutocompleteParams {
    fn as_map(&self) -> Map<String, Value> {
        Map::from_iter([
            ("q".to_owned(), Value::String(self.query.clone())),
            ("geo".to_owned(), Value::String(self.geo.clone())),
            ("hl".to_owned(), Value::String(self.hl.clone())),
        ])
    }

    fn describe(&self) -> String {
        format!("\"{}\" (geo={}, hl={})", self.query, self.geo, self.hl)
    }
}

fn clean_optional_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null() && value.as_str() != Some("")) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
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
    clean_optional_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn build_autocomplete_params(input: &Value) -> Result<GoogleTrendsAutocompleteParams> {
    let fields = input.as_object();
    let query = clean_optional_string(fields.and_then(|fields| fields.get("query")), "query", 100)?
        .map(Ok)
        .unwrap_or_else(|| {
            clean_required_string(fields.and_then(|fields| fields.get("q")), "query", 100)
        })?;
    let geo = clean_optional_string(fields.and_then(|fields| fields.get("geo")), "geo", 10)?
        .map(|geo| {
            if geo.eq_ignore_ascii_case("worldwide") {
                "Worldwide".to_owned()
            } else {
                geo.to_uppercase()
            }
        })
        .unwrap_or_else(|| "US".to_owned());
    let hl = clean_optional_string(fields.and_then(|fields| fields.get("hl")), "hl", 2)?
        .map(|hl| {
            if !hl.bytes().all(|byte| byte.is_ascii_alphabetic()) {
                bail!("hl must be a two-letter language code");
            }
            Ok(hl.to_ascii_lowercase())
        })
        .transpose()?
        .unwrap_or_else(|| "en".to_owned());

    Ok(GoogleTrendsAutocompleteParams { query, geo, hl })
}

#[derive(Debug)]
enum ScrappaRequestError {
    Timeout,
    Api { status: u16, message: String },
    Endpoint(String),
    Request(reqwest::Error),
    InvalidJson(serde_json::Error),
}

impl fmt::Display for ScrappaRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(
                formatter,
                "Scrappa API request timed out after {}ms",
                SCRAPPA_REQUEST_TIMEOUT.as_millis()
            ),
            Self::Api { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::Endpoint(message) => write!(formatter, "{message}"),
            Self::Request(error) => write!(formatter, "{error}"),
            Self::InvalidJson(error) => {
                write!(
                    formatter,
                    "Scrappa API response was not valid JSON: {error}"
                )
            }
        }
    }
}

impl Error for ScrappaRequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Request(error) => Some(error),
            Self::InvalidJson(error) => Some(error),
            Self::Timeout | Self::Api { .. } | Self::Endpoint(_) => None,
        }
    }
}

impl ScrappaRequestError {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout => true,
            Self::Api { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::Endpoint(_) => false,
            Self::Request(error) => error.is_timeout() || error.is_connect() || error.is_body(),
            Self::InvalidJson(_) => false,
        }
    }
}

struct ScrappaClient<'a> {
    http: &'a Client,
    base_url: &'a Url,
    api_key: &'a str,
}

impl ScrappaClient<'_> {
    async fn get_autocomplete(
        &self,
        params: &GoogleTrendsAutocompleteParams,
    ) -> std::result::Result<Value, ScrappaRequestError> {
        let mut last_error = None;
        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send_autocomplete(params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let should_retry = attempt < SCRAPPA_MAX_ATTEMPTS && error.is_retryable();
                    if !should_retry {
                        return Err(error);
                    }
                    let delay_ms = get_retry_delay_ms(attempt, retry_jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        SCRAPPA_MAX_ATTEMPTS,
                        delay_ms
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    last_error = Some(error);
                }
            }
        }
        Err(last_error.expect("at least one request attempt is configured"))
    }

    async fn send_autocomplete(
        &self,
        params: &GoogleTrendsAutocompleteParams,
    ) -> std::result::Result<Value, ScrappaRequestError> {
        let mut url = endpoint_url(self.base_url, &["google-trends", "autocomplete"])
            .map_err(|error| ScrappaRequestError::Endpoint(error.to_string()))?;
        for (key, value) in params.as_map() {
            url.query_pairs_mut()
                .append_pair(&key, value.as_str().unwrap_or_default());
        }

        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, SCRAPPA_USER_AGENT)
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(scrappa_transport_error)?;
        let status = response.status();
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) if error.is_timeout() => return Err(ScrappaRequestError::Timeout),
            Err(_error) if !status.is_success() => {
                let fallback = status.canonical_reason().unwrap_or("HTTP error");
                return Err(ScrappaRequestError::Api {
                    status: status.as_u16(),
                    message: fallback.to_owned(),
                });
            }
            Err(error) => return Err(ScrappaRequestError::Request(error)),
        };

        if !status.is_success() {
            return Err(ScrappaRequestError::Api {
                status: status.as_u16(),
                message: read_scrappa_error_message(&body, status),
            });
        }
        serde_json::from_str(&body).map_err(ScrappaRequestError::InvalidJson)
    }
}

fn scrappa_transport_error(error: reqwest::Error) -> ScrappaRequestError {
    if error.is_timeout() {
        ScrappaRequestError::Timeout
    } else {
        ScrappaRequestError::Request(error)
    }
}

fn retry_jitter_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as u64 % 1000)
        .unwrap_or(0)
}

fn get_retry_delay_ms(failed_attempt: u32, jitter_ms: u64) -> u64 {
    let backoff = 1000u64
        .saturating_mul(2u64.saturating_pow(failed_attempt))
        .saturating_add(jitter_ms);
    backoff.min(10_000)
}

fn read_scrappa_error_message(body: &str, status: StatusCode) -> String {
    let fallback = status.canonical_reason().unwrap_or("HTTP error");
    if body.is_empty() {
        return fallback.to_owned();
    }
    if let Ok(data) = serde_json::from_str::<Value>(body) {
        let Some(object) = data.as_object() else {
            return fallback.to_owned();
        };
        let mut message = object
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or_else(|| fallback.to_owned());
        if let Some(errors) = object.get("errors").and_then(Value::as_object) {
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
                        .unwrap_or_else(|| js_string(messages));
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
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn as_suggestion_entries(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(value) => Some(json!({ "suggestion": value })),
                Value::Object(_) => Some(item.clone()),
                _ => None,
            })
            .collect(),
        Value::Object(object) => object
            .get("suggestions")
            .filter(|value| !value.is_null())
            .or_else(|| object.get("results").filter(|value| !value.is_null()))
            .or_else(|| object.get("data").filter(|value| !value.is_null()))
            .map(as_suggestion_entries)
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn first_string(values: impl IntoIterator<Item = Value>) -> Option<Value> {
    values.into_iter().find(|value| {
        value
            .as_str()
            .is_some_and(|string| !string.trim().is_empty())
    })
}

fn build_autocomplete_dataset_items(
    response: &Value,
    params: &GoogleTrendsAutocompleteParams,
) -> Vec<Value> {
    let suggestions = response
        .get("suggestions")
        .filter(|value| !value.is_null())
        .or_else(|| {
            response
                .get("autocomplete")
                .filter(|value| !value.is_null())
        })
        .or_else(|| response.get("results").filter(|value| !value.is_null()))
        .or_else(|| response.get("data").filter(|value| !value.is_null()));
    let Some(suggestions) = suggestions else {
        return Vec::new();
    };
    let params = params.as_map();
    let response_time_ms = response
        .get("response_time_ms")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);
    let search_parameters = response
        .get("search_parameters")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);

    as_suggestion_entries(suggestions)
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let suggestion = first_string(
                ["suggestion", "query", "keyword", "title", "value", "name"]
                    .into_iter()
                    .filter_map(|field| entry.get(field).cloned()),
            )
            .unwrap_or(Value::Null);
            let result_type = first_string(entry.get("type").cloned()).unwrap_or(Value::Null);
            let mut item = entry.as_object().cloned().unwrap_or_default();
            item.insert("position".to_owned(), json!(index + 1));
            item.insert("suggestion".to_owned(), suggestion);
            item.insert("type".to_owned(), result_type);
            item.insert(
                "source_keyword".to_owned(),
                params.get("q").cloned().unwrap_or(Value::Null),
            );
            item.insert(
                "request_geo".to_owned(),
                params.get("geo").cloned().unwrap_or(Value::Null),
            );
            item.insert(
                "request_hl".to_owned(),
                params.get("hl").cloned().unwrap_or(Value::Null),
            );
            item.insert("response_time_ms".to_owned(), response_time_ms.clone());
            item.insert("search_parameters".to_owned(), search_parameters.clone());
            Value::Object(item)
        })
        .collect()
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error
        .downcast_ref::<ScrappaRequestError>()
        .is_some_and(|error| matches!(error, ScrappaRequestError::Timeout))
    {
        format!(
            "{}. The Google Trends autocomplete request exceeded the {}s Scrappa API timeout. Try a more specific keyword or run the request again.",
            error,
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        error.to_string()
    }
}

async fn run_actor(http: &Client, config: &Config) -> Result<()> {
    let apify = ApifyClient { http, config };
    let pricing = apify.get_pricing().await?;
    let input = apify
        .get_input()
        .await?
        .ok_or_else(|| anyhow!("Input is required"))?;
    let params = build_autocomplete_params(&input)?;
    println!(
        "Fetching Google Trends autocomplete suggestions for {}",
        params.describe()
    );

    let scrappa = ScrappaClient {
        http,
        base_url: &config.scrappa_api_base,
        api_key: &config.scrappa_api_key,
    };
    let response = scrappa.get_autocomplete(&params).await?;
    let dataset_items = build_autocomplete_dataset_items(&response, &params);
    let mut saved_suggestion_count = 0;
    let mut charge_limit_reached = false;

    if !dataset_items.is_empty() {
        if pricing.is_pay_per_event {
            let allowed_count = pricing.affordable_suggestion_count(dataset_items.len())?;
            let items_to_save = &dataset_items[..allowed_count];
            apify.push_dataset_items(items_to_save).await?;
            apify.charge_suggestions(allowed_count).await?;
            saved_suggestion_count = allowed_count;
            charge_limit_reached = allowed_count < dataset_items.len();

            if charge_limit_reached {
                let status_message = "Charge limit reached before saving all Google Trends autocomplete suggestion results.";
                println!(
                    "{status_message} {}",
                    json!({
                        "event": SUGGESTION_RESULT_CHARGE_EVENT,
                        "charged_count": saved_suggestion_count,
                        "requested_count": dataset_items.len(),
                    })
                );
            }
        } else {
            apify.push_dataset_items(&dataset_items).await?;
            saved_suggestion_count = dataset_items.len();
        }
    } else {
        println!("No Google Trends autocomplete suggestions found for this request");
    }

    let output = json!({
        "search_parameters": response.get("search_parameters").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "suggestion_count": dataset_items.len(),
        "saved_suggestion_count": saved_suggestion_count,
        "charge_limit_reached": charge_limit_reached,
        "raw_response_omitted": charge_limit_reached,
        "response_time_ms": response.get("response_time_ms").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "raw_response": if charge_limit_reached { Value::Null } else { response.clone() },
    });
    apify.put_output(&output).await?;

    if charge_limit_reached {
        let status_message =
            "Charge limit reached before saving all Google Trends autocomplete suggestion results.";
        if let Err(error) = apify.set_terminal_status_message(status_message).await {
            eprintln!("Could not set terminal Actor status message: {error}");
        }
        return Ok(());
    }

    println!("Google Trends autocomplete scraping completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "suggestion_results": dataset_items.len(),
            "response_time_ms": response.get("response_time_ms").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        })
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {}", actor_error_message(&error));
            std::process::exit(1);
        }
    };
    let http = Client::new();
    if let Err(error) = run_actor(&http, &config).await {
        let message = actor_error_message(&error);
        eprintln!("Actor failed: {message}");
        let apify = ApifyClient {
            http: &http,
            config: &config,
        };
        if let Err(status_error) = apify.set_terminal_status_message(&message).await {
            eprintln!("Could not set terminal Actor status message: {status_error}");
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests;
