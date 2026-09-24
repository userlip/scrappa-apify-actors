use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client};
use serde_json::Value;
use std::time::Duration;
use url::Url;

use crate::job::{js_string, ScrappaApiError};

pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) struct ScrappaClient {
    pub(crate) http: Client,
    pub(crate) base_url: String,
    pub(crate) api_key: String,
}

impl ScrappaClient {
    pub(crate) fn new(http: Client, base_url: String, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    pub(crate) async fn get_job(&self, params: &[(String, String)]) -> Result<Value> {
        let url = job_endpoint_url(&self.base_url, params)?;
        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(scrappa_transport_error)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .context("Failed to read Scrappa API error response")?;
            return Err(ScrappaApiError {
                status,
                message: scrappa_error_message(status, &body),
            }
            .into());
        }

        response
            .json::<Value>()
            .await
            .map_err(scrappa_transport_error)
    }
}

pub(crate) fn scrappa_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            REQUEST_TIMEOUT.as_millis()
        )
    } else {
        error.into()
    }
}

pub(crate) fn endpoint_url(base: &str, path: &[&str]) -> Result<Url> {
    let mut url = Url::parse(&format!("{}/", base.trim_end_matches('/')))
        .with_context(|| format!("Invalid API base URL: {base}"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow!("API base URL cannot contain query or fragment: {base}"))?;
        segments.pop_if_empty();
        for segment in path {
            segments.push(segment);
        }
    }
    Ok(url)
}

pub(crate) fn job_endpoint_url(base: &str, params: &[(String, String)]) -> Result<Url> {
    let mut url = endpoint_url(base, &["linkedin", "job"])?;
    url.query_pairs_mut().extend_pairs(params.iter());
    Ok(url)
}

pub(crate) fn scrappa_error_message(status: u16, body: &str) -> String {
    let Ok(error_data) = serde_json::from_str::<Value>(body) else {
        return if body.is_empty() {
            format!("HTTP {status}")
        } else {
            body.to_owned()
        };
    };

    let mut message = error_data
        .get("message")
        .filter(|value| !value.is_null())
        .map(js_string)
        .unwrap_or_else(|| format!("HTTP {status}"));

    if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
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
        message.push_str(" - ");
        message.push_str(&details);
    }

    message
}
