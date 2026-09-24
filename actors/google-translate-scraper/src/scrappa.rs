use std::{fmt, time::Duration};

use rand::Rng;
use reqwest::{Client, StatusCode, Url, header};
use serde_json::{Map, Value};

use crate::{input::TranslationRequest, run_translations::TranslationRunner};

pub const REQUEST_TIMEOUT_MS: u64 = 30_000;
const MAX_ATTEMPTS: usize = 2;
const USER_AGENT: &str = "thescrappa-google-translate-scraper/1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrappaError {
    Timeout { timeout_ms: u64 },
    Network(String),
    Http { status: u16, details: String },
    InvalidJson(String),
    Configuration(String),
}

impl ScrappaError {
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Http { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub fn is_authentication_error(&self) -> bool {
        self.status_code()
            .is_some_and(|status| matches!(status, 401 | 403))
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout { .. } | Self::Network(_) => true,
            Self::Http { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::InvalidJson(_) | Self::Configuration(_) => false,
        }
    }
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout { timeout_ms } => {
                write!(
                    formatter,
                    "Scrappa API request timed out after {timeout_ms}ms"
                )
            }
            Self::Network(message) => write!(formatter, "Scrappa API network error: {message}"),
            Self::Http { status, details } => {
                write!(formatter, "Scrappa API error ({status}): {details}")
            }
            Self::InvalidJson(message) | Self::Configuration(message) => {
                formatter.write_str(message)
            }
        }
    }
}

pub struct ScrappaClient {
    http: Client,
    api_key: String,
    base_url: String,
    timeout_ms: u64,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self, ScrappaError> {
        Self::with_timeout(api_key, base_url, REQUEST_TIMEOUT_MS)
    }

    fn with_timeout(
        api_key: String,
        base_url: String,
        timeout_ms: u64,
    ) -> Result<Self, ScrappaError> {
        let http = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|error| ScrappaError::Configuration(error.to_string()))?;
        Ok(Self {
            http,
            api_key,
            base_url,
            timeout_ms,
        })
    }

    async fn get_json_with_retry<F>(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
        attempts: usize,
        retry_delay: F,
    ) -> Result<Value, ScrappaError>
    where
        F: Fn(u32) -> Duration,
    {
        let attempts = attempts.max(1);
        let mut last_error = None;

        for attempt in 1..=attempts {
            match self.send(endpoint, params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt >= attempts || !error.is_retryable() {
                        return Err(error);
                    }
                    let delay = retry_delay(attempt as u32);
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{attempts} in {}ms.",
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ScrappaError::Configuration("Scrappa request did not run".to_owned())
        }))
    }

    async fn send(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
    ) -> Result<Value, ScrappaError> {
        let url = Url::parse(&format!("{}{endpoint}", self.base_url))
            .map_err(|error| ScrappaError::Configuration(error.to_string()))?;
        let response = self
            .http
            .get(url)
            .query(params)
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(|error| self.map_request_error(error))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| self.map_request_error(error))?;

        if !status.is_success() {
            return Err(ScrappaError::Http {
                status: status.as_u16(),
                details: read_error_message(status, &body),
            });
        }

        serde_json::from_str(&body).map_err(|error| ScrappaError::InvalidJson(error.to_string()))
    }

    fn map_request_error(&self, error: reqwest::Error) -> ScrappaError {
        if error.is_timeout() {
            ScrappaError::Timeout {
                timeout_ms: self.timeout_ms,
            }
        } else {
            ScrappaError::Network(error.to_string())
        }
    }
}

impl TranslationRunner for ScrappaClient {
    async fn translate(&self, request: &TranslationRequest) -> Result<Value, ScrappaError> {
        let mut params = Map::new();
        params.insert("text".to_owned(), Value::String(request.text.clone()));
        params.insert("source".to_owned(), Value::String(request.source.clone()));
        params.insert("target".to_owned(), Value::String(request.target.clone()));
        self.get_json_with_retry("/google-translate", &params, MAX_ATTEMPTS, retry_delay)
            .await
    }
}

fn retry_delay(failed_attempt: u32) -> Duration {
    let jitter_ms = rand::thread_rng().gen_range(0..1_000_u64);
    retry_delay_with_jitter(failed_attempt, jitter_ms)
}

fn retry_delay_with_jitter(failed_attempt: u32, jitter_ms: u64) -> Duration {
    let exponential_ms = 1_000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt));
    Duration::from_millis(exponential_ms.saturating_add(jitter_ms).min(10_000))
}

fn read_error_message(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    if body.is_empty() {
        return fallback;
    }

    let Ok(Value::Object(error_data)) = serde_json::from_str::<Value>(body) else {
        return collapse_and_truncate(body, 500);
    };
    let mut message = error_data
        .get("message")
        .filter(|value| !value.is_null())
        .or_else(|| error_data.get("error").filter(|value| !value.is_null()))
        .map(js_string)
        .unwrap_or(fallback);

    if let Some(Value::Object(errors)) =
        error_data.get("errors").filter(|value| is_js_truthy(value))
    {
        let details = errors
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
            .join("; ");
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details);
        }
    }

    message
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
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
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn is_js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn collapse_and_truncate(value: &str, max_utf16_length: usize) -> String {
    let mut collapsed = String::new();
    let mut previous_was_whitespace = false;
    for character in value.chars() {
        if is_javascript_whitespace(character) {
            if !previous_was_whitespace {
                collapsed.push(' ');
            }
            previous_was_whitespace = true;
        } else {
            collapsed.push(character);
            previous_was_whitespace = false;
        }
    }
    let trimmed = collapsed.trim_matches(is_javascript_whitespace);
    String::from_utf16_lossy(
        &trimmed
            .encode_utf16()
            .take(max_utf16_length)
            .collect::<Vec<_>>(),
    )
}

fn is_javascript_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver},
        },
        thread::{self, JoinHandle},
    };

    use serde_json::json;

    use super::*;

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: String,
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
                for response in responses {
                    let (mut stream, _) = loop {
                        if stopped.load(Ordering::Relaxed) {
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
                        503 => "Service Unavailable",
                        _ => "Mock Response",
                    };
                    let reply = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    if stream.write_all(reply.as_bytes()).is_err() {
                        return;
                    }
                }
            });
            Self {
                base_url: format!("http://{address}/api"),
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
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut request = String::new();
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            let bytes_read = reader.read_line(&mut line)?;
            if bytes_read == 0 {
                break;
            }
            if line == "\r\n" {
                request.push_str(&line);
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.trim().parse().unwrap_or(0);
                }
            }
            request.push_str(&line);
        }
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body)?;
        request.push_str(&String::from_utf8_lossy(&body));
        Ok(request)
    }

    fn request() -> TranslationRequest {
        TranslationRequest {
            index: 0,
            text: "Good morning".to_owned(),
            source: "en".to_owned(),
            target: "de".to_owned(),
        }
    }

    #[tokio::test]
    async fn sends_the_google_translate_request_with_auth_and_actor_user_agent() {
        let server = MockServer::start(vec![MockResponse {
            status: 200,
            body: r#"{"translated_text":"Hallo"}"#.to_owned(),
        }]);
        let client = ScrappaClient::new("test-key".to_owned(), server.base_url.clone()).unwrap();
        let response = client.translate(&request()).await.unwrap();
        let requests = server.requests();

        assert_eq!(response, json!({"translated_text":"Hallo"}));
        assert_eq!(requests.len(), 1);
        assert!(
            requests[0]
                .starts_with("GET /api/google-translate?text=Good+morning&source=en&target=de ")
        );
        let lowercase_request = requests[0].to_ascii_lowercase();
        assert!(lowercase_request.contains("x-api-key: test-key\r\n"));
        assert!(
            lowercase_request.contains("user-agent: thescrappa-google-translate-scraper/1.0\r\n")
        );
        assert!(!requests[0].contains("append="));
        assert!(!requests[0].contains("html="));
    }

    #[tokio::test]
    async fn retries_a_transient_error_and_returns_the_successful_response() {
        let server = MockServer::start(vec![
            MockResponse {
                status: 503,
                body: r#"{"error":"Temporary upstream failure."}"#.to_owned(),
            },
            MockResponse {
                status: 200,
                body: r#"{"translated_text":"Hallo"}"#.to_owned(),
            },
        ]);
        let client = ScrappaClient::new("test-key".to_owned(), server.base_url.clone()).unwrap();
        let params =
            serde_json::from_value(json!({"text":"Hello","source":"en","target":"de"})).unwrap();
        let result = client
            .get_json_with_retry("/google-translate", &params, 2, |_| Duration::ZERO)
            .await
            .unwrap();

        assert_eq!(result, json!({"translated_text":"Hallo"}));
        assert_eq!(server.requests().len(), 2);
    }

    #[tokio::test]
    async fn reports_scrappa_http_errors_and_does_not_retry_bad_input() {
        let server = MockServer::start(vec![MockResponse {
            status: 400,
            body: r#"{"message":"Invalid target language."}"#.to_owned(),
        }]);
        let client = ScrappaClient::new("test-key".to_owned(), server.base_url.clone()).unwrap();
        let params =
            serde_json::from_value(json!({"text":"Hello","source":"en","target":"invalid"}))
                .unwrap();
        let error = client
            .get_json_with_retry("/google-translate", &params, 3, |_| {
                panic!("a non-retryable response must not wait")
            })
            .await
            .unwrap_err();

        assert_eq!(
            error,
            ScrappaError::Http {
                status: 400,
                details: "Invalid target language.".to_owned(),
            }
        );
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn returns_the_last_transient_http_error_after_two_attempts() {
        let server = MockServer::start(vec![
            MockResponse {
                status: 503,
                body: r#"{"error":"Temporary upstream failure 1."}"#.to_owned(),
            },
            MockResponse {
                status: 503,
                body: r#"{"error":"Temporary upstream failure 2."}"#.to_owned(),
            },
        ]);
        let client = ScrappaClient::new("test-key".to_owned(), server.base_url.clone()).unwrap();
        let params =
            serde_json::from_value(json!({"text":"Hello","source":"en","target":"de"})).unwrap();
        let error = client
            .get_json_with_retry("/google-translate", &params, 2, |_| Duration::ZERO)
            .await
            .unwrap_err();

        assert_eq!(
            error,
            ScrappaError::Http {
                status: 503,
                details: "Temporary upstream failure 2.".to_owned(),
            }
        );
        assert_eq!(server.requests().len(), 2);
    }

    #[test]
    fn formats_structured_and_plain_text_errors_and_backoff() {
        assert_eq!(
            read_error_message(
                StatusCode::BAD_REQUEST,
                r#"{"error":"Bad request","errors":{"source":["Required","Invalid"]}}"#
            ),
            "Bad request - source: Required, Invalid"
        );
        assert_eq!(
            read_error_message(StatusCode::BAD_GATEWAY, "  first\n second "),
            "first second"
        );
        assert_eq!(
            read_error_message(StatusCode::BAD_GATEWAY, &"x".repeat(600)).len(),
            500
        );
        assert_eq!(
            retry_delay_with_jitter(1, 250),
            Duration::from_millis(2_250)
        );
        assert_eq!(
            retry_delay_with_jitter(4, 250),
            Duration::from_millis(10_000)
        );
        assert!(retry_delay(1) >= Duration::from_millis(2_000));
        assert!(retry_delay(1) < Duration::from_millis(3_000));
        assert!(ScrappaError::Timeout { timeout_ms: 30_000 }.is_retryable());
        assert!(ScrappaError::Network("fetch failed".to_owned()).is_retryable());
        assert!(
            ScrappaError::Http {
                status: 429,
                details: String::new()
            }
            .is_retryable()
        );
        assert!(
            !ScrappaError::Http {
                status: 400,
                details: String::new()
            }
            .is_retryable()
        );
    }
}
