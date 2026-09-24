use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Method, Response, StatusCode};
use serde_json::Value;
use std::{env, time::Duration};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub struct ApifyConfig {
    pub apify_api_base: Url,
    pub scrappa_api_base: String,
    pub apify_token: String,
    pub key_value_store_id: String,
    pub dataset_id: String,
    pub actor_run_id: String,
    pub input_key: String,
}

impl ApifyConfig {
    pub fn from_env() -> Result<Self> {
        let apify_api_base = base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?;
        let scrappa_api_base = env::var("SCRAPPA_API_BASE_URL")
            .unwrap_or_else(|_| "https://scrappa.co/api".to_owned());
        Url::parse(&scrappa_api_base)
            .context("SCRAPPA_API_BASE_URL must be a valid absolute URL")?;
        Ok(Self {
            apify_api_base,
            scrappa_api_base,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
        })
    }
}

#[derive(Default)]
pub struct DatasetBudget {
    initial_dataset_items: Option<u64>,
    saved_dataset_items: u64,
}

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub fn new(base_url: &Url, token: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("Could not create Apify HTTP client")?;
        Ok(Self {
            client,
            base_url: base_url.clone(),
            token: token.to_owned(),
        })
    }

    pub async fn get_input(&self, store_id: &str, input_key: &str) -> Result<Option<Value>> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&["key-value-stores", store_id, "records", input_key])?,
            )
            .send()
            .await
            .context("Apify INPUT request failed")?;
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

    pub async fn push_dataset_items(
        &self,
        dataset_id: &str,
        actor_run_id: &str,
        items: &[Value],
        budget: &mut DatasetBudget,
    ) -> Result<usize> {
        if items.is_empty() {
            return Ok(0);
        }
        let run_response = self
            .request(
                Method::GET,
                self.resource_url(&["actor-runs", actor_run_id])?,
            )
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run_response = successful_response(run_response, "fetch Actor run pricing").await?;
        let run: Value = run_response
            .json()
            .await
            .context("Actor run pricing response is not valid JSON")?;
        let limit = affordable_dataset_items(&run, items.len(), budget)?;
        let items = &items[..limit];
        if items.is_empty() {
            return Ok(0);
        }

        let saved_count = u64::try_from(items.len()).context("Dataset row count is too large")?;
        let new_saved_count = budget
            .saved_dataset_items
            .checked_add(saved_count)
            .ok_or_else(|| anyhow!("Dataset row count overflowed"))?;
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["datasets", dataset_id, "items"])?,
            )
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        successful_response(response, "store dataset items").await?;
        budget.saved_dataset_items = new_saved_count;
        Ok(items.len())
    }

    pub async fn put_output(&self, store_id: &str, output: &Value) -> Result<()> {
        let response = self
            .request(
                Method::PUT,
                self.resource_url(&["key-value-stores", store_id, "records", "OUTPUT"])?,
            )
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot be a base URL"))?;
        segments
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(resource.iter().copied()));
        drop(segments);
        Ok(url)
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))
        .and_then(|value| {
            if value.is_empty() {
                bail!("Required environment variable {name} is empty");
            }
            Ok(value)
        })
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn affordable_dataset_items(
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
    let current_dataset_items = counts
        .get(DATASET_ITEM_EVENT)
        .map(|count| {
            count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {DATASET_ITEM_EVENT}"))
        })
        .transpose()?
        .unwrap_or(0);
    let initial_dataset_items = *budget
        .initial_dataset_items
        .get_or_insert(current_dataset_items);
    let local_dataset_items = initial_dataset_items
        .checked_add(budget.saved_dataset_items)
        .ok_or_else(|| anyhow!("Dataset row count overflowed"))?;

    let mut spent = 0.0;
    let mut saw_dataset_count = false;
    for (event_name, count) in counts {
        let mut count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if event_name == DATASET_ITEM_EVENT {
            saw_dataset_count = true;
            count = count.max(local_dataset_items);
        }
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
    if !saw_dataset_count && local_dataset_items > 0 {
        spent += item_price * local_dataset_items as f64;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(max_charge: f64) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.25},
                            "other-event": {"eventPriceUsd": 0.50}
                        }
                    }
                },
                "chargedEventCounts": {
                    "apify-default-dataset-item": 2,
                    "other-event": 1
                },
                "options": {"maxTotalChargeUsd": max_charge}
            }
        })
    }

    #[test]
    fn limits_rows_by_remaining_run_spend_including_other_events() {
        let run = run(1.75);
        let mut budget = DatasetBudget::default();
        assert_eq!(affordable_dataset_items(&run, 10, &mut budget).unwrap(), 3);
    }

    #[test]
    fn keeps_confirmed_local_rows_when_run_metadata_lags() {
        let run = run(2.25);
        let mut budget = DatasetBudget::default();
        budget.initial_dataset_items = Some(2);
        budget.saved_dataset_items = 2;
        assert_eq!(affordable_dataset_items(&run, 10, &mut budget).unwrap(), 3);
    }

    #[test]
    fn does_not_charge_zero_price_events_against_budget() {
        let mut run = run(0.0);
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            ["apify-default-dataset-item"]["eventPriceUsd"] = json!(0.0);
        run["data"]["options"]["maxTotalChargeUsd"] = json!(0.0);
        let mut budget = DatasetBudget::default();
        assert_eq!(affordable_dataset_items(&run, 10, &mut budget).unwrap(), 10);
    }

    #[test]
    fn rejects_missing_or_invalid_pay_per_event_metadata() {
        let mut budget = DatasetBudget::default();
        assert!(
            affordable_dataset_items(&json!({"data": {}}), 1, &mut budget)
                .unwrap_err()
                .to_string()
                .contains("not configured for pay-per-event")
        );
        assert!(affordable_dataset_items(
            &json!({
                "data": {
                    "pricingInfo": {"pricingModel": "PAY_PER_EVENT"},
                    "options": {"maxTotalChargeUsd": 1},
                    "chargedEventCounts": {}
                }
            }),
            1,
            &mut budget
        )
        .unwrap_err()
        .to_string()
        .contains("event prices"));
    }
}
