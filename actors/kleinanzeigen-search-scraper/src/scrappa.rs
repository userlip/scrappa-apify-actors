use std::{
    error::Error as StdError,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use reqwest::{header, Client, StatusCode};
use serde_json::{Map, Value};

use crate::config::{endpoint_url, Config};
use crate::input::{js_string, js_truthy};

pub(crate) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const SCRAPPA_MAX_ATTEMPTS: usize = 3;

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

impl StdError for ScrappaApiError {}

#[derive(Debug)]
pub(crate) struct ScrappaTimeoutError {
    pub(crate) timeout: Duration,
}

impl fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            self.timeout.as_millis()
        )
    }
}

impl StdError for ScrappaTimeoutError {}

fn scrub_body(body: &str) -> String {
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn scrappa_error(status: u16, body: &str) -> ScrappaApiError {
    let fallback = StatusCode::from_u16(status)
        .ok()
        .and_then(|status| status.canonical_reason())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {status}"));
    let parsed = match serde_json::from_str::<Value>(body) {
        Ok(parsed) => parsed,
        Err(_) => {
            return ScrappaApiError {
                status,
                message: if body.is_empty() {
                    fallback
                } else {
                    scrub_body(body)
                },
            };
        }
    };
    let Some(object) = parsed.as_object() else {
        return ScrappaApiError {
            status,
            message: if parsed.is_null() {
                scrub_body(body)
            } else {
                fallback
            },
        };
    };

    let mut message = object
        .get("message")
        .filter(|value| !value.is_null())
        .map(js_string)
        .unwrap_or(fallback);
    if let Some(errors) = object.get("errors").filter(|value| js_truthy(value)) {
        let Some(errors) = errors.as_object() else {
            return ScrappaApiError {
                status,
                message: scrub_body(body),
            };
        };
        let mut details = Vec::with_capacity(errors.len());
        for (field, messages) in errors {
            let Some(messages) = messages.as_array() else {
                return ScrappaApiError {
                    status,
                    message: scrub_body(body),
                };
            };
            details.push(format!(
                "{field}: {}",
                messages
                    .iter()
                    .map(js_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details.join("; "));
        }
    }
    ScrappaApiError { status, message }
}

pub(crate) fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    for cause in error.chain() {
        if let Some(api_error) = cause.downcast_ref::<ScrappaApiError>() {
            return matches!(api_error.status, 408 | 429 | 500 | 502 | 503 | 504);
        }
        if cause.downcast_ref::<ScrappaTimeoutError>().is_some() {
            return true;
        }
        if let Some(request_error) = cause.downcast_ref::<reqwest::Error>() {
            return request_error.is_timeout() || request_error.is_connect();
        }
    }
    false
}

fn retry_delay(attempt: usize, jitter_ms: u64) -> Duration {
    let base_ms = 1_000_u64.saturating_mul(2_u64.saturating_pow(attempt as u32));
    Duration::from_millis(base_ms.saturating_add(jitter_ms).min(10_000))
}

fn retry_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::from(duration.subsec_nanos() % 1_000_000_000) / 1_000_000)
        .unwrap_or(0)
}

pub(crate) struct ScrappaClient<'a> {
    http: &'a Client,
    config: &'a Config,
    api_key: &'a str,
    timeout: Duration,
}

impl<'a> ScrappaClient<'a> {
    pub(super) fn new(http: &'a Client, config: &'a Config, api_key: &'a str) -> Self {
        Self {
            http,
            config,
            api_key,
            timeout: SCRAPPA_REQUEST_TIMEOUT,
        }
    }

    pub(super) async fn get(&self, params: &Map<String, Value>) -> Result<Value> {
        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send(params).await {
                Ok(response) => return Ok(response),
                Err(error)
                    if attempt < SCRAPPA_MAX_ATTEMPTS && is_retryable_scrappa_error(&error) =>
                {
                    let delay = retry_delay(attempt, retry_jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        SCRAPPA_MAX_ATTEMPTS,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("the retry loop always returns or continues for a bounded number of attempts")
    }

    async fn send(&self, params: &Map<String, Value>) -> Result<Value> {
        let mut url = endpoint_url(&self.config.scrappa_api_base, &["kleinanzeigen", "search"])?;
        for (key, value) in params {
            if value.is_null() || value.as_str() == Some("") || value == &Value::Bool(false) {
                continue;
            }
            let value = if value == &Value::Bool(true) {
                "1".to_owned()
            } else {
                js_string(value)
            };
            url.query_pairs_mut().append_pair(key, &value);
        }

        let response = self
            .http
            .get(url)
            .timeout(self.timeout)
            .header("X-API-Key", self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(
                header::USER_AGENT,
                "thescrappa-kleinanzeigen-search-scraper/1.0",
            )
            .send()
            .await
            .map_err(|error| scrappa_transport_error(error, self.timeout))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| scrappa_transport_error(error, self.timeout))?;
        if !status.is_success() {
            return Err(scrappa_error(status.as_u16(), &body).into());
        }
        serde_json::from_str(&body)
            .map_err(|error| anyhow!("Scrappa API response was not valid JSON: {error}"))
    }
}

fn scrappa_transport_error(error: reqwest::Error, timeout: Duration) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaTimeoutError { timeout }.into()
    } else {
        error.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::actor_failure_message;
    use anyhow::anyhow;
    use url::Url;

    #[test]
    fn preserves_retryable_statuses_timeout_deadline_and_error_text() {
        for status in [408, 429, 500, 502, 503, 504] {
            let error: anyhow::Error = scrappa_error(status, "busy").into();
            assert!(is_retryable_scrappa_error(&error));
        }
        for status in [400, 401, 403, 404] {
            let error: anyhow::Error = scrappa_error(status, "bad request").into();
            assert!(!is_retryable_scrappa_error(&error));
        }
        assert_eq!(retry_delay(1, 999), Duration::from_millis(2_999));
        assert_eq!(retry_delay(2, 999), Duration::from_millis(4_999));
        assert_eq!(retry_delay(5, 999), Duration::from_millis(10_000));
        assert!(is_retryable_scrappa_error(&anyhow!(ScrappaTimeoutError {
            timeout: SCRAPPA_REQUEST_TIMEOUT
        })));
        assert_eq!(
            scrappa_error(
                422,
                r#"{"message":"Invalid input","errors":{"query":["is required","is too long"]}}"#
            )
            .to_string(),
            "Scrappa API error (422): Invalid input - query: is required, is too long"
        );
        assert_eq!(
            scrappa_error(503, " unavailable \n now ").to_string(),
            "Scrappa API error (503): unavailable now"
        );
        assert_eq!(
            scrappa_error(400, "").to_string(),
            "Scrappa API error (400): Bad Request"
        );
        assert_eq!(
            scrappa_error(503, "[]").to_string(),
            "Scrappa API error (503): Service Unavailable"
        );
        assert_eq!(
            scrappa_error(503, "null").to_string(),
            "Scrappa API error (503): null"
        );
        let error: anyhow::Error = ScrappaTimeoutError {
            timeout: SCRAPPA_REQUEST_TIMEOUT,
        }
        .into();
        let message = actor_failure_message(&error);
        assert!(message.contains("timed out after 90000ms"));
        assert!(message.contains("exceeded the 90s Scrappa API timeout"));
        assert!(message.contains("Try fewer searches, narrower filters"));
    }

    #[tokio::test]
    async fn request_deadline_is_enforced() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            if let Ok((_stream, _)) = listener.accept().await {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
        let config = Config {
            apify_api_base: Url::parse(&format!("http://{address}")).unwrap(),
            scrappa_api_base: Url::parse(&format!("http://{address}/api")).unwrap(),
            apify_token: "test-token".to_owned(),
            key_value_store_id: "test-store".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: Some("scrappa-test-key".to_owned()),
        };
        let client = Client::new();
        let mut scrappa = ScrappaClient::new(&client, &config, "scrappa-test-key");
        scrappa.timeout = Duration::from_millis(5);
        let params =
            serde_json::from_value(serde_json::json!({"query": "iphone", "page": 1})).unwrap();
        let error = scrappa.send(&params).await.unwrap_err();
        assert!(error
            .downcast_ref::<ScrappaTimeoutError>()
            .is_some_and(|error| error.timeout == Duration::from_millis(5)));
        assert!(is_retryable_scrappa_error(&error));
        server.abort();
    }
}
