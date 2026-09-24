use crate::urls::endpoint_url;
use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use std::time::Duration;
use url::Url;

const APIFY_MAX_RETRIES: usize = 3;

pub struct ApifyClient {
    http: Client,
    base_url: Url,
    token: String,
    run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    dataset_budget: DatasetBudget,
}

impl ApifyClient {
    pub fn new(
        http: Client,
        base_url: Url,
        token: String,
        run_id: String,
        key_value_store_id: String,
        dataset_id: String,
        input_key: String,
    ) -> Self {
        Self {
            http,
            base_url,
            token,
            run_id,
            key_value_store_id,
            dataset_id,
            input_key,
            dataset_budget: DatasetBudget::default(),
        }
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = endpoint_url(
            &self.base_url,
            &[
                "v2",
                "key-value-stores",
                &self.key_value_store_id,
                "records",
                &self.input_key,
            ],
        )?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(reqwest::header::ACCEPT, "application/json")
                .send()
                .await
                .context("Failed to retrieve actor input from Apify API")?;

            if response.status() == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            let response = require_success(response, "input retrieval").await?;
            return response
                .json::<Value>()
                .await
                .context("Apify input record was not valid JSON")
                .map(Some);
        }
    }

    pub async fn dataset_capacity(&mut self, requested: usize) -> Result<usize> {
        if self.dataset_budget.run.is_none() {
            self.dataset_budget.run = Some(self.get_run_metadata().await?);
        }
        let run = self
            .dataset_budget
            .run
            .as_ref()
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        affordable_dataset_items(run, requested, self.dataset_budget.saved_rows)
    }

    pub async fn push_dataset_item(&mut self, item: &Value) -> Result<()> {
        let url = endpoint_url(
            &self.base_url,
            &["v2", "datasets", &self.dataset_id, "items"],
        )?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(item)
            .send()
            .await
            .context("Apify dataset write failed")?;
        require_success(response, "dataset write").await?;
        self.dataset_budget.saved_rows += 1;
        Ok(())
    }

    async fn get_run_metadata(&self) -> Result<Value> {
        let url = endpoint_url(&self.base_url, &["v2", "actor-runs", &self.run_id])?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(reqwest::header::ACCEPT, "application/json")
                .send()
                .await
                .context("Apify run pricing request failed")?;
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }
            let response = require_success(response, "run pricing request").await?;
            return response
                .json::<Value>()
                .await
                .context("Apify run pricing response was not valid JSON");
        }
    }
}

#[derive(Default)]
struct DatasetBudget {
    run: Option<Value>,
    saved_rows: usize,
}

fn affordable_dataset_items(
    run: &Value,
    requested: usize,
    locally_saved_rows: usize,
) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let pricing_model = data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Apify run did not provide the pricing model"))?;
    if pricing_model != "PAY_PER_EVENT" {
        return Ok(requested);
    }

    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => return Ok(requested),
        Some(value) => value
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?,
    };
    if max_charge == 0.0 {
        return Ok(requested);
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = events
        .get("apify-default-dataset-item")
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    if !item_price.is_finite() || item_price < 0.0 || !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    for (event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
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
    spent += item_price * locally_saved_rows as f64;
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let limit = max_charge + f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= limit)
        .count())
}

async fn require_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

// Do not retry dataset POSTs: a timed-out write may already have appended its row.
fn apify_retry_delay(status: StatusCode, retry_count: usize) -> Option<Duration> {
    if retry_count >= APIFY_MAX_RETRIES
        || (status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error())
    {
        return None;
    }
    Some(Duration::from_secs((retry_count + 1) as u64))
}

#[cfg(test)]
mod tests {
    use super::{affordable_dataset_items, apify_retry_delay};
    use reqwest::StatusCode;
    use serde_json::json;
    use std::time::Duration;

    fn run_pricing(max_charge: f64, charged_counts: serde_json::Value) -> serde_json::Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "apify-default-dataset-item": { "eventPriceUsd": 0.0001 },
                        "other-event": { "eventPriceUsd": 0.0002 }
                    }}
                },
                "chargedEventCounts": charged_counts,
                "options": { "maxTotalChargeUsd": max_charge }
            }
        })
    }

    #[test]
    fn positive_spending_limit_counts_custom_events_and_local_writes() {
        let run = run_pricing(
            0.001,
            json!({
                "apify-default-dataset-item": 2,
                "other-event": 1
            }),
        );
        assert_eq!(affordable_dataset_items(&run, 10, 1).unwrap(), 5);
    }

    #[test]
    fn zero_spending_limit_is_unlimited() {
        let run = run_pricing(0.0, json!({}));
        assert_eq!(affordable_dataset_items(&run, 7, 0).unwrap(), 7);
    }

    #[test]
    fn missing_spending_limit_is_unlimited() {
        let mut run = run_pricing(1.0, json!({}));
        run["data"]["options"]
            .as_object_mut()
            .unwrap()
            .remove("maxTotalChargeUsd");
        assert_eq!(affordable_dataset_items(&run, 7, 0).unwrap(), 7);
    }

    #[test]
    fn null_spending_limit_is_unlimited() {
        let mut run = run_pricing(1.0, json!({}));
        run["data"]["options"]["maxTotalChargeUsd"] = serde_json::Value::Null;
        assert_eq!(affordable_dataset_items(&run, 7, 0).unwrap(), 7);
    }

    #[test]
    fn non_ppe_pricing_does_not_apply_event_budget() {
        let mut run = run_pricing(0.0, json!({}));
        run["data"]["pricingInfo"]["pricingModel"] = json!("PRICE_PER_UNIT");
        assert_eq!(affordable_dataset_items(&run, 7, 0).unwrap(), 7);
    }

    #[test]
    fn free_default_item_event_does_not_limit_rows() {
        let mut run = run_pricing(0.001, json!({}));
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            ["apify-default-dataset-item"]["eventPriceUsd"] = json!(0.0);
        assert_eq!(affordable_dataset_items(&run, 7, 0).unwrap(), 7);
    }

    #[test]
    fn requires_ppe_metadata_and_valid_event_prices() {
        assert!(affordable_dataset_items(&json!({ "data": {} }), 1, 0)
            .unwrap_err()
            .to_string()
            .contains("pricing model"));

        let run = run_pricing(1.0, json!({ "unknown-event": 1 }));
        assert!(affordable_dataset_items(&run, 1, 0)
            .unwrap_err()
            .to_string()
            .contains("Missing price for charged event"));
    }

    #[test]
    fn retries_transient_apify_reads_with_bounded_delays() {
        assert_eq!(
            apify_retry_delay(StatusCode::TOO_MANY_REQUESTS, 0),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            apify_retry_delay(StatusCode::INTERNAL_SERVER_ERROR, 2),
            Some(Duration::from_secs(3))
        );
        assert_eq!(apify_retry_delay(StatusCode::BAD_REQUEST, 0), None);
        assert_eq!(apify_retry_delay(StatusCode::SERVICE_UNAVAILABLE, 3), None);
    }
}
