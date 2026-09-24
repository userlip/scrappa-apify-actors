use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    env,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: usize = 8;
const APIFY_MIN_RETRY_DELAY: Duration = Duration::from_millis(500);
const REVIEW_RESULT_EVENT: &str = "review-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

static CHARGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct ApifyConfig {
    pub base_url: String,
    pub token: String,
    pub actor_run_id: String,
    pub key_value_store_id: String,
    pub dataset_id: String,
    pub input_key: String,
}

impl ApifyConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            base_url: env::var("APIFY_API_PUBLIC_BASE_URL")
                .unwrap_or_else(|_| APIFY_API_DEFAULT.to_owned()),
            token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is missing"))
}

#[derive(Clone)]
pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    pub fn new(config: ApifyConfig) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(&config.base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token: config.token,
            actor_run_id: config.actor_run_id,
            key_value_store_id: config.key_value_store_id,
            dataset_id: config.dataset_id,
            input_key: config.input_key,
        })
    }

    fn resource_url(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot be a base URL"))?;
        path.pop_if_empty();
        path.extend(["v2"].into_iter().chain(segments.iter().copied()));
        drop(path);
        Ok(url)
    }

    fn record_url(&self, store_id: &str, record_key: &str) -> Result<Url> {
        self.resource_url(&["key-value-stores", store_id, "records", record_key])
    }

    async fn send(
        &self,
        method: Method,
        url: Url,
        body: Option<Value>,
        idempotency_key: Option<String>,
        operation: &str,
    ) -> Result<Response> {
        for retry in 0..=APIFY_MAX_RETRIES {
            let mut request = self
                .client
                .request(method.clone(), url.clone())
                .bearer_auth(&self.token)
                .header(reqwest::header::ACCEPT, "application/json");
            if let Some(body) = &body {
                request = request.json(body);
            }
            if let Some(idempotency_key) = &idempotency_key {
                request = request.header("Idempotency-Key", idempotency_key);
            }

            match request.send().await {
                Ok(response)
                    if is_retryable_status(response.status()) && retry < APIFY_MAX_RETRIES =>
                {
                    eprintln!(
                        "Apify API {operation} returned HTTP {}; retrying ({}/{})",
                        response.status(),
                        retry + 1,
                        APIFY_MAX_RETRIES
                    );
                    tokio::time::sleep(retry_delay(retry)).await;
                }
                Ok(response) => return Ok(response),
                Err(error) if retry < APIFY_MAX_RETRIES => {
                    eprintln!(
                        "Apify API {operation} failed: {error}; retrying ({}/{})",
                        retry + 1,
                        APIFY_MAX_RETRIES
                    );
                    tokio::time::sleep(retry_delay(retry)).await;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify API {operation} failed"));
                }
            }
        }
        unreachable!("the Apify retry loop always returns or fails")
    }

    async fn response_json(response: Response, operation: &str) -> Result<Value> {
        let status = response.status();
        let body = response
            .text()
            .await
            .with_context(|| format!("Failed to read Apify API {operation} response"))?;
        if !status.is_success() {
            bail!(format_apify_error(status, &body, operation));
        }
        serde_json::from_str(&body)
            .with_context(|| format!("Apify API {operation} returned invalid JSON"))
    }

    async fn ensure_success(response: Response, operation: &str) -> Result<()> {
        if response.status().is_success() {
            return Ok(());
        }
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!(format_apify_error(status, &body, operation));
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let response = self
            .send(
                Method::GET,
                self.record_url(&self.key_value_store_id, &self.input_key)?,
                None,
                None,
                "INPUT request",
            )
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Self::response_json(response, "INPUT request")
            .await
            .map(Some)
    }

    pub async fn get_event_budget(&self) -> Result<EventBudget> {
        if let (Some(pricing_info), Some(charged_event_counts)) = (
            env::var("APIFY_ACTOR_PRICING_INFO")
                .ok()
                .filter(|value| !value.is_empty()),
            env::var("APIFY_CHARGED_ACTOR_EVENT_COUNTS")
                .ok()
                .filter(|value| !value.is_empty()),
        ) {
            let pricing_info: Value = serde_json::from_str(&pricing_info)
                .context("APIFY_ACTOR_PRICING_INFO must contain valid JSON")?;
            let charged_event_counts: Value = serde_json::from_str(&charged_event_counts)
                .context("APIFY_CHARGED_ACTOR_EVENT_COUNTS must contain valid JSON")?;
            let max_total_charge_usd = env::var("ACTOR_MAX_TOTAL_CHARGE_USD")
                .ok()
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or(0.0);
            return EventBudget::from_run(&json!({
                "pricingInfo": pricing_info,
                "chargedEventCounts": charged_event_counts,
                "options": {"maxTotalChargeUsd": max_total_charge_usd}
            }));
        }

        let response = self
            .send(
                Method::GET,
                self.resource_url(&["actor-runs", &self.actor_run_id])?,
                None,
                None,
                "run pricing request",
            )
            .await?;
        let run = Self::response_json(response, "run pricing request").await?;
        EventBudget::from_run(&run)
    }

    pub async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let response = self
            .send(
                Method::POST,
                self.resource_url(&["datasets", &self.dataset_id, "items"])?,
                Some(Value::Array(items.to_vec())),
                None,
                "dataset write",
            )
            .await?;
        Self::ensure_success(response, "dataset write").await
    }

    pub async fn push_charged_dataset_items(
        &self,
        event_budget: &mut EventBudget,
        items: &[Value],
    ) -> Result<PushDataResult> {
        if items.is_empty() {
            return Ok(PushDataResult {
                charged_count: 0,
                event_charge_limit_reached: false,
            });
        }

        let item_count = event_budget.limit_dataset_items(REVIEW_RESULT_EVENT, items.len(), true);
        let limited_items = &items[..item_count];
        if !limited_items.is_empty() {
            self.push_dataset_items(limited_items).await?;
        }
        if limited_items.is_empty() {
            return Ok(PushDataResult {
                charged_count: 0,
                event_charge_limit_reached: true,
            });
        }

        let event_charge = event_budget
            .charge(self, REVIEW_RESULT_EVENT, limited_items.len())
            .await?;
        let dataset_item_charge = event_budget
            .charge(self, DEFAULT_DATASET_ITEM_EVENT, limited_items.len())
            .await?;
        Ok(PushDataResult {
            charged_count: event_charge
                .charged_count
                .saturating_add(dataset_item_charge.charged_count),
            event_charge_limit_reached: event_charge.event_charge_limit_reached
                || dataset_item_charge.event_charge_limit_reached,
        })
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let response = self
            .send(
                Method::PUT,
                self.record_url(&self.key_value_store_id, "OUTPUT")?,
                Some(output.clone()),
                None,
                "OUTPUT write",
            )
            .await?;
        Self::ensure_success(response, "OUTPUT write").await
    }

    pub async fn set_status_message(&self, message: &str) -> Result<()> {
        let response = self
            .send(
                Method::PUT,
                self.resource_url(&["actor-runs", &self.actor_run_id])?,
                Some(json!({
                    "runId": self.actor_run_id,
                    "statusMessage": message,
                    "isStatusMessageTerminal": true
                })),
                None,
                "status message update",
            )
            .await?;
        Self::ensure_success(response, "status message update").await
    }

    async fn charge_event(&self, event_name: &str, count: usize) -> Result<()> {
        let count = u64::try_from(count).context("Charge count is too large")?;
        let sequence = CHARGE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let idempotency_key = format!(
            "kununu-{}-{event_name}-{}-{sequence}",
            self.actor_run_id,
            std::process::id()
        );
        let response = self
            .send(
                Method::POST,
                self.resource_url(&["actor-runs", &self.actor_run_id, "charge"])?,
                Some(json!({"eventName": event_name, "count": count})),
                Some(idempotency_key),
                "event charge",
            )
            .await?;
        Self::ensure_success(response, "event charge").await
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn retry_delay(retry: usize) -> Duration {
    APIFY_MIN_RETRY_DELAY
        .checked_mul(2_u32.saturating_pow(retry as u32))
        .unwrap_or(Duration::from_secs(60))
}

fn format_apify_error(status: StatusCode, body: &str, operation: &str) -> String {
    let status_code = status.as_u16();
    let detail = if body.trim().is_empty() {
        format!("HTTP {status_code}")
    } else {
        body.trim().to_owned()
    };
    format!("Apify API error ({status_code}) while trying to {operation}: {detail}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushDataResult {
    pub charged_count: usize,
    pub event_charge_limit_reached: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChargeResult {
    charged_count: usize,
    event_charge_limit_reached: bool,
}

#[derive(Debug, Clone)]
pub struct EventBudget {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    charged_counts: HashMap<String, u64>,
}

impl EventBudget {
    pub(crate) fn from_run(run: &Value) -> Result<Self> {
        let data = run.get("data").unwrap_or(run);
        let pricing_info = data.get("pricingInfo");
        let is_pay_per_event = pricing_info
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        let mut event_prices = HashMap::new();
        if is_pay_per_event {
            let events = pricing_info
                .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
                .and_then(Value::as_object)
                .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
            for (event_name, event) in events {
                if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
                    if !price.is_finite() || price < 0.0 {
                        bail!("Apify run returned invalid price for charged event {event_name}");
                    }
                    event_prices.insert(event_name.clone(), price);
                }
            }
        }
        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|value| *value > 0.0)
            .unwrap_or(f64::INFINITY);
        let charged_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .map(|counts| {
                counts
                    .iter()
                    .map(|(event_name, count)| {
                        count
                            .as_u64()
                            .map(|count| (event_name.clone(), count))
                            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))
                    })
                    .collect::<Result<HashMap<_, _>>>()
            })
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_counts,
        })
    }

    pub fn is_pay_per_event(&self) -> bool {
        self.is_pay_per_event
    }

    pub fn max_event_charge_count(&self, event_name: &str) -> usize {
        if !self.is_pay_per_event {
            return usize::MAX;
        }
        let Some(price) = self.event_prices.get(event_name).copied() else {
            return usize::MAX;
        };
        self.max_charges_by_price(price)
    }

    fn limit_dataset_items(
        &self,
        event_name: &str,
        requested: usize,
        is_default_dataset: bool,
    ) -> usize {
        if !self.is_pay_per_event {
            return requested;
        }
        let item_price = self.event_prices.get(event_name).copied().unwrap_or(0.0)
            + if is_default_dataset {
                self.event_prices
                    .get(DEFAULT_DATASET_ITEM_EVENT)
                    .copied()
                    .unwrap_or(0.0)
            } else {
                0.0
            };
        let max_charged_count = if item_price > 0.0 {
            self.max_charges_by_price(item_price)
        } else {
            usize::MAX
        };
        if max_charged_count >= requested {
            return requested;
        }
        if requested > 0
            && max_charged_count == 0
            && self.total_charged_amount() <= self.max_total_charge_usd
        {
            return 1;
        }
        max_charged_count.min(requested)
    }

    fn max_charges_by_price(&self, price: f64) -> usize {
        if price == 0.0 {
            return usize::MAX;
        }
        let unrounded = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if unrounded.is_infinite() {
            return if unrounded.is_sign_positive() {
                usize::MAX
            } else {
                0
            };
        }
        if !unrounded.is_finite() {
            return 0;
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

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_counts
            .iter()
            .map(|(event_name, count)| {
                self.event_prices.get(event_name).copied().unwrap_or(0.0) * *count as f64
            })
            .sum::<f64>();
        format!("{total:.6}").parse::<f64>().unwrap_or(total)
    }

    async fn charge(
        &mut self,
        client: &ApifyClient,
        event_name: &str,
        requested_count: usize,
    ) -> Result<ChargeResult> {
        if !self.is_pay_per_event {
            return Ok(ChargeResult {
                charged_count: 0,
                event_charge_limit_reached: false,
            });
        }
        let max_event_charge_count = self.max_event_charge_count(event_name);
        let charged_count = if requested_count <= max_event_charge_count {
            requested_count
        } else if self.total_charged_amount() <= self.max_total_charge_usd {
            max_event_charge_count.saturating_add(1)
        } else {
            0
        };
        if charged_count == 0 {
            return Ok(ChargeResult {
                charged_count: 0,
                event_charge_limit_reached: requested_count > 0,
            });
        }

        let charged_count_u64 =
            u64::try_from(charged_count).context("Charged event count is too large")?;
        let current_count = self.charged_counts.get(event_name).copied().unwrap_or(0);
        let new_count = current_count
            .checked_add(charged_count_u64)
            .ok_or_else(|| anyhow!("Charged event count overflowed for {event_name}"))?;
        self.charged_counts.insert(event_name.to_owned(), new_count);

        if !event_name.starts_with("apify-") && self.event_prices.contains_key(event_name) {
            client.charge_event(event_name, charged_count).await?;
        }

        Ok(ChargeResult {
            charged_count,
            event_charge_limit_reached: self.max_event_charge_count(event_name) == 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        APIFY_MAX_RETRIES, ApifyClient, ApifyConfig, EventBudget, PushDataResult, retry_delay,
    };
    use crate::test_support::*;
    use reqwest::StatusCode;
    use serde_json::{Value, json};
    use std::time::Duration;

    fn config(base_url: String) -> ApifyConfig {
        ApifyConfig {
            base_url,
            token: "test-token".into(),
            actor_run_id: "test-run".into(),
            key_value_store_id: "test-store".into(),
            dataset_id: "test-dataset".into(),
            input_key: "INPUT".into(),
        }
    }

    fn ppe_run(max_total_charge_usd: f64, charged_event_counts: Value, prices: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": prices}
                },
                "options": {"maxTotalChargeUsd": max_total_charge_usd},
                "chargedEventCounts": charged_event_counts
            }
        })
    }

    fn review_prices() -> Value {
        json!({
            "review-result": {"eventPriceUsd": 0.25},
            "apify-default-dataset-item": {"eventPriceUsd": 0.0},
            "other-event": {"eventPriceUsd": 0.1}
        })
    }

    #[test]
    fn budget_counts_other_charged_events_and_limits_rows_by_review_price() {
        let budget =
            EventBudget::from_run(&ppe_run(0.6, json!({"other-event": 1}), review_prices()))
                .unwrap();
        assert!(budget.is_pay_per_event());
        assert_eq!(budget.max_event_charge_count("review-result"), 2);
        assert_eq!(budget.limit_dataset_items("review-result", 4, true), 2);
    }

    #[test]
    fn custom_and_default_dataset_event_prices_share_the_run_limit() {
        let mut prices = review_prices();
        prices["apify-default-dataset-item"] = json!({"eventPriceUsd": 0.1});
        let budget =
            EventBudget::from_run(&ppe_run(0.7, json!({"other-event": 1}), prices)).unwrap();
        assert_eq!(budget.max_event_charge_count("review-result"), 2);
        assert_eq!(budget.limit_dataset_items("review-result", 4, true), 1);
    }

    #[test]
    fn charge_manager_preserves_zero_limit_overage_and_non_ppe_behavior() {
        let budget = EventBudget::from_run(&ppe_run(0.0, json!({}), review_prices())).unwrap();
        // Apify SDK treats zero as an unset limit and therefore as unlimited.
        assert_eq!(budget.max_event_charge_count("review-result"), usize::MAX);

        let run = json!({"data":{"pricingInfo":{"pricingModel":"PAY_PER_USAGE"}}});
        let budget = EventBudget::from_run(&run).unwrap();
        assert!(!budget.is_pay_per_event());
        assert_eq!(budget.max_event_charge_count("review-result"), usize::MAX);
        assert_eq!(budget.limit_dataset_items("review-result", 5, true), 5);
    }

    #[test]
    fn retry_policy_matches_apify_client_defaults() {
        assert_eq!(APIFY_MAX_RETRIES, 8);
        assert_eq!(retry_delay(0), Duration::from_millis(500));
        assert_eq!(retry_delay(1), Duration::from_millis(1_000));
        assert_eq!(retry_delay(7), Duration::from_secs(64));
        assert!(super::is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(super::is_retryable_status(
            StatusCode::INTERNAL_SERVER_ERROR
        ));
        assert!(!super::is_retryable_status(StatusCode::BAD_REQUEST));
    }

    #[tokio::test]
    async fn input_fetch_retries_apify_5xx_and_uses_actor_auth() {
        let server = MockServer::start(vec![
            response(503, json!({"error":"temporary"})),
            response(200, json!({"targets":["de/bmwgroup"]})),
        ]);
        let client = ApifyClient::new(config(server.base_url.clone())).unwrap();
        assert_eq!(
            client.get_input().await.unwrap().unwrap()["targets"][0],
            "de/bmwgroup"
        );
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert_eq!(
            header(&requests[0], "Authorization").as_deref(),
            Some("Bearer test-token")
        );
    }

    #[tokio::test]
    async fn charged_dataset_write_saves_rows_before_charging_the_custom_event() {
        let server = MockServer::start(vec![response(200, json!({})), response(201, json!({}))]);
        let client = ApifyClient::new(config(server.base_url.clone())).unwrap();
        let mut budget = EventBudget::from_run(&ppe_run(1.0, json!({}), review_prices())).unwrap();
        let rows = vec![json!({"review_id":"r1"}), json!({"review_id":"r2"})];
        let result = client
            .push_charged_dataset_items(&mut budget, &rows)
            .await
            .unwrap();
        assert_eq!(
            result,
            PushDataResult {
                charged_count: 4,
                event_charge_limit_reached: false,
            }
        );

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/datasets/test-dataset/items"
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[0]).2).unwrap(),
            json!([{"review_id":"r1"},{"review_id":"r2"}])
        );
        assert_eq!(
            header(&requests[0], "Authorization").as_deref(),
            Some("Bearer test-token")
        );
        assert_eq!(
            request_parts(&requests[1]).1,
            "/v2/actor-runs/test-run/charge"
        );
        assert!(header(&requests[1], "Idempotency-Key").is_some());
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[1]).2).unwrap(),
            json!({"eventName":"review-result","count":2})
        );
    }

    #[tokio::test]
    async fn exhausted_custom_event_capacity_skips_the_upstream_page() {
        let budget = EventBudget::from_run(&ppe_run(0.1, json!({}), review_prices())).unwrap();
        assert_eq!(budget.max_event_charge_count("review-result"), 0);
    }
}
