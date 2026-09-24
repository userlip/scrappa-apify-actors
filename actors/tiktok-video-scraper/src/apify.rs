use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::Value;
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_MAX_RETRIES: usize = 2;
const DEFAULT_INPUT_KEY: &str = "INPUT";

pub(crate) struct ActorConfig {
    pub(crate) apify_api_base_url: Url,
    pub(crate) scrappa_api_base_url: Url,
    pub(crate) key_value_store_id: String,
    pub(crate) dataset_id: String,
    pub(crate) actor_run_id: String,
    pub(crate) input_key: String,
    pub(crate) apify_token: String,
    pub(crate) scrappa_api_key: String,
}

impl ActorConfig {
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
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| DEFAULT_INPUT_KEY.to_owned()),
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

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

pub(crate) struct ApifyClient<'a> {
    pub(crate) http: &'a Client,
    pub(crate) config: &'a ActorConfig,
}

impl ApifyClient<'_> {
    pub(crate) async fn get_input(&self) -> Result<Option<Value>> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &[
                "v2",
                "key-value-stores",
                &self.config.key_value_store_id,
                "records",
                &self.config.input_key,
            ],
        )?;
        let response =
            apify_get(self.http, url, &self.config.apify_token, "input retrieval").await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let response = require_apify_success(response, "input retrieval").await?;
        response
            .json::<Value>()
            .await
            .context("Apify input record was not valid JSON")
            .map(Some)
    }

    pub(crate) async fn run_pricing(&self) -> Result<Value> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "actor-runs", &self.config.actor_run_id],
        )?;
        let response = apify_get(
            self.http,
            url,
            &self.config.apify_token,
            "run pricing request",
        )
        .await?;
        require_apify_success(response, "run pricing request")
            .await?
            .json::<Value>()
            .await
            .context("Apify run pricing response was not valid JSON")
    }

    pub(crate) async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "datasets", &self.config.dataset_id, "items"],
        )?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .timeout(APIFY_REQUEST_TIMEOUT)
            .json(item)
            .send()
            .await
            .context("Apify dataset write failed")?;
        require_apify_success(response, "dataset write").await?;
        Ok(())
    }
}

pub(crate) async fn apify_get(
    client: &Client,
    url: Url,
    token: &str,
    operation: &str,
) -> Result<Response> {
    for retry_count in 0..=APIFY_MAX_RETRIES {
        let response = client
            .get(url.clone())
            .bearer_auth(token)
            .header(header::ACCEPT, "application/json")
            .timeout(APIFY_REQUEST_TIMEOUT)
            .send()
            .await;

        match response {
            Ok(response)
                if retry_count < APIFY_MAX_RETRIES && retryable_apify_status(response.status()) =>
            {
                eprintln!(
                    "Apify {operation} returned {}; retrying",
                    response.status().as_u16()
                );
                drop(response);
            }
            Ok(response) => return Ok(response),
            Err(error) if retry_count < APIFY_MAX_RETRIES => {
                eprintln!("Apify {operation} failed ({error}); retrying");
            }
            Err(error) => return Err(error).with_context(|| format!("Apify {operation} failed")),
        }

        tokio::time::sleep(Duration::from_secs((retry_count + 1) as u64)).await;
    }

    unreachable!("the bounded Apify retry loop always returns its final response")
}

fn retryable_apify_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

pub(crate) struct DatasetBudget {
    run: Option<Value>,
    saved_rows: usize,
}

impl DatasetBudget {
    pub(crate) fn new() -> Self {
        Self {
            run: None,
            saved_rows: 0,
        }
    }

    pub(crate) async fn can_save_one(&mut self, apify: &ApifyClient<'_>) -> Result<bool> {
        if self.run.is_none() {
            self.run = Some(apify.run_pricing().await?);
        }
        let run = self
            .run
            .as_ref()
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        Ok(affordable_dataset_items(run, 1, self.saved_rows)? > 0)
    }

    pub(crate) fn record_saved_row(&mut self) {
        self.saved_rows += 1;
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
            .ok_or_else(|| anyhow!("Apify run did not provide a valid spending limit"))?,
    };
    if max_charge == 0.0 {
        return Ok(requested);
    }
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
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
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
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
