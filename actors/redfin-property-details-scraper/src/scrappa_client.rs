use std::time::Duration;

use anyhow::{Context, Result};
use rand::Rng;
use reqwest::{header, Client};
use serde_json::Value;
use url::Url;

use crate::http_utils::endpoint_url;

pub(crate) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const SCRAPPA_MAX_ATTEMPTS: usize = 3;
pub(crate) const SCRAPPA_USER_AGENT: &str = "thescrappa-redfin-property-details-scraper/1.0";
const TERMINAL_PROPERTY_ERROR: &str = "Failed to fetch property details after multiple attempts";

#[derive(Debug)]
pub(crate) struct ScrappaApiError {
    pub(crate) kind: ScrappaApiErrorKind,
    pub(crate) status: Option<u16>,
    pub(crate) details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScrappaApiErrorKind {
    Timeout,
    Network,
    Http,
}

impl ScrappaApiError {
    fn timeout() -> Self {
        Self {
            kind: ScrappaApiErrorKind::Timeout,
            status: None,
            details: format!(
                "Scrappa API request timed out after {}ms",
                SCRAPPA_REQUEST_TIMEOUT.as_millis()
            ),
        }
    }

    fn network() -> Self {
        Self {
            kind: ScrappaApiErrorKind::Network,
            status: None,
            details: "Scrappa API network request failed".to_owned(),
        }
    }

    fn http(status: u16, details: String) -> Self {
        Self {
            kind: ScrappaApiErrorKind::Http,
            status: Some(status),
            details,
        }
    }
}

impl std::fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ScrappaApiErrorKind::Http => write!(
                formatter,
                "Scrappa API error ({}): {}",
                self.status.unwrap_or_default(),
                self.details
            ),
            _ => formatter.write_str(&self.details),
        }
    }
}

impl std::error::Error for ScrappaApiError {}

pub(crate) struct ScrappaClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    pub(crate) fn new(base_url: String, api_key: String) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(SCRAPPA_REQUEST_TIMEOUT)
                .build()
                .context("Failed to create Scrappa API HTTP client")?,
            base_url,
            api_key,
        })
    }

    pub(crate) async fn get_property(&self, property_id: u64) -> Result<Value> {
        let url = endpoint_url(&self.base_url, &["redfin", "property"])?;
        let mut last_error = None;

        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send_property_request(url.clone(), property_id).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let should_retry = is_retryable_scrappa_error(&error);
                    last_error = Some(error);
                    if !should_retry || attempt == SCRAPPA_MAX_ATTEMPTS {
                        break;
                    }

                    let delay_ms =
                        get_retry_delay_ms(attempt, rand::thread_rng().gen_range(0..1000));
                    let error = last_error.as_ref().expect("the request failed");
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        SCRAPPA_MAX_ATTEMPTS,
                        delay_ms
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }

        Err(last_error.expect("at least one Scrappa request attempt was made"))
    }

    async fn send_property_request(&self, url: Url, property_id: u64) -> Result<Value> {
        let response = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, SCRAPPA_USER_AGENT)
            .query(&[("property_id", property_id)])
            .send()
            .await
            .map_err(map_scrappa_transport_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let status_code = status.as_u16();
            let fallback = status
                .canonical_reason()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("HTTP {status_code}"));
            let details = match response.bytes().await {
                Ok(body) => scrappa_error_message(status_code, &body, &fallback),
                Err(error) if error.is_timeout() => {
                    return Err(ScrappaApiError::timeout().into());
                }
                Err(_) => fallback,
            };
            return Err(ScrappaApiError::http(status_code, details).into());
        }

        let body = response
            .bytes()
            .await
            .map_err(map_scrappa_transport_error)?;
        serde_json::from_slice(&body).context("Scrappa API response was not valid JSON")
    }
}

fn map_scrappa_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaApiError::timeout().into()
    } else {
        ScrappaApiError::network().into()
    }
}

fn scrappa_error_message(status: u16, body: &[u8], fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }

    if let Ok(error_data) = serde_json::from_slice::<Value>(body) {
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
                                .map(js_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {joined}")
                })
                .filter(|detail| !detail.ends_with(": "))
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details);
            }
        }
        return message;
    }

    let body = String::from_utf8_lossy(body);
    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let message = collapsed.chars().take(500).collect::<String>();
    if message.is_empty() {
        fallback.to_owned()
    } else if status == 0 {
        fallback.to_owned()
    } else {
        message
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    let Some(error) = error.downcast_ref::<ScrappaApiError>() else {
        return false;
    };
    match error.kind {
        ScrappaApiErrorKind::Timeout | ScrappaApiErrorKind::Network => true,
        ScrappaApiErrorKind::Http => {
            matches!(error.status, Some(408 | 429 | 500 | 502 | 503 | 504))
        }
    }
}

fn get_retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    (1000_u64
        .saturating_mul(2_u64.saturating_pow(failed_attempt as u32))
        .saturating_add(jitter_ms))
    .min(10_000)
}

pub(crate) fn is_per_property_scrappa_error(error: &ScrappaApiError) -> bool {
    matches!(error.status, Some(400 | 404 | 422))
        || (error.status == Some(500)
            && error
                .details
                .trim()
                .strip_suffix('.')
                .unwrap_or_else(|| error.details.trim())
                == TERMINAL_PROPERTY_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_transient_and_per_property_errors() {
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_scrappa_error(
                &ScrappaApiError::http(status, "temporary".to_owned()).into()
            ));
        }
        assert!(!is_retryable_scrappa_error(
            &ScrappaApiError::http(422, "invalid".to_owned()).into()
        ));
        assert!(is_per_property_scrappa_error(&ScrappaApiError::http(
            400,
            "Bad request".to_owned()
        )));
        assert!(is_per_property_scrappa_error(&ScrappaApiError::http(
            500,
            " Failed to fetch property details after multiple attempts. ".to_owned()
        )));
        assert!(!is_per_property_scrappa_error(&ScrappaApiError::http(
            500,
            "Server error".to_owned()
        )));
        assert_eq!(get_retry_delay_ms(1, 0), 2000);
        assert_eq!(get_retry_delay_ms(2, 500), 4500);
        assert_eq!(get_retry_delay_ms(20, 0), 10_000);
    }
}
