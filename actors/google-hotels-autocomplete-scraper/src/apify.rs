use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, StatusCode};
use serde_json::{json, Value};
use std::{
    env,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

pub const APIFY_API_BASE_URL: &str = "https://api.apify.com";
pub const RESULT_CHARGE_EVENT: &str = "hotel-suggestion-result";
const DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const APIFY_MAX_ATTEMPTS: u8 = 3;
const DATASET_POST_MAX_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
pub enum PricingMode {
    NonPayPerEvent,
    PayPerEvent(EventBudget),
}

#[derive(Debug)]
pub enum ChargedDatasetError {
    DatasetWrite(anyhow::Error),
    EventCharge(anyhow::Error),
}

#[derive(Debug)]
pub struct EventBudget {
    event_name: String,
    event_price_usd: f64,
    dataset_item_price_usd: f64,
    charged_usd: f64,
    max_total_charge_usd: Option<f64>,
}

impl EventBudget {
    pub fn from_run(run: &Value, max_total_charge_env: Option<f64>) -> Result<PricingMode> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        if data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(PricingMode::NonPayPerEvent);
        }

        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let event = events.get(RESULT_CHARGE_EVENT).ok_or_else(|| {
            anyhow!("Apify run did not provide the {RESULT_CHARGE_EVENT} event price")
        })?;
        let event_price_usd = event
            .get("eventPriceUsd")
            .and_then(Value::as_f64)
            .ok_or_else(|| {
                anyhow!("Apify run did not provide the {RESULT_CHARGE_EVENT} event price")
            })?;
        if !event_price_usd.is_finite() || event_price_usd < 0.0 {
            bail!("Apify run returned an invalid {RESULT_CHARGE_EVENT} event price");
        }
        let dataset_item_price_usd = events
            .get(DATASET_ITEM_CHARGE_EVENT)
            .map(|event| {
                event
                    .get("eventPriceUsd")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| {
                        anyhow!(
                            "Apify run did not provide the {DATASET_ITEM_CHARGE_EVENT} event price"
                        )
                    })
            })
            .transpose()?
            .unwrap_or(0.0);
        if !dataset_item_price_usd.is_finite() || dataset_item_price_usd < 0.0 {
            bail!("Apify run returned an invalid {DATASET_ITEM_CHARGE_EVENT} event price");
        }
        if !(event_price_usd + dataset_item_price_usd).is_finite() {
            bail!("Apify run returned invalid combined result event prices");
        }

        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_usd = 0.0;
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
            charged_usd += price * count as f64;
        }
        if !charged_usd.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        let run_limit = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|limit| *limit != 0.0);
        let max_total_charge_usd = match (run_limit, max_total_charge_env) {
            (Some(run_limit), Some(env_limit)) => Some(run_limit.min(env_limit)),
            (Some(run_limit), None) => Some(run_limit),
            (None, Some(env_limit)) => Some(env_limit),
            (None, None) => None,
        };
        if max_total_charge_usd.is_some_and(|limit| !limit.is_finite() || limit < 0.0) {
            bail!("Apify run returned an invalid total charge limit");
        }

        Ok(PricingMode::PayPerEvent(Self {
            event_name: RESULT_CHARGE_EVENT.to_owned(),
            event_price_usd,
            dataset_item_price_usd,
            charged_usd,
            max_total_charge_usd,
        }))
    }

    pub fn event_name(&self) -> &str {
        &self.event_name
    }

    pub fn affordable_count(&self, requested: usize) -> usize {
        let Some(max_total_charge_usd) = self.max_total_charge_usd else {
            return requested;
        };
        let row_price_usd = self.event_price_usd + self.dataset_item_price_usd;
        if row_price_usd == 0.0 {
            return requested;
        }

        let tolerance = f64::EPSILON * max_total_charge_usd.max(1.0);
        let remaining_usd = max_total_charge_usd - self.charged_usd;
        if remaining_usd + tolerance < 0.0 {
            return 0;
        }
        let count = ((remaining_usd + tolerance) / row_price_usd).floor();
        if !count.is_finite() || count >= usize::MAX as f64 {
            requested
        } else {
            requested.min(count.max(0.0) as usize)
        }
    }

    pub fn record_charge(&mut self, count: usize) {
        self.charged_usd += count as f64 * (self.event_price_usd + self.dataset_item_price_usd);
    }
}

#[derive(Clone)]
pub struct ApifyClient {
    client: Client,
    api_base_url: Url,
    token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    charge_sequence: u64,
}

impl ApifyClient {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: Client,
        api_base_url: Url,
        token: String,
        key_value_store_id: String,
        dataset_id: String,
        actor_run_id: String,
        input_key: String,
    ) -> Self {
        Self {
            client,
            api_base_url,
            token,
            key_value_store_id,
            dataset_id,
            actor_run_id,
            input_key,
            charge_sequence: 0,
        }
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .send_with_retries(|| self.client.get(url.clone()))
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let input = response_json(response, "Apify INPUT request").await?;
        Ok((!input.is_null()).then_some(input))
    }

    pub async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let response = self
            .send_with_retries(|| self.client.get(url.clone()))
            .await?;
        response_json(response, "Apify run pricing request").await
    }

    pub async fn set_terminal_status_message(&self, status_message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let body = json!({
            "runId": self.actor_run_id.as_str(),
            "statusMessage": status_message,
            "isStatusMessageTerminal": true
        });
        let response = self
            .send_with_retries(|| self.client.put(url.clone()).json(&body))
            .await?;
        require_success(response, "Apify terminal status message update").await
    }

    pub async fn charge_event(&mut self, event_name: &str, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        self.charge_sequence += 1;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let idempotency_key = format!(
            "{}-{timestamp}-{}-{}",
            self.actor_run_id,
            std::process::id(),
            self.charge_sequence
        );
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id, "charge"])?;
        let body = json!({"eventName": event_name, "count": count});
        let response = self
            .send_with_retries(|| {
                self.client
                    .post(url.clone())
                    .header("idempotency-key", &idempotency_key)
                    .json(&body)
            })
            .await?;
        require_success(response, "Apify event charge").await
    }

    pub async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let mut chunk_start = 0;
        while chunk_start < items.len() {
            let mut chunk_end = chunk_start;
            let mut chunk_size = 2;
            while chunk_end < items.len() {
                let item_size = serde_json::to_vec(&items[chunk_end])?.len();
                if chunk_end > chunk_start && chunk_size + item_size > DATASET_POST_MAX_BYTES {
                    break;
                }
                if item_size > DATASET_POST_MAX_BYTES {
                    bail!("Dataset item exceeds Apify's 5 MB API payload limit");
                }
                chunk_size += item_size;
                chunk_end += 1;
            }
            let body = &items[chunk_start..chunk_end];
            let response = self
                .send_with_retries(|| self.client.post(url.clone()).json(body))
                .await?;
            require_success(response, "Apify dataset item publication").await?;
            chunk_start = chunk_end;
        }
        Ok(())
    }

    pub async fn push_charged_dataset_items(
        &mut self,
        budget: &mut EventBudget,
        items: &[Value],
    ) -> std::result::Result<usize, ChargedDatasetError> {
        let count = budget.affordable_count(items.len());
        if count == 0 {
            return Ok(0);
        }

        self.push_dataset_items(&items[..count])
            .await
            .map_err(ChargedDatasetError::DatasetWrite)?;
        self.charge_event(budget.event_name(), count)
            .await
            .map_err(ChargedDatasetError::EventCharge)?;
        budget.record_charge(count);
        Ok(count)
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
            .send_with_retries(|| self.client.put(url.clone()).json(output))
            .await?;
        require_success(response, "Apify OUTPUT write").await
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.api_base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("Apify API base URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(segments.iter().copied());
        Ok(url)
    }

    async fn send_with_retries<F>(&self, mut request: F) -> Result<Response>
    where
        F: FnMut() -> reqwest::RequestBuilder,
    {
        for attempt in 1..=APIFY_MAX_ATTEMPTS {
            let response = request()
                .bearer_auth(&self.token)
                .timeout(APIFY_REQUEST_TIMEOUT)
                .send()
                .await;
            match response {
                Ok(response)
                    if response.status().is_success()
                        || !is_retryable_status(response.status())
                        || attempt == APIFY_MAX_ATTEMPTS =>
                {
                    return Ok(response);
                }
                Ok(_) => {}
                Err(error)
                    if attempt < APIFY_MAX_ATTEMPTS
                        && (error.is_timeout() || error.is_connect() || error.is_request()) =>
                {
                    eprintln!(
                        "Apify API request failed: {error}; retrying attempt {}/{}",
                        attempt + 1,
                        APIFY_MAX_ATTEMPTS
                    );
                }
                Err(error) => return Err(anyhow!("Apify API request failed: {error}")),
            }
            if attempt < APIFY_MAX_ATTEMPTS {
                let delay = Duration::from_secs(u64::from(attempt));
                tokio::time::sleep(delay).await;
            }
        }
        Err(anyhow!("Apify API request failed after retries"))
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 425 | 429 | 500 | 502 | 503 | 504)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
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
    response
        .json()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

async fn require_success(response: Response, operation: &str) -> Result<()> {
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

pub fn max_total_charge_from_env() -> Result<Option<f64>> {
    let Ok(value) = env::var("ACTOR_MAX_TOTAL_CHARGE_USD") else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    let value = value
        .parse::<f64>()
        .context("ACTOR_MAX_TOTAL_CHARGE_USD must be a number")?;
    if !value.is_finite() || value < 0.0 {
        bail!("ACTOR_MAX_TOTAL_CHARGE_USD must be a non-negative finite number");
    }
    Ok(Some(value))
}

pub fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{mpsc, Arc, Mutex},
        thread::{self, JoinHandle},
        time::Instant,
    };

    struct MockServer {
        base_url: Url,
        requests: Arc<Mutex<mpsc::Receiver<String>>>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<(u16, String)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (request_sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                for (status, body) in responses {
                    let deadline = Instant::now() + Duration::from_secs(10);
                    let (mut stream, _) = loop {
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                if Instant::now() >= deadline {
                                    return;
                                }
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(_) => return,
                        }
                    };
                    let request = read_request(&mut stream).unwrap_or_default();
                    let _ = request_sender.send(request);
                    let reason = match status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        500 => "Internal Server Error",
                        _ => "Mock Response",
                    };
                    let response = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
            });
            Self {
                base_url: Url::parse(&format!("http://{address}")).unwrap(),
                requests: Arc::new(Mutex::new(requests)),
                thread: Some(thread),
            }
        }

        fn next_request(&self) -> String {
            self.requests
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        loop {
            let count = stream.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n")
            else {
                continue;
            };
            let header_end = header_end + 4;
            let headers = String::from_utf8_lossy(&request[..header_end]).to_ascii_lowercase();
            let content_length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if request.len() >= header_end + content_length {
                break;
            }
        }
        Ok(String::from_utf8_lossy(&request).into_owned())
    }

    fn run_data(max_total: Value, charged_counts: Value) -> Value {
        json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "hotel-suggestion-result": {"eventPriceUsd": 0.00025},
                    "other-event": {"eventPriceUsd": 0.001}
                }}
            },
            "options": {"maxTotalChargeUsd": max_total},
            "chargedEventCounts": charged_counts
        }})
    }

    #[test]
    fn caps_event_budget_after_including_all_prior_charges() {
        let mut run = run_data(
            json!(0.0033),
            json!({
                "hotel-suggestion-result": 1,
                "apify-default-dataset-item": 2,
                "other-event": 2
            }),
        );
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            [DATASET_ITEM_CHARGE_EVENT] = json!({"eventPriceUsd": 0.0002});
        let PricingMode::PayPerEvent(mut budget) = EventBudget::from_run(&run, None).unwrap()
        else {
            panic!("expected pay-per-event pricing");
        };

        assert_eq!(budget.event_name(), RESULT_CHARGE_EVENT);
        assert_eq!(budget.affordable_count(100), 1);
        budget.record_charge(1);
        assert_eq!(budget.affordable_count(1), 0);
    }

    #[test]
    fn caps_rows_by_the_combined_result_and_dataset_item_prices() {
        let mut run = run_data(json!(0.0009), json!({}));
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            [DATASET_ITEM_CHARGE_EVENT] = json!({"eventPriceUsd": 0.0002});
        let PricingMode::PayPerEvent(budget) = EventBudget::from_run(&run, None).unwrap() else {
            panic!("expected pay-per-event pricing");
        };

        assert_eq!(budget.affordable_count(100), 2);
    }

    #[test]
    fn uses_the_lower_run_or_environment_spending_limit() {
        let run = run_data(json!(0.01), json!({}));
        let PricingMode::PayPerEvent(budget) = EventBudget::from_run(&run, Some(0.0005)).unwrap()
        else {
            panic!("expected pay-per-event pricing");
        };

        assert_eq!(budget.affordable_count(100), 2);
    }

    #[test]
    fn supports_an_unlimited_limit_and_zero_price_event() {
        let run = run_data(Value::Null, json!({}));
        let PricingMode::PayPerEvent(budget) = EventBudget::from_run(&run, None).unwrap() else {
            panic!("expected pay-per-event pricing");
        };
        assert_eq!(budget.affordable_count(17), 17);

        let mut free_event = run.clone();
        free_event["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            [RESULT_CHARGE_EVENT]["eventPriceUsd"] = json!(0.0);
        let PricingMode::PayPerEvent(budget) =
            EventBudget::from_run(&free_event, Some(0.0)).unwrap()
        else {
            panic!("expected pay-per-event pricing");
        };
        assert_eq!(budget.affordable_count(17), 17);
    }

    #[test]
    fn treats_zero_null_and_missing_run_limits_as_unlimited() {
        let mut missing_limit = run_data(json!(1.0), json!({}));
        missing_limit["data"]
            .as_object_mut()
            .unwrap()
            .remove("options");
        let runs = [
            ("null", run_data(Value::Null, json!({}))),
            ("missing", missing_limit),
            ("zero", run_data(json!(0.0), json!({}))),
        ];

        for (name, run) in runs {
            let PricingMode::PayPerEvent(budget) = EventBudget::from_run(&run, None).unwrap()
            else {
                panic!("expected pay-per-event pricing for {name} limit");
            };
            assert_eq!(budget.affordable_count(17), 17, "{name} limit");
        }
    }

    #[test]
    fn zero_run_limit_does_not_override_a_positive_environment_limit() {
        let run = run_data(json!(0.0), json!({}));
        let PricingMode::PayPerEvent(budget) = EventBudget::from_run(&run, Some(0.0005)).unwrap()
        else {
            panic!("expected pay-per-event pricing");
        };

        assert_eq!(budget.affordable_count(100), 2);
    }

    #[test]
    fn non_pay_per_event_models_do_not_create_custom_event_budgets() {
        let run = json!({"data": {"pricingInfo": {"pricingModel": "FREE"}}});
        assert!(matches!(
            EventBudget::from_run(&run, None).unwrap(),
            PricingMode::NonPayPerEvent
        ));
    }

    #[test]
    fn missing_custom_event_price_fails_before_work_starts() {
        let run = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {}}},
            "options": {"maxTotalChargeUsd": 1.0},
            "chargedEventCounts": {}
        }});
        assert!(EventBudget::from_run(&run, None)
            .unwrap_err()
            .to_string()
            .contains("hotel-suggestion-result event price"));
    }

    #[test]
    fn invalid_charge_counts_and_prices_are_rejected() {
        let run = run_data(json!(0.01), json!({"other-event": -1}));
        assert!(EventBudget::from_run(&run, None)
            .unwrap_err()
            .to_string()
            .contains("Invalid charged event count"));

        let run = run_data(json!(-1), json!({}));
        assert!(EventBudget::from_run(&run, None)
            .unwrap_err()
            .to_string()
            .contains("invalid total charge limit"));
    }

    #[tokio::test]
    async fn reads_input_and_pricing_then_charges_rows_and_writes_dataset_and_output() {
        let server = MockServer::start(vec![
            (200, r#"{"q":"Berlin"}"#.to_owned()),
            (200, r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"hotel-suggestion-result":{"eventPriceUsd":0.00025}}}},"options":{"maxTotalChargeUsd":0.01},"chargedEventCounts":{}}}"#.to_owned()),
            (201, "{}".to_owned()),
            (201, "{}".to_owned()),
            (200, "{}".to_owned()),
        ]);
        let mut apify = ApifyClient::new(
            Client::new(),
            server.base_url.clone(),
            "test-token".to_owned(),
            "store-test".to_owned(),
            "dataset-test".to_owned(),
            "run-test".to_owned(),
            "INPUT".to_owned(),
        );

        assert_eq!(
            apify.get_input().await.unwrap(),
            Some(json!({"q": "Berlin"}))
        );
        let run = apify.get_run().await.unwrap();
        let PricingMode::PayPerEvent(mut budget) = EventBudget::from_run(&run, None).unwrap()
        else {
            panic!("expected pay-per-event pricing");
        };
        let saved = apify
            .push_charged_dataset_items(
                &mut budget,
                &[
                    json!({"value": "Berlin"}),
                    json!({"value": "Berlin hotels"}),
                ],
            )
            .await
            .unwrap();
        assert_eq!(saved, 2);
        assert_eq!(budget.affordable_count(100), 38);
        apify
            .put_output(&json!({"suggestions_saved": 2}))
            .await
            .unwrap();

        let input = server.next_request();
        assert!(input.starts_with("GET /v2/key-value-stores/store-test/records/INPUT HTTP/1.1"));
        assert!(input
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token"));
        let pricing = server.next_request();
        assert!(pricing.starts_with("GET /v2/actor-runs/run-test HTTP/1.1"));
        let dataset = server.next_request();
        assert!(dataset.starts_with("POST /v2/datasets/dataset-test/items HTTP/1.1"));
        assert!(dataset.contains(r#"[{"value":"Berlin"},{"value":"Berlin hotels"}]"#));
        let charge = server.next_request();
        assert!(charge.starts_with("POST /v2/actor-runs/run-test/charge HTTP/1.1"));
        assert!(charge.to_ascii_lowercase().contains("idempotency-key:"));
        assert!(charge.contains(r#"{"eventName":"hotel-suggestion-result","count":2}"#));
        let output = server.next_request();
        assert!(output.starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT HTTP/1.1"));
        assert!(output.contains(r#"{"suggestions_saved":2}"#));
    }

    #[tokio::test]
    async fn saves_a_terminal_status_message_on_the_apify_run() {
        let server = MockServer::start(vec![(200, r#"{"data":{}}"#.to_owned())]);
        let apify = ApifyClient::new(
            Client::new(),
            server.base_url.clone(),
            "test-token".to_owned(),
            "store-test".to_owned(),
            "dataset-test".to_owned(),
            "run-test".to_owned(),
            "INPUT".to_owned(),
        );

        apify
            .set_terminal_status_message("Saved 2 suggestion results.")
            .await
            .unwrap();

        let request = server.next_request();
        assert!(request.starts_with("PUT /v2/actor-runs/run-test HTTP/1.1"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token"));
        let body = request.split("\r\n\r\n").nth(1).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!({
                "runId": "run-test",
                "statusMessage": "Saved 2 suggestion results.",
                "isStatusMessageTerminal": true
            })
        );
    }

    #[tokio::test]
    async fn failed_dataset_write_does_not_charge_saved_rows() {
        let server = MockServer::start(vec![(
            400,
            r#"{"error":"invalid dataset write"}"#.to_owned(),
        )]);
        let mut apify = ApifyClient::new(
            Client::new(),
            server.base_url.clone(),
            "test-token".to_owned(),
            "store-test".to_owned(),
            "dataset-test".to_owned(),
            "run-test".to_owned(),
            "INPUT".to_owned(),
        );
        let mut run = run_data(json!(0.0005), json!({}));
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            [DATASET_ITEM_CHARGE_EVENT] = json!({"eventPriceUsd": 0.00025});
        let PricingMode::PayPerEvent(mut budget) = EventBudget::from_run(&run, None).unwrap()
        else {
            panic!("expected pay-per-event pricing");
        };
        assert_eq!(budget.affordable_count(2), 1);

        let error = apify
            .push_charged_dataset_items(
                &mut budget,
                &[
                    json!({"value": "Berlin"}),
                    json!({"value": "Berlin hotels"}),
                ],
            )
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            ChargedDatasetError::DatasetWrite(error)
                if error.to_string().contains("Apify dataset item publication failed")
        ));
        assert_eq!(budget.affordable_count(2), 1);
        let dataset = server.next_request();
        assert!(dataset.starts_with("POST /v2/datasets/dataset-test/items HTTP/1.1"));
        assert!(dataset.contains(r#"[{"value":"Berlin"}]"#));
    }

    #[tokio::test]
    async fn limits_dataset_rows_and_charges_to_the_remaining_event_budget() {
        let server = MockServer::start(vec![(201, "{}".to_owned()), (201, "{}".to_owned())]);
        let mut apify = ApifyClient::new(
            Client::new(),
            server.base_url.clone(),
            "test-token".to_owned(),
            "store-test".to_owned(),
            "dataset-test".to_owned(),
            "run-test".to_owned(),
            "INPUT".to_owned(),
        );
        let run = run_data(json!(0.00025), json!({}));
        let PricingMode::PayPerEvent(mut budget) = EventBudget::from_run(&run, None).unwrap()
        else {
            panic!("expected pay-per-event pricing");
        };

        let saved = apify
            .push_charged_dataset_items(
                &mut budget,
                &[
                    json!({"value": "Berlin"}),
                    json!({"value": "Berlin hotels"}),
                ],
            )
            .await
            .unwrap();

        assert_eq!(saved, 1);
        let dataset = server.next_request();
        assert!(dataset.contains(r#"[{"value":"Berlin"}]"#));
        let charge = server.next_request();
        assert!(charge.contains(r#"{"eventName":"hotel-suggestion-result","count":1}"#));
    }

    #[tokio::test]
    async fn missing_input_record_is_reported_as_absent() {
        let server = MockServer::start(vec![(404, r#"{"error":"record not found"}"#.to_owned())]);
        let apify = ApifyClient::new(
            Client::new(),
            server.base_url.clone(),
            "test-token".to_owned(),
            "store-test".to_owned(),
            "dataset-test".to_owned(),
            "run-test".to_owned(),
            "INPUT".to_owned(),
        );

        assert_eq!(apify.get_input().await.unwrap(), None);
        assert!(server
            .next_request()
            .starts_with("GET /v2/key-value-stores/store-test/records/INPUT HTTP/1.1"));
    }
}
