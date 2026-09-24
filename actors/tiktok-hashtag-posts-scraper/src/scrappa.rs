use crate::{
    apify::{endpoint_url, ActorConfig},
    input::js_string,
    tiktok_response::{
        extract_challenges, get_challenge_id, get_challenge_name, select_challenge_for_hashtag,
    },
};
use anyhow::{anyhow, bail, Result};
use reqwest::Response;
use serde_json::Value;
use std::time::Duration;

pub(crate) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() || error.to_string().contains("aborted") {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            SCRAPPA_REQUEST_TIMEOUT.as_millis()
        )
    } else {
        anyhow!(error.to_string())
    }
}

async fn read_scrappa_error(response: Response) -> Result<String> {
    let status = response.status();
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) if error.is_timeout() => return Err(scrappa_request_error(error)),
        Err(_) => return Ok(fallback),
    };
    if body.is_empty() {
        return Ok(fallback);
    }
    if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
        let mut message = error_data
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or(fallback);
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    let messages = messages.as_array()?;
                    Some(format!(
                        "{field}: {}",
                        messages
                            .iter()
                            .map(js_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
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
    Ok(body
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect())
}

pub(crate) async fn fetch_scrappa_response(
    client: &reqwest::Client,
    config: &ActorConfig,
    endpoint: &[&str],
    query: &[(&str, String)],
) -> Result<Value> {
    let mut url = endpoint_url(&config.scrappa_api_base_url, endpoint)?;
    if !query.is_empty() {
        let mut query_pairs = url.query_pairs_mut();
        for (key, value) in query {
            query_pairs.append_pair(key, value);
        }
    }
    println!("[Scrappa] GET {url}");
    let response = client
        .get(url)
        .header("X-API-Key", &config.scrappa_api_key)
        .header("Accept", "application/json")
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

pub(crate) fn validate_scrappa_code(response: &Value, operation: &str) -> Result<()> {
    let Some(code) = response.get("code") else {
        return Ok(());
    };
    if code.as_f64().is_some_and(|code| code == 0.0) {
        return Ok(());
    }
    let message = response
        .get("msg")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or_else(|| "Unknown error".to_owned());
    bail!(
        "Scrappa {operation} API returned code {}: {message}",
        js_string(code)
    );
}

pub(crate) async fn resolve_challenge(
    client: &reqwest::Client,
    config: &ActorConfig,
    challenge_name: &str,
) -> Result<(String, Option<String>)> {
    let response = fetch_scrappa_response(
        client,
        config,
        &["tiktok", "challenges", "search"],
        &[
            ("keywords", challenge_name.to_owned()),
            ("count", "10".to_owned()),
        ],
    )
    .await?;
    validate_scrappa_code(&response, "TikTok Challenge Search")?;

    let data = response.get("data");
    let challenges = extract_challenges(data);
    let selection = select_challenge_for_hashtag(&challenges, challenge_name);
    let challenge_id = selection.and_then(get_challenge_id).ok_or_else(|| {
        anyhow!("Could not resolve TikTok hashtag \"{challenge_name}\" to a challenge_id")
    })?;
    let resolved_name = selection
        .map(get_challenge_name)
        .filter(|name| !name.is_empty())
        .or_else(|| Some(challenge_name.to_owned()));
    Ok((challenge_id, resolved_name))
}
