use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, Url};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_RETRIES: usize = 8;
const INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

#[derive(Debug, Default)]
pub struct DatasetBudget {
    remaining_items: usize,
}

impl DatasetBudget {
    pub fn remaining_items(&self) -> usize {
        self.remaining_items
    }
}

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub fn new(base_url: &str, token: String) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(REQUEST_TIMEOUT)
                // Keep non-idempotent dataset POSTs from being retried by reqwest.
                .retry(reqwest::retry::never())
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token,
        })
    }

    fn record_url(&self, store_id: &str, record_key: &str) -> Result<Url> {
        self.resource_url(&["key-value-stores", store_id, "records", record_key])
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(resource.iter().copied()));
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    async fn send_with_retry<F>(&self, operation: &str, request: F) -> Result<Response>
    where
        F: Fn() -> RequestBuilder,
    {
        for attempt in 0..=MAX_RETRIES {
            match request().send().await {
                Ok(response) if should_retry_status(response.status()) && attempt < MAX_RETRIES => {
                    let delay = retry_delay(attempt);
                    eprintln!(
                        "Apify {operation} returned {}; retrying ({}/{MAX_RETRIES}) after {}ms",
                        response.status(),
                        attempt + 1,
                        delay.as_millis()
                    );
                    sleep(delay).await;
                }
                Ok(response) => return Ok(response),
                Err(error) if !error.is_builder() && attempt < MAX_RETRIES => {
                    let delay = retry_delay(attempt);
                    eprintln!(
                        "Apify {operation} request failed; retrying ({}/{MAX_RETRIES}) after {}ms: {error}",
                        attempt + 1,
                        delay.as_millis()
                    );
                    sleep(delay).await;
                }
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("Apify {operation} request failed after {attempt} retries")
                    });
                }
            }
        }
        unreachable!("the retry loop always returns or continues")
    }

    pub async fn get_input(&self, store_id: &str, input_key: &str) -> Result<Option<Value>> {
        let url = self.record_url(store_id, input_key)?;
        let response = self
            .send_with_retry("INPUT request", || self.request(Method::GET, url.clone()))
            .await
            .context("Failed to fetch Actor input from the default key-value store")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Ok(Some(
            successful_response(response, "fetch Actor input")
                .await?
                .json()
                .await
                .context("Actor input record is not valid JSON")?,
        ))
    }

    pub async fn set_output(&self, store_id: &str, output: &Value) -> Result<()> {
        let url = self.record_url(store_id, "OUTPUT")?;
        let response = self
            .send_with_retry("OUTPUT write", || {
                self.request(Method::PUT, url.clone()).json(output)
            })
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn dataset_item_budget(
        &self,
        actor_run_id: &str,
        maximum_requested: usize,
    ) -> Result<DatasetBudget> {
        let url = self.resource_url(&["actor-runs", actor_run_id])?;
        let response = self
            .send_with_retry("run pricing request", || {
                self.request(Method::GET, url.clone())
            })
            .await
            .context("Apify run pricing request failed")?;
        let run = successful_response(response, "fetch Actor run pricing")
            .await?
            .json()
            .await
            .context("Actor run pricing response is not valid JSON")?;
        affordable_dataset_items(&run, maximum_requested)
    }

    pub async fn push_data(
        &self,
        dataset_id: &str,
        items: &[Value],
        budget: &mut DatasetBudget,
    ) -> Result<usize> {
        let items = &items[..items.len().min(budget.remaining_items)];
        if items.is_empty() {
            return Ok(0);
        }
        let url = self.resource_url(&["datasets", dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(items)
            .send()
            .await
            .context("Failed to store items in the default dataset")?;
        successful_response(response, "store dataset items").await?;
        budget.remaining_items -= items.len();
        Ok(items.len())
    }
}

fn should_retry_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn retry_delay(attempt: usize) -> Duration {
    INITIAL_RETRY_DELAY * 2_u32.saturating_pow(attempt as u32)
}

fn affordable_dataset_items(run: &Value, requested: usize) -> Result<DatasetBudget> {
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
    let item_price = events
        .get(DATASET_ITEM_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let max_charge = value
                .as_f64()
                .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
            if !max_charge.is_finite() || max_charge < 0.0 {
                bail!("Apify run returned invalid charging values");
            }
            (max_charge > 0.0).then_some(max_charge)
        }
    };
    if !item_price.is_finite() || item_price < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let current_dataset_items = counts
        .get(DATASET_ITEM_EVENT)
        .map(|count| {
            count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {DATASET_ITEM_EVENT}"))
        })
        .transpose()?
        .unwrap_or(0);

    if item_price == 0.0 {
        return Ok(DatasetBudget {
            remaining_items: requested,
        });
    }

    let mut spent = 0.0;
    for (event_name, count) in counts {
        let mut count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if event_name == DATASET_ITEM_EVENT {
            count = current_dataset_items;
        }
        if count == 0 {
            continue;
        }
        let price = events
            .get(event_name)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    let remaining_items = if let Some(max_charge) = max_charge {
        let tolerance = f64::EPSILON * max_charge.max(1.0);
        let available_charge = (max_charge - spent + tolerance).max(0.0);
        let affordable = (available_charge / item_price).floor();
        if affordable.is_finite() {
            (affordable as usize).min(requested)
        } else {
            requested
        }
    } else {
        requested
    };
    Ok(DatasetBudget { remaining_items })
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

    fn run(max_charge: f64, charged_counts: Value) -> Value {
        run_with_max_charge(Some(json!(max_charge)), charged_counts)
    }

    fn run_with_max_charge(max_charge: Option<Value>, charged_counts: Value) -> Value {
        let mut run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                        "other-event": {"eventPriceUsd": 0.05}
                    }}
                },
                "chargedEventCounts": charged_counts,
                "options": {}
            }
        });
        if let Some(max_charge) = max_charge {
            run["data"]["options"]["maxTotalChargeUsd"] = max_charge;
        }
        run
    }

    #[test]
    fn caps_rows_by_the_remaining_ppe_charge() {
        let budget = affordable_dataset_items(&run(0.25, json!({"other-event": 1})), 10).unwrap();
        assert_eq!(budget.remaining_items(), 2);
    }

    #[test]
    fn treats_a_missing_maximum_charge_as_unlimited() {
        let budget = affordable_dataset_items(&run_with_max_charge(None, json!({})), 10).unwrap();
        assert_eq!(budget.remaining_items(), 10);
    }

    #[test]
    fn treats_a_null_maximum_charge_as_unlimited() {
        let budget =
            affordable_dataset_items(&run_with_max_charge(Some(Value::Null), json!({})), 10)
                .unwrap();
        assert_eq!(budget.remaining_items(), 10);
    }

    #[test]
    fn treats_a_zero_maximum_charge_as_unlimited() {
        let budget = affordable_dataset_items(&run(0.0, json!({})), 10).unwrap();
        assert_eq!(budget.remaining_items(), 10);
    }

    #[test]
    fn accounts_for_existing_dataset_events_and_zero_charge_prices() {
        let budget = affordable_dataset_items(
            &run(
                0.25,
                json!({"apify-default-dataset-item": 1, "other-event": 1}),
            ),
            10,
        )
        .unwrap();
        assert_eq!(budget.remaining_items(), 1);

        let mut no_charge_run = run(0.0, json!({}));
        no_charge_run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            [DATASET_ITEM_EVENT]["eventPriceUsd"] = json!(0.0);
        assert_eq!(
            affordable_dataset_items(&no_charge_run, 10)
                .unwrap()
                .remaining_items(),
            10
        );
    }

    #[test]
    fn rejects_missing_or_invalid_ppe_metadata() {
        assert!(affordable_dataset_items(&json!({}), 10).is_err());
        let mut fixed_price = run(1.0, json!({}));
        fixed_price["data"]["pricingInfo"]["pricingModel"] = json!("FLAT");
        assert!(
            affordable_dataset_items(&fixed_price, 10)
                .unwrap_err()
                .to_string()
                .contains("not configured for pay-per-event")
        );
    }

    #[test]
    fn retry_policy_matches_the_apify_sdk_backoff_window() {
        assert_eq!(MAX_RETRIES, 8);
        assert_eq!(retry_delay(0), Duration::from_millis(500));
        assert_eq!(retry_delay(1), Duration::from_secs(1));
    }
}
