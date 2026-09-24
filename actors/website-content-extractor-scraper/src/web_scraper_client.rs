use std::time::Duration;

use reqwest::{header, Client, Response};
use serde_json::Value;
use url::Url;

use crate::input::WebScraperParams;

const USER_AGENT: &str = "thescrappa-website-content-extractor-scraper/1.0";
const RETRYABLE_HTTP_STATUSES: [u16; 5] = [429, 500, 502, 503, 504];

#[derive(Debug)]
pub enum ScrappaError {
    Http {
        status: u16,
        details: String,
        body: Option<Value>,
    },
    Timeout {
        timeout: Duration,
    },
    Request {
        message: String,
        retryable: bool,
    },
    ResponseBody(String),
}

impl ScrappaError {
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::Http { .. } => "scrappa_api_error",
            Self::Timeout { .. } => "scrappa_api_timeout",
            Self::Request { .. } | Self::ResponseBody(_) => "error",
        }
    }

    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Http { status, .. } => Some(*status),
            Self::Timeout { .. } | Self::Request { .. } | Self::ResponseBody(_) => None,
        }
    }

    pub fn body(&self) -> Option<&Value> {
        match self {
            Self::Http { body, .. } => body.as_ref(),
            Self::Timeout { .. } | Self::Request { .. } | Self::ResponseBody(_) => None,
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::Http { status, .. } => RETRYABLE_HTTP_STATUSES.contains(status),
            Self::Timeout { .. } => true,
            Self::Request { retryable, .. } => *retryable,
            Self::ResponseBody(_) => false,
        }
    }
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http {
                status, details, ..
            } => write!(
                formatter,
                "Scrappa Web Scraper API error ({status}): {details}"
            ),
            Self::Timeout { timeout } => write!(
                formatter,
                "Scrappa Web Scraper API request timed out after {}ms",
                timeout.as_millis()
            ),
            Self::Request { message, .. } | Self::ResponseBody(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for ScrappaError {}

pub struct ScrappaWebScraperClient {
    http: Client,
    api_key: String,
    base_url: Url,
    timeout: Duration,
    max_attempts: usize,
    retry_delay: Duration,
}

impl ScrappaWebScraperClient {
    pub fn new(
        api_key: String,
        base_url: Url,
        timeout: Duration,
        max_attempts: usize,
        retry_delay: Duration,
    ) -> Result<Self, reqwest::Error> {
        Ok(Self {
            http: Client::builder().build()?,
            api_key,
            base_url,
            timeout,
            max_attempts: max_attempts.max(1),
            retry_delay,
        })
    }

    pub async fn scrape_json(&self, params: &WebScraperParams) -> Result<Value, ScrappaError> {
        match self
            .fetch_body(params, "application/json", ResponseBodyType::Json)
            .await?
        {
            ScrappaResponseBody::Json(response) => Ok(response),
            ScrappaResponseBody::Text(_) => unreachable!("JSON requests return JSON bodies"),
        }
    }

    pub async fn scrape_markdown(&self, params: &WebScraperParams) -> Result<String, ScrappaError> {
        match self
            .fetch_body(
                params,
                "text/markdown, text/plain;q=0.9, */*;q=0.8",
                ResponseBodyType::Text,
            )
            .await?
        {
            ScrappaResponseBody::Text(response) => Ok(response),
            ScrappaResponseBody::Json(_) => unreachable!("Markdown requests return text bodies"),
        }
    }

    async fn fetch_body(
        &self,
        params: &WebScraperParams,
        accept: &str,
        body_type: ResponseBodyType,
    ) -> Result<ScrappaResponseBody, ScrappaError> {
        let url = build_request_url(&self.base_url, params)?;
        let mut last_error = None;

        for attempt in 1..=self.max_attempts {
            match tokio::time::timeout(self.timeout, self.send_once(url.clone(), accept, body_type))
                .await
            {
                Ok(Ok(response_body)) => return Ok(response_body),
                Ok(Err(error)) => last_error = Some(error),
                Err(_) => {
                    last_error = Some(ScrappaError::Timeout {
                        timeout: self.timeout,
                    })
                }
            }

            let error = last_error
                .as_ref()
                .expect("request attempt stores an error");
            if attempt >= self.max_attempts || !error.is_retryable() {
                break;
            }

            eprintln!(
                "Scrappa Web Scraper API request failed ({error}). Retrying attempt {}/{}.",
                attempt + 1,
                self.max_attempts
            );
            tokio::time::sleep(self.retry_delay).await;
        }

        Err(last_error.expect("at least one request attempt is configured"))
    }

    async fn send_once(
        &self,
        url: Url,
        accept: &str,
        body_type: ResponseBodyType,
    ) -> Result<ScrappaResponseBody, ScrappaError> {
        let request = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, accept)
            .header(header::USER_AGENT, USER_AGENT);
        let response = request
            .send()
            .await
            .map_err(|error| ScrappaError::Request {
                message: error.to_string(),
                retryable: error.is_timeout() || error.is_connect() || error.is_request(),
            })?;

        if response.status().is_success() {
            return match body_type {
                ResponseBodyType::Json => response
                    .json::<Value>()
                    .await
                    .map(ScrappaResponseBody::Json)
                    .map_err(|error| ScrappaError::ResponseBody(error.to_string())),
                ResponseBodyType::Text => response
                    .text()
                    .await
                    .map(ScrappaResponseBody::Text)
                    .map_err(|error| ScrappaError::ResponseBody(error.to_string())),
            };
        }

        Err(build_http_error(response).await)
    }
}

#[derive(Clone, Copy)]
enum ResponseBodyType {
    Json,
    Text,
}

enum ScrappaResponseBody {
    Json(Value),
    Text(String),
}

fn build_request_url(base_url: &Url, params: &WebScraperParams) -> Result<Url, ScrappaError> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| ScrappaError::Request {
            message: "Scrappa API base URL cannot contain path segments".to_owned(),
            retryable: false,
        })?
        .pop_if_empty()
        .push("web-scraper");
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("url", &params.url);
        if params.include_html == Some(true) {
            query.append_pair("include_html", "1");
        }
        query.append_pair("response_type", params.response_type.as_str());
    }
    Ok(url)
}

async fn build_http_error(response: Response) -> ScrappaError {
    let status = response.status();
    let status_code = status.as_u16();
    let fallback = status.canonical_reason().unwrap_or("Unknown status");
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let body_text = match response.text().await {
        Ok(body) => body,
        Err(error) => {
            return ScrappaError::Request {
                message: error.to_string(),
                retryable: error.is_timeout() || error.is_connect() || error.is_request(),
            }
        }
    };

    if body_text.is_empty() {
        return ScrappaError::Http {
            status: status_code,
            details: if fallback.is_empty() {
                format!("HTTP {status_code}")
            } else {
                fallback.to_owned()
            },
            body: None,
        };
    }

    if content_type.contains("application/json") {
        if let Ok(body) = serde_json::from_str::<Value>(&body_text) {
            return ScrappaError::Http {
                status: status_code,
                details: describe_json_error(&body, fallback),
                body: Some(body),
            };
        }
    }

    ScrappaError::Http {
        status: status_code,
        details: clean_text(&body_text),
        body: None,
    }
}

fn describe_json_error(body: &Value, fallback: &str) -> String {
    let mut message = body
        .get("message")
        .filter(|value| !value.is_null())
        .or_else(|| body.get("error"))
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned();
    if let Some(errors) = body.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .map(|(field, messages)| {
                let messages = messages
                    .as_array()
                    .map(|messages| {
                        messages
                            .iter()
                            .filter_map(Value::as_str)
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
    message
}

fn clean_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use reqwest::StatusCode;
    use serde_json::json;
    use url::Url;

    use crate::{
        input::{ResponseType, WebScraperParams},
        test_support::{start_mock_server, MockResponse},
    };

    use super::{ScrappaError, ScrappaWebScraperClient};

    fn build_client(base_url: Url, timeout: Duration, attempts: usize) -> ScrappaWebScraperClient {
        ScrappaWebScraperClient::new(
            "test-key".to_owned(),
            base_url,
            timeout,
            attempts,
            Duration::ZERO,
        )
        .unwrap()
    }

    fn json_params(include_html: Option<bool>) -> WebScraperParams {
        WebScraperParams {
            url: "https://example.com".to_owned(),
            include_html,
            response_type: ResponseType::Json,
        }
    }

    #[tokio::test]
    async fn sends_filtered_json_request_with_scrappa_auth_headers() {
        let server = start_mock_server(vec![MockResponse::json(200, r#"{"success":true}"#)]).await;
        let client = build_client(server.base_url(), Duration::from_secs(1), 1);

        let response = client.scrape_json(&json_params(Some(false))).await.unwrap();
        let request = server.requests().await.remove(0);

        assert_eq!(response, json!({"success": true}));
        assert!(request.starts_with(
            "GET /api/web-scraper?url=https%3A%2F%2Fexample.com&response_type=json HTTP/1.1"
        ));
        assert!(request.to_ascii_lowercase().contains("x-api-key: test-key"));
        assert!(request.contains("accept: application/json"));
        assert!(request.contains("thescrappa-website-content-extractor-scraper/1.0"));
    }

    #[tokio::test]
    async fn requests_markdown_and_returns_the_response_text() {
        let server = start_mock_server(vec![MockResponse::text(200, "# Example Domain")]).await;
        let client = build_client(server.base_url(), Duration::from_secs(1), 1);
        let params = WebScraperParams {
            url: "https://example.com".to_owned(),
            include_html: None,
            response_type: ResponseType::Markdown,
        };

        assert_eq!(
            client.scrape_markdown(&params).await.unwrap(),
            "# Example Domain"
        );
        let request = server.requests().await.remove(0);
        assert!(request.contains("accept: text/markdown, text/plain;q=0.9, */*;q=0.8"));
    }

    #[tokio::test]
    async fn retries_transient_status_and_returns_json_after_second_attempt() {
        let server = start_mock_server(vec![
            MockResponse::text_with_status(503, "Service Unavailable", "Temporary failure"),
            MockResponse::json(200, r#"{"success":true}"#),
        ])
        .await;
        let client = build_client(server.base_url(), Duration::from_secs(1), 2);

        assert_eq!(
            client.scrape_json(&json_params(None)).await.unwrap(),
            json!({"success": true})
        );
        assert_eq!(server.requests().await.len(), 2);
    }

    #[tokio::test]
    async fn does_not_retry_validation_status_and_preserves_error_details() {
        let server = start_mock_server(vec![MockResponse::json(
            400,
            r#"{"message":"Invalid request","error_code":"INVALID_URL","errors":{"url":["The url parameter is required."]}}"#,
        )])
        .await;
        let client = build_client(server.base_url(), Duration::from_secs(1), 2);

        let error = client.scrape_json(&json_params(None)).await.unwrap_err();
        assert_eq!(error.status_code(), Some(StatusCode::BAD_REQUEST.as_u16()));
        assert_eq!(
            error.to_string(),
            "Scrappa Web Scraper API error (400): Invalid request - url: The url parameter is required."
        );
        assert_eq!(error.body().unwrap()["error_code"], "INVALID_URL");
        assert_eq!(server.requests().await.len(), 1);
    }

    #[tokio::test]
    async fn bounds_each_request_by_the_configured_timeout_and_retries_it() {
        let server = start_mock_server(vec![
            MockResponse::delayed_text(100, 200, "OK", "too slow"),
            MockResponse::json(200, r#"{"success":true}"#),
        ])
        .await;
        let client = build_client(server.base_url(), Duration::from_millis(10), 2);

        assert_eq!(
            client.scrape_json(&json_params(None)).await.unwrap(),
            json!({"success": true})
        );
        assert_eq!(server.requests().await.len(), 2);

        let server =
            start_mock_server(vec![MockResponse::delayed_text(100, 200, "OK", "too slow")]).await;
        let client = build_client(server.base_url(), Duration::from_millis(10), 1);
        let error = client.scrape_json(&json_params(None)).await.unwrap_err();
        assert!(matches!(error, ScrappaError::Timeout { .. }));
    }

    #[tokio::test]
    async fn bounds_response_body_by_the_configured_timeout_and_retries_it() {
        let server = start_mock_server(vec![
            MockResponse::delayed_body_text(100, 200, "OK", r#"{"success":true}"#),
            MockResponse::json(200, r#"{"success":true}"#),
        ])
        .await;
        let client = build_client(server.base_url(), Duration::from_millis(10), 2);

        assert_eq!(
            client.scrape_json(&json_params(None)).await.unwrap(),
            json!({"success": true})
        );
        assert_eq!(server.requests().await.len(), 2);

        let server = start_mock_server(vec![MockResponse::delayed_body_text(
            100, 200, "OK", "too slow",
        )])
        .await;
        let client = build_client(server.base_url(), Duration::from_millis(10), 1);
        let params = WebScraperParams {
            url: "https://example.com".to_owned(),
            include_html: None,
            response_type: ResponseType::Markdown,
        };
        let error = client.scrape_markdown(&params).await.unwrap_err();
        assert!(matches!(error, ScrappaError::Timeout { .. }));
        assert_eq!(server.requests().await.len(), 1);
    }
}
