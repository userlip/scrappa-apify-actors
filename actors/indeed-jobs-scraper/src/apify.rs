use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use std::env;
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const MAX_DATASET_REQUEST_BYTES: usize = 5 * 1024 * 1024;

pub struct ApifyConfig {
    api_base_url: Url,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    token: String,
}

impl ApifyConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            token: required_env("APIFY_TOKEN")?,
        })
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.api_base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("Apify API base URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(segments);
        Ok(url)
    }
}

pub struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a ApifyConfig,
}

impl<'a> ApifyClient<'a> {
    pub fn new(http: &'a Client, config: &'a ApifyConfig) -> Self {
        Self { http, config }
    }

    pub async fn get_input(&self) -> Result<Value> {
        let url = self.config.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.config.token)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Value::Null);
        }
        response_json(response, "Apify INPUT request").await
    }

    pub async fn push_dataset_items(&self, items: &[Value]) -> Result<usize> {
        if items.is_empty() {
            return Ok(0);
        }

        let run_url = self
            .config
            .endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .get(run_url)
            .bearer_auth(&self.config.token)
            .header("Accept", "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = response_json(response, "Apify run pricing request").await?;
        let limit = affordable_dataset_items(&run, items.len())?;
        if limit == 0 {
            return Ok(0);
        }

        let items = &items[..limit];
        let chunks = dataset_chunks(items)?;
        for chunk in chunks {
            let url =
                self.config
                    .endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
            let response = self
                .http
                .post(url)
                .bearer_auth(&self.config.token)
                .header("Accept", "application/json")
                .json(chunk)
                .send()
                .await
                .context("Apify dataset write failed")?;
            ensure_success(response, "Apify dataset write").await?;
        }
        Ok(items.len())
    }

    pub async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.config.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.config.token)
            .header("Accept", "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
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

fn affordable_dataset_items(run: &Value, requested: usize) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let pricing_model = data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str);
    if pricing_model != Some("PAY_PER_EVENT") {
        return Ok(requested);
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = events
        .get(DATASET_ITEM_EVENT)
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
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let affordable = ((max_charge + tolerance - spent) / item_price).floor();
    if affordable <= 0.0 {
        return Ok(0);
    }
    Ok(requested.min(affordable as usize))
}

fn dataset_chunks(items: &[Value]) -> Result<Vec<&[Value]>> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut bytes = 2;

    for (index, item) in items.iter().enumerate() {
        let item_bytes = serde_json::to_vec(item)?.len();
        if item_bytes + 2 > MAX_DATASET_REQUEST_BYTES {
            bail!("An Indeed job result exceeds Apify's 5 MB dataset request limit");
        }
        let separator_bytes = usize::from(index > start);
        if bytes + separator_bytes + item_bytes > MAX_DATASET_REQUEST_BYTES {
            chunks.push(&items[start..index]);
            start = index;
            bytes = 2;
        }
        bytes += usize::from(index > start) + item_bytes;
    }
    if start < items.len() {
        chunks.push(&items[start..]);
    }
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn priced_run(max_charge: f64, counts: Value) -> Value {
        json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {
                "actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0002},
                    "apify-actor-start": {"eventPriceUsd": 0.00005}
                }
            }},
            "options": {"maxTotalChargeUsd": max_charge},
            "chargedEventCounts": counts
        }})
    }

    #[test]
    fn counts_only_items_that_fit_the_remaining_ppe_budget() {
        let run = priced_run(
            0.0005,
            json!({"apify-default-dataset-item": 1, "apify-actor-start": 1}),
        );
        assert_eq!(affordable_dataset_items(&run, 10).unwrap(), 1);
    }

    #[test]
    fn supports_non_ppe_and_zero_price_runs_without_capping() {
        assert_eq!(
            affordable_dataset_items(
                &json!({"data": {"pricingInfo": {"pricingModel": "PAY_PER_RESULT"}}}),
                5
            )
            .unwrap(),
            5
        );
        let mut free = priced_run(0.0, json!({}));
        free["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            ["apify-default-dataset-item"]["eventPriceUsd"] = json!(0.0);
        assert_eq!(affordable_dataset_items(&free, 5).unwrap(), 5);
    }

    #[test]
    fn rejects_incomplete_ppe_metadata_instead_of_writing_unbudgeted_results() {
        let missing_counts = priced_run(1.0, Value::Null);
        assert!(affordable_dataset_items(&missing_counts, 1).is_err());

        let missing_price = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {
                "actorChargeEvents": {}
            }},
            "options": {"maxTotalChargeUsd": 1.0},
            "chargedEventCounts": {}
        }});
        assert!(affordable_dataset_items(&missing_price, 1).is_err());
    }

    #[test]
    fn batches_dataset_items_below_the_apify_payload_limit() {
        let item = json!({"description": "x".repeat(2_600_000)});
        let items = vec![item.clone(), item.clone(), item.clone()];
        let chunks = dataset_chunks(&items).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks.iter().map(|chunk| chunk.len()).sum::<usize>(), 3);
    }
}
