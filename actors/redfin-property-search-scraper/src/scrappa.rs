use std::{fmt, time::{Duration, SystemTime, UNIX_EPOCH}};

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{Map, Value};
use url::Url;

pub(crate) const DEFAULT_API_BASE_URL: &str = "https://scrappa.co/api";
pub(crate) const REQUEST_TIMEOUT_MS: u64 = 90_000;
pub(crate) const MAX_ATTEMPTS: usize = 3;
const USER_AGENT: &str = "thescrappa-redfin-property-search-scraper/1.0";

#[derive(Debug)]
pub(crate) enum ScrappaError {
    Timeout { timeout_ms: u64 },
    Network,
    Http { status: u16, details: String },
    InvalidResponse,
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout { timeout_ms } => {
                write!(formatter, "Scrappa API request timed out after {timeout_ms}ms")
            }
            Self::Network => formatter.write_str("Scrappa API network request failed"),
            Self::Http { status, details } => {
                write!(formatter, "Scrappa API error ({status}): {details}")
            }
            Self::InvalidResponse => formatter.write_str("Scrappa API response was not valid JSON"),
        }
    }
}

impl std::error::Error for ScrappaError {}

impl ScrappaError {
    pub(crate) fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout { .. } | Self::Network => true,
            Self::Http { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::InvalidResponse => false,
        }
    }

    pub(crate) fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
    }
}

pub(crate) struct ScrappaClient {
    http: Client,
    base_url: String,
    api_key: String,
    timeout_ms: u64,
}

impl ScrappaClient {
    pub(crate) fn new(http: Client, base_url: String, api_key: String) -> Self {
        Self::with_timeout(http, base_url, api_key, REQUEST_TIMEOUT_MS)
    }

    fn with_timeout(http: Client, base_url: String, api_key: String, timeout_ms: u64) -> Self {
        Self { http, base_url, api_key, timeout_ms }
    }

    pub(crate) async fn get(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        let mut last_error = None;
        for attempt in 1..=MAX_ATTEMPTS {
            match self.send(endpoint, params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let should_retry = error.is_retryable() && attempt < MAX_ATTEMPTS;
                    if should_retry {
                        let delay_ms = retry_delay_ms(attempt, jitter_ms());
                        eprintln!(
                            "Scrappa API request failed ({}). Retrying attempt {}/{MAX_ATTEMPTS} in {delay_ms}ms.",
                            error,
                            attempt + 1,
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    }
                    last_error = Some(error);
                    if !should_retry {
                        break;
                    }
                }
            }
        }
        Err(last_error.expect("at least one request attempt").into())
    }

    async fn send(&self, endpoint: &str, params: &Map<String, Value>) -> std::result::Result<Value, ScrappaError> {
        let mut url = endpoint_url(&self.base_url, endpoint).map_err(|_| ScrappaError::Network)?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                match value {
                    Value::Null => {}
                    Value::String(value) if value.is_empty() => {}
                    Value::Bool(false) => {}
                    Value::Bool(true) => { query.append_pair(key, "1"); }
                    _ => { query.append_pair(key, &query_value(value)); }
                };
            }
        }

        let response = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .timeout(Duration::from_millis(self.timeout_ms))
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ScrappaError::Timeout { timeout_ms: self.timeout_ms }
                } else {
                    ScrappaError::Network
                }
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let fallback = response.status().canonical_reason().unwrap_or("Unknown status").to_owned();
            let body = match response.text().await {
                Ok(body) => body,
                Err(error) if error.is_timeout() => {
                    return Err(ScrappaError::Timeout { timeout_ms: self.timeout_ms });
                }
                Err(_) => String::new(),
            };
            return Err(ScrappaError::Http {
                status,
                details: error_details(status, &fallback, &body),
            });
        }

        response
            .json::<Value>()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ScrappaError::Timeout { timeout_ms: self.timeout_ms }
                } else if error.is_body() {
                    ScrappaError::Network
                } else {
                    ScrappaError::InvalidResponse
                }
            })
    }
}

pub(crate) fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    (1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt as u32)) + jitter_ms).min(10_000)
}

fn jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64
}

fn endpoint_url(base_url: &str, endpoint: &str) -> Result<Url> {
    let mut url = Url::parse(&format!("{}/", base_url.trim_end_matches('/')))
        .with_context(|| format!("Invalid Scrappa API base URL: {base_url}"))?;
    let mut segments = url
        .path_segments_mut()
        .map_err(|_| anyhow::anyhow!("Scrappa API base URL cannot contain a query or fragment"))?;
    segments.pop_if_empty();
    segments.extend(endpoint.trim_start_matches('/').split('/'));
    drop(segments);
    Ok(url)
}

fn query_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        _ => value.to_string(),
    }
}

fn error_details(_status: u16, fallback: &str, body: &str) -> String {
    let Ok(error_data) = serde_json::from_str::<Value>(body) else {
        return if body.is_empty() { fallback.to_owned() } else { truncate(&collapse_whitespace(body), 500) };
    };
    let mut message = error_data
        .get("message")
        .filter(|value| !value.is_null())
        .map(json_string)
        .unwrap_or_else(|| fallback.to_owned());
    if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .map(|(field, messages)| {
                let messages = messages
                    .as_array()
                    .map(|messages| messages.iter().map(json_string).collect::<Vec<_>>().join(", "))
                    .unwrap_or_else(|| json_string(messages));
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

fn json_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) => value.to_string(),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use reqwest::StatusCode;
    use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};

    async fn test_server(responses: Vec<(u16, String)>) -> (String, tokio::task::JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 2048];
                loop {
                    let read = stream.read(&mut buffer).await.unwrap();
                    if read == 0 { break; }
                    request.extend_from_slice(&buffer[..read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") { break; }
                }
                let reason = StatusCode::from_u16(status).unwrap().canonical_reason().unwrap_or("Unknown");
                let response = format!("HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).await.unwrap();
                requests.push(request);
            }
            requests
        });
        (format!("http://{address}/api"), server)
    }

    #[test]
    fn formats_scrappa_errors_and_bounded_retry_delays() {
        assert_eq!(
            error_details(422, "Unprocessable Entity", r#"{"message":"Invalid request","errors":{"region_id":["The region ID is required."]}}"#),
            "Invalid request - region_id: The region ID is required."
        );
        assert_eq!(retry_delay_ms(1, 0), 2000);
        assert_eq!(retry_delay_ms(2, 500), 4500);
        assert_eq!(retry_delay_ms(20, 0), 10000);
    }

    #[tokio::test]
    async fn sends_authenticated_requests_and_retries_transient_http_statuses() {
        let (base_url, server) = test_server(vec![
            (429, r#"{"message":"Try later"}"#.to_owned()),
            (200, r#"{"ok":true}"#.to_owned()),
        ]).await;
        let http = Client::builder().build().unwrap();
        let client = ScrappaClient::new(http, base_url, "secret".to_owned());
        let params = serde_json::from_value::<Map<String, Value>>(json!({"market":"seattle","page":2,"use_cache":true,"no":false,"absent":null})).unwrap();
        assert_eq!(client.get("/redfin/search", &params).await.unwrap(), json!({"ok":true}));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        let request = std::str::from_utf8(&requests[0]).unwrap();
        assert!(request.starts_with("GET /api/redfin/search?"));
        assert!(request.contains("market=seattle"));
        assert!(request.contains("page=2"));
        assert!(request.contains("use_cache=1"));
        assert!(!request.contains("no="));
        assert!(request.to_lowercase().contains("x-api-key: secret"));
        assert!(request.to_lowercase().contains("user-agent: thescrappa-redfin-property-search-scraper/1.0"));
    }

    #[tokio::test]
    async fn does_not_retry_non_transient_http_errors() {
        let (base_url, server) = test_server(vec![
            (422, r#"{"message":"Invalid request"}"#.to_owned()),
        ]).await;
        let client = ScrappaClient::new(Client::new(), base_url, "secret".to_owned());
        let error = client.get("/redfin/search", &Map::new()).await.unwrap_err();
        assert!(error.to_string().contains("Scrappa API error (422): Invalid request"));
        assert_eq!(server.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn classifies_scrappa_timeout_and_retries_it() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for _ in 0..MAX_ATTEMPTS {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = [0; 1024];
                let _ = stream.read(&mut buffer).await;
                tokio::time::sleep(Duration::from_millis(80)).await;
            }
        });
        let client = ScrappaClient::with_timeout(Client::new(), format!("http://{address}/api"), "secret".to_owned(), 20);
        let error = client.get("/redfin/search", &Map::new()).await.unwrap_err();
        assert!(error.downcast_ref::<ScrappaError>().is_some_and(ScrappaError::is_timeout));
        server.abort();
    }
}
