use anyhow::Result;
use reqwest::{Client, Url};
use serde_json::{Map, Value};
use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const REQUEST_TIMEOUT_MS: u64 = 90_000;
pub const MAX_ATTEMPTS: usize = 3;
const USER_AGENT: &str = "thescrappa-jameda-reviews-scraper/1.0";
const RETRYABLE_STATUS_CODES: [u16; 6] = [408, 429, 500, 502, 503, 504];

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    timeout_ms: u64,
}

impl ScrappaTimeoutError {
    fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }
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

impl Error for ScrappaTimeoutError {}

#[derive(Debug)]
struct ScrappaApiError {
    status: u16,
    message: String,
}

impl fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.message
        )
    }
}

impl Error for ScrappaApiError {}

pub struct ScrappaClient {
    client: Client,
    base_url: String,
    api_key: String,
    timeout_ms: u64,
    attempts: usize,
    retry_delay_scale: f64,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self> {
        Self::with_policy(api_key, base_url, REQUEST_TIMEOUT_MS, MAX_ATTEMPTS, 1.0)
    }

    fn with_policy(
        api_key: String,
        base_url: String,
        timeout_ms: u64,
        attempts: usize,
        retry_delay_scale: f64,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()?;
        Ok(Self {
            client,
            base_url,
            api_key,
            timeout_ms,
            attempts: attempts.max(1),
            retry_delay_scale,
        })
    }

    pub async fn get(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        let attempts = self.attempts;
        for attempt in 1..=attempts {
            match self.send(endpoint, params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt == attempts || !is_retryable_scrappa_error(&error) {
                        return Err(error);
                    }
                    let delay_ms =
                        retry_delay_ms(attempt, random_jitter_ms()) as f64 * self.retry_delay_scale;
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        attempts,
                        delay_ms as u64
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms as u64)).await;
                }
            }
        }
        unreachable!("attempts is always at least one")
    }

    async fn send(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        let mut url = build_scrappa_url(&self.base_url, endpoint)?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                let Some(value) = parameter_string(value) else {
                    continue;
                };
                query.append_pair(key, &value);
            }
        }

        let response = self
            .client
            .get(url.clone())
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(|error| map_request_error(error, self.timeout_ms))?;

        if !response.status().is_success() {
            let status = response.status();
            let fallback = status
                .canonical_reason()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            let message = match response.text().await {
                Ok(body) if !body.is_empty() => format_api_error_message(&body, &fallback),
                Ok(_) => fallback,
                Err(error) if error.is_timeout() => {
                    return Err(ScrappaTimeoutError::new(self.timeout_ms).into());
                }
                Err(_) => fallback,
            };
            return Err(ScrappaApiError {
                status: status.as_u16(),
                message,
            }
            .into());
        }

        response
            .json::<Value>()
            .await
            .map_err(|error| map_request_error(error, self.timeout_ms))
    }
}

pub fn build_scrappa_url(base_url: &str, endpoint: &str) -> Result<Url> {
    let normalized_base_url = format!("{}/", base_url.trim_end_matches('/'));
    let relative_endpoint = endpoint.trim_start_matches('/');
    Ok(Url::parse(&normalized_base_url)?.join(relative_endpoint)?)
}

pub fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponent = failed_attempt.min(63) as u32;
    (1_000_u64
        .saturating_mul(2_u64.saturating_pow(exponent))
        .saturating_add(jitter_ms))
    .min(10_000)
}

pub fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| RETRYABLE_STATUS_CODES.contains(&error.status))
    {
        return true;
    }
    error
        .downcast_ref::<reqwest::Error>()
        .is_some_and(|error| error.is_timeout() || error.is_connect())
}

fn parameter_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(value) if value.is_empty() => None,
        Value::Bool(false) => None,
        Value::Bool(true) => Some("1".to_owned()),
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Array(values) => Some(
            values
                .iter()
                .map(|value| {
                    if value.is_null() {
                        String::new()
                    } else {
                        value.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
        ),
        Value::Object(_) => Some("[object Object]".to_owned()),
    }
}

fn map_request_error(error: reqwest::Error, timeout_ms: u64) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaTimeoutError::new(timeout_ms).into()
    } else {
        error.into()
    }
}

fn format_api_error_message(body: &str, fallback: &str) -> String {
    if let Ok(Value::Object(error_data)) = serde_json::from_str::<Value>(body) {
        let message = error_data
            .get("message")
            .filter(|message| !message.is_null())
            .map(crate::request_params::javascript_string)
            .unwrap_or_else(|| fallback.to_owned());
        let details = error_data
            .get("errors")
            .and_then(Value::as_object)
            .map(|errors| {
                errors
                    .iter()
                    .map(|(field, messages)| {
                        let messages = messages
                            .as_array()
                            .map(|messages| {
                                messages
                                    .iter()
                                    .map(crate::request_params::javascript_string)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default();
                        format!("{field}: {messages}")
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .filter(|details| !details.is_empty());
        return match details {
            Some(details) => format!("{message} - {details}"),
            None => message,
        };
    }
    if serde_json::from_str::<Value>(body).is_ok() {
        return fallback.to_owned();
    }
    let flattened = body.split_whitespace().collect::<Vec<_>>().join(" ");
    flattened
        .chars()
        .take(500)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn random_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64
}

pub fn actor_error_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{error}. The Jameda reviews request exceeded the {}s Scrappa API timeout. Try fewer doctor URLs or run the request again.",
            REQUEST_TIMEOUT_MS / 1000
        )
    } else {
        format!("{error:#}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn calculates_exponential_backoff_with_cap() {
        assert_eq!(retry_delay_ms(1, 0), 2_000);
        assert_eq!(retry_delay_ms(2, 250), 4_250);
        assert_eq!(retry_delay_ms(3, 999), 8_999);
        assert_eq!(retry_delay_ms(4, 999), 10_000);
    }

    #[test]
    fn retries_timeout_transient_api_and_transport_errors() {
        assert!(is_retryable_scrappa_error(
            &ScrappaTimeoutError::new(1_000).into()
        ));
        assert!(is_retryable_scrappa_error(
            &ScrappaApiError {
                status: 429,
                message: "Too many requests".to_owned()
            }
            .into()
        ));
        assert!(is_retryable_scrappa_error(
            &ScrappaApiError {
                status: 503,
                message: "Service unavailable".to_owned()
            }
            .into()
        ));
        assert!(!is_retryable_scrappa_error(
            &ScrappaApiError {
                status: 400,
                message: "Bad request".to_owned()
            }
            .into()
        ));
        assert!(!is_retryable_scrappa_error(&anyhow::anyhow!(
            "market must be one of: DEU"
        )));
    }

    #[test]
    fn joins_base_urls_without_dropping_path_prefixes() {
        assert_eq!(
            build_scrappa_url("https://scrappa.co/api", "/jameda/reviews")
                .unwrap()
                .as_str(),
            "https://scrappa.co/api/jameda/reviews"
        );
        assert_eq!(
            build_scrappa_url("https://example.test/custom/api/", "jameda/reviews")
                .unwrap()
                .as_str(),
            "https://example.test/custom/api/jameda/reviews"
        );
    }

    #[test]
    fn formats_structured_and_plain_text_scrappa_errors() {
        assert_eq!(
            format_api_error_message(
                r#"{"message":"Invalid input","errors":{"doctor_url":["required","invalid"]}}"#,
                "Bad Request"
            ),
            "Invalid input - doctor_url: required, invalid"
        );
        let long = format_api_error_message(&"x".repeat(700), "Bad Gateway");
        assert_eq!(long.chars().count(), 500);
        assert_eq!(
            format_api_error_message("null", "Bad Request"),
            "Bad Request"
        );
    }

    #[test]
    fn turns_timeouts_into_the_actor_specific_deadline_message() {
        let error: anyhow::Error = ScrappaTimeoutError::new(90_000).into();
        assert_eq!(
            actor_error_message(&error),
            "Scrappa API request timed out after 90000ms. The Jameda reviews request exceeded the 90s Scrappa API timeout. Try fewer doctor URLs or run the request again."
        );
        assert_eq!(parameter_string(&json!(true)).as_deref(), Some("1"));
        assert_eq!(parameter_string(&json!(false)), None);
    }
}
