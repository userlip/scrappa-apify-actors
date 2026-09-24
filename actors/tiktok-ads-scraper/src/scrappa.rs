use crate::urls::endpoint_url;
use anyhow::{anyhow, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::time::Duration;
use url::Url;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
}

impl ScrappaClient {
    pub fn new(http: Client, base_url: Url, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    pub async fn get_ad(&self, url: &str) -> Result<Value> {
        let endpoint = endpoint_url(&self.base_url, &["tiktok", "ads", "details"])?;
        let response = self
            .http
            .get(endpoint)
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .query(&[("url", url)])
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    timeout_error()
                } else {
                    anyhow!("Scrappa API request failed: {error}")
                }
            })?;

        let response = require_success(response).await?;
        match response.json::<Value>().await {
            Ok(value) => Ok(value),
            Err(error) if error.is_timeout() => Err(timeout_error()),
            Err(error) => Err(anyhow!("Scrappa API response was not valid JSON: {error}")),
        }
    }
}

async fn require_success(response: Response) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let fallback = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.map_err(|error| {
        if error.is_timeout() {
            timeout_error()
        } else {
            anyhow!("Failed to read Scrappa API error response: {error}")
        }
    })?;
    let message = if body.is_empty() {
        fallback.to_owned()
    } else {
        error_message(&body, fallback)
    };
    Err(anyhow!(
        "Scrappa API error ({}): {message}",
        status.as_u16()
    ))
}

fn timeout_error() -> anyhow::Error {
    anyhow!(
        "Scrappa API request timed out after {}ms",
        REQUEST_TIMEOUT.as_millis()
    )
}

fn error_message(body: &str, fallback: &str) -> String {
    if let Ok(error) = serde_json::from_str::<Value>(body) {
        let mut message = error
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or_else(|| fallback.to_owned());
        if let Some(errors) = error.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let joined = match messages {
                        Value::Array(messages) => messages
                            .iter()
                            .map(js_string)
                            .collect::<Vec<_>>()
                            .join(", "),
                        value => js_string(value),
                    };
                    format!("{field}: {joined}")
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

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::Null => String::new(),
                _ => js_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{error_message, timeout_error, REQUEST_TIMEOUT};

    #[test]
    fn formats_structured_scrappa_errors() {
        assert_eq!(
            error_message(
                r#"{"message":"Invalid request","errors":{"url":["is required","must be a URL"]}}"#,
                "Bad Request"
            ),
            "Invalid request - url: is required, must be a URL"
        );
    }

    #[test]
    fn preserves_plain_text_errors_and_fallback_status() {
        assert_eq!(
            error_message("upstream unavailable", "Service Unavailable"),
            "upstream unavailable"
        );
        assert_eq!(error_message("{}", "Bad Request"), "Bad Request");
    }

    #[test]
    fn clips_long_plain_text_errors() {
        assert_eq!(error_message(&"x".repeat(600), "Bad Request").len(), 500);
    }

    #[test]
    fn preserves_the_scrappa_request_deadline() {
        assert_eq!(REQUEST_TIMEOUT.as_secs(), 60);
        assert_eq!(
            timeout_error().to_string(),
            "Scrappa API request timed out after 60000ms"
        );
    }
}
