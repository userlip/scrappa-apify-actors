use std::{env, time::Duration};

use reqwest::{Client, Method, Response, StatusCode, Url, header};
use serde_json::{Value, json};

use crate::charging::TRANSLATION_RESULT_CHARGE_EVENT;
use crate::{
    charging::{ChargingManager, DEFAULT_DATASET_ITEM_CHARGE_EVENT},
    results::TranslationDatasetItem,
    run_translations::{PushTranslationResult, TranslationOutput},
};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const CHARGE_MAX_ATTEMPTS: usize = 3;
const CHARGE_TOTAL_DEADLINE: Duration = Duration::from_secs(60);
const CHARGE_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const CHARGE_RETRY_DELAYS: [Duration; CHARGE_MAX_ATTEMPTS - 1] =
    [Duration::from_millis(250), Duration::from_millis(500)];

pub struct ApifyClient {
    http: Client,
    api_base: Url,
    token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
}

impl ApifyClient {
    pub fn from_env() -> Result<Self, String> {
        let api_base =
            env::var("APIFY_API_PUBLIC_BASE_URL").unwrap_or_else(|_| APIFY_API_DEFAULT.to_owned());
        let token = required_env("APIFY_TOKEN")?;
        let key_value_store_id = required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?;
        let dataset_id = required_env("ACTOR_DEFAULT_DATASET_ID")?;
        let actor_run_id = required_env("ACTOR_RUN_ID")?;
        let input_key = env::var("ACTOR_INPUT_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "INPUT".to_owned());
        Self::new(
            &api_base,
            token,
            key_value_store_id,
            dataset_id,
            actor_run_id,
            input_key,
        )
    }

    pub fn new(
        api_base: &str,
        token: String,
        key_value_store_id: String,
        dataset_id: String,
        actor_run_id: String,
        input_key: String,
    ) -> Result<Self, String> {
        let api_base = Url::parse(api_base)
            .map_err(|_| "APIFY_API_PUBLIC_BASE_URL must be a valid absolute URL".to_owned())?;
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| format!("Could not create Apify HTTP client: {error}"))?;

        Ok(Self {
            http,
            api_base,
            token,
            key_value_store_id,
            dataset_id,
            actor_run_id,
            input_key,
        })
    }

    pub async fn get_run_pricing(&self) -> Result<Value, String> {
        let url = self.resource_url(&["actor-runs", &self.actor_run_id])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .map_err(|error| format!("Apify run pricing request failed: {error}"))?;
        response_json(response, "Apify run pricing request").await
    }

    pub async fn get_input(&self) -> Result<Option<Value>, String> {
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
            .map_err(|error| format!("Apify INPUT request failed: {error}"))?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "fetch Actor input").await?;
        let body = response
            .text()
            .await
            .map_err(|error| format!("Failed to read Actor input response: {error}"))?;
        serde_json::from_str(&body)
            .map(Some)
            .map_err(|_| "Actor input record is not valid JSON".to_owned())
    }

    pub async fn push_dataset_item(&self, item: &TranslationDatasetItem) -> Result<(), String> {
        let url = self.resource_url(&["datasets", &self.dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(item)
            .send()
            .await
            .map_err(|error| format!("Apify dataset write failed: {error}"))?;
        successful_response(response, "store dataset item").await?;
        Ok(())
    }

    pub async fn set_output(&self, output: &Value) -> Result<(), String> {
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
            .map_err(|error| {
                format!("Failed to write OUTPUT to the default key-value store: {error}")
            })?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn charge_event(
        &self,
        event_name: &str,
        count: u64,
        idempotency_key: &str,
    ) -> Result<(), String> {
        let url = self.resource_url(&["actor-runs", &self.actor_run_id, "charge"])?;
        let request_body = json!({"eventName": event_name, "count": count});
        let result = tokio::time::timeout(CHARGE_TOTAL_DEADLINE, async {
            for attempt in 0..CHARGE_MAX_ATTEMPTS {
                let response = self
                    .request(Method::POST, url.clone())
                    .timeout(CHARGE_REQUEST_TIMEOUT)
                    .header("idempotency-key", idempotency_key)
                    .json(&request_body)
                    .send()
                    .await;

                let retry_message = match response {
                    Ok(response) if response.status().is_success() => return Ok(()),
                    Ok(response) => {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        let message = api_error(status, &body, "charge Actor event");
                        if !retryable_charge_status(status) {
                            return Err(message);
                        }
                        message
                    }
                    Err(error) => {
                        let message = format!("Apify event charge request failed: {error}");
                        if !(error.is_timeout() || error.is_connect() || error.is_request()) {
                            return Err(message);
                        }
                        message
                    }
                };

                if attempt + 1 == CHARGE_MAX_ATTEMPTS {
                    return Err(retry_message);
                }
                eprintln!(
                    "{retry_message}; retrying charge with the same idempotency key (attempt {}/{CHARGE_MAX_ATTEMPTS})",
                    attempt + 2
                );
                tokio::time::sleep(CHARGE_RETRY_DELAYS[attempt]).await;
            }

            unreachable!("charge retry loop always returns or exhausts its attempts")
        })
        .await;

        match result {
            Ok(result) => result,
            Err(_) => Err("Apify event charge request exceeded its 60 second deadline".to_owned()),
        }
    }

    pub async fn set_terminal_status_message(&self, message: &str) -> Result<(), String> {
        let url = self.resource_url(&["actor-runs", &self.actor_run_id])?;
        let response = self
            .request(Method::PUT, url)
            .timeout(Duration::from_secs(1))
            .json(&json!({
                "statusMessage": message,
                "isStatusMessageTerminal": true
            }))
            .send()
            .await
            .map_err(|error| format!("Apify status message update failed: {error}"))?;
        successful_response(response, "set Actor run status message").await?;
        Ok(())
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.http
            .request(method, url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
    }

    fn resource_url(&self, segments: &[&str]) -> Result<Url, String> {
        let mut url = self.api_base.clone();
        let mut path = url
            .path_segments_mut()
            .map_err(|_| "APIFY_API_PUBLIC_BASE_URL cannot contain path segments".to_owned())?;
        path.pop_if_empty();
        path.extend(std::iter::once("v2").chain(segments.iter().copied()));
        drop(path);
        Ok(url)
    }
}

pub struct ApifyTranslationOutput<'a> {
    client: &'a ApifyClient,
    charging: ChargingManager,
}

impl<'a> ApifyTranslationOutput<'a> {
    pub fn new(client: &'a ApifyClient, charging: ChargingManager) -> Self {
        Self { client, charging }
    }
}

impl TranslationOutput for ApifyTranslationOutput<'_> {
    fn charge_limit_status(&self, processed: usize, requested: usize) -> Option<String> {
        self.charging.charge_limit_status(processed, requested)
    }

    async fn push_translation_result(
        &mut self,
        item: &TranslationDatasetItem,
    ) -> Result<PushTranslationResult, String> {
        if !self.charging.is_pay_per_event() {
            self.client.push_dataset_item(item).await?;
            return Ok(PushTranslationResult {
                saved: true,
                status_message: None,
            });
        }

        let plan = self.charging.plan_dataset_push(item.success);
        if !plan.keep_item {
            if item.success {
                let status_message = format!(
                    "Charge limit reached before saving translation {}; stopping batch without writing uncharged success results.",
                    item.index + 1
                );
                eprintln!(
                    "{status_message} {{\"event\":\"{TRANSLATION_RESULT_CHARGE_EVENT}\",\"charged_count\":0}}"
                );
                return Ok(PushTranslationResult {
                    saved: false,
                    status_message: Some(status_message),
                });
            }
            // The existing TypeScript wrapper ignores pushData's ChargeResult for failure rows.
            return Ok(PushTranslationResult {
                saved: true,
                status_message: None,
            });
        }

        // Check charge configuration before persisting the result, then charge only after
        // Apify confirms the dataset append.
        for event_name in &plan.events_to_charge {
            if event_name != DEFAULT_DATASET_ITEM_CHARGE_EVENT
                && !self.charging.has_event_price(event_name)
            {
                return Err(format!(
                    "Apify PAY_PER_EVENT run did not provide a price for required event {event_name}"
                ));
            }
        }

        self.client.push_dataset_item(item).await?;
        if plan
            .events_to_charge
            .iter()
            .any(|event_name| event_name == DEFAULT_DATASET_ITEM_CHARGE_EVENT)
        {
            self.charging
                .record_charge(DEFAULT_DATASET_ITEM_CHARGE_EVENT, 1)?;
        }

        for event_name in &plan.events_to_charge {
            if event_name == DEFAULT_DATASET_ITEM_CHARGE_EVENT {
                continue;
            }

            let idempotency_key = format!(
                "{}-{}-translation-{}",
                self.client.actor_run_id, event_name, item.index
            );
            self.client
                .charge_event(event_name, 1, &idempotency_key)
                .await?;
            self.charging.record_charge(event_name, 1)?;
        }

        Ok(PushTranslationResult {
            saved: true,
            status_message: None,
        })
    }
}

fn retryable_charge_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 425 | 429 | 500 | 502 | 503 | 504)
}

async fn response_json(response: Response, operation: &str) -> Result<Value, String> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Failed to read {operation} response: {error}"))?;
    if !status.is_success() {
        return Err(api_error(status, &body, operation));
    }
    serde_json::from_str(&body).map_err(|_| format!("{operation} returned invalid JSON"))
}

async fn successful_response(response: Response, operation: &str) -> Result<Response, String> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    Err(api_error(status, &body, operation))
}

fn api_error(status: StatusCode, body: &str, operation: &str) -> String {
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    format!(
        "Apify API error ({}) while trying to {operation}: {reason}{detail}",
        status.as_u16()
    )
}

fn required_env(name: &str) -> Result<String, String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Required environment variable {name} is missing"))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver, Sender},
        thread::{self, JoinHandle},
    };

    use super::*;

    struct MockRequest {
        method: String,
        target: String,
        headers: BTreeMap<String, String>,
        body: String,
    }

    struct MockServer {
        base_url: String,
        requests: Receiver<MockRequest>,
        stop: Sender<()>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<(u16, String)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            listener.set_nonblocking(true).unwrap();
            let (sender, requests) = mpsc::channel();
            let (stop, stopped) = mpsc::channel();
            let thread = thread::spawn(move || {
                let mut served = 0;
                loop {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let request = read_mock_request(&mut stream);
                            sender.send(request).unwrap();
                            let (status, body) = responses
                                .get(served)
                                .cloned()
                                .unwrap_or((500, String::new()));
                            served += 1;
                            let reason = match status {
                                200 => "OK",
                                201 => "Created",
                                500 => "Internal Server Error",
                                _ => "Error",
                            };
                            write!(
                                stream,
                                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                                body.len()
                            )
                            .unwrap();
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if !matches!(stopped.try_recv(), Err(mpsc::TryRecvError::Empty)) {
                                break;
                            }
                            thread::sleep(std::time::Duration::from_millis(1));
                        }
                        Err(error) => panic!("mock server accept failed: {error}"),
                    }
                }
            });
            Self {
                base_url: format!("http://{address}"),
                requests,
                stop,
                thread: Some(thread),
            }
        }

        fn finish(mut self) -> Vec<MockRequest> {
            let _ = self.stop.send(());
            self.thread.take().unwrap().join().unwrap();
            self.requests.try_iter().collect()
        }
    }

    fn read_mock_request(stream: &mut TcpStream) -> MockRequest {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
            if let Some(header_end) = header_end {
                let header_text = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length = header_text
                    .lines()
                    .skip(1)
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0, "mock request ended before its body was read");
            bytes.extend_from_slice(&buffer[..count]);
        }
        let header_end = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let header_text = String::from_utf8_lossy(&bytes[..header_end]);
        let mut lines = header_text.lines();
        let mut request_line = lines.next().unwrap().split_whitespace();
        let method = request_line.next().unwrap().to_owned();
        let target = request_line.next().unwrap().to_owned();
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        let body = String::from_utf8(bytes[header_end + 4..].to_vec()).unwrap();
        MockRequest {
            method,
            target,
            headers,
            body,
        }
    }

    fn apify_client(base_url: &str) -> ApifyClient {
        ApifyClient::new(
            base_url,
            "test-token".to_owned(),
            "store-id".to_owned(),
            "dataset-id".to_owned(),
            "test-run".to_owned(),
            "INPUT".to_owned(),
        )
        .unwrap()
    }

    fn ppe_charging() -> ChargingManager {
        ChargingManager::from_run(&json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "translation-result": {"eventPriceUsd": 0.00025},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.001},
                "chargedEventCounts": {}
            }
        }))
        .unwrap()
    }

    fn translation_item() -> TranslationDatasetItem {
        TranslationDatasetItem {
            success: true,
            index: 3,
            text: "Good morning".to_owned(),
            translated_text: Some("Guten Morgen".to_owned()),
            source: "en".to_owned(),
            target: "de".to_owned(),
            error: None,
            status_code: None,
        }
    }

    #[tokio::test]
    async fn saves_dataset_item_before_charging_translation_result() {
        let server = MockServer::start(vec![(201, String::new()), (200, String::new())]);
        let client = apify_client(&server.base_url);
        let mut output = ApifyTranslationOutput::new(&client, ppe_charging());

        let result = output
            .push_translation_result(&translation_item())
            .await
            .unwrap();
        assert!(result.saved);

        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].target, "/v2/datasets/dataset-id/items");
        assert_eq!(requests[1].method, "POST");
        assert_eq!(requests[1].target, "/v2/actor-runs/test-run/charge");
        assert_eq!(
            requests[1].headers["idempotency-key"],
            "test-run-translation-result-translation-3"
        );
        assert_eq!(
            serde_json::from_str::<Value>(&requests[1].body).unwrap(),
            json!({"eventName": "translation-result", "count": 1})
        );
    }

    #[tokio::test]
    async fn does_not_charge_or_retry_when_dataset_append_fails() {
        let server = MockServer::start(vec![(500, "dataset failed".to_owned())]);
        let client = apify_client(&server.base_url);
        let mut output = ApifyTranslationOutput::new(&client, ppe_charging());

        let error = output
            .push_translation_result(&translation_item())
            .await
            .unwrap_err();
        assert!(error.contains("Apify API error (500)"));

        let requests = server.finish();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "POST");
        assert_eq!(requests[0].target, "/v2/datasets/dataset-id/items");
        assert!(
            requests
                .iter()
                .all(|request| request.target != "/v2/actor-runs/test-run/charge")
        );
    }
}
