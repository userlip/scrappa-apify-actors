use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Method, Response, StatusCode, Url};
use serde_json::{json, Value};
use std::{env, time::Duration};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const APIFY_MAX_RETRIES: usize = 2;
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub struct ApifyClient {
    http: Client,
    base_url: Url,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    pub fn from_env() -> Result<Self> {
        let token = required_env("APIFY_TOKEN")?;
        let actor_run_id = required_env("ACTOR_RUN_ID")?;
        let key_value_store_id = required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?;
        let dataset_id = required_env("ACTOR_DEFAULT_DATASET_ID")?;
        let base_url = env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT);
        let input_key = env_or_default("ACTOR_INPUT_KEY", "INPUT");

        Self::new(
            &base_url,
            token,
            actor_run_id,
            key_value_store_id,
            dataset_id,
            input_key,
        )
    }

    pub fn new(
        base_url: &str,
        token: String,
        actor_run_id: String,
        key_value_store_id: String,
        dataset_id: String,
        input_key: String,
    ) -> Result<Self> {
        Ok(Self {
            http: Client::builder().timeout(APIFY_REQUEST_TIMEOUT).build()?,
            base_url: Url::parse(base_url.trim_end_matches('/'))?,
            token,
            actor_run_id,
            key_value_store_id,
            dataset_id,
            input_key,
        })
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
            .send_with_retry(Method::GET, url, None, "input retrieval")
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = require_success(response, "input retrieval").await?;
        response
            .json::<Value>()
            .await
            .context("Apify input record was not valid JSON")
            .map(Some)
    }

    pub fn actor_run_id(&self) -> &str {
        &self.actor_run_id
    }

    pub async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let response = self
            .send_with_retry(Method::GET, url, None, "run pricing request")
            .await?;
        let response = require_success(response, "run pricing request").await?;
        response
            .json::<Value>()
            .await
            .context("Apify run pricing response was not valid JSON")
    }

    pub async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(item)
            .send()
            .await
            .context("Failed to publish dataset item to Apify API")?;
        require_success(response, "dataset item publication").await?;
        Ok(())
    }

    pub async fn charge_event(
        &self,
        event_name: &str,
        count: u64,
        idempotency_key: &str,
    ) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id, "charge"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .header("idempotency-key", idempotency_key)
            .json(&json!({"eventName": event_name, "count": count}))
            .send()
            .await
            .context("Apify run event charge request failed")?;
        require_success(response, "event charge").await?;
        Ok(())
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        self.put_record("OUTPUT", output).await
    }

    pub async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let body = json!({
            "runId": self.actor_run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true
        });
        let response = self
            .send_with_retry(
                Method::PUT,
                url,
                Some(body),
                "run status message publication",
            )
            .await?;
        require_success(response, "run status message publication").await?;
        Ok(())
    }

    async fn put_record(&self, key: &str, value: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            key,
        ])?;
        let response = self
            .send_with_retry(
                Method::PUT,
                url,
                Some(value.clone()),
                &format!("{key} record publication"),
            )
            .await?;
        require_success(response, &format!("{key} record publication")).await?;
        Ok(())
    }

    async fn send_with_retry(
        &self,
        method: Method,
        url: Url,
        body: Option<Value>,
        operation: &str,
    ) -> Result<Response> {
        let mut retry_count = 0;
        loop {
            let mut request = self
                .http
                .request(method.clone(), url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json");
            if let Some(body) = &body {
                request = request.json(body);
            }

            let response = match request.send().await {
                Ok(response) => response,
                Err(error) if can_retry_transport(&method, &error, retry_count) => {
                    tokio::time::sleep(apify_retry_delay(retry_count)).await;
                    retry_count += 1;
                    continue;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify {operation} failed"));
                }
            };

            if let Some(delay) =
                apify_retry_delay_for_status(&method, response.status(), retry_count)
            {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }
            return Ok(response);
        }
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|_| anyhow!("Apify API base URL cannot contain path segments"))?;
            path.pop_if_empty();
            for segment in segments {
                path.push(segment);
            }
        }
        Ok(url)
    }
}

async fn require_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

fn can_retry_transport(method: &Method, error: &reqwest::Error, retry_count: usize) -> bool {
    matches!(*method, Method::GET | Method::PUT)
        && retry_count < APIFY_MAX_RETRIES
        && (error.is_connect() || error.is_timeout())
}

fn apify_retry_delay_for_status(
    method: &Method,
    status: StatusCode,
    retry_count: usize,
) -> Option<Duration> {
    if !matches!(*method, Method::GET | Method::PUT)
        || retry_count >= APIFY_MAX_RETRIES
        || !matches!(
            status,
            StatusCode::TOO_MANY_REQUESTS
                | StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT
                | StatusCode::REQUEST_TIMEOUT
        )
    {
        return None;
    }
    Some(apify_retry_delay(retry_count))
}

fn apify_retry_delay(retry_count: usize) -> Duration {
    Duration::from_secs(1_u64 << retry_count.min(3))
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

#[cfg(test)]
mod tests {
    use super::ApifyClient;
    use serde_json::{json, Value};
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex},
        thread,
    };

    #[derive(Clone)]
    struct MockResponse {
        status: u16,
        body: String,
    }

    impl MockResponse {
        fn json(status: u16, body: Value) -> Self {
            Self {
                status,
                body: body.to_string(),
            }
        }
    }

    fn mock_server(responses: Vec<MockResponse>) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut buffer = [0_u8; 4096];
                loop {
                    let read = stream.read(&mut buffer).unwrap();
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                    if request_complete(&request) {
                        break;
                    }
                }
                captured
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&request).to_string());
                let reason = if response.status < 300 {
                    "OK"
                } else if response.status == 503 {
                    "Service Unavailable"
                } else {
                    "Error"
                };
                write!(
                    stream,
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.status,
                    reason,
                    response.body.len(),
                    response.body
                )
                .unwrap();
            }
        });
        (format!("http://{address}"), requests)
    }

    fn request_complete(request: &[u8]) -> bool {
        let Some(headers_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
            return false;
        };
        let headers = String::from_utf8_lossy(&request[..headers_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        request.len() >= headers_end + 4 + content_length
    }

    fn client(base_url: &str) -> ApifyClient {
        ApifyClient::new(
            base_url,
            "test-token".to_owned(),
            "run".to_owned(),
            "store".to_owned(),
            "dataset".to_owned(),
            "INPUT".to_owned(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn retrieves_input_from_the_configured_apify_key_value_store() {
        let (base_url, requests) =
            mock_server(vec![MockResponse::json(200, json!({"item_id": "123"}))]);
        let input = client(&base_url).get_input().await.unwrap().unwrap();

        assert_eq!(input, json!({"item_id": "123"}));
        let requests = requests.lock().unwrap();
        assert!(requests[0].starts_with("GET /v2/key-value-stores/store/records/INPUT HTTP/1.1"));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token"));
    }

    #[tokio::test]
    async fn writes_dataset_output_and_event_charge_with_idempotency() {
        let (base_url, requests) = mock_server(vec![
            MockResponse::json(201, json!({})),
            MockResponse::json(200, json!({})),
            MockResponse::json(201, json!({})),
            MockResponse::json(200, json!({})),
        ]);
        let apify = client(&base_url);
        apify
            .push_dataset_item(&json!({"request_success": true}))
            .await
            .unwrap();
        apify
            .charge_event("item-detail-result", 1, "run-item-detail-result-0")
            .await
            .unwrap();
        apify.set_output(&json!({"requested": 1})).await.unwrap();
        apify
            .set_status_message("Charge limit reached")
            .await
            .unwrap();

        let requests = requests.lock().unwrap();
        assert!(requests[0].starts_with("POST /v2/datasets/dataset/items HTTP/1.1"));
        assert!(requests[0].ends_with("{\"request_success\":true}"));
        assert!(requests[1].starts_with("POST /v2/actor-runs/run/charge HTTP/1.1"));
        assert!(requests[1]
            .to_ascii_lowercase()
            .contains("idempotency-key: run-item-detail-result-0"));
        assert!(requests[1].ends_with("{\"count\":1,\"eventName\":\"item-detail-result\"}"));
        assert!(requests[2].starts_with("PUT /v2/key-value-stores/store/records/OUTPUT HTTP/1.1"));
        assert!(requests[3].starts_with("PUT /v2/actor-runs/run HTTP/1.1"));
        assert!(requests[3].ends_with("{\"isStatusMessageTerminal\":true,\"runId\":\"run\",\"statusMessage\":\"Charge limit reached\"}"));
    }

    #[tokio::test]
    async fn missing_input_record_returns_none() {
        let (base_url, _) =
            mock_server(vec![MockResponse::json(404, json!({"error": "not found"}))]);
        assert_eq!(client(&base_url).get_input().await.unwrap(), None);
    }
}
