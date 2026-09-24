use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, StatusCode};
use serde_json::{json, Value};
use std::env;
use url::Url;

use crate::pricing::BOOKING_RESULT_CHARGE_EVENT;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";

pub struct ActorConfig {
    pub(crate) apify_api_base_url: Url,
    pub(crate) scrappa_api_base_url: Url,
    pub(crate) default_key_value_store_id: String,
    pub(crate) default_dataset_id: String,
    pub(crate) input_key: String,
    pub(crate) apify_token: String,
    pub(crate) actor_run_id: String,
    pub(crate) scrappa_api_key: String,
}

impl ActorConfig {
    pub fn new(
        apify_api_base_url: Url,
        scrappa_api_base_url: Url,
        default_key_value_store_id: impl Into<String>,
        default_dataset_id: impl Into<String>,
        apify_token: impl Into<String>,
        actor_run_id: impl Into<String>,
        scrappa_api_key: impl Into<String>,
    ) -> Self {
        Self {
            apify_api_base_url,
            scrappa_api_base_url,
            default_key_value_store_id: default_key_value_store_id.into(),
            default_dataset_id: default_dataset_id.into(),
            input_key: "INPUT".to_owned(),
            apify_token: apify_token.into(),
            actor_run_id: actor_run_id.into(),
            scrappa_api_key: scrappa_api_key.into(),
        }
    }

    pub(crate) fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.trim().is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

pub(crate) fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments.iter().copied());
    Ok(url)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
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
    response
        .json()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

pub(crate) async fn get_input(client: &Client, config: &ActorConfig) -> Result<Option<Value>> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify INPUT request failed")?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response_json(response, "Apify INPUT request")
        .await
        .map(Some)
}

pub(crate) async fn get_run_info(client: &Client, config: &ActorConfig) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify run pricing request failed")?;
    response_json(response, "Apify run pricing request").await
}

async fn ensure_apify_success(response: Response, operation: &str) -> Result<()> {
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
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

pub(crate) async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    items: &[Value],
) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(items)
        .send()
        .await
        .context("Apify dataset write failed")?;
    ensure_apify_success(response, "Apify dataset write").await
}

pub(crate) async fn charge_event(
    client: &Client,
    config: &ActorConfig,
    search_index: usize,
    count: usize,
) -> Result<()> {
    if count == 0 {
        return Ok(());
    }
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id, "charge"],
    )?;
    let idempotency_key = format!(
        "{}-{BOOKING_RESULT_CHARGE_EVENT}-{search_index}",
        config.actor_run_id
    );
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .header("idempotency-key", idempotency_key)
        .json(&json!({ "eventName": BOOKING_RESULT_CHARGE_EVENT, "count": count }))
        .send()
        .await
        .context("Apify event charge request failed")?;
    ensure_apify_success(response, "Apify event charge request").await
}

pub(crate) async fn set_status_message(
    client: &Client,
    config: &ActorConfig,
    message: &str,
) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .put(url)
        .bearer_auth(&config.apify_token)
        .json(&json!({
            "runId": config.actor_run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true
        }))
        .send()
        .await
        .context("Apify run status update failed")?;
    ensure_apify_success(response, "Apify run status update").await
}
