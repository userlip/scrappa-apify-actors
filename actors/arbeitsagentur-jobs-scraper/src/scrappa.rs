use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use reqwest::{header, Client, StatusCode};
use serde_json::{Map, Value};
use url::Url;

use super::endpoint_url;

pub(super) const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
pub(super) const SCRAPPA_USER_AGENT: &str = "thescrappa-arbeitsagentur-jobs-scraper/1.0";
pub(super) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const SCRAPPA_MAX_ATTEMPTS: usize = 4;
const SCRAPPA_MAX_RETRY_DELAY: Duration = Duration::from_secs(20);
pub(super) const SCRAPPA_REQUEST_DEADLINE: Duration = Duration::from_secs(180);

pub(super) struct ScrappaClient {
    pub(super) http: Client,
    pub(super) base_url: String,
    pub(super) api_key: String,
    pub(super) policy: ScrappaRetryPolicy,
}

#[derive(Clone, Copy)]
pub(super) struct ScrappaRetryPolicy {
    pub(super) attempts: usize,
    pub(super) request_timeout: Duration,
    pub(super) max_retry_delay: Duration,
    pub(super) request_deadline: Duration,
}

impl ScrappaRetryPolicy {
    fn production() -> Self {
        Self {
            attempts: SCRAPPA_MAX_ATTEMPTS,
            request_timeout: SCRAPPA_REQUEST_TIMEOUT,
            max_retry_delay: SCRAPPA_MAX_RETRY_DELAY,
            request_deadline: SCRAPPA_REQUEST_DEADLINE,
        }
    }
}

impl ScrappaClient {
    pub(super) fn new(http: Client, base_url: String, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
            policy: ScrappaRetryPolicy::production(),
        }
    }

    pub(super) async fn get_jobs(&self, params: &Map<String, Value>) -> Result<Value> {
        let url = build_jobs_url(&self.base_url, params)?;
        let deadline = Instant::now() + self.policy.request_deadline;

        for attempt in 1..=self.policy.attempts {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(ScrappaFailure::Timeout.into());
            }
            let timeout = self.policy.request_timeout.min(remaining);
            let result = self.send(&url, timeout).await;
            match result {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt == self.policy.attempts || !error.is_retryable() {
                        return Err(error.into());
                    }

                    let retry_after = match &error {
                        ScrappaFailure::Api {
                            status,
                            retry_after_ms,
                            ..
                        } => retry_after_ms.or_else(|| (*status == 503).then_some(20_000)),
                        _ => None,
                    };
                    let delay_ms = get_retry_delay_ms(
                        attempt,
                        retry_jitter_ms(),
                        retry_after,
                        duration_millis(self.policy.max_retry_delay),
                    );
                    let delay = Duration::from_millis(delay_ms);
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if delay >= remaining {
                        return Err(ScrappaFailure::Timeout.into());
                    }
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{}, in {delay_ms}ms.",
                        attempt + 1,
                        self.policy.attempts
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }

        unreachable!("the retry loop returns after its configured attempts")
    }

    async fn send(
        &self,
        url: &Url,
        timeout: Duration,
    ) -> std::result::Result<Value, ScrappaFailure> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, SCRAPPA_USER_AGENT)
            .timeout(timeout)
            .send()
            .await
            .map_err(ScrappaFailure::from_reqwest)?;

        let status = response.status();
        let retry_after_ms = response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(parse_retry_after_ms);
        let body = response
            .bytes()
            .await
            .map_err(ScrappaFailure::from_reqwest)?;

        if !status.is_success() {
            return Err(ScrappaFailure::Api {
                status: status.as_u16(),
                message: scrappa_error_message(status, &body),
                retry_after_ms,
            });
        }

        serde_json::from_slice(&body)
            .map_err(|error| ScrappaFailure::InvalidJson(error.to_string()))
    }
}

#[derive(Debug)]
pub(super) enum ScrappaFailure {
    Timeout,
    Api {
        status: u16,
        message: String,
        retry_after_ms: Option<u64>,
    },
    Transport(String),
    InvalidJson(String),
}

impl ScrappaFailure {
    fn from_reqwest(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::Timeout
        } else {
            Self::Transport(error.to_string())
        }
    }

    pub(super) fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout => true,
            Self::Api { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::Transport(_) | Self::InvalidJson(_) => false,
        }
    }
}

impl std::fmt::Display for ScrappaFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(
                formatter,
                "Scrappa API request timed out after {}ms",
                SCRAPPA_REQUEST_TIMEOUT.as_millis()
            ),
            Self::Api {
                status, message, ..
            } => write!(formatter, "Scrappa API error ({status}): {message}"),
            Self::Transport(message) | Self::InvalidJson(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ScrappaFailure {}

pub(super) fn build_jobs_url(base_url: &str, params: &Map<String, Value>) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["arbeitsagentur", "jobs"])?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            if value.is_null() || value.as_str().is_some_and(str::is_empty) {
                continue;
            }
            query.append_pair(key, &query_value(value));
        }
    }
    Ok(url)
}

pub(super) fn query_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
        Value::Null => String::new(),
    }
}

pub(super) fn get_retry_delay_ms(
    failed_attempt: usize,
    jitter_ms: u64,
    retry_after_ms: Option<u64>,
    max_retry_delay_ms: u64,
) -> u64 {
    let exponential_delay_ms = 1_000_u64
        .saturating_mul(2_u64.saturating_pow(failed_attempt.min(63) as u32))
        .saturating_add(jitter_ms);
    exponential_delay_ms
        .max(retry_after_ms.unwrap_or_default())
        .min(max_retry_delay_ms)
}

fn retry_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| (duration.subsec_nanos() / 1_000_000) as u64)
        .unwrap_or_default()
}

pub(super) fn parse_retry_after_ms(value: &str) -> Option<u64> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<f64>() {
        if seconds.is_finite() && seconds >= 0.0 {
            return Some((seconds * 1_000.0) as u64);
        }
    }

    let retry_at = httpdate::parse_http_date(value).ok()?;
    Some(
        retry_at
            .duration_since(SystemTime::now())
            .unwrap_or_default()
            .as_millis()
            .min(u64::MAX as u128) as u64,
    )
}

pub(super) fn duration_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

pub(super) fn scrappa_error_message(status: StatusCode, body: &[u8]) -> String {
    let fallback = status.canonical_reason().unwrap_or("HTTP error");
    let body = String::from_utf8_lossy(body);
    if body.is_empty() {
        return fallback.to_owned();
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
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

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    js_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}
