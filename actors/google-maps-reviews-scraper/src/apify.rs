use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, Url};
use serde_json::Value;
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_RETRIES: usize = 8;
const FIRST_RETRY_DELAY: Duration = Duration::from_millis(500);

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    pub fn new(
        base_url: &str,
        token: String,
        actor_run_id: String,
        key_value_store_id: String,
        dataset_id: String,
        input_key: String,
    ) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token,
            actor_run_id,
            key_value_store_id,
            dataset_id,
            input_key,
        })
    }

    fn endpoint(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot be a base URL"))?;
        path.pop_if_empty();
        path.extend(["v2"].into_iter().chain(resource.iter().copied()));
        drop(path);
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    async fn send_with_retries<F>(&self, operation: &str, mut build_request: F) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        let mut retry_count = 0;
        loop {
            match build_request().send().await {
                Ok(response)
                    if retryable_status(response.status()) && retry_count < MAX_RETRIES =>
                {
                    drop(response);
                    tokio::time::sleep(retry_delay(retry_count)).await;
                    retry_count += 1;
                }
                Ok(response) => return Ok(response),
                Err(_) if retry_count < MAX_RETRIES => {
                    tokio::time::sleep(retry_delay(retry_count)).await;
                    retry_count += 1;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify {operation} failed"))
                }
            }
        }
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .send_with_retries("input retrieval", || self.request(Method::GET, url.clone()))
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "input retrieval").await?;
        response
            .json()
            .await
            .context("Apify input record was not valid JSON")
            .map(Some)
    }

    pub async fn dataset_item_budget(&self) -> Result<usize> {
        let url = self.endpoint(&["actor-runs", &self.actor_run_id])?;
        let response = self
            .send_with_retries("run pricing request", || {
                self.request(Method::GET, url.clone())
            })
            .await?;
        let run: Value = successful_response(response, "run pricing request")
            .await?
            .json()
            .await
            .context("Apify run pricing response was not valid JSON")?;
        affordable_dataset_items(&run)
    }

    pub async fn push_data(&self, items: &[Value], remaining_budget: &mut usize) -> Result<usize> {
        let count = items.len().min(*remaining_budget);
        if count == 0 {
            return Ok(0);
        }
        let url = self.endpoint(&["datasets", &self.dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(&items[..count])
            .send()
            .await
            .context("Apify dataset item publication failed")?;
        successful_response(response, "dataset item publication").await?;
        *remaining_budget -= count;
        Ok(count)
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .send_with_retries("OUTPUT publication", || {
                self.request(Method::PUT, url.clone()).json(output)
            })
            .await?;
        successful_response(response, "OUTPUT publication").await?;
        Ok(())
    }
}

fn retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn retry_delay(retry_count: usize) -> Duration {
    FIRST_RETRY_DELAY.saturating_mul(2_u32.saturating_pow(retry_count as u32))
}

fn affordable_dataset_items(run: &Value) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let pricing_model = data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Apify run did not provide a pricing model"))?;
    if pricing_model != "PAY_PER_EVENT" {
        return Ok(usize::MAX);
    }
    let Some(max_charge_value) = data.pointer("/options/maxTotalChargeUsd") else {
        return Ok(usize::MAX);
    };
    if max_charge_value.is_null() {
        return Ok(usize::MAX);
    }
    let max_charge = max_charge_value
        .as_f64()
        .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned an invalid spending limit");
    }
    if max_charge == 0.0 {
        return Ok(usize::MAX);
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
            .ok_or_else(|| anyhow!("Invalid charged event count"))?;
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
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.is_empty() {
        format!("HTTP {status}")
    } else {
        body
    };
    bail!("Apify API error ({status}) while trying to {operation}: {detail}");
}

#[cfg(test)]
mod tests {
    use super::{affordable_dataset_items, retry_delay, retryable_status, ApifyClient};
    use crate::test_support::{MockResponse, MockServer};
    use reqwest::StatusCode;
    use serde_json::{json, Value};
    use std::time::Duration;

    fn pay_per_event_run(max_charge: Option<Value>, charged_events: Value) -> Value {
        let mut data = json!({
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                        "apify-actor-start": {"eventPriceUsd": 0.05}
                    }}
                },
                "chargedEventCounts": charged_events
        });
        if let Some(max_charge) = max_charge {
            data["options"]["maxTotalChargeUsd"] = max_charge;
        }
        json!({"data": data})
    }

    fn client(base_url: String) -> ApifyClient {
        ApifyClient::new(
            &base_url,
            "apify-test-token".to_owned(),
            "test-run".to_owned(),
            "test-store".to_owned(),
            "test-dataset".to_owned(),
            "INPUT".to_owned(),
        )
        .unwrap()
    }

    #[test]
    fn non_pay_per_event_runs_have_no_dataset_item_budget() {
        let run = json!({"data": {"pricingInfo": {"pricingModel": "FIXED_PRICE"}}});
        assert_eq!(affordable_dataset_items(&run).unwrap(), usize::MAX);
    }

    #[test]
    fn missing_null_and_zero_caps_are_unlimited() {
        for max_charge in [None, Some(Value::Null), Some(json!(0))] {
            assert_eq!(
                affordable_dataset_items(&pay_per_event_run(max_charge, json!({}))).unwrap(),
                usize::MAX
            );
        }
    }

    #[test]
    fn positive_cap_accounts_for_charges_across_event_types() {
        let run = pay_per_event_run(
            Some(json!(0.25)),
            json!({"apify-default-dataset-item": 1, "apify-actor-start": 1}),
        );
        assert_eq!(affordable_dataset_items(&run).unwrap(), 1);
    }

    #[test]
    fn retries_only_transient_apify_statuses_with_exponential_backoff() {
        assert!(retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(!retryable_status(StatusCode::BAD_REQUEST));
        assert_eq!(retry_delay(0), Duration::from_millis(500));
        assert_eq!(retry_delay(1), Duration::from_secs(1));
    }

    #[tokio::test]
    async fn reads_input_with_bearer_auth_and_treats_missing_input_as_empty() {
        let server =
            MockServer::start(vec![MockResponse::json(404, json!({"error": "not found"}))]);
        let apify = client(server.root_url());

        assert!(apify.get_input().await.unwrap().is_none());
        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET /v2/key-value-stores/test-store/records/INPUT "));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer apify-test-token\r\n"));
    }

    #[tokio::test]
    async fn caps_dataset_writes_and_publishes_output() {
        let server = MockServer::start(vec![
            MockResponse::json(200, pay_per_event_run(Some(json!(0.15)), json!({}))),
            MockResponse::json(201, json!({})),
            MockResponse::json(201, json!({})),
        ]);
        let apify = client(server.root_url());
        let budget = apify.dataset_item_budget().await.unwrap();
        let rows = vec![json!({"review_id": "r1"}), json!({"review_id": "r2"})];
        let mut remaining = budget;
        assert_eq!(apify.push_data(&rows, &mut remaining).await.unwrap(), 1);
        assert_eq!(remaining, 0);
        let output = json!({"items": rows, "nextPage": "next"});
        apify.set_output(&output).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests[1].starts_with("POST /v2/datasets/test-dataset/items "));
        assert!(requests[1].ends_with("[{\"review_id\":\"r1\"}]"));
        assert!(requests[2].starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT "));
        assert!(requests[2].ends_with(&output.to_string()));
    }

    #[tokio::test]
    async fn does_not_retry_dataset_post_after_the_response_is_lost() {
        let server = MockServer::start(vec![
            MockResponse::disconnect(),
            MockResponse::json(201, json!({})),
        ]);
        let apify = client(server.root_url());
        let mut remaining = 1;
        assert!(apify
            .push_data(&[json!({"review_id": "r1"})], &mut remaining)
            .await
            .is_err());
        assert_eq!(remaining, 1);
        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("POST /v2/datasets/test-dataset/items "));
        assert!(requests[0].ends_with("[{\"review_id\":\"r1\"}]"));
    }

    #[tokio::test]
    async fn retries_transient_output_put_failure() {
        let server = MockServer::start(vec![
            MockResponse::json(503, json!({"message": "retry"})),
            MockResponse::json(201, json!({})),
        ]);
        let apify = client(server.root_url());

        apify.set_output(&json!({"items": []})).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert!(requests
            .iter()
            .all(|request| request
                .starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT ")));
    }
}
