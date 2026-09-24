use anyhow::{Error, Result};
#[cfg(test)]
use reqwest::StatusCode;
use reqwest::{header, Client, Response, Url};
use serde_json::{Map, Value};
use std::{
    error::Error as StdError,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const REQUEST_TIMEOUT_MS: u64 = 60_000;
pub const MAX_ATTEMPTS: u32 = 3;
pub const DEFAULT_BASE_URL: &str = "https://scrappa.co/api";
const USER_AGENT: &str = "thescrappa-kununu-jobs-scraper/1.0";

#[derive(Debug)]
pub struct ScrappaTimeoutError {
    timeout_ms: u64,
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

impl StdError for ScrappaTimeoutError {}

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

impl StdError for ScrappaApiError {}

pub struct ScrappaClient {
    client: Client,
    base_url: Url,
    api_key: String,
    timeout: Duration,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: &str) -> Result<Self> {
        let base_url = Url::parse(base_url)?;
        let client = Client::builder()
            .timeout(Duration::from_millis(REQUEST_TIMEOUT_MS))
            .build()?;
        Ok(Self {
            client,
            base_url,
            api_key,
            timeout: Duration::from_millis(REQUEST_TIMEOUT_MS),
        })
    }

    pub async fn get(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        let url = build_url(&self.base_url, endpoint, params)?;
        let mut last_error = None;

        for attempt in 1..=MAX_ATTEMPTS {
            match self.send(&url).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let retryable = is_retryable_scrappa_error(&error);
                    last_error = Some(error);
                    if !retryable || attempt == MAX_ATTEMPTS {
                        break;
                    }
                    let jitter_ms = random_jitter_ms();
                    let delay_ms = get_retry_delay_ms(attempt, jitter_ms);
                    let error = last_error.as_ref().expect("the failed error was saved");
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{MAX_ATTEMPTS} in {delay_ms}ms.",
                        attempt + 1,
                        error
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }

        Err(last_error.expect("the request loop always makes at least one attempt"))
    }

    async fn send(&self, url: &Url) -> Result<Value> {
        let response = self
            .client
            .get(url.clone())
            .header(header::ACCEPT, "application/json")
            .header("X-API-Key", &self.api_key)
            .header(header::USER_AGENT, USER_AGENT)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|error| map_transport_error(error, self.timeout.as_millis() as u64))?;

        if !response.status().is_success() {
            return Err(ScrappaApiError {
                status: response.status().as_u16(),
                message: read_error_message(response).await,
            }
            .into());
        }

        match response.json::<Value>().await {
            Ok(response) => Ok(response),
            Err(error) => Err(map_transport_error(error, self.timeout.as_millis() as u64)),
        }
    }
}

fn build_url(base_url: &Url, endpoint: &str, params: &Map<String, Value>) -> Result<Url> {
    let mut url = Url::parse(&format!(
        "{}{endpoint}",
        base_url.as_str().trim_end_matches('/')
    ))?;
    let mut pairs = url.query_pairs_mut();
    for (key, value) in params {
        if value.is_null() || value.as_str() == Some("") {
            continue;
        }
        match value {
            Value::Array(values) => {
                for value in values {
                    if value.is_null() || value.as_str() == Some("") {
                        continue;
                    }
                    pairs.append_pair(&format!("{key}[]"), &query_value(value));
                }
            }
            value => {
                pairs.append_pair(key, &query_value(value));
            }
        };
    }
    drop(pairs);
    Ok(url)
}

fn query_value(value: &Value) -> String {
    match value {
        Value::Bool(value) => if *value { "1" } else { "0" }.to_owned(),
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::Null => String::new(),
                _ => query_value(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
        Value::Null => "null".to_owned(),
    }
}

async fn read_error_message(response: Response) -> String {
    let status = response.status();
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();

    let Ok(body) = response.text().await else {
        return fallback;
    };
    if body.is_empty() {
        return fallback;
    }
    if content_type.contains("application/json") {
        if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
            let mut message = error_data
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or(&fallback)
                .to_owned();
            if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
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
    }

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn map_transport_error(error: reqwest::Error, timeout_ms: u64) -> Error {
    if error.is_timeout() {
        ScrappaTimeoutError { timeout_ms }.into()
    } else {
        error.into()
    }
}

fn is_retryable_scrappa_error(error: &Error) -> bool {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| matches!(error.status, 408 | 429 | 500 | 502 | 503 | 504))
}

fn random_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| u64::from(time.subsec_nanos() % 1000))
        .unwrap_or_default()
}

fn get_retry_delay_ms(failed_attempt: u32, jitter_ms: u64) -> u64 {
    let backoff_ms = 1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt));
    backoff_ms.saturating_add(jitter_ms).min(10_000)
}

#[cfg(test)]
fn status_error(status: StatusCode, message: &str) -> Error {
    ScrappaApiError {
        status: status.as_u16(),
        message: message.to_owned(),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::{
        build_url, get_retry_delay_ms, is_retryable_scrappa_error, status_error,
        ScrappaTimeoutError,
    };
    use anyhow::Error;
    use serde_json::{json, Map, Value};
    use std::time::Duration;
    use url::Url;

    #[test]
    fn serializes_boolean_and_array_query_values_like_scrappa_api() {
        let params: Map<String, Value> = serde_json::from_value(json!({
            "query":"software engineer",
            "is_top_company":false,
            "workplace":["FULL_REMOTE","PARTLY_REMOTE"],
            "benefits":["flexWorkingHours","pensionPlan"],
            "ignored":null,
            "empty":""
        }))
        .unwrap();
        let url = build_url(
            &Url::parse("https://example.test/api").unwrap(),
            "/kununu/jobs",
            &params,
        )
        .unwrap();
        let query = url.query_pairs().into_owned().collect::<Vec<_>>();
        assert!(query.contains(&("is_top_company".to_owned(), "0".to_owned())));
        assert!(query.contains(&("workplace[]".to_owned(), "FULL_REMOTE".to_owned())));
        assert!(query.contains(&("workplace[]".to_owned(), "PARTLY_REMOTE".to_owned())));
        assert!(query
            .iter()
            .all(|(key, _)| key != "ignored" && key != "empty"));
    }

    #[test]
    fn retries_only_timeouts_and_retryable_http_statuses() {
        assert!(is_retryable_scrappa_error(&Error::new(
            ScrappaTimeoutError { timeout_ms: 60_000 }
        )));
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_scrappa_error(&status_error(
                reqwest::StatusCode::from_u16(status).unwrap(),
                "upstream error"
            )));
        }
        assert!(!is_retryable_scrappa_error(&status_error(
            reqwest::StatusCode::BAD_REQUEST,
            "bad request"
        )));
        assert_eq!(get_retry_delay_ms(1, 250), 2_250);
        assert_eq!(get_retry_delay_ms(2, 999), 4_999);
        assert_eq!(get_retry_delay_ms(5, 999), 10_000);
        assert_eq!(
            Duration::from_millis(get_retry_delay_ms(1, 250)).as_secs(),
            2
        );
    }
}
