use std::{collections::HashMap, env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use rand::Rng;
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Value};
use url::Url;

use crate::{
    config::{endpoint_url, Config},
    runner::{LocationWriter, SaveResult},
};

const APIFY_MAX_RETRIES: usize = 2;
const APIFY_RETRY_BASE: Duration = Duration::from_secs(1);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CHARGE_EVENT: &str = "location-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const OUTPUT_KEY: &str = "OUTPUT";

pub struct ApifyClient {
    http: Client,
    config: Config,
    budget: ChargeBudget,
    charge_sequence: u64,
}

#[derive(Debug, Clone)]
struct ChargeBudget {
    is_pay_per_event: bool,
    event_prices: HashMap<String, f64>,
    charged_event_counts: HashMap<String, u64>,
    max_total_charge_usd: f64,
}

impl Default for ChargeBudget {
    fn default() -> Self {
        Self {
            is_pay_per_event: false,
            event_prices: HashMap::new(),
            charged_event_counts: HashMap::new(),
            max_total_charge_usd: f64::INFINITY,
        }
    }
}

impl ChargeBudget {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_info = data.get("pricingInfo").unwrap_or(&Value::Null);
        let charged_counts = match data.get("chargedEventCounts") {
            Some(Value::Object(counts)) => Value::Object(counts.clone()),
            Some(Value::Null) | None => Value::Object(Default::default()),
            Some(_) => bail!("Apify run returned invalid charged event counts"),
        };
        let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
            Some(value) if !value.is_null() => value
                .as_f64()
                .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?,
            _ => f64::INFINITY,
        };
        Self::from_pricing(pricing_info, &charged_counts, max_charge)
    }

    fn from_pricing(pricing_info: &Value, charged_counts: &Value, max_charge: f64) -> Result<Self> {
        let is_pay_per_event =
            pricing_info.get("pricingModel").and_then(Value::as_str) == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self::default());
        }
        if !max_charge.is_finite() && max_charge != f64::INFINITY {
            bail!("Apify run returned an invalid spending limit");
        }
        if max_charge < 0.0 {
            bail!("Apify run returned an invalid spending limit");
        }

        let events = pricing_info
            .pointer("/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (event_name, event) in events {
            let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) else {
                continue;
            };
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned an invalid price for event {event_name}");
            }
            event_prices.insert(event_name.clone(), price);
        }

        let input_counts = charged_counts
            .as_object()
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut parsed_counts = HashMap::new();
        for (event_name, count) in input_counts {
            let count = count.as_u64().ok_or_else(|| {
                anyhow!("Apify run returned an invalid charged event count for {event_name}")
            })?;
            parsed_counts.insert(event_name.clone(), count);
        }

        Ok(Self {
            is_pay_per_event,
            event_prices,
            charged_event_counts: parsed_counts,
            max_total_charge_usd: max_charge,
        })
    }

    fn event_price(&self, event_name: &str) -> f64 {
        self.event_prices.get(event_name).copied().unwrap_or(0.0)
    }

    fn total_charged_amount(&self) -> f64 {
        self.charged_event_counts
            .iter()
            .map(|(name, count)| self.event_price(name) * (*count as f64))
            .sum()
    }

    fn max_charges_for_price(&self, price: f64) -> usize {
        if price == 0.0 || self.max_total_charge_usd == f64::INFINITY {
            return usize::MAX;
        }
        let remaining = self.max_total_charge_usd - self.total_charged_amount();
        let rounded_to_four_places = ((remaining / price) * 10_000.0).round() / 10_000.0;
        rounded_to_four_places
            .floor()
            .max(0.0)
            .min(usize::MAX as f64) as usize
    }

    fn item_price(&self) -> f64 {
        self.event_price(CHARGE_EVENT) + self.event_price(DEFAULT_DATASET_ITEM_EVENT)
    }

    fn affordable_rows(&self, requested: usize) -> usize {
        if !self.is_pay_per_event || requested == 0 {
            return requested;
        }
        let max_rows = self.max_charges_for_price(self.item_price());
        if max_rows >= requested {
            return requested;
        }
        // The JavaScript SDK writes one item just over the boundary so Apify can
        // terminate the run. Keep the same single-result boundary behavior.
        if max_rows == 0 && self.total_charged_amount() <= self.max_total_charge_usd {
            return 1;
        }
        max_rows
    }

    fn record_rows(&mut self, count: usize) {
        if !self.is_pay_per_event || count == 0 {
            return;
        }
        for event_name in [CHARGE_EVENT, DEFAULT_DATASET_ITEM_EVENT] {
            if self.event_prices.contains_key(event_name) {
                *self
                    .charged_event_counts
                    .entry(event_name.to_owned())
                    .or_default() += count as u64;
            }
        }
    }

    fn limit_reached(&self) -> bool {
        self.is_pay_per_event
            && [CHARGE_EVENT, DEFAULT_DATASET_ITEM_EVENT]
                .into_iter()
                .any(|event_name| {
                    let price = self.event_price(event_name);
                    price > 0.0 && self.max_charges_for_price(price) == 0
                })
    }
}

impl ApifyClient {
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self {
            http: Client::builder().timeout(APIFY_REQUEST_TIMEOUT).build()?,
            config,
            budget: ChargeBudget::default(),
            charge_sequence: 0,
        })
    }

    pub async fn initialize_charging(&mut self) -> Result<()> {
        let pricing_info = env::var("APIFY_ACTOR_PRICING_INFO")
            .ok()
            .filter(|value| !value.is_empty());
        let charged_counts = env::var("APIFY_CHARGED_ACTOR_EVENT_COUNTS")
            .ok()
            .filter(|value| !value.is_empty());
        if let (Some(pricing_info), Some(charged_counts)) = (pricing_info, charged_counts) {
            let pricing_info: Value = serde_json::from_str(&pricing_info)
                .context("APIFY_ACTOR_PRICING_INFO was not valid JSON")?;
            let charged_counts: Value = serde_json::from_str(&charged_counts)
                .context("APIFY_CHARGED_ACTOR_EVENT_COUNTS was not valid JSON")?;
            let max_charge = match env::var("ACTOR_MAX_TOTAL_CHARGE_USD") {
                Ok(value) => value
                    .parse::<f64>()
                    .context("ACTOR_MAX_TOTAL_CHARGE_USD was not a number")?,
                Err(_) => f64::INFINITY,
            };
            self.budget = ChargeBudget::from_pricing(
                &pricing_info,
                &charged_counts,
                if max_charge == 0.0 {
                    f64::INFINITY
                } else {
                    max_charge
                },
            )?;
            return Ok(());
        }

        let run = self.get_run().await?;
        self.budget = ChargeBudget::from_run(&run)?;
        Ok(())
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .send_with_retry(
                || self.authorized(self.http.get(url.clone())),
                "INPUT request",
                true,
            )
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        response_json(response, "INPUT request").await.map(Some)
    }

    pub async fn write_output(&self, locations: &[Value]) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            OUTPUT_KEY,
        ])?;
        let response = self
            .send_with_retry(
                || self.authorized(self.http.put(url.clone()).json(locations)),
                "OUTPUT record publication",
                true,
            )
            .await?;
        require_apify_success(response, "OUTPUT record publication").await
    }

    pub async fn set_status_message(&self, status_message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let payload = json!({ "statusMessage": status_message });
        let response = self
            .send_with_retry(
                || self.authorized(self.http.patch(url.clone()).json(&payload)),
                "run status update",
                true,
            )
            .await?;
        require_apify_success(response, "run status update").await
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .send_with_retry(
                || self.authorized(self.http.get(url.clone())),
                "run pricing request",
                true,
            )
            .await?;
        response_json(response, "run pricing request").await
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
    }

    fn authorized(&self, request: RequestBuilder) -> RequestBuilder {
        request
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
    }

    async fn send_with_retry<F>(
        &self,
        request: F,
        operation: &str,
        retryable_method: bool,
    ) -> Result<Response>
    where
        F: Fn() -> RequestBuilder,
    {
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let response = match request().send().await {
                Ok(response) => response,
                Err(error)
                    if retryable_method
                        && retry_count < APIFY_MAX_RETRIES
                        && is_retryable_transport(&error) =>
                {
                    tokio::time::sleep(retry_delay(retry_count)).await;
                    continue;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify {operation} failed"))
                }
            };

            if retryable_method && retry_count < APIFY_MAX_RETRIES {
                if let Some(delay) = response_retry_delay(&response, retry_count) {
                    drop(response);
                    tokio::time::sleep(delay).await;
                    continue;
                }
            }
            return Ok(response);
        }
        unreachable!("the Apify retry loop returns on its final attempt")
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .send_with_retry(
                || self.authorized(self.http.post(url.clone()).json(items)),
                "dataset item publication",
                true,
            )
            .await?;
        require_apify_success(response, "dataset item publication").await
    }

    async fn charge_location_event(&mut self, count: usize) -> Result<()> {
        if count == 0 || !self.budget.event_prices.contains_key(CHARGE_EVENT) {
            return Ok(());
        }
        self.charge_sequence += 1;
        let nonce: u64 = rand::thread_rng().gen();
        let idempotency_key = format!(
            "{}-{}-{}-{nonce}",
            self.config.actor_run_id, CHARGE_EVENT, self.charge_sequence
        );
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        let payload = json!({ "eventName": CHARGE_EVENT, "count": count });
        let response = self
            .send_with_retry(
                || {
                    self.authorized(self.http.post(url.clone()).json(&payload))
                        .header("idempotency-key", &idempotency_key)
                },
                "location-result charge",
                true,
            )
            .await?;
        require_apify_success(response, "location-result charge").await
    }
}

impl LocationWriter for ApifyClient {
    async fn save_locations(&mut self, items: &[Value]) -> Result<SaveResult> {
        if items.is_empty() {
            return Ok(SaveResult {
                saved_count: 0,
                limit_reached: false,
            });
        }

        let saved_count = if self.budget.is_pay_per_event {
            self.budget.affordable_rows(items.len())
        } else {
            items.len()
        };
        if saved_count == 0 {
            return Ok(SaveResult {
                saved_count: 0,
                limit_reached: true,
            });
        }

        self.push_dataset_items(&items[..saved_count]).await?;
        if self.budget.is_pay_per_event {
            self.charge_location_event(saved_count).await?;
            self.budget.record_rows(saved_count);
        }

        Ok(SaveResult {
            saved_count,
            limit_reached: self.budget.limit_reached() || saved_count < items.len(),
        })
    }
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("Failed to read Apify {operation} response"))?;
    if !status.is_success() {
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!("Apify {operation} failed ({}): {detail}", status.as_u16());
    }
    serde_json::from_str(&body).with_context(|| format!("Apify {operation} returned invalid JSON"))
}

async fn require_apify_success(response: Response, operation: &str) -> Result<()> {
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!(
        "Apify {operation} failed ({}): {}",
        status.as_u16(),
        body.trim()
    );
}

fn is_retryable_transport(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

fn retry_delay(retry_count: usize) -> Duration {
    APIFY_RETRY_BASE.saturating_mul((retry_count + 1) as u32)
}

fn response_retry_delay(response: &Response, retry_count: usize) -> Option<Duration> {
    let status = response.status();
    let retryable = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
    if !retryable {
        return None;
    }
    response
        .headers()
        .get(header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .or_else(|| Some(retry_delay(retry_count)))
}

#[cfg(test)]
fn pricing_budget_from_env_values(
    pricing_info: &str,
    charged_counts: &str,
    max_charge: f64,
) -> Result<ChargeBudget> {
    let pricing_info: Value = serde_json::from_str(pricing_info)
        .context("APIFY_ACTOR_PRICING_INFO was not valid JSON")?;
    let charged_counts: Value = serde_json::from_str(charged_counts)
        .context("APIFY_CHARGED_ACTOR_EVENT_COUNTS was not valid JSON")?;
    ChargeBudget::from_pricing(&pricing_info, &charged_counts, max_charge)
}

#[cfg(test)]
fn run_pricing_budget(run: &Value) -> Result<ChargeBudget> {
    ChargeBudget::from_run(run)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use serde_json::{json, Value};
    use url::Url;

    use crate::{
        config::Config,
        runner::{LocationWriter, SaveResult},
        test_utils::{MockRequest, MockResponse, MockServer},
    };

    use super::{
        pricing_budget_from_env_values, run_pricing_budget, ApifyClient, ChargeBudget, CHARGE_EVENT,
    };

    fn config(base: &str) -> Config {
        Config {
            apify_api_base: Url::parse(base).unwrap(),
            apify_token: "test-token".into(),
            actor_run_id: "run-1".into(),
            key_value_store_id: "kv-1".into(),
            dataset_id: "dataset-1".into(),
            input_key: "INPUT".into(),
            scrappa_api_base: Url::parse("http://127.0.0.1/api/").unwrap(),
            scrappa_api_key: "test-key".into(),
        }
    }

    fn ppe_run(max_charge: f64, counts: Value) -> Value {
        json!({ "data": {
            "pricingInfo": { "pricingModel": "PAY_PER_EVENT", "pricingPerEvent": { "actorChargeEvents": {
                "location-result": { "eventPriceUsd": 0.00025 },
                "apify-default-dataset-item": { "eventPriceUsd": 0.0 }
            } } },
            "chargedEventCounts": counts,
            "options": { "maxTotalChargeUsd": max_charge }
        } })
    }

    #[test]
    fn parses_apify_run_and_environment_pricing_shapes() {
        let run_budget =
            run_pricing_budget(&ppe_run(0.0005, json!({ "location-result": 1 }))).unwrap();
        assert!(run_budget.is_pay_per_event);
        assert_eq!(run_budget.charged_event_counts.get(CHARGE_EVENT), Some(&1));
        assert_eq!(run_budget.affordable_rows(10), 1);

        let env_budget = pricing_budget_from_env_values(
            r#"{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"location-result":{"eventPriceUsd":0.00025}}}}"#,
            r#"{"location-result":1}"#,
            0.0005,
        ).unwrap();
        assert_eq!(env_budget.affordable_rows(10), 1);
    }

    #[test]
    fn allows_all_items_outside_ppe_and_accounts_for_other_charged_events() {
        let free = ChargeBudget::from_run(&json!({ "data": {
            "pricingInfo": { "pricingModel": "FREE" },
            "chargedEventCounts": { "location-result": 999 },
            "options": { "maxTotalChargeUsd": 0.00025 }
        } }))
        .unwrap();
        assert!(!free.is_pay_per_event);
        assert_eq!(free.affordable_rows(5), 5);

        let budget = run_pricing_budget(&json!({ "data": {
            "pricingInfo": { "pricingModel": "PAY_PER_EVENT", "pricingPerEvent": { "actorChargeEvents": {
                "location-result": { "eventPriceUsd": 0.00025 },
                "other-event": { "eventPriceUsd": 0.00025 }
            } } },
            "chargedEventCounts": { "other-event": 1 },
            "options": { "maxTotalChargeUsd": 0.0005 }
        } })).unwrap();
        assert_eq!(budget.affordable_rows(10), 1);
    }

    #[tokio::test]
    async fn reads_input_and_writes_dataset_then_custom_charge_and_output_to_apify() {
        let calls = Arc::new(Mutex::new(Vec::<MockRequest>::new()));
        let handler_calls = calls.clone();
        let server = MockServer::start(move |request| {
            handler_calls.lock().unwrap().push(request.clone());
            if request.path == "/v2/key-value-stores/kv-1/records/INPUT" {
                return MockResponse::json(200, json!({ "queries": ["Berlin"] }));
            }
            MockResponse::json(201, json!({}))
        });
        let mut client = ApifyClient::new(config(&server.base_url(""))).unwrap();
        client.budget = ChargeBudget::from_run(&ppe_run(0.0005, json!({}))).unwrap();

        let input = client.get_input().await.unwrap().unwrap();
        assert_eq!(input, json!({ "queries": ["Berlin"] }));
        let rows = vec![json!({ "geocode": "1", "name": "Berlin", "type": "city" })];
        let saved = client.save_locations(&rows).await.unwrap();
        assert_eq!(
            saved,
            SaveResult {
                saved_count: 1,
                limit_reached: false
            }
        );
        client.write_output(&rows).await.unwrap();

        let calls = calls.lock().unwrap();
        assert_eq!(calls[0].header("authorization"), Some("Bearer test-token"));
        assert_eq!(calls[0].path, "/v2/key-value-stores/kv-1/records/INPUT");
        let dataset_position = calls
            .iter()
            .position(|request| request.path == "/v2/datasets/dataset-1/items")
            .unwrap();
        let charge_position = calls
            .iter()
            .position(|request| request.path == "/v2/actor-runs/run-1/charge")
            .unwrap();
        let output_position = calls
            .iter()
            .position(|request| request.path == "/v2/key-value-stores/kv-1/records/OUTPUT")
            .unwrap();
        assert!(dataset_position < charge_position && charge_position < output_position);
        assert!(calls[charge_position]
            .header("idempotency-key")
            .unwrap()
            .starts_with("run-1-location-result-1-"));
        assert_eq!(
            calls[charge_position].json_body(),
            json!({ "eventName": "location-result", "count": 1 })
        );
        assert_eq!(calls[dataset_position].json_body(), json!(rows));
        assert_eq!(calls[output_position].json_body(), json!(rows));
    }

    #[tokio::test]
    async fn limits_dataset_items_and_charges_to_the_available_budget() {
        let calls = Arc::new(Mutex::new(Vec::<MockRequest>::new()));
        let handler_calls = calls.clone();
        let server = MockServer::start(move |request| {
            handler_calls.lock().unwrap().push(request);
            MockResponse::json(201, json!({}))
        });
        let mut client = ApifyClient::new(config(&server.base_url(""))).unwrap();
        client.budget = ChargeBudget::from_run(&ppe_run(0.0005, json!({}))).unwrap();
        let rows = vec![
            json!({ "geocode": "1" }),
            json!({ "geocode": "2" }),
            json!({ "geocode": "3" }),
        ];

        let saved = client.save_locations(&rows).await.unwrap();
        assert_eq!(
            saved,
            SaveResult {
                saved_count: 2,
                limit_reached: true
            }
        );
        let calls = calls.lock().unwrap();
        let dataset = calls
            .iter()
            .find(|request| request.path == "/v2/datasets/dataset-1/items")
            .unwrap();
        let charge = calls
            .iter()
            .find(|request| request.path == "/v2/actor-runs/run-1/charge")
            .unwrap();
        assert_eq!(dataset.json_body(), json!([rows[0], rows[1]]));
        assert_eq!(
            charge.json_body(),
            json!({ "eventName": "location-result", "count": 2 })
        );
    }

    #[tokio::test]
    async fn preserves_sdk_boundary_charge_and_stops_after_the_over_limit_result() {
        let calls = Arc::new(Mutex::new(Vec::<MockRequest>::new()));
        let handler_calls = calls.clone();
        let server = MockServer::start(move |request| {
            handler_calls.lock().unwrap().push(request);
            MockResponse::json(201, json!({}))
        });
        let mut client = ApifyClient::new(config(&server.base_url(""))).unwrap();
        client.budget = ChargeBudget::from_pricing(
            &json!({ "pricingModel": "PAY_PER_EVENT", "pricingPerEvent": { "actorChargeEvents": {
                "location-result": { "eventPriceUsd": 0.00025 },
                "apify-default-dataset-item": { "eventPriceUsd": 0.00025 }
            } } }),
            &json!({}),
            0.0008,
        )
        .unwrap();

        let first = client
            .save_locations(&[json!({ "geocode": "1" })])
            .await
            .unwrap();
        let second = client
            .save_locations(&[json!({ "geocode": "2" })])
            .await
            .unwrap();
        assert_eq!(
            first,
            SaveResult {
                saved_count: 1,
                limit_reached: false
            }
        );
        assert_eq!(
            second,
            SaveResult {
                saved_count: 1,
                limit_reached: true
            }
        );
        let calls = calls.lock().unwrap();
        assert_eq!(
            calls
                .iter()
                .filter(|request| request.path == "/v2/datasets/dataset-1/items")
                .count(),
            2
        );
        assert_eq!(
            calls
                .iter()
                .filter(|request| request.path == "/v2/actor-runs/run-1/charge")
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn free_runs_write_results_without_event_charges() {
        let paths = Arc::new(Mutex::new(Vec::new()));
        let handler_paths = paths.clone();
        let server = MockServer::start(move |request| {
            handler_paths.lock().unwrap().push(request.path);
            MockResponse::json(201, json!({}))
        });
        let mut client = ApifyClient::new(config(&server.base_url(""))).unwrap();
        client.budget = ChargeBudget::default();
        let rows = vec![json!({ "geocode": "1" }), json!({ "geocode": "2" })];

        let saved = client.save_locations(&rows).await.unwrap();
        assert_eq!(
            saved,
            SaveResult {
                saved_count: 2,
                limit_reached: false
            }
        );
        assert_eq!(*paths.lock().unwrap(), vec!["/v2/datasets/dataset-1/items"]);
    }

    #[tokio::test]
    async fn retries_transient_dataset_write_failures() {
        let calls = Arc::new(Mutex::new(Vec::<MockRequest>::new()));
        let handler_calls = calls.clone();
        let server = MockServer::start(move |request| {
            let mut calls = handler_calls.lock().unwrap();
            calls.push(request);
            if calls.len() == 1 {
                MockResponse::json(503, json!({ "message": "Unavailable" }))
            } else {
                MockResponse::json(201, json!({}))
            }
        });
        let mut client = ApifyClient::new(config(&server.base_url(""))).unwrap();
        client.budget = ChargeBudget::default();

        let saved = client
            .save_locations(&[json!({ "geocode": "1" })])
            .await
            .unwrap();
        assert_eq!(saved.saved_count, 1);
        assert_eq!(calls.lock().unwrap().len(), 2);
    }
}
