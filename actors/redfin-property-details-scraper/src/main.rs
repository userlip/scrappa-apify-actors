use std::{
    collections::{HashMap, HashSet},
    env,
    process::ExitCode,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use rand::Rng;
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Map, Number, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const APIFY_MAX_RETRIES: usize = 2;
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const SCRAPPA_USER_AGENT: &str = "thescrappa-redfin-property-details-scraper/1.0";
const MAX_PROPERTIES_PER_RUN: usize = 100;
const REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT: &str = "property-result";
const DEFAULT_DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";
const TERMINAL_PROPERTY_ERROR: &str = "Failed to fetch property details after multiple attempts";
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq)]
struct RedfinPropertyDetailsRequest {
    params: RedfinPropertyDetailsParams,
    index: usize,
    input: Value,
    source: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RedfinPropertyDetailsParams {
    property_id: u64,
}

#[derive(Debug)]
struct ScrappaApiError {
    kind: ScrappaApiErrorKind,
    status: Option<u16>,
    details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrappaApiErrorKind {
    Timeout,
    Network,
    Http,
}

impl ScrappaApiError {
    fn timeout() -> Self {
        Self {
            kind: ScrappaApiErrorKind::Timeout,
            status: None,
            details: format!(
                "Scrappa API request timed out after {}ms",
                SCRAPPA_REQUEST_TIMEOUT.as_millis()
            ),
        }
    }

    fn network() -> Self {
        Self {
            kind: ScrappaApiErrorKind::Network,
            status: None,
            details: "Scrappa API network request failed".to_owned(),
        }
    }

    fn http(status: u16, details: String) -> Self {
        Self {
            kind: ScrappaApiErrorKind::Http,
            status: Some(status),
            details,
        }
    }
}

impl std::fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ScrappaApiErrorKind::Http => write!(
                formatter,
                "Scrappa API error ({}): {}",
                self.status.unwrap_or_default(),
                self.details
            ),
            _ => formatter.write_str(&self.details),
        }
    }
}

impl std::error::Error for ScrappaApiError {}

#[derive(Clone)]
struct ActorConfig {
    apify_api_base: String,
    apify_token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_base: String,
    scrappa_api_key: Option<String>,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_base: env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
            scrappa_api_key: env::var("SCRAPPA_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
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
    fn new(config: &ActorConfig) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Failed to create Apify API HTTP client")?,
            base_url: config.apify_api_base.clone(),
            token: config.apify_token.clone(),
            actor_run_id: config.actor_run_id.clone(),
            key_value_store_id: config.key_value_store_id.clone(),
            dataset_id: config.dataset_id.clone(),
            input_key: config.input_key.clone(),
        })
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.base_url, segments)
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let response = self
            .send_with_retries(
                || {
                    self.http
                        .get(url.clone())
                        .bearer_auth(&self.token)
                        .header(header::ACCEPT, "application/json")
                },
                "run pricing request",
                false,
            )
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
            .send_with_retries(
                || {
                    self.http
                        .get(url.clone())
                        .bearer_auth(&self.token)
                        .header(header::ACCEPT, "application/json")
                },
                "input retrieval",
                false,
            )
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        response_json(response, "input retrieval").await.map(Some)
    }

    async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(item)
            .send()
            .await
            .context("Failed to publish dataset item to Apify API")?;
        require_apify_success(response, "dataset item publication").await?;
        Ok(())
    }

    async fn charge_event(
        &self,
        event_name: &str,
        count: u64,
        idempotency_key: &str,
    ) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id, "charge"])?;
        let response = self
            .send_with_retries(
                || {
                    self.http
                        .post(url.clone())
                        .bearer_auth(&self.token)
                        .header(header::ACCEPT, "application/json")
                        .header("Idempotency-Key", idempotency_key)
                        .json(&json!({ "eventName": event_name, "count": count }))
                },
                "charge event request",
                true,
            )
            .await?;
        require_apify_success(response, "charge event request").await?;
        Ok(())
    }

    async fn set_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .send_with_retries(
                || {
                    self.http
                        .put(url.clone())
                        .bearer_auth(&self.token)
                        .header(header::ACCEPT, "application/json")
                        .json(output)
                },
                "OUTPUT record publication",
                false,
            )
            .await?;
        require_apify_success(response, "OUTPUT record publication").await?;
        Ok(())
    }

    async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let body = json!({
            "runId": self.actor_run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true,
        });
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to set Apify run status message")?;
        require_apify_success(response, "status message update").await?;
        Ok(())
    }

    async fn send_with_retries<F>(
        &self,
        make_request: F,
        operation: &str,
        retry_network_errors: bool,
    ) -> Result<Response>
    where
        F: Fn() -> RequestBuilder,
    {
        let mut retry_count = 0;
        loop {
            let response = match make_request().send().await {
                Ok(response) => response,
                Err(error)
                    if retry_network_errors
                        && retry_count < APIFY_MAX_RETRIES
                        && (error.is_timeout() || error.is_connect()) =>
                {
                    tokio::time::sleep(Duration::from_secs((retry_count + 1) as u64)).await;
                    retry_count += 1;
                    continue;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify {operation} failed"));
                }
            };

            if retry_count < APIFY_MAX_RETRIES
                && (response.status() == StatusCode::TOO_MANY_REQUESTS
                    || response.status().is_server_error())
            {
                drop(response);
                tokio::time::sleep(Duration::from_secs((retry_count + 1) as u64)).await;
                retry_count += 1;
                continue;
            }

            return Ok(response);
        }
    }
}

struct ScrappaClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    fn new(base_url: String, api_key: String) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(SCRAPPA_REQUEST_TIMEOUT)
                .build()
                .context("Failed to create Scrappa API HTTP client")?,
            base_url,
            api_key,
        })
    }

    async fn get_property(&self, property_id: u64) -> Result<Value> {
        let url = endpoint_url(&self.base_url, &["redfin", "property"])?;
        let mut last_error = None;

        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send_property_request(url.clone(), property_id).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let should_retry = is_retryable_scrappa_error(&error);
                    last_error = Some(error);
                    if !should_retry || attempt == SCRAPPA_MAX_ATTEMPTS {
                        break;
                    }

                    let delay_ms = get_retry_delay_ms(attempt, rand::thread_rng().gen_range(0..1000));
                    let error = last_error.as_ref().expect("the request failed");
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        SCRAPPA_MAX_ATTEMPTS,
                        delay_ms
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }

        Err(last_error.expect("at least one Scrappa request attempt was made"))
    }

    async fn send_property_request(&self, url: Url, property_id: u64) -> Result<Value> {
        let response = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, SCRAPPA_USER_AGENT)
            .query(&[("property_id", property_id)])
            .send()
            .await
            .map_err(map_scrappa_transport_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let status_code = status.as_u16();
            let fallback = status
                .canonical_reason()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("HTTP {status_code}"));
            let details = match response.bytes().await {
                Ok(body) => scrappa_error_message(status_code, &body, &fallback),
                Err(error) if error.is_timeout() => {
                    return Err(ScrappaApiError::timeout().into());
                }
                Err(_) => fallback,
            };
            return Err(ScrappaApiError::http(status_code, details).into());
        }

        let body = response
            .bytes()
            .await
            .map_err(map_scrappa_transport_error)?;
        serde_json::from_slice(&body).context("Scrappa API response was not valid JSON")
    }
}

fn map_scrappa_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaApiError::timeout().into()
    } else {
        ScrappaApiError::network().into()
    }
}

fn scrappa_error_message(status: u16, body: &[u8], fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }

    if let Ok(error_data) = serde_json::from_slice::<Value>(body) {
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
                                .map(js_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {joined}")
                })
                .filter(|detail| !detail.ends_with(": "))
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details);
            }
        }
        return message;
    }

    let body = String::from_utf8_lossy(body);
    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let message = collapsed.chars().take(500).collect::<String>();
    if message.is_empty() {
        fallback.to_owned()
    } else if status == 0 {
        fallback.to_owned()
    } else {
        message
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    let Some(error) = error.downcast_ref::<ScrappaApiError>() else {
        return false;
    };
    match error.kind {
        ScrappaApiErrorKind::Timeout | ScrappaApiErrorKind::Network => true,
        ScrappaApiErrorKind::Http => matches!(error.status, Some(408 | 429 | 500 | 502 | 503 | 504)),
    }
}

fn get_retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    (1000_u64
        .saturating_mul(2_u64.saturating_pow(failed_attempt as u32))
        .saturating_add(jitter_ms))
    .min(10_000)
}

fn is_per_property_scrappa_error(error: &ScrappaApiError) -> bool {
    matches!(error.status, Some(400 | 404 | 422))
        || (error.status == Some(500)
            && error
                .details
                .trim()
                .strip_suffix('.')
                .unwrap_or_else(|| error.details.trim())
                == TERMINAL_PROPERTY_ERROR)
}

struct PpeBudget {
    is_pay_per_event: bool,
    event_prices: HashMap<String, Option<f64>>,
    event_counts: HashMap<String, u64>,
    max_total_charge_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PushChargedPropertyResult {
    saved: bool,
    status_message: Option<String>,
    charged_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EventChargeResult {
    charged_count: u64,
    event_charge_limit_reached: bool,
}

impl PpeBudget {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_info = data.get("pricingInfo");
        if pricing_info
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(Self {
                is_pay_per_event: false,
                event_prices: HashMap::new(),
                event_counts: HashMap::new(),
                max_total_charge_usd: f64::INFINITY,
            });
        }

        let events = pricing_info
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (name, event) in events {
            let price = event.get("eventPriceUsd").and_then(Value::as_f64);
            if price.is_some_and(|price| !price.is_finite() || price < 0.0) {
                bail!("Apify run returned invalid price for charged event {name}");
            }
            event_prices.insert(name.clone(), price);
        }

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|value| *value != 0.0)
            .unwrap_or(f64::INFINITY);
        if !max_total_charge_usd.is_finite() && max_total_charge_usd != f64::INFINITY {
            bail!("Apify run returned an invalid spending limit");
        }

        let mut event_counts = HashMap::new();
        if let Some(counts) = data.get("chargedEventCounts").and_then(Value::as_object) {
            for (name, count) in counts {
                let count = count
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {name}"))?;
                event_counts.insert(name.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event: true,
            event_prices,
            event_counts,
            max_total_charge_usd,
        })
    }

    fn charge_limit_status(&self, total_results: usize, property_index: usize) -> Option<String> {
        if !self.is_pay_per_event
            || self.calculate_max_event_charge_count(REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT) > 0
        {
            return None;
        }

        Some(format!(
            "Charge limit reached before fetching Redfin property {}; {} property detail result(s) were saved.",
            property_index + 1,
            total_results
        ))
    }

    async fn push_property(
        &mut self,
        apify: &ApifyClient,
        property: &Value,
        property_index: usize,
    ) -> Result<PushChargedPropertyResult> {
        if !self.is_pay_per_event {
            apify.push_dataset_item(property).await?;
            return Ok(PushChargedPropertyResult {
                saved: true,
                status_message: None,
                charged_count: 0,
            });
        }

        self.push_dataset_data(apify, property, Some(REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT), property_index)
            .await
    }

    async fn push_error_item(&mut self, apify: &ApifyClient, item: &Value) -> Result<()> {
        self.push_dataset_data(apify, item, None, 0).await?;
        Ok(())
    }

    async fn push_dataset_data(
        &mut self,
        apify: &ApifyClient,
        item: &Value,
        event_name: Option<&str>,
        property_index: usize,
    ) -> Result<PushChargedPropertyResult> {
        let keep_item = self.calculate_push_data_limit(event_name) > 0;
        if !keep_item {
            let status_message = event_name.map(|_| {
                format!(
                    "Charge limit reached before saving Redfin property detail result {}.",
                    property_index + 1
                )
            });
            return Ok(PushChargedPropertyResult {
                saved: false,
                status_message,
                charged_count: 0,
            });
        }

        apify.push_dataset_item(item).await?;

        let mut charged_count = 0;
        let mut event_charge_limit_reached = false;
        if let Some(event_name) = event_name {
            let result = self
                .charge_event(
                    apify,
                    event_name,
                    1,
                    &format!("redfin-property-result-{}-{property_index}", apify.actor_run_id),
                )
                .await?;
            charged_count += result.charged_count;
            event_charge_limit_reached |= result.event_charge_limit_reached;
        }

        if self.is_pay_per_event {
            let default_result = self
                .charge_event(
                    apify,
                    DEFAULT_DATASET_ITEM_CHARGE_EVENT,
                    1,
                    &format!("redfin-default-dataset-item-{}-{property_index}", apify.actor_run_id),
                )
                .await?;
            charged_count += default_result.charged_count;
            event_charge_limit_reached |= default_result.event_charge_limit_reached;
        }

        if event_name.is_none() {
            return Ok(PushChargedPropertyResult {
                saved: true,
                status_message: None,
                charged_count,
            });
        }

        let status_message = if event_charge_limit_reached {
            let message = if charged_count >= 1 {
                format!(
                    "Charge limit reached after saving Redfin property detail result {}.",
                    property_index + 1
                )
            } else {
                format!(
                    "Charge limit reached before saving Redfin property detail result {}.",
                    property_index + 1
                )
            };
            println!(
                "{} {}",
                message,
                json!({
                    "event": REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT,
                    "charged_count": charged_count,
                    "property_index": property_index,
                })
            );
            Some(message)
        } else {
            None
        };

        Ok(PushChargedPropertyResult {
            saved: charged_count >= 1,
            status_message,
            charged_count,
        })
    }

    fn calculate_push_data_limit(&self, event_name: Option<&str>) -> u64 {
        let mut item_price = event_name
            .and_then(|name| self.event_prices.get(name).copied().flatten())
            .unwrap_or(0.0);
        item_price += self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_CHARGE_EVENT)
            .copied()
            .flatten()
            .unwrap_or(0.0);

        let max_count = if item_price > 0.0 {
            self.calculate_max_charges_by_price(item_price)
        } else {
            u64::MAX
        };
        if max_count >= 1 {
            return 1;
        }

        if self.total_charged_amount() <= self.max_total_charge_usd {
            1
        } else {
            0
        }
    }

    async fn charge_event(
        &mut self,
        apify: &ApifyClient,
        event_name: &str,
        requested_count: u64,
        idempotency_key: &str,
    ) -> Result<EventChargeResult> {
        let max_event_charge_count = self.calculate_max_event_charge_count(event_name);
        let total_charged = self.total_charged_amount();
        let charged_count = if requested_count <= max_event_charge_count {
            requested_count
        } else if total_charged <= self.max_total_charge_usd {
            max_event_charge_count.saturating_add(1)
        } else {
            0
        };

        if charged_count == 0 {
            return Ok(EventChargeResult {
                charged_count: 0,
                event_charge_limit_reached: requested_count > 0,
            });
        }

        let count = self.event_counts.entry(event_name.to_owned()).or_default();
        *count = count.saturating_add(charged_count);

        if !event_name.starts_with("apify-") && self.event_prices.contains_key(event_name) {
            apify
                .charge_event(event_name, charged_count, idempotency_key)
                .await?;
        }

        Ok(EventChargeResult {
            charged_count,
            event_charge_limit_reached: self.calculate_max_event_charge_count(event_name) == 0,
        })
    }

    fn calculate_max_event_charge_count(&self, event_name: &str) -> u64 {
        let Some(price) = self
            .event_prices
            .get(event_name)
            .copied()
            .flatten()
            .filter(|price| *price != 0.0)
        else {
            return u64::MAX;
        };
        self.calculate_max_charges_by_price(price)
    }

    fn calculate_max_charges_by_price(&self, price: f64) -> u64 {
        let unrounded = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !unrounded.is_finite() {
            return if unrounded.is_sign_positive() { u64::MAX } else { 0 };
        }

        let rounded = (unrounded * 10_000.0).round() / 10_000.0;
        rounded.floor().max(0.0).min(u64::MAX as f64) as u64
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .event_counts
            .iter()
            .map(|(name, count)| {
                self.event_prices
                    .get(name)
                    .copied()
                    .flatten()
                    .unwrap_or(0.0)
                    * *count as f64
            })
            .sum::<f64>();
        if total.is_finite() {
            (total * 1_000_000.0).round() / 1_000_000.0
        } else {
            total
        }
    }
}

async fn run_actor(config: ActorConfig, apify: &ApifyClient, run: &Value) -> Result<()> {
    let mut budget = PpeBudget::from_run(run)?;
    let api_key = config.scrappa_api_key.ok_or_else(|| {
        anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
    })?;

    let input = apify
        .get_input()
        .await?
        .ok_or_else(|| anyhow!("Input is required"))?;
    let requests = build_redfin_property_details_requests(&input)?;
    println!("Fetching {} Redfin property detail request(s)", requests.len());

    let scrappa = ScrappaClient::new(config.scrappa_api_base, api_key)?;
    let mut total_results = 0;
    let mut total_errors = 0;
    let mut status_message = None;
    let mut single_output_item = None;

    for request in &requests {
        status_message = budget.charge_limit_status(total_results, request.index);
        if let Some(message) = &status_message {
            println!(
                "{} {}",
                message,
                json!({
                    "event": REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT,
                    "properties_requested": requests.len(),
                    "results": total_results,
                    "next_property_index": request.index,
                })
            );
            break;
        }

        println!("Fetching Redfin details for {}", describe_request(request));
        match scrappa.get_property(request.params.property_id).await {
            Ok(response) => {
                let property = get_redfin_property_details(&response);
                if let Some(property) = property {
                    let item = build_redfin_property_details_dataset_item(property, request);
                    let push_result = budget
                        .push_property(apify, &item, request.index)
                        .await?;
                    if push_result.saved {
                        total_results += 1;
                        if requests.len() == 1 {
                            single_output_item = Some(item);
                        }
                    }
                    if push_result.status_message.is_some() {
                        status_message = push_result.status_message;
                        break;
                    }
                } else {
                    total_errors += 1;
                    let item = build_redfin_property_error_dataset_item(
                        request,
                        "Scrappa returned no property details",
                        None,
                    );
                    budget.push_error_item(apify, &item).await?;
                    if requests.len() == 1 {
                        single_output_item = Some(item);
                    }
                    println!(
                        "No Redfin property details found for property_id {}",
                        request.params.property_id
                    );
                }
            }
            Err(error) if is_per_property_error(&error) => {
                total_errors += 1;
                let scrappa_error = error
                    .downcast_ref::<ScrappaApiError>()
                    .expect("per-property errors are Scrappa HTTP errors");
                let item = build_redfin_property_error_dataset_item(
                    request,
                    &scrappa_error.details,
                    scrappa_error.status,
                );
                budget.push_error_item(apify, &item).await?;
                if requests.len() == 1 {
                    single_output_item = Some(item);
                }
                println!(
                    "Redfin property error for property_id {}: {}",
                    request.params.property_id, scrappa_error.details
                );
            }
            Err(error) => return Err(error),
        }
    }

    let output = if requests.len() == 1 {
        single_output_item.unwrap_or_else(|| {
            json!({
                "properties_requested": requests.len(),
                "results": total_results,
                "errors": total_errors,
                "status_message": status_message,
            })
        })
    } else {
        json!({
            "properties_requested": requests.len(),
            "results": total_results,
            "errors": total_errors,
            "status_message": status_message,
        })
    };
    apify.set_output(&output).await?;

    println!(
        "{}",
        status_message
            .as_ref()
            .map(|message| format!("Redfin property details completed: {message}"))
            .unwrap_or_else(|| "Redfin property details completed successfully".to_owned())
    );
    println!(
        "Results summary: {}",
        json!({
            "properties_requested": requests.len(),
            "results": total_results,
            "errors": total_errors,
            "status_message": status_message,
        })
    );
    Ok(())
}

fn is_per_property_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(is_per_property_scrappa_error)
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if let Some(scrappa_error) = error.downcast_ref::<ScrappaApiError>() {
        if scrappa_error.kind == ScrappaApiErrorKind::Timeout {
            return format!(
                "{}. The Redfin property details request exceeded the {}s Scrappa API timeout. Try fewer batched properties or run the request again.",
                scrappa_error,
                SCRAPPA_REQUEST_TIMEOUT.as_secs()
            );
        }
    }
    format!("{error:#}")
}

#[tokio::main]
async fn main() -> ExitCode {
    let config = match ActorConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {}", actor_error_message(&error));
            return ExitCode::FAILURE;
        }
    };
    let apify = match ApifyClient::new(&config) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Actor failed: {}", actor_error_message(&error));
            return ExitCode::FAILURE;
        }
    };
    let run = match apify.get_run().await {
        Ok(run) => run,
        Err(error) => {
            eprintln!("Actor failed: {}", actor_error_message(&error));
            return ExitCode::FAILURE;
        }
    };

    match run_actor(config, &apify, &run).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let message = actor_error_message(&error);
            eprintln!("Actor failed: {message}");
            let _ = apify.set_status_message(&message).await;
            ExitCode::FAILURE
        }
    }
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
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(trimmed.to_owned()))
}

fn clean_property_id(value: &Value, field: &str) -> Result<u64> {
    let property_id = match value {
        Value::String(value) if value.trim().chars().all(|character| character.is_ascii_digit()) => {
            value.trim().parse::<f64>().ok()
        }
        Value::Number(value) => {
            let number = value.as_f64().unwrap_or(f64::NAN);
            if number.is_finite() && number.fract() == 0.0 {
                Some(number)
            } else {
                None
            }
        }
        _ => None,
    }
    .ok_or_else(|| anyhow!("{field} must be an integer"))?;

    if property_id <= 0.0 {
        bail!("{field} must be greater than 0");
    }
    if property_id > MAX_SAFE_INTEGER as f64 {
        bail!("{field} is too large");
    }
    Ok(property_id as u64)
}

fn extract_redfin_property_id_from_url(value: &Value, field: &str) -> Result<u64> {
    let url_string = clean_string(Some(value), field, 2048)?
        .ok_or_else(|| anyhow!("{field} is required"))?;
    let url = Url::parse(&url_string).map_err(|_| {
        anyhow!("{field} must be a valid Redfin URL containing /home/{{property_id}}")
    })?;
    let hostname = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if hostname != "redfin.com" && !hostname.ends_with(".redfin.com") {
        bail!("{field} must be a Redfin URL");
    }

    let path = url.path();
    let path = path.strip_suffix('/').unwrap_or(path);
    let Some((prefix, property_id)) = path.rsplit_once("/home/") else {
        bail!("{field} must contain a /home/{{property_id}} path");
    };
    if property_id.is_empty()
        || property_id.contains('/')
        || !property_id.chars().all(|character| character.is_ascii_digit())
        || prefix.is_empty() && path != format!("/home/{property_id}")
    {
        bail!("{field} must contain a /home/{{property_id}} path");
    }

    let raw_id = json!(property_id);
    clean_property_id(&raw_id, field)
}

fn add_request(
    requests: &mut Vec<RedfinPropertyDetailsRequest>,
    seen_property_ids: &mut HashSet<u64>,
    property_id: u64,
    input: Value,
    source: &'static str,
) {
    if !seen_property_ids.insert(property_id) {
        return;
    }
    requests.push(RedfinPropertyDetailsRequest {
        params: RedfinPropertyDetailsParams { property_id },
        index: requests.len(),
        input,
        source,
    });
}

fn add_property_id_input(
    requests: &mut Vec<RedfinPropertyDetailsRequest>,
    seen_property_ids: &mut HashSet<u64>,
    value: &Value,
    source: &'static str,
    field: &str,
) -> Result<()> {
    let property_id = clean_property_id(value, field)?;
    let input = match value {
        Value::String(value) => Value::String(value.trim().to_owned()),
        _ => json!(property_id),
    };
    add_request(requests, seen_property_ids, property_id, input, source);
    Ok(())
}

fn add_url_input(
    requests: &mut Vec<RedfinPropertyDetailsRequest>,
    seen_property_ids: &mut HashSet<u64>,
    value: &Value,
    source: &'static str,
    field: &str,
) -> Result<()> {
    let url = clean_string(Some(value), field, 2048)?
        .ok_or_else(|| anyhow!("{field} is required"))?;
    let property_id = extract_redfin_property_id_from_url(&Value::String(url.clone()), field)?;
    add_request(
        requests,
        seen_property_ids,
        property_id,
        Value::String(url),
        source,
    );
    Ok(())
}

fn build_redfin_property_details_requests(input: &Value) -> Result<Vec<RedfinPropertyDetailsRequest>> {
    let fields = input.as_object();
    let mut requests = Vec::new();
    let mut seen_property_ids = HashSet::new();

    if let Some(value) = fields.and_then(|input| input.get("property_id")) {
        if !value.is_null() && value.as_str() != Some("") {
            add_property_id_input(
                &mut requests,
                &mut seen_property_ids,
                value,
                "property_id",
                "property_id",
            )?;
        }
    }

    if let Some(value) = fields.and_then(|input| input.get("url")) {
        if !value.is_null() && value.as_str() != Some("") {
            add_url_input(&mut requests, &mut seen_property_ids, value, "url", "url")?;
        }
    }

    if let Some(value) = fields.and_then(|input| input.get("property_ids")) {
        let Some(values) = value.as_array() else {
            bail!("property_ids must be an array");
        };
        for (index, property_id) in values.iter().enumerate() {
            add_property_id_input(
                &mut requests,
                &mut seen_property_ids,
                property_id,
                "property_ids",
                &format!("property_ids[{index}]"),
            )?;
        }
    }

    if let Some(value) = fields.and_then(|input| input.get("urls")) {
        let Some(values) = value.as_array() else {
            bail!("urls must be an array");
        };
        for (index, url) in values.iter().enumerate() {
            add_url_input(
                &mut requests,
                &mut seen_property_ids,
                url,
                "urls",
                &format!("urls[{index}]"),
            )?;
        }
    }

    if requests.is_empty() {
        bail!("Provide at least one property_id, property_ids item, url, or urls item");
    }
    if requests.len() > MAX_PROPERTIES_PER_RUN {
        bail!(
            "Input cannot include more than {MAX_PROPERTIES_PER_RUN} unique properties per run"
        );
    }
    Ok(requests)
}

fn describe_request(request: &RedfinPropertyDetailsRequest) -> String {
    format!(
        "property_id {} from {}",
        request.params.property_id, request.source
    )
}

fn get_redfin_property_details(response: &Value) -> Option<Map<String, Value>> {
    let data = response.get("data");
    if let Some(properties) = data.and_then(Value::as_array) {
        return properties.first().and_then(Value::as_object).cloned();
    }
    if let Some(property) = data.and_then(Value::as_object) {
        return Some(property.clone());
    }
    response.get("property").and_then(Value::as_object).cloned()
}

fn first_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().find_map(|value| {
        let value = (*value)?;
        match value {
            Value::String(value) if !value.trim().is_empty() => Some(value.clone()),
            Value::Number(value) if value.as_f64().is_some_and(f64::is_finite) => {
                Some(value.to_string())
            }
            _ => None,
        }
    })
}

fn first_number(values: &[Option<&Value>]) -> Option<Number> {
    values.iter().find_map(|value| {
        let value = (*value)?;
        let number = match value {
            Value::Number(value) => value.as_f64().filter(|number| number.is_finite()),
            Value::String(value) if !value.trim().is_empty() => {
                value.trim().parse::<f64>().ok().filter(|number| number.is_finite())
            }
            _ => None,
        }?;
        if number.fract() == 0.0 && number >= 0.0 && number <= u64::MAX as f64 {
            Some(Number::from(number as u64))
        } else if number.fract() == 0.0 && number >= i64::MIN as f64 {
            Some(Number::from(number as i64))
        } else {
            Number::from_f64(number)
        }
    })
}

fn build_redfin_property_details_dataset_item(
    property: Map<String, Value>,
    request: &RedfinPropertyDetailsRequest,
) -> Value {
    let mut item = property;
    let requested_id = json!(request.params.property_id);
    item.insert(
        "property_id".to_owned(),
        first_number(&[item.get("property_id"), Some(&requested_id)])
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );

    for field in ["address", "city", "state", "country", "price_label", "status_label", "url", "description"] {
        let value = first_string(&[item.get(field)]).map(Value::String).unwrap_or(Value::Null);
        item.insert(field.to_owned(), value);
    }
    item.insert(
        "zip".to_owned(),
        first_string(&[item.get("zip")]).map(Value::String).unwrap_or(Value::Null),
    );
    for field in [
        "price",
        "beds",
        "baths",
        "sqft",
        "lot_size",
        "year_built",
        "property_type",
        "status",
        "latitude",
        "longitude",
    ] {
        let value = first_number(&[item.get(field)]).map(Value::Number).unwrap_or(Value::Null);
        item.insert(field.to_owned(), value);
    }
    if !item.get("photos").is_some_and(Value::is_array) {
        item.insert("photos".to_owned(), Value::Array(Vec::new()));
    }
    item.insert("request_property_index".to_owned(), json!(request.index));
    item.insert(
        "request_property_id".to_owned(),
        json!(request.params.property_id),
    );
    item.insert("request_input".to_owned(), request.input.clone());
    item.insert("request_source".to_owned(), json!(request.source));
    Value::Object(item)
}

fn build_redfin_property_error_dataset_item(
    request: &RedfinPropertyDetailsRequest,
    message: &str,
    status_code: Option<u16>,
) -> Value {
    json!({
        "success": false,
        "property_id": request.params.property_id,
        "request_property_index": request.index,
        "request_property_id": request.params.property_id,
        "request_input": request.input,
        "request_source": request.source,
        "error": message,
        "status_code": status_code,
    })
}

fn endpoint_url(base_url: &str, segments: &[&str]) -> Result<Url> {
    let mut url = Url::parse(base_url).with_context(|| format!("Invalid API base URL: {base_url}"))?;
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        task::JoinHandle,
    };

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
    }

    #[derive(Debug)]
    struct CapturedRequest {
        line: String,
        headers: String,
        body: String,
    }

    async fn start_mock_server(
        responses: Vec<MockResponse>,
    ) -> (String, JoinHandle<Vec<CapturedRequest>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::with_capacity(responses.len());
            for response in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let request = read_http_request(&mut stream).await;
                let reason = match response.status {
                    201 => "Created",
                    503 => "Service Unavailable",
                    _ => "OK",
                };
                let body = response.body.as_bytes();
                let headers = format!(
                    "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
                    response.status,
                    reason,
                    body.len()
                );
                stream.write_all(headers.as_bytes()).await.unwrap();
                stream.write_all(body).await.unwrap();
                requests.push(request);
            }
            requests
        });
        (format!("http://{address}"), server)
    }

    async fn read_http_request(stream: &mut TcpStream) -> CapturedRequest {
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 2048];
        let header_end = loop {
            let read = stream.read(&mut chunk).await.unwrap();
            assert!(read > 0, "request ended before its headers were complete");
            bytes.extend_from_slice(&chunk[..read]);
            if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                break index + 4;
            }
        };
        let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        while bytes.len() - header_end < content_length {
            let read = stream.read(&mut chunk).await.unwrap();
            assert!(read > 0, "request ended before its body was complete");
            bytes.extend_from_slice(&chunk[..read]);
        }
        let body = String::from_utf8_lossy(&bytes[header_end..header_end + content_length]).to_string();
        let line = headers.lines().next().unwrap_or_default().to_owned();
        CapturedRequest {
            line,
            headers,
            body,
        }
    }

    #[test]
    fn builds_requests_in_input_order_and_deduplicates_ids_and_urls() {
        let input = json!({
            "property_id": "60791456",
            "url": "https://redfin.com/home/194191988?ref=one",
            "property_ids": [60791456, "194191988", 23232323],
            "urls": ["https://www.redfin.com/TN/Memphis/home/456456456/"]
        });

        let requests = build_redfin_property_details_requests(&input).unwrap();
        assert_eq!(
            requests
                .iter()
                .map(|request| request.params.property_id)
                .collect::<Vec<_>>(),
            vec![60791456, 194191988, 23232323, 456456456]
        );
        assert_eq!(requests[0].input, json!("60791456"));
        assert_eq!(requests[1].source, "url");
        assert_eq!(requests[1].input, json!("https://redfin.com/home/194191988?ref=one"));
        assert_eq!(describe_request(&requests[2]), "property_id 23232323 from property_ids");
    }

    #[test]
    fn validates_property_ids_urls_and_batch_limit() {
        assert_eq!(
            build_redfin_property_details_requests(&json!({"property_id": 0}))
                .unwrap_err()
                .to_string(),
            "property_id must be greater than 0"
        );
        assert_eq!(
            build_redfin_property_details_requests(&json!({"url": "https://example.com/home/123"}))
                .unwrap_err()
                .to_string(),
            "url must be a Redfin URL"
        );
        assert_eq!(
            build_redfin_property_details_requests(&json!({"url": "https://redfin.com/not-a-property"}))
                .unwrap_err()
                .to_string(),
            "url must contain a /home/{property_id} path"
        );

        let ids = (1..=101).collect::<Vec<_>>();
        assert_eq!(
            build_redfin_property_details_requests(&json!({"property_ids": ids}))
                .unwrap_err()
                .to_string(),
            "Input cannot include more than 100 unique properties per run"
        );
    }

    #[test]
    fn preserves_schema_prefill_and_batch_constraints() {
        let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema.pointer("/properties/property_id/default"), Some(&json!(60791456)));
        assert_eq!(
            schema.pointer("/properties/url/prefill"),
            Some(&json!("https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/60791456"))
        );
        assert_eq!(schema.pointer("/properties/property_ids/maxItems"), Some(&json!(100)));
        assert_eq!(schema.pointer("/properties/urls/maxItems"), Some(&json!(100)));
    }

    #[test]
    fn extracts_data_and_property_fallbacks() {
        let property = json!({"property_id": 60791456, "address": "1549 Ely St"});
        assert_eq!(
            get_redfin_property_details(&json!({"data": property})),
            property.as_object().cloned()
        );
        assert_eq!(
            get_redfin_property_details(&json!({"data": [property.clone()]})),
            property.as_object().cloned()
        );
        assert_eq!(
            get_redfin_property_details(&json!({"property": property.clone()})),
            property.as_object().cloned()
        );
        assert_eq!(get_redfin_property_details(&json!({"data": []})), None);
    }

    #[test]
    fn normalizes_property_fields_and_preserves_unmapped_upstream_fields() {
        let request = build_redfin_property_details_requests(&json!({
            "url": "https://redfin.com/home/60791456"
        }))
        .unwrap()
        .remove(0);
        let property = json!({
            "property_id": "60791456",
            "address": "1549 Ely St",
            "zip": 38106,
            "price": "125000",
            "beds": "3",
            "photos": [{"url": "https://example.com/photo.jpg"}],
            "description": "Property description",
            "upstream_only": {"kept": true}
        });
        let item = build_redfin_property_details_dataset_item(
            property.as_object().unwrap().clone(),
            &request,
        );

        assert_eq!(item["property_id"], json!(60791456));
        assert_eq!(item["zip"], json!("38106"));
        assert_eq!(item["price"], json!(125000));
        assert_eq!(item["beds"], json!(3));
        assert_eq!(item["photos"], property["photos"]);
        assert_eq!(item["request_property_index"], json!(0));
        assert_eq!(item["request_input"], json!("https://redfin.com/home/60791456"));
        assert_eq!(item["request_source"], json!("url"));
        assert_eq!(item["upstream_only"], json!({"kept": true}));
        assert_eq!(item["city"], Value::Null);
    }

    #[test]
    fn builds_non_charged_per_property_error_rows() {
        let request = build_redfin_property_details_requests(&json!({"property_id": 60791456}))
            .unwrap()
            .remove(0);
        assert_eq!(
            build_redfin_property_error_dataset_item(&request, "Property not found", Some(404)),
            json!({
                "success": false,
                "property_id": 60791456,
                "request_property_index": 0,
                "request_property_id": 60791456,
                "request_input": 60791456,
                "request_source": "property_id",
                "error": "Property not found",
                "status_code": 404
            })
        );
    }

    #[test]
    fn classifies_transient_and_per_property_errors() {
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_scrappa_error(
                &ScrappaApiError::http(status, "temporary".to_owned()).into()
            ));
        }
        assert!(!is_retryable_scrappa_error(
            &ScrappaApiError::http(422, "invalid".to_owned()).into()
        ));
        assert!(is_per_property_scrappa_error(&ScrappaApiError::http(
            400,
            "Bad request".to_owned()
        )));
        assert!(is_per_property_scrappa_error(&ScrappaApiError::http(
            500,
            " Failed to fetch property details after multiple attempts. ".to_owned()
        )));
        assert!(!is_per_property_scrappa_error(&ScrappaApiError::http(
            500,
            "Server error".to_owned()
        )));
        assert_eq!(get_retry_delay_ms(1, 0), 2000);
        assert_eq!(get_retry_delay_ms(2, 500), 4500);
        assert_eq!(get_retry_delay_ms(20, 0), 10_000);
    }

    #[test]
    fn calculates_per_event_budget_and_stops_before_the_next_property() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "property-result": {"eventPriceUsd": 0.0005}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.0005},
                "chargedEventCounts": {}
            }
        });
        let mut budget = PpeBudget::from_run(&run).unwrap();
        assert!(budget.charge_limit_status(0, 0).is_none());
        assert_eq!(budget.calculate_push_data_limit(Some("property-result")), 1);

        *budget.event_counts.entry("property-result".to_owned()).or_default() = 1;
        assert_eq!(
            budget.charge_limit_status(1, 1),
            Some("Charge limit reached before fetching Redfin property 2; 1 property detail result(s) were saved.".to_owned())
        );
    }

    #[test]
    fn leaves_non_ppe_runs_unlimited_and_handles_default_dataset_price() {
        let free_run = json!({"data": {"pricingInfo": {"pricingModel": "FREE"}}});
        let free = PpeBudget::from_run(&free_run).unwrap();
        assert!(!free.is_pay_per_event);
        assert!(free.charge_limit_status(0, 0).is_none());

        let paid_run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "property-result": {"eventPriceUsd": 0.0004},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.0005},
                "chargedEventCounts": {}
            }
        });
        let budget = PpeBudget::from_run(&paid_run).unwrap();
        assert_eq!(budget.calculate_push_data_limit(Some("property-result")), 1);
        assert_eq!(budget.calculate_push_data_limit(None), 1);
    }

    #[tokio::test]
    async fn runs_batch_with_scrappa_auth_retry_dataset_output_and_ppe_budget() {
        let responses = vec![
            MockResponse::json(
                200,
                json!({
                    "data": {
                        "pricingInfo": {
                            "pricingModel": "PAY_PER_EVENT",
                            "pricingPerEvent": {"actorChargeEvents": {
                                "property-result": {"eventPriceUsd": 0.0005}
                            }}
                        },
                        "options": {"maxTotalChargeUsd": 0.0005},
                        "chargedEventCounts": {}
                    }
                }),
            ),
            MockResponse::json(200, json!({"property_ids": [123, 456]})),
            MockResponse::json(503, json!({"message": "Temporary Scrappa issue"})),
            MockResponse::json(
                200,
                json!({"data": {"property_id": 123, "address": "1 Main St"}}),
            ),
            MockResponse::json(201, Value::Null),
            MockResponse::json(201, Value::Null),
            MockResponse::json(200, Value::Null),
        ];
        let (base_url, server) = start_mock_server(responses).await;
        let config = ActorConfig {
            apify_api_base: base_url.clone(),
            apify_token: "apify-test-token".to_owned(),
            actor_run_id: "run-test".to_owned(),
            key_value_store_id: "store-test".to_owned(),
            dataset_id: "dataset-test".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_base: format!("{base_url}/api"),
            scrappa_api_key: Some("scrappa-test-key".to_owned()),
        };
        let apify = ApifyClient::new(&config).unwrap();
        let run = apify.get_run().await.unwrap();
        run_actor(config, &apify, &run).await.unwrap();

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 7);
        assert!(requests[0]
            .line
            .starts_with("GET /v2/actor-runs/run-test HTTP/1.1"));
        assert!(requests[0]
            .headers
            .to_ascii_lowercase()
            .contains("authorization: bearer apify-test-token"));
        assert!(requests[1]
            .line
            .starts_with("GET /v2/key-value-stores/store-test/records/INPUT HTTP/1.1"));

        for request in [&requests[2], &requests[3]] {
            assert!(request
                .line
                .starts_with("GET /api/redfin/property?property_id=123 HTTP/1.1"));
            let headers = request.headers.to_ascii_lowercase();
            assert!(headers.contains("x-api-key: scrappa-test-key"));
            assert!(headers.contains("accept: application/json"));
            assert!(headers.contains(&format!("user-agent: {SCRAPPA_USER_AGENT}")));
        }

        assert!(requests[4]
            .line
            .starts_with("POST /v2/datasets/dataset-test/items HTTP/1.1"));
        let dataset_item: Value = serde_json::from_str(&requests[4].body).unwrap();
        assert_eq!(dataset_item["property_id"], json!(123));
        assert_eq!(dataset_item["address"], json!("1 Main St"));

        assert!(requests[5]
            .line
            .starts_with("POST /v2/actor-runs/run-test/charge HTTP/1.1"));
        assert!(requests[5]
            .headers
            .to_ascii_lowercase()
            .contains("idempotency-key: redfin-property-result-run-test-0"));
        let charged_event: Value = serde_json::from_str(&requests[5].body).unwrap();
        assert_eq!(charged_event, json!({"eventName": "property-result", "count": 1}));

        assert!(requests[6]
            .line
            .starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT HTTP/1.1"));
        let output: Value = serde_json::from_str(&requests[6].body).unwrap();
        assert_eq!(output["properties_requested"], json!(2));
        assert_eq!(output["results"], json!(1));
        assert_eq!(output["errors"], json!(0));
        assert_eq!(
            output["status_message"],
            json!("Charge limit reached after saving Redfin property detail result 1.")
        );
    }
}
