use anyhow::Result;
use reqwest::{header, Client};
use serde_json::Value;
use std::{
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

pub const DEFAULT_API_BASE_URL: &str = "https://scrappa.co/api";
pub const REQUEST_TIMEOUT_MS: u64 = 90_000;
pub const MAX_ATTEMPTS: usize = 3;
const USER_AGENT: &str = "thescrappa-vinted-item-details-scraper/1.0";

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    timeout_ms: u64,
}

impl ScrappaTimeoutError {
    fn new(timeout_ms: u64) -> Self {
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
pub struct ScrappaApiError {
    pub status: u16,
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
    message: String,
    retryable: bool,
}

impl fmt::Display for ScrappaTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ScrappaTransportError {}

pub struct ScrappaClient {
    http: Client,
    api_key: String,
    api_base_url: Url,
    timeout: Duration,
}

impl ScrappaClient {
    pub fn new(api_key: String, api_base_url: &str) -> Result<Self> {
        Self::with_timeout(
            api_key,
            api_base_url,
            Duration::from_millis(REQUEST_TIMEOUT_MS),
        )
    }

    fn with_timeout(api_key: String, api_base_url: &str, timeout: Duration) -> Result<Self> {
        Ok(Self {
            http: Client::builder().build()?,
            api_key,
            api_base_url: Url::parse(api_base_url.trim_end_matches('/'))?,
            timeout,
        })
    }

    pub async fn get_item_details(&self, item_id: &str, country: &str) -> Result<Value> {
        let url = self.endpoint("/vinted/item-details")?;
        let mut last_error = None;

        for attempt in 1..=MAX_ATTEMPTS {
            match self.send_once(url.clone(), item_id, country).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < MAX_ATTEMPTS && is_retryable_scrappa_error(&error) => {
                    let delay = retry_delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        MAX_ATTEMPTS,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                    last_error = Some(error);
                }
                Err(error) => return Err(error),
            }
        }

        Err(last_error.expect("at least one attempt is made"))
    }

    fn endpoint(&self, path: &str) -> Result<Url> {
        let base = self.api_base_url.as_str().trim_end_matches('/');
        Ok(Url::parse(&format!("{base}{path}"))?)
    }

    async fn send_once(&self, mut url: Url, item_id: &str, country: &str) -> Result<Value> {
        url.query_pairs_mut()
            .clear()
            .append_pair("item_id", item_id)
            .append_pair("country", country);

        let response = self
            .http
            .get(url)
            .header(header::ACCEPT, "application/json")
            .header("X-API-Key", &self.api_key)
            .header(header::USER_AGENT, USER_AGENT)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(transport_error)?;

        let status = response.status();
        if !status.is_success() {
            let message = read_error_message(response).await?;
            return Err(ScrappaApiError {
                status: status.as_u16(),
                message,
            }
            .into());
        }
        response.json::<Value>().await.map_err(transport_error)
    }
}

fn transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        return ScrappaTimeoutError::new(REQUEST_TIMEOUT_MS).into();
    }
    ScrappaTransportError {
        message: error.to_string(),
        retryable: error.is_connect() || error.is_body(),
    }
    .into()
}

async fn read_error_message(response: reqwest::Response) -> Result<String> {
    let status = response.status();
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let body = response.text().await.map_err(transport_error)?;
    if body.is_empty() {
        return Ok(fallback);
    }

    if let Ok(error) = serde_json::from_str::<Value>(&body) {
        let message = error
            .get("message")
            .filter(|message| !message.is_null())
            .map(js_string)
            .unwrap_or(fallback);
        let detail = error
            .get("errors")
            .and_then(Value::as_object)
            .map(|errors| {
                errors
                    .iter()
                    .map(|(field, messages)| {
                        let messages = messages
                            .as_array()
                            .map(|messages| {
                                messages
                                    .iter()
                                    .map(js_string)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_else(|| js_string(messages));
                        format!("{field}: {messages}")
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default();
        return Ok(if detail.is_empty() {
            message
        } else {
            format!("{message} - {detail}")
        });
    }

    let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
    Ok(body.chars().take(500).collect())
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".to_owned(),
        _ => value.to_string(),
    }
}

fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
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

pub(crate) fn is_actor_level_scrappa_failure(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| matches!(error.status, 401 | 403))
}

fn retry_delay(failed_attempt: usize) -> Duration {
    let base_ms =
        (1_000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt as u32))).min(10_000);
    let jitter_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
        % 1_000;
    Duration::from_millis((base_ms + jitter_ms).min(10_000))
}

pub(crate) fn timeout_item_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{error}. The Vinted item detail request exceeded the {}s Scrappa API timeout. Try a smaller batch or run the request again.",
            REQUEST_TIMEOUT_MS / 1_000
        )
    } else {
        error.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        is_actor_level_scrappa_failure, is_retryable_scrappa_error, retry_delay,
        timeout_item_message, ScrappaApiError, ScrappaClient, ScrappaTimeoutError,
    };
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex},
        thread,
        time::Duration,
    };

    fn mock_server(responses: Vec<(u16, String)>) -> (String, Arc<Mutex<Vec<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = requests.clone();
        thread::spawn(move || {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut buffer = [0_u8; 2048];
                loop {
                    let read = stream.read(&mut buffer).unwrap();
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                captured
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&request).to_string());
                let reason = match status {
                    200 => "OK",
                    404 => "Not Found",
                    503 => "Service Unavailable",
                    _ => "Error",
                };
                write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        (format!("http://{address}"), requests)
    }

    #[test]
    fn retries_transient_status_and_sends_scrappa_auth_and_query() {
        let (base_url, requests) = mock_server(vec![
            (503, r#"{"message":"temporarily unavailable"}"#.to_owned()),
            (200, r#"{"data":{"title":"Item"}}"#.to_owned()),
        ]);
        let client = ScrappaClient::with_timeout(
            "test-api-key".to_owned(),
            &base_url,
            Duration::from_secs(1),
        )
        .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let response = runtime
            .block_on(client.get_item_details("123", "DE"))
            .unwrap();

        assert_eq!(response, json!({"data": {"title": "Item"}}));
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("GET /vinted/item-details?item_id=123&country=DE HTTP/1.1"));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("x-api-key: test-api-key"));
        assert!(requests[0].contains("thescrappa-vinted-item-details-scraper/1.0"));
    }

    #[test]
    fn does_not_retry_permanent_api_errors_and_preserves_error_details() {
        let (base_url, requests) =
            mock_server(vec![(404, r#"{"message":"Item missing"}"#.to_owned())]);
        let client =
            ScrappaClient::with_timeout("key".to_owned(), &base_url, Duration::from_secs(1))
                .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let error = runtime
            .block_on(client.get_item_details("123", "DE"))
            .unwrap_err();

        assert_eq!(error.to_string(), "Scrappa API error (404): Item missing");
        assert_eq!(requests.lock().unwrap().len(), 1);
        assert!(!is_actor_level_scrappa_failure(&error));
    }

    #[test]
    fn distinguishes_actor_level_auth_errors_and_timeout_messages() {
        let auth_error = anyhow::Error::new(ScrappaApiError {
            status: 401,
            message: "Unauthorized".to_owned(),
        });
        assert!(is_actor_level_scrappa_failure(&auth_error));
        assert!(!is_retryable_scrappa_error(&auth_error));

        let timeout = anyhow::Error::new(ScrappaTimeoutError::new(90_000));
        assert!(is_retryable_scrappa_error(&timeout));
        assert_eq!(
            timeout_item_message(&timeout),
            "Scrappa API request timed out after 90000ms. The Vinted item detail request exceeded the 90s Scrappa API timeout. Try a smaller batch or run the request again."
        );
        assert_eq!(retry_delay(1).as_millis() >= 2_000, true);
        assert_eq!(retry_delay(1).as_millis() <= 3_000, true);
        assert_eq!(retry_delay(5).as_millis() <= 10_000, true);
    }
}
