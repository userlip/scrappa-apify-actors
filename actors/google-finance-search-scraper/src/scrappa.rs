use std::{fmt, time::Duration};

use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client};
use serde_json::Value;
use tokio::time::{sleep, timeout};

use crate::config::{endpoint_url, Config};
use crate::input::GoogleFinanceSearchRequest;

const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const SCRAPPA_MAX_ATTEMPTS: u32 = 3;
const SCRAPPA_USER_AGENT: &str = "thescrappa-google-finance-search-scraper/1.0";

#[derive(Debug)]
pub(crate) enum ScrappaFailure {
    Timeout,
    Http { status: u16, details: String },
    Network(String),
}

impl fmt::Display for ScrappaFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(formatter, "Scrappa API request timed out after 30000ms"),
            Self::Http { status, details } => {
                write!(formatter, "Scrappa API error ({status}): {details}")
            }
            Self::Network(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ScrappaFailure {}

impl ScrappaFailure {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout | Self::Network(_) => true,
            Self::Http { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
        }
    }
}

pub(crate) struct ScrappaClient<'a> {
    pub(crate) http: &'a Client,
    pub(crate) config: &'a Config,
}

impl ScrappaClient<'_> {
    pub(crate) async fn get_search(&self, params: &GoogleFinanceSearchRequest) -> Result<Value> {
        let mut last_error = None;
        for failed_attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send_search(params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let retryable = is_retryable_scrappa_error(&error);
                    if !retryable || failed_attempt == SCRAPPA_MAX_ATTEMPTS {
                        return Err(error);
                    }
                    let delay_ms = get_retry_delay_ms(failed_attempt, retry_jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        failed_attempt + 1,
                        SCRAPPA_MAX_ATTEMPTS,
                        delay_ms
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                    last_error = Some(error);
                }
            }
        }
        Err(last_error.expect("at least one Scrappa attempt is made"))
    }

    async fn send_search(&self, params: &GoogleFinanceSearchRequest) -> Result<Value> {
        let operation = async {
            let mut url =
                endpoint_url(&self.config.scrappa_api_base, &["google-finance", "search"])?;
            {
                let mut query = url.query_pairs_mut();
                query.append_pair("q", &params.q);
                if let Some(hl) = &params.hl {
                    query.append_pair("hl", hl);
                }
                if let Some(gl) = &params.gl {
                    query.append_pair("gl", gl);
                }
            }

            let response = self
                .http
                .get(url)
                .timeout(SCRAPPA_REQUEST_TIMEOUT)
                .header("X-API-Key", &self.config.scrappa_api_key)
                .header(header::ACCEPT, "application/json")
                .header(header::USER_AGENT, SCRAPPA_USER_AGENT)
                .send()
                .await
                .map_err(scrappa_transport_error)?;

            let status = response.status();
            if !status.is_success() {
                let fallback = status.canonical_reason().unwrap_or("HTTP error");
                let body = match response.text().await {
                    Ok(body) => body,
                    Err(error) if error.is_timeout() => {
                        return Err(anyhow!(ScrappaFailure::Timeout));
                    }
                    Err(_) => String::new(),
                };
                let details = parse_scrappa_error_body(&body, fallback);
                return Err(anyhow!(ScrappaFailure::Http {
                    status: status.as_u16(),
                    details,
                }));
            }

            let body = response.text().await.map_err(scrappa_transport_error)?;
            serde_json::from_str(&body).context("Scrappa API response was not valid JSON")
        };

        match timeout(SCRAPPA_REQUEST_TIMEOUT, operation).await {
            Ok(result) => result,
            Err(_) => Err(anyhow!(ScrappaFailure::Timeout)),
        }
    }
}

fn scrappa_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        return anyhow!(ScrappaFailure::Timeout);
    }
    if error.is_connect() || looks_like_network_error(&error.to_string()) {
        return anyhow!(ScrappaFailure::Network(error.to_string()));
    }
    anyhow::Error::new(error)
}

pub(crate) fn looks_like_network_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    [
        "fetch failed",
        "failed to fetch",
        "network",
        "terminated",
        "reset",
        "econnrefused",
        "econnreset",
        "socket hang up",
        "chunk",
    ]
    .iter()
    .any(|fragment| message.contains(fragment))
}

pub(crate) fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaFailure>()
        .is_some_and(ScrappaFailure::is_retryable)
}

pub(crate) fn describe_transient_failure(error: &anyhow::Error) -> String {
    match error.downcast_ref::<ScrappaFailure>() {
        Some(ScrappaFailure::Http { status, .. }) => {
            format!("Scrappa upstream returned {status} after retries")
        }
        Some(ScrappaFailure::Timeout) => "Scrappa API request timed out after 30000ms".to_owned(),
        Some(ScrappaFailure::Network(message)) => message.clone(),
        None => error.to_string(),
    }
}

pub(crate) fn get_retry_delay_ms(failed_attempt: u32, jitter_ms: u64) -> u64 {
    (1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt)) + jitter_ms).min(10_000)
}

fn retry_jitter_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::from(duration.subsec_micros() % 1000))
        .unwrap_or(0)
}

pub(crate) fn parse_scrappa_error_body(body: &str, fallback: &str) -> String {
    if let Ok(data) = serde_json::from_str::<Value>(body) {
        if let Some(object) = data.as_object() {
            let mut message = object
                .get("message")
                .filter(|value| !value.is_null())
                .map(js_string)
                .unwrap_or_else(|| fallback.to_owned());
            if let Some(errors) = object.get("errors").and_then(Value::as_object) {
                let details = errors
                    .iter()
                    .filter_map(|(field, messages)| {
                        let messages = messages.as_array()?;
                        Some(format!(
                            "{field}: {}",
                            messages
                                .iter()
                                .map(js_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
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
    if body.is_empty() {
        return fallback.to_owned();
    }
    collapse_whitespace(body, 500)
}

fn collapse_whitespace(value: &str, max_utf16_units: usize) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut result = String::new();
    let mut units = 0;
    for character in collapsed.chars() {
        let width = character.len_utf16();
        if units + width > max_utf16_units {
            break;
        }
        result.push(character);
        units += width;
    }
    result
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
