use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::env;
use url::Url;

use crate::{url_utils::endpoint_url, REQUEST_TIMEOUT};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const OUTPUT_KEY: &str = "OUTPUT";

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

#[derive(Clone, Debug)]
pub(crate) struct ActorConfig {
    pub(crate) apify_api_base_url: Url,
    pub(crate) scrappa_api_base_url: Url,
    pub(crate) default_key_value_store_id: String,
    pub(crate) default_dataset_id: String,
    pub(crate) input_key: String,
    pub(crate) actor_run_id: String,
    pub(crate) apify_token: String,
    pub(crate) scrappa_api_key: String,
}

impl ActorConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        if scrappa_api_key.trim().is_empty() {
            bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
        }

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            apify_token: required_env("APIFY_TOKEN")?,
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
        .header(reqwest::header::ACCEPT, "application/json")
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify INPUT request failed")?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response_json(response, "Apify INPUT request")
        .await
        .map(Some)
}

fn apify_run_url(base_url: &Url, run_id: &str) -> Result<Url> {
    endpoint_url(base_url, &["v2", "actor-runs", run_id])
}

#[derive(Default)]
pub(crate) struct DatasetBudget {
    initial_dataset_item_charges: Option<u64>,
    locally_confirmed_rows: u64,
}

impl DatasetBudget {
    fn effective_dataset_item_charges(&mut self, reported: u64) -> u64 {
        let initial = *self.initial_dataset_item_charges.get_or_insert(reported);
        reported.max(initial.saturating_add(self.locally_confirmed_rows))
    }

    fn record_successful_write(&mut self, rows: usize) {
        self.locally_confirmed_rows = self.locally_confirmed_rows.saturating_add(rows as u64);
    }
}

async fn run_dataset_capacity(
    client: &Client,
    config: &ActorConfig,
    requested: usize,
    budget: &mut DatasetBudget,
) -> Result<usize> {
    let response = client
        .get(apify_run_url(
            &config.apify_api_base_url,
            &config.actor_run_id,
        )?)
        .timeout(REQUEST_TIMEOUT)
        .header(reqwest::header::ACCEPT, "application/json")
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify run pricing request failed")?;
    let run = response_json(response, "Apify run pricing request").await?;
    affordable_dataset_items(&run, requested, budget)
}

pub(crate) fn affordable_dataset_items(
    run: &Value,
    requested: usize,
    budget: &mut DatasetBudget,
) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        bail!("Apify run is not configured for pay-per-event pricing");
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = events
        .get("apify-default-dataset-item")
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
    if !item_price.is_finite() || item_price < 0.0 || !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let reported_dataset_item_charges = counts
        .get("apify-default-dataset-item")
        .map(|count| {
            count.as_u64().ok_or_else(|| {
                anyhow!("Invalid charged event count for apify-default-dataset-item")
            })
        })
        .transpose()?
        .unwrap_or(0);
    let effective_dataset_item_charges =
        budget.effective_dataset_item_charges(reported_dataset_item_charges);

    let mut spent = 0.0;
    let has_dataset_item_count = counts.contains_key("apify-default-dataset-item");
    for (event_name, count) in counts {
        let reported_count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        let count = if event_name == "apify-default-dataset-item" {
            effective_dataset_item_charges
        } else {
            reported_count
        };
        if count == 0 {
            continue;
        }
        let price = events
            .get(event_name)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        spent += price * count as f64;
    }
    if !has_dataset_item_count && effective_dataset_item_charges > 0 {
        spent += item_price * effective_dataset_item_charges as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .count())
}

pub(crate) async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    items: &[Value],
    budget: &mut DatasetBudget,
) -> Result<usize> {
    if items.is_empty() {
        return Ok(0);
    }

    let capacity = run_dataset_capacity(client, config, items.len(), budget).await?;
    let items = &items[..items.len().min(capacity)];
    if items.is_empty() {
        return Ok(0);
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
    ensure_success(response, "Apify dataset write").await?;
    budget.record_successful_write(items.len());
    Ok(items.len())
}

pub(crate) async fn set_output(
    client: &Client,
    config: &ActorConfig,
    output: &Value,
) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            OUTPUT_KEY,
        ],
    )?;
    let body = serde_json::to_vec(output).context("Could not encode OUTPUT value")?;
    let response = client
        .put(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .bearer_auth(&config.apify_token)
        .body(body)
        .send()
        .await
        .context("Apify OUTPUT write failed")?;
    ensure_success(response, "Apify OUTPUT write").await
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

pub(crate) fn http_client() -> Result<Client> {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("Could not create HTTP client")
}
