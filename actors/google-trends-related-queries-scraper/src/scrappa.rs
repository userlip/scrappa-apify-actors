use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result, anyhow};
use reqwest::{Client, Response, header};
use serde_json::{Map, Value};
use url::Url;

const SCRAPPA_USER_AGENT: &str = "thescrappa-google-trends-related-queries-scraper/1.0";

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    timeout_ms: u64,
}

impl ScrappaTimeoutError {
    #[cfg(test)]
    pub fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
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

impl Error for ScrappaTimeoutError {}

#[derive(Debug)]
struct ScrappaApiError {
    status: u16,
    message: String,
    retry_after_ms: Option<u64>,
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
    http: Client,
    api_key: String,
    base_url: Url,
}

#[derive(Clone, Copy)]
pub struct RetryPolicy {
    pub timeout: Duration,
    pub attempts: usize,
    pub max_retry_delay: Duration,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: Url) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .build()
                .context("Failed to initialize Scrappa HTTP client")?,
            api_key,
            base_url,
        })
    }

    pub async fn get_json(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
        policy: RetryPolicy,
    ) -> Result<Value> {
        let attempts = policy.attempts.max(1);
        let mut last_error = None;

        for attempt in 1..=attempts {
            match self.get_once(endpoint, params, policy.timeout).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let retry_after_ms = error
                        .downcast_ref::<ScrappaApiError>()
                        .and_then(|error| error.retry_after_ms);
                    let retryable = is_retryable(&error);
                    last_error = Some(error);
                    if attempt >= attempts || !retryable {
                        break;
                    }

                    let jitter_ms = retry_jitter_ms();
                    let delay_ms = retry_delay_ms(
                        attempt,
                        jitter_ms,
                        retry_after_ms,
                        policy.max_retry_delay.as_millis().min(u64::MAX as u128) as u64,
                    );
                    let error = last_error.as_ref().expect("the error was just saved");
                    println!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{attempts} in {delay_ms}ms.",
                        attempt + 1
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Scrappa API request did not run")))
    }

    async fn get_once(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
        timeout: Duration,
    ) -> Result<Value> {
        let url = build_request_url(&self.base_url, endpoint, params)?;
        let response = self
            .http
            .get(url)
            .timeout(timeout)
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, SCRAPPA_USER_AGENT)
            .send()
            .await
            .map_err(|error| request_error(error, timeout))?;
        parse_json_response(response, timeout).await
    }
}

fn build_request_url(base_url: &Url, endpoint: &str, params: &Map<String, Value>) -> Result<Url> {
    let mut url = base_url.clone();
    let segments = endpoint
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    url.path_segments_mut()
        .map_err(|_| anyhow!("Scrappa API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
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
    Ok(url)
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value
            .as_f64()
            .filter(|number| number.fract() == 0.0 && number.abs() < 1e21)
            .map(|number| format!("{number:.0}"))
            .unwrap_or_else(|| value.to_string()),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

async fn parse_json_response(response: Response, timeout: Duration) -> Result<Value> {
    let status = response.status();
    let retry_after_ms = parse_retry_after_ms(
        response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok()),
    );
    let body = response
        .text()
        .await
        .map_err(|error| request_error(error, timeout))?;

    if !status.is_success() {
        let fallback = status.canonical_reason().unwrap_or_else(|| "HTTP status");
        let message = parse_api_error_message(&body, fallback, status.as_u16());
        return Err(ScrappaApiError {
            status: status.as_u16(),
            message,
            retry_after_ms,
        }
        .into());
    }

    serde_json::from_str(&body)
        .map_err(|error| anyhow!("Scrappa API response was not valid JSON: {error}"))
}

fn request_error(error: reqwest::Error, timeout: Duration) -> anyhow::Error {
    if error.is_timeout() {
        anyhow::Error::new(ScrappaTimeoutError {
            timeout_ms: timeout.as_millis().min(u64::MAX as u128) as u64,
        })
    } else {
        anyhow!(error.to_string())
    }
}

fn parse_api_error_message(body: &str, fallback: &str, status: u16) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if value.is_null() {
            return body.to_owned();
        }
        let mut message = value
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or_else(|| fallback.to_owned());
        if let Some(errors) = value.get("errors").filter(|value| json_truthy(value)) {
            let Some(errors) = errors.as_object() else {
                return body.to_owned();
            };
            let mut details = Vec::with_capacity(errors.len());
            for (field, messages) in errors {
                let Some(messages) = messages.as_array() else {
                    return body.to_owned();
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
        return message;
    }

    if body.is_empty() {
        return format!("HTTP {status}");
    }
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn json_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn parse_retry_after_ms(value: Option<&str>) -> Option<u64> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    if let Ok(seconds) = value.parse::<f64>() {
        if seconds.is_finite() && seconds >= 0.0 {
            return Some((seconds * 1_000.0).min(u64::MAX as f64) as u64);
        }
    }
    let retry_at = httpdate::parse_http_date(value).ok()?;
    Some(
        retry_at
            .duration_since(SystemTime::now())
            .unwrap_or_default()
            .as_millis()
            .min(u64::MAX as u128) as u64,
    )
}

pub fn is_retryable(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504))
}

pub fn retry_delay_ms(
    failed_attempt: usize,
    jitter_ms: u64,
    retry_after_ms: Option<u64>,
    max_delay_ms: u64,
) -> u64 {
    let exponent = failed_attempt.min(63) as u32;
    let exponential = 1_000_u64
        .saturating_mul(2_u64.saturating_pow(exponent))
        .saturating_add(jitter_ms);
    exponential
        .max(retry_after_ms.unwrap_or(0))
        .min(max_delay_ms)
}

fn retry_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
        % 1_000
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    fn mock_server(
        responses: Vec<(&'static str, &'static str, &'static str)>,
    ) -> (Url, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, headers, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                requests.push(read_request(&mut stream));
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
            requests
        });
        (
            Url::parse(&format!("http://{address}/api")).unwrap(),
            server,
        )
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            request.push_str(&line);
        }
        let length = request
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        request.push_str(&String::from_utf8_lossy(&body));
        request
    }

    #[test]
    fn retry_delays_match_exponential_and_retry_after_caps() {
        assert_eq!(retry_delay_ms(1, 0, None, 60_000), 2_000);
        assert_eq!(retry_delay_ms(2, 250, None, 60_000), 4_250);
        assert_eq!(retry_delay_ms(1, 0, Some(30_000), 30_000), 30_000);
        assert_eq!(retry_delay_ms(10, 0, Some(60_000), 20_000), 20_000);
        assert_eq!(parse_retry_after_ms(Some("1.5")), Some(1_500));
    }

    #[test]
    fn retries_only_timeouts_and_transient_http_statuses() {
        for status in [408, 429, 500, 502, 503, 504] {
            let error: anyhow::Error = ScrappaApiError {
                status,
                message: "temporary".to_owned(),
                retry_after_ms: None,
            }
            .into();
            assert!(is_retryable(&error));
        }
        for status in [400, 401, 403, 404, 422] {
            let error: anyhow::Error = ScrappaApiError {
                status,
                message: "permanent".to_owned(),
                retry_after_ms: None,
            }
            .into();
            assert!(!is_retryable(&error));
        }
        assert!(is_retryable(&anyhow::Error::new(ScrappaTimeoutError::new(
            1_000
        ))));
    }

    #[tokio::test]
    async fn sends_authenticated_get_and_retries_retryable_status() {
        let (base_url, server) = mock_server(vec![
            (
                "503 Service Unavailable",
                "Retry-After: 0\r\n",
                r#"{"message":"temporarily busy"}"#,
            ),
            ("200 OK", "", r#"{"ok":true}"#),
        ]);
        let client = ScrappaClient::new("test-key".to_owned(), base_url.clone()).unwrap();
        let params = serde_json::from_value(
            json!({"q":"coffee", "geo":"US", "include_cache":true, "skip_cache":false, "empty":""}),
        )
        .unwrap();
        let response = client
            .get_json(
                "/google-trends/related",
                &params,
                RetryPolicy {
                    timeout: Duration::from_secs(2),
                    attempts: 2,
                    max_retry_delay: Duration::ZERO,
                },
            )
            .await
            .unwrap();
        assert_eq!(response, json!({"ok":true}));

        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("GET /api/google-trends/related?"));
        let headers = requests[0].to_ascii_lowercase();
        assert!(headers.contains("x-api-key: test-key\r\n"));
        assert!(
            headers
                .contains("user-agent: thescrappa-google-trends-related-queries-scraper/1.0\r\n")
        );
        let url = Url::parse(&format!(
            "http://test{}",
            requests[0]
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
        ))
        .unwrap();
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "q").unwrap().1,
            "coffee"
        );
        assert_eq!(
            url.query_pairs()
                .find(|(key, _)| key == "include_cache")
                .unwrap()
                .1,
            "1"
        );
        assert!(
            url.query_pairs()
                .all(|(key, _)| key != "skip_cache" && key != "empty")
        );
    }

    #[tokio::test]
    async fn keeps_validation_and_auth_errors_non_retryable() {
        let (base_url, server) =
            mock_server(vec![("401 Unauthorized", "", r#"{"message":"no access"}"#)]);
        let client = ScrappaClient::new("test-key".to_owned(), base_url).unwrap();
        let error = client
            .get_json(
                "/not-authorized",
                &Map::new(),
                RetryPolicy {
                    timeout: Duration::from_secs(2),
                    attempts: 4,
                    max_retry_delay: Duration::ZERO,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "Scrappa API error (401): no access");
        assert!(!is_retryable(&error));
        assert_eq!(server.join().unwrap().len(), 1);
    }
}
