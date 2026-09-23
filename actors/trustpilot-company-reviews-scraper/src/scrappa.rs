use anyhow::{Result, anyhow};
use reqwest::{Client, StatusCode, Url};
use serde_json::Value;
use std::{error::Error, fmt, time::Duration};

pub const REQUEST_TIMEOUT_MS: u64 = 90_000;

#[derive(Debug)]
pub struct ScrappaTimeoutError;

impl fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {REQUEST_TIMEOUT_MS}ms"
        )
    }
}

impl Error for ScrappaTimeoutError {}

pub struct ScrappaClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(REQUEST_TIMEOUT_MS))
            .build()?;
        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }

    pub async fn get(
        &self,
        endpoint: &str,
        params: &serde_json::Map<String, Value>,
    ) -> Result<Value> {
        let base_url = self.base_url.trim_end_matches('/');
        let mut url = Url::parse(&format!("{base_url}{endpoint}"))?;
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in params {
                if value.is_null() || value.as_str() == Some("") {
                    continue;
                }
                query.append_pair(key, &parameter_string(value));
            }
        }
        let response = self
            .client
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(map_request_error)?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.map_err(map_request_error)?;
            return Err(anyhow!("{}", format_api_error(status, &body)));
        }
        response.json().await.map_err(map_request_error)
    }
}

fn map_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        ScrappaTimeoutError.into()
    } else {
        error.into()
    }
}

fn parameter_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => "[object Object]".into(),
    }
}

fn format_api_error(status: StatusCode, body: &str) -> String {
    let status_code = status.as_u16();
    let message = match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(data)) => {
            let mut message = data
                .get("message")
                .filter(|value| !value.is_null())
                .map(js_string)
                .unwrap_or_else(|| format!("HTTP {status_code}"));
            if let Some(errors) = data.get("errors").filter(|errors| is_truthy(errors)) {
                let details = match errors {
                    Value::Object(fields) => fields
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
                                .unwrap_or_default();
                            format!("{field}: {messages}")
                        })
                        .collect::<Vec<_>>()
                        .join("; "),
                    _ => String::new(),
                };
                if !details.is_empty() {
                    message.push_str(" - ");
                    message.push_str(&details);
                }
            }
            message
        }
        Ok(Value::Null) | Err(_) => {
            if body.is_empty() {
                format!("HTTP {status_code}")
            } else {
                body.chars().take(500).collect()
            }
        }
        Ok(_) => format!("HTTP {status_code}"),
    };
    format!("Scrappa API error ({status_code}): {message}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_error_is_bounded_without_losing_status() {
        let message = format_api_error(StatusCode::BAD_GATEWAY, &"x".repeat(5_000));
        assert!(message.starts_with("Scrappa API error (502): "));
        assert_eq!(
            message.chars().count(),
            "Scrappa API error (502): ".len() + 500
        );
    }
}
