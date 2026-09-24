use std::{collections::HashMap, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::{Value, json};
use uuid::Uuid;

const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const DOMAIN_RESULT_CHARGE_EVENT: &str = "domain-result";

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub fn new(base_url: &str, token: String) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token,
        })
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.set_query(None);
        url.set_fragment(None);
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot be a base URL"))?;
        path.pop_if_empty();
        path.extend(["v2"].into_iter().chain(resource.iter().copied()));
        drop(path);
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    pub async fn get_input(&self, store_id: &str, input_key: &str) -> Result<Value> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&["key-value-stores", store_id, "records", input_key])?,
            )
            .send()
            .await
            .context("Failed to fetch Actor input from the default key-value store")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Value::Null);
        }
        successful_response(response, "fetch Actor input")
            .await?
            .json()
            .await
            .context("Actor input record is not valid JSON")
    }

    pub async fn get_charge_budget(&self, actor_run_id: &str) -> Result<ChargeBudget> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&["actor-runs", actor_run_id])?,
            )
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = successful_response(response, "fetch Actor run pricing")
            .await?
            .json()
            .await
            .context("Actor run pricing response is not valid JSON")?;
        ChargeBudget::from_run(&run)
    }

    pub async fn push_dataset_item(&self, dataset_id: &str, item: &Value) -> Result<()> {
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["datasets", dataset_id, "items"] )?,
            )
            .json(item)
            .send()
            .await
            .context("Failed to store an item in the default dataset")?;
        successful_response(response, "store dataset item").await?;
        Ok(())
    }

    pub async fn charge_event(&self, actor_run_id: &str, event_name: &str) -> Result<()> {
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["actor-runs", actor_run_id, "charge"] )?,
            )
            .header("idempotency-key", Uuid::new_v4().to_string())
            .json(&json!({ "eventName": event_name, "count": 1 }))
            .send()
            .await
            .context("Apify event charge request failed")?;
        successful_response(response, "charge Actor event").await?;
        Ok(())
    }

    pub async fn set_output(&self, store_id: &str, output: &Value) -> Result<()> {
        let response = self
            .request(
                Method::PUT,
                self.resource_url(&["key-value-stores", store_id, "records", "OUTPUT"] )?,
            )
            .json(output)
            .send()
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn set_status_message(&self, actor_run_id: &str, message: &str) -> Result<()> {
        let response = self
            .request(
                Method::PUT,
                self.resource_url(&["actor-runs", actor_run_id])?,
            )
            .json(&json!({
                "runId": actor_run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true
            }))
            .send()
            .await
            .context("Failed to write Actor run status message")?;
        successful_response(response, "write Actor run status message").await?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ChargeBudget {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    charged_event_counts: HashMap<String, u64>,
}

impl ChargeBudget {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        if data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(Self {
                is_pay_per_event: false,
                max_total_charge_usd: f64::INFINITY,
                event_prices: HashMap::new(),
                charged_event_counts: HashMap::new(),
            });
        }

        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (event_name, event) in events {
            let price = event
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned an invalid price for event {event_name}");
            }
            event_prices.insert(event_name.clone(), price);
        }

        let max_total_charge_usd = match data.pointer("/options/maxTotalChargeUsd") {
            None | Some(Value::Null) => f64::INFINITY,
            Some(value) => {
                let amount = value
                    .as_f64()
                    .filter(|amount| amount.is_finite() && *amount >= 0.0)
                    .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
                amount
            }
        };
        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_event_counts = HashMap::new();
        for (event_name, count) in counts {
            let count = count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            if count > 0 && !event_prices.contains_key(event_name) {
                bail!("Apify run did not provide a price for charged event {event_name}");
            }
            charged_event_counts.insert(event_name.clone(), count);
        }

        Ok(Self {
            is_pay_per_event: true,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        })
    }

    pub fn is_pay_per_event(&self) -> bool {
        self.is_pay_per_event
    }

    pub fn require_domain_result_event(&self) -> Result<()> {
        if self.is_pay_per_event && !self.event_prices.contains_key(DOMAIN_RESULT_CHARGE_EVENT) {
            bail!("Apify PAY_PER_EVENT run is missing the domain-result charge event");
        }
        Ok(())
    }

    pub fn max_event_charge_count_within_limit(&self, event_name: &str) -> usize {
        if !self.is_pay_per_event {
            return usize::MAX;
        }
        let price = self.event_prices.get(event_name).copied().unwrap_or(0.0);
        self.affordable_count(price)
    }

    pub fn can_push_item(&self, explicit_event: Option<&str>) -> bool {
        if !self.is_pay_per_event {
            return true;
        }
        let explicit_price = explicit_event
            .and_then(|event_name| self.event_prices.get(event_name))
            .copied()
            .unwrap_or(0.0);
        let dataset_price = self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0.0);
        self.affordable_count(explicit_price + dataset_price) > 0
    }

    pub fn record_dataset_item(&mut self) {
        self.record_charge(DEFAULT_DATASET_ITEM_EVENT);
    }

    pub fn record_charge(&mut self, event_name: &str) {
        if !self.is_pay_per_event || !self.event_prices.contains_key(event_name) {
            return;
        }
        *self
            .charged_event_counts
            .entry(event_name.to_owned())
            .or_default() += 1;
    }

    fn affordable_count(&self, next_item_price: f64) -> usize {
        if next_item_price <= 0.0 || !next_item_price.is_finite() {
            return usize::MAX;
        }
        let spent = self.total_charged_amount();
        if !spent.is_finite() {
            return 0;
        }
        let amount = ((self.max_total_charge_usd - spent) / next_item_price * 10_000.0).round()
            / 10_000.0;
        amount.floor().max(0.0).min(usize::MAX as f64) as usize
    }

    fn total_charged_amount(&self) -> f64 {
        self.charged_event_counts
            .iter()
            .map(|(event_name, count)| {
                self.event_prices.get(event_name).copied().unwrap_or(0.0) * *count as f64
            })
            .sum()
    }
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

    fn pay_per_event_run(max_charge: f64, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "domain-result": { "eventPriceUsd": 0.10 },
                        "apify-default-dataset-item": { "eventPriceUsd": 0.02 },
                        "apify-actor-start": { "eventPriceUsd": 0.01 }
                    }}
                },
                "chargedEventCounts": counts,
                "options": { "maxTotalChargeUsd": max_charge }
            }
        })
    }

    #[test]
    fn pay_per_event_budget_accounts_for_every_event_and_default_dataset_items() {
        let mut budget = ChargeBudget::from_run(&pay_per_event_run(
            0.35,
            json!({ "apify-actor-start": 2 }),
        ))
        .unwrap();
        budget.require_domain_result_event().unwrap();

        assert_eq!(budget.max_event_charge_count_within_limit("domain-result"), 3);
        assert!(budget.can_push_item(Some("domain-result")));
        assert!(budget.can_push_item(None));

        budget.record_dataset_item();
        budget.record_charge("domain-result");
        assert_eq!(budget.max_event_charge_count_within_limit("domain-result"), 2);
        assert_eq!(budget.total_charged_amount(), 0.14);
    }

    #[test]
    fn pay_per_event_budget_stops_when_the_next_result_would_exceed_the_cap() {
        let mut budget = ChargeBudget::from_run(&pay_per_event_run(
            0.13,
            json!({ "apify-actor-start": 1 }),
        ))
        .unwrap();

        assert!(budget.can_push_item(Some("domain-result")));
        budget.record_dataset_item();
        budget.record_charge("domain-result");
        assert_eq!(budget.max_event_charge_count_within_limit("domain-result"), 0);
        assert!(!budget.can_push_item(Some("domain-result")));
    }

    #[test]
    fn non_pay_per_event_runs_do_not_apply_a_budget_or_require_custom_events() {
        let run = json!({ "data": { "pricingInfo": { "pricingModel": "FREE" } } });
        let budget = ChargeBudget::from_run(&run).unwrap();

        assert!(!budget.is_pay_per_event());
        assert!(budget.can_push_item(Some("domain-result")));
        budget.require_domain_result_event().unwrap();
    }

    #[test]
    fn zero_spending_limit_does_not_become_unlimited() {
        let budget = ChargeBudget::from_run(&pay_per_event_run(0.0, json!({}))).unwrap();

        assert_eq!(budget.max_event_charge_count_within_limit("domain-result"), 0);
        assert!(!budget.can_push_item(Some("domain-result")));
    }

    #[test]
    fn null_spending_limit_means_unlimited() {
        let mut run = pay_per_event_run(1.0, json!({}));
        run["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
        let budget = ChargeBudget::from_run(&run).unwrap();

        assert_eq!(budget.max_event_charge_count_within_limit("domain-result"), usize::MAX);
    }

    #[test]
    fn ppe_runs_must_define_the_custom_event_used_for_successful_results() {
        let run = pay_per_event_run(1.0, json!({}));
        let mut run = run;
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove("domain-result");
        let budget = ChargeBudget::from_run(&run).unwrap();

        assert_eq!(
            budget.require_domain_result_event().unwrap_err().to_string(),
            "Apify PAY_PER_EVENT run is missing the domain-result charge event"
        );
    }

    #[test]
    fn positive_existing_charge_counts_need_a_known_price() {
        let run = pay_per_event_run(1.0, json!({ "old-event": 1 }));
        assert!(ChargeBudget::from_run(&run)
            .unwrap_err()
            .to_string()
            .contains("did not provide a price for charged event old-event"));
    }
}
