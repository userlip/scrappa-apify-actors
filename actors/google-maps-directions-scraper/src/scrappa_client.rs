use std::{
    error::Error,
    fmt,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use url::Url;

use crate::{batch_runner::DirectionsClient, request_params::DirectionsRequest};

const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_TIMEOUT: Duration = Duration::from_secs(45);
const MAX_ATTEMPTS: usize = 3;
const USER_AGENT: &str = "thescrappa-google-maps-directions-scraper/1.0";

#[derive(Debug)]
pub struct ScrappaTimeoutError {
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

#[derive(Debug)]
struct ScrappaHttpError {
    status: u16,
    message: String,
}

impl fmt::Display for ScrappaHttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.message
        )
    }
}

impl Error for ScrappaHttpError {}

#[derive(Clone)]
pub struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
    retry_delay_override: Option<Duration>,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: Option<&str>) -> Result<Self> {
        let base_url = Url::parse(base_url.unwrap_or(SCRAPPA_API_DEFAULT))
            .context("SCRAPPA_API_BASE_URL must be a valid absolute URL")?;
        let http = Client::builder()
            .build()
            .context("Could not create Scrappa HTTP client")?;
        Ok(Self {
            http,
            base_url,
            api_key,
            retry_delay_override: None,
        })
    }

    async fn get_with_retries(
        &self,
        request: &DirectionsRequest,
        deadline: Instant,
    ) -> Result<Value> {
        let url = directions_url(&self.base_url, request)?;
        let mut last_error = None;

        for attempt in 1..=MAX_ATTEMPTS {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(last_error
                    .unwrap_or_else(|| anyhow::Error::new(ScrappaTimeoutError { timeout_ms: 0 })));
            }
            let timeout = SCRAPPA_TIMEOUT.min(remaining);

            match self.send(url.clone(), timeout).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let should_retry = attempt < MAX_ATTEMPTS && is_retryable(&error);
                    last_error = Some(error);
                    if !should_retry {
                        break;
                    }

                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        break;
                    }
                    let delay = self
                        .retry_delay_override
                        .unwrap_or_else(|| retry_delay(attempt))
                        .min(remaining);
                    if !delay.is_zero() {
                        eprintln!(
                            "Scrappa directions request failed. Retrying attempt {}/{} in {}ms.",
                            attempt + 1,
                            MAX_ATTEMPTS,
                            delay.as_millis()
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::Error::new(ScrappaTimeoutError { timeout_ms: 0 })))
    }

    async fn send(&self, url: Url, timeout: Duration) -> Result<Value> {
        let response = self
            .http
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header("X-API-Key", &self.api_key)
            .timeout(timeout)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    anyhow::Error::new(ScrappaTimeoutError {
                        timeout_ms: timeout.as_millis(),
                    })
                } else {
                    anyhow::Error::new(error).context("Scrappa directions request failed")
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let message = read_error_message(response, status).await;
            return Err(anyhow::Error::new(ScrappaHttpError {
                status: status.as_u16(),
                message,
            }));
        }

        response
            .json::<Value>()
            .await
            .context("Scrappa response was not valid JSON")
    }
}

impl DirectionsClient for ScrappaClient {
    async fn get_directions(
        &self,
        request: &DirectionsRequest,
        deadline: Instant,
    ) -> Result<Value> {
        self.get_with_retries(request, deadline).await
    }
}

fn directions_url(base_url: &Url, request: &DirectionsRequest) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("SCRAPPA_API_BASE_URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(["maps", "directions"]);
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in &request.params {
            query.append_pair(key, value);
        }
    }
    Ok(url)
}

fn retry_delay(failed_attempt: usize) -> Duration {
    let base_ms =
        (1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt as u32))).min(10_000);
    let jitter_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64;
    Duration::from_millis((base_ms + jitter_ms).min(10_000))
}

fn is_retryable(error: &anyhow::Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    if let Some(error) = error.downcast_ref::<ScrappaHttpError>() {
        return matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504);
    }
    error
        .downcast_ref::<reqwest::Error>()
        .is_some_and(|error| error.is_timeout() || error.is_connect())
}

async fn read_error_message(response: Response, status: StatusCode) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let Ok(body) = response.text().await else {
        return fallback;
    };
    if body.is_empty() {
        return fallback;
    }

    if let Ok(parsed) = serde_json::from_str::<Value>(&body) {
        let message = parsed
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(&fallback);
        let details = parsed
            .get("errors")
            .and_then(Value::as_object)
            .map(|errors| {
                errors
                    .iter()
                    .filter_map(|(field, messages)| {
                        let messages = messages.as_array()?;
                        let messages = messages
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ");
                        Some(format!("{field}: {messages}"))
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default();
        return [message.to_owned(), details]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" - ");
    }

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    use super::*;
    use crate::request_params::build_directions_requests;
    use serde_json::json;

    fn request() -> DirectionsRequest {
        build_directions_requests(Some(&json!({
            "origin": "New York, NY",
            "destination": "Central Park",
            "mode": "driving",
            "hl": "en",
            "gl": "us"
        })))
        .unwrap()
        .remove(0)
    }

    #[test]
    fn retry_delay_is_exponential_and_bounded() {
        assert_eq!(retry_delay_ms_for_test(1, 0), 2000);
        assert_eq!(retry_delay_ms_for_test(2, 0), 4000);
        assert_eq!(retry_delay_ms_for_test(3, 0), 8000);
        assert_eq!(retry_delay_ms_for_test(5, 0), 10000);
    }

    fn retry_delay_ms_for_test(attempt: usize, jitter: u64) -> u64 {
        (1000_u64.saturating_mul(2_u64.saturating_pow(attempt as u32)) + jitter).min(10_000)
    }

    #[test]
    fn classifies_retryable_http_failures_and_network_errors() {
        assert!(is_retryable(&anyhow::Error::new(ScrappaTimeoutError {
            timeout_ms: 1000
        })));
        assert!(is_retryable(&anyhow::Error::new(ScrappaHttpError {
            status: 429,
            message: "Too many requests".to_owned()
        })));
        assert!(!is_retryable(&anyhow::Error::new(ScrappaHttpError {
            status: 400,
            message: "Bad request".to_owned()
        })));
    }

    #[test]
    fn builds_directions_url_with_only_defined_parameters() {
        let url =
            directions_url(&Url::parse("https://example.test/api").unwrap(), &request()).unwrap();
        assert_eq!(url.as_str(), "https://example.test/api/maps/directions?origin=New+York%2C+NY&destination=Central+Park&mode=driving&hl=en&gl=us");
    }

    #[tokio::test]
    async fn retries_a_transient_response_and_sends_scrappa_auth_headers() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (first, _) = listener.accept().unwrap();
            let first = read_request_with_response(first, 503, "temporary failure");
            let (second, _) = listener.accept().unwrap();
            let second = read_request_with_response(
                second,
                200,
                r#"{"status":"OK","directions":[{"distance":1}]}"#,
            );
            vec![first, second]
        });
        let mut client = ScrappaClient::new(
            "test-key".to_owned(),
            Some(&format!("http://{address}/api")),
        )
        .unwrap();
        client.retry_delay_override = Some(Duration::ZERO);

        let response = client
            .get_with_retries(&request(), Instant::now() + Duration::from_secs(3))
            .await
            .unwrap();

        assert_eq!(response["directions"][0]["distance"], 1);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("GET /api/maps/directions?origin="));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("x-api-key: test-key"));
        assert!(requests[0].contains(USER_AGENT));
        assert!(requests[0].contains("origin=New+York%2C+NY"));
        assert!(requests[0].contains("gl=us"));
    }

    #[tokio::test]
    async fn bounds_a_request_to_the_remaining_batch_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request_headers(&stream);
            thread::sleep(Duration::from_millis(100));
            let _ = write_response(&mut stream, 200, r#"{"status":"OK","directions":[]}"#);
            request
        });
        let mut client = ScrappaClient::new(
            "test-key".to_owned(),
            Some(&format!("http://{address}/api")),
        )
        .unwrap();
        client.retry_delay_override = Some(Duration::ZERO);

        let error = client
            .get_with_retries(&request(), Instant::now() + Duration::from_millis(30))
            .await
            .unwrap_err();

        assert!(error.downcast_ref::<ScrappaTimeoutError>().is_some());
        assert!(server
            .join()
            .unwrap()
            .starts_with("GET /api/maps/directions?origin="));
    }

    fn read_request_with_response(mut stream: TcpStream, status: u16, body: &str) -> String {
        let request = read_request_headers(&stream);
        write_response(&mut stream, status, body).unwrap();
        request
    }

    fn read_request_headers(stream: &TcpStream) -> String {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut request = String::new();
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap() > 0 {
            let done = line == "\r\n";
            request.push_str(&line);
            line.clear();
            if done {
                break;
            }
        }
        request
    }

    fn write_response(stream: &mut TcpStream, status: u16, body: &str) -> std::io::Result<()> {
        write!(stream, "HTTP/1.1 {status} {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", if status < 300 { "OK" } else { "Service Unavailable" }, body.len())?;
        stream.flush()
    }
}
