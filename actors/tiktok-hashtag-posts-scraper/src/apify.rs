use anyhow::{anyhow, bail, Context, Result};
use reqwest::Response;
use serde_json::Value;
use std::{env, time::Duration};
use tokio::time::sleep;
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
pub(crate) const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
pub(crate) const APIFY_MAX_RETRIES: u32 = 8;
const APIFY_RETRY_DELAY_MS: u64 = 500;
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";

pub(crate) struct ActorConfig {
    pub(crate) apify_api_base_url: Url,
    pub(crate) scrappa_api_base_url: Url,
    pub(crate) default_key_value_store_id: String,
    pub(crate) default_dataset_id: String,
    pub(crate) actor_run_id: String,
    pub(crate) input_key: String,
    pub(crate) apify_token: String,
    pub(crate) scrappa_api_key: String,
}

impl ActorConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").unwrap_or_default();
        if scrappa_api_key.is_empty() {
            bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
        }

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
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

pub(crate) async fn send_apify_request<F>(operation: &str, build_request: F) -> Result<Response>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let mut retries = 0;
    loop {
        match build_request().timeout(APIFY_REQUEST_TIMEOUT).send().await {
            Ok(response)
                if (response.status().as_u16() == 429 || response.status().is_server_error())
                    && retries < APIFY_MAX_RETRIES =>
            {
                drop(response);
            }
            Ok(response) => return Ok(response),
            Err(error) if retries < APIFY_MAX_RETRIES => {
                eprintln!("Apify {operation} failed: {error}");
            }
            Err(error) => return Err(error).with_context(|| format!("Apify {operation} failed")),
        }

        let delay = Duration::from_millis(APIFY_RETRY_DELAY_MS * (1_u64 << retries));
        retries += 1;
        eprintln!(
            "Retrying Apify {operation} in {}ms ({retries}/{APIFY_MAX_RETRIES})",
            delay.as_millis()
        );
        sleep(delay).await;
    }
}

pub(crate) async fn get_input(
    client: &reqwest::Client,
    config: &ActorConfig,
) -> Result<Option<Value>> {
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
    let response = send_apify_request("INPUT request", || {
        client.get(url.clone()).bearer_auth(&config.apify_token)
    })
    .await?;
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    response_json(response, "Apify INPUT request")
        .await
        .map(Some)
}
#[derive(Default)]
pub(crate) struct DatasetBudget {
    run: Option<Value>,
    saved_rows: usize,
}

impl DatasetBudget {
    async fn capacity(
        &mut self,
        client: &reqwest::Client,
        config: &ActorConfig,
        requested: usize,
    ) -> Result<usize> {
        if self.run.is_none() {
            let url = endpoint_url(
                &config.apify_api_base_url,
                &["v2", "actor-runs", &config.actor_run_id],
            )?;
            let response = send_apify_request("run pricing request", || {
                client.get(url.clone()).bearer_auth(&config.apify_token)
            })
            .await?;
            self.run = Some(response_json(response, "Apify run pricing request").await?);
        }
        let run = self
            .run
            .as_ref()
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        affordable_dataset_items(run, requested, self.saved_rows)
    }
}

pub(crate) fn affordable_dataset_items(
    run: &Value,
    requested: usize,
    locally_saved_rows: usize,
) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        return Ok(requested);
    }
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => return Ok(requested),
        Some(value) => value
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?,
    };
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    if max_charge == 0.0 {
        return Ok(requested);
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
    if !item_price.is_finite() || item_price < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    for (event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count"))?;
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
    spent += item_price * locally_saved_rows as f64;
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
    client: &reqwest::Client,
    config: &ActorConfig,
    budget: &mut DatasetBudget,
    rows: &[Value],
) -> Result<usize> {
    if rows.is_empty() {
        return Ok(0);
    }
    let limit = budget.capacity(client, config, rows.len()).await?;
    let rows = &rows[..rows.len().min(limit)];
    if rows.is_empty() {
        return Ok(0);
    }
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = send_apify_request("dataset write", || {
        client
            .post(url.clone())
            .bearer_auth(&config.apify_token)
            .json(rows)
    })
    .await?;
    ensure_success(response, "Apify dataset write").await?;
    budget.saved_rows += rows.len();
    Ok(rows.len())
}

pub(crate) async fn put_output(
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
    let response = send_apify_request("OUTPUT write", || {
        client
            .put(url.clone())
            .bearer_auth(&config.apify_token)
            .json(output)
    })
    .await?;
    ensure_success(response, "Apify OUTPUT write").await
}
