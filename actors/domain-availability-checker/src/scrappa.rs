use std::{fmt, time::Duration};

use rand::Rng;
use reqwest::{Client, Response, Url};
use serde_json::{Map, Value};

const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const SCRAPPA_RETRYABLE_STATUS_CODES: [u16; 6] = [408, 429, 500, 502, 503, 504];

#[derive(Debug)]
pub enum ScrappaError {
    Http { status: u16, message: String },
    Network(String),
    Timeout { timeout_ms: u64 },
    Other(String),
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::Network(message) => write!(formatter, "Scrappa API network error: {message}"),
            Self::Timeout { timeout_ms } => {
                write!(formatter, "Scrappa API request timed out after {timeout_ms}ms")
            }
            Self::Other(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ScrappaError {}

impl ScrappaError {
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http { status, .. } => SCRAPPA_RETRYABLE_STATUS_CODES.contains(status),
            Self::Network(_) | Self::Timeout { .. } => true,
            Self::Other(_) => false,
        }
    }

    pub fn is_service_failure(&self) -> bool {
        match self {
            Self::Timeout { .. } | Self::Network(_) => true,
            Self::Http { status, .. } => SCRAPPA_RETRYABLE_STATUS_CODES.contains(status),
            Self::Other(_) => false,
        }
    }

    pub fn is_per_domain_failure(&self) -> bool {
        matches!(self, Self::Http { status: 400 | 404 | 422, .. })
    }

    pub fn http_status(&self) -> Option<u16> {
        match self {
            Self::Http { status, .. } => Some(*status),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct ScrappaClient {
    client: Client,
    api_key: String,
    base_url: String,
    timeout: Duration,
    #[cfg(test)]
    retry_delay_override: Option<Duration>,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String, timeout: Duration) -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::builder().build()?,
            api_key,
            base_url: base_url.trim_end_matches('/').to_owned(),
            timeout,
            #[cfg(test)]
            retry_delay_override: None,
        })
    }

    pub async fn get_availability(&self, domain: &str) -> Result<Value, ScrappaError> {
        let mut url = Url::parse(&format!("{}/domains/availability", self.base_url))
            .map_err(|error| ScrappaError::Other(format!("Invalid Scrappa API URL: {error}")))?;
        url.query_pairs_mut().append_pair("domain", domain);

        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send(&url).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < SCRAPPA_MAX_ATTEMPTS && error.is_retryable() => {
                    let delay = self.retry_delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {}ms.",
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("the retry loop returns on its final attempt")
    }

    async fn send(&self, url: &Url) -> Result<Value, ScrappaError> {
        let response = self
            .client
            .get(url.clone())
            .timeout(self.timeout)
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                reqwest::header::USER_AGENT,
                "thescrappa-domain-availability-checker/1.0",
            )
            .send()
            .await
            .map_err(|error| self.request_error(error))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = self.read_error_message(response).await?;
            return Err(ScrappaError::Http { status, message });
        }

        response.json().await.map_err(|error| {
            if error.is_timeout() {
                self.timeout_error()
            } else {
                ScrappaError::Other(format!("Could not parse Scrappa API JSON response: {error}"))
            }
        })
    }

    async fn read_error_message(&self, response: Response) -> Result<String, ScrappaError> {
        let fallback = response
            .status()
            .canonical_reason()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("HTTP {}", response.status().as_u16()));
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) if error.is_timeout() => return Err(self.timeout_error()),
            Err(_) => return Ok(fallback),
        };
        if body.is_empty() {
            return Ok(fallback);
        }

        if let Ok(Value::Object(error_data)) = serde_json::from_str::<Value>(&body) {
            return Ok(format_json_error(&error_data, &fallback));
        }
        Ok(body.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(500).collect())
    }

    fn request_error(&self, error: reqwest::Error) -> ScrappaError {
        if error.is_timeout() {
            self.timeout_error()
        } else {
            ScrappaError::Network(error.to_string())
        }
    }

    fn timeout_error(&self) -> ScrappaError {
        ScrappaError::Timeout {
            timeout_ms: self.timeout.as_millis().min(u64::MAX as u128) as u64,
        }
    }

    fn retry_delay(&self, failed_attempt: usize) -> Duration {
        #[cfg(test)]
        if let Some(delay) = self.retry_delay_override {
            return delay;
        }
        let jitter_ms = rand::rng().random_range(0..1000);
        Duration::from_millis(get_retry_delay_ms(failed_attempt, jitter_ms))
    }

    #[cfg(test)]
    fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay_override = Some(delay);
        self
    }
}

pub fn get_retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let base = 1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt.min(63) as u32));
    base.saturating_add(jitter_ms).min(10_000)
}

fn format_json_error(error_data: &Map<String, Value>, fallback: &str) -> String {
    let mut message = error_data
        .get("message")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or_else(|| fallback.to_owned());
    if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .map(|(field, values)| {
                let values = values
                    .as_array()
                    .map(|values| values.iter().map(js_string).collect::<Vec<_>>().join(", "))
                    .unwrap_or_else(|| js_string(values));
                format!("{field}: {values}")
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
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
        thread,
    };
    use reqwest::StatusCode;

    struct MockServer {
        base_url: String,
        requests: Receiver<String>,
        thread: Option<thread::JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<(u16, String)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                for (status, body) in responses {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_request(&mut stream).unwrap();
                    sender.send(request).unwrap();
                    let reason = if status == 200 { "OK" } else { "Unavailable" };
                    write!(
                        stream,
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();
                }
            });
            Self {
                base_url: format!("http://{address}/api"),
                requests,
                thread: Some(thread),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.try_iter().collect()
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        Ok(String::from_utf8_lossy(&request).into_owned())
    }

    fn response(status: u16, body: &str) -> (u16, String) {
        (status, body.to_owned())
    }

    #[tokio::test]
    async fn sends_scrappa_auth_user_agent_and_retries_transient_responses() {
        let server = MockServer::start(vec![
            response(503, r#"{"message":"temporarily unavailable"}"#),
            response(200, r#"{"domain":"example.com"}"#),
        ]);
        let client = ScrappaClient::new(
            "test-key".to_owned(),
            server.base_url.clone(),
            Duration::from_secs(2),
        )
        .unwrap()
        .with_retry_delay(Duration::ZERO);

        assert_eq!(
            client.get_availability("example.com").await.unwrap(),
            serde_json::json!({"domain":"example.com"})
        );
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        for request in requests {
            assert!(request.starts_with("GET /api/domains/availability?domain=example.com "));
            assert!(request.to_ascii_lowercase().contains("x-api-key: test-key"));
            assert!(request.to_ascii_lowercase().contains("accept: application/json"));
            assert!(request
                .to_ascii_lowercase()
                .contains("user-agent: thescrappa-domain-availability-checker/1.0"));
        }
    }

    #[test]
    fn retries_only_transient_service_errors_with_the_expected_backoff() {
        assert!(ScrappaError::Http {
            status: 429,
            message: "Rate limited".to_owned()
        }
        .is_retryable());
        assert!(!ScrappaError::Http {
            status: 422,
            message: "Invalid request".to_owned()
        }
        .is_retryable());
        assert_eq!(get_retry_delay_ms(1, 50), 2050);
        assert_eq!(get_retry_delay_ms(2, 50), 4050);
        assert_eq!(get_retry_delay_ms(8, 500), 10_000);
    }

    #[test]
    fn classifies_item_and_service_failures() {
        let invalid = ScrappaError::Http {
            status: 422,
            message: "Invalid".to_owned(),
        };
        assert!(invalid.is_per_domain_failure());
        assert!(!invalid.is_service_failure());

        let unavailable = ScrappaError::Http {
            status: StatusCode::SERVICE_UNAVAILABLE.as_u16(),
            message: "Unavailable".to_owned(),
        };
        assert!(!unavailable.is_per_domain_failure());
        assert!(unavailable.is_service_failure());
        assert!(ScrappaError::Timeout { timeout_ms: 25_000 }.is_service_failure());
        assert!(ScrappaError::Network("connection reset".to_owned()).is_service_failure());
    }
}
