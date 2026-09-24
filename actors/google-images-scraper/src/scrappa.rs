use std::time::Duration;

use anyhow::{anyhow, Result};
use rand::Rng;
use reqwest::{Client, StatusCode, Url};
use serde_json::Value;

pub const REQUEST_TIMEOUT_MS: u64 = 120_000;
pub const REQUEST_ATTEMPTS: usize = 2;
const RETRYABLE_STATUSES: [u16; 6] = [408, 429, 500, 502, 503, 504];

pub struct ScrappaClient {
    client: Client,
    api_key: String,
    base_url: Url,
}

impl ScrappaClient {
    pub fn new(api_key: impl Into<String>, base_url: &str) -> Result<Self> {
        Self::with_timeout(api_key, base_url.to_owned(), REQUEST_TIMEOUT_MS)
    }

    pub fn with_timeout(
        api_key: impl Into<String>,
        base_url: String,
        timeout_ms: u64,
    ) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_millis(timeout_ms))
                .build()
                .map_err(|error| anyhow!("Could not create Scrappa HTTP client: {error}"))?,
            api_key: api_key.into(),
            base_url: Url::parse(&base_url)
                .map_err(|error| anyhow!("Scrappa API base URL must be valid: {error}"))?,
        })
    }

    pub async fn get(&self, endpoint: &str, params: &Value) -> Result<Value> {
        self.get_with_retry_delay(endpoint, params, retry_delay)
            .await
    }

    async fn get_with_retry_delay<F>(
        &self,
        endpoint: &str,
        params: &Value,
        mut delay: F,
    ) -> Result<Value>
    where
        F: FnMut(usize) -> Duration,
    {
        let mut last_error = None;
        for attempt in 1..=REQUEST_ATTEMPTS {
            match self.request_once(endpoint, params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt >= REQUEST_ATTEMPTS || !error.retryable() {
                        return Err(anyhow!(error));
                    }
                    let wait = delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        REQUEST_ATTEMPTS,
                        wait.as_millis()
                    );
                    tokio::time::sleep(wait).await;
                    last_error = Some(error);
                }
            }
        }
        Err(anyhow!(
            last_error.expect("at least one request attempt is made")
        ))
    }

    async fn request_once(
        &self,
        endpoint: &str,
        params: &Value,
    ) -> std::result::Result<Value, ScrappaError> {
        let mut url = self.base_url.clone();
        let path = format!("{}{}", url.path().trim_end_matches('/'), endpoint);
        url.set_path(&path);
        if let Some(params) = params.as_object() {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                if !value.is_null() && value != "" {
                    query.append_pair(key, &json_string(value));
                }
            }
        }

        let response = self
            .client
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ScrappaError::new(
                        ScrappaErrorKind::Timeout,
                        format!("Scrappa API request timed out after {REQUEST_TIMEOUT_MS}ms"),
                    )
                } else {
                    ScrappaError::new(
                        ScrappaErrorKind::Connection,
                        "Scrappa API connection failed".into(),
                    )
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.map_err(|error| {
                response_read_error(error, "Failed to read Scrappa API error response")
            })?;
            return Err(ScrappaError::new(
                ScrappaErrorKind::Http(status.as_u16()),
                format!(
                    "Scrappa API error ({}): {}",
                    status.as_u16(),
                    http_error_details(status, &body)
                ),
            ));
        }

        let body = response
            .text()
            .await
            .map_err(|error| response_read_error(error, "Failed to read Scrappa API response"))?;
        serde_json::from_str(&body).map_err(|error| {
            ScrappaError::new(
                ScrappaErrorKind::Other,
                format!("Scrappa API response is not valid JSON: {error}"),
            )
        })
    }
}

fn response_read_error(error: reqwest::Error, message: &str) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::new(
            ScrappaErrorKind::Timeout,
            format!("Scrappa API request timed out after {REQUEST_TIMEOUT_MS}ms"),
        )
    } else {
        ScrappaError::new(ScrappaErrorKind::Other, format!("{message}: {error}"))
    }
}

fn retry_delay(failed_attempt: usize) -> Duration {
    let base = 1_000_u64
        .saturating_mul(2_u64.saturating_pow(failed_attempt as u32))
        .min(10_000);
    let jitter = rand::rng().random_range(0..1_000_u64);
    Duration::from_millis((base + jitter).min(10_000))
}

fn json_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values.iter().map(json_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn http_error_details(status: StatusCode, body: &str) -> String {
    let fallback = format!("HTTP {}", status.as_u16());
    let Ok(error_data) = serde_json::from_str::<Value>(body) else {
        return if body.is_empty() {
            fallback
        } else {
            body.to_owned()
        };
    };
    let mut message = error_data
        .get("message")
        .filter(|value| !value.is_null())
        .map(json_string)
        .unwrap_or(fallback);
    if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .map(|(field, messages)| {
                let joined = messages
                    .as_array()
                    .map(|messages| {
                        messages
                            .iter()
                            .map(json_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_else(|| json_string(messages));
                format!("{field}: {joined}")
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

#[derive(Debug, Clone, Copy)]
enum ScrappaErrorKind {
    Timeout,
    Connection,
    Http(u16),
    Other,
}

#[derive(Debug)]
struct ScrappaError {
    kind: ScrappaErrorKind,
    message: String,
}

impl ScrappaError {
    fn new(kind: ScrappaErrorKind, message: String) -> Self {
        Self { kind, message }
    }

    fn retryable(&self) -> bool {
        match self.kind {
            ScrappaErrorKind::Timeout | ScrappaErrorKind::Connection => true,
            ScrappaErrorKind::Http(status) => RETRYABLE_STATUSES.contains(&status),
            ScrappaErrorKind::Other => false,
        }
    }
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ScrappaError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockResponse, MockServer};
    use serde_json::json;

    #[tokio::test]
    async fn retries_a_transient_response_once_with_auth_and_query_params() {
        let server = MockServer::start(vec![
            MockResponse {
                status: 502,
                body: String::new(),
            },
            MockResponse {
                status: 200,
                body: r#"[{"position":1}]"#.into(),
            },
        ]);
        let client = ScrappaClient::with_timeout(
            "test-key",
            format!("{}/api", server.base_url),
            REQUEST_TIMEOUT_MS,
        )
        .unwrap();

        let response = client
            .get_with_retry_delay(
                "/images",
                &json!({"q":"coffee product photography", "page":2}),
                |_| Duration::ZERO,
            )
            .await
            .unwrap();

        assert_eq!(response, json!([{"position":1}]));
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert!(
            requests[0].starts_with("GET /api/images?page=2&q=coffee+product+photography HTTP/1.1")
        );
        assert!(requests[0]
            .lines()
            .any(|line| line.eq_ignore_ascii_case("X-API-Key: test-key")));
        assert!(requests[0]
            .lines()
            .any(|line| line.eq_ignore_ascii_case("Accept: application/json")));
    }

    #[tokio::test]
    async fn does_not_retry_invalid_requests_and_preserves_error_details() {
        let server = MockServer::start(vec![MockResponse {
            status: 400,
            body: r#"{"message":"Invalid query","errors":{"q":["required"]}}"#.into(),
        }]);
        let client = ScrappaClient::with_timeout(
            "test-key",
            format!("{}/api", server.base_url),
            REQUEST_TIMEOUT_MS,
        )
        .unwrap();

        let error = client
            .get_with_retry_delay("/images", &json!({"q":"invalid"}), |_| Duration::ZERO)
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Scrappa API error (400): Invalid query - q: required"
        );
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn keeps_the_original_per_request_deadline_and_attempt_count() {
        assert_eq!(REQUEST_TIMEOUT_MS, 120_000);
        assert_eq!(REQUEST_ATTEMPTS, 2);
    }
}
