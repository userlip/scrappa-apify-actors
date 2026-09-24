use anyhow::{bail, Context, Result};
use reqwest::{header, Client, Response};
use serde_json::{json, Value};
use url::Url;

use crate::config::{endpoint_url, Config};

const APIFY_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("{operation} response could not be read"))?;
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "{operation} failed with {} {reason}{detail}",
            status.as_u16()
        );
    }
    serde_json::from_str(&body).with_context(|| format!("{operation} returned invalid JSON"))
}

async fn ensure_success(response: Response, operation: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    bail!(
        "{operation} failed with {} {reason}{detail}",
        status.as_u16()
    );
}

pub(crate) struct ApifyClient<'a> {
    pub(crate) http: &'a Client,
    pub(crate) config: &'a Config,
}

impl ApifyClient<'_> {
    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
    }

    pub(crate) async fn get_input(&self) -> Result<Value> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .http
            .get(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        response_json(response, "Apify INPUT request").await
    }

    pub(crate) async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .get(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        response_json(response, "Apify run pricing request").await
    }

    pub(crate) async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }

    pub(crate) async fn charge_event(
        &self,
        event_name: &str,
        count: usize,
        idempotency_key: &str,
    ) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        let response = self
            .http
            .post(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .header("idempotency-key", idempotency_key)
            .json(&json!({"eventName": event_name, "count": count}))
            .send()
            .await
            .context("Apify finance search charge request failed")?;
        ensure_success(response, "Apify finance search charge request").await
    }

    pub(crate) async fn update_status(&self, status_message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .put(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(&json!({
                "runId": self.config.actor_run_id,
                "statusMessage": status_message,
                "isStatusMessageTerminal": true,
            }))
            .send()
            .await
            .context("Apify run status update failed")?;
        ensure_success(response, "Apify run status update").await
    }
}
