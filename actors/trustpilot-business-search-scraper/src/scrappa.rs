use anyhow::{Error, Result, anyhow};
use reqwest::{Client, Response, Url};
use serde_json::{Map, Value};
use std::{
    error::Error as StdError,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const REQUEST_TIMEOUT_MS: u64 = 90_000;
const MAX_ATTEMPTS: usize = 3;
const USER_AGENT: &str = "thescrappa-trustpilot-business-search-scraper/1.0";

pub struct ScrappaClient {
    client: Client,
    base_url: Url,
    api_key: String,
}

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    timeout_ms: u64,
}

impl fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            self.timeout_ms
        )
    }
}

impl StdError for ScrappaTimeoutError {}

impl ScrappaTimeoutError {
    pub fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: &str) -> Result<Self> {
        let base_url = Url::parse(base_url)
            .map_err(|_| anyhow!("SCRAPPA_API_BASE_URL must be a valid URL"))?;
        let client = Client::builder()
            .timeout(Duration::from_millis(REQUEST_TIMEOUT_MS))
            .build()
            .map_err(|error| anyhow!("Could not create Scrappa HTTP client: {error}"))?;
        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }

    pub async fn get(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        for attempt in 1..=MAX_ATTEMPTS {
            match self.send(endpoint, params).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < MAX_ATTEMPTS && is_retryable_error(&error) => {
                    let delay = retry_delay(attempt, jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{MAX_ATTEMPTS} in {}ms.",
                        error,
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("the configured retry loop always returns a response or error")
    }

    async fn send(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        let mut url = self.base_url.clone();
        url.set_path(&format!("{}{}", url.path().trim_end_matches('/'), endpoint));
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                if value.is_null() || value.as_str() == Some("") {
                    continue;
                }
                if let Some(boolean) = value.as_bool() {
                    if boolean {
                        query.append_pair(key, "1");
                    }
                    continue;
                }
                query.append_pair(key, &js_value_string(value));
            }
        }

        let response = self
            .client
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(transport_error)?;
        if !response.status().is_success() {
            let status = response.status();
            let message = response_error_message(response).await?;
            return Err(anyhow!(
                "Scrappa API error ({}): {message}",
                status.as_u16()
            ));
        }
        response.json().await.map_err(transport_error)
    }
}

fn transport_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::new(ScrappaTimeoutError::new(REQUEST_TIMEOUT_MS))
    } else {
        Error::new(error)
    }
}

async fn response_error_message(response: Response) -> Result<String> {
    let fallback = response
        .status()
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", response.status().as_u16()));
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) if error.is_timeout() => return Err(transport_error(error)),
        Err(_) => return Ok(fallback),
    };
    Ok(format_error_body(&body, &fallback))
}

fn format_error_body(body: &str, fallback: &str) -> String {
    if body.is_empty() {
        return fallback.into();
    }
    if let Ok(data) = serde_json::from_str::<Value>(&body) {
        let mut message = data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(&fallback)
            .to_owned();
        if let Some(errors) = data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    messages.as_array().map(|messages| {
                        format!(
                            "{field}: {}",
                            messages
                                .iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
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
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn js_value_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => {
            let Some(number) = value.as_f64() else {
                return value.to_string();
            };
            if number.fract() == 0.0 && number.abs() < 1e21 {
                format!("{number:.0}")
            } else {
                number.to_string()
            }
        }
        Value::Bool(value) => value.to_string(),
        Value::Array(values) => values
            .iter()
            .map(js_value_string)
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
        Value::Null => "null".into(),
    }
}

fn jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
        % 1_000
}

fn retry_delay(failed_attempt: usize, jitter_ms: u64) -> Duration {
    Duration::from_millis((1_000_u64.saturating_mul(1 << failed_attempt) + jitter_ms).min(10_000))
}

fn is_retryable_error(error: &Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if let Some(error) = error.downcast_ref::<reqwest::Error>() {
        if error.is_timeout() || error.is_connect() {
            return true;
        }
        let message = error.to_string();
        return [
            "ECONNRESET",
            "ECONNREFUSED",
            "ETIMEDOUT",
            "ENOTFOUND",
            "EAI_AGAIN",
        ]
        .iter()
        .any(|code| message.contains(code));
    }
    error.to_string().contains("Scrappa API error (")
        && [408, 429, 500, 502, 503, 504].iter().any(|status| {
            error
                .to_string()
                .starts_with(&format!("Scrappa API error ({status})"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_exponential_retry_delay_with_jitter_and_cap() {
        assert_eq!(retry_delay(1, 0).as_millis(), 2_000);
        assert_eq!(retry_delay(2, 250).as_millis(), 4_250);
        assert_eq!(retry_delay(3, 999).as_millis(), 8_999);
        assert_eq!(retry_delay(4, 999).as_millis(), 10_000);
    }

    #[test]
    fn formats_numeric_query_values_like_javascript_numbers() {
        assert_eq!(js_value_string(&serde_json::json!(3.0)), "3");
        assert_eq!(js_value_string(&serde_json::json!(4.5)), "4.5");
    }

    #[test]
    fn retries_only_transient_scrappa_statuses_and_transport_errors() {
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_error(&anyhow!(
                "Scrappa API error ({status}): temporary"
            )));
        }
        assert!(!is_retryable_error(&anyhow!(
            "Scrappa API error (400): invalid input"
        )));
        assert!(!is_retryable_error(&anyhow!("market must be one of: DEU")));
    }

    #[test]
    fn keeps_error_json_details_and_truncates_plain_text() {
        assert_eq!(
            format_error_body(
                r#"{"message":"Invalid input","errors":{"query":["is required","is too short"]}}"#,
                "Bad Request"
            ),
            "Invalid input - query: is required, is too short"
        );
        assert_eq!(
            format_error_body("too   many\nrequests", "Bad Request"),
            "too many requests"
        );
        assert_eq!(format_error_body("", "Bad Request"), "Bad Request");
        assert_eq!(
            format_error_body(&"x".repeat(510), "Bad Request").len(),
            500
        );
    }
}
