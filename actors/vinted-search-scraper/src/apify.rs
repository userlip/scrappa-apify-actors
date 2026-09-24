use std::collections::HashMap;

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Method, Response, StatusCode, Url};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::config::{APIFY_REQUEST_TIMEOUT, OUTPUT_KEY};

pub const ITEM_RESULT_CHARGE_EVENT: &str = "item-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub fn new(base_url: Url, token: String) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url,
            token,
        })
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot be a base URL"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(resource.iter().copied()));
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
    }

    pub async fn get_input(&self, store_id: &str, input_key: &str) -> Result<Option<Value>> {
        let url = self.resource_url(&["key-value-stores", store_id, "records", input_key])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        response_json(response, "INPUT request").await.map(Some)
    }

    pub async fn set_output(&self, store_id: &str, output: &Value) -> Result<()> {
        let url = self.resource_url(&["key-value-stores", store_id, "records", OUTPUT_KEY])?;
        let response = self
            .request(Method::PUT, url)
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "write OUTPUT").await
    }

    pub async fn get_charging_budget(&self, actor_run_id: &str) -> Result<ChargingBudget> {
        let url = self.resource_url(&["actor-runs", actor_run_id])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = response_json(response, "run pricing request").await?;
        ChargingBudget::from_run(&run)
    }

    pub async fn charge_event(
        &self,
        actor_run_id: &str,
        event_name: &str,
        count: usize,
    ) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let url = self.resource_url(&["actor-runs", actor_run_id, "charge"])?;
        let response = self
            .request(Method::POST, url)
            .header("idempotency-key", Uuid::new_v4().to_string())
            .json(&json!({
                "eventName": event_name,
                "count": count,
            }))
            .send()
            .await
            .context("Apify item-result charge request failed")?;
        ensure_success(response, "charge item-result events").await
    }

    pub async fn push_data(&self, dataset_id: &str, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.resource_url(&["datasets", dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "store dataset items").await
    }

    pub async fn set_status_message(
        &self,
        actor_run_id: &str,
        status_message: &str,
        is_terminal: bool,
    ) -> Result<()> {
        let url = self.resource_url(&["actor-runs", actor_run_id])?;
        let response = self
            .request(Method::PUT, url)
            .json(&json!({
                "runId": actor_run_id,
                "statusMessage": status_message,
                "isStatusMessageTerminal": is_terminal,
            }))
            .send()
            .await
            .context("Apify run status update failed")?;
        ensure_success(response, "update run status message").await
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChargingBudget {
    NonPayPerEvent,
    PayPerEvent(PayPerEventBudget),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PayPerEventBudget {
    event_prices: HashMap<String, f64>,
    charged_usd: f64,
    max_total_charge_usd: Option<f64>,
}

impl ChargingBudget {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        if data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(Self::NonPayPerEvent);
        }

        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (event_name, event) in events {
            if let Some(price) = configured_event_price(event_name, event)? {
                event_prices.insert(event_name.clone(), price);
            }
        }

        let max_total_charge_usd = match data.pointer("/options/maxTotalChargeUsd") {
            None | Some(Value::Null) => None,
            Some(value) => {
                let max_total_charge_usd = value
                    .as_f64()
                    .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
                if !max_total_charge_usd.is_finite() || max_total_charge_usd < 0.0 {
                    bail!("Apify run returned an invalid spending limit");
                }
                (max_total_charge_usd > 0.0).then_some(max_total_charge_usd)
            }
        };

        let charged_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_usd = 0.0;
        for (event_name, count) in charged_counts {
            let count = count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            if count == 0 {
                continue;
            }
            let price = event_prices
                .get(event_name)
                .copied()
                .or_else(|| (event_name == DEFAULT_DATASET_ITEM_EVENT).then_some(0.0))
                .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
            charged_usd += price * count as f64;
        }
        if !charged_usd.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        Ok(Self::PayPerEvent(PayPerEventBudget {
            event_prices,
            charged_usd,
            max_total_charge_usd,
        }))
    }
}

fn configured_event_price(event_name: &str, event: &Value) -> Result<Option<f64>> {
    if let Some(flat_price) = event.get("eventPriceUsd").filter(|price| !price.is_null()) {
        let price = flat_price
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run returned an invalid price for event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Apify run returned an invalid price for event {event_name}");
        }
        return Ok(Some(price));
    }

    let Some(tiered_prices) = event
        .get("eventTieredPricingUsd")
        .and_then(Value::as_object)
    else {
        return Ok(None);
    };
    let mut highest_price: Option<f64> = None;
    for (tier, tiered_price) in tiered_prices {
        let price = tiered_price
            .get("tieredEventPriceUsd")
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Invalid {tier} price for charged event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid {tier} price for charged event {event_name}");
        }
        highest_price = Some(highest_price.map_or(price, |highest| highest.max(price)));
    }
    highest_price
        .map(Some)
        .ok_or_else(|| anyhow!("Apify run did not provide tier prices for event {event_name}"))
}

impl PayPerEventBudget {
    pub fn affordable_items(&self, requested: usize) -> Result<usize> {
        let result_item_price = self.event_price(ITEM_RESULT_CHARGE_EVENT)?;
        let default_item_price = self.event_price(DEFAULT_DATASET_ITEM_EVENT)?;
        let price_per_saved_item = result_item_price + default_item_price;
        if !price_per_saved_item.is_finite() || price_per_saved_item < 0.0 {
            bail!("Apify run returned an invalid Vinted result price");
        }
        if price_per_saved_item == 0.0 {
            return Ok(requested);
        }

        let Some(max_total_charge_usd) = self.max_total_charge_usd else {
            return Ok(requested);
        };
        let tolerance = f64::EPSILON * max_total_charge_usd.max(1.0);
        Ok((1..=requested)
            .take_while(|count| {
                self.charged_usd + *count as f64 * price_per_saved_item
                    <= max_total_charge_usd + tolerance
            })
            .count())
    }

    pub fn record_saved_items(&mut self, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let price_per_saved_item = self.event_price(ITEM_RESULT_CHARGE_EVENT)?
            + self.event_price(DEFAULT_DATASET_ITEM_EVENT)?;
        self.charged_usd += count as f64 * price_per_saved_item;
        if !self.charged_usd.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }
        Ok(())
    }

    fn event_price(&self, event_name: &str) -> Result<f64> {
        self.event_prices
            .get(event_name)
            .copied()
            .or_else(|| (event_name == DEFAULT_DATASET_ITEM_EVENT).then_some(0.0))
            .ok_or_else(|| anyhow!("Apify run did not provide a price for event {event_name}"))
    }
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read Apify API response")?;
    if !status.is_success() {
        let detail = if body.trim().is_empty() {
            format!("HTTP {}", status.as_u16())
        } else {
            body.trim().to_owned()
        };
        bail!(
            "Apify API error ({}) while trying to {operation}: {detail}",
            status.as_u16()
        );
    }
    serde_json::from_str(&body).with_context(|| format!("Apify {operation} returned invalid JSON"))
}

async fn ensure_success(response: Response, operation: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        format!("HTTP {}", status.as_u16())
    } else {
        body.trim().to_owned()
    };
    bail!(
        "Apify API error ({}) while trying to {operation}: {detail}",
        status.as_u16()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver},
            Arc,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: Url,
        requests: Receiver<String>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (request_sender, requests) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                for response in responses {
                    let (mut stream, _) = loop {
                        if stopped.load(Ordering::Relaxed) || Instant::now() >= deadline {
                            return;
                        }
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(_) => return,
                        }
                    };
                    let request = read_request(&mut stream).unwrap_or_default();
                    if request_sender.send(request).is_err() {
                        return;
                    }
                    let reason = if response.status == 200 {
                        "OK"
                    } else {
                        "Created"
                    };
                    let reply = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    let _ = stream.write_all(reply.as_bytes());
                }
            });
            Self {
                base_url: Url::parse(&format!("http://{address}")).unwrap(),
                requests,
                stop,
                thread: Some(thread),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.try_iter().collect()
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        let mut content_length = 0;
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                if content_length == 0 {
                    let headers = String::from_utf8_lossy(&bytes[..header_end]);
                    content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                }
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn mock_response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    fn request_body(request: &str) -> &str {
        request
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .unwrap_or_default()
    }

    fn ppe_run(max_total_charge_usd: f64, charged_event_counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "item-result": {"eventPriceUsd": 0.0002},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                        "apify-actor-start": {"eventPriceUsd": 0.005}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_total_charge_usd},
                "chargedEventCounts": charged_event_counts
            }
        })
    }

    #[test]
    fn charges_custom_and_default_dataset_events_against_one_user_budget() {
        let run = ppe_run(0.0061, json!({"item-result": 1, "apify-actor-start": 1}));
        let ChargingBudget::PayPerEvent(mut budget) = ChargingBudget::from_run(&run).unwrap()
        else {
            panic!("expected PPE budget");
        };

        assert_eq!(budget.affordable_items(10).unwrap(), 3);
        budget.record_saved_items(2).unwrap();
        assert_eq!(budget.affordable_items(10).unwrap(), 1);
    }

    #[test]
    fn tiered_events_use_highest_tier_prices_for_current_and_prior_charges() {
        let mut run = ppe_run(0.006, json!({"apify-actor-start": 1, "item-result": 1}));
        let events = run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap();
        for event_name in [ITEM_RESULT_CHARGE_EVENT, DEFAULT_DATASET_ITEM_EVENT] {
            events.insert(
                event_name.to_owned(),
                json!({"eventTieredPricingUsd": {
                    "FREE": {"tieredEventPriceUsd": 0.0001},
                    "GOLD": {"tieredEventPriceUsd": 0.0003}
                }}),
            );
        }

        let ChargingBudget::PayPerEvent(mut budget) = ChargingBudget::from_run(&run).unwrap()
        else {
            panic!("expected PPE budget");
        };

        assert_eq!(budget.affordable_items(3).unwrap(), 1);
        budget.record_saved_items(1).unwrap();
        assert_eq!(budget.affordable_items(3).unwrap(), 0);
    }

    #[test]
    fn missing_default_dataset_price_contributes_zero_to_budget() {
        let mut run = ppe_run(
            0.0052,
            json!({"apify-actor-start": 1, "apify-default-dataset-item": 1}),
        );
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove(DEFAULT_DATASET_ITEM_EVENT);
        let ChargingBudget::PayPerEvent(budget) = ChargingBudget::from_run(&run).unwrap() else {
            panic!("expected PPE budget");
        };

        assert_eq!(budget.affordable_items(2).unwrap(), 1);
    }

    #[test]
    fn non_ppe_runs_do_not_apply_result_event_limits() {
        let budget = ChargingBudget::from_run(&json!({
            "data": {"pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM"}}
        }))
        .unwrap();
        assert_eq!(budget, ChargingBudget::NonPayPerEvent);
    }

    #[test]
    fn missing_ppe_charge_prices_fail_closed() {
        let mut run = ppe_run(1.0, json!({}));
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove("item-result");
        let ChargingBudget::PayPerEvent(budget) = ChargingBudget::from_run(&run).unwrap() else {
            panic!("expected PPE budget");
        };
        assert!(budget.affordable_items(1).is_err());
    }

    #[test]
    fn zero_spending_limit_is_unbounded() {
        let zero = ppe_run(0.0, json!({"apify-actor-start": 1}));
        let ChargingBudget::PayPerEvent(budget) = ChargingBudget::from_run(&zero).unwrap() else {
            panic!("expected PPE budget");
        };
        assert_eq!(budget.affordable_items(10).unwrap(), 10);
    }

    #[test]
    fn null_and_missing_spending_limits_are_unbounded() {
        let mut null = ppe_run(1.0, json!({"apify-actor-start": 1}));
        null["data"]["options"]["maxTotalChargeUsd"] = Value::Null;

        let mut missing = ppe_run(1.0, json!({"apify-actor-start": 1}));
        missing["data"]["options"]
            .as_object_mut()
            .unwrap()
            .remove("maxTotalChargeUsd");

        for run in [null, missing] {
            let ChargingBudget::PayPerEvent(budget) = ChargingBudget::from_run(&run).unwrap()
            else {
                panic!("expected PPE budget");
            };
            assert_eq!(budget.affordable_items(10).unwrap(), 10);
        }
    }

    #[tokio::test]
    async fn sends_idempotent_item_result_charge_to_current_run() {
        let server = MockServer::start(vec![mock_response(201, "{}")]);
        let client = ApifyClient::new(server.base_url.clone(), "test-token".to_owned()).unwrap();
        client
            .charge_event("test-run", ITEM_RESULT_CHARGE_EVENT, 2)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert!(request.starts_with("POST /v2/actor-runs/test-run/charge "));
        assert!(request.to_ascii_lowercase().contains("idempotency-key: "));
        assert_eq!(
            serde_json::from_str::<Value>(request_body(request)).unwrap(),
            json!({"eventName": "item-result", "count": 2})
        );
    }
}
