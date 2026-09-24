use anyhow::Result;
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::sleep;
use url::Url;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_ATTEMPTS: u32 = 3;

#[derive(Debug)]
pub enum ScrappaError {
    Timeout,
    Http { status: StatusCode, message: String },
    Transport(String),
    InvalidJson(String),
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(
                formatter,
                "Scrappa API request timed out after {}ms",
                REQUEST_TIMEOUT.as_millis()
            ),
            Self::Http { status, message } => {
                write!(
                    formatter,
                    "Scrappa API error ({}): {message}",
                    status.as_u16()
                )
            }
            Self::Transport(message) | Self::InvalidJson(message) => formatter.write_str(message),
        }
    }
}

impl Error for ScrappaError {}

pub async fn get_jobs_response(
    client: &Client,
    base_url: &Url,
    api_key: &str,
    params: &[(String, Value)],
) -> Result<Value> {
    let url = crate::indeed::build_jobs_url(base_url, params)?;
    get_with_retry(client, &url, api_key, MAX_ATTEMPTS, REQUEST_TIMEOUT)
        .await
        .map_err(Into::into)
}

async fn get_with_retry(
    client: &Client,
    url: &Url,
    api_key: &str,
    attempts: u32,
    timeout: Duration,
) -> std::result::Result<Value, ScrappaError> {
    let attempts = attempts.max(1);
    for attempt in 1..=attempts {
        match send_once(client, url, api_key, timeout).await {
            Ok(response) => return Ok(response),
            Err(error) if attempt < attempts && is_retryable(&error) => {
                let delay = retry_delay(attempt, random_jitter_ms());
                eprintln!(
                    "Scrappa API request failed ({error}). Retrying attempt {}/{attempts} in {}ms.",
                    attempt + 1,
                    delay.as_millis()
                );
                sleep(delay).await;
            }
            Err(error) => return Err(error),
        }
    }

    unreachable!("the retry loop always returns or fails")
}

async fn send_once(
    client: &Client,
    url: &Url,
    api_key: &str,
    timeout: Duration,
) -> std::result::Result<Value, ScrappaError> {
    let response = client
        .get(url.clone())
        .header("X-API-Key", api_key)
        .header("Accept", "application/json")
        .header("User-Agent", "thescrappa-indeed-jobs-scraper/1.0")
        .timeout(timeout)
        .send()
        .await
        .map_err(map_reqwest_error)?;

    let status = response.status();
    let body = if status.is_success() {
        response.text().await.map_err(map_reqwest_error)?
    } else {
        read_error_body(response).await
    };

    if !status.is_success() {
        let message = error_message(status, &body);
        return Err(ScrappaError::Http { status, message });
    }

    serde_json::from_str(&body).map_err(|error| {
        ScrappaError::InvalidJson(format!("Scrappa API response was not valid JSON: {error}"))
    })
}

async fn read_error_body(response: Response) -> String {
    response.text().await.unwrap_or_default()
}

fn map_reqwest_error(error: reqwest::Error) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout
    } else {
        ScrappaError::Transport(error.to_string())
    }
}

fn error_message(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    if body.is_empty() {
        return fallback;
    }

    if let Ok(Value::Object(error)) = serde_json::from_str::<Value>(body) {
        let mut message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(&fallback)
            .to_owned();
        if let Some(errors) = error.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    let messages = messages.as_array()?;
                    let messages = messages
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ");
                    (!messages.is_empty()).then(|| format!("{field}: {messages}"))
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

fn is_retryable(error: &ScrappaError) -> bool {
    match error {
        ScrappaError::Timeout => true,
        ScrappaError::Http { status, .. } => matches!(
            *status,
            StatusCode::REQUEST_TIMEOUT
                | StatusCode::TOO_MANY_REQUESTS
                | StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT
        ),
        ScrappaError::Transport(_) | ScrappaError::InvalidJson(_) => false,
    }
}

fn retry_delay(failed_attempt: u32, jitter_ms: u64) -> Duration {
    let exponential_ms = 1_000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt));
    Duration::from_millis(exponential_ms.saturating_add(jitter_ms).min(10_000))
}

fn random_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis()
        .into()
}

pub fn timeout_message() -> String {
    format!(
        "Scrappa API request timed out after {}ms. The Indeed Jobs request exceeded the {}s Scrappa API timeout. Try again or refine the query.",
        REQUEST_TIMEOUT.as_millis(),
        REQUEST_TIMEOUT.as_secs()
    )
}

pub fn is_timeout(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaError>()
        .is_some_and(|error| matches!(error, ScrappaError::Timeout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        sync::{Arc, Mutex},
        thread::{self, JoinHandle},
    };

    struct MockServer {
        base_url: Url,
        requests: Arc<Mutex<Vec<String>>>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<(u16, &'static str)>) -> Self {
            Self::start_with_delays(responses, vec![Duration::ZERO; 8])
        }

        fn start_with_delays(responses: Vec<(u16, &'static str)>, delays: Vec<Duration>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let captured_requests = Arc::clone(&requests);
            let thread = thread::spawn(move || {
                for (index, (status, body)) in responses.into_iter().enumerate() {
                    let started = std::time::Instant::now();
                    let mut stream = loop {
                        match listener.accept() {
                            Ok((stream, _)) => break stream,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                if started.elapsed() > Duration::from_secs(3) {
                                    return;
                                }
                                thread::sleep(Duration::from_millis(2));
                            }
                            Err(_) => return,
                        }
                    };
                    let request = read_request(&mut stream);
                    captured_requests.lock().unwrap().push(request);
                    if let Some(delay) = delays.get(index) {
                        thread::sleep(*delay);
                    }
                    let reason = StatusCode::from_u16(status)
                        .ok()
                        .and_then(|status| status.canonical_reason())
                        .unwrap_or("OK");
                    let response = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
            });

            Self {
                base_url: Url::parse(&format!("http://{address}")).unwrap(),
                requests,
                thread: Some(thread),
            }
        }

        fn finish(mut self) -> Vec<String> {
            if let Some(thread) = self.thread.take() {
                thread.join().unwrap();
            }
            self.requests.lock().unwrap().clone()
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        let _ = reader.read_line(&mut request);
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            let header = line.trim_end();
            request.push_str(&line);
            if header.is_empty() {
                break;
            }
        }
        let content_length = request
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|length| length.trim().parse::<usize>().ok())
            })
            .unwrap_or(0);
        let mut body = vec![0; content_length];
        let _ = reader.read_exact(&mut body);
        request.push_str(&String::from_utf8_lossy(&body));
        request
    }

    #[test]
    fn retry_delay_matches_the_original_backoff() {
        assert_eq!(retry_delay(1, 125), Duration::from_millis(2_125));
        assert_eq!(retry_delay(2, 999), Duration::from_millis(4_999));
        assert_eq!(retry_delay(9, 999), Duration::from_millis(10_000));
    }

    #[test]
    fn retries_only_the_configured_status_codes_and_timeouts() {
        for code in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable(&ScrappaError::Http {
                status: StatusCode::from_u16(code).unwrap(),
                message: String::new()
            }));
        }
        assert!(!is_retryable(&ScrappaError::Http {
            status: StatusCode::BAD_REQUEST,
            message: String::new()
        }));
        assert!(!is_retryable(&ScrappaError::Transport(
            "network error".to_owned()
        )));
        assert!(is_retryable(&ScrappaError::Timeout));
    }

    #[test]
    fn parses_scrappa_validation_errors_and_bounds_plain_text() {
        assert_eq!(
            error_message(
                StatusCode::BAD_REQUEST,
                r#"{"message":"Invalid input","errors":{"query":["required","too short"]}}"#
            ),
            "Invalid input - query: required, too short"
        );
        assert_eq!(
            error_message(StatusCode::BAD_REQUEST, "  plain\n  error  "),
            "plain error"
        );
        assert_eq!(
            error_message(StatusCode::BAD_REQUEST, &"x".repeat(501)).len(),
            500
        );
    }

    #[tokio::test]
    async fn retries_503_and_sends_the_original_auth_and_user_agent_headers() {
        let server = MockServer::start(vec![
            (503, r#"{"message":"Unavailable"}"#),
            (200, r#"{"data":{"jobs":[]}}"#),
        ]);
        let client = Client::new();
        let url = server.base_url.join("/indeed/jobs").unwrap();
        let result = get_with_retry(&client, &url, "test-key", 2, Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(result, serde_json::json!({"data":{"jobs":[]}}));

        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("GET /indeed/jobs HTTP/1.1"));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("x-api-key: test-key"));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("user-agent: thescrappa-indeed-jobs-scraper/1.0"));
    }

    #[tokio::test]
    async fn timeout_is_retried_but_non_retryable_http_errors_are_not() {
        let timeout_server = MockServer::start_with_delays(
            vec![(200, r#"{"data":{"jobs":[]}}"#), (200, r#"{"jobs":[]}"#)],
            vec![Duration::from_millis(100), Duration::ZERO],
        );
        let result = get_with_retry(
            &Client::new(),
            &timeout_server.base_url,
            "test-key",
            2,
            Duration::from_millis(20),
        )
        .await
        .unwrap();
        assert_eq!(result, serde_json::json!({"jobs":[]}));
        assert_eq!(timeout_server.finish().len(), 2);

        let bad_request_server = MockServer::start(vec![(400, r#"{"message":"Bad query"}"#)]);
        let error = get_with_retry(
            &Client::new(),
            &bad_request_server.base_url,
            "test-key",
            3,
            Duration::from_secs(1),
        )
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "Scrappa API error (400): Bad query");
        assert_eq!(bad_request_server.finish().len(), 1);
    }

    #[test]
    fn timeout_message_keeps_the_actor_guidance() {
        assert_eq!(
            timeout_message(),
            "Scrappa API request timed out after 60000ms. The Indeed Jobs request exceeded the 60s Scrappa API timeout. Try again or refine the query."
        );
    }
}
