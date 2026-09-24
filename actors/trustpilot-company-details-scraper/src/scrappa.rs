use crate::request_params::RequestParams;
use anyhow::{anyhow, Result};
use reqwest::{Client, Response, StatusCode, Url};
use serde_json::Value;
use std::{error::Error, fmt, time::Duration};
use tokio::time::sleep;

pub const REQUEST_TIMEOUT_MS: u64 = 90_000;
pub const MAX_ATTEMPTS: usize = 3;
const DEFAULT_BASE_URL: &str = "https://scrappa.co/api";
const USER_AGENT: &str = "thescrappa-trustpilot-company-details-scraper/1.0";

#[derive(Debug)]
pub struct ScrappaTimeoutError;

impl fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {REQUEST_TIMEOUT_MS}ms"
        )
    }
}

impl Error for ScrappaTimeoutError {}

pub struct ScrappaClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: Option<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(REQUEST_TIMEOUT_MS))
            .build()
            .map_err(|error| anyhow!("Could not create Scrappa HTTP client: {error}"))?;
        Ok(Self {
            client,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.into()),
            api_key,
        })
    }

    pub async fn get(
        &self,
        endpoint: &str,
        params: &RequestParams,
        attempts: usize,
    ) -> Result<Value> {
        let attempts = attempts.max(1);
        let mut last_error = None;

        for attempt in 1..=attempts {
            match self.send(endpoint, params).await {
                Ok(response) if response.status().is_success() => match response.json().await {
                    Ok(response) => return Ok(response),
                    Err(error) => {
                        let error = map_request_error(error, "Failed to read Scrappa API response");
                        if attempt < attempts && is_retryable_error(&error) {
                            last_error = Some(error);
                            self.wait_before_retry(attempt, attempts, last_error.as_ref().unwrap())
                                .await;
                            continue;
                        }
                        return Err(error);
                    }
                },
                Ok(response) => {
                    let status = response.status();
                    let body = match response.text().await {
                        Ok(body) => body,
                        Err(error) => {
                            let error = map_request_error(
                                error,
                                "Failed to read Scrappa API error response",
                            );
                            if attempt < attempts && is_retryable_error(&error) {
                                last_error = Some(error);
                                self.wait_before_retry(
                                    attempt,
                                    attempts,
                                    last_error.as_ref().unwrap(),
                                )
                                .await;
                                continue;
                            }
                            return Err(error);
                        }
                    };
                    let error = anyhow!(format_api_error(status, &body));
                    if attempt >= attempts || !retryable_status(status) {
                        return Err(error);
                    }
                    last_error = Some(error);
                    self.wait_before_retry(attempt, attempts, last_error.as_ref().unwrap())
                        .await;
                }
                Err(error) => {
                    if attempt >= attempts || !is_retryable_error(&error) {
                        return Err(error);
                    }
                    last_error = Some(error);
                    self.wait_before_retry(attempt, attempts, last_error.as_ref().unwrap())
                        .await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Scrappa API request failed")))
    }

    async fn send(&self, endpoint: &str, params: &RequestParams) -> Result<Response> {
        let mut url = Url::parse(&format!("{}{endpoint}", self.base_url))
            .map_err(|error| anyhow!("Invalid Scrappa API URL: {error}"))?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                if value.is_null() || value.as_str() == Some("") {
                    continue;
                }
                if let Value::Bool(value) = value {
                    if *value {
                        query.append_pair(key, "1");
                    }
                    continue;
                }
                query.append_pair(key, &javascript_string(value));
            }
        }

        self.client
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(|error| map_request_error(error, "Scrappa API request failed"))
    }

    async fn wait_before_retry(
        &self,
        failed_attempt: usize,
        attempts: usize,
        error: &anyhow::Error,
    ) {
        let jitter_ms = SystemTimeMillis::jitter_ms();
        let delay_ms = retry_delay_ms(failed_attempt, jitter_ms);
        eprintln!(
            "Scrappa API request failed ({error}). Retrying attempt {}/{attempts} in {delay_ms}ms.",
            failed_attempt + 1
        );
        sleep(Duration::from_millis(delay_ms)).await;
    }
}

fn map_request_error(error: reqwest::Error, context: &str) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaTimeoutError.into()
    } else {
        anyhow::Error::from(error).context(context.to_owned())
    }
}

fn is_retryable_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    error.chain().any(|cause| {
        let message = cause.to_string().to_ascii_lowercase();
        message.contains("connection reset")
            || message.contains("connection refused")
            || message.contains("timed out")
            || message.contains("dns")
            || message.contains("name or service not known")
            || message.contains("temporary failure in name resolution")
            || message.contains("error sending request")
            || message.contains("error trying to connect")
    })
}

fn retryable_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
}

fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponent = u32::try_from(failed_attempt).unwrap_or(u32::MAX);
    1_000_u64
        .saturating_mul(2_u64.saturating_pow(exponent))
        .saturating_add(jitter_ms)
        .min(10_000)
}

struct SystemTimeMillis;

impl SystemTimeMillis {
    fn jitter_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|time| u64::from(time.subsec_millis()))
            .unwrap_or_default()
    }
}

fn javascript_string(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(value) => {
            if *value {
                "1".into()
            } else {
                String::new()
            }
        }
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    javascript_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn javascript_display(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) => value
            .as_array()
            .unwrap()
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    javascript_display(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn format_api_error(status: StatusCode, body: &str) -> String {
    let status_code = status.as_u16();
    let fallback = status.canonical_reason().unwrap_or("Unknown status");
    let message = match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(error_data)) => {
            let mut message = error_data
                .get("message")
                .filter(|value| !value.is_null())
                .map(javascript_display)
                .unwrap_or_else(|| fallback.to_owned());
            if let Some(Value::Object(errors)) = error_data
                .get("errors")
                .filter(|value| javascript_truthy(value))
            {
                let details = errors
                    .iter()
                    .map(|(field, messages)| {
                        let messages = messages
                            .as_array()
                            .map(|messages| {
                                messages
                                    .iter()
                                    .map(javascript_display)
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
            message
        }
        Ok(_) => fallback.to_owned(),
        Err(_) if body.is_empty() => fallback.to_owned(),
        Err(_) => body
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(500)
            .collect(),
    };
    format!("Scrappa API error ({status_code}): {message}")
}

fn javascript_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_the_upstream_exponential_retry_delay() {
        assert_eq!(retry_delay_ms(1, 0), 2_000);
        assert_eq!(retry_delay_ms(2, 250), 4_250);
        assert_eq!(retry_delay_ms(3, 999), 8_999);
        assert_eq!(retry_delay_ms(4, 999), 10_000);
    }

    #[test]
    fn only_transient_scrappa_statuses_are_retryable() {
        for code in [408, 429, 500, 502, 503, 504] {
            assert!(retryable_status(StatusCode::from_u16(code).unwrap()));
        }
        for code in [400, 401, 404] {
            assert!(!retryable_status(StatusCode::from_u16(code).unwrap()));
        }
    }

    #[test]
    fn formats_json_and_bounded_plain_text_errors() {
        assert_eq!(
            format_api_error(
                StatusCode::BAD_REQUEST,
                r#"{"message":"Validation failed","errors":{"locale":["is invalid"]}}"#
            ),
            "Scrappa API error (400): Validation failed - locale: is invalid"
        );
        let plain = format_api_error(
            StatusCode::BAD_GATEWAY,
            &format!("{}  \n", "x".repeat(5_000)),
        );
        assert!(plain.starts_with("Scrappa API error (502): "));
        assert_eq!(
            plain.chars().count(),
            "Scrappa API error (502): ".len() + 500
        );
    }

    #[test]
    fn serializes_only_supported_query_value_shapes_like_url_search_params() {
        assert_eq!(javascript_string(&Value::Bool(true)), "1");
        assert_eq!(javascript_string(&Value::Bool(false)), "");
        assert_eq!(
            javascript_string(&serde_json::json!(["a", null, "b"])),
            "a,,b"
        );
        assert_eq!(
            javascript_string(&serde_json::json!({"a":1})),
            "[object Object]"
        );
    }
}
