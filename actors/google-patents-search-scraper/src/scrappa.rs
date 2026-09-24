use anyhow::{anyhow, Result};
use rand::random;
use reqwest::{header, Client, StatusCode};
use serde_json::{Map, Value};
use std::time::Duration;
use tokio::time::sleep;
use url::Url;

use crate::request_params::js_string;

pub const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_REQUEST_ATTEMPTS: u8 = 3;
const RETRYABLE_STATUS_CODES: [u16; 6] = [408, 429, 500, 502, 503, 504];

#[derive(Debug)]
enum ScrappaFailure {
    Timeout,
    Http { status: StatusCode, message: String },
    Transport(String),
    InvalidJson(String),
}

impl ScrappaFailure {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout => true,
            Self::Http { status, .. } => RETRYABLE_STATUS_CODES.contains(&status.as_u16()),
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
            Self::Http { status, message } => write!(
                formatter,
                "Scrappa API error ({}): {message}",
                status.as_u16()
            ),
            Self::Transport(message) => write!(formatter, "{message}"),
            Self::InvalidJson(message) => write!(
                formatter,
                "Scrappa API response was not valid JSON: {message}"
            ),
        }
    }
}

pub async fn search(
    client: &Client,
    api_base_url: &Url,
    api_key: &str,
    params: &Map<String, Value>,
) -> Result<Value> {
    let url = search_url(api_base_url, params)?;
    for attempt in 1..=SCRAPPA_REQUEST_ATTEMPTS {
        match fetch_once(client, &url, api_key).await {
            Ok(response) => return Ok(response),
            Err(error) if error.is_retryable() && attempt < SCRAPPA_REQUEST_ATTEMPTS => {
                let delay_ms = retry_delay_ms(attempt, random::<f64>());
                eprintln!(
                    "Scrappa API request failed ({error}). Retrying attempt {}/{SCRAPPA_REQUEST_ATTEMPTS} in {delay_ms}ms.",
                    attempt + 1
                );
                sleep(Duration::from_millis(delay_ms)).await;
            }
            Err(error) => return Err(anyhow!(error.to_string())),
        }
    }
    unreachable!("the retry loop always returns or continues")
}

fn search_url(api_base_url: &Url, params: &Map<String, Value>) -> Result<Url> {
    let mut url = api_base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("Scrappa API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(["google-patents", "search"]);
    for (key, value) in params {
        if !value.is_null() && value.as_str() != Some("") {
            url.query_pairs_mut().append_pair(key, &js_string(value));
        }
    }
    Ok(url)
}

async fn fetch_once(
    client: &Client,
    url: &Url,
    api_key: &str,
) -> std::result::Result<Value, ScrappaFailure> {
    let response = client
        .get(url.clone())
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .header("X-API-Key", api_key)
        .header(header::ACCEPT, "application/json")
        .header(
            header::USER_AGENT,
            "thescrappa-google-patents-search-scraper/1.0",
        )
        .send()
        .await
        .map_err(scrappa_transport_error)?;
    let status = response.status();
    let body = response.text().await.map_err(scrappa_transport_error)?;
    if !status.is_success() {
        return Err(ScrappaFailure::Http {
            status,
            message: read_error_message(status, &body),
        });
    }
    serde_json::from_str(&body).map_err(|error| ScrappaFailure::InvalidJson(error.to_string()))
}

fn scrappa_transport_error(error: reqwest::Error) -> ScrappaFailure {
    if error.is_timeout() {
        ScrappaFailure::Timeout
    } else {
        ScrappaFailure::Transport(error.to_string())
    }
}

fn read_error_message(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    if body.is_empty() {
        return fallback;
    }

    if let Ok(Value::Object(data)) = serde_json::from_str::<Value>(body) {
        let mut message = data
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or(fallback);
        if let Some(Value::Object(errors)) = data.get("errors").filter(|value| is_truthy(value)) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let values = messages
                        .as_array()
                        .map(|messages| messages.iter().map(js_string).collect::<Vec<_>>())
                        .unwrap_or_default();
                    format!("{field}: {}", values.join(", "))
                })
                .collect::<Vec<_>>();
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details.join("; "));
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

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn retry_delay_ms(failed_attempt: u8, random_value: f64) -> u64 {
    let base_delay = 1_000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt.into()));
    let jitter = (random_value.clamp(0.0, 0.999_999_999) * 1_000.0) as u64;
    base_delay.saturating_add(jitter).min(10_000)
}

pub fn actor_error_message(message: &str) -> String {
    if message.contains("Scrappa API request timed out after 60000ms") {
        format!(
            "{message}. The Google Patents request exceeded the {}s Scrappa API timeout. Try a more specific query or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn retries_only_transient_http_errors_and_timeouts() {
        assert!(ScrappaFailure::Timeout.is_retryable());
        for status in RETRYABLE_STATUS_CODES {
            assert!(ScrappaFailure::Http {
                status: StatusCode::from_u16(status).unwrap(),
                message: String::new(),
            }
            .is_retryable());
        }
        for status in [400, 401, 403, 404, 422] {
            assert!(!ScrappaFailure::Http {
                status: StatusCode::from_u16(status).unwrap(),
                message: String::new(),
            }
            .is_retryable());
        }
        assert!(!ScrappaFailure::Transport("fetch failed".to_owned()).is_retryable());
        assert!(!ScrappaFailure::InvalidJson("bad response".to_owned()).is_retryable());
    }

    #[test]
    fn preserves_status_error_messages_and_timeout_guidance() {
        let message = read_error_message(
            StatusCode::UNPROCESSABLE_ENTITY,
            r#"{"message":"Invalid query","errors":{"q":["required","too short"],"page":["too high"]}}"#,
        );
        assert_eq!(
            message,
            "Invalid query - q: required, too short; page: too high"
        );
        assert_eq!(
            read_error_message(StatusCode::BAD_REQUEST, "  plain\n text  "),
            "plain text"
        );
        assert_eq!(
            actor_error_message("Scrappa API request timed out after 60000ms"),
            "Scrappa API request timed out after 60000ms. The Google Patents request exceeded the 60s Scrappa API timeout. Try a more specific query or run the request again."
        );
    }

    #[test]
    fn constructs_upstream_url_with_encoded_filters() {
        let params =
            serde_json::from_value(json!({"q":"battery charging", "page":2, "status":"GRANT"}))
                .unwrap();
        let url = search_url(&Url::parse("https://scrappa.co/api").unwrap(), &params).unwrap();
        assert_eq!(
            url.as_str(),
            "https://scrappa.co/api/google-patents/search?q=battery+charging&page=2&status=GRANT"
        );
    }

    #[test]
    fn uses_the_same_exponential_retry_window_and_jitter() {
        assert_eq!(retry_delay_ms(1, 0.0), 2_000);
        assert_eq!(retry_delay_ms(1, 0.5), 2_500);
        assert_eq!(retry_delay_ms(2, 0.999), 4_999);
        assert_eq!(retry_delay_ms(10, 0.999), 10_000);
    }
}
