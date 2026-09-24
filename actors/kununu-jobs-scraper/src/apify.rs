use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Method, Response, StatusCode, Url};
use serde_json::{json, Value};
use std::{collections::HashMap, time::Duration};

pub const RESULT_CHARGE_EVENT: &str = "kununu-job-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
    run_id: String,
    store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    pub fn new(
        base_url: &str,
        token: String,
        run_id: String,
        store_id: String,
        dataset_id: String,
        input_key: String,
    ) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token,
            run_id,
            store_id,
            dataset_id,
            input_key,
        })
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .request(Method::GET, url)
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

    pub async fn charging_manager(&self) -> Result<ChargingManager> {
        let url = self.resource_url(&["actor-runs", &self.run_id])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = successful_response(response, "fetch Actor run pricing")
            .await?
            .json::<Value>()
            .await
            .context("Actor run pricing response is not valid JSON")?;
        ChargingManager::from_run(&run)
    }

    pub async fn push_data(
        &self,
        items: &[Value],
        charging: &mut ChargingManager,
        page: i64,
    ) -> Result<PushDataResult> {
        if items.is_empty() {
            return Ok(PushDataResult::default());
        }

        let saved_count = charging.push_count(items.len(), RESULT_CHARGE_EVENT, true);
        if saved_count == 0 {
            return Ok(PushDataResult {
                saved_count: 0,
                charged_count: 0,
                event_charge_limit_reached: true,
            });
        }

        let url = self.resource_url(&["datasets", &self.dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(&items[..saved_count])
            .send()
            .await
            .context("Failed to store Kununu job results in the default dataset")?;
        successful_response(response, "store Kununu job results").await?;

        if !charging.is_pay_per_event {
            return Ok(PushDataResult {
                saved_count,
                charged_count: 0,
                event_charge_limit_reached: false,
            });
        }

        if charging.has_event(RESULT_CHARGE_EVENT) {
            let url = self.resource_url(&["actor-runs", &self.run_id, "charge"])?;
            let idempotency_key = format!("{}-{RESULT_CHARGE_EVENT}-{page}", self.run_id);
            let response = self
                .request(Method::POST, url)
                .header("idempotency-key", idempotency_key)
                .json(&json!({"eventName": RESULT_CHARGE_EVENT, "count": saved_count}))
                .send()
                .await
                .context("Apify result charge request failed")?;
            successful_response(response, "charge Kununu job results").await?;
        } else {
            eprintln!("Attempting to charge for an unknown event '{RESULT_CHARGE_EVENT}'");
        }

        charging.record_charge(RESULT_CHARGE_EVENT, saved_count);
        charging.record_charge(DEFAULT_DATASET_ITEM_EVENT, saved_count);

        Ok(PushDataResult {
            saved_count,
            charged_count: saved_count.saturating_mul(2),
            event_charge_limit_reached: charging.is_event_limit_reached(RESULT_CHARGE_EVENT)
                || charging.is_event_limit_reached(DEFAULT_DATASET_ITEM_EVENT),
        })
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let url = self.resource_url(&["key-value-stores", &self.store_id, "records", "OUTPUT"])?;
        let response = self
            .request(Method::PUT, url)
            .json(output)
            .send()
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn set_terminal_status_message(&self, message: &str) -> Result<()> {
        let url = self.resource_url(&["actor-runs", &self.run_id])?;
        let response = self
            .request(Method::PUT, url)
            .timeout(Duration::from_secs(1))
            .json(&json!({
                "runId": self.run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true
            }))
            .send()
            .await
            .context("Failed to set Actor status message")?;
        successful_response(response, "set Actor status message").await?;
        Ok(())
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
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
            .header(header::ACCEPT, "application/json")
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PushDataResult {
    pub saved_count: usize,
    pub charged_count: usize,
    pub event_charge_limit_reached: bool,
}

#[derive(Debug, Clone)]
pub struct ChargingManager {
    is_pay_per_event: bool,
    event_prices: HashMap<String, f64>,
    charged_event_counts: HashMap<String, u64>,
    max_total_charge_usd: f64,
}

impl ChargingManager {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
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
                let price = event
                    .get("eventPriceUsd")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| {
                        anyhow!("Apify run did not provide the price for event {event_name}")
                    })?;
                if !price.is_finite() || price < 0.0 {
                    bail!("Apify run returned an invalid price for event {event_name}");
                }
                event_prices.insert(event_name.clone(), price);
            }
        }

        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .map(|events| {
                events
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

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|amount| *amount != 0.0)
            .unwrap_or(f64::INFINITY);
        if !max_total_charge_usd.is_finite() && max_total_charge_usd != f64::INFINITY {
            bail!("Apify run returned an invalid maximum total charge");
        }

        Ok(Self {
            is_pay_per_event,
            event_prices,
            charged_event_counts,
            max_total_charge_usd,
        })
    }

    pub fn push_count(
        &self,
        requested: usize,
        event_name: &str,
        is_default_dataset: bool,
    ) -> usize {
        if !self.is_pay_per_event {
            return requested;
        }

        let item_price = self.event_price(event_name)
            + if is_default_dataset {
                self.event_price(DEFAULT_DATASET_ITEM_EVENT)
            } else {
                0.0
            };
        let max_count = if item_price <= 0.0 {
            usize::MAX
        } else {
            self.affordable_count(item_price)
        };
        if max_count >= requested {
            return requested;
        }
        if requested > 0
            && max_count == 0
            && self.total_charged_amount() <= self.max_total_charge_usd
        {
            return 1;
        }
        requested.min(max_count)
    }

    pub fn record_charge(&mut self, event_name: &str, count: usize) {
        *self
            .charged_event_counts
            .entry(event_name.to_owned())
            .or_default() += count as u64;
    }

    pub fn is_event_limit_reached(&self, event_name: &str) -> bool {
        let price = self.event_price(event_name);
        price > 0.0 && self.affordable_count(price) == 0
    }

    fn has_event(&self, event_name: &str) -> bool {
        self.event_prices.contains_key(event_name)
    }

    fn event_price(&self, event_name: &str) -> f64 {
        self.event_prices.get(event_name).copied().unwrap_or(0.0)
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_event_counts
            .iter()
            .map(|(event_name, count)| self.event_price(event_name) * *count as f64)
            .sum::<f64>();
        (total * 1_000_000.0).round() / 1_000_000.0
    }

    fn affordable_count(&self, price: f64) -> usize {
        let unrounded = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !unrounded.is_finite() {
            return if unrounded.is_sign_positive() {
                usize::MAX
            } else {
                0
            };
        }
        let rounded = (unrounded * 10_000.0).round() / 10_000.0;
        rounded.floor().max(0.0).min(usize::MAX as f64) as usize
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
    use super::{ChargingManager, DEFAULT_DATASET_ITEM_EVENT, RESULT_CHARGE_EVENT};
    use serde_json::json;

    fn run(max_total: f64, counts: serde_json::Value) -> serde_json::Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "kununu-job-result": {"eventPriceUsd": 0.0002},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001}
                    }}
                },
                "chargedEventCounts": counts,
                "options": {"maxTotalChargeUsd": max_total}
            }
        })
    }

    #[test]
    fn limits_dataset_rows_by_combined_custom_and_default_dataset_prices() {
        let mut charging = ChargingManager::from_run(&run(0.001, json!({}))).unwrap();
        assert_eq!(charging.push_count(10, RESULT_CHARGE_EVENT, true), 3);
        charging.record_charge(RESULT_CHARGE_EVENT, 3);
        charging.record_charge(DEFAULT_DATASET_ITEM_EVENT, 3);
        assert!(charging.is_event_limit_reached(RESULT_CHARGE_EVENT));
        assert!(!charging.is_event_limit_reached(DEFAULT_DATASET_ITEM_EVENT));
    }

    #[test]
    fn allows_one_final_row_when_the_budget_was_already_reached() {
        let mut charging = ChargingManager::from_run(&run(
            0.0003,
            json!({"kununu-job-result":1,"apify-default-dataset-item":1}),
        ))
        .unwrap();
        assert_eq!(charging.push_count(5, RESULT_CHARGE_EVENT, true), 1);
        charging.record_charge(RESULT_CHARGE_EVENT, 1);
        charging.record_charge(DEFAULT_DATASET_ITEM_EVENT, 1);
        assert!(charging.is_event_limit_reached(RESULT_CHARGE_EVENT));
        assert_eq!(charging.push_count(5, RESULT_CHARGE_EVENT, true), 0);
    }

    #[test]
    fn accounts_for_preexisting_run_charges_and_unpriced_events() {
        let run = run(
            0.001,
            json!({"kununu-job-result":2,"apify-default-dataset-item":1,"other-event":4}),
        );
        let charging = ChargingManager::from_run(&run).unwrap();
        assert_eq!(charging.push_count(10, RESULT_CHARGE_EVENT, true), 1);

        let free = ChargingManager::from_run(&json!({
            "data":{"pricingInfo":{"pricingModel":"PER_UNIT"},"options":{}}
        }))
        .unwrap();
        assert_eq!(free.push_count(7, RESULT_CHARGE_EVENT, true), 7);
    }
}
