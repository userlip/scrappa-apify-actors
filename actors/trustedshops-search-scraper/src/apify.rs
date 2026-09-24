use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, Response, StatusCode, Url, header};
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};

pub const SHOP_RESULT_CHARGE_EVENT: &str = "shop-result";
pub const DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
    run_id: String,
    store_id: String,
    dataset_id: String,
    input_key: String,
    is_at_home: bool,
}

impl ApifyClient {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        base_url: &str,
        token: String,
        run_id: String,
        store_id: String,
        dataset_id: String,
        input_key: String,
        is_at_home: bool,
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
            is_at_home,
        })
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

    fn record_url(&self, store_id: &str, record_key: &str) -> Result<Url> {
        self.resource_url(&["key-value-stores", store_id, "records", record_key])
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let response = self
            .request(
                Method::GET,
                self.record_url(&self.store_id, &self.input_key)?,
            )
            .send()
            .await
            .context("Failed to fetch Actor input from the default key-value store")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        successful_response(response, "fetch Actor input")
            .await?
            .json()
            .await
            .map(Some)
            .context("Actor input record is not valid JSON")
    }

    pub async fn load_charge_manager(
        &self,
        local_max_charge: Option<f64>,
        test_ppe: bool,
    ) -> Result<ChargeManager> {
        if self.is_at_home {
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
                .context("Actor run pricing response is not valid JSON")?;
            return ChargeManager::from_run(&run);
        }
        Ok(ChargeManager::local(
            test_ppe,
            local_max_charge.unwrap_or(f64::INFINITY),
        ))
    }

    pub async fn push_data(&self, items: &[Value]) -> Result<()> {
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
            .context("Failed to store Trusted Shops results in the default dataset")?;
        successful_response(response, "store dataset items").await?;
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
        successful_response(response, "charge Trusted Shops result events").await?;
        Ok(())
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let response = self
            .request(Method::PUT, self.record_url(&self.store_id, "OUTPUT")?)
            .json(output)
            .send()
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn set_status_message(&self, status_message: &str) -> Result<()> {
        if !self.is_at_home {
            return Ok(());
        }
        let result = async {
            let response = self
                .request(
                    Method::PUT,
                    self.resource_url(&["actor-runs", &self.run_id])?,
                )
                .json(&json!({
                    "runId": self.run_id,
                    "statusMessage": status_message,
                    "isStatusMessageTerminal": true
                }))
                .send()
                .await
                .context("Failed to set Actor run status message")?;
            successful_response(response, "set Actor run status message").await?;
            Ok::<(), anyhow::Error>(())
        }
        .await;
        if let Err(error) = result {
            eprintln!("Warning: could not set Actor run status message: {error:#}");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChargeResult {
    pub charged_count: usize,
    pub event_charge_limit_reached: bool,
}

pub struct ChargeManager {
    is_pay_per_event: bool,
    is_at_home: bool,
    max_total_charge_usd: f64,
    event_prices: BTreeMap<String, Option<f64>>,
    charged_event_counts: BTreeMap<String, u64>,
}

impl ChargeManager {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let is_pay_per_event = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self::local(false, f64::INFINITY));
        }

        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let event_prices = events
            .iter()
            .map(|(event_name, event)| {
                (
                    event_name.clone(),
                    event.get("eventPriceUsd").and_then(Value::as_f64),
                )
            })
            .collect();
        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|value| *value != 0.0)
            .unwrap_or(f64::INFINITY);
        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .map(|counts| {
                counts
                    .iter()
                    .filter_map(|(event_name, count)| {
                        count.as_u64().map(|count| (event_name.clone(), count))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            is_pay_per_event,
            is_at_home: true,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        })
    }

    fn local(is_pay_per_event: bool, max_total_charge_usd: f64) -> Self {
        Self {
            is_pay_per_event,
            is_at_home: false,
            max_total_charge_usd,
            event_prices: BTreeMap::new(),
            charged_event_counts: BTreeMap::new(),
        }
    }

    pub fn is_pay_per_event(&self) -> bool {
        self.is_pay_per_event
    }

    pub fn items_to_keep(&self, requested_count: usize) -> usize {
        if !self.is_pay_per_event || requested_count == 0 {
            return requested_count;
        }
        let item_price = self.event_price(SHOP_RESULT_CHARGE_EVENT).unwrap_or(0.0)
            + self.event_price(DATASET_ITEM_CHARGE_EVENT).unwrap_or(0.0);
        let max_count = if item_price > 0.0 {
            self.max_event_charge_count_for_price(item_price)
        } else {
            usize::MAX
        };
        if max_count >= requested_count {
            return requested_count;
        }
        if max_count == 0 && self.total_charged_amount() <= self.max_total_charge_usd {
            return 1;
        }
        max_count
    }

    pub async fn charge(
        &mut self,
        apify: &ApifyClient,
        event_name: &str,
        requested_count: usize,
        idempotency_key: &str,
    ) -> Result<ChargeResult> {
        if !self.is_pay_per_event {
            return Ok(ChargeResult {
                charged_count: 0,
                event_charge_limit_reached: false,
            });
        }
        let max_count = self.max_event_charge_count(event_name);
        let charged_count = if requested_count <= max_count {
            requested_count
        } else if self.total_charged_amount() <= self.max_total_charge_usd {
            max_count.saturating_add(1)
        } else {
            0
        };

        if charged_count > 0 {
            if self.is_at_home
                && self.event_prices.contains_key(event_name)
                && !event_name.starts_with("apify-")
            {
                apify
                    .charge_event(event_name, charged_count, idempotency_key)
                    .await?;
            }
            let current_count = self
                .charged_event_counts
                .get(event_name)
                .copied()
                .unwrap_or(0);
            let next_count = current_count
                .checked_add(charged_count as u64)
                .ok_or_else(|| anyhow!("Charged event count overflowed"))?;
            self.charged_event_counts
                .insert(event_name.to_owned(), next_count);
        }

        Ok(ChargeResult {
            charged_count,
            event_charge_limit_reached: self.max_event_charge_count(event_name) == 0,
        })
    }

    fn event_price(&self, event_name: &str) -> Option<f64> {
        if !self.is_at_home {
            return Some(1.0);
        }
        self.event_prices.get(event_name).copied().flatten()
    }

    fn max_event_charge_count(&self, event_name: &str) -> usize {
        let Some(price) = self.event_price(event_name) else {
            return usize::MAX;
        };
        self.max_event_charge_count_for_price(price)
    }

    fn max_event_charge_count_for_price(&self, price: f64) -> usize {
        if price == 0.0 {
            return usize::MAX;
        }
        if price < 0.0 || !price.is_finite() {
            return 0;
        }
        let unrounded = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        let rounded = format!("{unrounded:.4}")
            .parse::<f64>()
            .unwrap_or(unrounded);
        if rounded.is_nan() {
            return 0;
        }
        if rounded.is_infinite() {
            return if rounded.is_sign_positive() {
                usize::MAX
            } else {
                0
            };
        }
        rounded.floor().max(0.0) as usize
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_event_counts
            .iter()
            .map(|(event_name, count)| {
                let price = self.event_price(event_name).unwrap_or(0.0);
                price * *count as f64
            })
            .sum::<f64>();
        (total * 1_000_000.0).round() / 1_000_000.0
    }
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let status_code = status.as_u16();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
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

    fn run_info(max_charge: f64, charged_counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "shop-result": {"eventPriceUsd": 0.1},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.02},
                        "other-event": {"eventPriceUsd": 0.05}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": charged_counts
            }
        })
    }

    #[test]
    fn pay_per_event_capacity_accounts_for_custom_and_dataset_events() {
        let manager = ChargeManager::from_run(&run_info(0.24, json!({"other-event": 1}))).unwrap();
        assert_eq!(manager.items_to_keep(10), 1);

        let manager =
            ChargeManager::from_run(&run_info(0.50, json!({"shop-result": 1, "other-event": 2})))
                .unwrap();
        assert_eq!(manager.items_to_keep(10), 2);
    }

    #[test]
    fn charge_limit_keeps_one_item_to_match_actor_sdk_over_limit_signal() {
        let manager = ChargeManager::from_run(&run_info(0.0, json!({}))).unwrap();
        assert!(manager.is_pay_per_event());
        assert_eq!(manager.items_to_keep(5), 5);

        let manager = ChargeManager::from_run(&run_info(0.01, json!({}))).unwrap();
        assert_eq!(manager.items_to_keep(5), 1);
    }

    #[test]
    fn non_pay_per_event_runs_keep_every_dataset_item() {
        let run = json!({"data": {"pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM"}}});
        let manager = ChargeManager::from_run(&run).unwrap();
        assert!(!manager.is_pay_per_event());
        assert_eq!(manager.items_to_keep(12), 12);
    }

    #[test]
    fn starting_charges_reduce_remaining_capacity_and_unknown_prices_cost_nothing() {
        let manager = ChargeManager::from_run(&run_info(
            0.50,
            json!({"shop-result": 1, "apify-default-dataset-item": 1, "unknown": 50}),
        ))
        .unwrap();
        assert_eq!(manager.items_to_keep(10), 3);
    }

    #[test]
    fn local_test_pricing_uses_unit_prices() {
        let manager = ChargeManager::local(true, 5.0);
        assert_eq!(manager.items_to_keep(5), 2);
    }

    #[tokio::test]
    async fn input_read_and_kv_dataset_writes_use_apify_storage_contract() {
        use crate::test_support::{MockServer, has_bearer_token, request_parts, response};

        let server = MockServer::start(vec![
            response(200, r#"{"q":"zalando","page":0}"#),
            response(200, "{}"),
            response(200, "{}"),
        ]);
        let apify = ApifyClient::new(
            &server.base_url,
            "test-token".to_owned(),
            "test-run".to_owned(),
            "test-store".to_owned(),
            "test-dataset".to_owned(),
            "INPUT".to_owned(),
            true,
        )
        .unwrap();

        assert_eq!(
            apify.get_input().await.unwrap(),
            Some(json!({"q":"zalando","page":0}))
        );
        apify
            .set_output(&json!({"pages_fetched": 1}))
            .await
            .unwrap();
        apify
            .set_status_message("Charge limit reached")
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_parts(&requests[0]).0, "GET");
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert_eq!(request_parts(&requests[1]).0, "PUT");
        assert_eq!(
            request_parts(&requests[1]).1,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
        assert_eq!(request_parts(&requests[2]).0, "PUT");
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
        assert!(has_bearer_token(&requests[0], "test-token"));
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[1]).2).unwrap(),
            json!({"pages_fetched": 1})
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[2]).2).unwrap(),
            json!({"runId":"test-run","statusMessage":"Charge limit reached","isStatusMessageTerminal":true})
        );
    }

    #[tokio::test]
    async fn custom_event_charge_sends_event_count_and_idempotency_key() {
        use crate::test_support::{MockServer, has_bearer_token, request_parts, response};

        let server = MockServer::start(vec![response(201, "{}")]);
        let apify = ApifyClient::new(
            &server.base_url,
            "test-token".to_owned(),
            "test-run".to_owned(),
            "test-store".to_owned(),
            "test-dataset".to_owned(),
            "INPUT".to_owned(),
            true,
        )
        .unwrap();

        apify
            .charge_event("shop-result", 2, "test-run-page-3")
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(request_parts(&requests[0]).0, "POST");
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/actor-runs/test-run/charge"
        );
        assert!(has_bearer_token(&requests[0], "test-token"));
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .contains("idempotency-key: test-run-page-3")
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[0]).2).unwrap(),
            json!({"eventName":"shop-result","count":2})
        );
    }

    #[tokio::test]
    async fn missing_input_is_not_found_and_not_an_apify_failure() {
        use crate::test_support::{MockServer, request_parts, response};

        let server = MockServer::start(vec![response(404, r#"{"error":"record-not-found"}"#)]);
        let apify = ApifyClient::new(
            &server.base_url,
            "test-token".to_owned(),
            "test-run".to_owned(),
            "test-store".to_owned(),
            "test-dataset".to_owned(),
            "INPUT".to_owned(),
            true,
        )
        .unwrap();
        assert_eq!(apify.get_input().await.unwrap(), None);
        assert_eq!(
            request_parts(&server.requests()[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
    }
}
