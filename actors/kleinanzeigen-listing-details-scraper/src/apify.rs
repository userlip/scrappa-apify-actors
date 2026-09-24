use std::{collections::HashMap, time::Duration};

use crate::error_utils::error_summary;
use anyhow::{Context, Result, anyhow};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, Url};
use serde_json::Value;
use tokio::time::sleep;

pub const LISTING_DETAIL_RESULT_CHARGE_EVENT: &str = "listing-detail-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: usize = 8;
const APIFY_RETRY_BASE_DELAY_MS: u64 = 500;

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    charging: Option<ChargingManager>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PushDataResult {
    pub saved: bool,
    pub event_charge_limit_reached: bool,
}

#[derive(Debug, Clone)]
struct ChargingManager {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    charged_event_counts: HashMap<String, u64>,
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
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token,
            actor_run_id,
            key_value_store_id,
            dataset_id,
            input_key,
            charging: None,
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

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    pub async fn load_charging_state(&mut self) -> Result<()> {
        let url = self.resource_url(&["actor-runs", &self.actor_run_id])?;
        let run = self.get_json(url, "Apify run pricing request").await?;
        self.charging = Some(ChargingManager::from_run(&run)?);
        Ok(())
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .send_with_retry("input retrieval", || self.request(Method::GET, url.clone()))
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        successful_response(response, "input retrieval")
            .await?
            .json::<Value>()
            .await
            .context("Apify input record was not valid JSON")
            .map(Some)
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .send_with_retry("OUTPUT record publication", || {
                self.request(Method::PUT, url.clone()).json(output)
            })
            .await?;
        successful_response(response, "OUTPUT record publication").await?;
        Ok(())
    }

    pub async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.resource_url(&["actor-runs", &self.actor_run_id])?;
        let body = serde_json::json!({ "statusMessage": message });
        let response = self
            .send_with_retry("Actor run status message update", || {
                self.request(Method::PUT, url.clone()).json(&body)
            })
            .await?;
        successful_response(response, "Actor run status message update").await?;
        Ok(())
    }

    pub fn is_pay_per_event(&self) -> bool {
        self.charging
            .as_ref()
            .is_some_and(|charging| charging.is_pay_per_event)
    }

    pub fn listing_event_capacity(&self, event_name: &str) -> usize {
        self.charging
            .as_ref()
            .map(|charging| charging.max_event_charge_count(event_name))
            .unwrap_or(0)
    }

    pub async fn push_data(
        &mut self,
        item: &Value,
        event_name: &str,
        request_index: usize,
    ) -> Result<PushDataResult> {
        if !self.is_pay_per_event() {
            self.push_dataset_item(item).await?;
            return Ok(PushDataResult {
                saved: true,
                event_charge_limit_reached: false,
            });
        }

        let should_push = self
            .charging
            .as_ref()
            .context("Apify charging state was not initialized")?
            .calculate_push_data_count(event_name, true, 1)
            > 0;
        if !should_push {
            return Ok(PushDataResult {
                saved: false,
                event_charge_limit_reached: true,
            });
        }

        self.push_dataset_item(item).await?;

        let charge_plan = {
            let charging = self
                .charging
                .as_mut()
                .context("Apify charging state was not initialized")?;
            let explicit_count = charging.record_charge(event_name, 1);
            let dataset_count = charging.record_charge(DEFAULT_DATASET_ITEM_EVENT, 1);
            let event_charge_limit_reached = charging.max_event_charge_count(event_name) == 0
                || charging.max_event_charge_count(DEFAULT_DATASET_ITEM_EVENT) == 0;
            (explicit_count, dataset_count, event_charge_limit_reached)
        };

        let (explicit_count, dataset_count, event_charge_limit_reached) = charge_plan;
        if explicit_count > 0
            && !event_name.starts_with("apify-")
            && self
                .charging
                .as_ref()
                .is_some_and(|charging| charging.event_prices.contains_key(event_name))
        {
            self.charge_event(
                event_name,
                explicit_count,
                &format!("{}-{event_name}-{request_index}", self.actor_run_id),
            )
            .await?;
        }

        Ok(PushDataResult {
            saved: explicit_count + dataset_count > 0,
            event_charge_limit_reached,
        })
    }

    async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.resource_url(&["datasets", &self.dataset_id, "items"])?;
        // Dataset appends can commit before a timeout, so retrying could duplicate a row.
        let response = self
            .request(Method::POST, url)
            .json(item)
            .send()
            .await
            .context("Apify API dataset item publication failed")?;
        successful_response(response, "dataset item publication").await?;
        Ok(())
    }

    async fn charge_event(
        &self,
        event_name: &str,
        count: u64,
        idempotency_key: &str,
    ) -> Result<()> {
        let url = self.resource_url(&["actor-runs", &self.actor_run_id, "charge"])?;
        let body = serde_json::json!({ "eventName": event_name, "count": count });
        let response = self
            .send_with_retry("pay-per-event charge", || {
                self.request(Method::POST, url.clone())
                    .header("Idempotency-Key", idempotency_key)
                    .json(&body)
            })
            .await?;
        successful_response(response, "pay-per-event charge").await?;
        Ok(())
    }

    async fn get_json(&self, url: Url, operation: &str) -> Result<Value> {
        let response = self
            .send_with_retry(operation, || self.request(Method::GET, url.clone()))
            .await?;
        successful_response(response, operation)
            .await?
            .json::<Value>()
            .await
            .with_context(|| format!("{operation} response was not valid JSON"))
    }

    async fn send_with_retry<F>(&self, operation: &str, mut build_request: F) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        for attempt in 1..=APIFY_MAX_RETRIES + 1 {
            match build_request().send().await {
                Ok(response)
                    if is_retryable_status(response.status()) && attempt <= APIFY_MAX_RETRIES =>
                {
                    let status = response.status();
                    drop(response);
                    let delay = retry_delay(attempt);
                    eprintln!(
                        "Apify API {operation} returned HTTP {}. Retrying attempt {}/{APIFY_MAX_RETRIES} in {}ms.",
                        status.as_u16(),
                        attempt + 1,
                        delay.as_millis(),
                    );
                    sleep(delay).await;
                }
                Ok(response) => return Ok(response),
                Err(error) if is_retryable_transport(&error) && attempt <= APIFY_MAX_RETRIES => {
                    let delay = retry_delay(attempt);
                    eprintln!(
                        "Apify API {operation} failed ({}). Retrying attempt {}/{APIFY_MAX_RETRIES} in {}ms.",
                        error_summary(&error.to_string()),
                        attempt + 1,
                        delay.as_millis(),
                    );
                    sleep(delay).await;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify API {operation} failed"));
                }
            }
        }
        unreachable!("the Apify retry loop always returns or fails")
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_retryable_transport(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

fn retry_delay(failed_attempt: usize) -> Duration {
    let exponent = failed_attempt.saturating_sub(1).min(8) as u32;
    Duration::from_millis(APIFY_RETRY_BASE_DELAY_MS * 2_u64.pow(exponent))
}

impl ChargingManager {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let is_pay_per_event = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");

        let max_total_charge_usd = match data.pointer("/options/maxTotalChargeUsd") {
            None | Some(Value::Null) => f64::INFINITY,
            Some(value) => {
                let amount = value
                    .as_f64()
                    .filter(|amount| amount.is_finite() && *amount >= 0.0)
                    .ok_or_else(|| anyhow!("Apify run returned invalid maxTotalChargeUsd"))?;
                if amount == 0.0 {
                    // Apify SDK 3.7.1 uses zero to mean an unbounded total charge.
                    f64::INFINITY
                } else {
                    amount
                }
            }
        };

        let mut event_prices = HashMap::new();
        if let Some(events) = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
        {
            for (name, event) in events {
                let price = event
                    .get("eventPriceUsd")
                    .and_then(Value::as_f64)
                    .filter(|price| price.is_finite() && *price >= 0.0)
                    .ok_or_else(|| {
                        anyhow!("Apify run returned an invalid price for event {name}")
                    })?;
                event_prices.insert(name.clone(), price);
            }
        }

        let mut charged_event_counts = HashMap::new();
        if let Some(counts) = data.get("chargedEventCounts").and_then(Value::as_object) {
            for (name, value) in counts {
                let count = value
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {name}"))?;
                charged_event_counts.insert(name.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        })
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_event_counts
            .iter()
            .map(|(name, count)| {
                self.event_prices.get(name).copied().unwrap_or_default() * *count as f64
            })
            .sum::<f64>();
        (total * 1_000_000.0).round() / 1_000_000.0
    }

    fn max_event_charge_count(&self, event_name: &str) -> usize {
        let price = self
            .event_prices
            .get(event_name)
            .copied()
            .unwrap_or_default();
        if !self.is_pay_per_event || price == 0.0 || self.max_total_charge_usd.is_infinite() {
            return usize::MAX;
        }
        self.max_charge_count_for_price(price)
    }

    fn max_charge_count_for_price(&self, price: f64) -> usize {
        if price == 0.0 {
            return usize::MAX;
        }
        let remaining = self.max_total_charge_usd - self.total_charged_amount();
        let quotient = remaining / price;
        if !quotient.is_finite() {
            return if quotient.is_sign_positive() {
                usize::MAX
            } else {
                0
            };
        }
        let rounded = (quotient * 10_000.0).round() / 10_000.0;
        rounded.floor().max(0.0).min(usize::MAX as f64) as usize
    }

    fn calculate_push_data_count(
        &self,
        event_name: &str,
        is_default_dataset: bool,
        requested: usize,
    ) -> usize {
        if !self.is_pay_per_event {
            return requested;
        }
        let explicit_price = self
            .event_prices
            .get(event_name)
            .copied()
            .unwrap_or_default();
        let dataset_price = if is_default_dataset {
            self.event_prices
                .get(DEFAULT_DATASET_ITEM_EVENT)
                .copied()
                .unwrap_or_default()
        } else {
            0.0
        };
        let combined_price = explicit_price + dataset_price;
        let max_count = self.max_charge_count_for_price(combined_price);
        if max_count >= requested {
            requested
        } else if max_count == 0 && self.total_charged_amount() <= self.max_total_charge_usd {
            requested.min(1)
        } else {
            requested.min(max_count)
        }
    }

    fn record_charge(&mut self, event_name: &str, requested: u64) -> u64 {
        let max_count = self.max_event_charge_count(event_name) as u64;
        let charged_count = if requested <= max_count {
            requested
        } else if self.total_charged_amount() <= self.max_total_charge_usd {
            max_count.saturating_add(1)
        } else {
            0
        };
        if charged_count > 0 {
            *self
                .charged_event_counts
                .entry(event_name.to_owned())
                .or_default() += charged_count;
        }
        charged_count
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
        body.trim().to_owned()
    };
    Err(anyhow!(
        "Apify API error ({status_code}) while trying to {operation}: {detail}"
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        ApifyClient, ChargingManager, DEFAULT_DATASET_ITEM_EVENT,
        LISTING_DETAIL_RESULT_CHARGE_EVENT, is_retryable_status, retry_delay,
    };
    use reqwest::{Client, StatusCode};
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        thread,
        time::{Duration, Instant},
    };

    fn charging(run: serde_json::Value) -> ChargingManager {
        ChargingManager::from_run(&run).unwrap()
    }

    #[test]
    fn treats_zero_missing_and_null_total_charge_caps_as_unlimited() {
        for options in [
            json!({"maxTotalChargeUsd": 0}),
            json!({}),
            json!({"maxTotalChargeUsd": null}),
        ] {
            let manager = charging(json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "listing-detail-result": {"eventPriceUsd": 0.00025},
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                            "apify-actor-start": {"eventPriceUsd": 0.0005}
                        }}
                    },
                    "options": options,
                    "chargedEventCounts": {"apify-actor-start": 1}
                }
            }));

            assert!(manager.max_total_charge_usd.is_infinite());
            assert_eq!(
                manager.max_event_charge_count(LISTING_DETAIL_RESULT_CHARGE_EVENT),
                usize::MAX
            );
            assert_eq!(
                manager.calculate_push_data_count(
                    LISTING_DETAIL_RESULT_CHARGE_EVENT,
                    true,
                    3
                ),
                3
            );
        }
    }

    #[test]
    fn preserves_positive_total_charge_cap_and_combined_event_prices() {
        let manager = charging(json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "listing-detail-result": {"eventPriceUsd": 0.00025},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                        "apify-actor-start": {"eventPriceUsd": 0.0005}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.0016},
                "chargedEventCounts": {"apify-actor-start": 1}
            }
        }));
        assert_eq!(
            manager.max_event_charge_count(LISTING_DETAIL_RESULT_CHARGE_EVENT),
            4
        );
        assert_eq!(manager.max_event_charge_count("unpriced-event"), usize::MAX);
        assert_eq!(
            manager.calculate_push_data_count(LISTING_DETAIL_RESULT_CHARGE_EVENT, true, 5),
            3
        );
    }

    #[test]
    fn models_synthetic_dataset_charges_and_ppe_limit_boundary() {
        let mut manager = charging(json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "listing-detail-result": {"eventPriceUsd": 0.00025},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.00025},
                "chargedEventCounts": {}
            }
        }));
        assert_eq!(
            manager.calculate_push_data_count(LISTING_DETAIL_RESULT_CHARGE_EVENT, true, 1),
            1
        );
        assert_eq!(
            manager.record_charge(LISTING_DETAIL_RESULT_CHARGE_EVENT, 1),
            1
        );
        assert_eq!(manager.record_charge(DEFAULT_DATASET_ITEM_EVENT, 1), 1);
        assert_eq!(
            manager.max_event_charge_count(LISTING_DETAIL_RESULT_CHARGE_EVENT),
            0
        );
        assert_eq!(
            manager.max_event_charge_count(DEFAULT_DATASET_ITEM_EVENT),
            usize::MAX
        );
    }

    #[test]
    fn non_ppe_run_has_unlimited_capacity() {
        let manager = charging(json!({
            "data": {
                "pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM", "pricingPerEvent": {"actorChargeEvents": {}}},
                "options": {"maxTotalChargeUsd": 0},
                "chargedEventCounts": {}
            }
        }));
        assert!(!manager.is_pay_per_event);
        assert_eq!(
            manager.max_event_charge_count(LISTING_DETAIL_RESULT_CHARGE_EVENT),
            usize::MAX
        );
        assert_eq!(
            manager.calculate_push_data_count(LISTING_DETAIL_RESULT_CHARGE_EVENT, true, 3),
            3
        );
    }

    #[test]
    fn retries_apify_rate_limits_and_server_errors_with_backoff() {
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
        assert_eq!(retry_delay(1), std::time::Duration::from_millis(500));
        assert_eq!(retry_delay(2), std::time::Duration::from_millis(1000));
    }

    #[tokio::test]
    async fn does_not_retry_dataset_post_after_response_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let received = Arc::new(AtomicUsize::new(0));
        let server_received = Arc::clone(&received);
        let server = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_millis(800);
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        read_request(&mut stream);
                        let attempt = server_received.fetch_add(1, Ordering::SeqCst);
                        if attempt == 0 {
                            thread::sleep(Duration::from_millis(120));
                        }
                        let _ = stream.write_all(
                            b"HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        );
                        if attempt > 0 {
                            break;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("dataset mock server failed: {error}"),
                }
            }
        });

        let mut apify = ApifyClient::new(
            &format!("http://{address}"),
            "token".into(),
            "run".into(),
            "store".into(),
            "dataset".into(),
            "INPUT".into(),
        )
        .unwrap();
        apify.client = Client::builder()
            .timeout(Duration::from_millis(40))
            .build()
            .unwrap();

        let result = apify
            .push_data(
                &json!({"id": "listing-1"}),
                LISTING_DETAIL_RESULT_CHARGE_EVENT,
                0,
            )
            .await;
        server.join().unwrap();

        assert!(
            result.is_err(),
            "the ambiguous timed-out write must be reported"
        );
        assert_eq!(received.load(Ordering::SeqCst), 1);
    }

    fn read_request(stream: &mut TcpStream) {
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        let mut content_length = None;
        loop {
            let count = stream.read(&mut chunk).unwrap();
            assert!(count > 0, "client closed before sending a complete request");
            request.extend_from_slice(&chunk[..count]);
            if let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                content_length.get_or_insert_with(|| {
                    headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0)
                });
                if request.len() >= header_end + 4 + content_length.unwrap() {
                    return;
                }
            }
        }
    }
}
