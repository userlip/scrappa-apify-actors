use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response};
use serde_json::{json, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const API_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_DATASET_REQUEST_BYTES: usize = 5_000_000;
pub const FLIGHT_RESULT_CHARGE_EVENT: &str = "flight-result";
const DEFAULT_DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";

pub struct Config {
    apify_api_base: Url,
    scrappa_api_base: Url,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    pub scrappa_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        let apify_base = env::var("APIFY_API_PUBLIC_BASE_URL")
            .or_else(|_| env::var("APIFY_API_BASE_URL"))
            .unwrap_or_else(|_| APIFY_API_DEFAULT.to_owned());
        let scrappa_base = env::var("SCRAPPA_API_BASE_URL")
            .unwrap_or_else(|_| "https://scrappa.co/api".to_owned());
        Ok(Self {
            apify_api_base: Url::parse(&apify_base)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid absolute URL")?,
            scrappa_api_base: Url::parse(&scrappa_base)
                .context("SCRAPPA_API_BASE_URL must be a valid absolute URL")?,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key,
        })
    }

    pub fn scrappa_api_base(&self) -> &Url {
        &self.scrappa_api_base
    }
}

pub struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

pub struct PushDataResult {
    pub saved_count: usize,
    pub event_charge_limit_reached: bool,
}

impl<'a> ApifyClient<'a> {
    pub fn new(http: &'a Client, config: &'a Config) -> Self {
        Self { http, config }
    }

    pub async fn get_input(&self) -> Result<Value> {
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
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        response_json(response, "Apify INPUT request").await
    }

    pub async fn push_dataset_items(&self, items: &[Value]) -> Result<PushDataResult> {
        if items.is_empty() {
            return Ok(PushDataResult {
                saved_count: 0,
                event_charge_limit_reached: false,
            });
        }
        let run = self.get_run().await?;
        let is_pay_per_event = run
            .pointer("/data/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        let save_count = if is_pay_per_event {
            affordable_result_count(&run, FLIGHT_RESULT_CHARGE_EVENT, items.len())?
        } else {
            items.len()
        };
        if is_pay_per_event && save_count == 0 {
            return Ok(PushDataResult {
                saved_count: 0,
                event_charge_limit_reached: true,
            });
        }

        let mut saved_count = 0;
        for range in dataset_item_batch_ranges(&items[..save_count])? {
            let batch = &items[range.clone()];
            self.store_dataset_items(batch).await?;
            if is_pay_per_event {
                let idempotency_key = format!(
                    "{}:{FLIGHT_RESULT_CHARGE_EVENT}:results:{}:count:{}",
                    self.config.actor_run_id,
                    range.start + 1,
                    batch.len()
                );
                self.charge_events(FLIGHT_RESULT_CHARGE_EVENT, batch.len(), &idempotency_key)
                    .await?;
            }
            saved_count += batch.len();
        }

        Ok(PushDataResult {
            saved_count,
            event_charge_limit_reached: is_pay_per_event && saved_count < items.len(),
        })
    }

    pub async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .http
            .put(url)
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }

    pub async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .put(url)
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(&json!({
                "runId": self.config.actor_run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true,
            }))
            .send()
            .await
            .context("Apify run status message request failed")?;
        ensure_success(response, "Apify run status message request").await
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .get(url)
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        response_json(response, "Apify run pricing request").await
    }

    async fn charge_events(
        &self,
        event_name: &str,
        count: usize,
        idempotency_key: &str,
    ) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        let response = self
            .http
            .post(url)
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .header("idempotency-key", idempotency_key)
            .json(&json!({
                "eventName": event_name,
                "count": count,
            }))
            .send()
            .await
            .context("Apify event charge request failed")?;
        ensure_success(response, "Apify event charge request").await
    }

    async fn store_dataset_items(&self, items: &[Value]) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let body = if items.len() == 1 {
            let item_body = serde_json::to_vec(&items[0])
                .context("Failed to serialize Apify dataset item")?;
            if item_body.len() > MAX_DATASET_REQUEST_BYTES {
                bail!(
                    "Apify dataset item exceeds the {MAX_DATASET_REQUEST_BYTES} byte request limit"
                );
            }
            if item_body.len() + 2 > MAX_DATASET_REQUEST_BYTES {
                item_body
            } else {
                serde_json::to_vec(items).context("Failed to serialize Apify dataset items")?
            }
        } else {
            serde_json::to_vec(items).context("Failed to serialize Apify dataset items")?
        };
        if body.len() > MAX_DATASET_REQUEST_BYTES {
            bail!(
                "Apify dataset items request exceeds the {MAX_DATASET_REQUEST_BYTES} byte request limit"
            );
        }
        let response = self
            .http
            .post(url)
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
    }
}

fn dataset_item_batch_ranges(items: &[Value]) -> Result<Vec<std::ops::Range<usize>>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut current_bytes: usize = 2;

    for (index, item) in items.iter().enumerate() {
        let item_bytes = serde_json::to_vec(item)?.len();
        if item_bytes > MAX_DATASET_REQUEST_BYTES {
            bail!(
                "Dataset item {} exceeds Apify's {}-byte request limit",
                index + 1,
                MAX_DATASET_REQUEST_BYTES
            );
        }
        let single_item_bytes = item_bytes.saturating_add(2);
        if single_item_bytes > MAX_DATASET_REQUEST_BYTES {
            if start < index {
                ranges.push(start..index);
            }
            ranges.push(index..index + 1);
            start = index + 1;
            current_bytes = 2;
            continue;
        }

        let separator_bytes = usize::from(index > start);
        let next_bytes = current_bytes
            .saturating_add(separator_bytes)
            .saturating_add(item_bytes);
        if next_bytes > MAX_DATASET_REQUEST_BYTES {
            ranges.push(start..index);
            start = index;
            current_bytes = single_item_bytes;
        } else {
            current_bytes = next_bytes;
        }
    }

    if start < items.len() {
        ranges.push(start..items.len());
    }
    Ok(ranges)
}

pub fn affordable_result_count(run: &Value, event_name: &str, requested: usize) -> Result<usize> {
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
    let result_event_price = required_event_price(events, event_name)?;
    let dataset_item_price = optional_event_price(events, DEFAULT_DATASET_ITEM_CHARGE_EVENT)?;
    let price_per_result = result_event_price + dataset_item_price;
    if !price_per_result.is_finite() {
        bail!("Apify run returned invalid charging values");
    }
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => f64::INFINITY,
        Some(value) => {
            let value = value
                .as_f64()
                .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
            if value == 0.0 {
                f64::INFINITY
            } else {
                value
            }
        }
    };
    if max_charge.is_nan() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    let empty_counts = serde_json::Map::new();
    let counts = match data.get("chargedEventCounts") {
        None | Some(Value::Null) => &empty_counts,
        Some(Value::Object(counts)) => counts,
        Some(_) => bail!("Apify run returned invalid charged event counts"),
    };
    let mut spent = 0.0;
    for (charged_event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {charged_event_name}"))?;
        if count == 0 {
            continue;
        }
        let price = optional_event_price(events, charged_event_name)?;
        if !price.is_finite() {
            bail!("Invalid price for charged event {charged_event_name}");
        }
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if price_per_result == 0.0 || max_charge.is_infinite() {
        return Ok(requested);
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * price_per_result <= max_charge + tolerance)
        .count())
}

fn required_event_price(events: &serde_json::Map<String, Value>, event_name: &str) -> Result<f64> {
    let event = events
        .get(event_name)
        .ok_or_else(|| anyhow!("Apify run did not provide the {event_name} event price"))?;
    let price = event
        .get("eventPriceUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the {event_name} event price"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned an invalid price for {event_name}");
    }
    Ok(price)
}

fn optional_event_price(events: &serde_json::Map<String, Value>, event_name: &str) -> Result<f64> {
    let Some(event) = events.get(event_name) else {
        return Ok(0.0);
    };
    let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) else {
        return Ok(0.0);
    };
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned an invalid price for {event_name}");
    }
    Ok(price)
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read API response")?;
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

    fn run(max_charge: Value, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "flight-result": {"eventPriceUsd": 0.2},
                            "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                            "actor-start": {"eventPriceUsd": 0.1}
                        }
                    }
                },
                "chargedEventCounts": counts,
                "options": {"maxTotalChargeUsd": max_charge}
            }
        })
    }

    #[test]
    fn positive_budget_includes_prior_charges_and_both_per_result_prices() {
        let run = run(json!(1.0), json!({"flight-result": 1, "actor-start": 1}));
        assert_eq!(
            affordable_result_count(&run, FLIGHT_RESULT_CHARGE_EVENT, 10).unwrap(),
            2
        );
    }

    #[test]
    fn zero_null_and_missing_budgets_mean_unlimited() {
        let mut missing_budget = run(json!(1.0), json!({}));
        missing_budget["data"]["options"]
            .as_object_mut()
            .unwrap()
            .remove("maxTotalChargeUsd");

        for (description, run) in [
            ("zero", run(json!(0.0), json!({}))),
            ("null", run(Value::Null, json!({}))),
            ("missing", missing_budget),
        ] {
            assert_eq!(
                affordable_result_count(&run, FLIGHT_RESULT_CHARGE_EVENT, 10).unwrap(),
                10,
                "{description} budget should mean unlimited"
            );
        }
    }

    #[test]
    fn missing_prices_and_charge_counts_fail_closed() {
        let mut missing_event = run(json!(1.0), json!({}));
        missing_event["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove(FLIGHT_RESULT_CHARGE_EVENT);
        assert!(affordable_result_count(&missing_event, FLIGHT_RESULT_CHARGE_EVENT, 1).is_err());

        let mut invalid_counts = run(json!(1.0), json!({}));
        invalid_counts["data"]["chargedEventCounts"] = json!("invalid");
        assert!(affordable_result_count(&invalid_counts, FLIGHT_RESULT_CHARGE_EVENT, 1).is_err());
    }

    #[tokio::test]
    async fn input_and_output_use_the_default_key_value_store_records() {
        let input = json!({
            "trip_type": "one_way",
            "origin": "JFK",
            "destination": "LAX",
            "departure_date": "2026-09-15"
        });
        let output = json!({"flights": [], "provider_field": "preserved"});
        let (base_url, server) = mock_server(vec![(200, input.to_string()), (201, String::new())]);
        let config = test_config(&base_url);
        let http = Client::new();
        let apify = ApifyClient::new(&http, &config);

        assert_eq!(apify.get_input().await.unwrap(), input);
        apify.put_output(&output).await.unwrap();

        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(
            requests[0].target,
            "/v2/key-value-stores/store-id/records/INPUT"
        );
        assert_eq!(requests[1].method, "PUT");
        assert_eq!(
            requests[1].target,
            "/v2/key-value-stores/store-id/records/OUTPUT"
        );
        assert_eq!(
            serde_json::from_str::<Value>(&requests[1].body).unwrap(),
            output
        );
        assert_authorized(&requests);
    }

    #[tokio::test]
    async fn pay_per_event_storage_charges_and_writes_only_affordable_results() {
        let run = json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "flight-result": {"eventPriceUsd": 0.2},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                    "apify-actor-start": {"eventPriceUsd": 0.05}
                }}
            },
            "chargedEventCounts": {"apify-actor-start": 1},
            "options": {"maxTotalChargeUsd": 0.75}
        }});
        let (base_url, server) = mock_server(vec![
            (200, run.to_string()),
            (201, String::new()),
            (200, "{}".to_owned()),
        ]);
        let config = test_config(&base_url);
        let http = Client::new();
        let apify = ApifyClient::new(&http, &config);
        let items = vec![
            json!({"position": 1}),
            json!({"position": 2}),
            json!({"position": 3}),
        ];

        let result = apify.push_dataset_items(&items).await.unwrap();

        assert_eq!(result.saved_count, 2);
        assert!(result.event_charge_limit_reached);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].target, "/v2/actor-runs/run-id");
        assert_eq!(requests[1].method, "POST");
        assert_eq!(requests[1].target, "/v2/datasets/dataset-id/items");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[1].body).unwrap(),
            json!([{"position": 1}, {"position": 2}])
        );
        assert_eq!(requests[2].method, "POST");
        assert_eq!(requests[2].target, "/v2/actor-runs/run-id/charge");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[2].body).unwrap(),
            json!({"eventName": "flight-result", "count": 2})
        );
        assert!(requests[2].headers.contains_key("idempotency-key"));
        assert_authorized(&requests);
    }

    #[tokio::test]
    async fn pay_per_event_dataset_writes_and_charges_each_size_limited_chunk() {
        use std::collections::HashSet;

        let run = json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "flight-result": {"eventPriceUsd": 0.2},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.1}
                }}
            },
            "chargedEventCounts": {},
            "options": {"maxTotalChargeUsd": 0}
        }});
        let mut responses = vec![(200, run.to_string())];
        for _ in 0..3 {
            responses.push((201, String::new()));
            responses.push((200, "{}".to_owned()));
        }
        let (base_url, server) = mock_server(responses);
        let config = test_config(&base_url);
        let http = Client::new();
        let apify = ApifyClient::new(&http, &config);
        let payload = "x".repeat(MAX_DATASET_REQUEST_BYTES / 2);
        let items = (1..=3)
            .map(|position| json!({"position": position, "payload": payload.clone()}))
            .collect::<Vec<_>>();

        let result = apify.push_dataset_items(&items).await.unwrap();

        assert_eq!(result.saved_count, items.len());
        assert!(!result.event_charge_limit_reached);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 7);
        assert_eq!(requests[0].target, "/v2/actor-runs/run-id");

        let mut saved_positions = Vec::new();
        let mut idempotency_keys = HashSet::new();
        let mut charged_count = 0;
        for chunk_index in 0..3 {
            let dataset_request = &requests[1 + chunk_index * 2];
            let charge_request = &requests[2 + chunk_index * 2];

            assert_eq!(dataset_request.method, "POST");
            assert_eq!(dataset_request.target, "/v2/datasets/dataset-id/items");
            assert!(dataset_request.body.len() <= MAX_DATASET_REQUEST_BYTES);
            let rows = serde_json::from_str::<Vec<Value>>(&dataset_request.body).unwrap();
            assert_eq!(rows.len(), 1);
            saved_positions.push(rows[0]["position"].as_u64().unwrap());

            assert_eq!(charge_request.method, "POST");
            assert_eq!(charge_request.target, "/v2/actor-runs/run-id/charge");
            let charge: Value = serde_json::from_str(&charge_request.body).unwrap();
            assert_eq!(charge["eventName"], FLIGHT_RESULT_CHARGE_EVENT);
            let count = charge["count"].as_u64().unwrap() as usize;
            assert_eq!(count, rows.len());
            charged_count += count;

            let idempotency_key = charge_request.headers.get("idempotency-key").unwrap();
            assert_eq!(
                idempotency_key,
                &format!(
                    "run-id:{FLIGHT_RESULT_CHARGE_EVENT}:results:{}:count:1",
                    chunk_index + 1
                )
            );
            assert!(idempotency_keys.insert(idempotency_key.clone()));
        }

        assert_eq!(saved_positions, vec![1, 2, 3]);
        assert_eq!(charged_count, items.len());
    }

    #[tokio::test]
    async fn near_limit_single_item_uses_object_payload_without_array_overflow() {
        let empty_item = json!({"position": 1, "payload": ""});
        let empty_item_size = serde_json::to_vec(&empty_item).unwrap().len();
        let items = vec![json!({
            "position": 1,
            "payload": "x".repeat(MAX_DATASET_REQUEST_BYTES - empty_item_size - 1)
        })];
        let item_size = serde_json::to_vec(&items[0]).unwrap().len();
        assert_eq!(item_size, MAX_DATASET_REQUEST_BYTES - 1);
        assert_eq!(dataset_item_batch_ranges(&items).unwrap(), vec![0..1]);

        let (base_url, server) = mock_server(vec![(201, String::new())]);
        let config = test_config(&base_url);
        let http = Client::new();
        let apify = ApifyClient::new(&http, &config);

        apify.store_dataset_items(&items).await.unwrap();

        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].target, "/v2/datasets/dataset-id/items");
        assert_eq!(requests[0].body.len(), MAX_DATASET_REQUEST_BYTES - 1);
        assert!(serde_json::from_str::<Value>(&requests[0].body)
            .unwrap()
            .is_object());
    }

    #[tokio::test]
    async fn non_pay_per_event_storage_keeps_all_result_counts_without_charging() {
        let run = json!({"data": {
            "pricingInfo": {"pricingModel": "DEVELOPER"}
        }});
        let (base_url, server) = mock_server(vec![
            (200, run.to_string()),
            (201, String::new()),
            (201, String::new()),
        ]);
        let config = test_config(&base_url);
        let http = Client::new();
        let apify = ApifyClient::new(&http, &config);
        let items = vec![
            json!({"position": 1, "payload": "x".repeat(3_000_000)}),
            json!({"position": 2, "payload": "y".repeat(3_000_000)}),
        ];

        let result = apify.push_dataset_items(&items).await.unwrap();

        assert_eq!(result.saved_count, items.len());
        assert!(!result.event_charge_limit_reached);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].target, "/v2/actor-runs/run-id");
        assert_eq!(requests[1].target, "/v2/datasets/dataset-id/items");
        assert_eq!(requests[2].target, "/v2/datasets/dataset-id/items");
        for (request, position) in [(&requests[1], 1), (&requests[2], 2)] {
            assert!(request.body.len() <= MAX_DATASET_REQUEST_BYTES);
            let chunk: Vec<Value> = serde_json::from_str(&request.body).unwrap();
            assert_eq!(chunk.len(), 1);
            assert_eq!(chunk[0]["position"], position);
        }
    }

    #[tokio::test]
    async fn pay_per_event_dataset_write_failure_does_not_charge_results() {
        let run = json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "flight-result": {"eventPriceUsd": 0.2},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.1}
                }}
            },
            "chargedEventCounts": {},
            "options": {"maxTotalChargeUsd": 1.0}
        }});
        let (base_url, server) = mock_server(vec![
            (200, run.to_string()),
            (500, "dataset rejected".to_owned()),
        ]);
        let config = test_config(&base_url);
        let http = Client::new();
        let apify = ApifyClient::new(&http, &config);

        assert!(apify
            .push_dataset_items(&[json!({"position": 1})])
            .await
            .is_err());

        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].target, "/v2/actor-runs/run-id");
        assert_eq!(requests[1].target, "/v2/datasets/dataset-id/items");
    }

    #[tokio::test]
    async fn ambiguous_dataset_write_failure_is_not_retried_or_charged() {
        let run = json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "flight-result": {"eventPriceUsd": 0.2},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.1}
                }}
            },
            "chargedEventCounts": {},
            "options": {"maxTotalChargeUsd": 1.0}
        }});
        let (base_url, server) = mock_server_with_replies(vec![Some((200, run.to_string())), None]);
        let config = test_config(&base_url);
        let http = Client::new();
        let apify = ApifyClient::new(&http, &config);

        assert!(apify
            .push_dataset_items(&[json!({"position": 1})])
            .await
            .is_err());

        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].target, "/v2/actor-runs/run-id");
        assert_eq!(requests[1].method, "POST");
        assert_eq!(requests[1].target, "/v2/datasets/dataset-id/items");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[1].body).unwrap(),
            json!([{"position": 1}])
        );
    }

    #[tokio::test]
    async fn pay_per_event_budget_exhaustion_skips_charge_and_dataset_write() {
        let run = json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "flight-result": {"eventPriceUsd": 0.2},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                    "apify-actor-start": {"eventPriceUsd": 0.05}
                }}
            },
            "chargedEventCounts": {"apify-actor-start": 1},
            "options": {"maxTotalChargeUsd": 0.05}
        }});
        let (base_url, server) = mock_server(vec![(200, run.to_string())]);
        let config = test_config(&base_url);
        let http = Client::new();
        let apify = ApifyClient::new(&http, &config);

        let result = apify
            .push_dataset_items(&[json!({"position": 1})])
            .await
            .unwrap();

        assert_eq!(result.saved_count, 0);
        assert!(result.event_charge_limit_reached);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].target, "/v2/actor-runs/run-id");
    }

    fn test_config(api_base: &str) -> Config {
        Config {
            apify_api_base: Url::parse(api_base).unwrap(),
            scrappa_api_base: Url::parse(api_base).unwrap(),
            apify_token: "apify-test-token".to_owned(),
            key_value_store_id: "store-id".to_owned(),
            dataset_id: "dataset-id".to_owned(),
            actor_run_id: "run-id".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
        }
    }

    fn assert_authorized(requests: &[CapturedRequest]) {
        assert!(requests.iter().all(|request| {
            request.headers.get("authorization").map(String::as_str)
                == Some("Bearer apify-test-token")
        }));
    }

    struct CapturedRequest {
        method: String,
        target: String,
        headers: std::collections::HashMap<String, String>,
        body: String,
    }

    fn mock_server(
        responses: Vec<(u16, String)>,
    ) -> (String, std::thread::JoinHandle<Vec<CapturedRequest>>) {
        mock_server_with_replies(responses.into_iter().map(Some).collect())
    }

    fn mock_server_with_replies(
        replies: Vec<Option<(u16, String)>>,
    ) -> (String, std::thread::JoinHandle<Vec<CapturedRequest>>) {
        use std::io::{BufRead, BufReader, Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let mut requests = Vec::with_capacity(replies.len());
            let mut replies = replies.into_iter().peekable();
            let started_at = std::time::Instant::now();
            let mut last_request_at = started_at;
            loop {
                if !requests.is_empty() && replies.peek().is_none() {
                    break;
                }
                let accepted = match listener.accept() {
                    Ok(accepted) => Some(accepted),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        let idle_timeout = if requests.is_empty() {
                            started_at.elapsed() >= std::time::Duration::from_secs(5)
                        } else {
                            last_request_at.elapsed() >= std::time::Duration::from_secs(2)
                        };
                        if idle_timeout {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("Mock server accept failed: {error}"),
                };
                let Some((stream, _)) = accepted else {
                    break;
                };
                let mut reader = BufReader::new(stream);
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or_default().to_owned();
                let target = parts.next().unwrap_or_default().to_owned();
                let mut headers = std::collections::HashMap::new();
                let mut content_length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':') {
                        let name = name.trim().to_ascii_lowercase();
                        let value = value.trim().to_owned();
                        if name == "content-length" {
                            content_length = value.parse().unwrap_or_default();
                        }
                        headers.insert(name, value);
                    }
                }
                let mut body = vec![0; content_length];
                reader.read_exact(&mut body).unwrap();
                let body = String::from_utf8(body).unwrap();
                requests.push(CapturedRequest {
                    method,
                    target,
                    headers,
                    body,
                });
                if let Some((status, response_body)) = replies
                    .next()
                    .expect("Mock server received more requests than configured replies")
                {
                    let mut stream = reader.into_inner();
                    let reason = match status {
                        201 => "Created",
                        500 => "Internal Server Error",
                        _ => "OK",
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response_body.len(),
                        response_body
                    )
                    .unwrap();
                }
                last_request_at = std::time::Instant::now();
            }
            requests
        });
        (format!("http://{address}"), server)
    }
}
