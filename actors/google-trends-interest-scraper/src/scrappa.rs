use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client, StatusCode};
use serde_json::Value;
use url::Url;

use crate::{config::Config, input::InterestParams};

pub const SCRAPPA_ENDPOINT: &str = "/google-trends/interest";
pub const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub const SCRAPPA_MAX_ATTEMPTS: usize = 3;
pub const SCRAPPA_USER_AGENT: &str = "thescrappa-google-trends-interest-scraper/1.0";

#[derive(Debug)]
pub struct ScrappaTimeoutError;

impl fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            SCRAPPA_REQUEST_TIMEOUT.as_millis()
        )
    }
}

impl Error for ScrappaTimeoutError {}

#[derive(Debug)]
pub struct ScrappaHttpError {
    pub status: StatusCode,
    pub message: String,
}

impl fmt::Display for ScrappaHttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status.as_u16(),
            self.message
        )
    }
}

impl Error for ScrappaHttpError {}

pub fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let backoff = 1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt as u32));
    backoff.saturating_add(jitter_ms).min(10_000)
}

pub fn retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    error
        .downcast_ref::<ScrappaHttpError>()
        .is_some_and(|error| matches!(error.status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504))
}

fn scrappa_timeout_or_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow::Error::new(ScrappaTimeoutError)
    } else {
        anyhow!("{error}")
    }
}

pub fn scrappa_error_message(body: &str, fallback: &str) -> String {
    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_owned();
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let joined = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .map(|message| match message {
                                    Value::String(message) => message.clone(),
                                    other => other.to_string(),
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {joined}")
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
    if body.is_empty() {
        return fallback.to_owned();
    }
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

async fn send_scrappa_request(
    client: &Client,
    config: &Config,
    params: &InterestParams,
) -> Result<Value> {
    let mut url = Url::parse(&format!("{}{SCRAPPA_ENDPOINT}", config.scrappa_api_base))
        .context("SCRAPPA_API_BASE_URL must be a valid absolute URL")?;
    url.query_pairs_mut().extend_pairs(params.query_pairs());

    let response = client
        .get(url)
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .header(header::ACCEPT, "application/json")
        .header("X-API-Key", &config.scrappa_api_key)
        .header(header::USER_AGENT, SCRAPPA_USER_AGENT)
        .send()
        .await
        .map_err(scrappa_timeout_or_request_error)?;

    if !response.status().is_success() {
        let status = response.status();
        let fallback = status.canonical_reason().unwrap_or("Unknown status");
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) if error.is_timeout() => {
                return Err(anyhow::Error::new(ScrappaTimeoutError))
            }
            Err(_) => {
                return Err(ScrappaHttpError {
                    status,
                    message: fallback.to_owned(),
                }
                .into())
            }
        };
        return Err(ScrappaHttpError {
            status,
            message: scrappa_error_message(&body, fallback),
        }
        .into());
    }

    let body = response
        .text()
        .await
        .map_err(scrappa_timeout_or_request_error)?;
    serde_json::from_str(&body).context("Scrappa API returned invalid JSON")
}

pub async fn fetch_interest(
    client: &Client,
    config: &Config,
    params: &InterestParams,
) -> Result<Value> {
    for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
        match send_scrappa_request(client, config, params).await {
            Ok(response) => return Ok(response),
            Err(error) if attempt < SCRAPPA_MAX_ATTEMPTS && retryable_scrappa_error(&error) => {
                let jitter = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_millis() as u64;
                let delay_ms = retry_delay_ms(attempt, jitter);
                eprintln!(
                    "Scrappa API request failed ({}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {delay_ms}ms.",
                    error,
                    attempt + 1
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the Scrappa retry loop always returns after the configured attempts")
}
