use std::{collections::HashMap, env};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const INPUT_KEY: &str = "INPUT";
const OUTPUT_KEY: &str = "OUTPUT";
pub const URL_RESULT_CHARGE_EVENT: &str = "url-result";

pub struct ApifyConfig {
    api_base_url: Url,
    token: String,
    key_value_store_id: String,
    input_key: String,
    dataset_id: String,
    run_id: String,
}

impl ApifyConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| INPUT_KEY.to_owned()),
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            run_id: required_env("ACTOR_RUN_ID")?,
        })
    }
}

pub struct ApifyClient {
    http: Client,
    config: ApifyConfig,
}

impl ApifyClient {
    pub fn new(config: ApifyConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Could not create Apify API client")?;
        Ok(Self { http, config })
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
            .http
            .get(url)
            .bearer_auth(&self.config.token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        require_apify_success(response, "INPUT request")
            .await?
            .json::<Value>()
            .await
            .context("Apify INPUT record was not valid JSON")
            .map(Some)
    }

    pub async fn get_billing_state(&self) -> Result<BillingState> {
        let run = self.get_run().await?;
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let is_pay_per_event = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(BillingState::default());
        }

        let pricing = RunPricing::from_run(&run)?;
        Ok(BillingState {
            is_pay_per_event: true,
            budget: ChargeBudget::new(pricing.charged_event_counts.clone()),
            pricing: Some(pricing),
        })
    }

    pub async fn push_dataset_item(
        &self,
        item: &Value,
        billing: &mut BillingState,
        idempotency_key: &str,
    ) -> Result<DatasetWriteResult> {
        let success = item.get("success").and_then(Value::as_bool) == Some(true);
        let should_charge = if billing.is_pay_per_event && success {
            let pricing = billing
                .pricing
                .as_ref()
                .ok_or_else(|| anyhow!("PPE run pricing is missing"))?;
            billing
                .budget
                .can_charge(pricing, URL_RESULT_CHARGE_EVENT)?
        } else {
            false
        };
        if billing.is_pay_per_event && success && !should_charge {
            return Ok(DatasetWriteResult::ChargeLimitReached);
        }

        self.write_dataset_item(item).await?;

        if should_charge {
            self.charge_event(URL_RESULT_CHARGE_EVENT, idempotency_key)
                .await?;
            billing.budget.record_charge(URL_RESULT_CHARGE_EVENT)?;
        }

        Ok(DatasetWriteResult::Saved)
    }

    pub async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            OUTPUT_KEY,
        ])?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.config.token)
            .header(header::ACCEPT, "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        require_apify_success(response, "OUTPUT write").await?;
        Ok(())
    }

    pub fn charge_idempotency_key(&self, item_index: usize) -> String {
        format!("{}:url-result:{item_index}", self.config.run_id)
    }

    async fn write_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.token)
            .header(header::ACCEPT, "application/json")
            .json(item)
            .send()
            .await
            .context("Apify dataset write failed")?;
        require_apify_success(response, "dataset write").await?;
        Ok(())
    }

    async fn charge_event(&self, event_name: &str, idempotency_key: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.run_id, "charge"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.token)
            .header(header::ACCEPT, "application/json")
            .header("idempotency-key", idempotency_key)
            .json(&json!({"eventName": event_name, "count": 1}))
            .send()
            .await
            .context("Apify charge request failed")?;
        require_apify_success(response, "charge request").await?;
        Ok(())
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.run_id])?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.config.token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        require_apify_success(response, "run pricing request")
            .await?
            .json::<Value>()
            .await
            .context("Apify run pricing request returned invalid JSON")
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.api_base_url, segments)
    }
}

#[derive(Debug, Default)]
pub struct BillingState {
    is_pay_per_event: bool,
    budget: ChargeBudget,
    pricing: Option<RunPricing>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetWriteResult {
    Saved,
    ChargeLimitReached,
}

#[derive(Debug, Default)]
struct ChargeBudget {
    initial_event_counts: Option<HashMap<String, u64>>,
    locally_charged: HashMap<String, u64>,
}

impl ChargeBudget {
    fn new(initial_event_counts: HashMap<String, u64>) -> Self {
        Self {
            initial_event_counts: Some(initial_event_counts),
            locally_charged: HashMap::new(),
        }
    }

    fn can_charge(&mut self, pricing: &RunPricing, event_name: &str) -> Result<bool> {
        let initial_event_counts = self
            .initial_event_counts
            .get_or_insert_with(|| pricing.charged_event_counts.clone());
        let local_event_count = *self.locally_charged.get(event_name).unwrap_or(&0);
        let initial_event_count = *initial_event_counts.get(event_name).unwrap_or(&0);
        let expected_event_count = initial_event_count
            .checked_add(local_event_count)
            .ok_or_else(|| anyhow!("Charged event count overflowed for {event_name}"))?;

        let mut event_counts = pricing.charged_event_counts.clone();
        for (name, count) in initial_event_counts.iter() {
            let current = event_counts.entry(name.clone()).or_default();
            *current = (*current).max(*count);
        }
        let current_event_count = event_counts.entry(event_name.to_owned()).or_default();
        *current_event_count = (*current_event_count).max(expected_event_count);

        let mut spent = 0.0;
        for (charged_event, count) in event_counts {
            if count == 0 {
                continue;
            }
            let price = pricing
                .event_prices
                .get(&charged_event)
                .ok_or_else(|| anyhow!("Missing price for charged event {charged_event}"))?;
            spent += price * count as f64;
        }
        if !spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        let event_price = pricing
            .event_prices
            .get(event_name)
            .ok_or_else(|| anyhow!("Apify run did not provide the {event_name} event price"))?;
        if *event_price == 0.0 {
            return Ok(true);
        }

        let Some(max_total_charge_usd) = pricing.max_total_charge_usd else {
            return Ok(true);
        };
        let tolerance = f64::EPSILON * max_total_charge_usd.max(1.0);
        Ok(spent + event_price <= max_total_charge_usd + tolerance)
    }

    fn record_charge(&mut self, event_name: &str) -> Result<()> {
        let count = self
            .locally_charged
            .entry(event_name.to_owned())
            .or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| anyhow!("Charged event count overflowed for {event_name}"))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct RunPricing {
    max_total_charge_usd: Option<f64>,
    event_prices: HashMap<String, f64>,
    charged_event_counts: HashMap<String, u64>,
}

impl RunPricing {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        if data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            bail!("Apify run is not configured for pay-per-event pricing");
        }

        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::with_capacity(events.len());
        for (event_name, event) in events {
            if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
                if !price.is_finite() || price < 0.0 {
                    bail!("Invalid price for charged event {event_name}");
                }
                event_prices.insert(event_name.to_owned(), price);
            }
        }

        let max_total_charge_usd = match data.pointer("/options/maxTotalChargeUsd") {
            Some(value) if !value.is_null() => Some(
                value
                    .as_f64()
                    .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?,
            ),
            _ => env::var("ACTOR_MAX_TOTAL_CHARGE_USD")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .map(|value| {
                    value
                        .parse::<f64>()
                        .context("ACTOR_MAX_TOTAL_CHARGE_USD is not a number")
                })
                .transpose()?,
        };
        if max_total_charge_usd.is_some_and(|limit| !limit.is_finite() || limit < 0.0) {
            bail!("Apify run returned an invalid spending limit");
        }

        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?
            .iter()
            .map(|(event_name, count)| {
                count
                    .as_u64()
                    .map(|count| (event_name.to_owned(), count))
                    .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        Ok(Self {
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
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
    let raw_url = env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_owned());
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

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    bail!(
        "Apify {operation} failed with {} {reason}{detail}",
        status.as_u16()
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::{json, Value};
    use url::Url;

    use crate::test_support::{start_mock_server, MockResponse};

    use super::{
        ApifyClient, ApifyConfig, BillingState, ChargeBudget, DatasetWriteResult, RunPricing,
        URL_RESULT_CHARGE_EVENT,
    };

    fn pricing(max_total_charge_usd: f64, counts: Value) -> RunPricing {
        RunPricing {
            max_total_charge_usd: Some(max_total_charge_usd),
            event_prices: [
                ("apify-actor-start".to_owned(), 0.0001),
                (URL_RESULT_CHARGE_EVENT.to_owned(), 0.0002),
            ]
            .into_iter()
            .collect(),
            charged_event_counts: serde_json::from_value(counts).unwrap(),
        }
    }

    fn ppe_run() -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-actor-start": {"eventPriceUsd": 0.0001},
                            "url-result": {"eventPriceUsd": 0.0002}
                        }
                    }
                },
                "options": {"maxTotalChargeUsd": 0.0003},
                "chargedEventCounts": {"apify-actor-start": 1}
            }
        })
    }

    fn apify_client(base_url: Url) -> ApifyClient {
        ApifyClient::new(ApifyConfig {
            api_base_url: base_url,
            token: "test-token".to_owned(),
            key_value_store_id: "test-kvs".to_owned(),
            input_key: "INPUT".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            run_id: "test-run".to_owned(),
        })
        .unwrap()
    }

    #[test]
    fn ppe_budget_counts_startup_charges_and_limits_success_results() {
        let run_pricing = pricing(0.0004, json!({"apify-actor-start": 1}));
        let mut budget = ChargeBudget::new(run_pricing.charged_event_counts.clone());

        assert!(budget
            .can_charge(&run_pricing, URL_RESULT_CHARGE_EVENT)
            .unwrap());
        budget.record_charge(URL_RESULT_CHARGE_EVENT).unwrap();
        assert!(!budget
            .can_charge(&run_pricing, URL_RESULT_CHARGE_EVENT)
            .unwrap());
    }

    #[test]
    fn ppe_budget_uses_reported_counts_when_the_run_api_catches_up() {
        let initial = pricing(0.0007, json!({"apify-actor-start": 1}));
        let mut budget = ChargeBudget::new(initial.charged_event_counts.clone());
        budget.record_charge(URL_RESULT_CHARGE_EVENT).unwrap();
        let updated = pricing(0.0007, json!({"apify-actor-start": 1, "url-result": 1}));

        assert!(budget
            .can_charge(&updated, URL_RESULT_CHARGE_EVENT)
            .unwrap());
    }

    #[test]
    fn ppe_budget_allows_charges_when_the_run_has_no_total_cap() {
        let run_pricing = RunPricing {
            max_total_charge_usd: None,
            event_prices: [(URL_RESULT_CHARGE_EVENT.to_owned(), 0.0002)]
                .into_iter()
                .collect(),
            charged_event_counts: HashMap::new(),
        };
        let mut budget = ChargeBudget::new(HashMap::new());

        assert!(budget
            .can_charge(&run_pricing, URL_RESULT_CHARGE_EVENT)
            .unwrap());
    }

    #[tokio::test]
    async fn saves_success_before_charging_and_stops_before_an_unaffordable_result() {
        let run_body = serde_json::to_string(&ppe_run()).unwrap();
        let server = start_mock_server(vec![
            MockResponse::json(200, &run_body),
            MockResponse::json(200, &run_body),
            MockResponse::text(201, ""),
            MockResponse::text(201, ""),
            MockResponse::json(200, &run_body),
        ])
        .await;
        let client = apify_client(server.base_url());
        let mut billing = client.get_billing_state().await.unwrap();
        let item = json!({"success": true, "input_url": "https://example.com"});

        assert_eq!(
            client
                .push_dataset_item(&item, &mut billing, "test-run:url-result:0")
                .await
                .unwrap(),
            DatasetWriteResult::Saved
        );
        assert_eq!(
            client
                .push_dataset_item(&item, &mut billing, "test-run:url-result:1")
                .await
                .unwrap(),
            DatasetWriteResult::ChargeLimitReached
        );

        let requests = server.requests().await;
        assert_eq!(requests.len(), 3);
        assert!(requests[1].starts_with("POST /api/v2/datasets/test-dataset/items HTTP/1.1"));
        assert!(requests[2].starts_with("POST /api/v2/actor-runs/test-run/charge HTTP/1.1"));
        assert!(requests[2]
            .to_ascii_lowercase()
            .contains("idempotency-key: test-run:url-result:0"));
        assert!(requests[2].contains("\"eventName\":\"url-result\""));
    }

    #[test]
    fn non_ppe_billing_state_does_not_charge() {
        assert!(!BillingState::default().is_pay_per_event);
    }
}
