use anyhow::{Context, Result, anyhow};
use reqwest::{Client, Response, StatusCode, Url};
use serde_json::{Map, Value};
use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const REQUEST_TIMEOUT: Duration = Duration::from_millis(25_000);
const USER_AGENT: &str = "thescrappa-google-finance-quote-scraper/1.0";

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    timeout: Duration,
}

impl ScrappaTimeoutError {
    fn new(timeout: Duration) -> Self {
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

impl Error for ScrappaTimeoutError {}

#[derive(Debug)]
pub struct ScrappaHttpError {
    pub status: u16,
    pub details: String,
}

impl ScrappaHttpError {
    pub fn new(status: u16, details: String) -> Self {
        Self { status, details }
    }
}

impl fmt::Display for ScrappaHttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.details
        )
    }
}

impl Error for ScrappaHttpError {}

pub struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
    timeout: Duration,
    #[cfg(test)]
    retry_delay_override: Option<Duration>,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: &str, timeout: Duration) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(timeout)
                .build()
                .context("Could not create Scrappa HTTP client")?,
            base_url: Url::parse(base_url).context("SCRAPPA_API_BASE_URL must be a valid URL")?,
            api_key,
            timeout,
            #[cfg(test)]
            retry_delay_override: None,
        })
    }

    pub async fn get(
        &self,
        endpoint: &str,
        params: &Map<String, Value>,
        attempts: usize,
    ) -> Result<Value> {
        let attempts = attempts.max(1);
        for attempt in 1..=attempts {
            match self.send_get(endpoint, params).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < attempts && is_retryable_scrappa_error(&error) => {
                    let delay = self.retry_delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{attempts} in {}ms.",
                        error,
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("the retry loop always returns its last response")
    }

    async fn send_get(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        let mut url = self.endpoint_url(endpoint)?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                if let Some(value) = query_value(value) {
                    query.append_pair(key, &value);
                }
            }
        }

        let response = self
            .http
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|error| map_request_error(error, self.timeout))?;

        let status = response.status();
        if !status.is_success() {
            let details = read_error_message(response, status, self.timeout).await?;
            return Err(ScrappaHttpError::new(status.as_u16(), details).into());
        }

        match response.json().await {
            Ok(response) => Ok(response),
            Err(error) if error.is_timeout() => Err(ScrappaTimeoutError::new(self.timeout).into()),
            Err(error) => Err(anyhow!("Scrappa API returned invalid JSON: {error}")),
        }
    }

    fn endpoint_url(&self, endpoint: &str) -> Result<Url> {
        let mut url = self.base_url.clone();
        let segments = endpoint
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty());
        url.path_segments_mut()
            .map_err(|_| anyhow!("SCRAPPA_API_BASE_URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(segments);
        Ok(url)
    }

    fn retry_delay(&self, failed_attempt: usize) -> Duration {
        #[cfg(test)]
        if let Some(delay) = self.retry_delay_override {
            return delay;
        }

        let jitter = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_millis() as u64;
        Duration::from_millis(get_retry_delay_ms(failed_attempt, jitter))
    }

    #[cfg(test)]
    fn without_retry_wait(mut self) -> Self {
        self.retry_delay_override = Some(Duration::ZERO);
        self
    }
}

fn query_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(true) => Some("1".into()),
        Value::Bool(false) => None,
        Value::String(value) => (!value.is_empty()).then(|| value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Array(values) => Some(
            values
                .iter()
                .map(js_string)
                .collect::<Vec<_>>()
                .join(","),
        ),
        Value::Object(_) => Some("[object Object]".into()),
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn map_request_error(error: reqwest::Error, timeout: Duration) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaTimeoutError::new(timeout).into()
    } else {
        error.into()
    }
}

async fn read_error_message(
    response: Response,
    status: StatusCode,
    timeout: Duration,
) -> Result<String> {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) if error.is_timeout() => return Err(ScrappaTimeoutError::new(timeout).into()),
        Err(_) => return Ok(fallback),
    };
    if body.is_empty() {
        return Ok(fallback);
    }

    if let Ok(Value::Object(data)) = serde_json::from_str::<Value>(&body) {
        let mut message = data
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or_else(|| fallback.clone());
        if data.get("errors").is_some_and(is_truthy) {
            if let Some(errors) = data.get("errors").and_then(Value::as_object) {
                let details = errors
                    .iter()
                    .map(|(field, values)| {
                        let values = values
                            .as_array()
                            .map(|values| values.iter().map(js_string).collect::<Vec<_>>())
                            .unwrap_or_default();
                        format!("{field}: {}", values.join(", "))
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                if !details.is_empty() {
                    message.push_str(" - ");
                    message.push_str(&details);
                }
            }
        }
        return Ok(message);
    }

    Ok(body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect())
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

pub fn get_retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponent = failed_attempt.min(20) as u32;
    let base = 1_000_u64.saturating_mul(2_u64.saturating_pow(exponent));
    base.saturating_add(jitter_ms).min(10_000)
}

pub fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if let Some(error) = error.downcast_ref::<ScrappaHttpError>() {
        return matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504);
    }

    let Some(status) = error
        .to_string()
        .strip_prefix("Scrappa API error (")
        .and_then(|message| message.split_once("):").map(|(status, _)| status))
        .and_then(|status| status.parse::<u16>().ok())
    else {
        return false;
    };
    matches!(status, 408 | 429 | 500 | 502 | 503 | 504)
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use super::{
        ScrappaClient, ScrappaHttpError, ScrappaTimeoutError, get_retry_delay_ms,
        is_retryable_scrappa_error,
    };
    use serde_json::{Map, json};
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver},
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    struct MockResponse {
        status: u16,
        body: String,
        delay: Duration,
    }

    struct MockServer {
        base_url: String,
        requests: Receiver<String>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(5);
                for response in responses {
                    let (mut stream, _) = loop {
                        if stopped.load(Ordering::Relaxed) || Instant::now() >= deadline {
                            return;
                        }
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(2));
                            }
                            Err(_) => return,
                        }
                    };
                    let request = read_request(&mut stream);
                    if sender.send(request).is_err() {
                        return;
                    }
                    if !response.delay.is_zero() {
                        thread::sleep(response.delay);
                    }
                    let reason = match response.status {
                        200 => "OK",
                        422 => "Unprocessable Entity",
                        500 => "Internal Server Error",
                        _ => "Mock Response",
                    };
                    let reply = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    let _ = stream.write_all(reply.as_bytes());
                }
            });
            Self {
                base_url: format!("http://{address}/api"),
                requests,
                stop,
                thread: Some(thread),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.try_iter().collect()
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        let mut expected_length = None;
        loop {
            let Ok(count) = stream.read(&mut buffer) else {
                break;
            };
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            if let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                if expected_length.is_none() {
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let content_length = headers.lines().find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|length| length.trim().parse::<usize>().ok())
                    });
                    expected_length = Some(header_end + 4 + content_length.unwrap_or(0));
                }
                if request.len() >= expected_length.unwrap_or(0) {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&request).to_string()
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.into(),
            delay: Duration::ZERO,
        }
    }

    #[tokio::test]
    async fn sends_api_auth_user_agent_and_filtered_get_parameters() {
        let server = MockServer::start(vec![response(200, r#"{"ok":true}"#)]);
        let client = ScrappaClient::new(
            "test-key".into(),
            &server.base_url,
            Duration::from_secs(1),
        )
        .unwrap();
        let params: Map<String, serde_json::Value> = json!({
            "symbol": "AAPL",
            "exchange": "NASDAQ",
            "use_cache": true,
            "debug": false,
            "empty": "",
            "absent": null,
        })
        .as_object()
        .unwrap()
        .clone();

        assert_eq!(client.get("/google-finance/quote", &params, 1).await.unwrap(), json!({"ok": true}));
        let request = server.requests().pop().unwrap();
        let request_line = request.lines().next().unwrap();
        assert!(request_line.starts_with("GET /api/google-finance/quote?"));
        assert!(request_line.contains("symbol=AAPL"));
        assert!(request_line.contains("exchange=NASDAQ"));
        assert!(request_line.contains("use_cache=1"));
        assert!(!request_line.contains("debug="));
        assert!(request.contains("x-api-key: test-key"));
        assert!(request.contains("user-agent: thescrappa-google-finance-quote-scraper/1.0"));
    }

    #[tokio::test]
    async fn retries_retryable_http_statuses_and_returns_the_next_success() {
        let server = MockServer::start(vec![
            response(500, r#"{"message":"Internal Server Error"}"#),
            response(200, r#"{"quote":{"summary":{"symbol":"MSFT"}}}"#),
        ]);
        let client = ScrappaClient::new(
            "test-key".into(),
            &server.base_url,
            Duration::from_secs(1),
        )
        .unwrap()
        .without_retry_wait();

        let result = client
            .get("/google-finance/quote", &json!({"symbol": "MSFT"}).as_object().unwrap().clone(), 3)
            .await
            .unwrap();
        assert_eq!(result["quote"]["summary"]["symbol"], "MSFT");
        assert_eq!(server.requests().len(), 2);
    }

    #[tokio::test]
    async fn retries_a_period_quote_three_times_then_falls_back_without_period_type() {
        let server = MockServer::start(vec![
            response(500, r#"{"message":"Internal Server Error"}"#),
            response(500, r#"{"message":"Internal Server Error"}"#),
            response(500, r#"{"message":"Internal Server Error"}"#),
            response(200, r#"{"quote":{"summary":{"symbol":"MSFT"}}}"#),
        ]);
        let client = ScrappaClient::new(
            "test-key".into(),
            &server.base_url,
            Duration::from_secs(1),
        )
        .unwrap()
        .without_retry_wait();
        let params: Map<String, serde_json::Value> = json!({
            "symbol": "MSFT",
            "period_type": "quarterly",
            "hl": "en",
        })
        .as_object()
        .unwrap()
        .clone();

        let result = crate::quote_fetch::fetch_quote_with_fallback(&client, &params, 3)
            .await
            .unwrap();
        assert_eq!(result.response["quote"]["summary"]["symbol"], "MSFT");
        assert_eq!(
            result.fallback.unwrap().reason,
            "scrappa_5xx_after_financial_period_request"
        );
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests[..3].iter().all(|request| {
            request
                .lines()
                .next()
                .is_some_and(|line| line.contains("period_type=quarterly"))
        }));
        assert!(requests[3]
            .lines()
            .next()
            .is_some_and(|line| !line.contains("period_type=")));
        assert!(requests[3]
            .lines()
            .next()
            .is_some_and(|line| line.contains("hl=en") && line.contains("symbol=MSFT")));
    }

    #[tokio::test]
    async fn does_not_retry_a_validation_error() {
        let server = MockServer::start(vec![response(
            422,
            r#"{"message":"Invalid request","errors":{"symbol":["The symbol is required."]}}"#,
        )]);
        let client = ScrappaClient::new(
            "test-key".into(),
            &server.base_url,
            Duration::from_secs(1),
        )
        .unwrap()
        .without_retry_wait();

        let error = client
            .get("/google-finance/quote", &Map::new(), 3)
            .await
            .unwrap_err();
        let error = error.downcast_ref::<ScrappaHttpError>().unwrap();
        assert_eq!(error.status, 422);
        assert_eq!(error.details, "Invalid request - symbol: The symbol is required.");
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn maps_a_slow_request_to_the_scrappa_timeout_error() {
        let mut slow_response = response(200, r#"{"ok":true}"#);
        slow_response.delay = Duration::from_millis(100);
        let server = MockServer::start(vec![slow_response]);
        let client = ScrappaClient::new(
            "test-key".into(),
            &server.base_url,
            Duration::from_millis(20),
        )
        .unwrap();

        let error = client
            .get("/google-finance/quote", &Map::new(), 1)
            .await
            .unwrap_err();
        assert!(error.downcast_ref::<ScrappaTimeoutError>().is_some());
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn classifies_only_the_expected_retry_conditions_and_bounds_backoff() {
        assert!(is_retryable_scrappa_error(
            &ScrappaTimeoutError::new(Duration::from_secs(1)).into()
        ));
        assert!(is_retryable_scrappa_error(
            &ScrappaHttpError::new(503, "Service unavailable".into()).into()
        ));
        assert!(!is_retryable_scrappa_error(
            &ScrappaHttpError::new(422, "Invalid request".into()).into()
        ));
        assert!(is_retryable_scrappa_error(
            &anyhow!("Scrappa API error (429): Rate limited")
        ));
        assert!(!is_retryable_scrappa_error(&anyhow!("connection refused")));
        assert_eq!(get_retry_delay_ms(1, 0), 2_000);
        assert_eq!(get_retry_delay_ms(2, 500), 4_500);
        assert_eq!(get_retry_delay_ms(20, 0), 10_000);
    }
}
