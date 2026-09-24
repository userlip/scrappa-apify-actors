use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::{Client, StatusCode, Url};
use serde_json::{Map, Value};
use tokio::time::sleep;

pub const SCRAPPA_REQUEST_TIMEOUT_MS: u64 = 90_000;
pub const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const SCRAPPA_USER_AGENT: &str = "thescrappa-booking-hotel-details-scraper/1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrappaError {
    Timeout,
    Network(String),
    Api { status: u16, message: String },
    InvalidJson(String),
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(
                formatter,
                "Scrappa API request timed out after {SCRAPPA_REQUEST_TIMEOUT_MS}ms"
            ),
            Self::Network(_) => write!(formatter, "Scrappa API network request failed"),
            Self::Api { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::InvalidJson(message) => write!(
                formatter,
                "Scrappa API response was not valid JSON: {message}"
            ),
        }
    }
}

impl std::error::Error for ScrappaError {}

impl ScrappaError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout | Self::Network(_) => true,
            Self::Api { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::InvalidJson(_) => false,
        }
    }

    pub fn is_actor_level_failure(&self) -> bool {
        matches!(
            self,
            Self::Api {
                status: 401 | 403,
                ..
            }
        )
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
    }
}

#[derive(Clone)]
pub struct ScrappaClient {
    http: Client,
    base_url: String,
    api_key: String,
    retry_delay_override: Option<Duration>,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self, String> {
        Self::build(
            api_key,
            base_url,
            Duration::from_millis(SCRAPPA_REQUEST_TIMEOUT_MS),
            None,
        )
    }

    fn build(
        api_key: String,
        base_url: String,
        timeout: Duration,
        retry_delay_override: Option<Duration>,
    ) -> Result<Self, String> {
        let http = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| format!("Could not create Scrappa HTTP client: {error}"))?;
        Ok(Self {
            http,
            base_url,
            api_key,
            retry_delay_override,
        })
    }

    pub async fn get(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
    ) -> Result<Value, ScrappaError> {
        let mut last_error = None;
        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send(endpoint, params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt == SCRAPPA_MAX_ATTEMPTS || !error.is_retryable() {
                        return Err(error);
                    }
                    let delay_ms = retry_delay_ms(attempt, random_jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {delay_ms}ms.",
                        attempt + 1
                    );
                    sleep(
                        self.retry_delay_override
                            .unwrap_or(Duration::from_millis(delay_ms)),
                    )
                    .await;
                    last_error = Some(error);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| ScrappaError::Network("unknown request failure".into())))
    }

    async fn send(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
    ) -> Result<Value, ScrappaError> {
        let raw_url = format!("{}{endpoint}", self.base_url);
        let mut url = Url::parse(&raw_url)
            .map_err(|error| ScrappaError::Network(format!("invalid request URL: {error}")))?;
        {
            let mut query = url.query_pairs_mut();
            for (name, value) in params {
                if value.is_null() || value.as_str() == Some("") {
                    continue;
                }
                if let Some(boolean) = value.as_bool() {
                    if boolean {
                        query.append_pair(name, "1");
                    }
                    continue;
                }
                query.append_pair(name, &js_string(value));
            }
        }

        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, SCRAPPA_USER_AGENT)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.map_err(map_request_error)?;
            return Err(ScrappaError::Api {
                status: status.as_u16(),
                message: format_api_error(status, &body),
            });
        }
        let body = response.text().await.map_err(map_request_error)?;
        serde_json::from_str(&body).map_err(|error| ScrappaError::InvalidJson(error.to_string()))
    }
}

pub fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponential = 1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt.min(32) as u32));
    exponential.saturating_add(jitter_ms).min(10_000)
}

fn random_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_micros() as u64 % 1000)
        .unwrap_or(0)
}

fn map_request_error(error: reqwest::Error) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout
    } else {
        ScrappaError::Network(error.to_string())
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    js_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn format_api_error(status: StatusCode, body: &str) -> String {
    let status_code = status.as_u16();
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {status_code}"));
    let message = match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(data)) => {
            let mut message = data
                .get("message")
                .filter(|value| !value.is_null())
                .map(js_string)
                .unwrap_or(fallback);
            if data.get("errors").is_some_and(js_truthy) {
                let details = data
                    .get("errors")
                    .and_then(Value::as_object)
                    .map(|errors| {
                        errors
                            .iter()
                            .map(|(field, messages)| {
                                let messages = messages
                                    .as_array()
                                    .map(|messages| {
                                        messages
                                            .iter()
                                            .map(js_string)
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    })
                                    .unwrap_or_default();
                                format!("{field}: {messages}")
                            })
                            .collect::<Vec<_>>()
                            .join("; ")
                    })
                    .unwrap_or_default();
                if !details.is_empty() {
                    message.push_str(" - ");
                    message.push_str(&details);
                }
            }
            message
        }
        Ok(Value::Null) | Err(_) => {
            if body.is_empty() {
                fallback
            } else {
                body.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(500)
                    .collect()
            }
        }
        Ok(_) => format!("HTTP {status_code}"),
    };
    message
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::mpsc,
        thread,
    };

    use super::*;
    use serde_json::json;

    struct MockServer {
        base_url: String,
        request: mpsc::Receiver<String>,
    }

    impl MockServer {
        fn start(responses: Vec<(u16, String)>) -> Self {
            Self::start_with_delays(
                responses
                    .into_iter()
                    .map(|(status, body)| (status, body, Duration::ZERO))
                    .collect(),
            )
        }

        fn start_with_delays(responses: Vec<(u16, String, Duration)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, request) = mpsc::channel();
            thread::spawn(move || {
                for (status, body, response_delay) in responses {
                    let (mut stream, _) = listener.accept().unwrap();
                    let sender = sender.clone();
                    thread::spawn(move || {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut request_data = Vec::new();
                        let mut buffer = [0; 2048];
                        loop {
                            let bytes = stream.read(&mut buffer).unwrap_or(0);
                            if bytes == 0 {
                                break;
                            }
                            request_data.extend_from_slice(&buffer[..bytes]);
                            if request_data.windows(4).any(|window| window == b"\r\n\r\n") {
                                break;
                            }
                        }
                        let _ = sender.send(String::from_utf8_lossy(&request_data).to_string());
                        thread::sleep(response_delay);
                        let reason = match status {
                            200 => "OK",
                            401 => "Unauthorized",
                            422 => "Unprocessable Entity",
                            503 => "Service Unavailable",
                            _ => "Mock Response",
                        };
                        let response = format!(
                            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                    });
                }
            });
            Self {
                base_url: format!("http://{address}/api"),
                request,
            }
        }
    }

    #[test]
    fn calculates_retry_backoff_and_classifies_error_types() {
        assert_eq!(retry_delay_ms(1, 0), 2000);
        assert_eq!(retry_delay_ms(2, 500), 4500);
        assert_eq!(retry_delay_ms(20, 0), 10_000);
        assert!(ScrappaError::Timeout.is_retryable());
        assert!(ScrappaError::Network("offline".into()).is_retryable());
        assert!(ScrappaError::Api {
            status: 429,
            message: "limited".into()
        }
        .is_retryable());
        assert!(!ScrappaError::Api {
            status: 422,
            message: "invalid".into()
        }
        .is_retryable());
        assert!(ScrappaError::Api {
            status: 401,
            message: "unauthorized".into()
        }
        .is_actor_level_failure());
    }

    #[tokio::test]
    async fn sends_booking_requests_with_scrappa_auth_user_agent_and_filtered_params() {
        let server = MockServer::start(vec![(200, "{\"ok\":true}".into())]);
        let client = ScrappaClient::build(
            "test-key".into(),
            server.base_url.clone(),
            Duration::from_secs(1),
            Some(Duration::ZERO),
        )
        .unwrap();
        let params = serde_json::from_value(json!({
            "country": "fr",
            "slug": "ritz paris",
            "use_cache": true,
            "debug": false,
            "empty": "",
            "missing": null
        }))
        .unwrap();

        assert_eq!(
            client.get("/booking/hotel", &params).await.unwrap(),
            json!({"ok": true})
        );
        let request = server.request.recv().unwrap();
        assert!(request.starts_with("GET /api/booking/hotel?"));
        assert!(request.contains("country=fr"));
        assert!(request.contains("slug=ritz+paris"));
        assert!(request.contains("use_cache=1"));
        assert!(!request.contains("debug="));
        let request = request.to_ascii_lowercase();
        assert!(request.contains("x-api-key: test-key"));
        assert!(request.contains(SCRAPPA_USER_AGENT));
    }

    #[tokio::test]
    async fn retries_transient_http_errors_and_does_not_retry_validation_errors() {
        let server = MockServer::start(vec![
            (503, "{\"message\":\"busy\"}".into()),
            (200, "{\"ok\":true}".into()),
        ]);
        let client = ScrappaClient::build(
            "key".into(),
            server.base_url,
            Duration::from_secs(1),
            Some(Duration::ZERO),
        )
        .unwrap();
        let params = Map::new();
        assert_eq!(
            client.get("/booking/hotel", &params).await.unwrap(),
            json!({"ok": true})
        );
        assert!(server
            .request
            .recv()
            .unwrap()
            .starts_with("GET /api/booking/hotel"));
        assert!(server
            .request
            .recv()
            .unwrap()
            .starts_with("GET /api/booking/hotel"));

        let server = MockServer::start(vec![(
            422,
            "{\"message\":\"Invalid request\",\"errors\":{\"slug\":[\"required\"]}}".into(),
        )]);
        let client = ScrappaClient::build(
            "key".into(),
            server.base_url,
            Duration::from_secs(1),
            Some(Duration::ZERO),
        )
        .unwrap();
        assert_eq!(
            client
                .get("/booking/hotel", &params)
                .await
                .unwrap_err()
                .to_string(),
            "Scrappa API error (422): Invalid request - slug: required"
        );
        assert!(server
            .request
            .recv()
            .unwrap()
            .starts_with("GET /api/booking/hotel"));
        assert!(server.request.try_recv().is_err());
    }

    #[tokio::test]
    async fn retries_timeouts_and_stops_after_three_attempts() {
        let server = MockServer::start_with_delays(vec![
            (200, "{\"ok\":true}".into(), Duration::from_millis(100)),
            (200, "{\"ok\":true}".into(), Duration::ZERO),
        ]);
        let client = ScrappaClient::build(
            "key".into(),
            server.base_url,
            Duration::from_millis(30),
            Some(Duration::ZERO),
        )
        .unwrap();
        assert_eq!(
            client.get("/booking/hotel", &Map::new()).await.unwrap(),
            json!({"ok": true})
        );
        assert!(server
            .request
            .recv()
            .unwrap()
            .starts_with("GET /api/booking/hotel"));
        assert!(server
            .request
            .recv()
            .unwrap()
            .starts_with("GET /api/booking/hotel"));

        let server = MockServer::start_with_delays(vec![
            (
                200,
                "{\"ok\":true}".into(),
                Duration::from_millis(100)
            );
            SCRAPPA_MAX_ATTEMPTS
        ]);
        let client = ScrappaClient::build(
            "key".into(),
            server.base_url,
            Duration::from_millis(30),
            Some(Duration::ZERO),
        )
        .unwrap();
        assert!(matches!(
            client.get("/booking/hotel", &Map::new()).await.unwrap_err(),
            ScrappaError::Timeout
        ));
        for _ in 0..SCRAPPA_MAX_ATTEMPTS {
            assert!(server
                .request
                .recv()
                .unwrap()
                .starts_with("GET /api/booking/hotel"));
        }
        assert!(server.request.try_recv().is_err());
    }
}
