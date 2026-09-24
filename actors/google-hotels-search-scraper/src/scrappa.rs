use anyhow::{Context, Result};
use rand::random_range;
use reqwest::{header, Client, Response};
use serde_json::{Map, Value};
use std::{fmt, time::Duration};
use url::Url;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
pub const REQUEST_ATTEMPTS: usize = 3;
const USER_AGENT: &str = "thescrappa-google-hotels-search-scraper/1.0";

#[derive(Debug)]
pub enum ScrappaError {
    Timeout,
    Api { status: u16, message: String },
    Transport(String),
    InvalidResponse(String),
}

impl ScrappaError {
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout => true,
            Self::Api { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::Transport(_) | Self::InvalidResponse(_) => false,
        }
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
            Self::Api { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::Transport(message) | Self::InvalidResponse(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for ScrappaError {}

pub struct ScrappaClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: &str) -> Result<Self> {
        Url::parse(base_url).context("SCRAPPA_API_BASE_URL must be a valid absolute URL")?;
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("Could not create Scrappa HTTP client")?;
        Ok(Self {
            client,
            api_key,
            base_url: base_url.trim_end_matches('/').to_owned(),
        })
    }

    pub async fn get(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
        attempts: usize,
    ) -> std::result::Result<Value, ScrappaError> {
        self.get_with_delay(endpoint, params, attempts, |failed_attempt| {
            retry_delay(failed_attempt, random_jitter_ms())
        })
        .await
    }

    async fn get_with_delay<F>(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
        attempts: usize,
        delay: F,
    ) -> std::result::Result<Value, ScrappaError>
    where
        F: Fn(usize) -> Duration,
    {
        let attempts = attempts.max(1);
        let url = build_request_url(&self.base_url, endpoint, params)?;
        let mut last_error = None;

        for attempt in 1..=attempts {
            match self.send(&url).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let retry = attempt < attempts && error.is_retryable();
                    if !retry {
                        return Err(error);
                    }
                    let delay = delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{attempts} in {}ms.",
                        attempt + 1,
                        delay.as_millis()
                    );
                    last_error = Some(error);
                    tokio::time::sleep(delay).await;
                }
            }
        }
        Err(last_error.expect("at least one Scrappa request attempt always runs"))
    }

    async fn send(&self, url: &Url) -> std::result::Result<Value, ScrappaError> {
        let response = self
            .client
            .get(url.clone())
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(map_request_error)?;

        if !response.status().is_success() {
            return Err(api_error(response).await?);
        }
        response.json().await.map_err(|error| {
            if error.is_timeout() {
                ScrappaError::Timeout
            } else {
                ScrappaError::InvalidResponse(format!("Scrappa API returned invalid JSON: {error}"))
            }
        })
    }
}

fn build_request_url(
    base_url: &str,
    endpoint: &str,
    params: &Map<String, Value>,
) -> std::result::Result<Url, ScrappaError> {
    let url = format!("{}{endpoint}", base_url);
    let mut url = Url::parse(&url)
        .map_err(|error| ScrappaError::Transport(format!("Invalid Scrappa URL: {error}")))?;
    for (name, value) in params {
        match value {
            Value::Null | Value::Bool(false) => {}
            Value::String(value) if value.is_empty() => {}
            Value::Bool(true) => {
                url.query_pairs_mut().append_pair(name, "1");
            }
            Value::String(value) => {
                url.query_pairs_mut().append_pair(name, value);
            }
            Value::Number(value) => {
                url.query_pairs_mut().append_pair(name, &value.to_string());
            }
            Value::Array(_) | Value::Object(_) => {}
        }
    }
    Ok(url)
}

fn map_request_error(error: reqwest::Error) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout
    } else {
        ScrappaError::Transport(error.to_string())
    }
}

async fn api_error(response: Response) -> std::result::Result<ScrappaError, ScrappaError> {
    let status = response.status();
    let fallback = status
        .canonical_reason()
        .unwrap_or_else(|| "HTTP status")
        .to_owned();
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) if error.is_timeout() => return Err(ScrappaError::Timeout),
        Err(_) => {
            return Ok(ScrappaError::Api {
                status: status.as_u16(),
                message: fallback,
            });
        }
    };
    Ok(ScrappaError::Api {
        status: status.as_u16(),
        message: parse_api_error_message(&body, &fallback),
    })
}

fn parse_api_error_message(body: &str, fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return compact_message(body);
    };
    let Some(object) = value.as_object() else {
        return compact_message(body);
    };
    let mut message = object
        .get("message")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or_else(|| fallback.to_owned());
    if let Some(errors) = object.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .filter_map(|(field, messages)| {
                messages.as_array().map(|messages| {
                    format!(
                        "{field}: {}",
                        messages
                            .iter()
                            .map(js_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
            })
            .collect::<Vec<_>>();
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details.join("; "));
        }
    }
    message
}

fn compact_message(body: &str) -> String {
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn random_jitter_ms() -> u64 {
    random_range(0..1000)
}

fn retry_delay(failed_attempt: usize, jitter_ms: u64) -> Duration {
    let backoff_multiplier = 2_u64.checked_pow(failed_attempt as u32).unwrap_or(u64::MAX);
    let base_ms = 1000_u64.saturating_mul(backoff_multiplier);
    Duration::from_millis(base_ms.saturating_add(jitter_ms).min(10_000))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
        thread::{self, JoinHandle},
    };

    struct MockServer {
        base_url: String,
        requests: Receiver<String>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<(u16, &'static str)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (request_sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                for (status, body) in responses {
                    let Ok((mut stream, _)) = listener.accept() else {
                        return;
                    };
                    let request = read_request(&mut stream);
                    let _ = request_sender.send(request);
                    let reason = match status {
                        200 => "OK",
                        400 => "Bad Request",
                        503 => "Service Unavailable",
                        _ => "Mock Response",
                    };
                    let reply = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(reply.as_bytes());
                }
            });
            Self {
                base_url: format!("http://{address}/api"),
                requests,
                thread: Some(thread),
            }
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut request = [0; 4096];
        let count = stream.read(&mut request).unwrap_or(0);
        String::from_utf8_lossy(&request[..count]).to_string()
    }

    #[tokio::test]
    async fn retries_retryable_status_and_preserves_auth_query_and_user_agent() {
        let server = MockServer::start(vec![
            (503, r#"{"message":"temporary"}"#),
            (200, r#"{"properties":[]}"#),
        ]);
        let client = ScrappaClient::new("secret-key".to_owned(), &server.base_url).unwrap();
        let params = serde_json::from_value(serde_json::json!({
            "q": "Paris, France",
            "free_cancellation": true,
            "currency": "EUR",
            "adults": 2
        }))
        .unwrap();
        let response = client
            .get_with_delay("/google-hotels/search", &params, 3, |_| Duration::ZERO)
            .await
            .unwrap();
        assert_eq!(response["properties"], serde_json::json!([]));

        let first = server.requests.recv().unwrap();
        let second = server.requests.recv().unwrap();
        assert!(first.starts_with("GET /api/google-hotels/search?"));
        assert!(first.contains("q=Paris%2C+France"));
        assert!(first.contains("free_cancellation=1"));
        assert!(first.contains("currency=EUR"));
        assert!(first.contains("adults=2"));
        assert!(first.to_ascii_lowercase().contains("x-api-key: secret-key"));
        assert!(first.contains(USER_AGENT));
        assert!(second.starts_with("GET /api/google-hotels/search?"));
    }

    #[tokio::test]
    async fn does_not_retry_non_retryable_status_codes() {
        let server = MockServer::start(vec![(400, r#"{"message":"Invalid input"}"#)]);
        let client = ScrappaClient::new("secret-key".to_owned(), &server.base_url).unwrap();
        let error = client
            .get_with_delay("/google-hotels/search", &Map::new(), 3, |_| Duration::ZERO)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ScrappaError::Api {
                status: 400,
                message
            } if message == "Invalid input"
        ));
        assert!(server.requests.recv().unwrap().starts_with("GET "));
    }

    #[test]
    fn preserves_retry_statuses_timeout_and_backoff_bounds() {
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(90));
        assert_eq!(REQUEST_ATTEMPTS, 3);
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(ScrappaError::Api {
                status,
                message: String::new()
            }
            .is_retryable());
        }
        assert!(!ScrappaError::Api {
            status: 401,
            message: String::new()
        }
        .is_retryable());
        assert!(ScrappaError::Timeout.is_retryable());
        assert_eq!(retry_delay(1, 0), Duration::from_secs(2));
        assert_eq!(retry_delay(2, 250), Duration::from_millis(4250));
        assert_eq!(retry_delay(5, 999), Duration::from_secs(10));
    }

    #[test]
    fn formats_upstream_validation_errors_and_plain_text() {
        assert_eq!(
            parse_api_error_message(
                r#"{"message":"Invalid","errors":{"q":["is required","too short"]}}"#,
                "Bad Request"
            ),
            "Invalid - q: is required, too short"
        );
        assert_eq!(
            parse_api_error_message("line\nwith   spaces", "Bad Request"),
            "line with spaces"
        );
        assert_eq!(parse_api_error_message("", "Bad Request"), "Bad Request");
    }
}
