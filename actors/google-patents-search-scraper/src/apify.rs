use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response};
use serde_json::{json, Value};
use std::env;
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub struct ActorConfig {
    apify_api_base_url: Url,
    pub scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
    pub scrappa_api_key: String,
}

impl ActorConfig {
    pub fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|key| !key.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base_url: base_url_from_env(
                "SCRAPPA_API_BASE_URL",
                "https://scrappa.co/api",
            )?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
        })
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.apify_api_base_url, segments)
    }

    #[cfg(test)]
    pub(crate) fn for_test(apify_api_base_url: Url, scrappa_api_base_url: Url) -> Self {
        Self {
            apify_api_base_url,
            scrappa_api_base_url,
            default_key_value_store_id: "store-test".to_owned(),
            default_dataset_id: "dataset-test".to_owned(),
            actor_run_id: "run-test".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "apify-test-token".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
        }
    }
}

pub struct DatasetSave {
    pub saved_count: usize,
    pub charge_limit_reached: bool,
}

pub struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a ActorConfig,
}

impl<'a> ApifyClient<'a> {
    pub fn new(http: &'a Client, config: &'a ActorConfig) -> Self {
        Self { http, config }
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.config.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.default_key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
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

    pub async fn save_dataset_items(&self, items: &[Value]) -> Result<DatasetSave> {
        if items.is_empty() {
            return Ok(DatasetSave {
                saved_count: 0,
                charge_limit_reached: false,
            });
        }

        let run_url = self
            .config
            .endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .get(run_url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = response_json(response, "Apify run pricing request").await?;
        let affordable_count = affordable_dataset_items(&run, items.len())?;
        let affordable_items = &items[..affordable_count];
        if affordable_items.is_empty() {
            return Ok(DatasetSave {
                saved_count: 0,
                charge_limit_reached: true,
            });
        }

        let url =
            self.config
                .endpoint(&["v2", "datasets", &self.config.default_dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(affordable_items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await?;

        Ok(DatasetSave {
            saved_count: affordable_count,
            charge_limit_reached: affordable_count < items.len(),
        })
    }

    pub async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.config.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.default_key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }

    pub async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self
            .config
            .endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(&json!({ "statusMessage": message }))
            .send()
            .await
            .context("Apify status message request failed")?;
        ensure_success(response, "Apify status message request").await
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
        .extend(segments.iter().copied());
    Ok(url)
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
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
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
    if !item_price.is_finite() || item_price < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .and_then(Value::as_f64)
        .or_else(|| {
            env::var("ACTOR_MAX_TOTAL_CHARGE_USD")
                .ok()
                .and_then(|value| value.parse::<f64>().ok())
        })
        .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
    if !max_charge.is_finite() || max_charge < 0.0 {
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
    let affordable = ((max_charge - spent + tolerance) / item_price)
        .floor()
        .max(0.0);
    Ok(requested.min(affordable as usize))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn priced_run(max_charge: f64, charged_events: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "apify-default-dataset-item": { "eventPriceUsd": 0.1 },
                        "apify-actor-start": { "eventPriceUsd": 0.05 }
                    }}
                },
                "options": { "maxTotalChargeUsd": max_charge },
                "chargedEventCounts": charged_events
            }
        })
    }

    #[test]
    fn caps_dataset_items_after_counting_every_charged_event() {
        assert_eq!(
            affordable_dataset_items(&priced_run(0.26, json!({"apify-actor-start":1})), 5).unwrap(),
            2
        );
        assert_eq!(
            affordable_dataset_items(&priced_run(0.15, json!({"apify-actor-start":1})), 5).unwrap(),
            1
        );
        assert_eq!(
            affordable_dataset_items(&priced_run(0.04, json!({"apify-actor-start":1})), 5).unwrap(),
            0
        );
    }

    #[test]
    fn leaves_non_ppe_runs_unlimited_and_fails_closed_on_bad_ppe_metadata() {
        assert_eq!(
            affordable_dataset_items(&json!({"data":{"pricingInfo":{"pricingModel":"FREE"}}}), 5)
                .unwrap(),
            5
        );
        let missing_event_count = priced_run(1.0, Value::Null);
        assert!(affordable_dataset_items(&missing_event_count, 2)
            .unwrap_err()
            .to_string()
            .contains("charged event counts"));
        let missing_price = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "apify-default-dataset-item": { "eventPriceUsd": 0.1 }
                    }}
                },
                "options": { "maxTotalChargeUsd": 1.0 },
                "chargedEventCounts": { "other-event": 1 }
            }
        });
        assert!(affordable_dataset_items(&missing_price, 2)
            .unwrap_err()
            .to_string()
            .contains("Missing price for charged event other-event"));
    }
}
