use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use reqwest::{header, Client};
use serde_json::{Map, Value};
use tokio::time::sleep;
use url::Url;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_ATTEMPTS: usize = 5;
const USER_AGENT: &str = "thescrappa-google-flights-search-scraper/1.0";
static RETRY_JITTER_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    pub timeout: Duration,
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
pub struct ScrappaNetworkError {
    source: String,
}

impl std::fmt::Display for ScrappaNetworkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = &self.source;
        write!(
            formatter,
            "Scrappa API request failed before receiving a response"
        )
    }
}

impl std::error::Error for ScrappaNetworkError {}

#[derive(Debug)]
pub struct ScrappaApiError {
    pub status: u16,
    pub message: String,
}

impl std::fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.message
        )
    }
}

impl std::error::Error for ScrappaApiError {}

pub struct ScrappaClient<'a> {
    http: &'a Client,
    base_url: &'a Url,
    api_key: &'a str,
}

impl<'a> ScrappaClient<'a> {
    pub fn new(http: &'a Client, base_url: &'a Url, api_key: &'a str) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    pub async fn get(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("Scrappa API base URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(endpoint.trim_start_matches('/').split('/'));
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                if value.is_null() || value.as_str() == Some("") {
                    continue;
                }
                match value {
                    Value::Bool(true) => {
                        query.append_pair(key, "1");
                    }
                    Value::Bool(false) => {}
                    _ => {
                        query.append_pair(key, &js_string(value));
                    }
                }
            }
        }

        for attempt in 1..=MAX_ATTEMPTS {
            match self.fetch_once(&url).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < MAX_ATTEMPTS && is_retryable_scrappa_error(&error) => {
                    let delay_ms = get_retry_delay_ms(attempt, retry_jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{MAX_ATTEMPTS} in {delay_ms}ms.",
                        error,
                        attempt + 1
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("Scrappa retries always return or fail inside the attempt loop")
    }

    async fn fetch_once(&self, url: &Url) -> Result<Value> {
        let response = self
            .http
            .get(url.clone())
            .timeout(REQUEST_TIMEOUT)
            .header("X-API-Key", self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        let body = response.text().await.map_err(transport_error)?;
        if !status.is_success() {
            let message = parse_error_message(status.as_u16(), status.canonical_reason(), &body);
            return Err(ScrappaApiError {
                status: status.as_u16(),
                message,
            }
            .into());
        }
        serde_json::from_str(&body)
            .map_err(|error| anyhow!("Scrappa API response was not valid JSON: {error}"))
    }
}

pub fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some()
        || error.downcast_ref::<ScrappaNetworkError>().is_some()
    {
        return true;
    }
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504))
}

pub fn get_retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponent = u32::try_from(failed_attempt).unwrap_or(u32::MAX);
    1_000_u64
        .saturating_mul(2_u64.saturating_pow(exponent))
        .saturating_add(jitter_ms)
        .min(10_000)
}

fn retry_jitter_ms() -> u64 {
    let clock_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64;
    (clock_ms + RETRY_JITTER_COUNTER.fetch_add(7919, Ordering::Relaxed)) % 1_000
}

fn transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        return ScrappaTimeoutError {
            timeout: REQUEST_TIMEOUT,
        }
        .into();
    }
    ScrappaNetworkError {
        source: error.to_string(),
    }
    .into()
}

fn parse_error_message(status: u16, reason: Option<&str>, body: &str) -> String {
    let fallback = reason
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("HTTP {status}"));
    let Ok(data) = serde_json::from_str::<Value>(body) else {
        return if body.trim().is_empty() {
            fallback
        } else {
            collapse_whitespace(body).chars().take(500).collect()
        };
    };
    let Some(object) = data.as_object() else {
        return fallback;
    };
    let mut message = object
        .get("message")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or(fallback);
    if let Some(errors) = object.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .filter_map(|(field, messages)| {
                let messages = messages.as_array()?;
                Some(format!(
                    "{field}: {}",
                    messages
                        .iter()
                        .map(js_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
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

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_only_transient_statuses_and_transport_failures() {
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_scrappa_error(
                &ScrappaApiError {
                    status,
                    message: "temporary".to_owned()
                }
                .into()
            ));
        }
        for status in [400, 401, 403, 404, 422] {
            assert!(!is_retryable_scrappa_error(
                &ScrappaApiError {
                    status,
                    message: "permanent".to_owned()
                }
                .into()
            ));
        }
        assert!(is_retryable_scrappa_error(
            &ScrappaTimeoutError {
                timeout: REQUEST_TIMEOUT
            }
            .into()
        ));
    }

    #[test]
    fn calculates_bounded_exponential_backoff() {
        assert_eq!(get_retry_delay_ms(1, 0), 2_000);
        assert_eq!(get_retry_delay_ms(2, 500), 4_500);
        assert_eq!(get_retry_delay_ms(20, 0), 10_000);
    }

    #[test]
    fn parses_scrappa_json_errors_and_falls_back_to_plain_text() {
        assert_eq!(
            parse_error_message(
                422,
                Some("Unprocessable Entity"),
                r#"{"message":"Invalid request","errors":{"origin":["The origin airport is required."]}}"#
            ),
            "Invalid request - origin: The origin airport is required."
        );
        assert_eq!(
            parse_error_message(
                503,
                Some("Service Unavailable"),
                "  service\n temporarily   unavailable "
            ),
            "service temporarily unavailable"
        );
        assert_eq!(
            parse_error_message(503, Some("Service Unavailable"), ""),
            "Service Unavailable"
        );
    }

    #[test]
    fn uses_the_actor_timeout_and_retry_contract() {
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(30));
        assert_eq!(MAX_ATTEMPTS, 5);
    }

    #[tokio::test]
    async fn sends_scrappa_auth_and_retries_a_transient_upstream_error() {
        use std::collections::HashMap;

        let (base_url, server) = mock_server(vec![
            (503, r#"{"message":"temporarily unavailable"}"#.to_owned()),
            (200, r#"{"flights":[]}"#.to_owned()),
        ]);
        let base_url = Url::parse(&format!("{base_url}/api")).unwrap();
        let http = Client::new();
        let client = ScrappaClient::new(&http, &base_url, "scrappa-test-key");
        let params: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "origin": "JFK",
            "destination": "LAX",
            "departure_date": "2026-09-15",
            "exclude_basic": false,
            "include_baggage": true,
            "airlines": "AA,DL"
        }))
        .unwrap();

        let response = client.get("/flights/one-way", &params).await.unwrap();

        assert_eq!(response, serde_json::json!({"flights": []}));
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].target, requests[1].target);
        assert!(requests[0].target.starts_with("/api/flights/one-way?"));
        assert_eq!(requests[0].headers["x-api-key"], "scrappa-test-key");
        assert_eq!(requests[0].headers["accept"], "application/json");
        assert_eq!(
            requests[0].headers["user-agent"],
            "thescrappa-google-flights-search-scraper/1.0"
        );
        assert!(!requests[0].headers.contains_key("authorization"));

        let request_url = Url::parse(&format!("http://mock{}", requests[0].target)).unwrap();
        let query: HashMap<String, String> = request_url.query_pairs().into_owned().collect();
        assert_eq!(query["origin"], "JFK");
        assert_eq!(query["destination"], "LAX");
        assert_eq!(query["departure_date"], "2026-09-15");
        assert_eq!(query["include_baggage"], "1");
        assert_eq!(query["airlines"], "AA,DL");
        assert!(!query.contains_key("exclude_basic"));
    }

    struct CapturedRequest {
        method: String,
        target: String,
        headers: std::collections::HashMap<String, String>,
    }

    fn mock_server(
        responses: Vec<(u16, String)>,
    ) -> (String, std::thread::JoinHandle<Vec<CapturedRequest>>) {
        use std::io::{BufRead, BufReader, Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let mut requests = Vec::with_capacity(responses.len());
            for (status, response_body) in responses {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream);
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or_default().to_owned();
                let target = parts.next().unwrap_or_default().to_owned();
                let mut headers = std::collections::HashMap::new();
                let mut content_length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':') {
                        let name = name.trim().to_ascii_lowercase();
                        let value = value.trim().to_owned();
                        if name == "content-length" {
                            content_length = value.parse().unwrap_or_default();
                        }
                        headers.insert(name, value);
                    }
                }
                let mut body = vec![0; content_length];
                reader.read_exact(&mut body).unwrap();
                let mut stream = reader.into_inner();
                let reason = match status {
                    503 => "Service Unavailable",
                    201 => "Created",
                    _ => "OK",
                };
                write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                )
                .unwrap();
                requests.push(CapturedRequest {
                    method,
                    target,
                    headers,
                });
            }
            requests
        });
        (format!("http://{address}"), server)
    }
}
