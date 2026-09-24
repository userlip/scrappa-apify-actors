use anyhow::{anyhow, bail, Context, Result};
use reqwest::Response;
use serde_json::Value;
use url::Url;

pub(crate) fn endpoint_url(base_url: &str, segments: &[&str]) -> Result<Url> {
    let mut url =
        Url::parse(base_url).with_context(|| format!("Invalid API base URL: {base_url}"))?;
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

pub(crate) async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let response = require_apify_success(response, operation).await?;
    response
        .json::<Value>()
        .await
        .with_context(|| format!("Apify {operation} response was not valid JSON"))
}

pub(crate) async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}
