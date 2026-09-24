use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, StatusCode};
use serde_json::{json, Value};
use url::Url;

use crate::config::{endpoint_url, ensure_success, response_json, Config};

pub(crate) const LISTING_RESULT_CHARGE_EVENT: &str = "listing-result";
const DEFAULT_DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";
const APIFY_MAX_CHARGE_RETRIES: usize = 8;
const APIFY_CHARGE_RETRY_DELAY: Duration = Duration::from_millis(500);

pub struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

impl<'a> ApifyClient<'a> {
    pub fn new(http: &'a Client, config: &'a Config) -> Self {
        Self { http, config }
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
    }

    fn authenticated(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
    }

    pub(super) async fn get_input(&self) -> Result<Value> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .authenticated(self.http.get(url))
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(json!({}));
        }
        response_json(response, "Apify INPUT request").await
    }

    pub(super) async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .authenticated(self.http.get(url))
            .send()
            .await
            .context("Apify run pricing request failed")?;
        response_json(response, "Apify run pricing request").await
    }

    async fn charge_event(&self, count: usize, idempotency_key: &str) -> Result<usize> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        for retry in 0..=APIFY_MAX_CHARGE_RETRIES {
            let response = self
                .authenticated(self.http.post(url.clone()))
                .header("idempotency-key", idempotency_key)
                .json(&json!({
                    "eventName": LISTING_RESULT_CHARGE_EVENT,
                    "count": count,
                }))
                .send()
                .await;
            let response = match response {
                Ok(response)
                    if is_retryable_charge_status(response.status())
                        && retry < APIFY_MAX_CHARGE_RETRIES =>
                {
                    tokio::time::sleep(charge_retry_delay(retry)).await;
                    continue;
                }
                Ok(response) => response,
                Err(error)
                    if is_retryable_charge_error(&error) && retry < APIFY_MAX_CHARGE_RETRIES =>
                {
                    tokio::time::sleep(charge_retry_delay(retry)).await;
                    continue;
                }
                Err(error) => {
                    return Err(error).context("Apify listing-result charge request failed");
                }
            };
            if !response.status().is_success() {
                return ensure_success(response, "Apify listing-result charge")
                    .await
                    .map(|()| count);
            }

            let body = match response.text().await {
                Ok(body) => body,
                Err(error)
                    if is_retryable_charge_error(&error) && retry < APIFY_MAX_CHARGE_RETRIES =>
                {
                    tokio::time::sleep(charge_retry_delay(retry)).await;
                    continue;
                }
                Err(error) => {
                    return Err(error)
                        .context("Apify listing-result charge response could not be read");
                }
            };
            return parse_charged_count(&body, count);
        }

        unreachable!("the charge retry loop always returns or retries")
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .authenticated(self.http.post(url))
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }

    pub(super) async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .authenticated(self.http.put(url))
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }

    pub async fn set_terminal_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .authenticated(self.http.put(url))
            .json(&json!({
                "runId": self.config.actor_run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true,
            }))
            .send()
            .await
            .context("Apify run status update failed")?;
        ensure_success(response, "Apify run status update").await
    }
}

fn parse_charged_count(body: &str, requested_count: usize) -> Result<usize> {
    if body.trim().is_empty() {
        return Ok(requested_count);
    }
    let result: Value = serde_json::from_str(body)
        .context("Apify listing-result charge returned invalid JSON")?;
    let charged_count = result
        .get("chargedCount")
        .or_else(|| result.pointer("/data/chargedCount"));
    let Some(charged_count) = charged_count else {
        let limit_reached = result
            .get("eventChargeLimitReached")
            .or_else(|| result.pointer("/data/eventChargeLimitReached"))
            .and_then(Value::as_bool)
            == Some(true);
        if limit_reached {
            bail!("Apify listing-result charge reached the limit without a charged count");
        }
        return Ok(requested_count);
    };
    let charged_count = charged_count.as_u64().ok_or_else(|| {
        anyhow!("Apify listing-result charge returned an invalid charged count")
    })?;
    let charged_count = usize::try_from(charged_count)
        .context("Apify listing-result charge count exceeds this platform's limit")?;
    if charged_count > requested_count {
        bail!("Apify listing-result charge returned more events than requested");
    }
    Ok(charged_count)
}

fn is_retryable_charge_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_retryable_charge_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request() || error.is_body()
}

fn charge_retry_delay(retry: usize) -> Duration {
    APIFY_CHARGE_RETRY_DELAY.saturating_mul(2_u32.saturating_pow(retry as u32))
}

#[derive(Default)]
pub(super) struct DatasetBudget {
    initial_listing_results: Option<u64>,
    initial_default_dataset_items: Option<u64>,
    saved_listing_results: u64,
    charge_attempts: u64,
}

pub(super) fn affordable_listing_count(
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

    let event_prices = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let listing_result_price = event_prices
        .get(LISTING_RESULT_CHARGE_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the listing-result event price"))?;
    if !listing_result_price.is_finite() || listing_result_price < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let max_charge = value
                .as_f64()
                .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
            if !max_charge.is_finite() || max_charge < 0.0 {
                bail!("Apify run returned invalid charging values");
            }
            (max_charge > 0.0).then_some(max_charge)
        }
    };
    let Some(max_charge) = max_charge else {
        return Ok(requested);
    };

    let default_dataset_item_price = match event_prices.get(DEFAULT_DATASET_ITEM_CHARGE_EVENT) {
        Some(event) => event
            .get("eventPriceUsd")
            .and_then(Value::as_f64)
            .ok_or_else(|| {
                anyhow!(
                    "Apify run did not provide the {DEFAULT_DATASET_ITEM_CHARGE_EVENT} event price"
                )
            })?,
        None => 0.0,
    };
    if !default_dataset_item_price.is_finite() || default_dataset_item_price < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    let price_per_listing = listing_result_price + default_dataset_item_price;
    if !price_per_listing.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }

    let charged_counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let current_listing_results = charged_counts
        .get(LISTING_RESULT_CHARGE_EVENT)
        .map(|count| {
            count.as_u64().ok_or_else(|| {
                anyhow!("Invalid charged event count for {LISTING_RESULT_CHARGE_EVENT}")
            })
        })
        .transpose()?
        .unwrap_or(0);
    let current_default_dataset_items = charged_counts
        .get(DEFAULT_DATASET_ITEM_CHARGE_EVENT)
        .map(|count| {
            count.as_u64().ok_or_else(|| {
                anyhow!("Invalid charged event count for {DEFAULT_DATASET_ITEM_CHARGE_EVENT}")
            })
        })
        .transpose()?
        .unwrap_or(0);
    let initial_listing_results = *budget
        .initial_listing_results
        .get_or_insert(current_listing_results);
    let initial_default_dataset_items = *budget
        .initial_default_dataset_items
        .get_or_insert(current_default_dataset_items);
    let local_listing_results = initial_listing_results
        .checked_add(budget.saved_listing_results)
        .ok_or_else(|| anyhow!("Listing result count overflowed"))?;
    let local_default_dataset_items = initial_default_dataset_items
        .checked_add(budget.saved_listing_results)
        .ok_or_else(|| anyhow!("Default dataset item count overflowed"))?;

    let mut spent = 0.0;
    let mut saw_listing_result_count = false;
    let mut saw_default_dataset_item_count = false;
    for (event_name, count) in charged_counts {
        let mut count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if event_name == LISTING_RESULT_CHARGE_EVENT {
            saw_listing_result_count = true;
            count = count.max(local_listing_results);
        } else if event_name == DEFAULT_DATASET_ITEM_CHARGE_EVENT {
            saw_default_dataset_item_count = true;
            count = count.max(local_default_dataset_items);
        }
        if count == 0 {
            continue;
        }

        let price = event_prices
            .get(event_name)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        spent += price * count as f64;
    }
    if !saw_listing_result_count && local_listing_results > 0 {
        spent += listing_result_price * local_listing_results as f64;
    }
    if !saw_default_dataset_item_count && local_default_dataset_items > 0 {
        spent += default_dataset_item_price * local_default_dataset_items as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if price_per_listing == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * price_per_listing <= max_charge + tolerance)
        .count())
}

pub(super) fn is_pay_per_event(run: &Value) -> Result<bool> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    Ok(data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        == Some("PAY_PER_EVENT"))
}

fn next_charge_idempotency_key(run_id: &str, budget: &mut DatasetBudget) -> String {
    budget.charge_attempts = budget.charge_attempts.saturating_add(1);
    format!(
        "{run_id}-{LISTING_RESULT_CHARGE_EVENT}-{}",
        budget.charge_attempts
    )
}

pub(super) struct PushChargedListingsResult {
    pub(super) saved_count: usize,
    pub(super) charge_limit_reached: bool,
}

pub(super) async fn push_charged_listings(
    apify: &ApifyClient<'_>,
    items: &[Value],
    budget: &mut DatasetBudget,
) -> Result<PushChargedListingsResult> {
    if items.is_empty() {
        return Ok(PushChargedListingsResult {
            saved_count: 0,
            charge_limit_reached: false,
        });
    }

    let run = apify.get_run().await?;
    if !is_pay_per_event(&run)? {
        apify.push_dataset_items(items).await?;
        return Ok(PushChargedListingsResult {
            saved_count: items.len(),
            charge_limit_reached: false,
        });
    }

    let saved_count = affordable_listing_count(&run, items.len(), budget)?;
    if saved_count == 0 {
        return Ok(PushChargedListingsResult {
            saved_count: 0,
            charge_limit_reached: true,
        });
    }
    let idempotency_key = next_charge_idempotency_key(&apify.config.actor_run_id, budget);
    let charged_count = apify.charge_event(saved_count, &idempotency_key).await?;
    if charged_count > 0 {
        apify.push_dataset_items(&items[..charged_count]).await?;
    }
    budget.saved_listing_results = budget
        .saved_listing_results
        .checked_add(charged_count as u64)
        .ok_or_else(|| anyhow!("Listing result count overflowed"))?;

    Ok(PushChargedListingsResult {
        saved_count: charged_count,
        charge_limit_reached: charged_count < items.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn limits_ppe_to_budget_after_all_charged_events() {
        let run = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "listing-result": {"eventPriceUsd": 0.1},
                "apify-actor-start": {"eventPriceUsd": 0.05}
            }}},
            "options": {"maxTotalChargeUsd": 0.25},
            "chargedEventCounts": {"listing-result": 1, "apify-actor-start": 1}
        }});
        let mut budget = DatasetBudget::default();
        assert_eq!(affordable_listing_count(&run, 5, &mut budget).unwrap(), 1);
        budget.saved_listing_results = 2;
        assert_eq!(affordable_listing_count(&run, 5, &mut budget).unwrap(), 0);
    }

    #[test]
    fn missing_null_and_zero_limits_are_unbounded() {
        let missing = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "listing-result": {"eventPriceUsd": 0.1}
            }}},
            "options": {},
            "chargedEventCounts": {}
        }});
        let null = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "listing-result": {"eventPriceUsd": 0.1}
            }}},
            "options": {"maxTotalChargeUsd": null},
            "chargedEventCounts": {}
        }});
        let zero = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "listing-result": {"eventPriceUsd": 0.1}
            }}},
            "options": {"maxTotalChargeUsd": 0},
            "chargedEventCounts": {}
        }});
        for run in [missing, null, zero] {
            let mut budget = DatasetBudget::default();
            assert_eq!(affordable_listing_count(&run, 5, &mut budget).unwrap(), 5);
        }
    }

    #[test]
    fn budgets_listing_and_default_dataset_events_with_prior_counts() {
        let run = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "listing-result": {"eventPriceUsd": 0.1},
                "apify-default-dataset-item": {"eventPriceUsd": 0.05},
                "apify-actor-start": {"eventPriceUsd": 0.05}
            }}},
            "options": {"maxTotalChargeUsd": 0.6},
            "chargedEventCounts": {
                "listing-result": 1,
                "apify-default-dataset-item": 1,
                "apify-actor-start": 1
            }
        }});
        let mut budget = DatasetBudget::default();

        assert_eq!(affordable_listing_count(&run, 5, &mut budget).unwrap(), 2);

        budget.saved_listing_results = 1;
        assert_eq!(affordable_listing_count(&run, 5, &mut budget).unwrap(), 1);
    }
}
