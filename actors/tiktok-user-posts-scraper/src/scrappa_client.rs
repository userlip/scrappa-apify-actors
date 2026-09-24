use anyhow::{anyhow, bail, Result};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use std::time::Duration;

use crate::apify_client::endpoint_url;
use crate::params::{is_js_whitespace, TikTokUserPostsParams};
use crate::response::js_string;
use crate::ActorConfig;

pub(crate) const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const SCRAPPA_ENDPOINT: [&str; 3] = ["tiktok", "user", "posts"];
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) async fn scrappa_response(
    client: &Client,
    config: &ActorConfig,
    params: &TikTokUserPostsParams,
) -> Result<Value> {
    let mut url = endpoint_url(&config.scrappa_api_base_url, &SCRAPPA_ENDPOINT)?;
    params.append_to_url(&mut url);
    let response = client
        .get(url)
        .header("X-API-Key", &config.scrappa_api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(scrappa_request_error)?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let message = read_scrappa_error(response).await?;
        bail!("Scrappa API error ({status}): {message}");
    }
    response.json().await.map_err(scrappa_request_error)
}

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            SCRAPPA_REQUEST_TIMEOUT.as_millis()
        )
    } else {
        error.into()
    }
}

async fn read_scrappa_error(response: Response) -> Result<String> {
    let status = response.status();
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) if error.is_timeout() => return Err(scrappa_request_error(error)),
        Err(_) => return Ok(format_scrappa_error(status, "")),
    };
    Ok(format_scrappa_error(status, &body))
}

pub(crate) fn format_scrappa_error(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    if body.is_empty() {
        return fallback;
    }
    if let Ok(data) = serde_json::from_str::<Value>(&body) {
        let mut message = data
            .get("message")
            .filter(|message| !message.is_null())
            .map(js_string)
            .unwrap_or(fallback);
        if let Some(errors) = data.get("errors").and_then(Value::as_object) {
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
        return message;
    }
    body.split(is_js_whitespace)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}
