use std::{time::Duration, time::SystemTime, time::UNIX_EPOCH};

use anyhow::{anyhow, Result};
use reqwest::{header, Client, StatusCode};
use serde_json::{Map, Value};
use url::Url;

use crate::config::SCRAPPA_REQUEST_TIMEOUT;

const REQUEST_USER_AGENT: &str = "thescrappa-vinted-search-scraper/1.0";
const MAX_ERROR_BODY_LENGTH: usize = 500;

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    timeout: Duration,
}

impl ScrappaTimeoutError {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl std::fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            self.timeout.as_millis()
        )
    }
}

impl std::error::Error for ScrappaTimeoutError {}

#[derive(Debug)]
enum ScrappaRequestError {
    Timeout,
    Transport(reqwest::Error),
    Http { status: StatusCode, message: String },
    InvalidJson(String),
}

impl ScrappaRequestError {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout => true,
            Self::Transport(error) => {
                error.is_timeout() || error.is_connect() || {
                    let details = format!("{error:?} {error}");
                    [
                        "ECONNRESET",
                        "ECONNREFUSED",
                        "ETIMEDOUT",
                        "ENOTFOUND",
                        "EAI_AGAIN",
                    ]
                    .iter()
                    .any(|code| details.contains(code))
                }
            }
            Self::Http { status, .. } => {
                matches!(status.as_u16(), 404 | 408 | 429 | 500 | 502 | 503 | 504)
            }
            Self::InvalidJson(_) => false,
        }
    }
}

impl std::fmt::Display for ScrappaRequestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(
                formatter,
                "{}",
                ScrappaTimeoutError::new(SCRAPPA_REQUEST_TIMEOUT)
            ),
            Self::Transport(error) => write!(formatter, "Scrappa API request failed: {error}"),
            Self::Http { status, message } => {
                write!(
                    formatter,
                    "Scrappa API error ({}): {message}",
                    status.as_u16()
                )
            }
            Self::InvalidJson(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ScrappaRequestError {}

pub struct ScrappaClient {
    client: Client,
    api_base_url: Url,
    api_key: String,
}

impl ScrappaClient {
    pub fn new(api_base_url: Url, api_key: String) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .build()
                .map_err(|error| anyhow!("Could not create Scrappa HTTP client: {error}"))?,
            api_base_url,
            api_key,
        })
    }

    pub async fn get(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
        attempts: usize,
    ) -> Result<Value> {
        let attempts = attempts.max(1);
        let mut last_error = None;

        for attempt in 1..=attempts {
            match self.send(endpoint, params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let retryable = error.is_retryable();
                    last_error = Some(error);
                    if !retryable || attempt >= attempts {
                        break;
                    }

                    let delay = retry_delay_ms(attempt - 1, retry_jitter_ms());
                    let error_message = last_error
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    eprintln!(
                        "Scrappa API request failed ({error_message}). Retrying attempt {}/{attempts} in {delay}ms.",
                        attempt + 1
                    );
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }

        match last_error {
            Some(ScrappaRequestError::Timeout) => Err(anyhow::Error::new(
                ScrappaTimeoutError::new(SCRAPPA_REQUEST_TIMEOUT),
            )),
            Some(error) => Err(anyhow!("{error}")),
            None => Err(anyhow!("Scrappa API request failed")),
        }
    }

    async fn send(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
    ) -> std::result::Result<Value, ScrappaRequestError> {
        let url = build_scrappa_url(&self.api_base_url, endpoint, params);
        let response = self
            .client
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, REQUEST_USER_AGENT)
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(map_request_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = match response.text().await {
                Ok(body) => body,
                Err(error) if error.is_timeout() => return Err(ScrappaRequestError::Timeout),
                Err(_) => String::new(),
            };
            return Err(ScrappaRequestError::Http {
                status,
                message: error_message(status, &body),
            });
        }

        response.json::<Value>().await.map_err(map_json_error)
    }
}

fn map_request_error(error: reqwest::Error) -> ScrappaRequestError {
    if error.is_timeout() {
        ScrappaRequestError::Timeout
    } else {
        ScrappaRequestError::Transport(error)
    }
}

fn map_json_error(error: reqwest::Error) -> ScrappaRequestError {
    if error.is_timeout() {
        ScrappaRequestError::Timeout
    } else if error.is_decode() {
        ScrappaRequestError::InvalidJson(format!(
            "Scrappa API response was not valid JSON: {error}"
        ))
    } else {
        ScrappaRequestError::Transport(error)
    }
}

fn build_scrappa_url(base_url: &Url, endpoint: &str, params: &Map<String, Value>) -> Url {
    let mut url = base_url.clone();
    {
        let segments = endpoint.split('/').filter(|segment| !segment.is_empty());
        url.path_segments_mut()
            .expect("Scrappa API base URL must support path segments")
            .pop_if_empty()
            .extend(segments);
    }
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            match value {
                Value::Null => {}
                Value::String(value) if value.is_empty() => {}
                Value::Bool(false) => {}
                Value::Bool(true) => {
                    query.append_pair(key, "1");
                }
                Value::Number(value) => {
                    query.append_pair(key, &value.to_string());
                }
                Value::String(value) => {
                    query.append_pair(key, value);
                }
                Value::Array(_) | Value::Object(_) => {
                    query.append_pair(key, &value.to_string());
                }
            }
        }
    }
    url
}

fn error_message(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    if body.is_empty() {
        return fallback;
    }
    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or(fallback);
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let messages = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .map(|value| match value {
                                    Value::String(value) => value.clone(),
                                    Value::Null => String::new(),
                                    value => value.to_string(),
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {messages}")
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details);
            }
        }
        return message;
    }

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_ERROR_BODY_LENGTH)
        .collect()
}

fn retry_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64
}

pub fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponent = u32::try_from(failed_attempt).unwrap_or(u32::MAX);
    1_000_u64
        .saturating_mul(2_u64.saturating_pow(exponent))
        .saturating_add(jitter_ms)
        .min(10_000)
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
                let deadline = Instant::now() + Duration::from_secs(15);
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
                    let reason = match response.status {
                        200 => "OK",
                        400 => "Bad Request",
                        404 => "Not Found",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
                        _ => "Mock Response",
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
                base_url: Url::parse(&format!("http://{address}/api")).unwrap(),
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

    #[test]
    fn retries_use_exponential_delay_with_ten_second_cap() {
        assert_eq!(retry_delay_ms(0, 0), 1_000);
        assert_eq!(retry_delay_ms(1, 0), 2_000);
        assert_eq!(retry_delay_ms(20, 0), 10_000);
        assert_eq!(retry_delay_ms(0, 900), 1_900);
    }

    #[tokio::test]
    async fn retries_vinted_catalog_404_and_preserves_request_headers_and_params() {
        let server = MockServer::start(vec![
            mock_response(
                404,
                r#"{"message":"The Vinted API returned an error. Please try again."}"#,
            ),
            mock_response(200, r#"{"items":[]}"#),
        ]);
        let client =
            ScrappaClient::new(server.base_url.clone(), "test-api-key".to_owned()).unwrap();
        let params = serde_json::from_value(json!({
            "query": "nike shoes",
            "country": "DE",
            "page": 2,
            "per_page": 50,
            "include": null,
            "enabled": true,
            "disabled": false
        }))
        .unwrap();

        assert_eq!(
            client.get("/vinted/search", &params, 2).await.unwrap(),
            json!({"items":[]})
        );

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("GET /api/vinted/search?"));
        assert!(requests[0].contains("country=DE"));
        assert!(requests[0].contains("query=nike+shoes"));
        assert!(requests[0].contains("enabled=1"));
        assert!(!requests[0].contains("disabled="));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("x-api-key: test-api-key"));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("user-agent: thescrappa-vinted-search-scraper/1.0"));
    }

    #[tokio::test]
    async fn does_not_retry_validation_errors_and_formats_json_error_details() {
        let server = MockServer::start(vec![mock_response(
            400,
            r#"{"message":"Invalid input","errors":{"country":["unsupported"]}}"#,
        )]);
        let client =
            ScrappaClient::new(server.base_url.clone(), "test-api-key".to_owned()).unwrap();
        let error = client
            .get("/vinted/search", &Map::new(), 3)
            .await
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            "Scrappa API error (400): Invalid input - country: unsupported"
        );
        assert_eq!(server.requests().len(), 1);
    }
}
