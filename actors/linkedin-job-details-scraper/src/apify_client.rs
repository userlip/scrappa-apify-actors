use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Value};
use url::Url;

use crate::scrappa::endpoint_url;

pub(crate) const APIFY_API_DEFAULT: &str = "https://api.apify.com";
pub(crate) const STATUS_MESSAGE_TIMEOUT: Duration = Duration::from_secs(1);
pub(crate) const APIFY_MAX_RETRIES: usize = 2;

pub(crate) struct ApifyConfig {
    pub(crate) api_base: String,
    pub(crate) token: String,
    pub(crate) actor_run_id: String,
    pub(crate) key_value_store_id: String,
    pub(crate) dataset_id: String,
    pub(crate) input_key: String,
}

impl ApifyConfig {
    pub(crate) fn from_env() -> Result<Self> {
        Ok(Self {
            api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
        })
    }
}

pub(crate) fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

pub(crate) fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

pub(crate) struct ApifyClient {
    pub(crate) http: Client,
    pub(crate) base_url: String,
    pub(crate) token: String,
    pub(crate) actor_run_id: String,
    pub(crate) key_value_store_id: String,
    pub(crate) dataset_id: String,
    pub(crate) input_key: String,
}

impl ApifyClient {
    pub(crate) fn new(http: Client, config: &ApifyConfig) -> Self {
        Self {
            http,
            base_url: config.api_base.clone(),
            token: config.token.clone(),
            actor_run_id: config.actor_run_id.clone(),
            key_value_store_id: config.key_value_store_id.clone(),
            dataset_id: config.dataset_id.clone(),
            input_key: config.input_key.clone(),
        }
    }

    pub(crate) fn endpoint(&self, path: &[&str]) -> Result<Url> {
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
        let mut retry_count = 0;
        loop {
            let response = match self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let Some(delay) = apify_transport_retry_delay(retry_count) else {
                        return Err(error).context("Failed to retrieve actor input from Apify API");
                    };
                    tokio::time::sleep(delay).await;
                    retry_count += 1;
                    continue;
                }
            };

            if response.status() == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if let Some(delay) = apify_retry_delay("GET", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            let response = require_apify_success(response, "input retrieval").await?;
            return response
                .json::<Value>()
                .await
                .context("Apify input record was not valid JSON")
                .map(Some);
        }
    }

    pub(crate) async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let mut retry_count = 0;
        loop {
            let response = match self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let Some(delay) = apify_transport_retry_delay(retry_count) else {
                        return Err(error).context("Apify run pricing request failed");
                    };
                    tokio::time::sleep(delay).await;
                    retry_count += 1;
                    continue;
                }
            };

            if let Some(delay) = apify_retry_delay("GET", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            return require_apify_success(response, "run pricing request")
                .await?
                .json::<Value>()
                .await
                .context("Apify run pricing response was not valid JSON");
        }
    }

    pub(crate) async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(item)
            .send()
            .await
            .context("Failed to publish dataset item to Apify API")?;
        require_apify_success(response, "dataset item publication").await?;
        Ok(())
    }

    pub(crate) async fn charge_event(
        &self,
        event_name: &str,
        count: usize,
        idempotency_key: &str,
    ) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id, "charge"])?;
        let body = json!({ "eventName": event_name, "count": count });
        let mut retry_count = 0;
        loop {
            let response = match self
                .http
                .post(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .header("idempotency-key", idempotency_key)
                .json(&body)
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let Some(delay) = apify_transport_retry_delay(retry_count) else {
                        return Err(error)
                            .with_context(|| format!("Apify {event_name} charge request failed"));
                    };
                    tokio::time::sleep(delay).await;
                    retry_count += 1;
                    continue;
                }
            };

            if let Some(delay) = apify_retry_delay("POST", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            require_apify_success(response, &format!("{event_name} charge")).await?;
            return Ok(());
        }
    }

    pub(crate) async fn put_record(&self, key: &str, value: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            key,
        ])?;
        let mut retry_count = 0;
        loop {
            let response = match self
                .http
                .put(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .json(value)
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let Some(delay) = apify_transport_retry_delay(retry_count) else {
                        return Err(error)
                            .with_context(|| format!("Failed to write {key} record to Apify API"));
                    };
                    tokio::time::sleep(delay).await;
                    retry_count += 1;
                    continue;
                }
            };

            if let Some(delay) = apify_retry_delay("PUT", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            require_apify_success(response, &format!("{key} record publication")).await?;
            return Ok(());
        }
    }

    pub(crate) async fn set_terminal_status_message(&self, message: &str) {
        let url = match self.endpoint(&["v2", "actor-runs", &self.actor_run_id]) {
            Ok(url) => url,
            Err(error) => {
                eprintln!("Failed to set terminal Actor status message: {error}");
                return;
            }
        };
        let response = self
            .http
            .put(url)
            .timeout(STATUS_MESSAGE_TIMEOUT)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(&json!({
                "runId": self.actor_run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true,
            }))
            .send()
            .await;

        match response {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                eprintln!(
                    "Failed to set terminal Actor status message ({}): {body}",
                    status.as_u16()
                );
            }
            Err(error) => eprintln!("Failed to set terminal Actor status message: {error}"),
        }
    }
}

pub(crate) fn apify_retry_delay(
    method: &str,
    status: StatusCode,
    retry_count: usize,
) -> Option<Duration> {
    let retryable_method = matches!(method, "GET" | "PUT" | "POST");
    let transient_status = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
    if !retryable_method || !transient_status || retry_count >= APIFY_MAX_RETRIES {
        return None;
    }

    Some(Duration::from_secs((retry_count + 1) as u64))
}

pub(crate) fn apify_transport_retry_delay(retry_count: usize) -> Option<Duration> {
    (retry_count < APIFY_MAX_RETRIES).then(|| Duration::from_secs((retry_count + 1) as u64))
}

pub(crate) async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}
