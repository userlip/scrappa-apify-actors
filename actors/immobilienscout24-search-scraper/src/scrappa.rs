use crate::{
    config::{SCRAPPA_MAX_ATTEMPTS, SCRAPPA_REQUEST_TIMEOUT},
    endpoint::endpoint_url,
    input::SearchParams,
};
use anyhow::{anyhow, Result};
use reqwest::{header, Client};
use serde_json::{json, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

#[derive(Debug)]
pub(crate) enum ScrappaError {
    Api { status: u16, message: String },
    Timeout,
    Request(String),
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Api { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::Timeout => write!(
                formatter,
                "Scrappa API request timed out after {}ms",
                SCRAPPA_REQUEST_TIMEOUT.as_millis()
            ),
            Self::Request(message) => formatter.write_str(message),
        }
    }
}

pub(crate) struct ScrappaClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    pub(crate) fn new(http: Client, base_url: String, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    pub(crate) async fn search(
        &self,
        params: &SearchParams,
    ) -> std::result::Result<Value, ScrappaError> {
        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send_search(params).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt >= SCRAPPA_MAX_ATTEMPTS || !is_retryable_scrappa_error(&error) {
                        return Err(error);
                    }
                    let delay = scrappa_retry_delay(attempt);
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {}ms.",
                        attempt + 1,
                        delay.as_millis()
                    );
                    sleep(delay).await;
                }
            }
        }
        unreachable!("the retry loop returns on success or final failure")
    }

    async fn send_search(&self, params: &SearchParams) -> std::result::Result<Value, ScrappaError> {
        let mut url = endpoint_url(&self.base_url, &["immobilienscout24", "search"])
            .map_err(|error| ScrappaError::Request(error.to_string()))?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params.query_pairs() {
                query.append_pair(&key, &value);
            }
        }
        eprintln!("[Scrappa] GET {url}");
        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .header(
                header::USER_AGENT,
                "thescrappa-immobilienscout24-search-scraper/1.0",
            )
            .send()
            .await
            .map_err(scrappa_transport_error)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let fallback = response
                .status()
                .canonical_reason()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("HTTP {status}"));
            let body = match response.text().await {
                Ok(body) => body,
                Err(error) if error.is_timeout() => return Err(ScrappaError::Timeout),
                Err(_) => String::new(),
            };
            return Err(ScrappaError::Api {
                status,
                message: scrappa_error_message(&body, &fallback),
            });
        }

        response
            .json::<Value>()
            .await
            .map_err(scrappa_transport_error)
    }
}

fn scrappa_transport_error(error: reqwest::Error) -> ScrappaError {
    if error.is_timeout() {
        ScrappaError::Timeout
    } else {
        ScrappaError::Request(error.to_string())
    }
}

pub(crate) fn is_retryable_scrappa_error(error: &ScrappaError) -> bool {
    match error {
        ScrappaError::Timeout => true,
        ScrappaError::Api { status, .. } => {
            matches!(*status, 408 | 429 | 500 | 502 | 503 | 504)
        }
        ScrappaError::Request(_) => false,
    }
}

pub(crate) fn scrappa_retry_delay(failed_attempt: usize) -> Duration {
    let jitter_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_millis() as u64;
    let base_ms = 1000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt as u32));
    Duration::from_millis((base_ms + jitter_ms).min(10_000))
}

pub(crate) fn scrappa_error_message(body: &str, fallback: &str) -> String {
    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let message = error_data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(fallback);
        let mut result = message.to_owned();
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let messages = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .map(value_to_js_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {messages}")
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                result.push_str(" - ");
                result.push_str(&details);
            }
        }
        return result;
    }

    let body = body.trim();
    if body.is_empty() {
        fallback.to_owned()
    } else {
        body.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(500)
            .collect()
    }
}

fn value_to_js_string(value: &Value) -> String {
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
                    value_to_js_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

pub(crate) fn is_handled_empty_search_error(error: &ScrappaError) -> bool {
    let ScrappaError::Api { status, message } = error else {
        return false;
    };
    if *status == 502 {
        return true;
    }
    if *status != 400 {
        return false;
    }
    let message = message.to_ascii_lowercase();
    message.contains("invalid_location")
        || message
            .split_once("location ")
            .is_some_and(|(_, rest)| rest.contains("not found"))
}

pub(crate) async fn search_immobilienscout24(
    client: &ScrappaClient,
    params: &SearchParams,
) -> Result<Value> {
    match client.search(params).await {
        Ok(response) => Ok(response),
        Err(error) if is_handled_empty_search_error(&error) => {
            let (status, message) = match &error {
                ScrappaError::Api { status, message } => (Some(*status), message.clone()),
                _ => unreachable!("only API errors are mapped as empty search results"),
            };
            eprintln!(
                "Scrappa ImmobilienScout24 search returned no usable result; saving a clean zero-result output. {}",
                json!({
                    "status": status,
                    "message": message,
                    "request_location": params.location,
                    "request_type": params.property_type,
                })
            );
            Ok(json!({
                "success": false,
                "total_results": 0,
                "page": params.page,
                "total_pages": 0,
                "results": [],
                "error": {
                    "message": message,
                    "status": status,
                },
            }))
        }
        Err(error) => {
            let message = match error {
                ScrappaError::Timeout => format!(
                    "{}. The ImmobilienScout24 request exceeded the {}s Scrappa API timeout. Try a smaller page size or run the request again.",
                    ScrappaError::Timeout,
                    SCRAPPA_REQUEST_TIMEOUT.as_secs()
                ),
                error => error.to_string(),
            };
            Err(anyhow!(message))
        }
    }
}
