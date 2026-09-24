use reqwest::{header, Client};
use serde_json::Value;
use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_ATTEMPTS: u8 = 3;
const USER_AGENT: &str = "thescrappa-google-hotels-autocomplete-scraper/1.0";
const RETRYABLE_STATUS_CODES: [u16; 6] = [408, 429, 500, 502, 503, 504];

#[derive(Debug, PartialEq, Eq)]
pub enum ScrappaError {
    Timeout,
    Connection,
    Http {
        status: u16,
        message: String,
        retryable: bool,
    },
    InvalidUrl(String),
    InvalidJson,
}

impl ScrappaError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout | Self::Connection => true,
            Self::Http {
                status, retryable, ..
            } => RETRYABLE_STATUS_CODES.contains(status) || (*status == 403 && *retryable),
            Self::InvalidUrl(_) | Self::InvalidJson => false,
        }
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
    }
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(
                formatter,
                "Scrappa API request timed out after {}ms",
                REQUEST_TIMEOUT.as_millis()
            ),
            Self::Connection => write!(formatter, "Scrappa API connection failed"),
            Self::Http {
                status, message, ..
            } => write!(formatter, "Scrappa API error ({status}): {message}"),
            Self::InvalidUrl(message) => write!(formatter, "Invalid Scrappa API URL: {message}"),
            Self::InvalidJson => write!(formatter, "Scrappa API response was not valid JSON"),
        }
    }
}

impl Error for ScrappaError {}

#[derive(Clone)]
pub struct ScrappaClient {
    client: Client,
    api_key: String,
    base_url: Url,
}

impl ScrappaClient {
    pub fn new(client: Client, api_key: String, base_url: Url) -> Self {
        Self {
            client,
            api_key,
            base_url,
        }
    }

    pub async fn get(
        &self,
        endpoint: &str,
        params: &[(String, String)],
        attempts: u8,
    ) -> Result<Value, ScrappaError> {
        self.get_with_timeout(endpoint, params, attempts, REQUEST_TIMEOUT)
            .await
    }

    async fn get_with_timeout(
        &self,
        endpoint: &str,
        params: &[(String, String)],
        attempts: u8,
        timeout: Duration,
    ) -> Result<Value, ScrappaError> {
        let url = self.request_url(endpoint, params)?;
        let attempts = attempts.max(1);
        for attempt in 1..=attempts {
            match self.send(&url, timeout).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < attempts && error.is_retryable() => {
                    let delay =
                        Duration::from_millis(get_retry_delay_ms(attempt, random_jitter_ms()));
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{attempts} in {}ms.",
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("the retry loop always returns a response or an error")
    }

    fn request_url(
        &self,
        endpoint: &str,
        params: &[(String, String)],
    ) -> Result<Url, ScrappaError> {
        let mut url = self.base_url.clone();
        let segments = endpoint.trim_matches('/').split('/');
        url.path_segments_mut()
            .map_err(|_| {
                ScrappaError::InvalidUrl("base URL cannot contain path segments".to_owned())
            })?
            .pop_if_empty()
            .extend(segments);
        url.query_pairs_mut()
            .extend_pairs(params.iter().map(|(key, value)| (key, value)));
        Ok(url)
    }

    async fn send(&self, url: &Url, timeout: Duration) -> Result<Value, ScrappaError> {
        let response = self
            .client
            .get(url.clone())
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .timeout(timeout)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ScrappaError::Timeout
                } else {
                    ScrappaError::Connection
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let fallback = status
                .canonical_reason()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            let body = match response.text().await {
                Ok(body) => body,
                Err(error) if error.is_timeout() => return Err(ScrappaError::Timeout),
                Err(_) => String::new(),
            };
            let (message, retryable) = parse_http_error(&body, &fallback);
            return Err(ScrappaError::Http {
                status: status.as_u16(),
                message,
                retryable,
            });
        }

        response.json().await.map_err(|_| ScrappaError::InvalidJson)
    }
}

pub fn get_retry_delay_ms(failed_attempt: u8, jitter_ms: u64) -> u64 {
    (1_000_u64.saturating_mul(2_u64.saturating_pow(u32::from(failed_attempt))))
        .saturating_add(jitter_ms)
        .min(10_000)
}

fn random_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
        % 1_000
}

fn parse_http_error(body: &str, fallback: &str) -> (String, bool) {
    if body.is_empty() {
        return (fallback.to_owned(), false);
    }
    if let Ok(error) = serde_json::from_str::<Value>(body) {
        let mut message = error
            .get("message")
            .and_then(Value::as_str)
            .or_else(|| error.get("error").and_then(Value::as_str))
            .unwrap_or(fallback)
            .to_owned();
        if let Some(errors) = error.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    let messages = messages.as_array()?;
                    let messages = messages
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ");
                    (!messages.is_empty()).then(|| format!("{field}: {messages}"))
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details);
            }
        }
        let retryable = error.get("retryable").and_then(Value::as_bool) == Some(true);
        return (message, retryable);
    }

    let normalized = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let message = normalized.chars().take(500).collect();
    (message, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{mpsc, Arc, Mutex},
        thread::{self, JoinHandle},
        time::Instant,
    };

    struct MockResponse {
        status: u16,
        body: String,
        delay: Duration,
    }

    struct MockServer {
        base_url: Url,
        requests: Arc<Mutex<mpsc::Receiver<String>>>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (request_sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                for response in responses {
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
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let request = read_request(&mut stream).unwrap_or_default();
                    let _ = request_sender.send(request);
                    thread::sleep(response.delay);
                    let reason = match response.status {
                        200 => "OK",
                        403 => "Forbidden",
                        500 => "Internal Server Error",
                        503 => "Service Unavailable",
                        _ => "Mock Response",
                    };
                    let body = response.body;
                    let message = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(message.as_bytes());
                }
            });
            Self {
                base_url: Url::parse(&format!("http://{address}/api/")).unwrap(),
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
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        Ok(String::from_utf8_lossy(&request).into_owned())
    }

    fn mock_response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
            delay: Duration::ZERO,
        }
    }

    #[test]
    fn calculates_capped_exponential_delays() {
        assert_eq!(get_retry_delay_ms(1, 0), 2_000);
        assert_eq!(get_retry_delay_ms(3, 500), 8_500);
        assert_eq!(get_retry_delay_ms(10, 999), 10_000);
    }

    #[test]
    fn retries_only_transient_http_errors_or_body_flagged_403() {
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(ScrappaError::Http {
                status,
                message: "error".to_owned(),
                retryable: false,
            }
            .is_retryable());
        }
        assert!(ScrappaError::Http {
            status: 403,
            message: "blocked".to_owned(),
            retryable: true,
        }
        .is_retryable());
        assert!(!ScrappaError::Http {
            status: 403,
            message: "out of credits".to_owned(),
            retryable: false,
        }
        .is_retryable());
        assert!(!ScrappaError::Http {
            status: 422,
            message: "invalid".to_owned(),
            retryable: true,
        }
        .is_retryable());
        assert!(!ScrappaError::InvalidJson.is_retryable());
    }

    #[tokio::test]
    async fn sends_scrappa_auth_context_and_encoded_query_params() {
        let server = MockServer::start(vec![mock_response(200, r#"{"suggestions":[]}"#)]);
        let client = Client::new();
        let scrappa = ScrappaClient::new(client, "test-key".to_owned(), server.base_url.clone());
        let response = scrappa
            .get(
                "/google-hotels/autocomplete",
                &[
                    ("q".to_owned(), "Paris hotels & cafes".to_owned()),
                    ("type".to_owned(), "all".to_owned()),
                ],
                1,
            )
            .await
            .unwrap();
        let request = server.next_request();

        assert_eq!(response, serde_json::json!({"suggestions": []}));
        assert!(request.contains(
            "GET /api/google-hotels/autocomplete?q=Paris+hotels+%26+cafes&type=all HTTP/1.1"
        ));
        assert!(request.to_ascii_lowercase().contains("x-api-key: test-key"));
        assert!(request
            .to_ascii_lowercase()
            .contains("accept: application/json"));
        assert!(request
            .to_ascii_lowercase()
            .contains("user-agent: thescrappa-google-hotels-autocomplete-scraper/1.0"));
    }

    #[tokio::test]
    async fn retries_transient_http_errors_and_returns_the_successful_response() {
        let server = MockServer::start(vec![
            mock_response(503, "Unavailable"),
            mock_response(200, r#"{"suggestions":[]}"#),
        ]);
        let scrappa = ScrappaClient::new(
            Client::new(),
            "test-key".to_owned(),
            server.base_url.clone(),
        );

        let response = scrappa
            .get("/google-hotels/autocomplete", &[], 2)
            .await
            .unwrap();

        assert_eq!(response, serde_json::json!({"suggestions": []}));
        assert!(server
            .next_request()
            .contains("/api/google-hotels/autocomplete"));
        assert!(server
            .next_request()
            .contains("/api/google-hotels/autocomplete"));
    }

    #[tokio::test]
    async fn retries_only_body_flagged_403_responses() {
        let server = MockServer::start(vec![
            mock_response(403, r#"{"error":"Upstream blocked","retryable":true}"#),
            mock_response(200, r#"{"suggestions":[]}"#),
        ]);
        let scrappa = ScrappaClient::new(
            Client::new(),
            "test-key".to_owned(),
            server.base_url.clone(),
        );

        let response = scrappa
            .get("/google-hotels/autocomplete", &[], 2)
            .await
            .unwrap();

        assert_eq!(response, serde_json::json!({"suggestions": []}));
        assert!(!server.next_request().is_empty());
        assert!(!server.next_request().is_empty());
    }

    #[tokio::test]
    async fn preserves_structured_errors_and_does_not_retry_unflagged_403() {
        let server = MockServer::start(vec![mock_response(
            403,
            r#"{"message":"Invalid request","errors":{"q":["The query is required"]}}"#,
        )]);
        let scrappa = ScrappaClient::new(
            Client::new(),
            "test-key".to_owned(),
            server.base_url.clone(),
        );

        let error = scrappa
            .get("/google-hotels/autocomplete", &[], 3)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (403): Invalid request - q: The query is required"
        );
        assert!(!error.is_retryable());
        assert!(!server.next_request().is_empty());
    }

    #[tokio::test]
    async fn request_deadline_becomes_a_timeout_error() {
        let mut response = mock_response(200, r#"{"suggestions":[]}"#);
        response.delay = Duration::from_millis(100);
        let server = MockServer::start(vec![response]);
        let scrappa = ScrappaClient::new(
            Client::new(),
            "test-key".to_owned(),
            server.base_url.clone(),
        );

        let error = scrappa
            .get_with_timeout(
                "/google-hotels/autocomplete",
                &[],
                1,
                Duration::from_millis(10),
            )
            .await
            .unwrap_err();
        assert_eq!(error, ScrappaError::Timeout);
        assert!(error.is_timeout());
    }
}
