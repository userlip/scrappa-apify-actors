use crate::{
    config::{Config, APIFY_MAX_RETRIES},
    endpoint::endpoint_url,
};
use anyhow::{bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Value};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

pub(crate) struct ApifyClient {
    http: Client,
    base_url: String,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    pub(crate) fn new(http: Client, config: &Config) -> Self {
        Self {
            http,
            base_url: config.apify_api_base.clone(),
            token: config.apify_token.clone(),
            actor_run_id: config.actor_run_id.clone(),
            key_value_store_id: config.key_value_store_id.clone(),
            dataset_id: config.dataset_id.clone(),
            input_key: config.input_key.clone(),
        }
    }

    fn endpoint(&self, path: &[&str]) -> Result<url::Url> {
        endpoint_url(&self.base_url, path)
    }

    pub(crate) async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
                .context("Failed to retrieve actor input from Apify API")?;
            if response.status() == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                sleep(delay).await;
                continue;
            }
            let response = require_apify_success(response, "input retrieval").await?;
            return response
                .json::<Value>()
                .await
                .context("Apify input record was not valid JSON")
                .map(Some);
        }
        unreachable!("bounded Apify retry loop returns a response")
    }

    pub(crate) async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
                .context("Apify run pricing request failed")?;
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                sleep(delay).await;
                continue;
            }
            return require_apify_success(response, "run pricing request")
                .await?
                .json::<Value>()
                .await
                .context("Apify run pricing response was not valid JSON");
        }
        unreachable!("bounded Apify retry loop returns a response")
    }

    pub(crate) async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Failed to publish dataset items to Apify API")?;
        require_apify_success(response, "dataset item publication")
            .await?
            .bytes()
            .await
            .context("Could not finish Apify dataset item publication")?;
        Ok(())
    }

    pub(crate) async fn charge_event(&self, event_name: &str, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id, "charge"])?;
        let idempotency_key = format!(
            "{}-{event_name}-{}-{}",
            self.actor_run_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            std::process::id()
        );
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let response = self
                .http
                .post(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .header("idempotency-key", &idempotency_key)
                .json(&json!({ "eventName": event_name, "count": count }))
                .send()
                .await
                .context("Apify event charge request failed")?;
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                sleep(delay).await;
                continue;
            }
            require_apify_success(response, "event charge")
                .await?
                .bytes()
                .await?;
            return Ok(());
        }
        unreachable!("bounded Apify retry loop returns a response")
    }

    pub(crate) async fn set_terminal_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let body = json!({
            "runId": self.actor_run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true,
        });
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let response = self
                .http
                .put(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .json(&body)
                .send()
                .await
                .context("Apify status message update failed")?;
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                sleep(delay).await;
                continue;
            }
            require_apify_success(response, "status message update")
                .await?
                .bytes()
                .await
                .context("Could not finish Apify status message update")?;
            return Ok(());
        }
        unreachable!("bounded Apify retry loop returns a response")
    }

    pub(crate) async fn put_record(&self, key: &str, value: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            key,
        ])?;
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let response = self
                .http
                .put(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .json(value)
                .send()
                .await
                .with_context(|| format!("Failed to write {key} record to Apify API"))?;
            if let Some(delay) = apify_retry_delay(response.status(), retry_count) {
                drop(response);
                sleep(delay).await;
                continue;
            }
            require_apify_success(response, &format!("{key} record publication"))
                .await?
                .bytes()
                .await
                .with_context(|| format!("Could not finish {key} record publication"))?;
            return Ok(());
        }
        unreachable!("bounded Apify retry loop returns a response")
    }
}

fn apify_retry_delay(status: StatusCode, retry_count: usize) -> Option<Duration> {
    if retry_count >= APIFY_MAX_RETRIES
        || (status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error())
    {
        return None;
    }
    Some(Duration::from_secs((retry_count + 1) as u64))
}

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}
