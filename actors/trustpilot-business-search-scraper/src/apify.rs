use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::Value;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static CHARGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub fn new(base_url: &str, token: String) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token,
        })
    }

    pub async fn get_input(&self, store_id: &str, input_key: &str) -> Result<Option<Value>> {
        let response = self
            .request(Method::GET, self.record_url(store_id, input_key)?)
            .send()
            .await
            .context("Failed to fetch Actor input from the default key-value store")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "fetch Actor input").await?;
        Ok(Some(
            response
                .json()
                .await
                .context("Actor input record is not valid JSON")?,
        ))
    }

    pub async fn get_run(&self, actor_run_id: &str) -> Result<Value> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&["actor-runs", actor_run_id])?,
            )
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let response = successful_response(response, "fetch Actor run pricing").await?;
        response
            .json()
            .await
            .context("Apify run pricing response is not valid JSON")
    }

    pub async fn push_dataset_items(&self, dataset_id: &str, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["datasets", dataset_id, "items"])?,
            )
            .json(items)
            .send()
            .await
            .context("Failed to store business results in the default dataset")?;
        successful_response(response, "store dataset items").await?;
        Ok(())
    }

    pub async fn charge_event(
        &self,
        actor_run_id: &str,
        event_name: &str,
        count: usize,
    ) -> Result<usize> {
        if count == 0 {
            return Ok(0);
        }
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["actor-runs", actor_run_id, "charge"])?,
            )
            .header(
                "idempotency-key",
                charge_idempotency_key(actor_run_id, event_name),
            )
            .json(&serde_json::json!({"eventName": event_name, "count": count}))
            .send()
            .await
            .context("Apify event charge request failed")?;
        successful_response(response, "charge business result events").await?;
        // The REST charge endpoint returns `{}` on success; it does not return the SDK's `chargedCount`.
        Ok(count)
    }

    pub async fn set_terminal_status_message(
        &self,
        actor_run_id: &str,
        message: &str,
    ) -> Result<()> {
        let response = self
            .request(
                Method::PUT,
                self.resource_url(&["actor-runs", actor_run_id])?,
            )
            .timeout(Duration::from_secs(1))
            .json(&serde_json::json!({
                "runId": actor_run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true
            }))
            .send()
            .await
            .context("Apify run status update failed")?;
        successful_response(response, "set terminal status message").await?;
        Ok(())
    }

    pub async fn set_output(&self, store_id: &str, output: &Value) -> Result<()> {
        let response = self
            .request(Method::PUT, self.record_url(store_id, "OUTPUT")?)
            .json(output)
            .send()
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    fn record_url(&self, store_id: &str, record_key: &str) -> Result<Url> {
        self.resource_url(&["key-value-stores", store_id, "records", record_key])
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot be a base URL"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(resource.iter().copied()));
        Ok(url)
    }
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let status_code = status.as_u16();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.is_empty() {
        format!("HTTP {status_code}")
    } else {
        body
    };
    bail!("Apify API error ({status_code}) while trying to {operation}: {detail}");
}

fn charge_idempotency_key(actor_run_id: &str, event_name: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = CHARGE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{actor_run_id}-{event_name}-{timestamp}-{sequence}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_idempotency_keys_are_unique_per_request() {
        let first = charge_idempotency_key("run-1", "business-result");
        let second = charge_idempotency_key("run-1", "business-result");
        assert!(first.starts_with("run-1-business-result-"));
        assert_ne!(first, second);
    }
}
