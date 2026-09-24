use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde_json::Value;
use tokio::time::sleep;
use url::Url;

use crate::config::{endpoint_url, Config};

const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const APIFY_MAX_RETRIES: usize = 8;
const APIFY_RETRY_BASE_DELAY: Duration = Duration::from_millis(500);
pub const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);

#[derive(Default)]
pub struct DatasetBudget {
    initial_dataset_items: Option<u64>,
    saved_dataset_items: u64,
}

pub struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

impl<'a> ApifyClient<'a> {
    pub fn new(http: &'a Client, config: &'a Config) -> Self {
        Self { http, config }
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
    }

    async fn send_with_retries<F>(&self, operation: &str, mut build_request: F) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        let mut retry_count = 0;
        loop {
            let response = match build_request().send().await {
                Ok(response) => response,
                Err(error) => {
                    let Some(delay) = apify_transport_retry_delay(&error, retry_count) else {
                        return Err(error)
                            .with_context(|| format!("Apify {operation} request failed"));
                    };
                    sleep(delay).await;
                    retry_count += 1;
                    continue;
                }
            };
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                sleep(delay).await;
                retry_count += 1;
                continue;
            }

            return Ok(response);
        }
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .send_with_retries("INPUT", || {
                self.http
                    .get(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
            })
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        response_json(response, "Apify INPUT request")
            .await
            .map(Some)
    }

    pub async fn push_dataset_items(
        &self,
        items: &[Value],
        budget: &mut DatasetBudget,
    ) -> Result<usize> {
        if items.is_empty() {
            return Ok(0);
        }

        let run_url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let run_response = self
            .send_with_retries("run pricing", || {
                self.http
                    .get(run_url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
            })
            .await?;
        let run = response_json(run_response, "Apify run pricing request").await?;
        let affordable_items = affordable_dataset_items(&run, items.len(), budget)?;
        if affordable_items == 0 {
            return Ok(0);
        }

        let items = &items[..affordable_items];
        let saved_count = u64::try_from(items.len()).context("Dataset row count is too large")?;
        let new_saved_count = budget
            .saved_dataset_items
            .checked_add(saved_count)
            .ok_or_else(|| anyhow!("Dataset row count overflowed"))?;
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .send_with_retries("dataset write", || {
                self.http
                    .post(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
                    .json(items)
            })
            .await?;
        ensure_success(response, "Apify dataset write").await?;

        budget.saved_dataset_items = new_saved_count;
        Ok(items.len())
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
            .send_with_retries("OUTPUT write", || {
                self.http
                    .put(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
                    .json(output)
            })
            .await?;
        ensure_success(response, "Apify OUTPUT write").await
    }
}

pub fn affordable_dataset_items(
    run: &Value,
    requested: usize,
    budget: &mut DatasetBudget,
) -> Result<usize> {
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
        .get(DATASET_ITEM_EVENT)
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
    let current_dataset_items = counts
        .get(DATASET_ITEM_EVENT)
        .map(|count| {
            count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {DATASET_ITEM_EVENT}"))
        })
        .transpose()?
        .unwrap_or(0);
    let initial_dataset_items = *budget
        .initial_dataset_items
        .get_or_insert(current_dataset_items);
    let locally_saved_dataset_items = initial_dataset_items
        .checked_add(budget.saved_dataset_items)
        .ok_or_else(|| anyhow!("Dataset row count overflowed"))?;

    let mut spent = 0.0;
    let mut saw_dataset_count = false;
    for (event_name, count) in counts {
        let mut count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if event_name == DATASET_ITEM_EVENT {
            saw_dataset_count = true;
            count = count.max(locally_saved_dataset_items);
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

    if !saw_dataset_count && locally_saved_dataset_items > 0 {
        spent += item_price * locally_saved_dataset_items as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .count())
}

pub fn apify_retry_delay(status: StatusCode, retry_count: usize) -> Option<Duration> {
    if retry_count >= APIFY_MAX_RETRIES
        || (status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error())
    {
        return None;
    }

    Some(apify_retry_backoff(retry_count))
}

fn apify_transport_retry_delay(error: &reqwest::Error, retry_count: usize) -> Option<Duration> {
    if error.is_builder() || retry_count >= APIFY_MAX_RETRIES {
        return None;
    }

    Some(apify_retry_backoff(retry_count))
}

fn apify_retry_backoff(retry_count: usize) -> Duration {
    APIFY_RETRY_BASE_DELAY * 2_u32.pow(retry_count as u32)
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
    use serde_json::{json, Value};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        task::JoinHandle,
    };
    use url::Url;

    use super::{
        affordable_dataset_items, apify_retry_delay, apify_transport_retry_delay, ApifyClient,
        DatasetBudget, APIFY_REQUEST_TIMEOUT, DATASET_ITEM_EVENT,
    };
    use crate::config::Config;

    fn run(max_charge: f64, dataset_rows: u64, other_event_count: u64) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            "other-event": {"eventPriceUsd": 0.0001}
                        }
                    }
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": {
                    "apify-default-dataset-item": dataset_rows,
                    "other-event": other_event_count
                }
            }
        })
    }

    #[test]
    fn applies_remaining_pay_per_event_budget_and_other_event_charges() {
        let mut budget = DatasetBudget::default();
        assert_eq!(
            affordable_dataset_items(&run(0.0012, 2, 3), 10, &mut budget).unwrap(),
            1
        );
    }

    #[test]
    fn tracks_local_dataset_writes_when_run_charge_counts_lag() {
        let mut budget = DatasetBudget::default();
        let run = run(0.0006, 0, 0);

        assert_eq!(affordable_dataset_items(&run, 1, &mut budget).unwrap(), 1);
        budget.saved_dataset_items = 1;
        assert_eq!(affordable_dataset_items(&run, 1, &mut budget).unwrap(), 1);
        budget.saved_dataset_items = 2;
        assert_eq!(affordable_dataset_items(&run, 1, &mut budget).unwrap(), 0);
    }

    #[test]
    fn allows_non_pay_per_event_runs_without_event_pricing() {
        let mut budget = DatasetBudget::default();
        for pricing_model in ["FREE", "PRICE_PER_DATASET_ITEM", "FLAT_PRICE_PER_MONTH"] {
            let non_ppe = json!({"data": {"pricingInfo": {"pricingModel": pricing_model}}});
            assert_eq!(
                affordable_dataset_items(&non_ppe, 3, &mut budget).unwrap(),
                3,
                "unexpected item limit for {pricing_model}"
            );
        }
    }

    #[test]
    fn treats_missing_null_and_zero_ppe_caps_as_unbounded() {
        for max_charge in [None, Some(Value::Null), Some(json!(0.0))] {
            let mut run = run(0.0003, 8, 3);
            if let Some(max_charge) = max_charge {
                run["data"]["options"]["maxTotalChargeUsd"] = max_charge;
            } else {
                run["data"]["options"]
                    .as_object_mut()
                    .unwrap()
                    .remove("maxTotalChargeUsd");
            }

            assert_eq!(
                affordable_dataset_items(&run, 4, &mut DatasetBudget::default()).unwrap(),
                4
            );
        }
    }

    #[test]
    fn rejects_missing_dataset_item_price_for_capped_ppe_runs() {
        let mut budget = DatasetBudget::default();

        let mut invalid = run(1.0, 0, 0);
        invalid["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove(DATASET_ITEM_EVENT);
        assert!(affordable_dataset_items(&invalid, 1, &mut budget).is_err());
    }

    #[test]
    fn retries_apify_requests_for_transient_responses_with_exponential_backoff() {
        assert_eq!(
            apify_retry_delay(reqwest::StatusCode::TOO_MANY_REQUESTS, 0),
            Some(std::time::Duration::from_millis(500))
        );
        assert_eq!(
            apify_retry_delay(reqwest::StatusCode::INTERNAL_SERVER_ERROR, 1),
            Some(std::time::Duration::from_secs(1))
        );
        assert_eq!(
            apify_retry_delay(reqwest::StatusCode::SERVICE_UNAVAILABLE, 7),
            Some(std::time::Duration::from_secs(64))
        );
        assert_eq!(
            apify_retry_delay(reqwest::StatusCode::SERVICE_UNAVAILABLE, 8),
            None
        );
        assert_eq!(apify_retry_delay(reqwest::StatusCode::BAD_REQUEST, 0), None);
    }

    #[test]
    fn apify_api_requests_keep_the_sdk_deadline() {
        assert_eq!(APIFY_REQUEST_TIMEOUT, std::time::Duration::from_secs(360));
    }

    #[test]
    fn does_not_retry_invalid_apify_request_builds() {
        let timeout = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(1))
            .build()
            .unwrap();
        let builder_error = timeout.get("not a url").build().unwrap_err();

        assert_eq!(apify_transport_retry_delay(&builder_error, 0), None);
    }

    async fn mock_api_responses(responses: Vec<Option<u16>>) -> (Url, JoinHandle<Vec<Vec<u8>>>) {
        mock_api_responses_with_run(responses, run(0.001, 0, 0)).await
    }

    async fn mock_api_responses_with_run(
        responses: Vec<Option<u16>>,
        run_response: Value,
    ) -> (Url, JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for status in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut chunk = [0; 4096];
                loop {
                    let length = socket.read(&mut chunk).await.unwrap();
                    if length == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..length]);
                    let Some(headers_end) = request
                        .windows(4)
                        .position(|window| window == b"\r\n\r\n")
                        .map(|index| index + 4)
                    else {
                        continue;
                    };
                    let headers =
                        String::from_utf8_lossy(&request[..headers_end]).to_ascii_lowercase();
                    let body_length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .and_then(|length| length.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if request.len() >= headers_end + body_length {
                        break;
                    }
                }
                let is_run_request = request.starts_with(b"GET /api/v2/actor-runs/");
                requests.push(request);
                let Some(status) = status else {
                    continue;
                };
                let reason = match status {
                    201 => "Created",
                    429 => "Too Many Requests",
                    _ => "OK",
                };
                let body = if is_run_request {
                    run_response.to_string()
                } else {
                    "{}".to_owned()
                };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        (
            Url::parse(&format!("http://{address}/api")).unwrap(),
            server,
        )
    }

    fn config_with_apify_base(apify_api_base: Url) -> Config {
        Config {
            apify_api_base,
            scrappa_api_base: Url::parse("https://scrappa.example/api").unwrap(),
            apify_token: "test-token".to_owned(),
            actor_run_id: "run-id".to_owned(),
            key_value_store_id: "store-id".to_owned(),
            dataset_id: "dataset-id".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "scrappa-key".to_owned(),
        }
    }

    #[tokio::test]
    async fn publishes_non_ppe_items_without_custom_event_charges() {
        let run_response = json!({"data": {"pricingInfo": {"pricingModel": "FREE"}}});
        let (base_url, server) =
            mock_api_responses_with_run(vec![Some(200), Some(201)], run_response).await;
        let config = config_with_apify_base(base_url);
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap();
        let client = ApifyClient::new(&http, &config);
        let mut budget = DatasetBudget::default();

        let saved = client
            .push_dataset_items(
                &[json!({"name": "Coffee"}), json!({"name": "Bakery"})],
                &mut budget,
            )
            .await
            .unwrap();
        assert_eq!(saved, 2);
        assert_eq!(budget.saved_dataset_items, 2);

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with(b"GET /api/v2/actor-runs/run-id"));
        assert!(requests[1].starts_with(b"POST /api/v2/datasets/dataset-id/items"));
        assert!(String::from_utf8_lossy(&requests[1]).contains("Coffee"));
        assert!(String::from_utf8_lossy(&requests[1]).contains("Bakery"));
    }

    #[tokio::test]
    async fn retries_dataset_writes_after_transient_apify_responses() {
        let (base_url, server) = mock_api_responses(vec![Some(200), Some(429), Some(201)]).await;
        let config = config_with_apify_base(base_url);
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap();
        let client = ApifyClient::new(&http, &config);
        let mut budget = DatasetBudget::default();

        let saved = client
            .push_dataset_items(&[json!({"name": "Coffee"})], &mut budget)
            .await
            .unwrap();
        assert_eq!(saved, 1);
        assert_eq!(budget.saved_dataset_items, 1);

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].starts_with(b"GET /api/v2/actor-runs/run-id"));
        assert!(requests[1].starts_with(b"POST /api/v2/datasets/dataset-id/items"));
        assert_eq!(requests[1], requests[2]);
    }

    #[tokio::test]
    async fn retries_apify_transport_failures() {
        let (base_url, server) = mock_api_responses(vec![None, Some(200)]).await;
        let config = config_with_apify_base(base_url);
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap();
        let client = ApifyClient::new(&http, &config);
        let url = client
            .endpoint(&["v2", "key-value-stores", "store-id", "records", "INPUT"])
            .unwrap();

        let response = client
            .send_with_retries("INPUT", || {
                http.get(url.clone()).bearer_auth(&config.apify_token)
            })
            .await
            .unwrap();
        assert!(response.status().is_success());

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
    }
}
