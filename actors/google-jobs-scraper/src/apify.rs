use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{Map, Value};
use url::Url;

const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const MAX_DATASET_BATCH_BYTES: usize = 4_000_000;
const MAX_DATASET_ITEM_BYTES: usize = 4_900_000;

pub struct ApifyClient {
    http: Client,
    base_url: Url,
    api_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
}

impl ApifyClient {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        http: Client,
        base_url: Url,
        api_token: String,
        key_value_store_id: String,
        dataset_id: String,
        actor_run_id: String,
        input_key: String,
    ) -> Self {
        Self {
            http,
            base_url,
            api_token,
            key_value_store_id,
            dataset_id,
            actor_run_id,
            input_key,
        }
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .request(self.http.get(url))
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "Apify INPUT request").await?;
        let input = response
            .json()
            .await
            .context("Apify INPUT record is not valid JSON")?;
        Ok(Some(input))
    }

    pub async fn dataset_item_limit(&self, requested: usize) -> Result<usize> {
        if requested == 0 {
            return Ok(0);
        }
        let url = self.resource_url(&["actor-runs", &self.actor_run_id])?;
        let response = self
            .request(self.http.get(url))
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run: Value = successful_response(response, "Apify run pricing request")
            .await?
            .json()
            .await
            .context("Apify run pricing response is not valid JSON")?;
        affordable_dataset_items(&run, requested)
    }

    pub async fn push_dataset_items(&self, items: &[Value]) -> Result<usize> {
        let batches = dataset_batches(items)?;
        let mut pushed = 0;
        for batch in batches {
            let url = self.resource_url(&["datasets", &self.dataset_id, "items"])?;
            let response = self
                .request(self.http.post(url))
                .json(&batch)
                .send()
                .await
                .context("Apify dataset write request failed")?;
            successful_response(response, "store Google Jobs dataset items").await?;
            pushed += batch.len();
        }
        Ok(pushed)
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .request(self.http.put(url))
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT request failed")?;
        successful_response(response, "write Google Jobs OUTPUT").await?;
        Ok(())
    }

    fn request(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .bearer_auth(&self.api_token)
            .header(header::ACCEPT, "application/json")
    }

    fn resource_url(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(segments.iter().copied()));
        Ok(url)
    }
}

fn dataset_batches(items: &[Value]) -> Result<Vec<Vec<Value>>> {
    let mut batches = Vec::new();
    let mut batch = Vec::new();
    let mut batch_bytes = 2;
    for item in items {
        let item_bytes = serde_json::to_vec(item)
            .context("Could not serialize Google Jobs dataset item")?
            .len();
        if item_bytes + 2 > MAX_DATASET_ITEM_BYTES {
            bail!(
                "Google Jobs dataset item exceeds the {} byte Apify item limit",
                MAX_DATASET_ITEM_BYTES
            );
        }
        let separator_bytes = usize::from(!batch.is_empty());
        if !batch.is_empty() && batch_bytes + separator_bytes + item_bytes > MAX_DATASET_BATCH_BYTES {
            batches.push(std::mem::take(&mut batch));
            batch_bytes = 2;
        }
        let separator_bytes = usize::from(!batch.is_empty());
        batch_bytes += separator_bytes + item_bytes;
        batch.push(item.clone());
    }
    if !batch.is_empty() {
        batches.push(batch);
    }
    Ok(batches)
}

fn affordable_dataset_items(run: &Value, requested: usize) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        return Ok(requested);
    }

    let Some(max_total_charge) = data.pointer("/options/maxTotalChargeUsd") else {
        return Ok(requested);
    };
    if max_total_charge.is_null() {
        return Ok(requested);
    }
    let max_total_charge = max_total_charge
        .as_f64()
        .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
    if !max_total_charge.is_finite() || max_total_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    if max_total_charge == 0.0 {
        return Ok(requested);
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = event_price(events, DATASET_ITEM_EVENT)?;
    if !item_price.is_finite() || item_price < 0.0 {
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
        let price = event_price(events, event_name)?;
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_total_charge.max(1.0);
    let affordable = ((max_total_charge - spent + tolerance) / item_price)
        .floor()
        .max(0.0);
    Ok(requested.min(affordable.min(usize::MAX as f64) as usize))
}

fn event_price(events: &Map<String, Value>, event_name: &str) -> Result<f64> {
    let price = events
        .get(event_name)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the price for event {event_name}"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("Invalid price for charged event {event_name}");
    }
    Ok(price)
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let status_code = status.as_u16();
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        format!("HTTP {status_code}")
    } else {
        body
    };
    bail!("{operation} failed with {status_code} {reason}: {detail}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockResponse, MockServer};
    use serde_json::json;
    use std::time::Duration;

    fn pricing_run(max_total_charge: Value, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-default-dataset-item": { "eventPriceUsd": 0.01 },
                            "apify-actor-start": { "eventPriceUsd": 0.02 }
                        }
                    }
                },
                "options": { "maxTotalChargeUsd": max_total_charge },
                "chargedEventCounts": counts
            }
        })
    }

    #[test]
    fn respects_positive_spending_limit_and_charges_from_other_events() {
        let run = pricing_run(json!(0.05), json!({ "apify-actor-start": 2 }));
        assert_eq!(affordable_dataset_items(&run, 10).unwrap(), 1);
    }

    #[test]
    fn treats_missing_null_and_zero_spending_limits_as_unlimited() {
        let mut missing = pricing_run(Value::Null, json!({}));
        missing["data"]["options"] = json!({});
        assert_eq!(affordable_dataset_items(&missing, 7).unwrap(), 7);

        let null = pricing_run(Value::Null, json!({}));
        assert_eq!(affordable_dataset_items(&null, 7).unwrap(), 7);

        let zero = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "apify-default-dataset-item": { "eventPriceUsd": 0.01 },
                        "apify-actor-start": { "eventPriceUsd": 0.02 }
                    }}
                },
                "options": { "maxTotalChargeUsd": 0.0 },
                "chargedEventCounts": { "apify-actor-start": 1 }
            }
        });
        assert_eq!(affordable_dataset_items(&zero, 7).unwrap(), 7);
    }

    #[test]
    fn returns_all_dataset_items_for_non_ppe_runs() {
        for pricing_model in ["FREE", "FIXED_PRICE"] {
            let run = json!({ "data": { "pricingInfo": { "pricingModel": pricing_model } } });
            assert_eq!(affordable_dataset_items(&run, 7).unwrap(), 7);
        }
    }

    #[test]
    fn rejects_missing_prices_and_invalid_counts_for_positive_caps() {
        let no_dataset_price = json!({
            "data": {
                "pricingInfo": { "pricingModel": "PAY_PER_EVENT", "pricingPerEvent": { "actorChargeEvents": {} } },
                "options": { "maxTotalChargeUsd": 1.0 },
                "chargedEventCounts": {}
            }
        });
        assert!(affordable_dataset_items(&no_dataset_price, 1).is_err());

        let invalid_count = pricing_run(json!(1.0), json!({ "apify-actor-start": -1 }));
        assert!(affordable_dataset_items(&invalid_count, 1).is_err());
    }

    #[tokio::test]
    async fn uses_apify_auth_and_writes_dataset_rows_and_full_output() {
        let server = MockServer::start(vec![
            MockResponse::json(200, r#"{"q":"nurse jobs"}"#),
            MockResponse::json(
                200,
                r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.01}}}},"options":{"maxTotalChargeUsd":1.0},"chargedEventCounts":{}}}"#,
            ),
            MockResponse::json(201, "{}"),
            MockResponse::json(201, "{}"),
        ]);
        let base_url = Url::parse(&server.base_url()).unwrap();
        let client = ApifyClient::new(
            Client::builder().timeout(Duration::from_secs(1)).build().unwrap(),
            base_url,
            "apify-test-token".to_owned(),
            "store-id".to_owned(),
            "dataset-id".to_owned(),
            "run-id".to_owned(),
            "INPUT".to_owned(),
        );
        assert_eq!(client.get_input().await.unwrap().unwrap()["q"], "nurse jobs");
        assert_eq!(client.dataset_item_limit(2).await.unwrap(), 2);
        let jobs = vec![json!({ "title": "Nurse" }), json!({ "title": "RN" })];
        assert_eq!(client.push_dataset_items(&jobs).await.unwrap(), 2);
        let output = json!({ "jobs": jobs });
        client.set_output(&output).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests[0].starts_with("GET /v2/key-value-stores/store-id/records/INPUT HTTP/1.1"));
        assert!(requests[1].starts_with("GET /v2/actor-runs/run-id HTTP/1.1"));
        assert!(requests[2].starts_with("POST /v2/datasets/dataset-id/items HTTP/1.1"));
        assert!(requests[3].starts_with("PUT /v2/key-value-stores/store-id/records/OUTPUT HTTP/1.1"));
        assert!(requests
            .iter()
            .all(|request| request.to_ascii_lowercase().contains("authorization: bearer apify-test-token")));
        assert!(requests[2].contains(r#"[{"title":"Nurse"},{"title":"RN"}]"#));
        assert!(requests[3].contains(r#"{"jobs":[{"title":"Nurse"},{"title":"RN"}]}"#));
    }

    #[tokio::test]
    async fn writes_dataset_rows_and_output_for_non_ppe_runs_without_custom_charges() {
        let server = MockServer::start(vec![
            MockResponse::json(200, r#"{"data":{"pricingInfo":{"pricingModel":"FREE"}}}"#),
            MockResponse::json(201, "{}"),
            MockResponse::json(201, "{}"),
        ]);
        let client = ApifyClient::new(
            Client::builder().timeout(Duration::from_secs(1)).build().unwrap(),
            Url::parse(&server.base_url()).unwrap(),
            "apify-test-token".to_owned(),
            "store-id".to_owned(),
            "dataset-id".to_owned(),
            "run-id".to_owned(),
            "INPUT".to_owned(),
        );
        let jobs = vec![json!({ "title": "Nurse" }), json!({ "title": "RN" })];
        let output = json!({ "jobs": jobs });

        let allowed_items = client.dataset_item_limit(jobs.len()).await.unwrap();
        assert_eq!(allowed_items, jobs.len());
        assert_eq!(client.push_dataset_items(&jobs[..allowed_items]).await.unwrap(), 2);
        client.set_output(&output).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].starts_with("GET /v2/actor-runs/run-id HTTP/1.1"));
        assert!(requests[1].starts_with("POST /v2/datasets/dataset-id/items HTTP/1.1"));
        assert!(requests[2].starts_with("PUT /v2/key-value-stores/store-id/records/OUTPUT HTTP/1.1"));
        assert!(requests.iter().all(|request| !request.contains("/charges")));
        assert!(requests[1].contains(r#"[{"title":"Nurse"},{"title":"RN"}]"#));
        assert!(requests[2].contains(r#"{"jobs":[{"title":"Nurse"},{"title":"RN"}]}"#));
    }

    #[test]
    fn splits_large_dataset_writes_and_rejects_oversized_rows() {
        let items = vec![
            json!({ "text": "a".repeat(2_100_000) }),
            json!({ "text": "b".repeat(2_100_000) }),
        ];
        let batches = dataset_batches(&items).unwrap();
        assert_eq!(batches.len(), 2);
        assert!(batches.iter().all(|batch| batch.len() == 1));

        let oversized = vec![json!({ "text": "x".repeat(MAX_DATASET_ITEM_BYTES) })];
        assert!(dataset_batches(&oversized).is_err());
    }
}
