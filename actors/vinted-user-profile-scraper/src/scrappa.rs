use std::time::Duration;

use reqwest::{header, Client, Response, StatusCode};
use serde_json::Value;
use url::Url;

use crate::{
    request_params::VintedUserProfileRequest,
    runtime_budget::{retry_delay_ms, SCRAPPA_REQUEST_TIMEOUT_MS},
};

const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const USER_AGENT: &str = "thescrappa-vinted-user-profile-scraper/1.0";
const RETRYABLE_STATUSES: [u16; 6] = [408, 429, 500, 502, 503, 504];

#[derive(Debug)]
pub enum ScrappaError {
    Api { status: u16, message: String },
    Timeout { timeout_ms: u64 },
    Transport(String),
    InvalidResponse(String),
}

impl ScrappaError {
    pub fn is_auth_failure(&self) -> bool {
        matches!(
            self,
            Self::Api {
                status: 401 | 403,
                ..
            }
        )
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::Api { status, .. } => StatusCode::from_u16(*status)
                .ok()
                .is_some_and(is_retryable_status),
            Self::Timeout { .. } | Self::Transport(_) => true,
            Self::InvalidResponse(_) => false,
        }
    }
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Api { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::Timeout { timeout_ms } => {
                write!(
                    formatter,
                    "Scrappa API request timed out after {timeout_ms}ms"
                )
            }
            Self::Transport(message) | Self::InvalidResponse(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for ScrappaError {}

#[derive(Clone)]
pub struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
    timeout_ms: u64,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: Option<&str>) -> anyhow::Result<Self> {
        Self::with_timeout(api_key, base_url, SCRAPPA_REQUEST_TIMEOUT_MS)
    }

    fn with_timeout(
        api_key: String,
        base_url: Option<&str>,
        timeout_ms: u64,
    ) -> anyhow::Result<Self> {
        let base_url = Url::parse(base_url.unwrap_or(SCRAPPA_API_DEFAULT))
            .map_err(|_| anyhow::anyhow!("SCRAPPA_API_BASE_URL must be a valid absolute URL"))?;
        let http = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|error| anyhow::anyhow!("Could not create Scrappa HTTP client: {error}"))?;

        Ok(Self {
            http,
            base_url,
            api_key,
            timeout_ms,
        })
    }

    pub async fn get(
        &self,
        request: &VintedUserProfileRequest,
        attempts: usize,
    ) -> Result<Value, ScrappaError> {
        let attempts = attempts.max(1);

        for attempt in 1..=attempts {
            match self.send(request).await {
                Ok(response) => return Ok(response),
                Err(error) if attempt < attempts && error.is_retryable() => {
                    let delay_ms = retry_delay_ms(attempt - 1);
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{attempts} in {delay_ms}ms.",
                        attempt + 1
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("at least one Scrappa request attempt is configured")
    }

    async fn send(&self, request: &VintedUserProfileRequest) -> Result<Value, ScrappaError> {
        let mut url = self.endpoint()?;
        url.query_pairs_mut()
            .append_pair("user_id", &request.user_id)
            .append_pair("country", &request.country);

        let response = self
            .http
            .get(url)
            .header(header::ACCEPT, "application/json")
            .header("X-API-Key", &self.api_key)
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(|error| map_request_error(error, self.timeout_ms))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = read_error_message(response, self.timeout_ms).await?;
            return Err(ScrappaError::Api { status, message });
        }

        response
            .json()
            .await
            .map_err(|error| map_request_error(error, self.timeout_ms))
    }

    fn endpoint(&self) -> Result<Url, ScrappaError> {
        let mut url = self.base_url.clone();
        let mut path = url.path_segments_mut().map_err(|_| {
            ScrappaError::InvalidResponse("SCRAPPA_API_BASE_URL cannot be a base URL".into())
        })?;
        path.pop_if_empty().extend(["vinted", "user-profile"]);
        drop(path);
        Ok(url)
    }
}

fn map_request_error(error: reqwest::Error, timeout_ms: u64) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout { timeout_ms }
    } else if error.is_decode() {
        ScrappaError::InvalidResponse(error.to_string())
    } else {
        ScrappaError::Transport(error.to_string())
    }
}

async fn read_error_message(response: Response, timeout_ms: u64) -> Result<String, ScrappaError> {
    let fallback = response
        .status()
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", response.status().as_u16()));
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) if error.is_timeout() => {
            return Err(ScrappaError::Timeout { timeout_ms });
        }
        Err(_) => return Ok(fallback),
    };
    if body.is_empty() {
        return Ok(fallback);
    }

    let Ok(data) = serde_json::from_str::<Value>(&body) else {
        return Ok(format_plain_error_body(&body));
    };
    Ok(format_error_data(&data, &fallback))
}

fn format_error_data(data: &Value, fallback: &str) -> String {
    let mut message = data
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or(&fallback)
        .to_owned();
    if let Some(errors) = data.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .filter_map(|(field, messages)| {
                messages.as_array().map(|messages| {
                    format!(
                        "{field}: {}",
                        messages
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
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

fn format_plain_error_body(body: &str) -> String {
    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

pub fn is_retryable_status(status: StatusCode) -> bool {
    RETRYABLE_STATUSES.contains(&status.as_u16())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        thread::{self, JoinHandle},
        time::Duration,
    };

    #[test]
    fn retries_only_transient_scrappa_statuses() {
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_status(StatusCode::from_u16(status).unwrap()));
        }
        for status in [400, 401, 403, 404, 422] {
            assert!(!is_retryable_status(StatusCode::from_u16(status).unwrap()));
        }
    }

    #[test]
    fn classifies_authentication_errors_without_relying_on_message_text() {
        let unauthorized = ScrappaError::Api {
            status: 401,
            message: "credentials rejected".into(),
        };
        let invalid_input = ScrappaError::Api {
            status: 400,
            message: "unauthorized word in validation detail".into(),
        };
        assert!(unauthorized.is_auth_failure());
        assert!(!invalid_input.is_auth_failure());
    }

    #[test]
    fn parses_json_error_details_and_truncates_plain_text() {
        let data = json!({"message": "Invalid input", "errors": {"user_id": ["must exist"]}});
        assert_eq!(
            format_error_data(&data, "Bad Request"),
            "Invalid input - user_id: must exist"
        );

        let long_text = "a ".repeat(400);
        assert_eq!(format_plain_error_body(&long_text).chars().count(), 500);
    }

    #[tokio::test]
    async fn sends_expected_request_and_retries_a_transient_response() {
        let (base_url, server) = mock_server(vec![
            (503, "temporarily unavailable".into(), Duration::ZERO),
            (200, r#"{"success":true}"#.into(), Duration::ZERO),
        ]);
        let base_url = format!("{base_url}/api");
        let client = ScrappaClient::new("test-key".into(), Some(&base_url)).unwrap();
        let request = VintedUserProfileRequest {
            user_id: "255914028".into(),
            country: "DE".into(),
            index: 0,
        };

        assert_eq!(client.get(&request, 2).await.unwrap()["success"], true);
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 2);
        assert!(
            requests[0].starts_with("GET /api/vinted/user-profile?user_id=255914028&country=DE ")
        );
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("x-api-key: test-key"));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("user-agent: thescrappa-vinted-user-profile-scraper/1.0"));
    }

    #[tokio::test]
    async fn enforces_a_request_deadline_and_does_not_retry_invalid_json() {
        let (base_url, server) = mock_server(vec![(
            200,
            r#"{"success":true}"#.into(),
            Duration::from_millis(100),
        )]);
        let client = ScrappaClient::with_timeout("test-key".into(), Some(&base_url), 10).unwrap();
        let request = VintedUserProfileRequest {
            user_id: "255914028".into(),
            country: "FR".into(),
            index: 0,
        };

        assert!(matches!(
            client.get(&request, 1).await.unwrap_err(),
            ScrappaError::Timeout { timeout_ms: 10 }
        ));
        let _ = server.join().unwrap();

        let (base_url, server) = mock_server(vec![(200, "not json".into(), Duration::ZERO)]);
        let client = ScrappaClient::new("test-key".into(), Some(&base_url)).unwrap();
        assert!(matches!(
            client.get(&request, 2).await.unwrap_err(),
            ScrappaError::InvalidResponse(_)
        ));
        assert_eq!(server.join().unwrap().len(), 1);
    }

    fn mock_server(responses: Vec<(u16, String, Duration)>) -> (String, JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let base_url = format!("http://{address}");
        let thread = thread::spawn(move || {
            let mut requests = Vec::with_capacity(responses.len());
            for (status, body, delay) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                requests.push(read_request(&mut stream));
                thread::sleep(delay);
                let response = format!(
                    "HTTP/1.1 {status} Mock Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
            requests
        });
        (base_url, thread)
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1_024];
        loop {
            let count = stream.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8_lossy(&request).to_string()
    }
}
