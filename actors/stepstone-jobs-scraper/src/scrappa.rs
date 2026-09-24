use anyhow::{Error, Result, anyhow};
use reqwest::{Response, Url};
use serde_json::{Map, Value};
use std::{
    error::Error as StdError,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const RETRYABLE_STATUS_CODES: [u16; 6] = [408, 429, 500, 502, 503, 504];

pub struct ScrappaClient {
    client: reqwest::Client,
    api_key: String,
    base_url: Url,
    timeout: Duration,
    max_attempts: usize,
    max_retry_delay_ms: u64,
}

impl ScrappaClient {
    pub fn new(
        api_key: String,
        base_url: &str,
        timeout: Duration,
        max_attempts: usize,
        max_retry_delay_ms: u64,
    ) -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .build()
                .map_err(|error| anyhow!("Could not create Scrappa HTTP client: {error}"))?,
            api_key,
            base_url: Url::parse(base_url).map_err(|error| {
                anyhow!("SCRAPPA_API_BASE_URL must be a valid absolute URL: {error}")
            })?,
            timeout,
            max_attempts: max_attempts.max(1),
            max_retry_delay_ms,
        })
    }

    pub async fn get(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        let url = self.request_url(endpoint, params)?;
        let mut last_error = None;

        for attempt in 1..=self.max_attempts {
            match self.send(&url).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt >= self.max_attempts || !is_retryable(&error) {
                        return Err(error);
                    }

                    let retry_after_ms = error
                        .downcast_ref::<ScrappaApiError>()
                        .and_then(|error| error.retry_after_ms);
                    let delay_ms = get_retry_delay_ms(
                        attempt,
                        jitter_ms(),
                        retry_after_ms,
                        self.max_retry_delay_ms,
                    );
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{} in {}ms.",
                        error,
                        attempt + 1,
                        self.max_attempts,
                        delay_ms
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Scrappa API request did not complete")))
    }

    async fn send(&self, url: &Url) -> Result<Value> {
        let response = self
            .client
            .get(url.clone())
            .timeout(self.timeout)
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .header("User-Agent", "thescrappa-stepstone-jobs-scraper/1.0")
            .send()
            .await
            .map_err(|error| request_error(error, self.timeout))?;

        if !response.status().is_success() {
            let status = response.status();
            let retry_after_ms = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| parse_retry_after_ms(value, SystemTime::now()));
            let message = read_error_message(response, self.timeout).await?;
            return Err(Error::new(ScrappaApiError {
                status_code: status.as_u16(),
                message,
                retry_after_ms,
            }));
        }

        response
            .json()
            .await
            .map_err(|error| request_error(error, self.timeout))
            .map_err(|error| {
                if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
                    error
                } else {
                    anyhow!("Scrappa API response was not valid JSON: {error}")
                }
            })
    }

    fn request_url(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.set_query(None);
        url.set_fragment(None);
        let segments = endpoint
            .trim_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty());
        url.path_segments_mut()
            .map_err(|_| anyhow!("SCRAPPA_API_BASE_URL cannot be a base URL"))?
            .pop_if_empty()
            .extend(segments);

        let query_pairs = params
            .iter()
            .filter(|(_, value)| {
                !value.is_null() && value.as_str().is_none_or(|string| !string.is_empty())
            })
            .map(|(key, value)| {
                let value = match value {
                    Value::Bool(value) => if *value { "1" } else { "0" }.to_owned(),
                    _ => javascript_string(value),
                };
                (key, value)
            })
            .collect::<Vec<_>>();
        if !query_pairs.is_empty() {
            let mut query = url.query_pairs_mut();
            for (key, value) in query_pairs {
                query.append_pair(key, &value);
            }
        }
        Ok(url)
    }
}

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    pub(crate) timeout: Duration,
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

impl StdError for ScrappaTimeoutError {}

#[derive(Debug)]
struct ScrappaApiError {
    status_code: u16,
    message: String,
    retry_after_ms: Option<u64>,
}

impl fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status_code, self.message
        )
    }
}

impl StdError for ScrappaApiError {}

fn request_error(error: reqwest::Error, timeout: Duration) -> Error {
    if error.is_timeout() {
        Error::new(ScrappaTimeoutError { timeout })
    } else {
        anyhow!("Scrappa API request failed: {error}")
    }
}

fn is_retryable(error: &Error) -> bool {
    error.downcast_ref::<ScrappaTimeoutError>().is_some()
        || error
            .downcast_ref::<ScrappaApiError>()
            .is_some_and(|error| RETRYABLE_STATUS_CODES.contains(&error.status_code))
}

pub fn get_retry_delay_ms(
    failed_attempt: usize,
    jitter_ms: u64,
    retry_after_ms: Option<u64>,
    max_retry_delay_ms: u64,
) -> u64 {
    let exponential_delay_ms = 1000u64
        .saturating_mul(2u64.saturating_pow(failed_attempt.min(u32::MAX as usize) as u32))
        .saturating_add(jitter_ms);
    exponential_delay_ms
        .max(retry_after_ms.unwrap_or_default())
        .min(max_retry_delay_ms)
}

fn jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64
        % 1000
}

fn parse_retry_after_ms(value: &str, now: SystemTime) -> Option<u64> {
    let seconds = value.parse::<f64>().ok();
    if let Some(seconds) = seconds.filter(|seconds| seconds.is_finite() && *seconds >= 0.0) {
        let milliseconds = seconds * 1000.0;
        return Some(if milliseconds >= u64::MAX as f64 {
            u64::MAX
        } else {
            milliseconds as u64
        });
    }

    let retry_at = httpdate::parse_http_date(value).ok()?;
    let delay = retry_at.duration_since(now).unwrap_or(Duration::ZERO);
    Some(delay.as_millis().min(u64::MAX as u128) as u64)
}

async fn read_error_message(response: Response, timeout: Duration) -> Result<String> {
    let fallback = response
        .status()
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", response.status().as_u16()));
    let body = response
        .text()
        .await
        .map_err(|error| request_error(error, timeout))?;
    if body.is_empty() {
        return Ok(fallback);
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .or_else(|| error_data.pointer("/error/message").and_then(Value::as_str))
            .unwrap_or(&fallback)
            .to_owned();
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    let messages = messages.as_array()?;
                    let messages = messages
                        .iter()
                        .map(javascript_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    Some(format!("{field}: {messages}"))
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

    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    Ok(collapsed.chars().take(500).collect())
}

fn javascript_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    javascript_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockResponse, MockServer, has_header, request_parts};
    use serde_json::json;

    fn client(server: &MockServer, timeout_ms: u64, max_retry_delay_ms: u64) -> ScrappaClient {
        ScrappaClient::new(
            "test-key".to_owned(),
            &format!("{}/api", server.base_url),
            Duration::from_millis(timeout_ms),
            4,
            max_retry_delay_ms,
        )
        .unwrap()
    }

    #[test]
    fn retries_only_timeouts_and_transient_api_statuses() {
        assert!(is_retryable(&Error::new(ScrappaTimeoutError {
            timeout: Duration::from_secs(1),
        })));
        assert!(is_retryable(&Error::new(ScrappaApiError {
            status_code: 429,
            message: "busy".to_owned(),
            retry_after_ms: None,
        })));
        assert!(is_retryable(&Error::new(ScrappaApiError {
            status_code: 503,
            message: "busy".to_owned(),
            retry_after_ms: None,
        })));
        assert!(!is_retryable(&Error::new(ScrappaApiError {
            status_code: 400,
            message: "bad request".to_owned(),
            retry_after_ms: None,
        })));
        assert!(!is_retryable(&anyhow!("Scrappa API error (503): forged")));
    }

    #[test]
    fn calculates_bounded_delays_and_honors_both_retry_after_formats() {
        assert_eq!(get_retry_delay_ms(1, 0, None, 60_000), 2_000);
        assert_eq!(get_retry_delay_ms(2, 250, None, 60_000), 4_250);
        assert_eq!(get_retry_delay_ms(1, 0, Some(15_000), 60_000), 15_000);
        assert_eq!(get_retry_delay_ms(1, 0, Some(60_000), 20_000), 20_000);
        assert_eq!(parse_retry_after_ms("1.5", UNIX_EPOCH), Some(1500));
        assert_eq!(parse_retry_after_ms("not a date", UNIX_EPOCH), None);

        let now = httpdate::parse_http_date("Wed, 21 Oct 2015 07:28:00 GMT").unwrap();
        assert_eq!(
            parse_retry_after_ms("Wed, 21 Oct 2015 07:28:02 GMT", now),
            Some(2000)
        );
    }

    #[tokio::test]
    async fn sends_api_key_and_boolean_query_values() {
        let server = MockServer::start(vec![
            MockResponse::json(200, json!({"success": true})),
            MockResponse::json(200, json!({"success": true})),
        ]);
        let client = client(&server, 1000, 0);
        let false_params = json!({"query": "Software Entwickler", "work_from_home": false});
        let true_params = json!({"query": "Software Entwickler", "work_from_home": true});

        client
            .get("/stepstone/jobs", false_params.as_object().unwrap())
            .await
            .unwrap();
        client
            .get("/stepstone/jobs", true_params.as_object().unwrap())
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            request_parts(&requests[0]).1.split('?').next(),
            Some("/api/stepstone/jobs")
        );
        assert_eq!(
            Url::parse(&format!(
                "{}{}",
                server.base_url,
                request_parts(&requests[0]).1
            ))
            .unwrap()
            .query_pairs()
            .find(|(key, _)| key == "work_from_home")
            .unwrap()
            .1,
            "0"
        );
        assert_eq!(
            Url::parse(&format!(
                "{}{}",
                server.base_url,
                request_parts(&requests[1]).1
            ))
            .unwrap()
            .query_pairs()
            .find(|(key, _)| key == "work_from_home")
            .unwrap()
            .1,
            "1"
        );
        assert!(has_header(&requests[0], "X-API-Key", "test-key"));
        assert!(has_header(&requests[0], "Accept", "application/json"));
        assert!(has_header(
            &requests[0],
            "User-Agent",
            "thescrappa-stepstone-jobs-scraper/1.0"
        ));
    }

    #[tokio::test]
    async fn retries_a_timeout_and_then_returns_the_successful_response() {
        let server = MockServer::start(vec![
            MockResponse::json(200, json!({"ignored": true})).delay_ms(150),
            MockResponse::json(200, json!({"success": true})),
        ]);
        let client = client(&server, 100, 0);
        let result = client.get("/stepstone/jobs", &Map::new()).await.unwrap();

        assert_eq!(result, json!({"success": true}));
        assert_eq!(server.requests().len(), 2);
    }

    #[tokio::test]
    async fn retries_transient_errors_four_times_and_preserves_nested_error_text() {
        let server = MockServer::start(vec![
            MockResponse::json(
                503,
                json!({"error": {"message": "Stepstone is temporarily unavailable."}}),
            ),
            MockResponse::json(
                503,
                json!({"error": {"message": "Stepstone is temporarily unavailable."}}),
            ),
            MockResponse::json(
                503,
                json!({"error": {"message": "Stepstone is temporarily unavailable."}}),
            ),
            MockResponse::json(
                503,
                json!({"error": {"message": "Stepstone is temporarily unavailable."}}),
            ),
        ]);
        let client = client(&server, 1000, 0);

        let error = client
            .get("/stepstone/jobs", &Map::new())
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Stepstone is temporarily unavailable.")
        );
        assert_eq!(server.requests().len(), 4);
    }

    #[tokio::test]
    async fn retries_a_rate_limit_response_with_retry_after() {
        let server = MockServer::start(vec![
            MockResponse::json(429, json!({"message": "Too Many Requests"}))
                .header("Retry-After", "60"),
            MockResponse::json(200, json!({"success": true})),
        ]);
        let client = client(&server, 1000, 0);

        assert_eq!(
            client.get("/stepstone/jobs", &Map::new()).await.unwrap(),
            json!({"success": true})
        );
        assert_eq!(server.requests().len(), 2);
    }

    #[tokio::test]
    async fn does_not_retry_non_transient_api_errors() {
        let server = MockServer::start(vec![MockResponse::json(
            400,
            json!({"message": "Invalid job type", "errors": {"job_type": ["unknown value"]}}),
        )]);
        let client = client(&server, 1000, 0);

        let error = client
            .get("/stepstone/jobs", &Map::new())
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (400): Invalid job type - job_type: unknown value"
        );
        assert_eq!(server.requests().len(), 1);
    }
}
