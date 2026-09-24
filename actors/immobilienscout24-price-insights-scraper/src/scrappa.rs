use reqwest::{Client, Response, StatusCode, Url};
use serde_json::Value;
use std::{time::Duration, time::SystemTime};
use tokio::time::{sleep, timeout};

const PRICE_INSIGHTS_PATH: [&str; 2] = ["immobilienscout24", "price-insights"];
const REQUEST_ATTEMPTS: usize = 2;
const USER_AGENT: &str = "thescrappa-immobilienscout24-price-insights-scraper/1.0";

#[derive(Clone)]
pub struct ScrappaClient {
    client: Client,
    api_key: String,
    base_url: Url,
    request_timeout: Duration,
    retry_delay_override: Option<Duration>,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: Url, request_timeout: Duration) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
            request_timeout,
            retry_delay_override: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay_override = Some(delay);
        self
    }

    pub async fn get_price_insights(&self, location: &str) -> Result<Value, ScrappaFailure> {
        let url = self.price_insights_url(location)?;
        let mut last_error = None;

        for attempt in 1..=REQUEST_ATTEMPTS {
            match self.send_once(url.clone()).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let should_retry = attempt < REQUEST_ATTEMPTS && error.retryable;
                    last_error = Some(error);
                    if !should_retry {
                        break;
                    }

                    let error = last_error
                        .as_ref()
                        .expect("the request error was just stored");
                    let delay = self
                        .retry_delay_override
                        .unwrap_or_else(|| get_retry_delay(attempt));
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{REQUEST_ATTEMPTS} in {}ms.",
                        error.retry_log_message(),
                        attempt + 1,
                        delay.as_millis()
                    );
                    sleep(delay).await;
                }
            }
        }

        Err(last_error.expect("the request loop always runs at least once"))
    }

    fn price_insights_url(&self, location: &str) -> Result<Url, ScrappaFailure> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| {
                ScrappaFailure::non_retryable("Scrappa API base URL cannot contain path segments")
            })?
            .pop_if_empty()
            .extend(PRICE_INSIGHTS_PATH);
        url.set_query(None);
        url.query_pairs_mut().append_pair("location", location);
        Ok(url)
    }

    async fn send_once(&self, url: Url) -> Result<Value, ScrappaFailure> {
        let request = self
            .client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .timeout(self.request_timeout);

        let result = timeout(self.request_timeout, async {
            let response = request
                .send()
                .await
                .map_err(|error| ScrappaFailure::from_request(error, self.request_timeout))?;
            if !response.status().is_success() {
                return Err(scrappa_http_failure(response, self.request_timeout).await);
            }

            let body = response
                .text()
                .await
                .map_err(|error| ScrappaFailure::from_response_body(error, self.request_timeout))?;
            serde_json::from_str(&body)
                .map_err(|error| ScrappaFailure::non_retryable(error.to_string()))
        })
        .await;

        result.unwrap_or_else(|_| Err(ScrappaFailure::timeout(self.request_timeout)))
    }
}

#[derive(Debug)]
pub struct ScrappaFailure {
    pub message: String,
    pub status: Option<u16>,
    pub(crate) retryable: bool,
    pub(crate) retry_message: Option<String>,
}

impl ScrappaFailure {
    fn timeout(timeout: Duration) -> Self {
        let message = format!(
            "Scrappa API request timed out after {}ms",
            timeout.as_millis()
        );
        Self {
            retry_message: Some(message.clone()),
            message,
            status: None,
            retryable: true,
        }
    }

    fn non_retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: None,
            retryable: false,
            retry_message: None,
        }
    }

    fn from_request(error: reqwest::Error, request_timeout: Duration) -> Self {
        if error.is_timeout() {
            return Self::timeout(request_timeout);
        }
        let message = if error.is_connect() {
            "fetch failed".to_owned()
        } else {
            error.to_string()
        };
        let retryable = error.is_connect() || error.is_timeout();
        Self {
            retry_message: Some(message.clone()),
            message,
            status: None,
            retryable,
        }
    }

    fn from_response_body(error: reqwest::Error, request_timeout: Duration) -> Self {
        if error.is_timeout() {
            return Self::timeout(request_timeout);
        }
        let message = error.to_string();
        Self {
            retry_message: Some(message.clone()),
            message,
            status: None,
            retryable: error.is_connect() || error.is_timeout() || error.is_body(),
        }
    }

    fn http(status: StatusCode, response_message: String) -> Self {
        let code = status.as_u16();
        Self {
            message: response_message.clone(),
            status: Some(code),
            retryable: matches!(code, 408 | 429 | 500 | 502 | 503 | 504),
            retry_message: Some(format!("Scrappa API error ({code}): {response_message}")),
        }
    }

    fn retry_log_message(&self) -> &str {
        self.retry_message.as_deref().unwrap_or(&self.message)
    }
}

async fn scrappa_http_failure(response: Response, request_timeout: Duration) -> ScrappaFailure {
    let status = response.status();
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    match response.text().await {
        Ok(body) if body.is_empty() => ScrappaFailure::http(status, fallback),
        Ok(body) => ScrappaFailure::http(status, parse_error_message(&body, &fallback)),
        Err(error) => ScrappaFailure::from_response_body(error, request_timeout),
    }
}

fn parse_error_message(body: &str, fallback: &str) -> String {
    if let Ok(error) = serde_json::from_str::<Value>(body) {
        let mut message = error
            .get("message")
            .filter(|message| !message.is_null())
            .map(javascript_string)
            .unwrap_or_else(|| fallback.to_owned());
        if let Some(errors) = error.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let messages = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .map(javascript_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_else(|| javascript_string(messages));
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

fn javascript_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::Null => String::new(),
                _ => javascript_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn get_retry_delay(attempt: usize) -> Duration {
    let backoff_ms = 1_000_u64.saturating_mul(2_u64.saturating_pow(attempt as u32));
    let jitter_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|time| u64::from(time.subsec_millis()))
        .unwrap_or_default();
    Duration::from_millis((backoff_ms + jitter_ms).min(10_000))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockResponse, MockServer};
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn formats_json_and_plain_text_error_messages_like_the_actor_sdk() {
        assert_eq!(
            parse_error_message(
                r#"{"message":"Validation failed","errors":{"location":["is invalid","is required"]}}"#,
                "Bad Request"
            ),
            "Validation failed - location: is invalid, is required"
        );
        assert_eq!(
            parse_error_message("  upstream   unavailable\nnow ", "Bad Request"),
            "upstream unavailable now"
        );
        assert_eq!(parse_error_message("", "Bad Request"), "");
    }

    #[test]
    fn retries_only_the_original_transient_status_codes() {
        for code in [408, 429, 500, 502, 503, 504] {
            assert!(
                ScrappaFailure::http(StatusCode::from_u16(code).unwrap(), "temporary".to_owned())
                    .retryable
            );
        }
        for code in [400, 401, 403, 404, 501] {
            assert!(
                !ScrappaFailure::http(StatusCode::from_u16(code).unwrap(), "permanent".to_owned())
                    .retryable
            );
        }
    }

    #[test]
    fn retries_after_two_seconds_plus_jitter_on_the_first_failure() {
        let delay = get_retry_delay(1);
        assert!(delay >= Duration::from_secs(2));
        assert!(delay <= Duration::from_secs(3));
    }

    #[tokio::test]
    async fn retries_a_transient_scrappa_response_with_the_existing_request_contract() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_in_handler = Arc::clone(&attempts);
        let server = MockServer::start(move |_| {
            if attempts_in_handler.fetch_add(1, Ordering::Relaxed) == 0 {
                MockResponse::json(503, json!({"message": "temporarily unavailable"}))
            } else {
                MockResponse::json(200, json!({"success": true}))
            }
        });
        let client = ScrappaClient::new(
            "scrappa-test-key".to_owned(),
            Url::parse(&format!(
                "{}/api",
                server.base_url.as_str().trim_end_matches('/')
            ))
            .unwrap(),
            Duration::from_secs(1),
        )
        .with_retry_delay(Duration::ZERO);

        let response = client.get_price_insights("New York & Mitte").await.unwrap();

        assert_eq!(response, json!({"success": true}));
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].target.ends_with("?location=New+York+%26+Mitte"));
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].header("x-api-key"), Some("scrappa-test-key"));
        assert_eq!(requests[0].header("authorization"), None);
        assert_eq!(requests[0].header("accept"), Some("application/json"));
        assert_eq!(requests[0].header("user-agent"), Some(USER_AGENT));
    }

    #[tokio::test]
    async fn bounds_each_scrappa_attempt_and_retries_timeouts_once() {
        let server = MockServer::start(|_| {
            MockResponse::json(200, json!({"success": true})).delayed(Duration::from_millis(300))
        });
        let client = ScrappaClient::new(
            "scrappa-test-key".to_owned(),
            Url::parse(&format!(
                "{}/api",
                server.base_url.as_str().trim_end_matches('/')
            ))
            .unwrap(),
            Duration::from_millis(50),
        )
        .with_retry_delay(Duration::ZERO);

        let error = client.get_price_insights("Berlin").await.unwrap_err();

        assert_eq!(error.message, "Scrappa API request timed out after 50ms");
        assert_eq!(error.status, None);
        assert_eq!(server.requests().len(), 2);
    }
}
