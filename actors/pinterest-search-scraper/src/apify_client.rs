use std::{
    env,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Value};
use url::Url;

use crate::{api_url::endpoint_url, scrappa_client::SCRAPPA_API_DEFAULT};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
pub(crate) const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
pub(crate) const APIFY_RETRY_MIN_DELAY: Duration = Duration::from_millis(500);
pub(crate) const APIFY_MAX_RETRIES: usize = 8;
pub(crate) const MAX_DATASET_REQUEST_BYTES: usize = 4 * 1024 * 1024;
static CHARGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) struct Config {
    pub(crate) apify_api_base: Url,
    pub(crate) apify_token: String,
    pub(crate) actor_run_id: String,
    pub(crate) key_value_store_id: String,
    pub(crate) dataset_id: String,
    pub(crate) input_key: String,
    pub(crate) scrappa_api_base: Url,
    pub(crate) scrappa_api_key: Option<String>,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base: apify_api_base_from_env()?,
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            scrappa_api_key: env::var("SCRAPPA_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
        })
    }
}

fn apify_api_base_from_env() -> Result<Url> {
    let raw_url = env::var("APIFY_API_BASE_URL")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("APIFY_API_PUBLIC_BASE_URL")
                .ok()
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| APIFY_API_DEFAULT.to_owned());
    Url::parse(&raw_url).context("APIFY_API_BASE_URL must be a valid absolute URL")
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env_or_default(name, default);
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

pub(crate) struct ApifyClient {
    http: Client,
    base_url: Url,
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

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.base_url, segments)
    }

    async fn send_with_retry<F>(&self, operation: &str, mut build_request: F) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        let mut retry_count = 0;
        loop {
            match build_request().timeout(APIFY_REQUEST_TIMEOUT).send().await {
                Ok(response) => {
                    if retry_count < APIFY_MAX_RETRIES && retryable_apify_status(response.status())
                    {
                        let delay = apify_retry_delay(retry_count);
                        drop(response);
                        tokio::time::sleep(delay).await;
                        retry_count += 1;
                        continue;
                    }
                    return Ok(response);
                }
                Err(error)
                    if retry_count < APIFY_MAX_RETRIES
                        && (error.is_timeout() || error.is_connect() || error.is_request()) =>
                {
                    tokio::time::sleep(apify_retry_delay(retry_count)).await;
                    retry_count += 1;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify {operation} request failed"));
                }
            }
        }
    }

    pub(crate) async fn get_actor_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let response = self
            .send_with_retry("run pricing", || {
                self.http
                    .get(url.clone())
                    .bearer_auth(&self.token)
                    .header(header::ACCEPT, "application/json")
            })
            .await?;
        response_json(response, "run pricing request").await
    }

    pub(crate) async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .send_with_retry("input retrieval", || {
                self.http
                    .get(url.clone())
                    .bearer_auth(&self.token)
                    .header(header::ACCEPT, "application/json")
            })
            .await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let response = require_apify_success(response, "input retrieval").await?;
        let body = response
            .bytes()
            .await
            .context("Failed to read actor input from Apify API")?;
        serde_json::from_slice(&body)
            .context("Apify input record was not valid JSON")
            .map(Some)
    }

    pub(crate) async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        for chunk in dataset_item_chunks(items)? {
            let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
            let response = self
                .send_with_retry("dataset write", || {
                    self.http
                        .post(url.clone())
                        .bearer_auth(&self.token)
                        .header(header::ACCEPT, "application/json")
                        .json(&chunk)
                })
                .await?;
            require_apify_success(response, "dataset item publication").await?;
        }
        Ok(())
    }

    pub(crate) async fn charge_event(&self, event_name: &str, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }

        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id, "charge"])?;
        let idempotency_key = self.new_idempotency_key();
        let response = self
            .send_with_retry("event charge", || {
                self.http
                    .post(url.clone())
                    .bearer_auth(&self.token)
                    .header(header::ACCEPT, "application/json")
                    .header("idempotency-key", &idempotency_key)
                    .json(&json!({ "eventName": event_name, "count": count }))
            })
            .await?;
        require_apify_success(response, "event charge").await?;
        Ok(())
    }

    pub(crate) async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .send_with_retry("OUTPUT write", || {
                self.http
                    .put(url.clone())
                    .bearer_auth(&self.token)
                    .header(header::ACCEPT, "application/json")
                    .json(output)
            })
            .await?;
        require_apify_success(response, "OUTPUT record publication").await?;
        Ok(())
    }

    pub(crate) async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let response = self
            .send_with_retry("status message update", || {
                self.http
                    .put(url.clone())
                    .bearer_auth(&self.token)
                    .header(header::ACCEPT, "application/json")
                    .json(&json!({
                        "runId": self.actor_run_id,
                        "statusMessage": message,
                        "isStatusMessageTerminal": true
                    }))
            })
            .await?;
        require_apify_success(response, "status message update").await?;
        Ok(())
    }

    fn new_idempotency_key(&self) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let sequence = CHARGE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        format!(
            "pinterest-search-{}-{timestamp}-{sequence}",
            self.actor_run_id
        )
    }
}

pub(crate) fn retryable_apify_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

pub(crate) fn apify_retry_delay(retry_count: usize) -> Duration {
    let exponent = u32::try_from(retry_count).unwrap_or(u32::MAX);
    APIFY_RETRY_MIN_DELAY.saturating_mul(2_u32.saturating_pow(exponent))
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let response = require_apify_success(response, operation).await?;
    response
        .json::<Value>()
        .await
        .with_context(|| format!("Apify {operation} response was not valid JSON"))
}

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

pub(crate) fn dataset_item_chunks(items: &[Value]) -> Result<Vec<Vec<Value>>> {
    let mut chunks = Vec::new();
    let mut current = Vec::new();
    let mut current_size = 2;

    for item in items {
        let item_size = serde_json::to_vec(item)
            .context("Failed to encode dataset item")?
            .len();
        if item_size + 2 > MAX_DATASET_REQUEST_BYTES {
            bail!("Pinterest dataset item exceeds the Apify dataset request size limit");
        }

        let added_size = item_size + usize::from(!current.is_empty());
        if !current.is_empty() && current_size + added_size > MAX_DATASET_REQUEST_BYTES {
            chunks.push(std::mem::take(&mut current));
            current_size = 2;
        }
        current_size += item_size + usize::from(!current.is_empty());
        current.push(item.clone());
    }

    if !current.is_empty() {
        chunks.push(current);
    }
    Ok(chunks)
}
