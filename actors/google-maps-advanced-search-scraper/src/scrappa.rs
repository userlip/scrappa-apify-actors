use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, StatusCode};
use serde_json::Value;

use crate::{config::Config, input::build_search_url};

fn scrappa_error_message(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    if body.is_empty() {
        return fallback;
    }

    if let Ok(data) = serde_json::from_str::<Value>(body) {
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
                        let joined = messages
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{field}: {joined}")
                    })
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

pub(crate) async fn fetch_search(http: &Client, config: &Config, input: &Value) -> Result<Value> {
    let url = build_search_url(input, &config.scrappa_api_base_url)?;
    let response = http
        .get(url)
        .timeout(config.scrappa_request_timeout)
        .header("X-API-Key", &config.scrappa_api_key)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "Scrappa API request timed out after {}ms",
                    config.scrappa_request_timeout.as_millis()
                )
            } else {
                anyhow!(error.to_string())
            }
        })?;

    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read Scrappa API response")?;
    if !status.is_success() {
        bail!(
            "Scrappa API error ({}): {}",
            status.as_u16(),
            scrappa_error_message(status, &body)
        );
    }
    serde_json::from_str(&body).context("Scrappa API returned invalid JSON")
}
