use crate::ports::{PushResult, ResultsSink};
use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Request, RequestBuilder, Response, StatusCode};
use serde_json::{json, Value};
use std::{collections::HashMap, env, time::Duration};
use tokio::time::sleep;
use url::Url;

pub const RESULT_EVENT: &str = "challenge-post-result";
pub const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const APIFY_CLIENT_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: usize = 8;
const APIFY_RETRY_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub retries: usize,
    pub minimum_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            retries: APIFY_MAX_RETRIES,
            minimum_delay: APIFY_RETRY_DELAY,
        }
    }
}

#[derive(Clone)]
pub struct ActorConfig {
    pub apify_api_base_url: Url,
    pub default_key_value_store_id: String,
    pub default_dataset_id: String,
    pub actor_run_id: String,
    pub input_key: String,
    pub apify_token: String,
}

impl ActorConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
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
        .extend(segments);
    Ok(url)
}

pub struct ApifyClient {
    client: Client,
    config: ActorConfig,
    retry_policy: RetryPolicy,
}

impl ApifyClient {
    pub fn new(config: ActorConfig) -> Result<Self> {
        Self::with_retry_policy(config, RetryPolicy::default(), APIFY_CLIENT_TIMEOUT)
    }

    pub fn with_retry_policy(
        config: ActorConfig,
        retry_policy: RetryPolicy,
        timeout: Duration,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .context("Failed to create Apify HTTP client")?;
        Ok(Self {
            client,
            config,
            retry_policy,
        })
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &[
                "v2",
                "key-value-stores",
                &self.config.default_key_value_store_id,
                "records",
                &self.config.input_key,
            ],
        )?;
        self.get_json(url, "Apify INPUT request", true).await
    }

    pub async fn get_run(&self) -> Result<Value> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "actor-runs", &self.config.actor_run_id],
        )?;
        self.get_json(url, "Apify run pricing request", false)
            .await?
            .ok_or_else(|| anyhow!("Apify run pricing request returned no run"))
    }

    pub async fn push_dataset_items(&self, rows: &[Value]) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "datasets", &self.config.default_dataset_id, "items"],
        )?;
        let request = self
            .client
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .json(rows);
        self.send_unit_request_once(request, "Apify dataset write")
            .await
    }

    async fn charge_event(&self, event_name: &str, idempotency_key: &str) -> Result<()> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "actor-runs", &self.config.actor_run_id, "charge"],
        )?;
        let request = self
            .client
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header("idempotency-key", idempotency_key)
            .json(&json!({ "eventName": event_name, "count": 1 }));
        self.send_unit_request(request, "Apify event charge").await
    }

    async fn get_json(
        &self,
        url: Url,
        operation: &str,
        missing_record_is_empty: bool,
    ) -> Result<Option<Value>> {
        let request = self
            .client
            .get(url)
            .bearer_auth(&self.config.apify_token)
            .build()
            .with_context(|| format!("{operation} could not be built"))?;

        for attempt in 0..=self.retry_policy.retries {
            let response = match self
                .client
                .execute(clone_request(&request, operation)?)
                .await
            {
                Ok(response) => response,
                Err(error) if is_retryable_error(&error) && attempt < self.retry_policy.retries => {
                    self.wait_before_retry(attempt).await;
                    continue;
                }
                Err(error) => return Err(anyhow!("{operation} failed: {error}")),
            };

            if response.status() == StatusCode::NOT_FOUND && missing_record_is_empty {
                return Ok(None);
            }
            if is_retryable_status(response.status()) && attempt < self.retry_policy.retries {
                self.wait_before_retry(attempt).await;
                continue;
            }
            if !response.status().is_success() {
                return Err(response_error(response, operation).await);
            }

            match response.json::<Value>().await {
                Ok(value) => return Ok(Some(value)),
                Err(_) if attempt < self.retry_policy.retries => {
                    self.wait_before_retry(attempt).await;
                }
                Err(error) => return Err(anyhow!("{operation} returned invalid JSON: {error}")),
            }
        }

        unreachable!("the Apify JSON request loop always returns or fails")
    }

    async fn send_unit_request(&self, request: RequestBuilder, operation: &str) -> Result<()> {
        let request = request
            .build()
            .with_context(|| format!("{operation} could not be built"))?;

        for attempt in 0..=self.retry_policy.retries {
            let response = match self
                .client
                .execute(clone_request(&request, operation)?)
                .await
            {
                Ok(response) => response,
                Err(error) if is_retryable_error(&error) && attempt < self.retry_policy.retries => {
                    self.wait_before_retry(attempt).await;
                    continue;
                }
                Err(error) => return Err(anyhow!("{operation} failed: {error}")),
            };

            if is_retryable_status(response.status()) && attempt < self.retry_policy.retries {
                self.wait_before_retry(attempt).await;
                continue;
            }
            if !response.status().is_success() {
                return Err(response_error(response, operation).await);
            }
            return Ok(());
        }

        unreachable!("the Apify request loop always returns or fails")
    }

    async fn send_unit_request_once(&self, request: RequestBuilder, operation: &str) -> Result<()> {
        let request = request
            .build()
            .with_context(|| format!("{operation} could not be built"))?;
        let response = self
            .client
            .execute(request)
            .await
            .map_err(|error| anyhow!("{operation} failed: {error}"))?;
        if !response.status().is_success() {
            return Err(response_error(response, operation).await);
        }
        Ok(())
    }

    async fn wait_before_retry(&self, attempt: usize) {
        let multiplier = 1_u32
            .checked_shl(attempt.min(31) as u32)
            .unwrap_or(u32::MAX);
        sleep(self.retry_policy.minimum_delay.saturating_mul(multiplier)).await;
    }
}

fn clone_request(request: &Request, operation: &str) -> Result<Request> {
    request.try_clone().ok_or_else(|| {
        anyhow!("{operation} request cannot be retried because its body is not reusable")
    })
}

fn is_retryable_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request() || error.is_body()
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

async fn response_error(response: Response, operation: &str) -> anyhow::Error {
    let status = response.status();
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    anyhow!(
        "{operation} failed with {} {reason}{detail}",
        status.as_u16()
    )
}

#[derive(Debug, Clone)]
pub struct RunPricing {
    pub is_pay_per_event: bool,
    prices: HashMap<String, f64>,
    charged_counts: HashMap<String, u64>,
    max_total_charge_usd: f64,
}

impl RunPricing {
    pub fn from_run_response(run: &Value) -> Result<Self> {
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
                prices: HashMap::new(),
                charged_counts: HashMap::new(),
                max_total_charge_usd: f64::INFINITY,
            });
        }

        let events = pricing_info
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut prices = HashMap::new();
        for (event_name, event) in events {
            if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
                if !price.is_finite() || price < 0.0 {
                    bail!("Apify run returned an invalid price for charged event {event_name}");
                }
                prices.insert(event_name.clone(), price);
            }
        }

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .unwrap_or(f64::INFINITY);
        if max_total_charge_usd.is_nan() || max_total_charge_usd < 0.0 {
            bail!("Apify run returned an invalid spending limit");
        }

        let mut charged_counts = HashMap::new();
        if let Some(counts) = data.get("chargedEventCounts").and_then(Value::as_object) {
            for (event_name, count) in counts {
                let count = count
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
                charged_counts.insert(event_name.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event,
            prices,
            charged_counts,
            max_total_charge_usd,
        })
    }

    pub fn available_result_capacity(&self, requested: usize) -> usize {
        if !self.is_pay_per_event {
            return requested;
        }
        let price = self.prices.get(RESULT_EVENT).copied().unwrap_or(0.0)
            + self
                .prices
                .get(DEFAULT_DATASET_ITEM_EVENT)
                .copied()
                .unwrap_or(0.0);
        requested.min(self.max_count_by_price(price))
    }

    fn available_for_event(&self, event_name: &str) -> usize {
        let price = self.prices.get(event_name).copied().unwrap_or(0.0);
        self.max_count_by_price(price)
    }

    fn max_count_by_price(&self, price: f64) -> usize {
        if price <= 0.0 || self.max_total_charge_usd == f64::INFINITY {
            return usize::MAX;
        }
        let available = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !available.is_finite() {
            return if available.is_sign_positive() {
                usize::MAX
            } else {
                0
            };
        }
        if available <= 0.0 {
            return 0;
        }
        let capacity = available.floor();
        let mut capacity = if capacity >= usize::MAX as f64 {
            usize::MAX
        } else {
            capacity as usize
        };
        let total = self.total_charged_amount();
        while capacity > 0 && total + price * capacity as f64 > self.max_total_charge_usd {
            capacity -= 1;
        }
        capacity
    }

    fn total_charged_amount(&self) -> f64 {
        self.charged_counts
            .iter()
            .map(|(event_name, count)| {
                self.prices.get(event_name).copied().unwrap_or(0.0) * *count as f64
            })
            .sum::<f64>()
    }

    fn can_push_one_default_result(&self) -> bool {
        let total = self.total_charged_amount();
        let item_price = self.prices.get(RESULT_EVENT).copied().unwrap_or(0.0)
            + self
                .prices
                .get(DEFAULT_DATASET_ITEM_EVENT)
                .copied()
                .unwrap_or(0.0);
        total + item_price <= self.max_total_charge_usd
    }

    fn record_charge(&mut self, event_name: &str) {
        *self
            .charged_counts
            .entry(event_name.to_owned())
            .or_insert(0) += 1;
    }
}

pub struct ApifyActor {
    api: ApifyClient,
    pricing: RunPricing,
    next_charge_index: u64,
}

impl ApifyActor {
    pub fn new(api: ApifyClient, pricing: RunPricing) -> Self {
        let next_charge_index = pricing
            .charged_counts
            .get(RESULT_EVENT)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        Self {
            api,
            pricing,
            next_charge_index,
        }
    }

    async fn push_one_video(&mut self, row: &Value) -> Result<PushResult> {
        if !self.pricing.can_push_one_default_result() {
            return Ok(PushResult {
                saved: 0,
                limit_reached: true,
            });
        }

        self.api
            .push_dataset_items(std::slice::from_ref(row))
            .await?;

        // Apify charges this synthetic event from default dataset writes. Keep it
        // in the local budget before the next result is considered.
        self.pricing.record_charge(DEFAULT_DATASET_ITEM_EVENT);
        self.pricing.record_charge(RESULT_EVENT);

        if self.pricing.prices.contains_key(RESULT_EVENT) {
            let idempotency_key = format!(
                "{}-{}-{}",
                self.api.config.actor_run_id, RESULT_EVENT, self.next_charge_index
            );
            self.next_charge_index = self.next_charge_index.saturating_add(1);
            self.api
                .charge_event(RESULT_EVENT, &idempotency_key)
                .await?;
        }

        let limit_reached = self.pricing.available_for_event(RESULT_EVENT) == 0
            || self.pricing.available_for_event(DEFAULT_DATASET_ITEM_EVENT) == 0;
        Ok(PushResult {
            saved: 1,
            limit_reached,
        })
    }
}

impl ResultsSink for ApifyActor {
    fn available_capacity(&self, requested: usize) -> usize {
        self.pricing.available_result_capacity(requested)
    }

    async fn push_videos(&mut self, rows: &[Value]) -> Result<PushResult> {
        if rows.is_empty() {
            return Ok(PushResult {
                saved: 0,
                limit_reached: false,
            });
        }
        if !self.pricing.is_pay_per_event {
            self.api.push_dataset_items(rows).await?;
            return Ok(PushResult {
                saved: rows.len(),
                limit_reached: false,
            });
        }

        let mut saved = 0;
        for row in rows {
            let result = self.push_one_video(row).await?;
            saved += result.saved;
            if result.limit_reached {
                return Ok(PushResult {
                    saved,
                    limit_reached: true,
                });
            }
        }
        Ok(PushResult {
            saved,
            limit_reached: false,
        })
    }
}
