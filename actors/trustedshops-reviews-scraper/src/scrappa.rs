use reqwest::{Client, Url};
use serde_json::{Map, Value};
use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const REQUEST_TIMEOUT_MS: u64 = 90_000;
const MAX_ATTEMPTS: usize = 3;
const RETRY_BASE_DELAY_MS: u64 = 1_000;
const RETRY_JITTER_MS: u64 = 1_000;
const MAX_RETRY_DELAY_MS: u64 = 10_000;
const USER_AGENT: &str = "thescrappa-trustedshops-reviews-scraper/1.0";

pub struct ScrappaClient {
    client: Client,
    base_url: String,
    api_key: String,
    attempts: usize,
    retry_base_delay_ms: u64,
    retry_jitter_ms: u64,
}

#[derive(Debug)]
pub enum ScrappaError {
    Timeout,
    Api { status: u16, message: String },
    Transport(reqwest::Error),
    InvalidResponse(reqwest::Error),
    InvalidUrl(url::ParseError),
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(
                formatter,
                "Scrappa API request timed out after {REQUEST_TIMEOUT_MS}ms"
            ),
            Self::Api { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::Transport(error) => write!(formatter, "{error}"),
            Self::InvalidResponse(error) => write!(formatter, "{error}"),
            Self::InvalidUrl(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for ScrappaError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transport(error) | Self::InvalidResponse(error) => Some(error),
            Self::InvalidUrl(error) => Some(error),
            Self::Timeout | Self::Api { .. } => None,
        }
    }
}

impl ScrappaError {
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout => true,
            Self::Api { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::Transport(error) => error.is_timeout() || error.is_connect(),
            Self::InvalidResponse(_) | Self::InvalidUrl(_) => false,
        }
    }
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self, ScrappaError> {
        Self::with_policy(
            api_key,
            base_url,
            Duration::from_millis(REQUEST_TIMEOUT_MS),
            MAX_ATTEMPTS,
            RETRY_BASE_DELAY_MS,
            RETRY_JITTER_MS,
        )
    }

    fn with_policy(
        api_key: String,
        base_url: String,
        timeout: Duration,
        attempts: usize,
        retry_base_delay_ms: u64,
        retry_jitter_ms: u64,
    ) -> Result<Self, ScrappaError> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(ScrappaError::Transport)?;
        Ok(Self {
            client,
            base_url,
            api_key,
            attempts: attempts.max(1),
            retry_base_delay_ms,
            retry_jitter_ms,
        })
    }

    pub async fn get(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
    ) -> Result<Value, ScrappaError> {
        let mut last_error = None;
        for attempt in 1..=self.attempts {
            match self.send(endpoint, params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt >= self.attempts || !error.is_retryable() {
                        return Err(error);
                    }
                    let jitter = random_jitter_ms(self.retry_jitter_ms);
                    let delay = retry_delay_ms(attempt, self.retry_base_delay_ms, jitter);
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{}, in {delay}ms.",
                        attempt + 1,
                        self.attempts
                    );
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    last_error = Some(error);
                }
            }
        }
        Err(last_error.expect("at least one Scrappa request attempt is made"))
    }

    async fn send(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
    ) -> Result<Value, ScrappaError> {
        let base_url = self.base_url.trim_end_matches('/');
        let mut url =
            Url::parse(&format!("{base_url}{endpoint}")).map_err(ScrappaError::InvalidUrl)?;
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
                    continue;
                }
                query.append_pair(key, &js_string(value));
            }
        }

        let response = self
            .client
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(map_request_error)?;

        let status = response.status();
        if !status.is_success() {
            let fallback = status
                .canonical_reason()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            let body = response.text().await.map_err(map_request_error)?;
            return Err(ScrappaError::Api {
                status: status.as_u16(),
                message: api_error_message(&body, &fallback),
            });
        }
        response.json().await.map_err(map_request_error)
    }
}

fn retry_delay_ms(failed_attempt: usize, base_delay_ms: u64, jitter_ms: u64) -> u64 {
    (base_delay_ms
        .saturating_mul(2_u64.saturating_pow(failed_attempt as u32))
        .saturating_add(jitter_ms))
    .min(MAX_RETRY_DELAY_MS)
}

fn random_jitter_ms(maximum: u64) -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    nanos % maximum.max(1)
}

fn map_request_error(error: reqwest::Error) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout
    } else if error.is_decode() {
        ScrappaError::InvalidResponse(error)
    } else {
        ScrappaError::Transport(error)
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) => value
            .as_array()
            .unwrap()
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
        Value::Object(_) => "[object Object]".into(),
    }
}

fn api_error_message(body: &str, fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }
    if let Ok(data) = serde_json::from_str::<Value>(body) {
        let mut message = data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_owned();
        if let Some(errors) = data.get("errors").and_then(Value::as_object) {
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
        return message;
    }
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

#[cfg(test)]
fn retryable_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::mpsc,
        thread,
    };

    fn mock_server(responses: Vec<(u16, &'static str, u64)>) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            for (status, body, delay_ms) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut byte = [0; 1];
                while stream.read_exact(&mut byte).is_ok() {
                    request.push(byte[0]);
                    if request.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                sender
                    .send(String::from_utf8_lossy(&request).into_owned())
                    .unwrap();
                if delay_ms > 0 {
                    thread::sleep(Duration::from_millis(delay_ms));
                }
                let reason = match status {
                    200 => "OK",
                    400 => "Bad Request",
                    429 => "Too Many Requests",
                    503 => "Service Unavailable",
                    _ => "Mock Response",
                };
                write!(stream, "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        (format!("http://{address}"), receiver)
    }

    #[tokio::test]
    async fn retries_transient_api_errors_and_sends_the_expected_request() {
        let (base_url, requests) = mock_server(vec![
            (503, r#"{"message":"Service unavailable"}"#, 0),
            (429, r#"{"message":"Too many requests"}"#, 0),
            (200, r#"{"reviews":[]}"#, 0),
        ]);
        let client = ScrappaClient::with_policy(
            "test-key".into(),
            base_url,
            Duration::from_secs(2),
            3,
            0,
            0,
        )
        .unwrap();
        let params = serde_json::from_value::<Map<String, Value>>(
            json!({"page":2,"size":20,"market":"DEU","include":true,"skip":false}),
        )
        .unwrap();
        assert_eq!(
            client
                .get("/trustedshops/reviews/TSID", &params)
                .await
                .unwrap(),
            json!({"reviews":[]})
        );
        let requests = requests.try_iter().collect::<Vec<_>>();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].starts_with("GET /trustedshops/reviews/TSID?"));
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .contains("x-api-key: test-key")
        );
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .contains("user-agent: thescrappa-trustedshops-reviews-scraper/1.0")
        );
        assert!(requests[0].contains("include=1"));
        assert!(!requests[0].contains("skip="));
    }

    #[tokio::test]
    async fn does_not_retry_validation_errors_and_formats_api_details() {
        let (base_url, requests) = mock_server(vec![(
            400,
            r#"{"message":"Bad input","errors":{"page":["must be positive"]}}"#,
            0,
        )]);
        let client =
            ScrappaClient::with_policy("key".into(), base_url, Duration::from_secs(2), 3, 0, 0)
                .unwrap();
        let error = client.get("/reviews", &Map::new()).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (400): Bad input - page: must be positive"
        );
        assert!(!error.is_retryable());
        assert_eq!(requests.try_iter().count(), 1);
    }

    #[tokio::test]
    async fn maps_request_deadlines_to_retryable_timeouts() {
        let (base_url, _requests) = mock_server(vec![(200, "{}", 80)]);
        let client =
            ScrappaClient::with_policy("key".into(), base_url, Duration::from_millis(20), 1, 0, 0)
                .unwrap();
        let error = client.get("/reviews", &Map::new()).await.unwrap_err();
        assert!(error.is_timeout());
        assert_eq!(
            error.to_string(),
            "Scrappa API request timed out after 90000ms"
        );
        assert!(error.is_retryable());
    }

    #[test]
    fn uses_exponential_retry_delays_and_transient_statuses() {
        assert_eq!(retry_delay_ms(1, 1_000, 0), 2_000);
        assert_eq!(retry_delay_ms(2, 1_000, 1_000), 5_000);
        assert_eq!(retry_delay_ms(5, 1_000, 1_000), MAX_RETRY_DELAY_MS);
        assert!(retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(!retryable_status(StatusCode::BAD_REQUEST));
    }
}
