use std::{
    env,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Response, header};
use serde_json::{Value, json};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const RELATED_RESULT_CHARGE_EVENT: &str = "related-result";

pub struct ApifyConfig {
    api_base_url: Url,
    api_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
}

impl ApifyConfig {
    pub fn from_env() -> Result<Self> {
        let api_base_url =
            env::var("APIFY_API_PUBLIC_BASE_URL").unwrap_or_else(|_| APIFY_API_DEFAULT.to_owned());
        Ok(Self {
            api_base_url: Url::parse(&api_base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid absolute URL")?,
            api_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatasetPushResult {
    pub charged_count: usize,
    pub event_charge_limit_reached: bool,
}

pub struct ApifyClient {
    http: Client,
    config: ApifyConfig,
}

impl ApifyClient {
    pub fn new(config: ApifyConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .context("Failed to initialize Apify HTTP client")?;
        Ok(Self { http, config })
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = endpoint_url(
            &self.config.api_base_url,
            &[
                "v2",
                "key-value-stores",
                &self.config.key_value_store_id,
                "records",
                &self.config.input_key,
            ],
        )?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.config.api_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == reqwest::StatusCode::NOT_FOUND
            || response.status() == reqwest::StatusCode::NO_CONTENT
        {
            return Ok(None);
        }
        response_json(response, "Apify INPUT request")
            .await
            .map(Some)
    }

    pub async fn push_dataset_items(&self, items: &[Value]) -> Result<DatasetPushResult> {
        if items.is_empty() {
            return Ok(DatasetPushResult {
                charged_count: 0,
                event_charge_limit_reached: false,
            });
        }

        let run = self.get_run().await?;
        if run
            .pointer("/data/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            self.write_dataset_items(items).await?;
            return Ok(DatasetPushResult {
                charged_count: items.len(),
                event_charge_limit_reached: false,
            });
        }

        let charged_count = affordable_event_count(&run, RELATED_RESULT_CHARGE_EVENT, items.len())?;
        if charged_count == 0 {
            return Ok(DatasetPushResult {
                charged_count,
                event_charge_limit_reached: true,
            });
        }

        self.charge_event(RELATED_RESULT_CHARGE_EVENT, charged_count)
            .await?;
        self.write_dataset_items(&items[..charged_count]).await?;
        Ok(DatasetPushResult {
            charged_count,
            event_charge_limit_reached: charged_count < items.len(),
        })
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let url = endpoint_url(
            &self.config.api_base_url,
            &[
                "v2",
                "key-value-stores",
                &self.config.key_value_store_id,
                "records",
                "OUTPUT",
            ],
        )?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.config.api_token)
            .header(header::ACCEPT, "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }

    pub async fn set_terminal_status_message(&self, status_message: &str) -> Result<()> {
        let url = endpoint_url(
            &self.config.api_base_url,
            &["v2", "actor-runs", &self.config.actor_run_id],
        )?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.config.api_token)
            .header(header::ACCEPT, "application/json")
            .json(&json!({
                "runId": self.config.actor_run_id,
                "statusMessage": status_message,
                "isStatusMessageTerminal": true
            }))
            .send()
            .await
            .context("Apify run status message update failed")?;
        ensure_success(response, "Apify run status message update").await
    }

    async fn get_run(&self) -> Result<Value> {
        let url = endpoint_url(
            &self.config.api_base_url,
            &["v2", "actor-runs", &self.config.actor_run_id],
        )?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.config.api_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        response_json(response, "Apify run pricing request").await
    }

    async fn charge_event(&self, event_name: &str, count: usize) -> Result<()> {
        let url = endpoint_url(
            &self.config.api_base_url,
            &["v2", "actor-runs", &self.config.actor_run_id, "charge"],
        )?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let idempotency_key = format!("{}-{event_name}-{timestamp}", self.config.actor_run_id);
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.api_token)
            .header(header::ACCEPT, "application/json")
            .header("idempotency-key", idempotency_key)
            .json(&json!({"eventName": event_name, "count": count}))
            .send()
            .await
            .context("Apify event charge request failed")?;
        ensure_success(response, "Apify event charge").await
    }

    async fn write_dataset_items(&self, items: &[Value]) -> Result<()> {
        let url = endpoint_url(
            &self.config.api_base_url,
            &["v2", "datasets", &self.config.dataset_id, "items"],
        )?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.api_token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("Apify API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read Apify API response")?;
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

fn affordable_event_count(run: &Value, event_name: &str, requested: usize) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = event_price(
        events
            .get(event_name)
            .ok_or_else(|| anyhow!("Apify run did not provide the {event_name} event price"))?,
        event_name,
    )?;
    let Some(max_charge_value) = data.pointer("/options/maxTotalChargeUsd") else {
        return Ok(requested);
    };
    if max_charge_value.is_null() {
        return Ok(requested);
    }
    let max_charge = max_charge_value
        .as_f64()
        .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
    if !item_price.is_finite() || item_price < 0.0 || !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    for (charged_event, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {charged_event}"))?;
        if count == 0 {
            continue;
        }
        let price = event_price(
            events
                .get(charged_event)
                .ok_or_else(|| anyhow!("Missing price for charged event {charged_event}"))?,
            charged_event,
        )?;
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let limit = max_charge + f64::EPSILON * max_charge.max(1.0);
    let mut low = 0;
    let mut high = requested;
    while low < high {
        let distance = high - low;
        let middle = low + distance / 2 + distance % 2;
        if spent + middle as f64 * item_price <= limit {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    Ok(low)
}

fn event_price(event: &Value, event_name: &str) -> Result<f64> {
    let price = event
        .get("eventPriceUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the {event_name} event price"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned an invalid price for charged event {event_name}");
    }
    Ok(price)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(max_charge: Value, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel":"PAY_PER_EVENT",
                    "pricingPerEvent":{"actorChargeEvents":{
                        "related-result":{"eventPriceUsd":0.10},
                        "other":{"eventPriceUsd":0.20}
                    }}
                },
                "chargedEventCounts":counts,
                "options":{"maxTotalChargeUsd":max_charge}
            }
        })
    }

    #[test]
    fn caps_rows_using_all_existing_event_charges() {
        let run = run(json!(0.60), json!({"related-result":1,"other":1}));
        assert_eq!(
            affordable_event_count(&run, "related-result", 5).unwrap(),
            3
        );
    }

    #[test]
    fn allows_all_rows_without_a_limit_and_handles_zero_price() {
        assert_eq!(
            affordable_event_count(&run(Value::Null, json!({})), "related-result", 9).unwrap(),
            9
        );
        let mut free_run = run(json!(0.0), json!({}));
        free_run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]["related-result"]
            ["eventPriceUsd"] = json!(0.0);
        assert_eq!(
            affordable_event_count(&free_run, "related-result", 9).unwrap(),
            9
        );
    }

    #[test]
    fn reports_no_affordable_rows_when_other_charges_used_the_budget() {
        let run = run(json!(0.19), json!({"other":1}));
        assert_eq!(
            affordable_event_count(&run, "related-result", 5).unwrap(),
            0
        );
    }

    #[test]
    fn requires_configured_event_price_and_valid_counts() {
        let mut missing_event_run = run(json!(1.0), json!({}));
        missing_event_run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove("related-result");
        assert!(
            affordable_event_count(&missing_event_run, "related-result", 2)
                .unwrap_err()
                .to_string()
                .contains("event price")
        );
        let invalid_count_run = run(json!(1.0), json!({"other":-1}));
        assert!(affordable_event_count(&invalid_count_run, "related-result", 2).is_err());
    }
}
