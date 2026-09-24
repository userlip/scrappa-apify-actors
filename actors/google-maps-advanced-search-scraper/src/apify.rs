use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Value};
use url::Url;

use crate::config::{endpoint_url, Config};

pub(crate) const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_APIFY_REQUEST_ATTEMPTS: usize = 3;
const DATASET_BATCH_MAX_BYTES: usize = 4_500_000;

fn transient_apify_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_EARLY
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn apify_retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(200 * 2_u64.pow((attempt.saturating_sub(1)) as u32))
}

async fn send_apify_with_retries<F>(make_request: F, operation: &str) -> Result<Response>
where
    F: Fn() -> RequestBuilder,
{
    for attempt in 1..=MAX_APIFY_REQUEST_ATTEMPTS {
        match make_request().timeout(APIFY_REQUEST_TIMEOUT).send().await {
            Ok(response)
                if transient_apify_status(response.status())
                    && attempt < MAX_APIFY_REQUEST_ATTEMPTS =>
            {
                tokio::time::sleep(apify_retry_delay(attempt)).await;
            }
            Ok(response) => return Ok(response),
            Err(_) if attempt < MAX_APIFY_REQUEST_ATTEMPTS => {
                tokio::time::sleep(apify_retry_delay(attempt)).await;
            }
            Err(error) => return Err(anyhow!("{operation} failed: {error}")),
        }
    }
    unreachable!("retry loop always returns a response or error")
}

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

fn next_idempotency_key(actor_run_id: &str, event_name: &str) -> String {
    static NEXT_CHARGE_ID: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = NEXT_CHARGE_ID.fetch_add(1, Ordering::Relaxed);
    format!("{actor_run_id}-{event_name}-{now}-{sequence}")
}

pub(crate) struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

impl<'a> ApifyClient<'a> {
    pub(crate) fn new(http: &'a Client, config: &'a Config) -> Self {
        Self { http, config }
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base_url, segments)
    }

    pub(crate) async fn get_input(&self) -> Result<Value> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = send_apify_with_retries(
            || {
                self.http
                    .get(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
            },
            "Apify INPUT request",
        )
        .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Value::Null);
        }
        response_json(response, "Apify INPUT request").await
    }

    pub(crate) async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = send_apify_with_retries(
            || {
                self.http
                    .get(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
            },
            "Apify run pricing request",
        )
        .await?;
        response_json(response, "Apify run pricing request").await
    }

    pub(crate) async fn charge(&self, event_name: &str, count: usize) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        let count = u64::try_from(count).context("Charge event count is too large")?;
        let idempotency_key = next_idempotency_key(&self.config.actor_run_id, event_name);
        let body = json!({ "eventName": event_name, "count": count });
        let response = send_apify_with_retries(
            || {
                self.http
                    .post(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
                    .header("idempotency-key", &idempotency_key)
                    .json(&body)
            },
            "Apify event charge",
        )
        .await?;
        ensure_success(response, "Apify event charge").await
    }

    pub(crate) async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        for batch in dataset_batches(items)? {
            let response = self
                .http
                .post(url.clone())
                .bearer_auth(&self.config.apify_token)
                .header(header::ACCEPT, "application/json")
                .json(&batch)
                .timeout(APIFY_REQUEST_TIMEOUT)
                .send()
                .await
                .map_err(|error| anyhow!("Apify dataset write failed: {error}"))?;
            ensure_success(response, "Apify dataset write").await?;
        }
        Ok(())
    }

    pub(crate) async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = send_apify_with_retries(
            || {
                self.http
                    .put(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
                    .json(output)
            },
            "Apify OUTPUT write",
        )
        .await?;
        ensure_success(response, "Apify OUTPUT write").await
    }
}

pub(crate) fn dataset_batches(items: &[Value]) -> Result<Vec<Vec<Value>>> {
    let mut batches = Vec::new();
    let mut batch = Vec::new();
    let mut batch_bytes = 2;

    for item in items {
        let item_bytes = serde_json::to_vec(item)
            .context("Could not serialize Apify dataset item")?
            .len();
        if item_bytes + 2 > DATASET_BATCH_MAX_BYTES {
            bail!("Apify dataset item exceeds the 4.5 MB write limit");
        }
        let separator_bytes = usize::from(!batch.is_empty());
        if !batch.is_empty() && batch_bytes + separator_bytes + item_bytes > DATASET_BATCH_MAX_BYTES
        {
            batches.push(std::mem::take(&mut batch));
            batch_bytes = 2;
        }
        if !batch.is_empty() {
            batch_bytes += 1;
        }
        batch_bytes += item_bytes;
        batch.push(item.clone());
    }

    if !batch.is_empty() {
        batches.push(batch);
    }
    Ok(batches)
}
