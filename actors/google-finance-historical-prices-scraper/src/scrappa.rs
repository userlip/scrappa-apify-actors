use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use reqwest::{header, Client, Response};
use serde_json::Value;
use url::Url;

use crate::input::HistoricalPricesRequest;

const SCRAPPA_ENDPOINT: [&str; 2] = ["google-finance", "historical"];
const ACTOR_USER_AGENT: &str = "thescrappa-google-finance-historical-prices-scraper/1.0";

#[derive(Debug, PartialEq, Eq)]
pub enum ScrappaError {
    Http { status: u16, message: String },
    Timeout { message: String },
    Transport { message: String, retryable: bool },
    InvalidJson { message: String },
}

impl ScrappaError {
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Http { status, .. } => Some(*status),
            _ => None,
        }
    }

    pub(crate) fn retryable(&self) -> bool {
        match self {
            Self::Http { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::Timeout { .. } => true,
            Self::Transport { retryable, .. } => *retryable,
            Self::InvalidJson { .. } => false,
        }
    }
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::Timeout { message }
            | Self::Transport { message, .. }
            | Self::InvalidJson { message } => formatter.write_str(message),
        }
    }
}

pub struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
    timeout: Duration,
    max_attempts: u32,
    retry_base_delay_ms: u64,
}

impl ScrappaClient {
    pub fn new(base_url: Url, api_key: String, timeout: Duration, max_attempts: u32) -> Self {
        Self::with_retry_base_delay(base_url, api_key, timeout, max_attempts, 1000)
    }

    pub(crate) fn with_retry_base_delay(
        base_url: Url,
        api_key: String,
        timeout: Duration,
        max_attempts: u32,
        retry_base_delay_ms: u64,
    ) -> Self {
        Self {
            http: Client::new(),
            base_url,
            api_key,
            timeout,
            max_attempts: max_attempts.max(1),
            retry_base_delay_ms,
        }
    }

    pub async fn get_historical(
        &self,
        request: &HistoricalPricesRequest,
    ) -> Result<Value, ScrappaError> {
        let url = self
            .request_url(request)
            .map_err(|error| ScrappaError::Transport {
                message: error.to_string(),
                retryable: false,
            })?;

        for attempt in 1..=self.max_attempts {
            match self.send(&url).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < self.max_attempts && error.retryable() => {
                    let delay = self.retry_delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{}, in {}ms.",
                        attempt + 1,
                        self.max_attempts,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }

        Err(ScrappaError::Transport {
            message: "Scrappa API request failed without an error response".to_owned(),
            retryable: false,
        })
    }

    fn request_url(&self, request: &HistoricalPricesRequest) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("Scrappa API base URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(SCRAPPA_ENDPOINT);
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in [
                ("symbol", Some(request.symbol.clone())),
                ("exchange", request.exchange.clone()),
                ("range", request.range.map(|range| range.to_string())),
                ("start_date", request.start_date.clone()),
                ("end_date", request.end_date.clone()),
                ("interval", request.interval.clone()),
                ("hl", request.hl.clone()),
                ("gl", request.gl.clone()),
            ] {
                if let Some(value) = value {
                    query.append_pair(key, &value);
                }
            }
        }
        Ok(url)
    }

    async fn send(&self, url: &Url) -> Result<Value, ScrappaError> {
        let response = self
            .http
            .get(url.clone())
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, ACTOR_USER_AGENT)
            .header("X-API-Key", &self.api_key)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|error| self.map_transport_error(error))?;

        if !response.status().is_success() {
            return Err(self.read_http_error(response).await?);
        }
        response.json().await.map_err(|error| {
            if error.is_timeout() {
                self.timeout_error()
            } else {
                ScrappaError::InvalidJson {
                    message: format!("Scrappa API returned invalid JSON: {error}"),
                }
            }
        })
    }

    async fn read_http_error(&self, response: Response) -> Result<ScrappaError, ScrappaError> {
        let status = response.status();
        let fallback = status
            .canonical_reason()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
        let body = response.text().await.map_err(|error| {
            if error.is_timeout() {
                self.timeout_error()
            } else {
                self.map_transport_error(error)
            }
        })?;
        let message = if body.is_empty() {
            fallback
        } else if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
            self.json_error_message(&error_data, &fallback)
        } else {
            body.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(500)
                .collect()
        };
        Ok(ScrappaError::Http {
            status: status.as_u16(),
            message,
        })
    }

    fn json_error_message(&self, error_data: &Value, fallback: &str) -> String {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .or_else(|| error_data.get("error").and_then(Value::as_str))
            .unwrap_or(fallback)
            .to_owned();

        if let Some(code) = error_data.get("code").and_then(Value::as_str) {
            message.push_str(&format!(" [{code}]"));
        }
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    messages.as_array().map(|messages| {
                        let messages = messages
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ");
                        (field, messages)
                    })
                })
                .filter(|(_, messages)| !messages.is_empty())
                .map(|(field, messages)| format!("{field}: {messages}"))
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                message.push_str(&format!(" - {details}"));
            }
        }
        message
    }

    fn map_transport_error(&self, error: reqwest::Error) -> ScrappaError {
        if error.is_timeout() {
            return self.timeout_error();
        }
        ScrappaError::Transport {
            message: error.to_string(),
            retryable: error.is_connect() || error.is_body() || error.is_request(),
        }
    }

    fn timeout_error(&self) -> ScrappaError {
        ScrappaError::Timeout {
            message: format!(
                "Scrappa API request timed out after {}ms",
                self.timeout.as_millis()
            ),
        }
    }

    fn retry_delay(&self, failed_attempt: u32) -> Duration {
        let exponent = failed_attempt.min(31);
        let base = self.retry_base_delay_ms.saturating_mul(1_u64 << exponent);
        let jitter = if self.retry_base_delay_ms == 0 {
            0
        } else {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|time| (time.subsec_nanos() as u64) % 1001)
                .unwrap_or(0)
        };
        Duration::from_millis(base.saturating_add(jitter).min(10_000))
    }
}
