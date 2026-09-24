use anyhow::{anyhow, Result};
use reqwest::{header, Client, Response};
use serde_json::{Map, Value};
use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

pub const REQUEST_TIMEOUT_MS: u64 = 90_000;
pub const MAX_ATTEMPTS: usize = 3;
const DEFAULT_BASE_URL: &str = "https://scrappa.co/api";

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    pub timeout_ms: u64,
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
struct ScrappaTransportError {
    retryable: bool,
    message: String,
}

impl fmt::Display for ScrappaTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ScrappaTransportError {}

pub fn get_retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponential = 1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt as u32));
    exponential.saturating_add(jitter_ms).min(10_000)
}

fn retry_delay(failed_attempt: usize) -> Duration {
    let jitter_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::from(duration.subsec_nanos() % 1000))
        .unwrap_or(0);
    Duration::from_millis(get_retry_delay_ms(failed_attempt, jitter_ms))
}

pub fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if let Some(error) = error.downcast_ref::<ScrappaApiError>() {
        return matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504);
    }
    error
        .downcast_ref::<ScrappaTransportError>()
        .is_some_and(|error| error.retryable)
}

fn reqwest_error(error: reqwest::Error, timeout_ms: u64) -> anyhow::Error {
    if error.is_timeout() {
        return ScrappaTimeoutError { timeout_ms }.into();
    }
    let retryable = error.is_connect()
        || [
            "ECONNRESET",
            "ECONNREFUSED",
            "ETIMEDOUT",
            "ENOTFOUND",
            "EAI_AGAIN",
        ]
        .iter()
        .any(|code| error.to_string().contains(code));
    ScrappaTransportError {
        retryable,
        message: error.to_string(),
    }
    .into()
}

pub struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
    request_timeout_ms: u64,
    retry_delay: fn(usize) -> Duration,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: Option<&str>) -> Result<Self> {
        Self::with_timeouts(
            api_key,
            base_url.unwrap_or(DEFAULT_BASE_URL),
            Duration::from_millis(REQUEST_TIMEOUT_MS),
            retry_delay,
        )
    }

    fn with_timeouts(
        api_key: String,
        base_url: &str,
        request_timeout: Duration,
        retry_delay: fn(usize) -> Duration,
    ) -> Result<Self> {
        let http = Client::builder()
            .timeout(request_timeout)
            .build()
            .map_err(|error| anyhow!("Could not create Scrappa HTTP client: {error}"))?;
        Ok(Self {
            http,
            base_url: Url::parse(base_url).map_err(|error| {
                anyhow!("SCRAPPA_API_BASE_URL must be a valid absolute URL: {error}")
            })?,
            api_key,
            request_timeout_ms: request_timeout.as_millis().min(u128::from(u64::MAX)) as u64,
            retry_delay,
        })
    }

    fn endpoint(&self) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("SCRAPPA_API_BASE_URL must be a base URL"))?
            .pop_if_empty()
            .extend(["jameda", "search"]);
        Ok(url)
    }

    pub async fn get(&self, params: &Map<String, Value>) -> Result<Value> {
        let mut url = self.endpoint()?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                match value {
                    Value::Null => continue,
                    Value::String(value) if value.is_empty() => continue,
                    Value::Bool(false) => continue,
                    Value::Bool(true) => {
                        query.append_pair(key, "1");
                    }
                    Value::String(value) => {
                        query.append_pair(key, value);
                    }
                    _ => {
                        query.append_pair(key, &value.to_string());
                    }
                }
            }
        }

        for attempt in 1..=MAX_ATTEMPTS {
            match self.send_once(&url).await {
                Ok(value) => return Ok(value),
                Err(error) if attempt < MAX_ATTEMPTS && is_retryable_scrappa_error(&error) => {
                    let delay = (self.retry_delay)(attempt);
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{MAX_ATTEMPTS} in {}ms.",
                        error,
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("the retry loop always returns a final attempt result")
    }

    async fn send_once(&self, url: &Url) -> Result<Value> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, "thescrappa-jameda-search-scraper/1.0")
            .send()
            .await
            .map_err(|error| reqwest_error(error, self.request_timeout_ms))?;
        if !response.status().is_success() {
            return Err(scrappa_api_error(response, self.request_timeout_ms).await?);
        }
        response
            .json::<Value>()
            .await
            .map_err(|error| reqwest_error(error, self.request_timeout_ms))
    }
}

async fn scrappa_api_error(response: Response, timeout_ms: u64) -> Result<anyhow::Error> {
    let status = response.status();
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) if error.is_timeout() => return Err(reqwest_error(error, timeout_ms)),
        Err(_) => String::new(),
    };
    let message = if body.is_empty() {
        fallback
    } else if let Some(message) = json_error_message(&body, &fallback) {
        message
    } else {
        truncate_utf16(&body.split_whitespace().collect::<Vec<_>>().join(" "), 500)
    };
    Ok(ScrappaApiError {
        status: status.as_u16(),
        message,
    }
    .into())
}

fn json_error_message(body: &str, fallback: &str) -> Option<String> {
    let data = serde_json::from_str::<Value>(body).ok()?;
    let object = data.as_object()?;
    let mut message = object
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned();
    if let Some(errors) = object.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .filter_map(|(field, messages)| {
                messages.as_array().map(|messages| {
                    let joined = messages
                        .iter()
                        .map(|message| {
                            message
                                .as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| message.to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{field}: {joined}")
                })
            })
            .collect::<Vec<_>>()
            .join("; ");
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details);
        }
    }
    Some(message)
}

fn truncate_utf16(value: &str, limit: usize) -> String {
    let mut units = 0;
    let mut output = String::new();
    for character in value.chars() {
        let length = character.len_utf16();
        if units + length > limit {
            break;
        }
        output.push(character);
        units += length;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;
    use serde_json::json;
    use std::{
        collections::HashMap,
        io::{Read, Write},
        net::TcpListener,
        sync::mpsc,
        thread,
    };

    fn no_retry_delay(_: usize) -> Duration {
        Duration::ZERO
    }

    fn request_url(request: &str) -> url::Url {
        let target = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap();
        url::Url::parse(&format!("http://localhost{target}")).unwrap()
    }

    fn response(status: u16, body: &str) -> String {
        format!(
            "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            StatusCode::from_u16(status).unwrap().canonical_reason().unwrap_or("OK"),
            body.len()
        )
    }

    fn mock_server(
        responses: Vec<String>,
    ) -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        let thread = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                let mut chunk = [0; 4096];
                loop {
                    let count = stream.read(&mut chunk).unwrap_or(0);
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&chunk[..count]);
                    if request.windows(4).any(|part| part == b"\r\n\r\n") {
                        break;
                    }
                }
                sender
                    .send(String::from_utf8_lossy(&request).into_owned())
                    .unwrap();
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (format!("http://{address}/api"), receiver, thread)
    }

    #[test]
    fn retries_only_timeout_transient_status_and_connect_errors() {
        assert!(is_retryable_scrappa_error(
            &ScrappaTimeoutError { timeout_ms: 90_000 }.into()
        ));
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_scrappa_error(
                &ScrappaApiError {
                    status,
                    message: String::new()
                }
                .into()
            ));
        }
        for status in [400, 401, 403, 404] {
            assert!(!is_retryable_scrappa_error(
                &ScrappaApiError {
                    status,
                    message: String::new()
                }
                .into()
            ));
        }
        assert!(!is_retryable_scrappa_error(&anyhow!("invalid URL")));
        assert!(!is_retryable_scrappa_error(
            &ScrappaTransportError {
                retryable: false,
                message: "bad request".into()
            }
            .into()
        ));
        assert!(is_retryable_scrappa_error(
            &ScrappaTransportError {
                retryable: true,
                message: "connection reset".into()
            }
            .into()
        ));
    }

    #[test]
    fn calculates_the_existing_backoff_schedule() {
        assert_eq!(get_retry_delay_ms(1, 500), 2_500);
        assert_eq!(get_retry_delay_ms(2, 900), 4_900);
        assert_eq!(get_retry_delay_ms(8, 999), 10_000);
    }

    #[tokio::test]
    async fn retries_transient_scrappa_responses_and_preserves_auth_and_query() {
        let (base_url, requests, server) = mock_server(vec![
            response(503, r#"{"message":"temporarily unavailable"}"#),
            response(200, r#"{"data":[]}"#),
        ]);
        let client = ScrappaClient::with_timeouts(
            "test-api-key".into(),
            &base_url,
            Duration::from_secs(1),
            no_retry_delay,
        )
        .unwrap();
        let params: Map<String, Value> = serde_json::from_value(json!({
            "q":"Hals & Nase", "loc":"München", "per_page":28, "page":2
        }))
        .unwrap();
        assert_eq!(client.get(&params).await.unwrap(), json!({"data":[]}));
        server.join().unwrap();

        let first = requests.recv().unwrap();
        let second = requests.recv().unwrap();
        for request in [first, second] {
            let request_url = request_url(&request);
            assert_eq!(request_url.path(), "/api/jameda/search");
            let query = request_url.query_pairs().into_owned().collect::<HashMap<_, _>>();
            assert_eq!(query.get("q").map(String::as_str), Some("Hals & Nase"));
            assert_eq!(query.get("loc").map(String::as_str), Some("München"));
            assert_eq!(query.get("per_page").map(String::as_str), Some("28"));
            assert_eq!(query.get("page").map(String::as_str), Some("2"));
            assert!(request
                .to_ascii_lowercase()
                .contains("x-api-key: test-api-key"));
            assert!(request
                .to_ascii_lowercase()
                .contains("user-agent: thescrappa-jameda-search-scraper/1.0"));
        }
    }

    #[tokio::test]
    async fn formats_non_transient_scrappa_errors_without_retrying() {
        let (base_url, requests, server) = mock_server(vec![response(
            422,
            r#"{"message":"Invalid search","errors":{"q":["too short","not supported"]}}"#,
        )]);
        let client = ScrappaClient::with_timeouts(
            "test-api-key".into(),
            &base_url,
            Duration::from_secs(1),
            no_retry_delay,
        )
        .unwrap();
        let params = serde_json::from_value(json!({"q":"test"})).unwrap();
        let error = client.get(&params).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (422): Invalid search - q: too short, not supported"
        );
        server.join().unwrap();
        assert!(requests
            .recv()
            .unwrap()
            .starts_with("GET /api/jameda/search?q=test HTTP/1.1"));
        assert!(requests.try_recv().is_err());
    }

    #[tokio::test]
    async fn enforces_the_request_deadline_and_retries_each_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (attempt_sender, attempt_receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            for _ in 0..MAX_ATTEMPTS {
                let (mut stream, _) = listener.accept().unwrap();
                attempt_sender.send(()).unwrap();
                thread::sleep(Duration::from_millis(60));
                let _ = stream.write_all(response(200, "{}").as_bytes());
            }
        });
        let client = ScrappaClient::with_timeouts(
            "test-api-key".into(),
            &format!("http://{address}/api"),
            Duration::from_millis(20),
            no_retry_delay,
        )
        .unwrap();
        let params = serde_json::from_value(json!({"q":"test"})).unwrap();
        let error = client.get(&params).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API request timed out after 20ms"
        );
        assert!(error.downcast_ref::<ScrappaTimeoutError>().is_some());
        for _ in 0..MAX_ATTEMPTS {
            attempt_receiver
                .recv_timeout(Duration::from_secs(1))
                .unwrap();
        }
        server.join().unwrap();
    }
}
