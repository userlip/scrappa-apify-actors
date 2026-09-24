use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use reqwest::{header, Client, StatusCode};
use serde_json::{Map, Value};
use url::Url;

use crate::config::endpoint_url;

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

pub(crate) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const ACTOR_USER_AGENT: &str = "thescrappa-google-finance-intraday-scraper/1.0";

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

#[derive(Debug)]
pub(crate) struct ScrappaTimeoutError {
    timeout_ms: u128,
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

impl Error for ScrappaTimeoutError {}

pub(crate) struct ScrappaClient<'a> {
    http: &'a Client,
    api_key: &'a str,
    base_url: &'a Url,
}

impl<'a> ScrappaClient<'a> {
    pub(crate) fn new(http: &'a Client, api_key: &'a str, base_url: &'a Url) -> Self {
        Self {
            http,
            api_key,
            base_url,
        }
    }

    pub(crate) async fn get_intraday(&self, params: &Map<String, Value>) -> Result<Value> {
        let mut url = endpoint_url(self.base_url, &["google-finance", "intraday"])?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                if value.is_null() || value.as_str() == Some("") {
                    continue;
                }
                if let Some(value) = value.as_bool() {
                    if value {
                        query.append_pair(key, "1");
                    }
                } else {
                    query.append_pair(key, &js_string(value));
                }
            }
        }

        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send_intraday(&url).await {
                Ok(response) => return Ok(response),
                Err(error)
                    if attempt < SCRAPPA_MAX_ATTEMPTS && is_retryable_scrappa_error(&error) =>
                {
                    let delay_ms = retry_delay_ms(attempt, random_jitter_ms());
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {delay_ms}ms.",
                        error,
                        attempt + 1,
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("retry loop always returns the final result")
    }

    async fn send_intraday(&self, url: &Url) -> Result<Value> {
        let response = self
            .http
            .get(url.clone())
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .header("X-API-Key", self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, ACTOR_USER_AGENT)
            .send()
            .await
            .map_err(scrappa_request_error)?;

        let status = response.status();
        let body = response.text().await.map_err(scrappa_request_error)?;
        if !status.is_success() {
            return Err(scrappa_api_error(status, &body).into());
        }
        serde_json::from_str(&body)
            .map_err(|error| anyhow!("Scrappa API response was not valid JSON: {error}"))
    }
}

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaTimeoutError {
            timeout_ms: SCRAPPA_REQUEST_TIMEOUT.as_millis(),
        }
        .into()
    } else {
        anyhow!(error)
    }
}

fn scrappa_api_error(status: StatusCode, body: &str) -> ScrappaApiError {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let Ok(data) = serde_json::from_str::<Value>(body) else {
        return ScrappaApiError {
            status: status.as_u16(),
            message: if body.is_empty() {
                fallback
            } else {
                body.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(500)
                    .collect()
            },
        };
    };
    let Some(object) = data.as_object() else {
        return ScrappaApiError {
            status: status.as_u16(),
            message: body.to_owned(),
        };
    };
    let mut message = object
        .get("message")
        .filter(|value| !value.is_null())
        .or_else(|| object.get("error").filter(|value| !value.is_null()))
        .map(js_string)
        .unwrap_or(fallback);
    if let Some(errors) = object.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .map(|(field, messages)| {
                let messages = match messages.as_array() {
                    Some(messages) => messages
                        .iter()
                        .map(js_string)
                        .collect::<Vec<_>>()
                        .join(", "),
                    None => js_string(messages),
                };
                format!("{field}: {messages}")
            })
            .collect::<Vec<_>>();
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details.join("; "));
        }
    }
    ScrappaApiError {
        status: status.as_u16(),
        message,
    }
}

fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if let Some(api_error) = error.downcast_ref::<ScrappaApiError>() {
        return matches!(api_error.status, 408 | 429 | 500 | 502 | 503 | 504);
    }
    error.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|error| error.is_timeout() || error.is_connect() || error.is_body())
    })
}

fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponent = u32::try_from(failed_attempt).unwrap_or(u32::MAX);
    let backoff = 1_000u64.saturating_mul(1u64.checked_shl(exponent).unwrap_or(u64::MAX));
    backoff.saturating_add(jitter_ms).min(10_000)
}

fn random_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
        % 1_000
}

pub(crate) fn is_no_data_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| error.status == StatusCode::NOT_FOUND.as_u16())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::mpsc,
        thread,
    };

    #[tokio::test]
    async fn sends_authenticated_intraday_request_to_scrappa_api() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, request_receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut byte = [0; 1];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            request_sender
                .send(String::from_utf8(request).unwrap())
                .unwrap();

            let body = r#"{"graph":[]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let base_url = Url::parse(&format!("http://{address}/api")).unwrap();
        let http = Client::new();
        let params = json!({
            "symbol": "AAPL",
            "exchange": "NASDAQ",
            "hl": "en",
            "gl": "us",
        });
        let response = ScrappaClient::new(&http, "test-scrappa-key", &base_url)
            .get_intraday(params.as_object().unwrap())
            .await
            .unwrap();

        assert_eq!(response["graph"], json!([]));
        let request = request_receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .to_ascii_lowercase();
        assert!(request.starts_with("get /api/google-finance/intraday?"));
        for query_param in ["symbol=aapl", "exchange=nasdaq", "hl=en", "gl=us"] {
            assert!(
                request.contains(query_param),
                "missing {query_param} in request: {request}"
            );
        }
        assert!(request.contains("x-api-key: test-scrappa-key"));
        assert!(request.contains("accept: application/json"));
        assert!(request.contains(ACTOR_USER_AGENT));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn retries_a_transient_scrappa_error() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, request_receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            for (status, reason, body) in [
                (503, "Service Unavailable", r#"{"error":"retry"}"#),
                (200, "OK", r#"{"graph":[]}"#),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut byte = [0; 1];
                while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                }
                request_sender
                    .send(String::from_utf8(request).unwrap())
                    .unwrap();
                write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });

        let base_url = Url::parse(&format!("http://{address}/api")).unwrap();
        let http = Client::new();
        let params = json!({ "symbol": "AAPL" });
        let response = ScrappaClient::new(&http, "test-scrappa-key", &base_url)
            .get_intraday(params.as_object().unwrap())
            .await
            .unwrap();

        assert_eq!(response["graph"], json!([]));
        for _ in 0..2 {
            let request = request_receiver
                .recv_timeout(Duration::from_secs(2))
                .unwrap();
            assert!(request.starts_with("GET /api/google-finance/intraday?symbol=AAPL"));
        }
        server.join().unwrap();
    }

    #[test]
    fn formats_upstream_errors_and_retry_policy() {
        assert_eq!(
            scrappa_api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                r#"{"error":"Intraday data is temporarily unavailable. Please retry."}"#,
            )
            .to_string(),
            "Scrappa API error (503): Intraday data is temporarily unavailable. Please retry."
        );
        assert_eq!(
            scrappa_api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                r#"{"message":"Invalid request","errors":{"symbol":["The stock symbol is required."]}}"#,
            )
            .to_string(),
            "Scrappa API error (422): Invalid request - symbol: The stock symbol is required."
        );
        assert!(is_retryable_scrappa_error(
            &ScrappaTimeoutError { timeout_ms: 60_000 }.into()
        ));
        assert!(is_retryable_scrappa_error(
            &scrappa_api_error(StatusCode::TOO_MANY_REQUESTS, "rate limited").into()
        ));
        assert!(is_retryable_scrappa_error(
            &scrappa_api_error(StatusCode::BAD_GATEWAY, "upstream failed").into()
        ));
        assert!(!is_retryable_scrappa_error(
            &scrappa_api_error(StatusCode::NOT_FOUND, "no data").into()
        ));
        assert!(!is_retryable_scrappa_error(
            &scrappa_api_error(StatusCode::UNPROCESSABLE_ENTITY, "invalid").into()
        ));
        assert_eq!(retry_delay_ms(1, 0), 2_000);
        assert_eq!(retry_delay_ms(2, 500), 4_500);
        assert_eq!(retry_delay_ms(20, 0), 10_000);
    }
}
