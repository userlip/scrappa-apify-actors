use std::{
    collections::HashMap,
    env,
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_ENDPOINT: &str = "/google-trends/interest";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const TIMELINE_POINT_CHARGE_EVENT: &str = "timeline-point";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const SCRAPPA_USER_AGENT: &str = "thescrappa-google-trends-interest-scraper/1.0";

struct Config {
    apify_api_base: Url,
    scrappa_api_base: String,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base: env::var("SCRAPPA_API_BASE_URL")
                .unwrap_or_else(|_| SCRAPPA_API_DEFAULT.to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key: env::var("SCRAPPA_API_KEY").unwrap_or_default(),
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
        .extend(segments.iter().copied());
    Ok(url)
}

#[derive(Debug, PartialEq, Eq)]
struct InterestParams {
    q: String,
    geo: Option<String>,
    time_range: Option<String>,
    hl: Option<String>,
    search_type: Option<String>,
}

impl InterestParams {
    fn query_pairs(&self) -> Vec<(&'static str, &str)> {
        let mut pairs = vec![("q", self.q.as_str())];
        if let Some(value) = self.geo.as_deref() {
            pairs.push(("geo", value));
        }
        if let Some(value) = self.time_range.as_deref() {
            pairs.push(("time_range", value));
        }
        if let Some(value) = self.hl.as_deref() {
            pairs.push(("hl", value));
        }
        if let Some(value) = self.search_type.as_deref() {
            pairs.push(("search_type", value));
        }
        pairs
    }

    fn value(&self, field: &str) -> Option<&str> {
        match field {
            "q" => Some(self.q.as_str()),
            "geo" => self.geo.as_deref(),
            "time_range" => self.time_range.as_deref(),
            "hl" => self.hl.as_deref(),
            "search_type" => self.search_type.as_deref(),
            _ => None,
        }
    }

    fn describe(&self) -> String {
        let filters = ["geo", "time_range", "hl", "search_type"]
            .into_iter()
            .filter_map(|field| self.value(field).map(|value| format!("{field}={value}")))
            .collect::<Vec<_>>();
        let suffix = if filters.is_empty() {
            String::new()
        } else {
            format!(" ({})", filters.join(", "))
        };
        format!("\"{}\"{suffix}", self.q)
    }
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
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

fn clean_geo(value: Option<&Value>) -> Result<Option<String>> {
    let Some(geo) = clean_string(value, "geo", 10)? else {
        return Ok(None);
    };
    if geo.eq_ignore_ascii_case("worldwide") {
        return Ok(Some("Worldwide".to_owned()));
    }
    Ok(Some(geo.to_uppercase()))
}

fn clean_language(value: Option<&Value>) -> Result<Option<String>> {
    let Some(language) = clean_string(value, "hl", 2)? else {
        return Ok(None);
    };
    if language.len() != 2 || !language.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        bail!("hl must be a two-letter language code");
    }
    Ok(Some(language.to_lowercase()))
}

fn clean_enum(value: Option<&Value>, field: &str, values: &[&str]) -> Result<Option<String>> {
    let Some(value) = clean_string(value, field, 20)? else {
        return Ok(None);
    };
    let normalized = value.to_lowercase();
    if !values.contains(&normalized.as_str()) {
        bail!("{field} must be one of: {}", values.join(", "));
    }
    Ok(Some(normalized))
}

fn build_interest_params(input: &Value) -> Result<InterestParams> {
    Ok(InterestParams {
        q: clean_required_string(input.get("q"), "q", 100)?,
        geo: clean_geo(input.get("geo"))?,
        time_range: clean_enum(
            input.get("time_range"),
            "time_range",
            &["1h", "4h", "1d", "7d", "30d", "90d", "1y", "5y", "all"],
        )?,
        hl: clean_language(input.get("hl"))?,
        search_type: clean_enum(
            input.get("search_type"),
            "search_type",
            &["web", "images", "news", "youtube", "shopping"],
        )?,
    })
}

fn timeline_points(value: Option<&Value>) -> Vec<&Map<String, Value>> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .collect()
}

fn get_timeline_points(response: &Value) -> Vec<&Map<String, Value>> {
    let primary = timeline_points(response.get("timeline_data"));
    if !primary.is_empty() {
        return primary;
    }
    timeline_points(
        response
            .get("interest_over_time")
            .and_then(|value| value.get("data_points")),
    )
}

fn build_timeline_dataset_items(response: &Value, params: &InterestParams) -> Vec<Value> {
    let points = get_timeline_points(response);
    let interest = response
        .get("interest_over_time")
        .and_then(Value::as_object);
    let search_parameters = response
        .get("search_parameters")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);

    points
        .into_iter()
        .enumerate()
        .map(|(index, point)| {
            let mut item = point.clone();
            item.insert("position".to_owned(), json!(index + 1));
            item.insert(
                "timestamp".to_owned(),
                point.get("timestamp").cloned().unwrap_or(Value::Null),
            );
            item.insert(
                "date".to_owned(),
                point.get("date").cloned().unwrap_or(Value::Null),
            );
            item.insert(
                "value".to_owned(),
                point.get("value").cloned().unwrap_or(Value::Null),
            );
            for field in ["average", "max_value", "min_value"] {
                item.insert(
                    field.to_owned(),
                    interest
                        .and_then(|summary| summary.get(field))
                        .cloned()
                        .unwrap_or(Value::Null),
                );
            }
            for (field, param) in [
                ("request_q", "q"),
                ("request_geo", "geo"),
                ("request_time_range", "time_range"),
                ("request_hl", "hl"),
                ("request_search_type", "search_type"),
            ] {
                item.insert(
                    field.to_owned(),
                    params
                        .value(param)
                        .map_or(Value::Null, |value| json!(value)),
                );
            }
            item.insert(
                "response_time_ms".to_owned(),
                response
                    .get("response_time_ms")
                    .cloned()
                    .filter(|value| !value.is_null())
                    .unwrap_or(Value::Null),
            );
            item.insert("search_parameters".to_owned(), search_parameters.clone());
            Value::Object(item)
        })
        .collect()
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("Failed to read {operation} response"))?;
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

async fn get_input(client: &Client, config: &Config) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base,
        &[
            "v2",
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .context("Apify INPUT request failed")?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(Value::Null);
    }
    response_json(response, "Apify INPUT request").await
}

async fn get_actor_run(client: &Client, config: &Config) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .context("Apify actor run pricing request failed")?;
    response_json(response, "Apify actor run pricing request").await
}

#[derive(Debug)]
struct ScrappaTimeoutError;

impl fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            SCRAPPA_REQUEST_TIMEOUT.as_millis()
        )
    }
}

impl Error for ScrappaTimeoutError {}

#[derive(Debug)]
struct ScrappaHttpError {
    status: StatusCode,
    message: String,
}

impl fmt::Display for ScrappaHttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status.as_u16(),
            self.message
        )
    }
}

impl Error for ScrappaHttpError {}

fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let backoff = 1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt as u32));
    backoff.saturating_add(jitter_ms).min(10_000)
}

fn retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    error
        .downcast_ref::<ScrappaHttpError>()
        .is_some_and(|error| matches!(error.status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504))
}

fn scrappa_timeout_or_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow::Error::new(ScrappaTimeoutError)
    } else {
        anyhow!("{error}")
    }
}

fn scrappa_error_message(body: &str, fallback: &str) -> String {
    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_owned();
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let joined = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .map(|message| match message {
                                    Value::String(message) => message.clone(),
                                    other => other.to_string(),
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {joined}")
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
    if body.is_empty() {
        return fallback.to_owned();
    }
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

async fn send_scrappa_request(
    client: &Client,
    config: &Config,
    params: &InterestParams,
) -> Result<Value> {
    let mut url = Url::parse(&format!("{}{SCRAPPA_ENDPOINT}", config.scrappa_api_base))
        .context("SCRAPPA_API_BASE_URL must be a valid absolute URL")?;
    url.query_pairs_mut().extend_pairs(params.query_pairs());

    let response = client
        .get(url)
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .header(header::ACCEPT, "application/json")
        .header("X-API-Key", &config.scrappa_api_key)
        .header(header::USER_AGENT, SCRAPPA_USER_AGENT)
        .send()
        .await
        .map_err(scrappa_timeout_or_request_error)?;

    if !response.status().is_success() {
        let status = response.status();
        let fallback = status.canonical_reason().unwrap_or("Unknown status");
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) if error.is_timeout() => {
                return Err(anyhow::Error::new(ScrappaTimeoutError))
            }
            Err(_) => {
                return Err(ScrappaHttpError {
                    status,
                    message: fallback.to_owned(),
                }
                .into())
            }
        };
        return Err(ScrappaHttpError {
            status,
            message: scrappa_error_message(&body, fallback),
        }
        .into());
    }

    let body = response
        .text()
        .await
        .map_err(scrappa_timeout_or_request_error)?;
    serde_json::from_str(&body).context("Scrappa API returned invalid JSON")
}

async fn fetch_interest(
    client: &Client,
    config: &Config,
    params: &InterestParams,
) -> Result<Value> {
    for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
        match send_scrappa_request(client, config, params).await {
            Ok(response) => return Ok(response),
            Err(error) if attempt < SCRAPPA_MAX_ATTEMPTS && retryable_scrappa_error(&error) => {
                let jitter = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_millis() as u64;
                let delay_ms = retry_delay_ms(attempt, jitter);
                eprintln!(
                    "Scrappa API request failed ({}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {delay_ms}ms.",
                    error,
                    attempt + 1
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the Scrappa retry loop always returns after the configured attempts")
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ChargeResult {
    event_charge_limit_reached: bool,
    charged_count: usize,
}

impl ChargeResult {
    fn merge(self, other: Self) -> Self {
        Self {
            event_charge_limit_reached: self.event_charge_limit_reached
                || other.event_charge_limit_reached,
            charged_count: self.charged_count.saturating_add(other.charged_count),
        }
    }
}

struct PpeBudget {
    prices: HashMap<String, f64>,
    charged_counts: HashMap<String, u64>,
    max_total_charge_usd: f64,
}

impl PpeBudget {
    fn from_actor_run(run_response: &Value) -> Result<Option<Self>> {
        let run = run_response.get("data").unwrap_or(run_response);
        let Some(pricing_info) = run.get("pricingInfo") else {
            return Ok(None);
        };
        if pricing_info.get("pricingModel").and_then(Value::as_str) != Some("PAY_PER_EVENT") {
            return Ok(None);
        }

        let events = pricing_info
            .pointer("/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .context("Actor run charge-event pricing is missing or invalid")?;
        let mut prices = HashMap::new();
        for (name, event) in events {
            let price = event
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if !price.is_finite() || price < 0.0 {
                bail!("Actor run charge-event price is invalid for {name}");
            }
            prices.insert(name.clone(), price);
        }

        let charged_counts = run
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .context("Actor run charged event counts are missing or invalid")?
            .iter()
            .map(|(name, count)| {
                count
                    .as_u64()
                    .map(|count| (name.clone(), count))
                    .with_context(|| format!("Actor run charged event count is invalid for {name}"))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        let configured_limit = run
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|limit| *limit != 0.0)
            .unwrap_or(f64::INFINITY);
        if configured_limit < 0.0 || configured_limit.is_nan() {
            bail!("Actor run maxTotalChargeUsd is invalid");
        }

        Ok(Some(Self {
            prices,
            charged_counts,
            max_total_charge_usd: configured_limit,
        }))
    }

    fn price(&self, event_name: &str) -> f64 {
        self.prices.get(event_name).copied().unwrap_or(0.0)
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_counts
            .iter()
            .map(|(name, count)| self.price(name) * *count as f64)
            .sum::<f64>();
        (total * 1_000_000.0).round() / 1_000_000.0
    }

    fn max_charges_by_price(&self, price: f64) -> usize {
        if price == 0.0 {
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
        let rounded = (unrounded * 10_000.0).round() / 10_000.0;
        if rounded <= 0.0 {
            return 0;
        }
        rounded.floor().min(usize::MAX as f64) as usize
    }

    fn max_event_charge_count(&self, event_name: &str) -> usize {
        let price = self.price(event_name);
        if price == 0.0 {
            usize::MAX
        } else {
            self.max_charges_by_price(price)
        }
    }

    fn dataset_item_limit(&self, requested: usize) -> usize {
        let price_per_item =
            self.price(TIMELINE_POINT_CHARGE_EVENT) + self.price(DEFAULT_DATASET_ITEM_EVENT);
        let affordable = if price_per_item > 0.0 {
            self.max_charges_by_price(price_per_item)
        } else {
            usize::MAX
        };
        if affordable >= requested {
            return requested;
        }
        if requested > 0
            && affordable == 0
            && self.total_charged_amount() <= self.max_total_charge_usd
        {
            return 1;
        }
        affordable.min(requested)
    }

    fn charge(&mut self, event_name: &str, requested: usize) -> ChargeResult {
        let max_count = self.max_event_charge_count(event_name);
        let charged_count = if requested <= max_count {
            requested
        } else if self.total_charged_amount() <= self.max_total_charge_usd {
            max_count.saturating_add(1)
        } else {
            0
        };
        if charged_count == 0 {
            return ChargeResult {
                event_charge_limit_reached: requested > 0,
                charged_count: 0,
            };
        }
        *self
            .charged_counts
            .entry(event_name.to_owned())
            .or_default() += charged_count as u64;
        let limit_reached = self.max_event_charge_count(event_name) == 0;
        ChargeResult {
            event_charge_limit_reached: limit_reached,
            charged_count,
        }
    }
}

fn ppe_items_result(
    budget: &mut PpeBudget,
    requested: usize,
) -> (usize, ChargeResult, ChargeResult) {
    let kept = budget.dataset_item_limit(requested);
    if kept == 0 {
        return (
            kept,
            ChargeResult {
                event_charge_limit_reached: requested > 0,
                charged_count: 0,
            },
            ChargeResult::default(),
        );
    }
    let custom_event = budget.charge(TIMELINE_POINT_CHARGE_EVENT, kept);
    let dataset_event = budget.charge(DEFAULT_DATASET_ITEM_EVENT, kept);
    (kept, custom_event, dataset_event)
}

async fn push_dataset_items(client: &Client, config: &Config, items: &[Value]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let url = endpoint_url(
        &config.apify_api_base,
        &["v2", "datasets", &config.dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .json(items)
        .send()
        .await
        .context("Apify dataset write failed")?;
    ensure_success(response, "Apify dataset write").await
}

async fn charge_timeline_points(client: &Client, config: &Config, count: usize) -> Result<()> {
    if count == 0 {
        return Ok(());
    }
    let url = endpoint_url(
        &config.apify_api_base,
        &["v2", "actor-runs", &config.actor_run_id, "charge"],
    )?;
    let idempotency_key = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .header(
            "Idempotency-Key",
            format!(
                "{}-{}-{idempotency_key}",
                config.actor_run_id, TIMELINE_POINT_CHARGE_EVENT
            ),
        )
        .json(&json!({
            "eventName": TIMELINE_POINT_CHARGE_EVENT,
            "count": count,
        }))
        .send()
        .await
        .context("Apify timeline-point charge request failed")?;
    ensure_success(response, "Apify timeline-point charge request").await
}

async fn put_output(client: &Client, config: &Config, output: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base,
        &[
            "v2",
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            "OUTPUT",
        ],
    )?;
    let response = client
        .put(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .json(output)
        .send()
        .await
        .context("Apify OUTPUT write failed")?;
    ensure_success(response, "Apify OUTPUT write").await
}

async fn put_terminal_status_message(
    client: &Client,
    config: &Config,
    status_message: &str,
) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .put(url)
        .timeout(Duration::from_secs(1))
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .json(&json!({
            "runId": config.actor_run_id,
            "statusMessage": status_message,
            "isStatusMessageTerminal": true,
        }))
        .send()
        .await
        .context("Apify status-message update failed")?;
    ensure_success(response, "Apify status-message update").await
}

async fn run_actor(client: &Client, config: &Config) -> Result<()> {
    let run = get_actor_run(client, config).await?;
    let ppe_budget = PpeBudget::from_actor_run(&run)?;
    if config.scrappa_api_key.is_empty() {
        bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
    }
    let input = get_input(client, config).await?;
    if input.is_null() {
        bail!("Input is required");
    }

    let params = build_interest_params(&input)?;
    println!(
        "Fetching Google Trends interest over time for {}",
        params.describe()
    );
    let response = fetch_interest(client, config, &params).await?;
    let dataset_items = build_timeline_dataset_items(&response, &params);

    if dataset_items.is_empty() {
        println!("No Google Trends timeline points found for this request");
    } else if let Some(mut budget) = ppe_budget {
        let (kept, custom_charge, dataset_charge) =
            ppe_items_result(&mut budget, dataset_items.len());
        push_dataset_items(client, config, &dataset_items[..kept]).await?;
        charge_timeline_points(client, config, custom_charge.charged_count).await?;
        let charge_result = custom_charge.merge(dataset_charge);
        if charge_result.event_charge_limit_reached
            && charge_result.charged_count < dataset_items.len()
        {
            let status_message =
                "Charge limit reached before saving all Google Trends timeline points.";
            println!(
                "{status_message} {}",
                json!({
                    "event": TIMELINE_POINT_CHARGE_EVENT,
                    "charged_count": charge_result.charged_count,
                    "requested_count": dataset_items.len(),
                })
            );
            if let Err(error) = put_terminal_status_message(client, config, status_message).await {
                eprintln!("Warning: {error}");
            }
            return Ok(());
        }
        put_output(client, config, &response).await?;
    } else {
        push_dataset_items(client, config, &dataset_items).await?;
        put_output(client, config, &response).await?;
    }

    if dataset_items.is_empty() {
        put_output(client, config, &response).await?;
    }
    println!("Google Trends interest scraping completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "timeline_points": dataset_items.len(),
            "average": response.pointer("/interest_over_time/average").filter(|value| !value.is_null()).unwrap_or(&Value::Null),
            "max_value": response.pointer("/interest_over_time/max_value").filter(|value| !value.is_null()).unwrap_or(&Value::Null),
            "min_value": response.pointer("/interest_over_time/min_value").filter(|value| !value.is_null()).unwrap_or(&Value::Null),
            "response_time_ms": response.get("response_time_ms").filter(|value| !value.is_null()).unwrap_or(&Value::Null),
        })
    );
    Ok(())
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{}. The Google Trends interest request exceeded the {}s Scrappa API timeout. Try a shorter time range, a more specific keyword, or run the request again.",
            error,
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        error.to_string()
    }
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
    let client = match Client::builder().build() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = run_actor(&client, &config).await {
        let message = actor_error_message(&error);
        eprintln!("Actor failed: {message}");
        if let Err(status_error) = put_terminal_status_message(&client, &config, &message).await {
            eprintln!("Warning: {status_error}");
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    const TIMELINE_RESPONSE: &str = r#"{"search_parameters":{"keyword":"tesla","geo":"US"},"timeline_data":[{"timestamp":1704067200,"date":"2024-01-01","value":42},{"timestamp":1704672000,"date":"2024-01-08","value":58}],"interest_over_time":{"average":50,"max_value":100,"min_value":12},"response_time_ms":587}"#;
    const THREE_POINT_RESPONSE: &str = r#"{"timeline_data":[{"value":1},{"value":2},{"value":3}]}"#;
    const NON_PPE_RUN: &str = r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_ACTOR"},"chargedEventCounts":{},"options":{}}}"#;

    fn test_config(server_url: Url, scrappa_api_key: &str) -> Config {
        Config {
            apify_api_base: server_url.clone(),
            scrappa_api_base: format!("{server_url}api"),
            apify_token: "test-apify-token".to_owned(),
            key_value_store_id: "store-id".to_owned(),
            dataset_id: "dataset-id".to_owned(),
            actor_run_id: "run-id".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: scrappa_api_key.to_owned(),
        }
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
        (Url::parse(&format!("http://{address}/")).unwrap(), server)
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut reader = BufReader::new(stream);
        let mut headers = String::new();
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        headers.push_str(&request_line);
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            headers.push_str(&line);
        }
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        format!("{headers}\r\n{}", String::from_utf8(body).unwrap())
    }

    fn request_body(request: &str) -> Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    fn ppe_run(event_price: f64, dataset_item_price: Option<f64>, budget: f64) -> Value {
        let mut events = json!({
            "timeline-point": { "eventPriceUsd": event_price }
        });
        if let Some(price) = dataset_item_price {
            events[DEFAULT_DATASET_ITEM_EVENT] = json!({ "eventPriceUsd": price });
        }
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": events }
                },
                "chargedEventCounts": {},
                "options": { "maxTotalChargeUsd": budget }
            }
        })
    }

    #[test]
    fn builds_normalized_interest_params_and_log_description() {
        let params = build_interest_params(&json!({
            "q": " tesla ",
            "geo": " us ",
            "time_range": "1Y",
            "hl": "EN",
            "search_type": "YouTube"
        }))
        .unwrap();
        assert_eq!(
            params,
            InterestParams {
                q: "tesla".to_owned(),
                geo: Some("US".to_owned()),
                time_range: Some("1y".to_owned()),
                hl: Some("en".to_owned()),
                search_type: Some("youtube".to_owned()),
            }
        );
        assert_eq!(
            params.describe(),
            "\"tesla\" (geo=US, time_range=1y, hl=en, search_type=youtube)"
        );
    }

    #[test]
    fn normalizes_worldwide_and_omits_empty_optional_values() {
        let params = build_interest_params(&json!({
            "q": " bitcoin ",
            "geo": " worldwide ",
            "hl": "   "
        }))
        .unwrap();
        assert_eq!(params.geo.as_deref(), Some("Worldwide"));
        assert_eq!(params.hl, None);
        assert_eq!(
            params.query_pairs(),
            vec![("q", "bitcoin"), ("geo", "Worldwide")]
        );
    }

    #[test]
    fn matches_input_validation_messages_and_limits() {
        assert_eq!(
            build_interest_params(&json!({})).unwrap_err().to_string(),
            "q is required"
        );
        assert_eq!(
            build_interest_params(&json!({ "q": "tesla", "time_range": "2y" }))
                .unwrap_err()
                .to_string(),
            "time_range must be one of: 1h, 4h, 1d, 7d, 30d, 90d, 1y, 5y, all"
        );
        assert_eq!(
            build_interest_params(&json!({ "q": "tesla", "hl": "eng" }))
                .unwrap_err()
                .to_string(),
            "hl must be 2 characters or fewer"
        );
        assert_eq!(
            build_interest_params(&json!({ "q": "tesla", "search_type": "podcasts" }))
                .unwrap_err()
                .to_string(),
            "search_type must be one of: web, images, news, youtube, shopping"
        );
        assert!(build_interest_params(&json!({ "q": "x".repeat(101) })).is_err());
        assert!(build_interest_params(&json!({ "q": "tesla", "geo": 1 })).is_err());
    }

    #[test]
    fn preserves_actor_schema_prefills_and_defaults() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["properties"]["q"]["prefill"], "tesla");
        assert_eq!(schema["properties"]["geo"]["default"], "US");
        assert_eq!(schema["properties"]["time_range"]["default"], "1y");
        assert_eq!(schema["properties"]["hl"]["default"], "en");
        assert_eq!(schema["properties"]["search_type"]["default"], "web");
    }

    #[test]
    fn builds_dataset_rows_from_primary_timeline_and_summary() {
        let params = build_interest_params(&json!({
            "q": "tesla", "geo": "US", "time_range": "1y", "hl": "en", "search_type": "web"
        }))
        .unwrap();
        let response: Value = serde_json::from_str(TIMELINE_RESPONSE).unwrap();
        let items = build_timeline_dataset_items(&response, &params);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["position"], 1);
        assert_eq!(items[0]["timestamp"], 1704067200_i64);
        assert_eq!(items[0]["date"], "2024-01-01");
        assert_eq!(items[0]["value"], 42);
        assert_eq!(items[0]["average"], 50);
        assert_eq!(items[0]["max_value"], 100);
        assert_eq!(items[0]["min_value"], 12);
        assert_eq!(items[0]["request_q"], "tesla");
        assert_eq!(items[0]["request_search_type"], "web");
        assert_eq!(items[0]["response_time_ms"], 587);
        assert_eq!(
            items[0]["search_parameters"],
            json!({ "keyword": "tesla", "geo": "US" })
        );
    }

    #[test]
    fn falls_back_to_interest_data_points_when_primary_has_no_objects() {
        let response = json!({
            "timeline_data": [null, ["not a point"]],
            "interest_over_time": {
                "data_points": [
                    { "timestamp": 1, "date": "fallback", "value": 7 },
                    null
                ]
            }
        });
        let params = build_interest_params(&json!({ "q": "tesla" })).unwrap();
        let items = build_timeline_dataset_items(&response, &params);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["date"], "fallback");
        assert_eq!(items[0]["position"], 1);
        assert_eq!(items[0]["request_geo"], Value::Null);
    }

    #[test]
    fn keeps_an_over_budget_item_to_match_the_sdk_charge_limit_behavior() {
        let mut budget = PpeBudget::from_actor_run(&ppe_run(0.01, None, 0.005))
            .unwrap()
            .unwrap();
        let (kept, custom, dataset) = ppe_items_result(&mut budget, 3);
        assert_eq!(kept, 1);
        assert_eq!(custom.charged_count, 1);
        assert!(custom.event_charge_limit_reached);
        assert_eq!(dataset.charged_count, 1);
        assert_eq!(custom.merge(dataset).charged_count, 2);
    }

    #[test]
    fn combined_ppe_prices_limit_rows_to_the_user_budget() {
        let run = ppe_run(0.0003, Some(0.0001), 0.001);
        let mut budget = PpeBudget::from_actor_run(&run).unwrap().unwrap();
        budget
            .charged_counts
            .insert("apify-actor-start".to_owned(), 1);
        // The start event has no configured price in this fixture and therefore matches the SDK's zero-priced fallback.
        assert_eq!(budget.dataset_item_limit(4), 2);
    }

    #[test]
    fn matches_retryable_scrappa_statuses_and_backoff_schedule() {
        for status in [408, 429, 500, 502, 503, 504] {
            let error = anyhow::Error::new(ScrappaHttpError {
                status: StatusCode::from_u16(status).unwrap(),
                message: "retry".to_owned(),
            });
            assert!(retryable_scrappa_error(&error));
        }
        let permanent = anyhow::Error::new(ScrappaHttpError {
            status: StatusCode::BAD_REQUEST,
            message: "bad input".to_owned(),
        });
        assert!(!retryable_scrappa_error(&permanent));
        assert!(retryable_scrappa_error(&anyhow::Error::new(
            ScrappaTimeoutError
        )));
        assert_eq!(retry_delay_ms(1, 250), 2250);
        assert_eq!(retry_delay_ms(2, 250), 4250);
        assert_eq!(retry_delay_ms(3, 250), 8250);
        assert_eq!(retry_delay_ms(4, 250), 10_000);
    }

    #[test]
    fn formats_timeout_message_with_the_actor_specific_guidance() {
        let message = actor_error_message(&anyhow::Error::new(ScrappaTimeoutError));
        assert_eq!(
            message,
            "Scrappa API request timed out after 60000ms. The Google Trends interest request exceeded the 60s Scrappa API timeout. Try a shorter time range, a more specific keyword, or run the request again."
        );
    }

    #[test]
    fn formats_scrappa_validation_errors_like_the_typescript_client() {
        assert_eq!(
            scrappa_error_message(
                r#"{"message":"The given data was invalid.","errors":{"q":["The q field is required.","Another message"]}}"#,
                "Unprocessable Entity"
            ),
            "The given data was invalid. - q: The q field is required., Another message"
        );
        assert_eq!(
            scrappa_error_message(" upstream   unavailable ", "Service Unavailable"),
            "upstream unavailable"
        );
    }

    #[tokio::test]
    async fn retries_retryable_scrappa_status_and_preserves_upstream_auth_and_query() {
        let (base_url, server) = mock_server(vec![
            (
                "503 Service Unavailable",
                r#"{"message":"upstream unavailable"}"#,
            ),
            ("200 OK", TIMELINE_RESPONSE),
        ]);
        let config = test_config(base_url.clone(), "scrappa-test-key");
        let params = build_interest_params(&json!({
            "q": "tesla model y", "geo": "US", "time_range": "1y", "hl": "en", "search_type": "web"
        }))
        .unwrap();
        let response = fetch_interest(&Client::new(), &config, &params)
            .await
            .unwrap();
        assert_eq!(response["timeline_data"].as_array().unwrap().len(), 2);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        for request in requests {
            let request_lower = request.to_ascii_lowercase();
            assert!(request_lower.starts_with("get /api/google-trends/interest?q=tesla+model+y&geo=us&time_range=1y&hl=en&search_type=web"));
            assert!(request_lower.contains("x-api-key: scrappa-test-key"));
            assert!(request_lower.contains(SCRAPPA_USER_AGENT));
        }
    }

    #[tokio::test]
    async fn writes_dataset_and_full_output_for_non_ppe_runs() {
        let input = r#"{"q":"tesla","geo":"US","time_range":"1y","hl":"en","search_type":"web"}"#;
        let (base_url, server) = mock_server(vec![
            ("200 OK", NON_PPE_RUN),
            ("200 OK", input),
            ("200 OK", TIMELINE_RESPONSE),
            ("201 Created", ""),
            ("201 Created", ""),
        ]);
        let config = test_config(base_url, "scrappa-test-key");
        run_actor(&Client::new(), &config).await.unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 5);
        assert!(requests[3]
            .to_ascii_lowercase()
            .starts_with("post /v2/datasets/dataset-id/items"));
        let rows = request_body(&requests[3]);
        assert_eq!(rows.as_array().unwrap().len(), 2);
        assert!(requests[4]
            .to_ascii_lowercase()
            .starts_with("put /v2/key-value-stores/store-id/records/output"));
        assert_eq!(
            request_body(&requests[4]),
            serde_json::from_str::<Value>(TIMELINE_RESPONSE).unwrap()
        );
        assert!(requests
            .iter()
            .all(|request| !request.to_ascii_lowercase().contains("/charge")));
    }

    #[tokio::test]
    async fn charges_ppe_results_with_the_named_event_and_stores_output() {
        let input = r#"{"q":"tesla"}"#;
        let run = serde_json::to_string(&ppe_run(0.0001, Some(0.0), 1.0)).unwrap();
        let (base_url, server) = mock_server(vec![
            ("200 OK", Box::leak(run.into_boxed_str())),
            ("200 OK", input),
            ("200 OK", TIMELINE_RESPONSE),
            ("201 Created", ""),
            ("201 Created", "{}"),
            ("201 Created", ""),
        ]);
        let config = test_config(base_url, "scrappa-test-key");
        run_actor(&Client::new(), &config).await.unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 6);
        let charge_request = &requests[4];
        assert!(charge_request
            .to_ascii_lowercase()
            .starts_with("post /v2/actor-runs/run-id/charge"));
        assert!(charge_request
            .to_ascii_lowercase()
            .contains("idempotency-key: run-id-timeline-point-"));
        assert_eq!(
            request_body(charge_request),
            json!({ "eventName": "timeline-point", "count": 2 })
        );
        assert!(requests[5]
            .to_ascii_lowercase()
            .starts_with("put /v2/key-value-stores/store-id/records/output"));
    }

    #[tokio::test]
    async fn stops_after_partial_ppe_write_with_terminal_status_message() {
        let input = r#"{"q":"tesla"}"#;
        let run = serde_json::to_string(&ppe_run(0.01, None, 0.005)).unwrap();
        let (base_url, server) = mock_server(vec![
            ("200 OK", Box::leak(run.into_boxed_str())),
            ("200 OK", input),
            ("200 OK", THREE_POINT_RESPONSE),
            ("201 Created", ""),
            ("201 Created", "{}"),
            ("200 OK", "{}"),
        ]);
        let config = test_config(base_url, "scrappa-test-key");
        run_actor(&Client::new(), &config).await.unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 6);
        assert_eq!(request_body(&requests[3]).as_array().unwrap().len(), 1);
        assert_eq!(request_body(&requests[4])["count"], 1);
        assert!(requests[5]
            .to_ascii_lowercase()
            .starts_with("put /v2/actor-runs/run-id "));
        let status = request_body(&requests[5]);
        assert_eq!(
            status["statusMessage"],
            "Charge limit reached before saving all Google Trends timeline points."
        );
        assert_eq!(status["isStatusMessageTerminal"], true);
        assert!(requests
            .iter()
            .all(|request| !request.to_ascii_lowercase().contains("records/output")));
    }
}
