use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::Url;

pub const SCRAPPA_REQUEST_TIMEOUT_MS: u64 = 60_000;
pub const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const ACTOR_USER_AGENT: &str = "thescrappa-redfin-valuation-scraper/1.0";

#[derive(Debug)]
pub enum ScrappaError {
    Timeout { timeout_ms: u64 },
    Network,
    Http { status: u16, details: String },
    InvalidResponse(String),
    Request(String),
}

impl ScrappaError {
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Http { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub fn details(&self) -> Option<&str> {
        match self {
            Self::Http { details, .. } => Some(details),
            _ => None,
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout { .. } | Self::Network => true,
            Self::Http { status, .. } => matches!(status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::InvalidResponse(_) | Self::Request(_) => false,
        }
    }
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout { timeout_ms } => write!(
                formatter,
                "Scrappa API request timed out after {timeout_ms}ms"
            ),
            Self::Network => write!(formatter, "Scrappa API network request failed"),
            Self::Http { status, details } => {
                write!(formatter, "Scrappa API error ({status}): {details}")
            }
            Self::InvalidResponse(message) => {
                write!(formatter, "Scrappa API returned invalid JSON: {message}")
            }
            Self::Request(message) => write!(formatter, "Scrappa API request failed: {message}"),
        }
    }
}

impl std::error::Error for ScrappaError {}

#[derive(Clone)]
pub struct ScrappaClient {
    client: Client,
    api_key: String,
    base_url: String,
    timeout_ms: u64,
}

impl ScrappaClient {
    pub fn new(
        api_key: String,
        base_url: Option<String>,
        timeout_ms: u64,
    ) -> Result<Self, ScrappaError> {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|error| ScrappaError::Request(error.to_string()))?;
        Ok(Self {
            client,
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://scrappa.co/api".to_owned()),
            timeout_ms,
        })
    }

    pub async fn get(
        &self,
        endpoint: &str,
        property_id: &str,
        listing_id: Option<&str>,
    ) -> Result<Value, ScrappaError> {
        let mut last_error = None;
        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send(endpoint, property_id, listing_id).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let retryable = error.is_retryable();
                    last_error = Some(error);
                    if attempt == SCRAPPA_MAX_ATTEMPTS || !retryable {
                        break;
                    }
                    let error = last_error.as_ref().expect("set before use");
                    let delay_ms = get_retry_delay_ms(attempt, random_jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        SCRAPPA_MAX_ATTEMPTS,
                        delay_ms,
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
        Err(last_error
            .unwrap_or_else(|| ScrappaError::Request("request failed without an error".to_owned())))
    }

    async fn send(
        &self,
        endpoint: &str,
        property_id: &str,
        listing_id: Option<&str>,
    ) -> Result<Value, ScrappaError> {
        let full_url = format!("{}{endpoint}", self.base_url);
        let mut url =
            Url::parse(&full_url).map_err(|error| ScrappaError::Request(error.to_string()))?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("property_id", property_id);
            if let Some(listing_id) = listing_id {
                query.append_pair("listing_id", listing_id);
            }
        }

        let response = self
            .client
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header("Accept", "application/json")
            .header("User-Agent", ACTOR_USER_AGENT)
            .send()
            .await
            .map_err(|error| map_request_error(error, self.timeout_ms))?;

        let status = response.status();
        let fallback = status.canonical_reason().unwrap_or("HTTP response");
        let body = response
            .bytes()
            .await
            .map_err(|error| map_request_error(error, self.timeout_ms))?;
        if !status.is_success() {
            return Err(ScrappaError::Http {
                status: status.as_u16(),
                details: read_error_message(status, fallback, &body),
            });
        }
        serde_json::from_slice(&body)
            .map_err(|error| ScrappaError::InvalidResponse(error.to_string()))
    }
}

fn map_request_error(error: reqwest::Error, timeout_ms: u64) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout { timeout_ms }
    } else if error.is_connect() || error.is_request() || error.is_body() {
        ScrappaError::Network
    } else {
        ScrappaError::Request(error.to_string())
    }
}

pub fn get_retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponential = 1_000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt.min(63) as u32));
    exponential.saturating_add(jitter_ms).min(10_000)
}

fn random_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::from(elapsed.subsec_millis()))
        .unwrap_or_default()
        % 1_000
}

fn read_error_message(status: StatusCode, fallback: &str, body: &[u8]) -> String {
    if body.is_empty() {
        return if fallback.is_empty() {
            format!("HTTP {status}")
        } else {
            fallback.to_owned()
        };
    }
    if let Ok(error_data) = serde_json::from_slice::<Value>(body) {
        let mut message = error_data
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or_else(|| {
                if fallback.is_empty() {
                    format!("HTTP {status}")
                } else {
                    fallback.to_owned()
                }
            });
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let joined = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .map(js_join_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {joined}")
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
    let normalized = String::from_utf8_lossy(body)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    truncate_utf16(&normalized, 500)
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values
            .iter()
            .map(js_join_string)
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn js_join_string(value: &Value) -> String {
    if value.is_null() {
        String::new()
    } else {
        js_string(value)
    }
}

fn truncate_utf16(value: &str, max_units: usize) -> String {
    let mut result = String::new();
    let mut units = 0;
    for character in value.chars() {
        let character_units = character.len_utf16();
        if units + character_units > max_units {
            break;
        }
        result.push(character);
        units += character_units;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn serve_once(
        status: u16,
        body: &'static str,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 4096];
            let length = stream.read(&mut request).await.unwrap();
            request.truncate(length);
            let reason = StatusCode::from_u16(status)
                .unwrap()
                .canonical_reason()
                .unwrap_or("Error");
            let response = format!("HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            stream.write_all(response.as_bytes()).await.unwrap();
            String::from_utf8(request).unwrap()
        });
        (format!("http://{address}/api"), task)
    }

    #[tokio::test]
    async fn sends_scrappa_auth_headers_and_get_parameters() {
        let (base_url, task) = serve_once(200, r#"{"data":{"predictedValue":850000}}"#).await;
        let client = ScrappaClient::new("test-key".to_owned(), Some(base_url), 1_000).unwrap();
        let response = client
            .get("/redfin/valuation", "194191988", Some("207388793"))
            .await
            .unwrap();
        let request = task.await.unwrap();
        assert_eq!(response["data"]["predictedValue"], 850000);
        assert!(request
            .starts_with("GET /api/redfin/valuation?property_id=194191988&listing_id=207388793 "));
        assert!(request.to_ascii_lowercase().contains("x-api-key: test-key"));
        assert!(request
            .to_ascii_lowercase()
            .contains("accept: application/json"));
        assert!(request.contains(ACTOR_USER_AGENT));
    }

    #[tokio::test]
    async fn formats_scrappa_validation_errors_and_marks_retryable_statuses() {
        let (base_url, task) = serve_once(422, r#"{"message":"Invalid request","errors":{"property_id":["The property ID is required."]}}"#).await;
        let client = ScrappaClient::new("test-key".to_owned(), Some(base_url), 1_000).unwrap();
        let error = client
            .get("/redfin/valuation", "1", None)
            .await
            .unwrap_err();
        task.await.unwrap();
        assert_eq!(error.status(), Some(422));
        assert_eq!(
            error.details(),
            Some("Invalid request - property_id: The property ID is required.")
        );
        assert!(!error.is_retryable());
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(ScrappaError::Http {
                status,
                details: String::new()
            }
            .is_retryable());
        }
        assert_eq!(get_retry_delay_ms(1, 500), 2_500);
        assert_eq!(get_retry_delay_ms(2, 900), 4_900);
        assert_eq!(get_retry_delay_ms(4, 999), 10_000);
    }

    #[tokio::test]
    async fn enforces_the_configured_upstream_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        });
        let client = ScrappaClient::new(
            "test-key".to_owned(),
            Some(format!("http://{address}/api")),
            5,
        )
        .unwrap();
        let error = client
            .send("/redfin/valuation", "1", None)
            .await
            .unwrap_err();
        assert!(matches!(error, ScrappaError::Timeout { timeout_ms: 5 }));
        server.abort();
    }
}
