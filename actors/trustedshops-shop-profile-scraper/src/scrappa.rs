use anyhow::{Context, Result, anyhow};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

pub const REQUEST_TIMEOUT_MS: u64 = 90_000;
const MAX_ATTEMPTS: u32 = 3;
const USER_AGENT: &str = "thescrappa-trustedshops-shop-profile-scraper/1.0";

pub struct ScrappaClient {
    client: Client,
    api_key: String,
    base_url: String,
    timeout: Duration,
    retry_delay_override: Option<Duration>,
}

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    timeout_ms: u64,
    source: Option<reqwest::Error>,
}

impl ScrappaTimeoutError {
    pub fn new(timeout_ms: u64, source: Option<reqwest::Error>) -> Self {
        Self { timeout_ms, source }
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

impl Error for ScrappaTimeoutError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_ref()
            .map(|error| error as &(dyn Error + 'static))
    }
}

#[derive(Debug)]
pub struct ScrappaApiError {
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

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .build()
                .context("Could not create Scrappa HTTP client")?,
            api_key,
            base_url: base_url.trim_end_matches('/').to_owned(),
            timeout: Duration::from_millis(REQUEST_TIMEOUT_MS),
            retry_delay_override: None,
        })
    }

    pub async fn get_shop_profile(&self, tsid: &str) -> Result<Value> {
        let endpoint = format!("/trustedshops/shop/{tsid}");
        let url = Url::parse(&format!("{}{endpoint}", self.base_url))
            .context("SCRAPPA_API_BASE_URL must be a valid URL")?;

        for attempt in 1..=MAX_ATTEMPTS {
            match self.send(&url).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < MAX_ATTEMPTS && is_retryable(&error) => {
                    let delay = self.retry_delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        MAX_ATTEMPTS,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }

        Err(anyhow!("Scrappa API request exhausted its retry attempts"))
    }

    async fn send(&self, url: &Url) -> Result<Value> {
        let response = self
            .client
            .get(url.clone())
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|error| self.map_request_error(error))?;

        if !response.status().is_success() {
            return Err(self.api_error(response).await?);
        }

        match response.json::<Value>().await {
            Ok(response) => Ok(response),
            Err(error) => {
                let error = self.map_request_error(error);
                if is_retryable(&error) {
                    Err(error)
                } else {
                    Err(anyhow!("Scrappa API response was not valid JSON"))
                }
            }
        }
    }

    async fn api_error(&self, response: Response) -> Result<anyhow::Error> {
        let status = response.status();
        let fallback = status
            .canonical_reason()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
        let body = response
            .text()
            .await
            .map_err(|error| self.map_request_error(error))?;
        let message = format_scrappa_error_message(status, &body, &fallback);

        Ok(ScrappaApiError {
            status: status.as_u16(),
            message,
        }
        .into())
    }

    fn map_request_error(&self, error: reqwest::Error) -> anyhow::Error {
        if error.is_timeout() {
            ScrappaTimeoutError::new(self.timeout.as_millis() as u64, Some(error)).into()
        } else {
            error.into()
        }
    }

    fn retry_delay(&self, failed_attempt: u32) -> Duration {
        if let Some(delay) = self.retry_delay_override {
            return delay;
        }

        let base_ms = 1_000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt));
        let jitter_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64
            % 1_000;
        Duration::from_millis((base_ms + jitter_ms).min(10_000))
    }

    #[cfg(test)]
    fn with_test_options(mut self, timeout: Duration, retry_delay: Duration) -> Self {
        self.timeout = timeout;
        self.retry_delay_override = Some(retry_delay);
        self
    }
}

pub fn is_retryable(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if let Some(error) = error.downcast_ref::<ScrappaApiError>() {
        return matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504);
    }

    error.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|error| error.is_timeout() || error.is_connect() || error.is_body())
    })
}

fn format_scrappa_error_message(_status: StatusCode, body: &str, fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_owned();
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    let messages = messages.as_array()?;
                    let messages = messages
                        .iter()
                        .map(|message| {
                            message
                                .as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| message.to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    Some(format!("{field}: {messages}"))
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

    let message = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if message.is_empty() {
        fallback.to_owned()
    } else {
        message.chars().take(500).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockResponse, MockServer, request_parts};
    use serde_json::json;

    fn client(server: &MockServer) -> ScrappaClient {
        ScrappaClient::new(
            "test-api-key".to_owned(),
            format!("{}/api", server.base_url),
        )
        .unwrap()
        .with_test_options(Duration::from_secs(1), Duration::ZERO)
    }

    #[tokio::test]
    async fn retries_transient_api_errors_then_returns_profile_data() {
        let server = MockServer::start(vec![
            MockResponse::json(503, json!({"message":"temporary outage"})),
            MockResponse::json(200, json!({"shop":{"tsid":"id","name":"Example"}})),
        ]);
        let response = client(&server)
            .get_shop_profile("XFB15FFBDE1DEE7A55D292A7D48598A6A")
            .await
            .unwrap();

        assert_eq!(response["shop"]["name"], "Example");
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/api/trustedshops/shop/XFB15FFBDE1DEE7A55D292A7D48598A6A"
        );
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .contains("x-api-key: test-api-key")
        );
        assert!(requests[0].contains(USER_AGENT));
    }

    #[tokio::test]
    async fn does_not_retry_non_transient_api_errors_and_formats_validation_details() {
        let server = MockServer::start(vec![MockResponse::json(
            400,
            json!({
                "message":"Invalid input",
                "errors":{"market":["must be DEU", "is required"]}
            }),
        )]);
        let error = client(&server)
            .get_shop_profile("XFB15FFBDE1DEE7A55D292A7D48598A6A")
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Scrappa API error (400): Invalid input - market: must be DEU, is required"
        );
        assert!(!is_retryable(&error));
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn retries_timeout_errors_three_times() {
        let delayed_response = || {
            let mut response = MockResponse::json(200, json!({"shop":{}}));
            response.delay = Duration::from_millis(100);
            response
        };
        let server = MockServer::start(vec![
            delayed_response(),
            delayed_response(),
            delayed_response(),
        ]);
        let client = ScrappaClient::new(
            "test-api-key".to_owned(),
            format!("{}/api", server.base_url),
        )
        .unwrap()
        .with_test_options(Duration::from_millis(10), Duration::ZERO);
        let error = client
            .get_shop_profile("XFB15FFBDE1DEE7A55D292A7D48598A6A")
            .await
            .unwrap_err();

        assert!(error.downcast_ref::<ScrappaTimeoutError>().is_some());
        assert!(is_retryable(&error));
        assert_eq!(server.requests().len(), 3);
        assert_eq!(
            error.to_string(),
            "Scrappa API request timed out after 10ms"
        );
    }

    #[test]
    fn retries_expected_http_and_transport_errors_only() {
        assert!(is_retryable(
            &ScrappaApiError {
                status: 429,
                message: "busy".to_owned()
            }
            .into()
        ));
        assert!(is_retryable(
            &ScrappaApiError {
                status: 503,
                message: "busy".to_owned()
            }
            .into()
        ));
        assert!(!is_retryable(
            &ScrappaApiError {
                status: 400,
                message: "invalid".to_owned()
            }
            .into()
        ));
        assert_eq!(
            format_scrappa_error_message(
                StatusCode::BAD_GATEWAY,
                "backend  failed\n",
                "Bad Gateway"
            ),
            "backend failed"
        );
    }
}
