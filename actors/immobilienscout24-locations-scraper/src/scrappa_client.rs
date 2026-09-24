use std::time::Duration;

use rand::Rng;
use reqwest::{header, Client, StatusCode};
use serde_json::Value;
use url::Url;

use crate::config::endpoint_url;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_ATTEMPTS: usize = 2;
const USER_AGENT: &str = "thescrappa-immobilienscout24-locations-scraper/1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrappaError {
    Api {
        status: u16,
        response_message: String,
    },
    Timeout {
        timeout_ms: u64,
    },
    Transport {
        message: String,
        retryable: bool,
    },
    InvalidJson {
        message: String,
    },
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Api {
                status,
                response_message,
            } => {
                write!(
                    formatter,
                    "Scrappa API error ({status}): {response_message}"
                )
            }
            Self::Timeout { timeout_ms } => {
                write!(
                    formatter,
                    "Scrappa API request timed out after {timeout_ms}ms"
                )
            }
            Self::Transport { message, .. } | Self::InvalidJson { message } => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for ScrappaError {}

impl ScrappaError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout { .. } => true,
            Self::Api { status, .. } => [408, 429, 500, 502, 503, 504].contains(status),
            Self::Transport { retryable, .. } => *retryable,
            Self::InvalidJson { .. } => false,
        }
    }

    pub fn query_failure_message(&self) -> String {
        match self {
            Self::Api {
                status,
                response_message,
            } => {
                format!("Scrappa returned HTTP {status}: {response_message}")
            }
            Self::Timeout { .. } => format!("{self}; retry this query later"),
            _ => self.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct ScrappaClient {
    http: Client,
    api_base: Url,
    api_key: String,
    timeout_ms: u64,
    retry_jitter_ms: u64,
    retry_base_ms: u64,
}

impl ScrappaClient {
    pub fn new(api_base: Url, api_key: String) -> Result<Self, reqwest::Error> {
        Self::with_retry_options(api_base, api_key, REQUEST_TIMEOUT, 1_000, 1_000)
    }

    fn with_retry_options(
        api_base: Url,
        api_key: String,
        timeout: Duration,
        retry_base_ms: u64,
        retry_jitter_ms: u64,
    ) -> Result<Self, reqwest::Error> {
        Ok(Self {
            http: Client::builder().timeout(timeout).build()?,
            api_base,
            api_key,
            timeout_ms: timeout.as_millis() as u64,
            retry_jitter_ms,
            retry_base_ms,
        })
    }

    pub async fn get_locations(&self, query: &str, limit: usize) -> Result<Value, ScrappaError> {
        let url =
            endpoint_url(&self.api_base, &["immobilienscout24", "locations"]).map_err(|error| {
                ScrappaError::Transport {
                    message: error.to_string(),
                    retryable: false,
                }
            })?;
        let mut url = url;
        url.query_pairs_mut()
            .append_pair("query", query)
            .append_pair("limit", &limit.to_string());

        for attempt in 1..=MAX_ATTEMPTS {
            match self.get_once(&url).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < MAX_ATTEMPTS && error.is_retryable() => {
                    let delay = self.retry_delay(attempt);
                    eprintln!(
                        "Scrappa request failed ({error}). Retrying {}/{MAX_ATTEMPTS} in {}ms.",
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("the Scrappa retry loop returns on its final attempt")
    }

    async fn get_once(&self, url: &Url) -> Result<Value, ScrappaError> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ScrappaError::Timeout {
                        timeout_ms: self.timeout_ms,
                    }
                } else {
                    ScrappaError::Transport {
                        message: error.to_string(),
                        retryable: error.is_connect(),
                    }
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.map_err(|error| {
                if error.is_timeout() {
                    ScrappaError::Timeout {
                        timeout_ms: self.timeout_ms,
                    }
                } else {
                    ScrappaError::Transport {
                        message: error.to_string(),
                        retryable: error.is_connect() || error.is_timeout(),
                    }
                }
            })?;
            return Err(ScrappaError::Api {
                status: status.as_u16(),
                response_message: read_error_message(status, &body),
            });
        }

        response.json().await.map_err(|error| {
            if error.is_timeout() {
                ScrappaError::Timeout {
                    timeout_ms: self.timeout_ms,
                }
            } else if error.is_body() || error.is_connect() {
                ScrappaError::Transport {
                    message: error.to_string(),
                    retryable: true,
                }
            } else {
                ScrappaError::InvalidJson {
                    message: error.to_string(),
                }
            }
        })
    }

    fn retry_delay(&self, failed_attempt: usize) -> Duration {
        let exponent = u32::try_from(failed_attempt).unwrap_or(u32::MAX);
        let base = self
            .retry_base_ms
            .saturating_mul(2_u64.saturating_pow(exponent));
        let jitter = if self.retry_jitter_ms == 0 {
            0
        } else {
            rand::thread_rng().gen_range(0..=self.retry_jitter_ms)
        };
        Duration::from_millis(base.saturating_add(jitter).min(10_000))
    }
}

fn read_error_message(status: StatusCode, body: &str) -> String {
    let normalized = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return status
            .canonical_reason()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    }
    if let Ok(value) = serde_json::from_str::<Value>(&normalized) {
        if let Some(message) = value.get("message").and_then(Value::as_str) {
            return message.to_owned();
        }
    }
    normalized.chars().take(500).collect()
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use reqwest::StatusCode;
    use serde_json::json;
    use url::Url;

    use crate::test_utils::{MockResponse, MockServer};

    use super::{read_error_message, ScrappaClient, ScrappaError, USER_AGENT};

    #[test]
    fn retryability_matches_transient_http_and_network_errors() {
        assert!(ScrappaError::Timeout { timeout_ms: 1_000 }.is_retryable());
        assert!(ScrappaError::Api {
            status: 503,
            response_message: "Unavailable".into()
        }
        .is_retryable());
        assert!(!ScrappaError::Api {
            status: 422,
            response_message: "Invalid".into()
        }
        .is_retryable());
        assert!(ScrappaError::Transport {
            message: "network down".into(),
            retryable: true
        }
        .is_retryable());
        assert!(!ScrappaError::InvalidJson {
            message: "bad json".into()
        }
        .is_retryable());
    }

    #[test]
    fn parses_scrappa_error_messages_with_a_bounded_fallback() {
        assert_eq!(
            read_error_message(StatusCode::BAD_REQUEST, "{\"message\":\"Bad input\"}"),
            "Bad input"
        );
        assert_eq!(
            read_error_message(StatusCode::BAD_REQUEST, "  Bad   input  "),
            "Bad input"
        );
        assert_eq!(
            read_error_message(StatusCode::BAD_GATEWAY, ""),
            "Bad Gateway"
        );
        assert_eq!(
            read_error_message(StatusCode::BAD_REQUEST, &"x".repeat(510)).len(),
            500
        );
    }

    #[tokio::test]
    async fn sends_scrappa_key_agent_query_and_limit_to_the_locations_endpoint() {
        let server = MockServer::start(|request| {
            assert_eq!(request.method, "GET");
            assert_eq!(request.header("x-api-key"), Some("test-key"));
            assert_eq!(request.header("user-agent"), Some(USER_AGENT));
            let url = Url::parse(&format!("http://localhost{}", request.path)).unwrap();
            assert_eq!(url.path(), "/api/immobilienscout24/locations");
            assert_eq!(
                url.query_pairs().find(|(key, _)| key == "query").unwrap().1,
                "Berlin Mitte"
            );
            assert_eq!(
                url.query_pairs().find(|(key, _)| key == "limit").unwrap().1,
                "5"
            );
            MockResponse::json(200, json!({ "locations": [] }))
        });
        let client = ScrappaClient::with_retry_options(
            Url::parse(&server.base_url("api")).unwrap(),
            "test-key".into(),
            Duration::from_secs(1),
            0,
            0,
        )
        .unwrap();

        assert_eq!(
            client.get_locations("Berlin Mitte", 5).await.unwrap(),
            json!({ "locations": [] })
        );
    }

    #[tokio::test]
    async fn retries_a_retryable_response_once() {
        let count = Arc::new(AtomicUsize::new(0));
        let server_count = count.clone();
        let server = MockServer::start(move |_| {
            if server_count.fetch_add(1, Ordering::SeqCst) == 0 {
                MockResponse::json(503, json!({ "message": "Unavailable" }))
            } else {
                MockResponse::json(200, json!({ "locations": [{ "geocode": "1" }] }))
            }
        });
        let client = ScrappaClient::with_retry_options(
            Url::parse(&server.base_url("api")).unwrap(),
            "test-key".into(),
            Duration::from_secs(1),
            0,
            0,
        )
        .unwrap();

        assert_eq!(
            client.get_locations("Berlin", 1).await.unwrap(),
            json!({ "locations": [{ "geocode": "1" }] })
        );
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn enforces_request_deadline_and_retries_timeout_once() {
        let count = Arc::new(AtomicUsize::new(0));
        let server_count = count.clone();
        let server = MockServer::start(move |_| {
            server_count.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(100));
            MockResponse::json(200, json!({ "locations": [] }))
        });
        let client = ScrappaClient::with_retry_options(
            Url::parse(&server.base_url("api")).unwrap(),
            "test-key".into(),
            Duration::from_millis(20),
            0,
            0,
        )
        .unwrap();

        let error = client.get_locations("Berlin", 1).await.unwrap_err();
        assert_eq!(error, ScrappaError::Timeout { timeout_ms: 20 });
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }
}
