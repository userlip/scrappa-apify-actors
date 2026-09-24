use std::{error::Error, fmt, time::Duration};

use reqwest::{Client, Response};
use serde_json::Value;
use url::Url;

const USER_AGENT: &str = "thescrappa-google-jobs-scraper/1.0";

#[derive(Debug)]
pub enum ScrappaError {
    Timeout(String),
    HttpStatus { status: u16, message: String },
    Transport(String),
    InvalidJson(String),
    Fallback(String),
}

impl ScrappaError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout(_) => true,
            Self::HttpStatus { status, .. } => {
                matches!(*status, 408 | 429 | 500 | 502 | 503 | 504)
            }
            Self::Transport(_) | Self::InvalidJson(_) | Self::Fallback(_) => false,
        }
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout(_))
    }
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout(message) | Self::Fallback(message) => formatter.write_str(message),
            Self::HttpStatus { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::Transport(message) | Self::InvalidJson(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl Error for ScrappaError {}

pub struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
    timeout: Duration,
    #[cfg(test)]
    retry_delay_override: Option<Duration>,
}

impl ScrappaClient {
    pub fn new(base_url: &str, api_key: String, timeout: Duration) -> Result<Self, ScrappaError> {
        let base_url = Url::parse(base_url)
            .map_err(|error| ScrappaError::Transport(format!("Invalid Scrappa API URL: {error}")))?;
        let http = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| ScrappaError::Transport(format!("Could not create Scrappa HTTP client: {error}")))?;
        Ok(Self {
            http,
            base_url,
            api_key,
            timeout,
            #[cfg(test)]
            retry_delay_override: None,
        })
    }

    #[cfg(test)]
    pub(crate) fn set_retry_delay_override(&mut self, delay: Duration) {
        self.retry_delay_override = Some(delay);
    }

    pub async fn get(
        &self,
        endpoint: &str,
        params: &[(String, String)],
        attempts: usize,
    ) -> Result<Value, ScrappaError> {
        let attempts = attempts.max(1);
        let mut last_error = None;
        for attempt in 1..=attempts {
            match self.send_get(endpoint, params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let should_retry = attempt < attempts && error.is_retryable();
                    let description = error.to_string();
                    last_error = Some(error);
                    if !should_retry {
                        break;
                    }
                    let delay = self.retry_delay(attempt as u32);
                    eprintln!(
                        "Scrappa API request failed ({description}). Retrying attempt {}/{attempts} in {}ms.",
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
        Err(last_error.expect("at least one Scrappa attempt is made"))
    }

    async fn send_get(
        &self,
        endpoint: &str,
        params: &[(String, String)],
    ) -> Result<Value, ScrappaError> {
        let mut url = self.endpoint_url(endpoint)?;
        url.query_pairs_mut()
            .extend_pairs(params.iter().map(|(key, value)| (key, value)));
        let response = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(|error| self.map_request_error(error))?;

        if !response.status().is_success() {
            return Err(read_http_error(response).await);
        }
        let body = response
            .text()
            .await
            .map_err(|error| self.map_request_error(error))?;
        serde_json::from_str(&body)
            .map_err(|error| ScrappaError::InvalidJson(format!("Scrappa API returned invalid JSON: {error}")))
    }

    fn endpoint_url(&self, endpoint: &str) -> Result<Url, ScrappaError> {
        let mut url = self.base_url.clone();
        let segments = endpoint.trim_start_matches('/').split('/');
        url.path_segments_mut()
            .map_err(|_| ScrappaError::Transport("Scrappa API base URL cannot contain path segments".to_owned()))?
            .pop_if_empty()
            .extend(segments);
        Ok(url)
    }

    fn map_request_error(&self, error: reqwest::Error) -> ScrappaError {
        if error.is_timeout() {
            ScrappaError::Timeout(format!(
                "Scrappa API request timed out after {}ms",
                self.timeout.as_millis()
            ))
        } else {
            ScrappaError::Transport(error.to_string())
        }
    }

    fn retry_delay(&self, failed_attempt: u32) -> Duration {
        #[cfg(test)]
        if let Some(delay) = self.retry_delay_override {
            return delay;
        }
        Duration::from_millis(get_retry_delay_ms(failed_attempt, random_jitter_ms()))
    }
}

async fn read_http_error(response: Response) -> ScrappaError {
    let status = response.status();
    let status_code = status.as_u16();
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {status_code}"));
    let body = response.text().await.unwrap_or_default();
    let message = if body.trim().is_empty() {
        fallback
    } else {
        parse_json_error(&body, &fallback).unwrap_or_else(|| {
            body.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(500)
                .collect()
        })
    };
    ScrappaError::HttpStatus {
        status: status_code,
        message,
    }
}

fn parse_json_error(body: &str, fallback: &str) -> Option<String> {
    let error_data: Value = serde_json::from_str(body).ok()?;
    let message = error_data
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned();
    let mut details = Vec::new();
    if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
        for (field, messages) in errors {
            let Some(messages) = messages.as_array() else {
                continue;
            };
            let messages = messages.iter().map(js_string).collect::<Vec<_>>().join(", ");
            if !messages.is_empty() {
                details.push(format!("{field}: {messages}"));
            }
        }
    }
    if details.is_empty() {
        Some(message)
    } else {
        Some(format!("{message} - {}", details.join("; ")))
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}

pub fn get_retry_delay_ms(failed_attempt: u32, jitter_ms: u64) -> u64 {
    let exponential = 1_000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt));
    exponential.saturating_add(jitter_ms).min(10_000)
}

fn random_jitter_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| u64::from(time.subsec_millis()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockResponse, MockServer};

    fn client(base_url: &str, timeout: Duration) -> ScrappaClient {
        ScrappaClient::new(base_url, "scrappa-test-key".to_owned(), timeout).unwrap()
    }

    #[tokio::test]
    async fn sends_query_parameters_auth_and_user_agent() {
        let server = MockServer::start(vec![MockResponse::json(
            200,
            r#"{"jobs":[{"title":"Engineer"}]}"#,
        )]);
        let http_client = client(&server.base_url(), Duration::from_secs(1));
        let response = http_client
            .get(
                "/google/jobs",
                &[
                    ("q".to_owned(), "software engineer".to_owned()),
                    ("gl".to_owned(), "us".to_owned()),
                ],
                1,
            )
            .await
            .unwrap();
        assert_eq!(response["jobs"][0]["title"], "Engineer");
        let requests = server.requests();
        assert!(requests[0].starts_with("GET /google/jobs?q=software+engineer&gl=us HTTP/1.1"));
        assert!(requests[0].to_ascii_lowercase().contains("x-api-key: scrappa-test-key"));
        assert!(requests[0].to_ascii_lowercase().contains("user-agent: thescrappa-google-jobs-scraper/1.0"));
    }

    #[tokio::test]
    async fn retries_transient_statuses_but_not_client_errors() {
        let server = MockServer::start(vec![
            MockResponse::json(504, r#"{"message":"Gateway Timeout"}"#),
            MockResponse::json(200, r#"{"jobs":[]}"#),
        ]);
        let mut http_client = client(&server.base_url(), Duration::from_secs(1));
        http_client.set_retry_delay_override(Duration::ZERO);
        assert!(http_client.get("/google/jobs", &[], 3).await.is_ok());
        assert_eq!(server.requests().len(), 2);

        let server = MockServer::start(vec![MockResponse::json(
            400,
            r#"{"message":"Invalid query","errors":{"q":["is required","must be text"]}}"#,
        )]);
        let http_client = client(&server.base_url(), Duration::from_secs(1));
        let error = http_client.get("/google/jobs", &[], 3).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (400): Invalid query - q: is required, must be text"
        );
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn maps_the_sixty_second_request_deadline_to_a_retryable_timeout() {
        let server = MockServer::start(vec![MockResponse::delayed_json(
            200,
            r#"{"jobs":[]}"#,
            Duration::from_millis(100),
        )]);
        let http_client = client(&server.base_url(), Duration::from_millis(20));
        let error = http_client.get("/google/jobs", &[], 1).await.unwrap_err();
        assert!(matches!(error, ScrappaError::Timeout(_)));
        assert!(error.is_retryable());
        assert_eq!(
            get_retry_delay_ms(1, 250),
            2_250
        );
        assert_eq!(get_retry_delay_ms(2, 500), 4_500);
        assert_eq!(get_retry_delay_ms(4, 750), 10_000);
    }
}
