use std::{error::Error, fmt, time::Duration};

use anyhow::{Context, Result, anyhow};
use rand::Rng;
use reqwest::{Client, Response, Url};
use serde_json::Value;
use tokio::time::{sleep, timeout};

use crate::error_utils::error_summary;

pub const DETAIL_REQUEST_TIMEOUT_MS: u64 = 90_000;
pub const DISCOVERY_REQUEST_TIMEOUT_MS: u64 = 60_000;
pub const MAX_DETAIL_ATTEMPTS: usize = 3;
const USER_AGENT: &str = "thescrappa-kleinanzeigen-listing-details-scraper/1.0";

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    timeout_ms: u64,
}

impl ScrappaTimeoutError {
    fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }
}

impl fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            self.timeout_ms
        )
    }
}

impl Error for ScrappaTimeoutError {}

#[derive(Debug)]
pub struct ScrappaApiError {
    pub status: u16,
    pub message: String,
}

impl fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.message
        )
    }
}

impl Error for ScrappaApiError {}

pub struct ScrappaClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self> {
        let client = Client::builder()
            .build()
            .context("Could not create Scrappa HTTP client")?;
        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }

    pub async fn search(&self, query: &str) -> Result<Value> {
        self.get(
            "/kleinanzeigen/search",
            &[("query", query), ("page", "1")],
            1,
            DISCOVERY_REQUEST_TIMEOUT_MS,
        )
        .await
    }

    pub async fn listing_detail(&self, ad_id: &str, attempts: usize) -> Result<Value> {
        self.get(
            "/kleinanzeigen/details",
            &[("ad_id", ad_id)],
            attempts,
            if attempts == 1 {
                DISCOVERY_REQUEST_TIMEOUT_MS
            } else {
                DETAIL_REQUEST_TIMEOUT_MS
            },
        )
        .await
    }

    async fn get(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
        attempts: usize,
        timeout_ms: u64,
    ) -> Result<Value> {
        let attempts = attempts.max(1);
        let mut last_error = None;

        for attempt in 1..=attempts {
            match self.send(endpoint, params, timeout_ms).await {
                Ok(value) => return Ok(value),
                Err(error) => {
                    let retryable = is_retryable_error(&error);
                    if attempt >= attempts || !retryable {
                        return Err(error);
                    }

                    let jitter_ms = rand::thread_rng().gen_range(0..=1000);
                    let delay_ms = retry_delay_ms(attempt, jitter_ms);
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{attempts} in {delay_ms}ms.",
                        error_summary(&error.to_string()),
                        attempt + 1,
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Scrappa API request failed")))
    }

    async fn send(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
        timeout_ms: u64,
    ) -> Result<Value> {
        let url = request_url(&self.base_url, endpoint, params)?;
        let request = async {
            let response = self
                .client
                .get(url)
                .header("X-API-Key", &self.api_key)
                .header(reqwest::header::ACCEPT, "application/json")
                .header(reqwest::header::USER_AGENT, USER_AGENT)
                .send()
                .await
                .map_err(anyhow::Error::new)?;

            if !response.status().is_success() {
                return Err(format_api_error(response).await);
            }

            response
                .json::<Value>()
                .await
                .context("Scrappa API response was not valid JSON")
        };

        timeout(Duration::from_millis(timeout_ms), request)
            .await
            .map_err(|_| anyhow::Error::new(ScrappaTimeoutError::new(timeout_ms)))?
    }
}

fn request_url(base_url: &str, endpoint: &str, params: &[(&str, &str)]) -> Result<Url> {
    let base = base_url.trim_end_matches('/');
    let endpoint = if endpoint.starts_with('/') {
        endpoint.to_owned()
    } else {
        format!("/{endpoint}")
    };
    let mut url = Url::parse(&format!("{base}{endpoint}"))
        .context("SCRAPPA_API_BASE_URL must be a valid URL")?;
    url.query_pairs_mut().extend_pairs(params.iter().copied());
    Ok(url)
}

pub fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponent = failed_attempt.min(10) as u32;
    (1000_u64.saturating_mul(2_u64.saturating_pow(exponent)) + jitter_ms).min(10_000)
}

pub fn is_retryable_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504))
    {
        return true;
    }
    error.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|error| error.is_timeout() || error.is_connect())
    })
}

async fn format_api_error(response: Response) -> anyhow::Error {
    let status = response.status();
    let status_code = status.as_u16();
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {status_code}"));
    let body = response.text().await.unwrap_or_default();
    let message = if body.is_empty() {
        fallback
    } else if let Some(message) = parse_json_error(&body, &fallback) {
        error_summary(&message)
    } else {
        error_summary(&body.split_whitespace().collect::<Vec<_>>().join(" "))
    };

    anyhow::Error::new(ScrappaApiError {
        status: status_code,
        message,
    })
}

fn parse_json_error(body: &str, fallback: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(body).ok()?;
    let mut message = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned();

    if let Some(errors) = value.get("errors").and_then(Value::as_object) {
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
    Some(message)
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex},
        thread::{self, JoinHandle},
        time::Duration,
    };

    use super::{
        DETAIL_REQUEST_TIMEOUT_MS, DISCOVERY_REQUEST_TIMEOUT_MS, ScrappaApiError, ScrappaClient,
        ScrappaTimeoutError, is_retryable_error, parse_json_error, request_url, retry_delay_ms,
    };
    use anyhow::Error;

    fn mock_http_server(
        responses: Vec<(u16, Duration, &'static str)>,
    ) -> (String, Arc<Mutex<Vec<String>>>, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let received_requests = Arc::clone(&requests);
        let thread = thread::spawn(move || {
            for (status, delay, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 1024];
                loop {
                    let count = stream.read(&mut buffer).unwrap();
                    bytes.extend_from_slice(&buffer[..count]);
                    if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                received_requests
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&bytes).to_ascii_lowercase());
                thread::sleep(delay);
                let reason = match status {
                    200 => "OK",
                    400 => "Bad Request",
                    503 => "Service Unavailable",
                    _ => "Mock Response",
                };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (format!("http://{address}"), requests, thread)
    }

    #[test]
    fn retries_timeout_and_transient_http_errors_only() {
        assert!(is_retryable_error(&Error::new(ScrappaTimeoutError::new(
            1000
        ))));
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_error(&Error::new(ScrappaApiError {
                status,
                message: "temporary".into(),
            })));
        }
        for status in [400, 401, 403, 404] {
            assert!(!is_retryable_error(&Error::new(ScrappaApiError {
                status,
                message: "permanent".into(),
            })));
        }
        assert_eq!(retry_delay_ms(1, 0), 2000);
        assert_eq!(retry_delay_ms(2, 0), 4000);
        assert_eq!(retry_delay_ms(8, 1000), 10_000);
        assert_eq!(DETAIL_REQUEST_TIMEOUT_MS, 90_000);
        assert_eq!(DISCOVERY_REQUEST_TIMEOUT_MS, 60_000);
    }

    #[test]
    fn builds_api_query_values_without_losing_spaces() {
        let url = request_url(
            "https://scrappa.co/api/",
            "/kleinanzeigen/search",
            &[("query", "e bike berlin"), ("page", "1")],
        )
        .unwrap();
        assert_eq!(url.path(), "/api/kleinanzeigen/search");
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "query").unwrap().1,
            "e bike berlin"
        );
    }

    #[test]
    fn formats_json_api_errors_with_field_details() {
        assert_eq!(
            parse_json_error(
                r#"{"message":"Bad input","errors":{"ad_id":["required","invalid"]}}"#,
                "fallback"
            )
            .as_deref(),
            Some("Bad input - ad_id: required, invalid")
        );
    }

    #[tokio::test]
    async fn retries_a_transient_scrappa_response_and_keeps_authentication() {
        let (base_url, requests, server) = mock_http_server(vec![
            (
                503,
                Duration::ZERO,
                r#"{"message":"temporarily unavailable"}"#,
            ),
            (200, Duration::ZERO, r#"{"data":{"ok":true}}"#),
        ]);
        let client = ScrappaClient::new("test-key".into(), base_url).unwrap();
        let response = client.get("/details", &[], 2, 5_000).await.unwrap();

        assert_eq!(response["data"]["ok"], true);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(
            requests
                .iter()
                .all(|request| request.contains("x-api-key: test-key"))
        );
        drop(requests);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn enforces_the_per_request_deadline() {
        let (base_url, requests, server) =
            mock_http_server(vec![(200, Duration::from_millis(80), r#"{"ok":true}"#)]);
        let client = ScrappaClient::new("test-key".into(), base_url).unwrap();
        let error = client.get("/details", &[], 1, 10).await.unwrap_err();

        assert_eq!(
            error.to_string(),
            "Scrappa API request timed out after 10ms"
        );
        assert_eq!(requests.lock().unwrap().len(), 1);
        server.join().unwrap();
    }
}
