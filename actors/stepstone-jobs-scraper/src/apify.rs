use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::Value;
use std::time::Duration;

const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    input_key: String,
    dataset_id: String,
}

impl ApifyClient {
    pub fn new(
        base_url: &str,
        token: String,
        actor_run_id: String,
        key_value_store_id: String,
        input_key: String,
        dataset_id: String,
    ) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token,
            actor_run_id,
            key_value_store_id,
            input_key,
            dataset_id,
        })
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.key_value_store_id,
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

    pub async fn dataset_item_budget(&self) -> Result<usize> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&["actor-runs", &self.actor_run_id])?,
            )
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = successful_response(response, "fetch Actor run pricing")
            .await?
            .json()
            .await
            .context("Actor run pricing response is not valid JSON")?;
        affordable_dataset_items(&run)
    }

    pub async fn push_data(&self, items: &[Value], remaining_budget: &mut usize) -> Result<usize> {
        let item_count = items.len().min(*remaining_budget);
        if item_count == 0 {
            return Ok(0);
        }

        let response = self
            .request(
                Method::POST,
                self.resource_url(&["datasets", &self.dataset_id, "items"])?,
            )
            .json(&items[..item_count])
            .send()
            .await
            .context("Failed to store job items in the default dataset")?;
        successful_response(response, "store dataset items").await?;
        *remaining_budget -= item_count;
        Ok(item_count)
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .request(Method::PUT, url)
            .json(output)
            .send()
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
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
}

fn affordable_dataset_items(run: &Value) -> Result<usize> {
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
        .get("apify-default-dataset-item")
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => None,
        Some(value) => Some(
            value
                .as_f64()
                .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?,
        ),
    };
    if !item_price.is_finite()
        || item_price < 0.0
        || max_charge.is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        bail!("Apify run returned invalid charging values");
    }
    let Some(max_charge) = max_charge.filter(|value| *value > 0.0) else {
        return Ok(usize::MAX);
    };

    let charged_events = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    for (event_name, count) in charged_events {
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
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(usize::MAX);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let available_charge = (max_charge - spent + tolerance).max(0.0);
    let affordable_items = (available_charge / item_price).floor();
    Ok(if affordable_items.is_finite() {
        affordable_items as usize
    } else {
        usize::MAX
    })
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
    use crate::test_support::{MockResponse, MockServer, has_header, request_parts};
    use serde_json::json;

    fn pricing_response(max_charge: f64, charged_counts: Value) -> MockResponse {
        MockResponse::json(
            200,
            json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                            "other-event": {"eventPriceUsd": 0.05}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_charge},
                    "chargedEventCounts": charged_counts
                }
            }),
        )
    }

    fn pricing_run(options: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                        "other-event": {"eventPriceUsd": 0.05}
                    }}
                },
                "options": options,
                "chargedEventCounts": {}
            }
        })
    }

    fn client(server: &MockServer) -> ApifyClient {
        ApifyClient::new(
            &server.base_url,
            "test-token".to_owned(),
            "test-run".to_owned(),
            "test-store".to_owned(),
            "INPUT".to_owned(),
            "test-dataset".to_owned(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn caps_dataset_rows_to_the_remaining_default_item_charge_budget() {
        let server = MockServer::start(vec![
            pricing_response(0.1, json!({})),
            MockResponse::json(201, json!({})),
        ]);
        let apify = client(&server);
        let mut budget = apify.dataset_item_budget().await.unwrap();
        assert_eq!(budget, 1);

        let saved = apify
            .push_data(
                &[json!({"id": "first"}), json!({"id": "second"})],
                &mut budget,
            )
            .await
            .unwrap();
        assert_eq!(saved, 1);
        assert_eq!(budget, 0);

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_parts(&requests[0]).1, "/v2/actor-runs/test-run");
        assert!(has_header(
            &requests[0],
            "Authorization",
            "Bearer test-token"
        ));
        let (method, path, body) = request_parts(&requests[1]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert!(has_header(
            &requests[1],
            "Authorization",
            "Bearer test-token"
        ));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{"id": "first"}])
        );
    }

    #[tokio::test]
    async fn zero_max_total_charge_is_unlimited_for_dataset_writes() {
        let server = MockServer::start(vec![
            pricing_response(0.0, json!({})),
            MockResponse::json(201, json!({})),
        ]);
        let apify = client(&server);
        let mut budget = apify.dataset_item_budget().await.unwrap();
        assert_eq!(budget, usize::MAX);

        assert_eq!(
            apify
                .push_data(&[json!({"id": "first"})], &mut budget)
                .await
                .unwrap(),
            1
        );
        assert_eq!(server.requests().len(), 2);
    }

    #[test]
    fn missing_max_total_charge_is_unlimited() {
        assert_eq!(
            affordable_dataset_items(&pricing_run(json!({}))).unwrap(),
            usize::MAX
        );
    }

    #[test]
    fn null_max_total_charge_is_unlimited() {
        assert_eq!(
            affordable_dataset_items(&pricing_run(json!({"maxTotalChargeUsd": null}))).unwrap(),
            usize::MAX
        );
    }

    #[test]
    fn subtracts_all_existing_charges_from_the_run_limit() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                        "other-event": {"eventPriceUsd": 0.05}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.5},
                "chargedEventCounts": {"apify-default-dataset-item": 1, "other-event": 2}
            }
        });
        assert_eq!(affordable_dataset_items(&run).unwrap(), 3);
    }

    #[test]
    fn refuses_to_authorize_rows_when_pricing_data_is_missing_or_invalid() {
        assert!(affordable_dataset_items(&json!({})).is_err());
        assert!(
            affordable_dataset_items(&json!({"data": {
                "pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM"}
            }}))
            .is_err()
        );
        assert!(affordable_dataset_items(&json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {}}},
            "options": {"maxTotalChargeUsd": 1.0},
            "chargedEventCounts": {}
        }})).is_err());
    }

    #[tokio::test]
    async fn reads_input_and_writes_the_raw_response_to_output() {
        let output = json!({"data": {"jobs": [{"title": "Engineer"}]}});
        let server = MockServer::start(vec![
            MockResponse::json(200, json!({"query": "Engineer"})),
            MockResponse::json(201, json!({})),
        ]);
        let apify = client(&server);
        assert_eq!(
            apify.get_input().await.unwrap(),
            Some(json!({"query": "Engineer"}))
        );
        apify.set_output(&output).await.unwrap();

        let requests = server.requests();
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert_eq!(
            request_parts(&requests[1]).1,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[1]).2).unwrap(),
            output
        );
        assert!(requests.iter().all(|request| has_header(
            request,
            "Authorization",
            "Bearer test-token"
        )));
    }

    #[tokio::test]
    async fn missing_input_record_uses_the_default_input_path() {
        let server =
            MockServer::start(vec![MockResponse::json(404, json!({"error": "not found"}))]);
        let apify = client(&server);
        assert_eq!(apify.get_input().await.unwrap(), None);
    }
}
