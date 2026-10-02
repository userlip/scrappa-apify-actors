use crate::scrappa_retry::{ScrappaRetryExt, ENTRY_TIME_BUDGET};
use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::{Client, StatusCode, Url};
use serde_json::Value;

pub const REQUEST_TIMEOUT_MS: u64 = 120_000;

pub struct ScrappaClient {
    client: Client,
    api_key: String,
    base_url: Url,
    timeout: Duration,
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
            timeout: Duration::from_millis(timeout_ms),
        })
    }

    pub async fn get(&self, endpoint: &str, params: &Value) -> Result<Value> {
        self.request_once(endpoint, params)
            .await
            .map_err(anyhow::Error::new)
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
            .timeout(self.timeout)
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send_scrappa_with_retry("Scrappa API request")
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ScrappaError::new(
                        ScrappaErrorKind::Timeout,
                        format!(
                            "Scrappa API retry budget expired after {}ms",
                            ENTRY_TIME_BUDGET.as_millis()
                        ),
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
            format!(
                "Scrappa API retry budget expired after {}ms",
                ENTRY_TIME_BUDGET.as_millis()
            ),
        )
    } else {
        ScrappaError::new(ScrappaErrorKind::Other, format!("{message}: {error}"))
    }
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
            .get(
                "/images",
                &json!({"q":"coffee product photography", "page":2}),
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
            .get("/images", &json!({"q":"invalid"}))
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Scrappa API error (400): Invalid query - q: required"
        );
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn keeps_the_configured_client_timeout_for_the_retry_helper_to_cap() {
        assert_eq!(REQUEST_TIMEOUT_MS, 120_000);
    }
}
