#[cfg(test)]
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response};
use serde_json::{json, Map, Value};
use url::Url;

const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub struct ApifyClient {
    http: Client,
    base_url: Url,
    token: String,
    run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PushResult {
    pub charged_count: usize,
    pub event_charge_limit_reached: bool,
}

struct DatasetState {
    item_count: usize,
    items: Vec<Value>,
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
        }
    }

    pub async fn get_input(&self) -> Result<Value> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status().as_u16() == 404 {
            return Ok(json!({}));
        }
        response_json(response, "Apify INPUT request").await
    }

    pub async fn push_data(&self, items: &[Value], event_name: &str) -> Result<PushResult> {
        let mut run = self.get_run().await?;
        let dataset = self.get_dataset_state(items.len()).await?;
        if dataset.item_count > items.len() {
            bail!(
                "Apify dataset contains {} rows but the Scrappa response has only {}",
                dataset.item_count,
                items.len()
            );
        }
        if dataset.items != items[..dataset.item_count] {
            bail!("Apify dataset rows do not match the Scrappa response prefix");
        }

        if items.is_empty() {
            if is_pay_per_event(&run) && charged_event_count(&run, event_name)? > 0 {
                bail!("Apify has charged for {event_name} events but the Scrappa response has no rows");
            }
            return Ok(PushResult {
                charged_count: 0,
                event_charge_limit_reached: false,
            });
        }

        if !is_pay_per_event(&run) {
            if dataset.item_count < items.len() {
                self.write_dataset_items(&items[dataset.item_count..])
                    .await?;
            }
            return Ok(PushResult {
                charged_count: items.len(),
                event_charge_limit_reached: false,
            });
        }

        let mut dataset_count = dataset.item_count;
        let mut charged_count = charged_event_count(&run, event_name)?;
        if charged_count > items.len() {
            bail!(
                "Apify has charged for {charged_count} {event_name} events but the Scrappa response has only {} rows",
                items.len()
            );
        }

        let mut refresh_run = false;
        if dataset_count > charged_count {
            let uncharged_count = dataset_count - charged_count;
            let affordable = affordable_event_charges(&run, event_name, uncharged_count)?;
            if affordable > 0 {
                let range_end = charged_count + affordable;
                self.charge_event(event_name, charged_count, range_end)
                    .await?;
                charged_count = range_end;
                refresh_run = true;
            }
            if charged_count < dataset_count {
                return Ok(PushResult {
                    charged_count,
                    event_charge_limit_reached: true,
                });
            }
        }

        if charged_count > dataset_count {
            self.write_dataset_items(&items[dataset_count..charged_count])
                .await?;
            dataset_count = charged_count;
            refresh_run = true;
        }

        if dataset_count == items.len() {
            return Ok(PushResult {
                charged_count: dataset_count,
                event_charge_limit_reached: false,
            });
        }

        if refresh_run {
            run = self.get_run().await?;
        }
        let limit = affordable_event_items(&run, event_name, items.len() - dataset_count)?;
        if limit > 0 {
            let range_end = dataset_count + limit;
            self.charge_event(event_name, dataset_count, range_end)
                .await?;
            self.write_dataset_items(&items[dataset_count..range_end])
                .await?;
        }
        Ok(PushResult {
            charged_count: dataset_count + limit,
            event_charge_limit_reached: limit < items.len() - dataset_count,
        })
    }

    pub async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }

    pub async fn set_terminal_status_message(&self, status_message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.run_id])?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(&json!({
                "runId": self.run_id,
                "statusMessage": status_message,
                "isStatusMessageTerminal": true
            }))
            .send()
            .await
            .context("Apify run status update failed")?;
        ensure_success(response, "Apify run status update").await
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.run_id])?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        response_json(response, "Apify run pricing request").await
    }

    async fn get_dataset_state(&self, requested_items: usize) -> Result<DatasetState> {
        let mut url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        url.query_pairs_mut()
            .append_pair("limit", &requested_items.max(1).to_string());
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify dataset read failed")?;
        if !response.status().is_success() {
            let _ = response_json(response, "Apify dataset read").await?;
            bail!("Apify dataset read unexpectedly succeeded with an error status");
        }
        let item_count = response
            .headers()
            .get("x-apify-pagination-total")
            .context("Apify dataset read did not include its total item count")?
            .to_str()
            .context("Apify dataset read returned an invalid item count")?
            .parse::<usize>()
            .context("Apify dataset read returned an invalid item count")?;
        let value = response_json(response, "Apify dataset read").await?;
        let items = value
            .as_array()
            .context("Apify dataset read returned a non-array body")?
            .clone();
        if items.len() > item_count {
            bail!("Apify dataset read returned more rows than its total item count");
        }
        Ok(DatasetState { item_count, items })
    }

    async fn charge_event(
        &self,
        event_name: &str,
        range_start: usize,
        range_end: usize,
    ) -> Result<()> {
        let count = range_end - range_start;
        let url = self.endpoint(&["v2", "actor-runs", &self.run_id, "charge"])?;
        let idempotency_key = format!("{}-{event_name}-{range_start}-{range_end}", self.run_id);
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .header("idempotency-key", idempotency_key)
            .json(&json!({ "eventName": event_name, "count": count }))
            .send()
            .await
            .context("Apify event charge request failed")?;
        ensure_success(response, "Apify event charge").await
    }

    async fn write_dataset_items(&self, items: &[Value]) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("Apify API base URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(segments.iter().copied());
        Ok(url)
    }
}

pub fn is_pay_per_event(run: &Value) -> bool {
    run.pointer("/data/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        == Some("PAY_PER_EVENT")
}

pub fn affordable_event_items(run: &Value, event_name: &str, requested: usize) -> Result<usize> {
    affordable_event_count(run, event_name, requested, true)
}

fn affordable_event_charges(run: &Value, event_name: &str, requested: usize) -> Result<usize> {
    affordable_event_count(run, event_name, requested, false)
}

fn charged_event_count(run: &Value, event_name: &str) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let Some(value) = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .and_then(|counts| counts.get(event_name))
    else {
        return Ok(0);
    };
    let count = value
        .as_u64()
        .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
    usize::try_from(count).context("Apify returned a charged event count that is too large")
}

fn affordable_event_count(
    run: &Value,
    event_name: &str,
    requested: usize,
    include_dataset_item_charge: bool,
) -> Result<usize> {
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
    let market_event_price = event_price(events, event_name)?;
    let dataset_item_price = if include_dataset_item_charge {
        optional_event_price(events, DATASET_ITEM_EVENT)?
    } else {
        0.0
    };
    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .filter(|value| !value.is_null())
        .map(|value| {
            value
                .as_f64()
                .ok_or_else(|| anyhow!("Apify run returned invalid spending limit"))
        })
        .transpose()?
        .unwrap_or(f64::INFINITY);
    if !max_charge.is_finite() && max_charge != f64::INFINITY || max_charge < 0.0 {
        bail!("Apify run returned invalid spending limit");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0f64;
    for (charged_event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {charged_event_name}"))?;
        if count == 0 {
            continue;
        }
        let price = event_price(events, charged_event_name)?;
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }

    let per_item_price = market_event_price + dataset_item_price;
    if per_item_price == 0.0 || max_charge == f64::INFINITY {
        return Ok(requested);
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let remaining = max_charge - spent + tolerance;
    if remaining <= 0.0 {
        return Ok(0);
    }
    let affordable = (remaining / per_item_price).floor();
    if !affordable.is_finite() || affordable >= requested as f64 {
        return Ok(requested);
    }
    Ok(affordable.max(0.0) as usize)
}

fn event_price(events: &Map<String, Value>, event_name: &str) -> Result<f64> {
    let Some(event) = events.get(event_name) else {
        bail!("Apify run did not provide the {event_name} event price");
    };
    let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) else {
        bail!("Apify run did not provide the {event_name} event price");
    };
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned invalid price for charged event {event_name}");
    }
    Ok(price)
}

fn optional_event_price(events: &Map<String, Value>, event_name: &str) -> Result<f64> {
    let Some(event) = events.get(event_name) else {
        return Ok(0.0);
    };
    let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) else {
        bail!("Apify run did not provide the {event_name} event price");
    };
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned invalid price for charged event {event_name}");
    }
    Ok(price)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read Apify API response")?;
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
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
    serde_json::from_str(&body).with_context(|| format!("{operation} returned invalid JSON"))
}

async fn ensure_success(response: Response, operation: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockResponse, MockServer};

    fn run(max_total_charge: Value, charged_events: Value, events: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": events }
                },
                "options": { "maxTotalChargeUsd": max_total_charge },
                "chargedEventCounts": charged_events
            }
        })
    }

    fn prices() -> Value {
        json!({
            "market-item": { "eventPriceUsd": 0.1 },
            "apify-default-dataset-item": { "eventPriceUsd": 0.2 },
            "apify-actor-start": { "eventPriceUsd": 0.5 }
        })
    }

    #[test]
    fn budget_includes_custom_result_and_automatic_dataset_item_charges() {
        let run = run(
            json!(2.0),
            json!({
                "apify-actor-start": 1,
                "apify-default-dataset-item": 1
            }),
            prices(),
        );
        // $0.7 already charged; each result costs $0.1 for market-item and $0.2 for its dataset row.
        assert_eq!(affordable_event_items(&run, "market-item", 10).unwrap(), 4);
    }

    #[test]
    fn allows_all_rows_when_the_dataset_synthetic_event_is_disabled() {
        let run = run(
            json!(1.0),
            json!({}),
            json!({ "market-item": { "eventPriceUsd": 0.1 } }),
        );
        assert_eq!(affordable_event_items(&run, "market-item", 8).unwrap(), 8);
    }

    #[test]
    fn validates_pricing_and_count_metadata_before_writing() {
        let missing_event = run(json!(1.0), json!({}), json!({}));
        assert!(affordable_event_items(&missing_event, "market-item", 1)
            .unwrap_err()
            .to_string()
            .contains("market-item event price"));

        let invalid_count = run(json!(1.0), json!({ "apify-actor-start": -1 }), prices());
        assert!(affordable_event_items(&invalid_count, "market-item", 1).is_err());
    }

    #[tokio::test]
    async fn reads_input_and_defaults_missing_input_to_an_empty_object() {
        let server = MockServer::start(vec![MockResponse::json(404, r#"{"error":"not found"}"#)]);
        let client = test_client(&server.base_url());
        assert_eq!(client.get_input().await.unwrap(), json!({}));
        let requests = server.join();
        assert_eq!(
            requests[0].target,
            "/v2/key-value-stores/store-id/records/INPUT"
        );
        assert_eq!(requests[0].headers["authorization"], "Bearer test-token");
    }

    #[tokio::test]
    async fn includes_run_id_when_setting_terminal_status_message() {
        let server = MockServer::start(vec![MockResponse::json(200, "")]);
        let client = test_client(&server.base_url());
        let status_message = "Charge limit reached before saving all items.";

        client
            .set_terminal_status_message(status_message)
            .await
            .unwrap();

        let requests = server.join();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "PUT");
        assert_eq!(requests[0].target, "/v2/actor-runs/test-run");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[0].body).unwrap(),
            json!({
                "runId": "test-run",
                "statusMessage": status_message,
                "isStatusMessageTerminal": true
            })
        );
    }

    #[tokio::test]
    async fn pushes_non_ppe_rows_without_custom_charge() {
        let run = json!({ "data": { "pricingInfo": { "pricingModel": "PAY_PER_RESULT" } } });
        let server = MockServer::start(vec![
            MockResponse::json(200, &run.to_string()),
            empty_dataset(),
            MockResponse::json(201, ""),
        ]);
        let client = test_client(&server.base_url());
        let items = vec![json!({ "position": 1 }), json!({ "position": 2 })];
        let result = client.push_data(&items, "market-item").await.unwrap();
        assert_eq!(
            result,
            PushResult {
                charged_count: 2,
                event_charge_limit_reached: false
            }
        );
        let requests = server.join();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].target, "/v2/actor-runs/test-run");
        assert_eq!(requests[1].method, "GET");
        assert_eq!(requests[1].target, "/v2/datasets/dataset-id/items?limit=2");
        assert_eq!(requests[2].target, "/v2/datasets/dataset-id/items");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[2].body).unwrap(),
            json!(items)
        );
    }

    #[tokio::test]
    async fn rejects_an_empty_scrappa_response_when_dataset_rows_already_exist() {
        let run = json!({ "data": { "pricingInfo": { "pricingModel": "PAY_PER_RESULT" } } });
        let server = MockServer::start(vec![
            MockResponse::json(200, &run.to_string()),
            MockResponse::json(200, r#"[{"position":1}]"#)
                .with_header("X-Apify-Pagination-Total", "1"),
        ]);
        let client = test_client(&server.base_url());

        let error = client
            .push_data(&[], "market-item")
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("dataset contains 1 rows"));
        let requests = server.join();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].method, "GET");
        assert_eq!(requests[1].target, "/v2/datasets/dataset-id/items?limit=1");
    }

    #[tokio::test]
    async fn resumes_a_charged_range_after_dataset_write_failure() {
        let uncharged_run = run(json!(2.0), json!({}), prices());
        let charged_run = run(json!(2.0), json!({ "market-item": 1 }), prices());
        let server = MockServer::start(vec![
            MockResponse::json(200, &uncharged_run.to_string()),
            empty_dataset(),
            MockResponse::json(200, ""),
            MockResponse::json(500, r#"{"error":"dataset unavailable"}"#),
            MockResponse::json(200, &charged_run.to_string()),
            empty_dataset(),
            MockResponse::json(201, ""),
        ]);
        let client = test_client(&server.base_url());
        let items = vec![json!({ "position": 1 })];

        assert!(client.push_data(&items, "market-item").await.is_err());
        let result = client.push_data(&items, "market-item").await.unwrap();
        assert_eq!(
            result,
            PushResult {
                charged_count: 1,
                event_charge_limit_reached: false
            }
        );

        let requests = server.join();
        assert_eq!(requests.len(), 7);
        assert_eq!(requests[2].target, "/v2/actor-runs/test-run/charge");
        assert_eq!(
            requests[2].headers["idempotency-key"],
            "test-run-market-item-0-1"
        );
        assert_eq!(requests[3].method, "POST");
        assert_eq!(requests[3].target, "/v2/datasets/dataset-id/items");
        assert_eq!(requests[3].body, r#"[{"position":1}]"#);
        assert_eq!(requests[6].method, "POST");
        assert_eq!(requests[6].target, "/v2/datasets/dataset-id/items");
        assert_eq!(requests[6].body, r#"[{"position":1}]"#);
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.target == "/v2/actor-runs/test-run/charge")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn charges_the_affordable_range_before_writing_dataset_items() {
        let run = run(json!(1.7), json!({ "apify-actor-start": 1 }), prices());
        let server = MockServer::start(vec![
            MockResponse::json(200, &run.to_string()),
            empty_dataset(),
            MockResponse::json(200, ""),
            MockResponse::json(201, ""),
        ]);
        let client = test_client(&server.base_url());
        let items = (1..=6)
            .map(|position| json!({ "position": position }))
            .collect::<Vec<_>>();
        let result = client.push_data(&items, "market-item").await.unwrap();
        assert_eq!(
            result,
            PushResult {
                charged_count: 4,
                event_charge_limit_reached: true
            }
        );
        let requests = server.join();
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[1].method, "GET");
        assert_eq!(requests[1].target, "/v2/datasets/dataset-id/items?limit=6");
        assert_eq!(requests[2].target, "/v2/actor-runs/test-run/charge");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[2].body).unwrap(),
            json!({ "eventName": "market-item", "count": 4 })
        );
        assert_eq!(
            requests[2].headers["idempotency-key"],
            "test-run-market-item-0-4"
        );
        assert_eq!(requests[3].method, "POST");
        assert_eq!(requests[3].target, "/v2/datasets/dataset-id/items");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[3].body).unwrap(),
            json!([
                { "position": 1 },
                { "position": 2 },
                { "position": 3 },
                { "position": 4 }
            ])
        );
    }

    #[tokio::test]
    async fn resumes_rows_written_before_market_event_charge_after_restart() {
        let run = run(
            json!(2.0),
            json!({
                "apify-actor-start": 1,
                "apify-default-dataset-item": 4
            }),
            prices(),
        );
        let server = MockServer::start(vec![
            MockResponse::json(200, &run.to_string()),
            MockResponse::json(
                200,
                r#"[{"position":1},{"position":2},{"position":3},{"position":4}]"#,
            )
            .with_header("X-Apify-Pagination-Total", "4"),
            MockResponse::json(200, ""),
        ]);
        let client = test_client(&server.base_url());
        let items = (1..=4)
            .map(|position| json!({ "position": position }))
            .collect::<Vec<_>>();

        let result = client.push_data(&items, "market-item").await.unwrap();

        assert_eq!(
            result,
            PushResult {
                charged_count: 4,
                event_charge_limit_reached: false
            }
        );
        let requests = server.join();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].target, "/v2/actor-runs/test-run");
        assert_eq!(requests[1].method, "GET");
        assert_eq!(requests[1].target, "/v2/datasets/dataset-id/items?limit=4");
        assert_eq!(requests[2].target, "/v2/actor-runs/test-run/charge");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[2].body).unwrap(),
            json!({ "eventName": "market-item", "count": 4 })
        );
        assert_eq!(
            requests[2].headers["idempotency-key"],
            "test-run-market-item-0-4"
        );
        assert!(requests.iter().all(|request| {
            !(request.method == "POST" && request.target == "/v2/datasets/dataset-id/items")
        }));
    }

    #[tokio::test]
    async fn charges_affordable_part_of_an_existing_unbilled_range() {
        let run = run(
            json!(1.5),
            json!({
                "apify-actor-start": 1,
                "apify-default-dataset-item": 4
            }),
            prices(),
        );
        let existing_items = (1..=4)
            .map(|position| json!({ "position": position }))
            .collect::<Vec<_>>();
        let server = MockServer::start(vec![
            MockResponse::json(200, &run.to_string()),
            MockResponse::json(200, &serde_json::to_string(&existing_items).unwrap())
                .with_header("X-Apify-Pagination-Total", "4"),
            MockResponse::json(200, ""),
        ]);
        let client = test_client(&server.base_url());

        let result = client
            .push_data(&existing_items, "market-item")
            .await
            .unwrap();

        assert_eq!(
            result,
            PushResult {
                charged_count: 2,
                event_charge_limit_reached: true
            }
        );
        let requests = server.join();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[2].target, "/v2/actor-runs/test-run/charge");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[2].body).unwrap(),
            json!({ "eventName": "market-item", "count": 2 })
        );
        assert_eq!(
            requests[2].headers["idempotency-key"],
            "test-run-market-item-0-2"
        );
    }

    fn empty_dataset() -> MockResponse {
        MockResponse::json(200, "[]").with_header("X-Apify-Pagination-Total", "0")
    }

    #[tokio::test]
    async fn writes_raw_output_to_the_default_key_value_store() {
        let server = MockServer::start(vec![MockResponse::json(200, "")]);
        let client = test_client(&server.base_url());
        let output = json!({ "markets": { "us": [{ "stock": "AAPL" }] } });
        client.put_output(&output).await.unwrap();
        let requests = server.join();
        assert_eq!(requests[0].method, "PUT");
        assert_eq!(
            requests[0].target,
            "/v2/key-value-stores/store-id/records/OUTPUT"
        );
        assert_eq!(
            serde_json::from_str::<Value>(&requests[0].body).unwrap(),
            output
        );
    }

    fn test_client(base_url: &str) -> ApifyClient {
        let mut base_url = Url::parse(base_url).unwrap();
        base_url.set_path("");
        ApifyClient::new(
            Client::builder()
                .timeout(Duration::from_secs(1))
                .build()
                .unwrap(),
            base_url,
            "test-token".to_owned(),
            "test-run".to_owned(),
            "store-id".to_owned(),
            "dataset-id".to_owned(),
            "INPUT".to_owned(),
        )
    }
}
