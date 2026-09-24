use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{Map, Value};
use std::{
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

const USER_AGENT: &str = "thescrappa-linkedin-jobs-search-scraper/1.0";

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    timeout: Duration,
}

impl ScrappaTimeoutError {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
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

impl std::error::Error for ScrappaTimeoutError {}

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

impl std::error::Error for ScrappaApiError {}

pub struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
    timeout: Duration,
    max_attempts: usize,
}

impl ScrappaClient {
    pub fn new(
        base_url: &str,
        api_key: String,
        timeout: Duration,
        max_attempts: usize,
    ) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(timeout)
                .build()
                .map_err(|error| anyhow!("Could not create Scrappa HTTP client: {error}"))?,
            base_url: Url::parse(base_url).context("SCRAPPA_API_BASE_URL must be a valid URL")?,
            api_key,
            timeout,
            max_attempts: max_attempts.max(1),
        })
    }

    pub async fn get(&self, params: &Map<String, Value>) -> Result<Value> {
        self.get_with_retry_delay(params, get_retry_delay).await
    }

    async fn get_with_retry_delay<F>(
        &self,
        params: &Map<String, Value>,
        retry_delay: F,
    ) -> Result<Value>
    where
        F: Fn(usize) -> Duration,
    {
        let url = self.search_url(params)?;
        for attempt in 1..=self.max_attempts {
            match self.send(&url).await {
                Ok(value) => return Ok(value),
                Err(error) if attempt < self.max_attempts && is_retryable(&error) => {
                    let delay = retry_delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        self.max_attempts,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("the retry loop returns a response or error")
    }

    fn search_url(&self, params: &Map<String, Value>) -> Result<Url> {
        let mut url = self.base_url.clone();
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow!("SCRAPPA_API_BASE_URL cannot be a base URL"))?;
        path.pop_if_empty().push("search-light");
        drop(path);
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                let value = match value {
                    Value::Null | Value::Bool(false) => continue,
                    Value::Bool(true) => "1".to_owned(),
                    Value::String(value) if value.is_empty() => continue,
                    Value::String(value) => value.clone(),
                    value => value.to_string(),
                };
                query.append_pair(key, &value);
            }
        }
        Ok(url)
    }

    async fn send(&self, url: &Url) -> Result<Value> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(|error| self.request_error(error))?;
        if !response.status().is_success() {
            let status = response.status();
            let message = self.read_error_message(response, status).await?;
            return Err(ScrappaApiError {
                status: status.as_u16(),
                message,
            }
            .into());
        }
        response
            .json()
            .await
            .map_err(|error| self.request_error(error))
    }

    async fn read_error_message(&self, response: Response, status: StatusCode) -> Result<String> {
        let fallback = status
            .canonical_reason()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
        let body = response
            .text()
            .await
            .map_err(|error| self.request_error(error))?;
        if body.is_empty() {
            return Ok(fallback);
        }

        if let Ok(error) = serde_json::from_str::<Value>(&body) {
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
                            .collect::<Vec<_>>();
                        (!messages.is_empty()).then(|| format!("{field}: {}", messages.join(", ")))
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                if !details.is_empty() {
                    message.push_str(" - ");
                    message.push_str(&details);
                }
            }
            return Ok(message);
        }

        Ok(body
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(500)
            .collect())
    }

    fn request_error(&self, error: reqwest::Error) -> anyhow::Error {
        if error.is_timeout() {
            ScrappaTimeoutError::new(self.timeout).into()
        } else {
            anyhow!("Scrappa API request failed: {error}")
        }
    }
}

fn is_retryable(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504))
}

fn get_retry_delay(failed_attempt: usize) -> Duration {
    let jitter = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64;
    let backoff = 1_000u64.saturating_mul(2u64.saturating_pow(failed_attempt.min(32) as u32));
    Duration::from_millis(backoff.saturating_add(jitter).min(10_000))
}

#[cfg(test)]
mod tests {
    use super::{
        get_retry_delay, is_retryable, ScrappaApiError, ScrappaClient, ScrappaTimeoutError,
    };
    use serde_json::{json, Map, Value};
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
        thread::{self, JoinHandle},
        time::Duration,
    };

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: String,
        requests: Receiver<String>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                for response in responses {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_request(&mut stream);
                    sender.send(request).unwrap();
                    let reason = match response.status {
                        200 => "OK",
                        422 => "Unprocessable Entity",
                        429 => "Too Many Requests",
                        503 => "Service Unavailable",
                        _ => "Error",
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
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
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            if let Some(thread) = self.thread.take() {
                thread.join().unwrap();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            let size = stream.read(&mut chunk).unwrap();
            request.extend_from_slice(&chunk[..size]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8(request).unwrap()
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    #[tokio::test]
    async fn retries_transient_statuses_and_preserves_auth_query_and_user_agent() {
        let server = MockServer::start(vec![
            response(503, r#"{"message":"busy"}"#),
            response(
                200,
                r#"{"organic_results":[],"pagination":{"current_page":2}}"#,
            ),
        ]);
        let client = ScrappaClient::new(
            &server.base_url,
            "test-key".to_owned(),
            Duration::from_secs(1),
            3,
        )
        .unwrap();
        let params: Map<String, Value> = serde_json::from_value(json!({
            "query": "site:linkedin.com/jobs/view/ software engineer remote",
            "page": 2,
            "num": 10
        }))
        .unwrap();
        let result = client
            .get_with_retry_delay(&params, |_| Duration::ZERO)
            .await
            .unwrap();
        assert_eq!(result["pagination"]["current_page"], 2);

        let first_request = server.requests.recv().unwrap().to_ascii_lowercase();
        let second_request = server.requests.recv().unwrap().to_ascii_lowercase();
        for request in [first_request, second_request] {
            assert!(request.starts_with("get /api/search-light?"));
            assert!(request.contains("x-api-key: test-key"));
            assert!(request.contains("user-agent: thescrappa-linkedin-jobs-search-scraper/1.0"));
            assert!(request
                .contains("query=site%3alinkedin.com%2fjobs%2fview%2f+software+engineer+remote"));
            assert!(request.contains("page=2"));
        }
    }

    #[tokio::test]
    async fn does_not_retry_client_errors_and_formats_validation_details() {
        let server = MockServer::start(vec![response(
            422,
            r#"{"message":"Invalid search","errors":{"query":["is required","is too long"]}}"#,
        )]);
        let client = ScrappaClient::new(
            &server.base_url,
            "test-key".to_owned(),
            Duration::from_secs(1),
            3,
        )
        .unwrap();
        let error = client
            .get_with_retry_delay(&Map::new(), |_| Duration::ZERO)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (422): Invalid search - query: is required, is too long"
        );
        assert!(server
            .requests
            .recv()
            .unwrap()
            .starts_with("GET /api/search-light?"));
        assert!(server.requests.try_recv().is_err());
    }

    #[test]
    fn retry_rules_and_backoff_match_the_typescript_client() {
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable(&anyhow::Error::new(ScrappaApiError {
                status,
                message: "temporary".to_owned(),
            })));
        }
        assert!(!is_retryable(&anyhow::Error::new(ScrappaApiError {
            status: 422,
            message: "bad input".to_owned(),
        })));
        assert!(is_retryable(&anyhow::Error::new(ScrappaTimeoutError::new(
            Duration::from_secs(60)
        ))));
        assert!((Duration::from_secs(2)..Duration::from_secs(3)).contains(&get_retry_delay(1)));
        assert!((Duration::from_secs(4)..Duration::from_secs(5)).contains(&get_retry_delay(2)));
        assert_eq!(get_retry_delay(10), Duration::from_secs(10));
    }
}
