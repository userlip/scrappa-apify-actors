use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use reqwest::{Client, Response};
use serde_json::Value;
use url::Url;

pub const REQUEST_TIMEOUT_MS: u64 = 90_000;
pub const MAX_ATTEMPTS: usize = 3;
const BASE_RETRY_DELAY_MS: u64 = 1_000;
const MAX_RETRY_DELAY_MS: u64 = 10_000;
const USER_AGENT: &str = "thescrappa-vinted-user-items-scraper/1.0";

#[derive(Debug)]
pub enum ScrappaError {
    Timeout,
    Api { status: u16, message: String },
    Transport(reqwest::Error),
    InvalidJson(String),
    InvalidBaseUrl(String),
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(
                formatter,
                "Scrappa API request timed out after {REQUEST_TIMEOUT_MS}ms"
            ),
            Self::Api { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::Transport(error) => write!(formatter, "{error}"),
            Self::InvalidJson(error) => write!(formatter, "{error}"),
            Self::InvalidBaseUrl(base) => write!(formatter, "Invalid API base URL: {base}"),
        }
    }
}

impl Error for ScrappaError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            Self::InvalidJson(_) => None,
            _ => None,
        }
    }
}

pub struct ScrappaClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl ScrappaClient {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            base_url: base_url.into(),
        }
    }

    pub async fn get(
        &self,
        endpoint: &str,
        params: &[(&str, String)],
    ) -> Result<Value, ScrappaError> {
        let url = endpoint_url(&self.base_url, endpoint)?;
        for attempt in 1..=MAX_ATTEMPTS {
            match self.send(&url, params).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < MAX_ATTEMPTS && is_retryable(&error) => {
                    let delay_ms = get_retry_delay_ms(attempt - 1, random_jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{MAX_ATTEMPTS} in {delay_ms}ms.",
                        error, attempt + 1,
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("the retry loop always returns its last result")
    }

    async fn send(&self, url: &Url, params: &[(&str, String)]) -> Result<Value, ScrappaError> {
        let response = self
            .client
            .get(url.clone())
            .query(params)
            .header("X-API-Key", &self.api_key)
            .header("Accept", "application/json")
            .header("User-Agent", USER_AGENT)
            .timeout(Duration::from_millis(REQUEST_TIMEOUT_MS))
            .send()
            .await
            .map_err(map_transport_error)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = read_error_message(response, status).await?;
            return Err(ScrappaError::Api { status, message });
        }
        response.json().await.map_err(|error| map_json_error(error))
    }
}

pub fn get_retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let multiplier = 1_u64.checked_shl(failed_attempt as u32).unwrap_or(u64::MAX);
    BASE_RETRY_DELAY_MS
        .saturating_mul(multiplier)
        .saturating_add(jitter_ms)
        .min(MAX_RETRY_DELAY_MS)
}

pub fn is_retryable(error: &ScrappaError) -> bool {
    match error {
        ScrappaError::Timeout => true,
        ScrappaError::Api { status, .. } => is_retryable_status(*status),
        ScrappaError::Transport(error) => {
            error.is_timeout() || error.is_connect() || error.is_body()
        }
        ScrappaError::InvalidJson(_) | ScrappaError::InvalidBaseUrl(_) => false,
    }
}

fn is_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 429 | 500 | 502 | 503 | 504)
}

fn map_transport_error(error: reqwest::Error) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout
    } else {
        ScrappaError::Transport(error)
    }
}

fn map_json_error(error: reqwest::Error) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout
    } else if error.is_decode() {
        ScrappaError::InvalidJson(error.to_string())
    } else {
        ScrappaError::Transport(error)
    }
}

async fn read_error_message(response: Response, status: u16) -> Result<String, ScrappaError> {
    let fallback = response
        .status()
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {status}"));
    let body = response.text().await.map_err(map_transport_error)?;
    if body.is_empty() {
        return Ok(fallback);
    }
    if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(&fallback)
            .to_owned();
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    messages.as_array().map(|messages| {
                        let joined = messages
                            .iter()
                            .map(|message| message.as_str().unwrap_or_default())
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{field}: {joined}")
                    })
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details);
            }
        }
        return Ok(message);
    }
    Ok(body
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect())
}

fn endpoint_url(base: &str, endpoint: &str) -> Result<Url, ScrappaError> {
    let mut url = Url::parse(base.trim_end_matches('/'))
        .map_err(|_| ScrappaError::InvalidBaseUrl(base.to_owned()))?;
    let mut path = url
        .path_segments_mut()
        .map_err(|_| ScrappaError::InvalidBaseUrl(base.to_owned()))?;
    path.pop_if_empty();
    for segment in endpoint.trim_matches('/').split('/') {
        path.push(segment);
    }
    drop(path);
    Ok(url)
}

fn random_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64
        % 1_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exponential_retry_delay_matches_actor_policy() {
        assert_eq!(get_retry_delay_ms(0, 0), 1_000);
        assert_eq!(get_retry_delay_ms(0, 999), 1_999);
        assert_eq!(get_retry_delay_ms(1, 0), 2_000);
        assert_eq!(get_retry_delay_ms(20, 0), 10_000);
    }

    #[test]
    fn retries_only_transient_api_statuses_and_transport_timeouts() {
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable(&ScrappaError::Api {
                status,
                message: "temporary".to_owned(),
            }));
        }
        for status in [400, 401, 403, 404, 422] {
            assert!(!is_retryable(&ScrappaError::Api {
                status,
                message: "request rejected".to_owned(),
            }));
        }
        assert!(is_retryable(&ScrappaError::Timeout));
        assert!(!is_retryable(&ScrappaError::InvalidBaseUrl(
            "bad".to_owned()
        )));
    }

    #[test]
    fn formats_api_validation_errors_like_the_typescript_client() {
        let body = r#"{"message":"Invalid request","errors":{"user_id":["is required"],"page":["must be an integer"]}}"#;
        let error = serde_json::from_str::<Value>(body).unwrap();
        let mut message = error["message"].as_str().unwrap().to_owned();
        let details = error["errors"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(field, messages)| {
                format!(
                    "{field}: {}",
                    messages
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(Value::as_str)
                        .map(Option::unwrap)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        message.push_str(" - ");
        message.push_str(&details);
        assert_eq!(
            message,
            "Invalid request - user_id: is required; page: must be an integer"
        );
    }
}
