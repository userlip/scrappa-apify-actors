use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use reqwest::{header, Client, StatusCode};
use serde_json::Value;
use url::Url;

use crate::{api_url::endpoint_url, pinterest_input::PinterestSearchParams};

pub(crate) const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
pub(crate) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
pub(crate) const SCRAPPA_MAX_ATTEMPTS: usize = 3;

#[derive(Debug)]
pub(crate) struct ScrappaApiError {
    pub(crate) status: u16,
    pub(crate) message: String,
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

#[derive(Debug)]
pub(crate) struct ScrappaTimeoutError;

impl std::fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            SCRAPPA_REQUEST_TIMEOUT.as_millis()
        )
    }
}

impl std::error::Error for ScrappaTimeoutError {}

pub(crate) struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
}

impl ScrappaClient {
    pub(crate) fn new(base_url: Url, api_key: String) -> Result<Self> {
        let http = Client::builder()
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .build()
            .context("Failed to create Scrappa HTTP client")?;
        Ok(Self {
            http,
            base_url,
            api_key,
        })
    }

    pub(crate) async fn pinterest_search(&self, params: &PinterestSearchParams) -> Result<Value> {
        let mut url = endpoint_url(&self.base_url, &["pinterest", "search"])?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("query", &params.query);
            query.append_pair("limit", &params.limit.to_string());
            if let Some(bookmark) = &params.bookmark {
                query.append_pair("bookmark", bookmark);
            }
        }

        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send_search(&url).await {
                Ok(response) => return Ok(response),
                Err(error)
                    if attempt < SCRAPPA_MAX_ATTEMPTS && is_retryable_scrappa_error(&error) =>
                {
                    let delay = scrappa_retry_delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {}ms.",
                        error,
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("Scrappa attempts always return or error")
    }

    async fn send_search(&self, url: &Url) -> Result<Value> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(
                header::USER_AGENT,
                "thescrappa-pinterest-search-scraper/1.0",
            )
            .send()
            .await
            .map_err(scrappa_transport_error)?;

        let status = response.status();
        let body = response.text().await.map_err(scrappa_transport_error)?;
        if !status.is_success() {
            return Err(ScrappaApiError {
                status: status.as_u16(),
                message: scrappa_api_error_message(status, &body),
            }
            .into());
        }

        serde_json::from_str(&body).context("Scrappa API response was not valid JSON")
    }
}

fn scrappa_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow::Error::new(ScrappaTimeoutError)
    } else {
        anyhow::Error::new(error)
    }
}

pub(crate) fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if let Some(error) = error.downcast_ref::<ScrappaApiError>() {
        return matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504);
    }
    error
        .downcast_ref::<reqwest::Error>()
        .is_some_and(|error| error.is_timeout() || error.is_connect())
}

pub(crate) fn scrappa_retry_delay(failed_attempt: usize) -> Duration {
    let exponent = u32::try_from(failed_attempt).unwrap_or(u32::MAX);
    let base_ms = 1000_u64.saturating_mul(2_u64.saturating_pow(exponent));
    let jitter_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64;
    Duration::from_millis(base_ms.saturating_add(jitter_ms).min(10_000))
}

pub(crate) fn scrappa_api_error_message(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    if body.is_empty() {
        return fallback;
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        if let Some(object) = error_data.as_object() {
            let mut message = object
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or(&fallback)
                .to_owned();
            if let Some(errors) = object.get("errors").and_then(Value::as_object) {
                let details = errors
                    .iter()
                    .filter_map(|(field, messages)| {
                        messages.as_array().map(|messages| {
                            let messages = messages
                                .iter()
                                .map(|message| match message {
                                    Value::String(message) => message.clone(),
                                    value => value.to_string(),
                                })
                                .collect::<Vec<_>>()
                                .join(", ");
                            format!("{field}: {messages}")
                        })
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
    }

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}
