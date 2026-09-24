use anyhow::{Context, Result, anyhow};
use reqwest::{Client, Response, StatusCode, Url};
use serde_json::Value;
use std::{error::Error, fmt, time::Duration};

use crate::request_params::RequestParams;

pub const REQUEST_TIMEOUT_MS: u64 = 90_000;
pub const REQUEST_DEADLINE_MS: u64 = 180_000;
const REVIEWS_ENDPOINT: &str = "/kununu/reviews";
const MAX_ATTEMPTS: usize = 4;
const RETRY_BACKOFF_MS: u64 = 500;

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    timeout: Duration,
}

impl ScrappaTimeoutError {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub fn timeout_seconds(&self) -> u64 {
        self.timeout.as_secs()
    }
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

impl Error for ScrappaTimeoutError {}

pub struct ScrappaClient {
    client: Client,
    base_url: String,
    api_key: String,
    request_deadline: Duration,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self> {
        Self::with_timeout(api_key, base_url, Duration::from_millis(REQUEST_TIMEOUT_MS))
    }

    fn with_timeout(api_key: String, base_url: String, timeout: Duration) -> Result<Self> {
        Self::with_timeouts(
            api_key,
            base_url,
            timeout,
            Duration::from_millis(REQUEST_DEADLINE_MS),
        )
    }

    fn with_timeouts(
        api_key: String,
        base_url: String,
        timeout: Duration,
        request_deadline: Duration,
    ) -> Result<Self> {
        let client = Client::builder().timeout(timeout).build()?;
        Ok(Self {
            client,
            base_url,
            api_key,
            request_deadline,
        })
    }

    pub async fn get(&self, params: &RequestParams) -> Result<Value> {
        let mut url = Url::parse(&format!(
            "{}{}",
            self.base_url.trim_end_matches('/'),
            REVIEWS_ENDPOINT
        ))
        .context("Scrappa API base URL must be valid")?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                if value.is_null() || value.as_str() == Some("") {
                    continue;
                }
                if let Value::Array(values) = value {
                    for value in values {
                        if !value.is_null() && value.as_str() != Some("") {
                            query.append_pair(&format!("{key}[]"), &js_string(value));
                        }
                    }
                    continue;
                }
                query.append_pair(key, &js_string(value));
            }
        }

        let request = async {
            for attempt in 0..MAX_ATTEMPTS {
                let response = self
                    .client
                    .get(url.clone())
                    .header("X-API-Key", &self.api_key)
                    .header(reqwest::header::ACCEPT, "application/json")
                    .send()
                    .await;

                let result = match response {
                    Ok(response)
                        if is_retryable_status(response.status()) && attempt + 1 < MAX_ATTEMPTS =>
                    {
                        tokio::time::sleep(retry_delay(attempt)).await;
                        continue;
                    }
                    Ok(response) => response_json(response).await,
                    Err(error) => Err(map_request_error(error)),
                };

                match result {
                    Ok(value) => return Ok(value),
                    Err(error)
                        if attempt + 1 < MAX_ATTEMPTS && is_retryable_request_error(&error) =>
                    {
                        tokio::time::sleep(retry_delay(attempt)).await;
                    }
                    Err(error) => return Err(error),
                }
            }

            unreachable!("Scrappa request attempts are bounded above zero")
        };

        match tokio::time::timeout(self.request_deadline, request).await {
            Ok(result) => result,
            Err(_) => Err(ScrappaTimeoutError::new(self.request_deadline).into()),
        }
    }
}

fn map_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaTimeoutError::new(Duration::from_millis(REQUEST_TIMEOUT_MS)).into()
    } else {
        error.into()
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn is_retryable_request_error(error: &anyhow::Error) -> bool {
    error.downcast_ref::<ScrappaTimeoutError>().is_some()
        || error
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|error| error.is_connect() || error.is_timeout())
}

fn retry_delay(retry: usize) -> Duration {
    Duration::from_millis(RETRY_BACKOFF_MS * 2_u64.saturating_pow(retry as u32))
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::Null => String::new(),
                value => js_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

async fn response_json(response: Response) -> Result<Value> {
    let status = response.status();
    let body = response.text().await.map_err(map_request_error)?;
    if !status.is_success() {
        return Err(anyhow!(format_api_error(status, &body)));
    }
    serde_json::from_str(&body).context("Scrappa API returned invalid JSON")
}

fn format_api_error(status: StatusCode, body: &str) -> String {
    let status_code = status.as_u16();
    let message = match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(data)) => {
            let mut message = data
                .get("message")
                .filter(|value| !value.is_null())
                .map(js_string)
                .unwrap_or_else(|| format!("HTTP {status_code}"));
            if let Some(errors) = data.get("errors").filter(|errors| is_truthy(errors)) {
                let Some(errors) = errors.as_object() else {
                    return format!("Scrappa API error ({status_code}): {}", body);
                };
                let mut details = Vec::with_capacity(errors.len());
                for (field, messages) in errors {
                    let Some(messages) = messages.as_array() else {
                        return format!("Scrappa API error ({status_code}): {}", body);
                    };
                    let messages = messages
                        .iter()
                        .map(js_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    details.push(format!("{field}: {messages}"));
                }
                if !details.is_empty() {
                    message.push_str(" - ");
                    message.push_str(&details.join("; "));
                }
            }
            message
        }
        Ok(Value::Null) => body.to_owned(),
        Ok(_) => format!("HTTP {status_code}"),
        Err(_) if body.is_empty() => format!("HTTP {status_code}"),
        Err(_) => body.to_owned(),
    };
    format!("Scrappa API error ({status_code}): {message}")
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        REQUEST_DEADLINE_MS, REQUEST_TIMEOUT_MS, ScrappaClient, ScrappaTimeoutError,
        format_api_error,
    };
    use crate::{request_params::build_request_plan, test_support::*};
    use reqwest::{StatusCode, Url};
    use serde_json::{Map, json};
    use std::time::Duration;

    fn client(base_url: String) -> ScrappaClient {
        ScrappaClient::new("test-api-key".into(), base_url).unwrap()
    }

    #[tokio::test]
    async fn encodes_array_query_params_and_scrappa_authentication() {
        let server = MockServer::start(vec![response(200, json!({"success": true}))]);
        let plan = build_request_plan(&json!({
            "targets": ["de/bmwgroup"],
            "score_filters": ["excellent", "good"]
        }))
        .unwrap();
        let params = crate::request_params::page_params(&plan, &plan.targets[0], 1);
        let result = client(server.base_url.clone()).get(&params).await.unwrap();
        assert_eq!(result["success"], true);

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        let (method, path, _) = request_parts(&requests[0]);
        assert_eq!(method, "GET");
        assert!(path.starts_with("/kununu/reviews?"));
        assert_eq!(
            header(&requests[0], "X-API-Key").as_deref(),
            Some("test-api-key")
        );
        assert_eq!(
            header(&requests[0], "Accept").as_deref(),
            Some("application/json")
        );
        let raw_url = format!("{}{}", server.base_url, path);
        let url = Url::parse(&raw_url).unwrap();
        let query = url.query_pairs().into_owned().collect::<Vec<_>>();
        assert!(query.contains(&("country".into(), "de".into())));
        assert!(query.contains(&("company_slug".into(), "bmwgroup".into())));
        assert!(query.contains(&("page".into(), "1".into())));
        assert_eq!(
            query
                .iter()
                .filter(|(key, _)| key == "score_filters[]")
                .map(|(_, value)| value.clone())
                .collect::<Vec<_>>(),
            ["excellent", "good"]
        );
    }

    #[tokio::test]
    async fn formats_scrappa_validation_errors_without_retrying_permanent_4xx() {
        let server = MockServer::start(vec![response(
            422,
            json!({"message":"Invalid input", "errors":{"score_filters":["must be valid"]}}),
        )]);
        let error = client(server.base_url.clone())
            .get(&Map::new())
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (422): Invalid input - score_filters: must be valid"
        );
        assert_eq!(server.requests().len(), 1);

        assert_eq!(REQUEST_TIMEOUT_MS, 90_000);
        assert_eq!(REQUEST_DEADLINE_MS, 180_000);
        assert!(REQUEST_DEADLINE_MS < 300_000);
        assert_eq!(
            ScrappaTimeoutError::new(Duration::from_millis(REQUEST_TIMEOUT_MS)).to_string(),
            "Scrappa API request timed out after 90000ms"
        );
    }

    #[tokio::test]
    async fn retries_service_unavailable_then_returns_success() {
        let server = MockServer::start(vec![
            response(503, json!({"message":"temporarily unavailable"})),
            response(200, json!({"success": true})),
        ]);

        let result = client(server.base_url.clone())
            .get(&Map::new())
            .await
            .unwrap();

        assert_eq!(result["success"], true);
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert!(
            requests
                .iter()
                .all(|request| request_parts(request).0 == "GET")
        );
    }

    #[tokio::test]
    async fn does_not_retry_permanent_client_errors() {
        let server = MockServer::start(vec![response(400, json!({"message":"invalid request"}))]);

        let error = client(server.base_url.clone())
            .get(&Map::new())
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "Scrappa API error (400): invalid request"
        );
        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(request_parts(&requests[0]).0, "GET");
    }

    #[test]
    fn retries_only_the_transient_http_status_ranges() {
        assert!(super::is_retryable_status(StatusCode::REQUEST_TIMEOUT));
        assert!(super::is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(super::is_retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(!super::is_retryable_status(StatusCode::BAD_REQUEST));
        assert!(!super::is_retryable_status(
            StatusCode::UNPROCESSABLE_ENTITY
        ));
    }

    #[tokio::test]
    async fn request_timeout_is_classified_as_the_scrappa_timeout_error() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server_thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = read_timeout_request(&mut stream);
            std::thread::sleep(Duration::from_millis(100));
        });
        let client = ScrappaClient::with_timeouts(
            "test-api-key".into(),
            format!("http://{address}"),
            Duration::from_secs(1),
            Duration::from_millis(10),
        )
        .unwrap();
        let error = client.get(&Map::new()).await.unwrap_err();
        assert!(error.downcast_ref::<ScrappaTimeoutError>().is_some());
        server_thread.join().unwrap();
    }

    #[test]
    fn plain_text_api_errors_include_the_status_and_body() {
        assert_eq!(
            format_api_error(StatusCode::BAD_GATEWAY, "upstream unavailable"),
            "Scrappa API error (502): upstream unavailable"
        );
    }

    fn read_timeout_request(stream: &mut std::net::TcpStream) -> std::io::Result<()> {
        use std::io::Read;
        let mut buffer = [0; 2048];
        let _ = stream.read(&mut buffer)?;
        Ok(())
    }
}
