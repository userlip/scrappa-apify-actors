use std::time::Duration;

use anyhow::{Context, Result};
use rand::Rng;
use reqwest::{header, Client, StatusCode};
use serde_json::Value;
use url::Url;

use crate::{apify::endpoint_url, doctor_details::js_string};

pub(crate) const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
pub(crate) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
pub(crate) const SCRAPPA_MAX_ATTEMPTS: usize = 3;

pub(crate) struct ScrappaClient {
    http: Client,
    base_url: String,
    api_key: String,
}

#[derive(Debug)]
pub(crate) struct ScrappaError {
    pub(crate) message: String,
    pub(crate) retryable: bool,
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ScrappaError {}

impl ScrappaClient {
    pub(crate) fn new(api_key: String, base_url: String) -> Result<Self> {
        let http = Client::builder()
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .build()
            .context("Failed to configure Scrappa API client")?;
        Ok(Self {
            http,
            base_url,
            api_key,
        })
    }

    pub(crate) async fn get(&self, doctor_url: &str) -> std::result::Result<Value, ScrappaError> {
        self.get_with_delay(doctor_url, |attempt| {
            let jitter_ms = rand::thread_rng().gen_range(0..1000);
            Duration::from_millis(get_retry_delay_ms(attempt, jitter_ms))
        })
        .await
    }

    pub(crate) async fn get_with_delay<F>(
        &self,
        doctor_url: &str,
        mut retry_delay: F,
    ) -> std::result::Result<Value, ScrappaError>
    where
        F: FnMut(usize) -> Duration,
    {
        let mut url =
            endpoint_url(&self.base_url, &["jameda", "doctor-details"]).map_err(|error| {
                ScrappaError {
                    message: error.to_string(),
                    retryable: false,
                }
            })?;
        url.query_pairs_mut().append_pair("doctor_url", doctor_url);

        let mut last_error = None;
        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send(url.clone()).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let retryable = error.retryable;
                    last_error = Some(error);
                    if attempt == SCRAPPA_MAX_ATTEMPTS || !retryable {
                        break;
                    }

                    let delay = retry_delay(attempt);
                    let error = last_error.as_ref().expect("Scrappa request error recorded");
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {}ms.",
                        error.message,
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(last_error.expect("Scrappa request made at least one attempt"))
    }

    async fn send(&self, url: Url) -> std::result::Result<Value, ScrappaError> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .header(
                header::USER_AGENT,
                "thescrappa-jameda-doctor-details-scraper/1.0",
            )
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ScrappaError {
                        message: format!(
                            "Scrappa API request timed out after {}ms",
                            SCRAPPA_REQUEST_TIMEOUT.as_millis()
                        ),
                        retryable: true,
                    }
                } else {
                    ScrappaError {
                        message: "fetch failed".to_owned(),
                        retryable: true,
                    }
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let fallback = status
                .canonical_reason()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            let body = response.text().await.map_err(|error| {
                if error.is_timeout() {
                    ScrappaError {
                        message: format!(
                            "Scrappa API request timed out after {}ms",
                            SCRAPPA_REQUEST_TIMEOUT.as_millis()
                        ),
                        retryable: true,
                    }
                } else {
                    ScrappaError {
                        message: format!("Scrappa API error ({}): {fallback}", status.as_u16()),
                        retryable: is_retryable_scrappa_status(status),
                    }
                }
            })?;
            return Err(ScrappaError {
                message: format!(
                    "Scrappa API error ({}): {}",
                    status.as_u16(),
                    scrappa_error_message(status.as_u16(), &body, &fallback)
                ),
                retryable: is_retryable_scrappa_status(status),
            });
        }

        response.json::<Value>().await.map_err(|error| {
            if error.is_timeout() {
                ScrappaError {
                    message: format!(
                        "Scrappa API request timed out after {}ms",
                        SCRAPPA_REQUEST_TIMEOUT.as_millis()
                    ),
                    retryable: true,
                }
            } else {
                ScrappaError {
                    message: error.to_string(),
                    retryable: false,
                }
            }
        })
    }
}

pub(crate) fn is_retryable_scrappa_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
}

pub(crate) fn get_retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponential =
        1000u64.saturating_mul(1u64.checked_shl(failed_attempt as u32).unwrap_or(u64::MAX));
    exponential.saturating_add(jitter_ms).min(10_000)
}

pub(crate) fn scrappa_error_message(status: u16, body: &str, fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }
    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let Some(error_data) = error_data.as_object() else {
            return fallback.to_owned();
        };
        let mut message = error_data
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or_else(|| fallback.to_owned());
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
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
        return message;
    }
    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        fallback.to_owned()
    } else {
        let message: String = collapsed.chars().take(500).collect();
        if message.is_empty() {
            format!("HTTP {status}")
        } else {
            message
        }
    }
}
