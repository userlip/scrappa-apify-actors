use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Method, RequestBuilder, Response, StatusCode};
use serde_json::{Map, Value};
use std::time::Duration;
use tokio::time::sleep;

use crate::{endpoint_url, ActorConfig};

pub(super) const APIFY_MAX_RETRIES: u8 = 8;
const APIFY_INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);
const MAX_DATASET_REQUEST_BYTES: usize = 9 * 1024 * 1024;
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub(super) fn apify_retry_delay(retry_number: u8) -> Duration {
    APIFY_INITIAL_RETRY_DELAY * 2_u32.pow(u32::from(retry_number.saturating_sub(1)))
}

pub(super) fn retryable_apify_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

pub(super) async fn send_apify_request(
    client: &reqwest::Client,
    request: RequestBuilder,
) -> Result<Response> {
    let request = request
        .build()
        .context("Could not build Apify API request")?;
    let retryable_method = request.method() == Method::GET || request.method() == Method::PUT;
    let mut last_error = None;

    for attempt in 0..=APIFY_MAX_RETRIES {
        let request = request
            .try_clone()
            .ok_or_else(|| anyhow!("Apify API request body could not be retried"))?;
        match client.execute(request).await {
            Ok(response)
                if retryable_method
                    && retryable_apify_status(response.status())
                    && attempt < APIFY_MAX_RETRIES =>
            {
                drop(response);
                sleep(apify_retry_delay(attempt + 1)).await;
            }
            Ok(response) => return Ok(response),
            Err(error)
                if retryable_method
                    && (error.is_connect() || error.is_timeout())
                    && attempt < APIFY_MAX_RETRIES =>
            {
                last_error = Some(error);
                sleep(apify_retry_delay(attempt + 1)).await;
            }
            Err(error) => return Err(error).context("Apify API request failed"),
        }
    }

    let error = last_error
        .map(anyhow::Error::from)
        .unwrap_or_else(|| anyhow!("Apify API request failed after retries"));
    Err(error).context("Apify API request failed after retries")
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

async fn response_status(response: Response, operation: &str) -> Result<()> {
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

async fn get_input(client: &reqwest::Client, config: &ActorConfig) -> Result<Option<Value>> {
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
    let response = send_apify_request(
        client,
        client
            .get(url)
            .bearer_auth(&config.apify_token)
            .header(header::ACCEPT, "application/json"),
    )
    .await
    .context("Apify INPUT request failed")?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response_json(response, "Apify INPUT request")
        .await
        .map(Some)
}

fn event_price(events: &Map<String, Value>, event_name: &str) -> Result<f64> {
    let price = events
        .get(event_name)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the price for event {event_name}"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned an invalid price for event {event_name}");
    }
    Ok(price)
}

pub(super) fn affordable_dataset_items(
    run: &Value,
    requested: usize,
    environment_max_charge: Option<f64>,
    locally_saved_dataset_items: usize,
) -> Result<Option<usize>> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let Some(pricing_model) = data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
    else {
        bail!("Apify run pricing model is missing");
    };
    if pricing_model != "PAY_PER_EVENT" {
        return Ok(None);
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = event_price(events, DATASET_ITEM_EVENT)?;
    let max_charge = match data
        .pointer("/options/maxTotalChargeUsd")
        .and_then(Value::as_f64)
    {
        Some(0.0) => None,
        Some(max_charge) => Some(max_charge),
        None => environment_max_charge.filter(|max_charge| *max_charge != 0.0),
    };
    let Some(max_charge) = max_charge else {
        return Ok(Some(requested));
    };
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned an invalid spending limit");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let reported_dataset_items = match counts.get(DATASET_ITEM_EVENT) {
        Some(count) => count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {DATASET_ITEM_EVENT}"))?,
        None => 0,
    };
    let locally_saved_dataset_items = u64::try_from(locally_saved_dataset_items)
        .context("Too many dataset items were saved during this actor run")?;
    let charged_dataset_items = reported_dataset_items.max(locally_saved_dataset_items);
    let mut spent = item_price * charged_dataset_items as f64;
    for (event_name, count) in counts {
        if event_name == DATASET_ITEM_EVENT {
            continue;
        }
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if count > 0 {
            let price = events
                .get(event_name)
                .and_then(|event| event.get("eventPriceUsd"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned an invalid price for event {event_name}");
            }
            spent += price * count as f64;
        }
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(Some(requested));
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let affordable = (1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .count();
    Ok(Some(affordable))
}

async fn run_dataset_capacity(
    client: &reqwest::Client,
    config: &ActorConfig,
    requested: usize,
    locally_saved_dataset_items: usize,
) -> Result<Option<usize>> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = send_apify_request(
        client,
        client
            .get(url)
            .bearer_auth(&config.apify_token)
            .header(header::ACCEPT, "application/json"),
    )
    .await
    .context("Apify run pricing request failed")?;
    let run = response_json(response, "Apify run pricing request").await?;
    affordable_dataset_items(
        &run,
        requested,
        config.max_total_charge_usd,
        locally_saved_dataset_items,
    )
}

fn dataset_chunks(items: &[Value]) -> Result<Vec<&[Value]>> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut byte_count = 2;

    for (index, item) in items.iter().enumerate() {
        let item_bytes = serde_json::to_vec(item)
            .context("Could not serialize Apify dataset item")?
            .len();
        if item_bytes + 2 > MAX_DATASET_REQUEST_BYTES {
            bail!("Apify dataset items cannot exceed 9 MB");
        }
        let separator_bytes = usize::from(index > start);
        if index > start && byte_count + separator_bytes + item_bytes > MAX_DATASET_REQUEST_BYTES {
            chunks.push(&items[start..index]);
            start = index;
            byte_count = 2;
        }
        byte_count += usize::from(index > start) + item_bytes;
    }
    if start < items.len() {
        chunks.push(&items[start..]);
    }
    Ok(chunks)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct DatasetSave {
    pub(super) saved_count: usize,
    pub(super) charge_limit_reached: bool,
}

#[derive(Debug, Default)]
struct DatasetBudget {
    saved_items: usize,
}

async fn push_dataset_items(
    client: &reqwest::Client,
    config: &ActorConfig,
    items: &[Value],
    dataset_budget: &mut DatasetBudget,
) -> Result<DatasetSave> {
    if items.is_empty() {
        return Ok(DatasetSave::default());
    }

    let capacity =
        run_dataset_capacity(client, config, items.len(), dataset_budget.saved_items).await?;
    let saved_count = capacity.unwrap_or(items.len()).min(items.len());
    if saved_count == 0 {
        return Ok(DatasetSave {
            saved_count: 0,
            charge_limit_reached: true,
        });
    }

    let items_to_save = &items[..saved_count];
    let chunks = dataset_chunks(items_to_save)?;
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;

    for chunk in chunks {
        let response = send_apify_request(
            client,
            client
                .post(url.clone())
                .bearer_auth(&config.apify_token)
                .header(header::ACCEPT, "application/json")
                .json(chunk),
        )
        .await
        .context("Apify dataset write failed")?;
        response_status(response, "Apify dataset write").await?;
        dataset_budget.saved_items = dataset_budget
            .saved_items
            .checked_add(chunk.len())
            .ok_or_else(|| anyhow!("Dataset item count exceeded the actor's capacity"))?;
    }

    Ok(DatasetSave {
        saved_count,
        charge_limit_reached: saved_count < items.len(),
    })
}

async fn set_output_value(
    client: &reqwest::Client,
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
            "OUTPUT",
        ],
    )?;
    let response = send_apify_request(
        client,
        client
            .put(url)
            .bearer_auth(&config.apify_token)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .json(output),
    )
    .await
    .context("Apify OUTPUT write failed")?;
    response_status(response, "Apify OUTPUT write").await
}

pub(super) struct ApifyClient<'a> {
    client: &'a Client,
    config: &'a ActorConfig,
    dataset_budget: DatasetBudget,
}

impl<'a> ApifyClient<'a> {
    pub(super) fn new(client: &'a Client, config: &'a ActorConfig) -> Self {
        Self {
            client,
            config,
            dataset_budget: DatasetBudget::default(),
        }
    }

    pub(super) async fn get_input(&self) -> Result<Option<Value>> {
        get_input(self.client, self.config).await
    }

    pub(super) async fn push_dataset_items(&mut self, items: &[Value]) -> Result<DatasetSave> {
        push_dataset_items(self.client, self.config, items, &mut self.dataset_budget).await
    }

    pub(super) async fn set_output_value(&self, output: &Value) -> Result<()> {
        set_output_value(self.client, self.config, output).await
    }
}
