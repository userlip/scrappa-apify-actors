use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};

const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
    store_id: String,
    input_key: String,
    dataset_id: String,
    run_id: String,
}

#[derive(Clone, Debug)]
pub struct RunPricing {
    pub is_pay_per_event: bool,
    event_prices: HashMap<String, f64>,
    charged_counts: HashMap<String, u64>,
    max_total_charge_usd: f64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PushChargeResult {
    pub saved_count: usize,
    pub charged_count: usize,
    pub event_charge_limit_reached: bool,
}

impl ApifyClient {
    pub fn new(
        base_url: &str,
        token: String,
        store_id: String,
        input_key: String,
        dataset_id: String,
        run_id: String,
    ) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .context("Could not create Apify API client")?,
            base_url: Url::parse(base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token,
            store_id,
            input_key,
            dataset_id,
            run_id,
        })
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot be a base URL"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(resource.iter().copied()));
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&[
                    "key-value-stores",
                    &self.store_id,
                    "records",
                    &self.input_key,
                ])?,
            )
            .send()
            .await
            .context("Failed to fetch Actor input from the default key-value store")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "fetch Actor input").await?;
        Ok(Some(
            response
                .json()
                .await
                .context("Actor input record is not valid JSON")?,
        ))
    }

    pub async fn get_run_pricing(&self) -> Result<RunPricing> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&["actor-runs", &self.run_id])?,
            )
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = successful_response(response, "fetch Actor run pricing")
            .await?
            .json::<Value>()
            .await
            .context("Apify run pricing response is not valid JSON")?;
        RunPricing::from_run(&run)
    }

    pub async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["datasets", &self.dataset_id, "items"])?,
            )
            .json(items)
            .send()
            .await
            .context("Failed to store review items in the default dataset")?;
        successful_response(response, "store review items").await?;
        Ok(())
    }

    pub async fn charge_event(
        &self,
        event_name: &str,
        count: usize,
        idempotency_key: &str,
    ) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["actor-runs", &self.run_id, "charge"])?,
            )
            .header("idempotency-key", idempotency_key)
            .json(&json!({"eventName": event_name, "count": count}))
            .send()
            .await
            .context("Apify event charge request failed")?;
        successful_response(response, "charge Actor event").await?;
        Ok(())
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let response = self
            .request(
                Method::PUT,
                self.resource_url(&["key-value-stores", &self.store_id, "records", "OUTPUT"])?,
            )
            .json(output)
            .send()
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn set_terminal_status_message(&self, message: &str) -> Result<()> {
        let response = self
            .request(
                Method::PUT,
                self.resource_url(&["actor-runs", &self.run_id])?,
            )
            .json(&json!({
                "runId": self.run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true,
            }))
            .send()
            .await
            .context("Failed to set Actor run status message")?;
        successful_response(response, "set Actor run status message").await?;
        Ok(())
    }
}

impl RunPricing {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_info = data
            .get("pricingInfo")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let is_pay_per_event =
            pricing_info.get("pricingModel").and_then(Value::as_str) == Some("PAY_PER_EVENT");
        let mut event_prices = HashMap::new();
        if is_pay_per_event {
            let events = pricing_info
                .pointer("/pricingPerEvent/actorChargeEvents")
                .and_then(Value::as_object)
                .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
            for (event_name, event) in events {
                if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
                    if !price.is_finite() || price < 0.0 {
                        bail!("Apify run returned invalid charge price for event {event_name}");
                    }
                    event_prices.insert(event_name.clone(), price);
                }
            }
        }
        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(f64::INFINITY);
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
            event_prices,
            charged_counts,
            max_total_charge_usd,
        })
    }

    pub fn chargeable_event_count(&self, event_name: &str) -> Option<usize> {
        if !self.is_pay_per_event {
            return None;
        }
        let Some(price) = self.event_prices.get(event_name).copied() else {
            return None;
        };
        if price == 0.0 {
            return None;
        }
        let total = round_to(self.total_charged_amount(), 6);
        let count = ((self.max_total_charge_usd - total) / price).max(0.0);
        Some(round_to(count, 4).floor().clamp(0.0, usize::MAX as f64) as usize)
    }

    pub fn dataset_push_limit(&self, requested: usize, event_name: &str) -> usize {
        if !self.is_pay_per_event || requested == 0 {
            return requested;
        }
        let item_price = self.event_prices.get(event_name).copied().unwrap_or(0.0)
            + self
                .event_prices
                .get(DEFAULT_DATASET_ITEM_EVENT)
                .copied()
                .unwrap_or(0.0);
        if item_price <= 0.0 {
            return requested;
        }
        let remaining = self.max_total_charge_usd - round_to(self.total_charged_amount(), 6);
        let affordable = round_to((remaining / item_price).max(0.0), 4)
            .floor()
            .clamp(0.0, usize::MAX as f64) as usize;
        if affordable >= requested {
            requested
        } else if affordable == 0 && self.total_charged_amount() <= self.max_total_charge_usd {
            1
        } else {
            affordable.min(requested)
        }
    }

    pub fn record_dataset_push(&mut self, event_name: &str, count: usize) -> PushChargeResult {
        if !self.is_pay_per_event {
            return PushChargeResult {
                saved_count: count,
                charged_count: 0,
                event_charge_limit_reached: false,
            };
        }
        if count == 0 {
            let event_charge_limit_reached = self
                .chargeable_event_count(event_name)
                .is_some_and(|capacity| capacity == 0)
                || self
                    .chargeable_event_count(DEFAULT_DATASET_ITEM_EVENT)
                    .is_some_and(|capacity| capacity == 0);
            return PushChargeResult {
                saved_count: 0,
                charged_count: 0,
                event_charge_limit_reached,
            };
        }
        *self
            .charged_counts
            .entry(event_name.to_owned())
            .or_default() += count as u64;
        *self
            .charged_counts
            .entry(DEFAULT_DATASET_ITEM_EVENT.to_owned())
            .or_default() += count as u64;
        let event_charge_limit_reached = self
            .chargeable_event_count(event_name)
            .is_some_and(|capacity| capacity == 0)
            || self
                .chargeable_event_count(DEFAULT_DATASET_ITEM_EVENT)
                .is_some_and(|capacity| capacity == 0);
        PushChargeResult {
            saved_count: count,
            charged_count: count.saturating_mul(2),
            event_charge_limit_reached,
        }
    }

    fn total_charged_amount(&self) -> f64 {
        self.charged_counts
            .iter()
            .map(|(event_name, count)| {
                self.event_prices.get(event_name).copied().unwrap_or(0.0) * *count as f64
            })
            .sum()
    }

    pub fn has_event_price(&self, event_name: &str) -> bool {
        self.event_prices.contains_key(event_name)
    }
}

fn round_to(value: f64, decimal_places: u32) -> f64 {
    let multiplier = 10_f64.powi(decimal_places as i32);
    (value * multiplier).round() / multiplier
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let status_code = status.as_u16();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.is_empty() {
        format!("HTTP {status_code}")
    } else {
        body
    };
    bail!("Apify API error ({status_code}) while trying to {operation}: {detail}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pricing(prices: Value, counts: Value, maximum: f64) -> RunPricing {
        RunPricing::from_run(&json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": prices}
                },
                "chargedEventCounts": counts,
                "options": {"maxTotalChargeUsd": maximum}
            }
        }))
        .unwrap()
    }

    #[test]
    fn calculates_remaining_custom_event_capacity_with_other_charges_included() {
        let budget = pricing(
            json!({
                "review-result": {"eventPriceUsd": 0.00025},
                "apify-actor-start": {"eventPriceUsd": 0.0003},
            }),
            json!({"apify-actor-start": 1}),
            0.001,
        );
        assert_eq!(budget.chargeable_event_count("review-result"), Some(2));
    }

    #[test]
    fn limits_default_dataset_push_by_combined_event_prices_and_records_saved_items() {
        let mut budget = pricing(
            json!({
                "review-result": {"eventPriceUsd": 0.00025},
                "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
            }),
            json!({}),
            0.001,
        );
        assert_eq!(budget.dataset_push_limit(5, "review-result"), 2);
        let result = budget.record_dataset_push("review-result", 2);
        assert_eq!(
            result,
            PushChargeResult {
                saved_count: 2,
                charged_count: 4,
                event_charge_limit_reached: false
            }
        );
        assert_eq!(budget.chargeable_event_count("review-result"), Some(1));
        assert_eq!(budget.dataset_push_limit(5, "review-result"), 1);
        assert_eq!(budget.dataset_push_limit(5, "unknown-event"), 3);
    }

    #[test]
    fn detects_exhausted_limit_after_a_charged_page() {
        let mut budget = pricing(
            json!({"review-result": {"eventPriceUsd": 0.00025}}),
            json!({}),
            0.0005,
        );
        assert_eq!(budget.chargeable_event_count("review-result"), Some(2));
        assert_eq!(budget.dataset_push_limit(5, "review-result"), 2);
        let result = budget.record_dataset_push("review-result", 2);
        assert_eq!(
            result,
            PushChargeResult {
                saved_count: 2,
                charged_count: 4,
                event_charge_limit_reached: true
            }
        );
        assert_eq!(budget.chargeable_event_count("review-result"), Some(0));
    }

    #[test]
    fn allows_non_ppe_runs_without_event_budget_limits() {
        let pricing = RunPricing::from_run(
            &json!({ "data": {"pricingInfo":{"pricingModel":"PRICE_PER_DATASET_ITEM"}} }),
        )
        .unwrap();
        assert!(!pricing.is_pay_per_event);
        assert_eq!(pricing.chargeable_event_count("review-result"), None);
        assert_eq!(pricing.dataset_push_limit(10, "review-result"), 10);
    }
}
