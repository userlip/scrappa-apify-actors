use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Value};
use url::Url;

use crate::{
    http_utils::{endpoint_url, require_apify_success, response_json},
    ActorConfig,
};

const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const APIFY_MAX_RETRIES: usize = 2;

pub(crate) struct ApifyClient {
    http: Client,
    base_url: String,
    token: String,
    pub(crate) actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    pub(crate) fn new(config: &ActorConfig) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Failed to create Apify API HTTP client")?,
            base_url: config.apify_api_base.clone(),
            token: config.apify_token.clone(),
            actor_run_id: config.actor_run_id.clone(),
            key_value_store_id: config.key_value_store_id.clone(),
            dataset_id: config.dataset_id.clone(),
            input_key: config.input_key.clone(),
        })
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.base_url, segments)
    }

    pub(crate) async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let response = self
            .send_with_retries(
                || {
                    self.http
                        .get(url.clone())
                        .bearer_auth(&self.token)
                        .header(header::ACCEPT, "application/json")
                },
                "run pricing request",
                false,
            )
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
            .send_with_retries(
                || {
                    self.http
                        .get(url.clone())
                        .bearer_auth(&self.token)
                        .header(header::ACCEPT, "application/json")
                },
                "input retrieval",
                false,
            )
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        response_json(response, "input retrieval").await.map(Some)
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
        count: u64,
        idempotency_key: &str,
    ) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id, "charge"])?;
        let response = self
            .send_with_retries(
                || {
                    self.http
                        .post(url.clone())
                        .bearer_auth(&self.token)
                        .header(header::ACCEPT, "application/json")
                        .header("Idempotency-Key", idempotency_key)
                        .json(&json!({ "eventName": event_name, "count": count }))
                },
                "charge event request",
                true,
            )
            .await?;
        require_apify_success(response, "charge event request").await?;
        Ok(())
    }

    pub(crate) async fn set_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .send_with_retries(
                || {
                    self.http
                        .put(url.clone())
                        .bearer_auth(&self.token)
                        .header(header::ACCEPT, "application/json")
                        .json(output)
                },
                "OUTPUT record publication",
                false,
            )
            .await?;
        require_apify_success(response, "OUTPUT record publication").await?;
        Ok(())
    }

    pub(crate) async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let body = json!({
            "runId": self.actor_run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true,
        });
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to set Apify run status message")?;
        require_apify_success(response, "status message update").await?;
        Ok(())
    }

    async fn send_with_retries<F>(
        &self,
        make_request: F,
        operation: &str,
        retry_network_errors: bool,
    ) -> Result<Response>
    where
        F: Fn() -> RequestBuilder,
    {
        let mut retry_count = 0;
        loop {
            let response = match make_request().send().await {
                Ok(response) => response,
                Err(error)
                    if retry_network_errors
                        && retry_count < APIFY_MAX_RETRIES
                        && (error.is_timeout() || error.is_connect()) =>
                {
                    tokio::time::sleep(Duration::from_secs((retry_count + 1) as u64)).await;
                    retry_count += 1;
                    continue;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify {operation} failed"));
                }
            };

            if retry_count < APIFY_MAX_RETRIES
                && (response.status() == StatusCode::TOO_MANY_REQUESTS
                    || response.status().is_server_error())
            {
                drop(response);
                tokio::time::sleep(Duration::from_secs((retry_count + 1) as u64)).await;
                retry_count += 1;
                continue;
            }

            return Ok(response);
        }
    }
}
