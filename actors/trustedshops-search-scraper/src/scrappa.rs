use anyhow::{Context, Result, anyhow};
use rand::Rng;
use reqwest::{Client, Response, Url, header};
use serde_json::{Map, Value};
use std::{error::Error, fmt, time::Duration};

pub const REQUEST_TIMEOUT_MS: u64 = 90_000;
pub const MAX_ATTEMPTS: usize = 3;
const USER_AGENT: &str = "thescrappa-trustedshops-search-scraper/1.0";

#[derive(Debug)]
pub struct ScrappaTimeoutError;

impl fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {REQUEST_TIMEOUT_MS}ms"
        )
    }
}

impl Error for ScrappaTimeoutError {}

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

impl Error for ScrappaApiError {}

pub struct ScrappaClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(REQUEST_TIMEOUT_MS))
            .build()
            .context("Could not create Scrappa HTTP client")?;
        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }

    pub async fn get(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        let mut last_error = None;
        for attempt in 1..=MAX_ATTEMPTS {
            match self.send(endpoint, params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt == MAX_ATTEMPTS || !is_retryable(&error) {
                        return Err(error);
                    }
                    let delay_ms = get_retry_delay_ms(attempt, rand::rng().random_range(0..=1000));
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{MAX_ATTEMPTS} in {delay_ms}ms.",
                        error,
                        attempt + 1
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    last_error = Some(error);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow!("Scrappa API request failed")))
    }

    async fn send(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        let mut url = Url::parse(&format!("{}{endpoint}", self.base_url))
            .context("Scrappa API base URL must be valid")?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                if value.is_null() || value.as_str() == Some("") {
                    continue;
                }
                if value.as_bool() == Some(false) {
                    continue;
                }
                let value = if value.as_bool() == Some(true) {
                    "1".to_owned()
                } else {
                    js_string(value)
                };
                query.append_pair(key, &value);
            }
        }

        let response = self
            .client
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let message = read_error_message(response).await?;
            return Err(ScrappaApiError {
                status: status.as_u16(),
                message,
            }
            .into());
        }
        response.json().await.map_err(map_request_error)
    }
}

pub fn get_retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    1_000u64
        .saturating_mul(2u64.saturating_pow(failed_attempt as u32))
        .saturating_add(jitter_ms)
        .min(10_000)
}

pub fn is_retryable(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if let Some(error) = error.downcast_ref::<ScrappaApiError>() {
        return matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504);
    }
    error.downcast_ref::<reqwest::Error>().is_some_and(|error| {
        error.is_timeout()
            || error.is_connect()
            || is_retryable_transport_message(&format!("{error:?}"))
    })
}

fn is_retryable_transport_message(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    [
        "econnreset",
        "econnrefused",
        "etimedout",
        "enotfound",
        "eai_again",
        "connection reset",
        "connection refused",
        "connection closed",
        "temporary failure in name resolution",
        "failed to lookup address",
        "name or service not known",
        "no such host",
        "dns error",
    ]
    .iter()
    .any(|cause| message.contains(cause))
}

async fn read_error_message(response: Response) -> Result<String> {
    let status = response.status();
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let body = response.text().await.map_err(map_request_error)?;
    if body.is_empty() {
        return Ok(fallback);
    }
    if let Some(message) = parse_json_error(&body, &fallback) {
        return Ok(message);
    }
    Ok(plain_text_message(&body))
}

fn plain_text_message(body: &str) -> String {
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn parse_json_error(body: &str, fallback: &str) -> Option<String> {
    let data: Value = serde_json::from_str(body).ok()?;
    let Some(object) = data.as_object() else {
        return Some(fallback.to_owned());
    };
    let mut message = object
        .get("message")
        .filter(|value| !value.is_null())
        .map(js_string)
        .unwrap_or_else(|| fallback.to_owned());
    if let Some(errors) = object.get("errors").filter(|value| js_truthy(value)) {
        let fields = errors.as_object()?;
        let mut details = Vec::with_capacity(fields.len());
        for (field, messages) in fields {
            let messages = messages.as_array()?;
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
    Some(message)
}

fn map_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaTimeoutError.into()
    } else {
        error.into()
    }
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
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn retry_policy_matches_transient_scrappa_failures() {
        let timeout = anyhow::Error::new(ScrappaTimeoutError);
        assert!(is_retryable(&timeout));

        for status in [408, 429, 500, 502, 503, 504] {
            let error = anyhow::Error::new(ScrappaApiError {
                status,
                message: "temporary".to_owned(),
            });
            assert!(is_retryable(&error), "status {status} should retry");
        }
        for status in [400, 401, 403, 404] {
            let error = anyhow::Error::new(ScrappaApiError {
                status,
                message: "permanent".to_owned(),
            });
            assert!(!is_retryable(&error), "status {status} should not retry");
        }
        assert!(!is_retryable(&anyhow::Error::msg("market must be valid")));
        for message in ["connection reset by peer", "ECONNREFUSED", "EAI_AGAIN"] {
            assert!(is_retryable_transport_message(message));
        }
        assert!(!is_retryable_transport_message(
            "error decoding response body"
        ));
    }

    #[test]
    fn retry_backoff_matches_actor_attempts_and_cap() {
        assert_eq!(get_retry_delay_ms(1, 0), 2_000);
        assert_eq!(get_retry_delay_ms(2, 999), 4_999);
        assert_eq!(get_retry_delay_ms(5, 999), 10_000);
    }

    #[test]
    fn formats_upstream_error_payloads_and_truncates_text() {
        assert_eq!(
            parse_json_error(
                r#"{"message":"Bad query","errors":{"q":["too short","required"]}}"#,
                "Bad Request"
            )
            .unwrap(),
            "Bad query - q: too short, required"
        );
        assert_eq!(
            parse_json_error("\"plain JSON string\"", "Bad Request").unwrap(),
            "Bad Request"
        );
        let long_text = format!("  {}  ", "x".repeat(600));
        assert_eq!(plain_text_message(&long_text), "x".repeat(500));
    }

    #[test]
    fn scrappa_timeout_error_matches_the_public_message() {
        assert_eq!(
            ScrappaTimeoutError.to_string(),
            "Scrappa API request timed out after 90000ms"
        );
    }

    #[test]
    fn query_parameters_encode_values_and_skip_empty_values() {
        let mut params = Map::new();
        params.insert("q".to_owned(), Value::String("H&M partner".to_owned()));
        params.insert("page".to_owned(), json!(2));
        params.insert("empty".to_owned(), Value::String(String::new()));
        params.insert("enabled".to_owned(), Value::Bool(true));
        params.insert("disabled".to_owned(), Value::Bool(false));
        let mut url = Url::parse("https://scrappa.co/api/trustedshops/search").unwrap();
        for (key, value) in &params {
            if value.is_null() || value.as_str() == Some("") || value.as_bool() == Some(false) {
                continue;
            }
            let value = if value.as_bool() == Some(true) {
                "1".to_owned()
            } else {
                js_string(value)
            };
            url.query_pairs_mut().append_pair(key, &value);
        }
        let query = url.query().unwrap();
        assert!(query.contains("q=H%26M+partner"));
        assert!(query.contains("page=2"));
        assert!(query.contains("enabled=1"));
        assert!(!query.contains("empty="));
        assert!(!query.contains("disabled="));
    }

    #[tokio::test]
    async fn retries_transient_http_status_and_sends_expected_scrappa_credentials() {
        use crate::test_support::{MockServer, response};

        let server = MockServer::start(vec![
            response(503, "temporarily unavailable"),
            response(200, r#"{"shops":[]}"#),
        ]);
        let client =
            ScrappaClient::new("test-api-key".to_owned(), server.base_url.clone()).unwrap();
        let mut params = Map::new();
        params.insert("q".to_owned(), Value::String("H&M partner".to_owned()));
        params.insert("market".to_owned(), Value::String("FRA".to_owned()));
        params.insert("page".to_owned(), Value::from(2));

        assert_eq!(
            client.get("/trustedshops/search", &params).await.unwrap(),
            json!({"shops": []})
        );

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        for request in requests {
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("x-api-key: test-api-key")
            );
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("accept: application/json")
            );
            assert!(request.contains(USER_AGENT));
            let path = request.split_whitespace().nth(1).unwrap();
            assert!(path.starts_with("/trustedshops/search?"));
            assert!(path.contains("q=H%26M+partner"));
            assert!(path.contains("market=FRA"));
            assert!(path.contains("page=2"));
        }
    }

    #[tokio::test]
    async fn does_not_retry_permanent_upstream_status() {
        use crate::test_support::{MockServer, response};

        let server = MockServer::start(vec![response(400, r#"{"message":"invalid query"}"#)]);
        let client =
            ScrappaClient::new("test-api-key".to_owned(), server.base_url.clone()).unwrap();
        let error = client
            .get("/trustedshops/search", &Map::new())
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "Scrappa API error (400): invalid query");
        assert_eq!(server.requests().len(), 1);
    }
}
