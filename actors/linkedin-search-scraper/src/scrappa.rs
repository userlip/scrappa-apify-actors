use std::{
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::{header, Client, Response, StatusCode};
use serde_json::Value;
use url::Url;

use crate::search::append_search_params;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub const MAX_ATTEMPTS: usize = 3;
const MAX_RETRY_DELAY: Duration = Duration::from_secs(10);
const USER_AGENT: &str = "thescrappa-linkedin-search-scraper/1.0";

#[derive(Debug)]
pub enum ScrappaError {
    Timeout,
    Api { status: StatusCode, message: String },
    Request(reqwest::Error),
    InvalidJson(reqwest::Error),
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
                write!(
                    formatter,
                    "Scrappa API error ({}): {message}",
                    status.as_u16()
                )
            }
            Self::Request(error) | Self::InvalidJson(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ScrappaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Request(error) | Self::InvalidJson(error) => Some(error),
            Self::Timeout | Self::Api { .. } => None,
        }
    }
}

impl ScrappaError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout => true,
            Self::Api { status, .. } => {
                matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
            }
            Self::Request(_) | Self::InvalidJson(_) => false,
        }
    }

    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
    }
}

#[derive(Clone, Copy)]
pub struct RetryPolicy {
    pub attempts: usize,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter_max_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            attempts: MAX_ATTEMPTS,
            base_delay: Duration::from_secs(1),
            max_delay: MAX_RETRY_DELAY,
            jitter_max_ms: 1000,
        }
    }
}

impl RetryPolicy {
    fn delay_after(&self, failed_attempt: usize) -> Duration {
        let jitter_ms = if self.jitter_max_ms == 0 {
            0
        } else {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos() as u64
                % self.jitter_max_ms
        };
        retry_delay(self.base_delay, self.max_delay, failed_attempt, jitter_ms)
    }
}

pub fn retry_delay(
    base_delay: Duration,
    max_delay: Duration,
    failed_attempt: usize,
    jitter_ms: u64,
) -> Duration {
    let exponential = base_delay.as_millis().saturating_mul(
        1u128
            .checked_shl(failed_attempt.min(127) as u32)
            .unwrap_or(u128::MAX),
    );
    let capped = exponential
        .saturating_add(jitter_ms as u128)
        .min(max_delay.as_millis());
    Duration::from_millis(capped.min(u64::MAX as u128) as u64)
}

pub struct ScrappaClient {
    client: Client,
}

impl ScrappaClient {
    pub fn new(timeout: Duration) -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: Client::builder().timeout(timeout).build()?,
        })
    }

    pub async fn search(
        &self,
        base_url: &Url,
        api_key: &str,
        input: &Value,
        retry_policy: RetryPolicy,
    ) -> std::result::Result<Value, ScrappaError> {
        let url = search_url(base_url, input);
        let attempts = retry_policy.attempts.max(1);
        let mut last_error = None;

        for attempt in 1..=attempts {
            match self.send(&url, api_key).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let should_retry = attempt < attempts && error.is_retryable();
                    if !should_retry {
                        return Err(error);
                    }
                    let delay = retry_policy.delay_after(attempt);
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        attempts,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.expect("at least one request attempt was made"))
    }

    async fn send(&self, url: &Url, api_key: &str) -> std::result::Result<Value, ScrappaError> {
        let response = self
            .client
            .get(url.clone())
            .header("X-API-Key", api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(request_error)?;

        if !response.status().is_success() {
            return Err(api_error(response).await);
        }

        response.json().await.map_err(|error| {
            if error.is_timeout() {
                ScrappaError::Timeout
            } else {
                ScrappaError::InvalidJson(error)
            }
        })
    }
}

fn request_error(error: reqwest::Error) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout
    } else {
        ScrappaError::Request(error)
    }
}

fn search_url(base_url: &Url, input: &Value) -> Url {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .expect("Scrappa base URL must support path segments")
        .pop_if_empty()
        .extend(["linkedin", "search"]);
    append_search_params(&mut url, input);
    url
}

async fn api_error(response: Response) -> ScrappaError {
    let status = response.status();
    let fallback = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.unwrap_or_default();
    let message = if body.is_empty() {
        fallback.to_owned()
    } else {
        parse_error_message(&body, fallback)
    };
    ScrappaError::Api { status, message }
}

fn parse_error_message(body: &str, fallback: &str) -> String {
    if let Ok(error) = serde_json::from_str::<Value>(body) {
        let mut message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_owned();
        if let Some(errors) = error.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let messages = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .filter_map(Value::as_str)
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
        return message;
    }

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_timeouts_and_selected_http_statuses_only() {
        assert!(ScrappaError::Timeout.is_retryable());
        assert!(ScrappaError::Api {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: String::new()
        }
        .is_retryable());
        assert!(ScrappaError::Api {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: String::new()
        }
        .is_retryable());
        assert!(!ScrappaError::Api {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: String::new()
        }
        .is_retryable());
    }

    #[test]
    fn uses_exponential_retry_delay_with_ten_second_cap() {
        let one_second = Duration::from_secs(1);
        let cap = Duration::from_secs(10);
        assert_eq!(retry_delay(one_second, cap, 1, 0), Duration::from_secs(2));
        assert_eq!(retry_delay(one_second, cap, 2, 0), Duration::from_secs(4));
        assert_eq!(retry_delay(one_second, cap, 10, 0), cap);
        assert_eq!(
            retry_delay(one_second, cap, 1, 750),
            Duration::from_millis(2750)
        );
    }

    #[test]
    fn parses_json_validation_errors_and_plain_error_bodies() {
        assert_eq!(
            parse_error_message(
                r#"{"message":"Invalid query","errors":{"query":["required","too short"]}}"#,
                "Bad Request"
            ),
            "Invalid query - query: required, too short"
        );
        assert_eq!(
            parse_error_message("upstream  failed\ntry later", "Bad Request"),
            "upstream failed try later"
        );
        assert_eq!(parse_error_message("", "Bad Request"), "");
    }
}
