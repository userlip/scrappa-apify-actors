use anyhow::{Context, Result};
use reqwest::{Client, Response, Url};
use serde_json::Value;
use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const REQUEST_TIMEOUT_MS: u64 = 90_000;
pub const MAX_ATTEMPTS: usize = 3;

#[derive(Debug)]
pub enum ScrappaError {
    Timeout { timeout_ms: u64 },
    Request(String),
    Api { status: u16, message: String },
    InvalidJson(String),
}

impl ScrappaError {
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout { .. } => true,
            Self::Api { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::Request(_) | Self::InvalidJson(_) => false,
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
            Self::Request(message) | Self::InvalidJson(message) => write!(formatter, "{message}"),
            Self::Api { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
        }
    }
}

impl Error for ScrappaError {}

pub struct ScrappaClient {
    client: Client,
    base_url: String,
    api_key: String,
    timeout_ms: u64,
    max_attempts: usize,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self> {
        Self::with_policy(api_key, base_url, REQUEST_TIMEOUT_MS, MAX_ATTEMPTS)
    }

    fn with_policy(
        api_key: String,
        base_url: String,
        timeout_ms: u64,
        max_attempts: usize,
    ) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_millis(timeout_ms))
                .build()
                .context("Could not create Scrappa HTTP client")?,
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key,
            timeout_ms,
            max_attempts: max_attempts.max(1),
        })
    }

    pub async fn get(
        &self,
        endpoint: &str,
        params: &[(&str, String)],
    ) -> Result<Value, ScrappaError> {
        let mut url = Url::parse(&format!("{}{endpoint}", self.base_url))
            .map_err(|error| ScrappaError::Request(error.to_string()))?;
        for (key, value) in params {
            if !value.is_empty() {
                url.query_pairs_mut().append_pair(key, value);
            }
        }

        for attempt in 1..=self.max_attempts {
            match self.send(&url).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < self.max_attempts && error.is_retryable() => {
                    let delay_ms = retry_delay_ms(attempt, random_jitter_ms());
                    println!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        self.max_attempts,
                        delay_ms
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("at least one Scrappa request attempt is always made")
    }

    async fn send(&self, url: &Url) -> Result<Value, ScrappaError> {
        let response = self
            .client
            .get(url.clone())
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                reqwest::header::USER_AGENT,
                "thescrappa-immowelt-property-search-scraper/1.0",
            )
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ScrappaError::Timeout {
                        timeout_ms: self.timeout_ms,
                    }
                } else {
                    ScrappaError::Request(error.to_string())
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let message = self.read_error_message(response).await?;
            return Err(ScrappaError::Api {
                status: status.as_u16(),
                message,
            });
        }

        response.json().await.map_err(|error| {
            if error.is_timeout() {
                ScrappaError::Timeout {
                    timeout_ms: self.timeout_ms,
                }
            } else {
                ScrappaError::InvalidJson(format!(
                    "Scrappa API response was not valid JSON: {error}"
                ))
            }
        })
    }

    async fn read_error_message(&self, response: Response) -> Result<String, ScrappaError> {
        let fallback = response
            .status()
            .canonical_reason()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("HTTP {}", response.status().as_u16()));
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) if error.is_timeout() => {
                return Err(ScrappaError::Timeout {
                    timeout_ms: self.timeout_ms,
                });
            }
            Err(_) => return Ok(fallback.clone()),
        };
        if body.is_empty() {
            return Ok(fallback);
        }
        Ok(format_error_body(&body, &fallback))
    }
}

pub fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    (1_000_u64
        .saturating_mul(2_u64.saturating_pow(failed_attempt as u32))
        .saturating_add(jitter_ms))
    .min(10_000)
}

fn random_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
        % 1_000
}

fn format_error_body(body: &str, fallback: &str) -> String {
    if let Ok(data) = serde_json::from_str::<Value>(body) {
        let mut message = data
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or_else(|| fallback.to_owned());
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
        return message;
    }
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::Null => String::new(),
                _ => js_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc,
        thread,
    };

    #[test]
    fn retries_timeouts_and_only_selected_http_errors() {
        assert!(ScrappaError::Timeout { timeout_ms: 1 }.is_retryable());
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(ScrappaError::Api {
                status,
                message: "temporary".into()
            }
            .is_retryable());
        }
        for status in [400, 401, 403, 404, 501] {
            assert!(!ScrappaError::Api {
                status,
                message: "permanent".into()
            }
            .is_retryable());
        }
        assert!(!ScrappaError::Request("network error".into()).is_retryable());
        assert!(!ScrappaError::InvalidJson("bad json".into()).is_retryable());
    }

    #[test]
    fn retry_backoff_matches_the_node_actor_policy() {
        assert_eq!(retry_delay_ms(1, 0), 2_000);
        assert_eq!(retry_delay_ms(2, 999), 4_999);
        assert_eq!(retry_delay_ms(5, 999), 10_000);
    }

    #[test]
    fn formats_api_error_json_and_bounded_plain_text() {
        let json = serde_json::json!({
            "message": "Invalid request",
            "errors": {"location": ["is invalid", "is too long"]}
        });
        assert_eq!(
            format_error_body(&json.to_string(), "Bad Request"),
            "Invalid request - location: is invalid, is too long"
        );
        assert_eq!(
            format_error_body(&" x  ".repeat(300), "Bad Request")
                .chars()
                .count(),
            500
        );
    }

    #[tokio::test]
    async fn retries_429_and_preserves_auth_headers_and_query_params() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, requests) = mpsc::channel();
        let server = thread::spawn(move || {
            for (status, body) in [(429, "{\"message\":\"slow down\"}"), (200, "{\"ok\":true}")] {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_request(&mut stream);
                sender.send(request).unwrap();
                let reason = if status == 429 {
                    "Too Many Requests"
                } else {
                    "OK"
                };
                let reply = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(reply.as_bytes()).unwrap();
            }
        });
        let client = ScrappaClient::with_policy(
            "test-key".into(),
            format!("http://{address}/api"),
            5_000,
            2,
        )
        .unwrap();
        let response = client
            .get(
                "/immowelt/search",
                &[
                    ("location", "Berlin Mitte".into()),
                    ("type", "apartment-rent".into()),
                    ("page", "2".into()),
                    ("per_page", "20".into()),
                ],
            )
            .await
            .unwrap();
        assert_eq!(response, serde_json::json!({"ok": true}));
        server.join().unwrap();
        let requests = requests.into_iter().collect::<Vec<_>>();
        assert_eq!(requests.len(), 2);
        for request in requests {
            assert!(request.to_ascii_lowercase().contains("x-api-key: test-key"));
            assert!(request
                .to_ascii_lowercase()
                .contains("accept: application/json"));
            assert!(request
                .to_ascii_lowercase()
                .contains("user-agent: thescrappa-immowelt-property-search-scraper/1.0"));
            let path = request.lines().next().unwrap();
            assert!(path.starts_with("GET /api/immowelt/search?"));
            assert!(path.contains("location=Berlin+Mitte"));
            assert!(path.contains("page=2"));
        }
    }

    #[tokio::test]
    async fn applies_the_configured_request_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = read_request(&mut stream);
            thread::sleep(Duration::from_millis(100));
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
        });
        let client =
            ScrappaClient::with_policy("test-key".into(), format!("http://{address}/api"), 20, 1)
                .unwrap();
        let error = client.get("/immowelt/search", &[]).await.unwrap_err();
        assert!(matches!(error, ScrappaError::Timeout { timeout_ms: 20 }));
        server.join().unwrap();
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut bytes = Vec::new();
        let mut buffer = [0; 1024];
        loop {
            let count = stream.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..count]);
            if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }
}
