use crate::scrappa_retry::ScrappaRetryExt;
use std::{fmt, time::Duration};

use reqwest::{header, Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;
use url::Url;

pub const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
pub const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(45);
const USER_AGENT: &str = "thescrappa-google-finance-markets-scraper/1.0";

#[derive(Debug)]
pub enum ScrappaError {
    Timeout { timeout: Duration },
    Http { status: StatusCode, message: String },
    Transport(reqwest::Error),
    InvalidJson(serde_json::Error),
}

impl ScrappaError {
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
    }
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout { timeout } => write!(
                formatter,
                "Scrappa API request timed out after {}ms",
                timeout.as_millis()
            ),
            Self::Http { status, message } => {
                write!(
                    formatter,
                    "Scrappa API error ({}): {message}",
                    status.as_u16()
                )
            }
            Self::Transport(error) => write!(formatter, "{error}"),
            Self::InvalidJson(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ScrappaError {}

pub struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
    timeout: Duration,
}

impl ScrappaClient {
    pub fn new(base_url: Url, api_key: String, timeout: Duration) -> Result<Self, reqwest::Error> {
        let http = Client::builder().timeout(timeout).build()?;
        Ok(Self {
            http,
            base_url,
            api_key,
            timeout,
        })
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        params: &serde_json::Map<String, Value>,
    ) -> Result<T, ScrappaError> {
        self.send::<T>(endpoint, params).await
    }

    async fn send<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        params: &serde_json::Map<String, Value>,
    ) -> Result<T, ScrappaError> {
        let url = build_request_url(&self.base_url, endpoint, params);
        let response = self
            .http
            .get(url)
            .timeout(self.timeout)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .header("X-API-Key", &self.api_key)
            .send_scrappa_with_retry("Scrappa API request")
            .await
            .map_err(|error| classify_transport_error(error, self.timeout))?;
        parse_response(response, self.timeout).await
    }
}

pub fn build_request_url(
    base_url: &Url,
    endpoint: &str,
    params: &serde_json::Map<String, Value>,
) -> Url {
    let mut url = base_url.clone();
    let endpoint = endpoint.trim_start_matches('/');
    url.path_segments_mut()
        .expect("Scrappa API base URL must support path segments")
        .pop_if_empty()
        .extend(endpoint.split('/'));
    {
        let mut query = url.query_pairs_mut();
        for (name, value) in params {
            if let Some(value) = value.as_str() {
                if !value.is_empty() {
                    query.append_pair(name, value);
                }
            }
        }
    }
    url
}

fn classify_transport_error(error: reqwest::Error, timeout: Duration) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout { timeout }
    } else {
        ScrappaError::Transport(error)
    }
}

async fn parse_response<T: DeserializeOwned>(
    response: Response,
    timeout: Duration,
) -> Result<T, ScrappaError> {
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|error| classify_transport_error(error, timeout))?;
    if !status.is_success() {
        return Err(ScrappaError::Http {
            status,
            message: scrappa_error_message(status, &body),
        });
    }
    serde_json::from_slice(&body).map_err(ScrappaError::InvalidJson)
}

fn scrappa_error_message(status: StatusCode, body: &[u8]) -> String {
    let body = String::from_utf8_lossy(body);
    let fallback = status.canonical_reason().unwrap_or("HTTP status");
    if body.is_empty() {
        return fallback.to_owned();
    }

    if let Ok(Value::Object(data)) = serde_json::from_str::<Value>(&body) {
        let message = data
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or_else(|| fallback.to_owned());
        if let Some(errors) = data.get("errors").filter(|value| js_truthy(value)) {
            let Some(errors) = errors.as_object() else {
                return compact_body(&body);
            };
            let mut details = Vec::with_capacity(errors.len());
            for (field, messages) in errors {
                let Some(messages) = messages.as_array() else {
                    return compact_body(&body);
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
                return format!("{message} - {}", details.join("; "));
            }
        }
        return message;
    }
    compact_body(&body)
}

fn compact_body(body: &str) -> String {
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(number) => number.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::MARKET_TRENDS;
    use crate::test_support::{MockResponse, MockServer};
    use serde_json::json;

    #[test]
    fn builds_authenticated_markets_url_with_encoded_query_values() {
        let base_url = Url::parse("https://scrappa.test/api").unwrap();
        let params = json!({ "trend": "most-active", "hl": "zh-cn", "gl": "us" });
        let url = build_request_url(
            &base_url,
            "/google-finance/markets",
            params.as_object().unwrap(),
        );
        assert_eq!(
            url.as_str(),
            "https://scrappa.test/api/google-finance/markets?trend=most-active&hl=zh-cn&gl=us"
        );
        assert_eq!(MARKET_TRENDS.len(), 7);
    }

    #[tokio::test]
    async fn sends_expected_headers_and_retries_retryable_statuses() {
        let server = MockServer::start(vec![
            MockResponse::json(503, r#"{"message":"Service unavailable"}"#),
            MockResponse::json(200, r#"{"ok":true}"#),
        ]);
        let client = ScrappaClient::new(
            Url::parse(&server.base_url()).unwrap(),
            "test-scrappa-key".to_owned(),
            Duration::from_secs(1),
        )
        .unwrap();
        let params = json!({ "trend": "gainers" });
        let response: Value = client
            .get("/google-finance/markets", params.as_object().unwrap())
            .await
            .unwrap();
        assert_eq!(response, json!({ "ok": true }));

        let requests = server.join();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(
            requests[0].target,
            "/api/google-finance/markets?trend=gainers"
        );
        assert_eq!(requests[0].headers["x-api-key"], "test-scrappa-key");
        assert_eq!(requests[0].headers["accept"], "application/json");
        assert_eq!(
            requests[0].headers["user-agent"],
            "thescrappa-google-finance-markets-scraper/1.0"
        );
    }

    #[tokio::test]
    async fn formats_scrappa_validation_errors_and_does_not_retry_them() {
        let server = MockServer::start(vec![MockResponse::json(
            422,
            r#"{"message":"The given data was invalid.","errors":{"trend":["must be valid"],"hl":["unsupported"]}}"#,
        )]);
        let client = ScrappaClient::new(
            Url::parse(&server.base_url()).unwrap(),
            "test-key".to_owned(),
            Duration::from_secs(1),
        )
        .unwrap();
        let error = client
            .get::<Value>("/google-finance/markets", &serde_json::Map::new())
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (422): The given data was invalid. - trend: must be valid; hl: unsupported"
        );
        assert_eq!(server.join().len(), 1);
    }

    #[tokio::test]
    async fn converts_request_deadline_to_scrappa_timeout() {
        let server = MockServer::start(vec![
            MockResponse::json(200, r#"{"ok":true}"#).after(Duration::from_millis(150))
        ]);
        let timeout = Duration::from_millis(25);
        let client = ScrappaClient::new(
            Url::parse(&server.base_url()).unwrap(),
            "test-key".to_owned(),
            timeout,
        )
        .unwrap();
        let error = client
            .get::<Value>("/google-finance/markets", &serde_json::Map::new())
            .await
            .unwrap_err();
        assert!(error.is_timeout());
        assert_eq!(
            error.to_string(),
            "Scrappa API request timed out after 25ms"
        );
        server.join();
    }
}
