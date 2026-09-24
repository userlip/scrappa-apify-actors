use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client};
use serde_json::Value;
use url::Url;

use crate::{
    challenge::{challenge_url, ChallengeRequest},
    config::{ActorConfig, SCRAPPA_REQUEST_TIMEOUT},
};

pub(crate) struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
}

impl ScrappaClient {
    pub(crate) fn new(config: &ActorConfig) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(SCRAPPA_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Scrappa HTTP client")?,
            base_url: config.scrappa_api_base_url.clone(),
            api_key: config.scrappa_api_key.clone(),
        })
    }

    pub(crate) async fn fetch(&self, request: &ChallengeRequest) -> Result<Value> {
        let url = challenge_url(&self.base_url, request)?;
        let response = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    anyhow!(
                        "Scrappa API request timed out after {}ms",
                        SCRAPPA_REQUEST_TIMEOUT.as_millis()
                    )
                } else {
                    anyhow!("Scrappa API request failed: {error}")
                }
            })?;
        if !response.status().is_success() {
            let status = response.status();
            let fallback = status.canonical_reason().unwrap_or("Unknown status");
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Scrappa API error ({}): {}",
                status.as_u16(),
                scrappa_error_message(&body, fallback, status.as_u16())
            ));
        }
        response
            .json()
            .await
            .context("Scrappa API returned invalid JSON")
    }
}

pub(crate) fn scrappa_error_message(body: &str, fallback: &str, status: u16) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }
    if let Ok(error) = serde_json::from_str::<Value>(body) {
        let mut message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_owned();
        if let Some(errors) = error.get("errors").and_then(Value::as_object) {
            let detail = errors
                .iter()
                .map(|(field, messages)| {
                    let messages = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {messages}")
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !detail.is_empty() {
                message.push_str(" - ");
                message.push_str(&detail);
            }
        }
        return message;
    }
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        format!("HTTP {status}")
    } else {
        compact.chars().take(500).collect()
    }
}

pub(crate) fn challenge_error(response: &Value) -> Option<String> {
    let code = response.get("code")?;
    if code.as_f64() == Some(0.0) {
        return None;
    }
    let code = serde_json::to_string(code).unwrap_or_else(|_| "null".to_owned());
    let message = response
        .get("msg")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or_else(|| "Unknown error".to_owned());
    Some(format!(
        "Scrappa TikTok Challenge Details API returned code {code}: {message}"
    ))
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}
