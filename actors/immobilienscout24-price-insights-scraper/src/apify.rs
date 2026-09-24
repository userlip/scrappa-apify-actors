use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Method, Response, StatusCode};
use serde_json::{json, Value};
use std::{collections::HashMap, env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const PRICE_INSIGHT_RESULT_EVENT: &str = "price-insight-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    token: String,
}

impl ApifyClient {
    pub fn from_env() -> Result<Self> {
        let base_url =
            env::var("APIFY_API_PUBLIC_BASE_URL").unwrap_or_else(|_| APIFY_API_BASE_URL.to_owned());
        Self::new(
            &base_url,
            required_env("APIFY_TOKEN")?,
            required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            required_env("ACTOR_DEFAULT_DATASET_ID")?,
            required_env("ACTOR_RUN_ID")?,
            env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
        )
    }

    pub fn new(
        base_url: &str,
        token: String,
        default_key_value_store_id: String,
        default_dataset_id: String,
        actor_run_id: String,
        input_key: String,
    ) -> Result<Self> {
        let base_url = Url::parse(base_url)
            .context("APIFY_API_PUBLIC_BASE_URL must be a valid absolute URL")?;
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("Could not create Apify HTTP client")?;

        Ok(Self {
            client,
            base_url,
            default_key_value_store_id,
            default_dataset_id,
            actor_run_id,
            input_key,
            token,
        })
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint_url(&[
            "key-value-stores",
            &self.default_key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        Ok(Some(response_json(response, "Apify INPUT request").await?))
    }

    pub async fn get_pricing_info(&self) -> Result<ActorPricing> {
        let url = self.endpoint_url(&["actor-runs", &self.actor_run_id])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = response_json(response, "Apify run pricing request").await?;
        ActorPricing::from_run(&run)
    }

    pub async fn push_dataset_item(
        &self,
        item: &Value,
        request_index: usize,
        pricing: &mut ActorPricing,
    ) -> Result<PushDataResult> {
        if pricing.is_pay_per_event && !pricing.can_write_result() {
            return Ok(PushDataResult {
                charged_count: 0,
                event_charge_limit_reached: true,
            });
        }

        let url = self.endpoint_url(&["datasets", &self.default_dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(&[item])
            .send()
            .await
            .context("Apify dataset write failed")?;
        successful_response(response, "Apify dataset write").await?;

        if !pricing.is_pay_per_event {
            return Ok(PushDataResult {
                charged_count: 0,
                event_charge_limit_reached: false,
            });
        }

        self.charge_result_event(request_index).await?;
        pricing.record_result_charge();

        Ok(PushDataResult {
            charged_count: 1,
            event_charge_limit_reached: pricing.event_limit_reached(),
        })
    }

    pub async fn set_terminal_status(&self, status_message: &str) -> Result<()> {
        let url = self.endpoint_url(&["actor-runs", &self.actor_run_id])?;
        let response = self
            .request(Method::PUT, url)
            .json(&json!({
                "runId": self.actor_run_id,
                "statusMessage": status_message,
                "isStatusMessageTerminal": true,
            }))
            .send()
            .await
            .context("Apify run status update failed")?;
        successful_response(response, "Apify run status update").await?;
        Ok(())
    }

    async fn charge_result_event(&self, request_index: usize) -> Result<()> {
        let url = self.endpoint_url(&["actor-runs", &self.actor_run_id, "charge"])?;
        let idempotency_key = format!(
            "{}-{PRICE_INSIGHT_RESULT_EVENT}-{request_index}",
            self.actor_run_id
        );
        let response = self
            .request(Method::POST, url)
            .header("idempotency-key", idempotency_key)
            .json(&json!({
                "eventName": PRICE_INSIGHT_RESULT_EVENT,
                "count": 1,
            }))
            .send()
            .await
            .context("Apify price-insight-result charge request failed")?;
        successful_response(response, "Apify price-insight-result charge request").await?;
        Ok(())
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    fn endpoint_url(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(segments.iter().copied()));
        Ok(url)
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

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let response = successful_response(response, operation).await?;
    response
        .json()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PushDataResult {
    pub charged_count: usize,
    pub event_charge_limit_reached: bool,
}

pub struct ActorPricing {
    is_pay_per_event: bool,
    budget: Option<ChargeBudget>,
}

impl ActorPricing {
    fn from_run(run_response: &Value) -> Result<Self> {
        let run = run_response.get("data").unwrap_or(run_response);
        let pricing_info = run
            .get("pricingInfo")
            .context("Apify run pricing information is missing")?;
        let is_pay_per_event =
            pricing_info.get("pricingModel").and_then(Value::as_str) == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self {
                is_pay_per_event: false,
                budget: None,
            });
        }

        let events = pricing_info
            .pointer("/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .context("Apify run did not provide event prices")?;
        let mut event_prices = HashMap::new();
        for (event_name, event) in events {
            if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
                if !price.is_finite() || price < 0.0 {
                    bail!("Apify run returned an invalid price for event {event_name}");
                }
                event_prices.insert(event_name.clone(), price);
            }
        }
        if !event_prices.contains_key(PRICE_INSIGHT_RESULT_EVENT) {
            bail!("Apify run did not provide the price-insight-result event price");
        }

        let max_total_charge_usd = match run.pointer("/options/maxTotalChargeUsd") {
            None | Some(Value::Null) => f64::INFINITY,
            Some(value) => {
                let limit = value
                    .as_f64()
                    .filter(|limit| limit.is_finite() && *limit >= 0.0)
                    .context("Apify run returned an invalid spending limit")?;
                if limit == 0.0 {
                    f64::INFINITY
                } else {
                    limit
                }
            }
        };
        let charged_counts = run
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .context("Apify run did not provide charged event counts")?;
        let mut counts = HashMap::new();
        let mut total_charged_usd = 0.0;
        for (event_name, count) in charged_counts {
            let count = count.as_u64().with_context(|| {
                format!("Apify run returned an invalid charged event count for {event_name}")
            })?;
            counts.insert(event_name.clone(), count);
            if count > 0 {
                total_charged_usd +=
                    event_prices.get(event_name).copied().unwrap_or(0.0) * count as f64;
            }
        }
        if !total_charged_usd.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        Ok(Self {
            is_pay_per_event: true,
            budget: Some(ChargeBudget {
                event_prices,
                counts,
                max_total_charge_usd,
                total_charged_usd,
            }),
        })
    }

    pub fn is_pay_per_event(&self) -> bool {
        self.is_pay_per_event
    }

    fn can_write_result(&self) -> bool {
        self.budget
            .as_ref()
            .is_some_and(|budget| budget.result_capacity() > 0)
    }

    fn record_result_charge(&mut self) {
        if let Some(budget) = &mut self.budget {
            budget.record_result_charge();
        }
    }

    fn event_limit_reached(&self) -> bool {
        self.budget
            .as_ref()
            .is_some_and(ChargeBudget::result_event_limit_reached)
    }
}

struct ChargeBudget {
    event_prices: HashMap<String, f64>,
    counts: HashMap<String, u64>,
    max_total_charge_usd: f64,
    total_charged_usd: f64,
}

impl ChargeBudget {
    fn result_capacity(&self) -> usize {
        let result_price = self.event_price(PRICE_INSIGHT_RESULT_EVENT);
        let dataset_price = self.event_price(DEFAULT_DATASET_ITEM_EVENT);
        self.count_within_limit(result_price + dataset_price)
    }

    fn result_event_limit_reached(&self) -> bool {
        self.event_limit_reached(PRICE_INSIGHT_RESULT_EVENT)
            || self.event_limit_reached(DEFAULT_DATASET_ITEM_EVENT)
    }

    fn event_limit_reached(&self, event_name: &str) -> bool {
        let price = self.event_price(event_name);
        price > 0.0 && self.count_within_limit(price) == 0
    }

    fn event_price(&self, event_name: &str) -> f64 {
        self.event_prices.get(event_name).copied().unwrap_or(0.0)
    }

    fn count_within_limit(&self, price: f64) -> usize {
        if price <= 0.0 || !self.max_total_charge_usd.is_finite() {
            return usize::MAX;
        }

        let remaining = self.max_total_charge_usd - self.total_charged_usd;
        if remaining <= 0.0 {
            return 0;
        }

        let count = ((remaining / price) * 10_000.0).round() / 10_000.0;
        if count >= usize::MAX as f64 {
            usize::MAX
        } else {
            count.floor() as usize
        }
    }

    fn record_result_charge(&mut self) {
        for event_name in [PRICE_INSIGHT_RESULT_EVENT, DEFAULT_DATASET_ITEM_EVENT] {
            let price = self.event_price(event_name);
            if price > 0.0 || self.event_prices.contains_key(event_name) {
                *self.counts.entry(event_name.to_owned()).or_default() += 1;
                self.total_charged_usd += price;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(prices: Value, counts: Value, limit: Option<Value>) -> Value {
        let mut run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": prices }
                },
                "chargedEventCounts": counts,
                "options": {}
            }
        });
        if let Some(limit) = limit {
            run["data"]["options"]["maxTotalChargeUsd"] = limit;
        }
        run
    }

    #[test]
    fn reads_the_ppe_event_and_remaining_budget_from_the_run() {
        let pricing = ActorPricing::from_run(&run(
            json!({PRICE_INSIGHT_RESULT_EVENT: {"eventPriceUsd": 0.0005}}),
            json!({}),
            Some(json!(0.001)),
        ))
        .unwrap();
        assert!(pricing.is_pay_per_event());
        assert!(pricing.can_write_result());
        assert!(!pricing.event_limit_reached());
    }

    #[test]
    fn stops_dataset_writes_when_an_existing_charge_exhausted_budget() {
        let pricing = ActorPricing::from_run(&run(
            json!({PRICE_INSIGHT_RESULT_EVENT: {"eventPriceUsd": 0.0005}}),
            json!({PRICE_INSIGHT_RESULT_EVENT: 1}),
            Some(json!(0.0005)),
        ))
        .unwrap();
        assert!(!pricing.can_write_result());
        assert!(pricing.event_limit_reached());
    }

    #[test]
    fn counts_custom_and_configured_default_dataset_events_together() {
        let mut pricing = ActorPricing::from_run(&run(
            json!({
                PRICE_INSIGHT_RESULT_EVENT: {"eventPriceUsd": 0.0004},
                DEFAULT_DATASET_ITEM_EVENT: {"eventPriceUsd": 0.0001}
            }),
            json!({}),
            Some(json!(0.0005)),
        ))
        .unwrap();
        assert!(pricing.can_write_result());
        pricing.record_result_charge();
        assert!(!pricing.can_write_result());
        assert!(pricing.event_limit_reached());
    }

    #[test]
    fn treats_zero_total_charge_limit_as_unbounded() {
        let pricing = ActorPricing::from_run(&run(
            json!({PRICE_INSIGHT_RESULT_EVENT: {"eventPriceUsd": 0.0005}}),
            json!({}),
            Some(json!(0)),
        ))
        .unwrap();

        assert!(pricing.can_write_result());
        assert!(!pricing.event_limit_reached());
    }

    #[test]
    fn treats_null_total_charge_limit_as_unbounded() {
        let pricing = ActorPricing::from_run(&run(
            json!({PRICE_INSIGHT_RESULT_EVENT: {"eventPriceUsd": 0.0005}}),
            json!({}),
            Some(Value::Null),
        ))
        .unwrap();

        assert!(pricing.can_write_result());
        assert!(!pricing.event_limit_reached());
    }

    #[test]
    fn treats_missing_total_charge_limit_as_unbounded() {
        let pricing = ActorPricing::from_run(&run(
            json!({PRICE_INSIGHT_RESULT_EVENT: {"eventPriceUsd": 0.0005}}),
            json!({}),
            None,
        ))
        .unwrap();

        assert!(pricing.can_write_result());
        assert!(!pricing.event_limit_reached());
    }

    #[test]
    fn non_ppe_runs_do_not_require_custom_event_prices() {
        let pricing = ActorPricing::from_run(&json!({
            "data": { "pricingInfo": { "pricingModel": "FREE" } }
        }))
        .unwrap();
        assert!(!pricing.is_pay_per_event());
    }
}
