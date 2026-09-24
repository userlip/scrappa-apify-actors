use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::Value;
use url::Url;

const USER_AGENT: &str = "thescrappa-similarweb-traffic-analytics-scraper/1.0";
const MAX_ATTEMPTS: usize = 2;

#[derive(Debug)]
pub(crate) struct ScrappaHttpError {
    pub(crate) status: u16,
    details: String,
}

impl std::fmt::Display for ScrappaHttpError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.details
        )
    }
}

impl std::error::Error for ScrappaHttpError {}

#[derive(Debug)]
pub(crate) struct ScrappaTimeoutError {
    timeout: Duration,
}

impl std::fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            self.timeout.as_millis()
        )
    }
}

impl std::error::Error for ScrappaTimeoutError {}

#[derive(Clone)]
pub(crate) struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
    timeout: Duration,
    retry_delay_override: Option<Duration>,
}

impl ScrappaClient {
    pub(crate) fn new(http: Client, base_url: Url, api_key: String, timeout: Duration) -> Self {
        Self {
            http,
            base_url,
            api_key,
            timeout,
            retry_delay_override: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay_override = Some(delay);
        self
    }

    pub(crate) async fn get_similarweb(&self, domain: &str) -> Result<Value> {
        let url = endpoint_url(&self.base_url, "similarweb")?;
        let mut url = url;
        url.query_pairs_mut().append_pair("domain", domain);

        for attempt in 1..=MAX_ATTEMPTS {
            match self.send(url.clone()).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt == MAX_ATTEMPTS || !is_retryable_scrappa_error(&error) {
                        return Err(error);
                    }
                    let delay = self.retry_delay_override.unwrap_or_else(|| {
                        Duration::from_millis(retry_delay_ms(attempt, random_jitter_ms()))
                    });
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        MAX_ATTEMPTS,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }

        unreachable!("the retry loop always returns a response or error")
    }

    async fn send(&self, url: Url) -> Result<Value> {
        let response = self
            .http
            .get(url)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .header("X-API-Key", &self.api_key)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    anyhow::Error::new(ScrappaTimeoutError {
                        timeout: self.timeout,
                    })
                } else {
                    anyhow::Error::new(error)
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let details = read_error_message(response, status, self.timeout).await?;
            return Err(anyhow::Error::new(ScrappaHttpError {
                status: status.as_u16(),
                details,
            }));
        }

        let bytes = response.bytes().await.map_err(|error| {
            if error.is_timeout() {
                anyhow::Error::new(ScrappaTimeoutError {
                    timeout: self.timeout,
                })
            } else {
                anyhow::Error::new(error)
            }
        })?;
        serde_json::from_slice(&bytes).map_err(|error| anyhow!("{error}"))
    }
}

fn endpoint_url(base_url: &Url, endpoint: &str) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("Scrappa API base URL cannot contain path segments"))?
        .pop_if_empty()
        .push(endpoint);
    Ok(url)
}

async fn read_error_message(
    response: Response,
    status: StatusCode,
    timeout: Duration,
) -> Result<String> {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) if error.is_timeout() => {
            return Err(anyhow::Error::new(ScrappaTimeoutError { timeout }));
        }
        Err(_) => return Ok(fallback),
    };
    if body.is_empty() {
        return Ok(fallback);
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or(fallback);
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
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

pub(crate) fn is_scrappa_not_found(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaHttpError>()
        .is_some_and(|error| error.status == StatusCode::NOT_FOUND.as_u16())
}

fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if let Some(error) = error.downcast_ref::<ScrappaHttpError>() {
        return matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504);
    }
    error.downcast_ref::<reqwest::Error>().is_some_and(|error| {
        error.is_timeout() || error.is_connect() || error.is_request() || error.is_body()
    })
}

fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    1_000_u64
        .saturating_mul(2_u64.saturating_pow(failed_attempt as u32))
        .saturating_add(jitter_ms)
        .min(10_000)
}

fn random_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::from(duration.subsec_millis()) % 1_000)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_only_transient_upstream_failures() {
        for status in [408, 429, 500, 502, 503, 504] {
            let error = anyhow::Error::new(ScrappaHttpError {
                status,
                details: "temporary".to_owned(),
            });
            assert!(is_retryable_scrappa_error(&error));
        }
        for status in [400, 401, 404] {
            let error = anyhow::Error::new(ScrappaHttpError {
                status,
                details: "permanent".to_owned(),
            });
            assert!(!is_retryable_scrappa_error(&error));
        }
        assert!(is_retryable_scrappa_error(&anyhow::Error::new(
            ScrappaTimeoutError {
                timeout: Duration::from_secs(1),
            }
        )));
    }

    #[test]
    fn calculates_the_same_capped_retry_delays() {
        assert_eq!(retry_delay_ms(1, 250), 2_250);
        assert_eq!(retry_delay_ms(4, 250), 10_000);
    }

    #[test]
    fn identifies_scrappa_not_found_as_a_domain_no_data_result() {
        let error = anyhow::Error::new(ScrappaHttpError {
            status: 404,
            details: "No traffic data available".to_owned(),
        });
        assert!(is_scrappa_not_found(&error));
    }
}
