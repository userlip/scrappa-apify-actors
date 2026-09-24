use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

use super::{
    apify::endpoint_url,
    config::ActorConfig,
    input::{js_number_string, js_string, js_truthy, TikTokFollowersParams},
};

pub(super) fn check_scrappa_code(response: &Value, api_name: &str) -> Result<()> {
    if let Some(code) = response.get("code") {
        if code.as_f64() != Some(0.0) {
            let message = response
                .get("msg")
                .filter(|message| !message.is_null())
                .map(js_string)
                .unwrap_or_else(|| "Unknown error".to_owned());
            bail!(
                "Scrappa TikTok {api_name} API returned code {}: {message}",
                js_string(code)
            );
        }
    }
    Ok(())
}

pub(super) fn request_timeout_message(timeout: Duration) -> String {
    format!(
        "Scrappa API request timed out after {}ms",
        timeout.as_millis()
    )
}

pub(super) fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn joined_error_messages(value: &Value) -> Option<String> {
    let messages = value.as_array()?;
    Some(
        messages
            .iter()
            .map(|message| {
                if message.is_null() {
                    String::new()
                } else {
                    js_string(message)
                }
            })
            .collect::<Vec<_>>()
            .join(", "),
    )
}

pub(super) fn format_scrappa_error_body(body: &str, fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        if !error_data.is_null() {
            let mut message = error_data
                .get("message")
                .filter(|message| !message.is_null())
                .map(js_string)
                .unwrap_or_else(|| fallback.to_owned());

            if let Some(errors) = error_data.get("errors").filter(|errors| js_truthy(errors)) {
                let details = match errors {
                    Value::Object(errors) => errors
                        .iter()
                        .filter_map(|(field, messages)| {
                            joined_error_messages(messages)
                                .map(|messages| format!("{field}: {messages}"))
                        })
                        .collect::<Vec<_>>(),
                    Value::Array(errors) => errors
                        .iter()
                        .enumerate()
                        .filter_map(|(index, messages)| {
                            joined_error_messages(messages)
                                .map(|messages| format!("{index}: {messages}"))
                        })
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                };
                if !details.is_empty() {
                    message.push_str(" - ");
                    message.push_str(&details.join("; "));
                }
            }
            return message;
        }
    }

    collapse_whitespace(body).chars().take(500).collect()
}

pub(super) async fn get_scrappa_json(
    client: &Client,
    config: &ActorConfig,
    endpoint: &[&str],
    params: &[(&str, String)],
) -> Result<Value> {
    let mut url = endpoint_url(&config.scrappa_api_base_url, endpoint)?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            if !value.is_empty() {
                query.append_pair(key, value);
            }
        }
    }

    let response = client
        .get(url)
        .timeout(config.scrappa_request_timeout)
        .header("X-API-Key", &config.scrappa_api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "{}",
                    request_timeout_message(config.scrappa_request_timeout)
                )
            } else {
                anyhow!("Scrappa API request failed: {error}")
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        let fallback = status
            .canonical_reason()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
        let body = response.text().await.map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "{}",
                    request_timeout_message(config.scrappa_request_timeout)
                )
            } else {
                anyhow!(fallback.clone())
            }
        })?;
        bail!(
            "Scrappa API error ({}): {}",
            status.as_u16(),
            format_scrappa_error_body(&body, &fallback)
        );
    }

    let body = response.text().await.map_err(|error| {
        if error.is_timeout() {
            anyhow!(
                "{}",
                request_timeout_message(config.scrappa_request_timeout)
            )
        } else {
            anyhow!("Scrappa API response could not be read: {error}")
        }
    })?;
    serde_json::from_str(&body).context("Scrappa API response was not valid JSON")
}

pub(super) async fn resolve_tiktok_user_id(
    client: &Client,
    config: &ActorConfig,
    params: &TikTokFollowersParams,
) -> Result<String> {
    if let Some(user_id) = params.lookup.user_id() {
        return Ok(user_id.to_owned());
    }

    let unique_id = params
        .lookup
        .unique_id()
        .ok_or_else(|| anyhow!("TikTok unique_id or user_id is required"))?;
    println!("Resolving TikTok user_id for {unique_id}");

    let response = get_scrappa_json(
        client,
        config,
        &["tiktok", "user", "profile"],
        &[("unique_id", unique_id.to_owned())],
    )
    .await?;
    check_scrappa_code(&response, "Profile")?;

    let data = response.get("data");
    let profile = match data {
        Some(Value::Array(profiles)) => profiles.first(),
        Some(value) => Some(value),
        None => None,
    };
    let user_id = profile
        .and_then(|profile| {
            profile
                .get("user_id")
                .filter(|user_id| !user_id.is_null())
                .or_else(|| {
                    profile
                        .get("user")
                        .and_then(|user| user.get("id"))
                        .filter(|user_id| !user_id.is_null())
                })
        })
        .and_then(|user_id| match user_id {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(js_number_string(value)),
            _ => None,
        })
        .map(|user_id| user_id.trim().to_owned())
        .filter(|user_id| !user_id.is_empty())
        .ok_or_else(|| anyhow!("Could not resolve TikTok user_id for {unique_id}"))?;

    Ok(user_id)
}
