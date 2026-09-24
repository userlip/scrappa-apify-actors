use rand::random;
use reqwest::{header, Client, Response};
use serde_json::Value;
use std::{collections::BTreeMap, error::Error, fmt, time::Duration};
use tokio::time::{sleep, timeout};
use url::Url;

use crate::booking::booking_search_url;

pub const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
pub const SCRAPPA_MAX_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrappaErrorKind {
    Timeout,
    Network,
    Api(u16),
    InvalidJson,
    Other,
}

#[derive(Debug)]
pub struct ScrappaError {
    message: String,
    kind: ScrappaErrorKind,
}

impl ScrappaError {
    fn timeout() -> Self {
        Self {
            message: format!(
                "Scrappa API request timed out after {}ms",
                SCRAPPA_REQUEST_TIMEOUT.as_millis()
            ),
            kind: ScrappaErrorKind::Timeout,
        }
    }

    fn network() -> Self {
        Self {
            message: "Scrappa API network request failed".to_owned(),
            kind: ScrappaErrorKind::Network,
        }
    }

    pub(crate) fn is_timeout(&self) -> bool {
        self.kind == ScrappaErrorKind::Timeout
    }

    pub fn is_retryable(&self) -> bool {
        match self.kind {
            ScrappaErrorKind::Timeout | ScrappaErrorKind::Network => true,
            ScrappaErrorKind::Api(status) => matches!(status, 408 | 429 | 500 | 502 | 503 | 504),
            ScrappaErrorKind::InvalidJson | ScrappaErrorKind::Other => false,
        }
    }
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ScrappaError {}

pub fn retry_delay_ms(failed_attempt: u32, jitter_ms: u64) -> u64 {
    let backoff = 1_000u64.saturating_mul(2u64.saturating_pow(failed_attempt));
    backoff.saturating_add(jitter_ms).min(10_000)
}

pub struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
}

impl ScrappaClient {
    pub fn new(http: Client, base_url: Url, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    pub async fn get(
        &self,
        params: &BTreeMap<String, Value>,
    ) -> std::result::Result<Value, ScrappaError> {
        let url = booking_search_url(&self.base_url, params).map_err(|error| ScrappaError {
            message: error.to_string(),
            kind: ScrappaErrorKind::Other,
        })?;

        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            let result = match timeout(SCRAPPA_REQUEST_TIMEOUT, self.send_once(&url)).await {
                Ok(result) => result,
                Err(_) => Err(ScrappaError::timeout()),
            };
            match result {
                Ok(data) => return Ok(data),
                Err(error) => {
                    if attempt == SCRAPPA_MAX_ATTEMPTS || !error.is_retryable() {
                        return Err(error);
                    }
                    let jitter_ms = (random::<f64>() * 1_000.0) as u64;
                    let delay_ms = retry_delay_ms(attempt, jitter_ms);
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {delay_ms}ms.",
                        attempt + 1
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }

        unreachable!("The retry loop returns after its final attempt")
    }

    async fn send_once(&self, url: &Url) -> std::result::Result<Value, ScrappaError> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, "thescrappa-booking-search-scraper/1.0")
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ScrappaError::timeout()
                } else {
                    ScrappaError::network()
                }
            })?;

        if !response.status().is_success() {
            return Err(scrappa_api_error(response).await);
        }

        response.json().await.map_err(|_| ScrappaError {
            message: "Scrappa API response was not valid JSON".to_owned(),
            kind: ScrappaErrorKind::InvalidJson,
        })
    }
}

pub fn parsed_scrappa_error_message(body: &str, fallback: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    let mut message = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned();
    if let Some(errors) = value.get("errors").and_then(Value::as_object) {
        let messages = errors
            .iter()
            .filter_map(|(field, values)| {
                let values = values.as_array()?;
                let values = values.iter().filter_map(Value::as_str).collect::<Vec<_>>();
                (!values.is_empty()).then(|| format!("{field}: {}", values.join(", ")))
            })
            .collect::<Vec<_>>();
        if !messages.is_empty() {
            message.push_str(" - ");
            message.push_str(&messages.join("; "));
        }
    }
    Some(message)
}

async fn scrappa_api_error(response: Response) -> ScrappaError {
    let status = response.status();
    let fallback = status
        .canonical_reason()
        .unwrap_or("Unknown status")
        .to_owned();
    let message = match response.text().await {
        Ok(body) if !body.is_empty() => parsed_scrappa_error_message(&body, &fallback)
            .unwrap_or_else(|| {
                body.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(500)
                    .collect()
            }),
        Ok(_) => fallback,
        Err(error) if error.is_timeout() => return ScrappaError::timeout(),
        Err(_) => fallback,
    };
    ScrappaError {
        message: format!("Scrappa API error ({}): {message}", status.as_u16()),
        kind: ScrappaErrorKind::Api(status.as_u16()),
    }
}
