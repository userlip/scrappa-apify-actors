use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client, Response};
use serde_json::{json, Value};
use std::{error::Error as StdError, fmt, time::Duration};
use url::Url;

use crate::endpoint_url;

const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub(super) struct ScrappaError {
    pub(super) status_code: Option<u16>,
    message: String,
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for ScrappaError {}

pub(super) struct ScrappaClient<'a> {
    http: &'a reqwest::Client,
    base_url: &'a Url,
    api_key: &'a str,
}

impl<'a> ScrappaClient<'a> {
    pub(super) fn new(http: &'a Client, base_url: &'a Url, api_key: &'a str) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    pub(super) async fn get_photos(
        &self,
        business_id: &str,
        input: Option<&Value>,
    ) -> Result<Value> {
        let url = build_photos_url(self.base_url, business_id, input)?;
        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key)
            .header(header::ACCEPT, "application/json")
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|error| {
                let message = if error.is_timeout() {
                    format!(
                        "Scrappa API request timed out after {}ms",
                        SCRAPPA_REQUEST_TIMEOUT.as_millis()
                    )
                } else {
                    format!("Scrappa API request failed: {error}")
                };
                ScrappaError {
                    status_code: None,
                    message,
                }
            })?;

        if !response.status().is_success() {
            let status_code = response.status().as_u16();
            let message = scrappa_error_message(response).await;
            return Err(ScrappaError {
                status_code: Some(status_code),
                message: format!("Scrappa API error ({status_code}): {message}"),
            }
            .into());
        }

        response
            .json()
            .await
            .context("Scrappa API response was not valid JSON")
    }
}

pub(super) fn build_photos_url(
    base_url: &Url,
    business_id: &str,
    input: Option<&Value>,
) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["maps", "photos"])?;
    url.query_pairs_mut()
        .append_pair("business_id", business_id);

    if input
        .and_then(|input| input.get("use_cache"))
        .and_then(Value::as_bool)
        != Some(false)
    {
        url.query_pairs_mut().append_pair("use_cache", "1");
    }

    if let Some(maximum_cache_age) = input
        .and_then(|input| input.get("maximum_cache_age"))
        .and_then(query_value)
    {
        url.query_pairs_mut()
            .append_pair("maximum_cache_age", &maximum_cache_age);
    }
    Ok(url)
}

fn query_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(false) => None,
        Value::Bool(true) => Some("1".to_owned()),
        Value::String(value) if value.is_empty() => None,
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Array(values) => Some(
            values
                .iter()
                .map(|value| match value {
                    Value::Null => String::new(),
                    Value::String(value) => value.clone(),
                    Value::Object(_) => "[object Object]".to_owned(),
                    value => value.to_string(),
                })
                .collect::<Vec<_>>()
                .join(","),
        ),
        Value::Object(_) => Some("[object Object]".to_owned()),
    }
}

async fn scrappa_error_message(response: Response) -> String {
    let fallback = response
        .status()
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", response.status().as_u16()));
    let Ok(body) = response.text().await else {
        return fallback;
    };
    if body.is_empty() {
        return fallback;
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| fallback.clone());
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, values)| {
                    let messages = values
                        .as_array()
                        .map(|values| {
                            values
                                .iter()
                                .map(|value| {
                                    value
                                        .as_str()
                                        .map(str::to_owned)
                                        .unwrap_or_else(|| value.to_string())
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .or_else(|| values.as_str().map(str::to_owned))
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

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

pub(super) fn photo_results(response: &Value) -> Result<(Vec<Value>, Value)> {
    if let Some(photos) = response.as_array() {
        return Ok((photos.clone(), Value::Null));
    }
    let photos = response
        .get("items")
        .filter(|items| !items.is_null())
        .or_else(|| response.get("data"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let photos = photos
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow!("Scrappa API response photos must be an array"))?;
    let next_page = response.get("nextPage").cloned().unwrap_or(Value::Null);
    Ok((photos, next_page))
}
