use anyhow::{Context, Result, anyhow};
use reqwest::{Client, StatusCode, Url};
use serde_json::{Map, Value};
use std::time::Duration;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub struct ScrappaClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    pub fn new(api_key: String, base_url: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .context("Could not create Scrappa HTTP client")?;
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key,
        })
    }

    pub async fn get(&self, endpoint: &str, params: &Map<String, Value>) -> Result<Value> {
        let mut url = Url::parse(&format!("{}{endpoint}", self.base_url))
            .context("Scrappa API base URL must be valid")?;
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
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(map_request_error)?;
        let status = response.status();
        let body = response.text().await.map_err(map_request_error)?;
        if !status.is_success() {
            return Err(anyhow!(format_api_error(status, &body)));
        }
        serde_json::from_str(&body).context("Scrappa API response was not valid JSON")
    }
}

fn map_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            REQUEST_TIMEOUT.as_millis()
        )
    } else {
        anyhow!("Scrappa API request failed: {error}")
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
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".into(),
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
            if let Some(Value::Object(errors)) = data.get("errors") {
                let details = errors
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
                    .join("; ");
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
                body.to_owned()
            }
        }
        Ok(_) => format!("HTTP {status_code}"),
    };
    format!("Scrappa API error ({status_code}): {message}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_scrappa_error_details_and_status() {
        assert_eq!(
            format_api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                r#"{"message":"Invalid input","errors":{"query":["is required","must be text"]}}"#
            ),
            "Scrappa API error (422): Invalid input - query: is required, must be text"
        );
        assert_eq!(
            format_api_error(StatusCode::BAD_GATEWAY, "upstream unavailable"),
            "Scrappa API error (502): upstream unavailable"
        );
    }

    #[test]
    fn applies_the_existing_upstream_deadline_without_application_retries() {
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(60));
    }

    #[test]
    fn preserves_full_plain_text_error_bodies() {
        let body = "x".repeat(1_000);
        assert_eq!(
            format_api_error(StatusCode::BAD_GATEWAY, &body),
            format!("Scrappa API error (502): {body}")
        );
    }
}
