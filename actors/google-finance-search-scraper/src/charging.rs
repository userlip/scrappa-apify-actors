use std::collections::HashMap;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;

use crate::apify::ApifyClient;

pub(crate) const FINANCE_SEARCH_RESULT_EVENT: &str = "finance-search-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

#[derive(Default)]
pub(crate) struct ChargingManager {
    is_pay_per_event: bool,
    event_prices: HashMap<String, f64>,
    charged_counts: HashMap<String, u64>,
    max_total_charge_usd: f64,
}

impl ChargingManager {
    pub(crate) fn from_run(run_response: &Value) -> Result<Self> {
        let run = run_response
            .get("data")
            .filter(|data| data.is_object())
            .unwrap_or(run_response);
        let pricing = run.get("pricingInfo");
        let is_pay_per_event = pricing
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self::default());
        }

        let event_definitions = pricing
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (event_name, definition) in event_definitions {
            if let Some(price) = definition.get("eventPriceUsd").and_then(Value::as_f64) {
                if !price.is_finite() || price < 0.0 {
                    bail!("Apify run returned an invalid price for event {event_name}");
                }
                event_prices.insert(event_name.clone(), price);
            }
        }

        let max_total_charge_usd = match run.pointer("/options/maxTotalChargeUsd") {
            None | Some(Value::Null) => f64::INFINITY,
            Some(value) => {
                let limit = value
                    .as_f64()
                    .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
                if !limit.is_finite() || limit < 0.0 {
                    bail!("Apify run returned an invalid spending limit");
                }
                limit
            }
        };

        let mut charged_counts = HashMap::new();
        if let Some(counts) = run.get("chargedEventCounts").and_then(Value::as_object) {
            for (event_name, count) in counts {
                let count = count.as_u64().ok_or_else(|| {
                    anyhow!("Apify run returned an invalid charged count for {event_name}")
                })?;
                charged_counts.insert(event_name.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event,
            event_prices,
            charged_counts,
            max_total_charge_usd,
        })
    }

    pub(crate) fn max_items_within_budget(&self, requested: usize) -> Result<usize> {
        if !self.is_pay_per_event {
            return Ok(requested);
        }

        let event_price = self
            .event_prices
            .get(FINANCE_SEARCH_RESULT_EVENT)
            .copied()
            .ok_or_else(|| {
                anyhow!("Apify run did not provide a price for {FINANCE_SEARCH_RESULT_EVENT}")
            })?;
        let dataset_item_price = self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0.0);
        let item_price = event_price + dataset_item_price;
        if !item_price.is_finite() {
            bail!("Apify run returned invalid charging values");
        }
        if item_price == 0.0 || self.max_total_charge_usd.is_infinite() {
            return Ok(requested);
        }

        let charged_total =
            self.charged_counts
                .iter()
                .try_fold(0.0, |total, (event_name, count)| {
                    let price = self.event_prices.get(event_name).copied().unwrap_or(0.0);
                    let next_total = total + price * (*count as f64);
                    if next_total.is_finite() {
                        Ok(next_total)
                    } else {
                        Err(anyhow!("Apify run returned invalid charged totals"))
                    }
                })?;
        let charged_total = (charged_total * 1_000_000.0).round() / 1_000_000.0;
        let remaining = self.max_total_charge_usd - charged_total;
        if remaining <= 0.0 {
            return Ok(0);
        }

        // Match the SDK's four-decimal rounding before flooring to avoid float noise at the cap.
        let affordable = ((remaining / item_price) * 10_000.0).round() / 10_000.0;
        Ok(requested.min(affordable.floor() as usize))
    }

    pub(crate) fn record_saved_items(&mut self, count: usize) -> Result<()> {
        if !self.is_pay_per_event || count == 0 {
            return Ok(());
        }
        increment_count(&mut self.charged_counts, FINANCE_SEARCH_RESULT_EVENT, count)?;
        if self.event_prices.contains_key(DEFAULT_DATASET_ITEM_EVENT) {
            increment_count(&mut self.charged_counts, DEFAULT_DATASET_ITEM_EVENT, count)?;
        }
        Ok(())
    }
}

fn increment_count(
    counts: &mut HashMap<String, u64>,
    event_name: &str,
    count: usize,
) -> Result<()> {
    let count = u64::try_from(count).context("Dataset row count is too large")?;
    let current = counts.entry(event_name.to_owned()).or_default();
    *current = current
        .checked_add(count)
        .ok_or_else(|| anyhow!("Charged event count overflowed"))?;
    Ok(())
}
pub(crate) struct PushSearchItemsResult {
    pub(crate) pushed: bool,
    pub(crate) status_message: Option<String>,
}

pub(crate) async fn push_search_items(
    apify: &ApifyClient<'_>,
    charging: &mut ChargingManager,
    items: &[Value],
    charge_sequence: usize,
) -> Result<PushSearchItemsResult> {
    if items.is_empty() {
        return Ok(PushSearchItemsResult {
            pushed: true,
            status_message: None,
        });
    }

    let limit = charging.max_items_within_budget(items.len())?;
    let saved_items = &items[..limit];
    if !saved_items.is_empty() {
        apify.push_dataset_items(saved_items).await?;
        if charging.is_pay_per_event {
            let idempotency_key = format!(
                "finance-search-{}-{charge_sequence}",
                apify.config.actor_run_id
            );
            apify
                .charge_event(
                    FINANCE_SEARCH_RESULT_EVENT,
                    saved_items.len(),
                    &idempotency_key,
                )
                .await?;
            charging.record_saved_items(saved_items.len())?;
        }
    }

    if limit < items.len() {
        return Ok(PushSearchItemsResult {
            pushed: false,
            status_message: Some(
                "Charge limit reached before saving all Google Finance search results.".to_owned(),
            ),
        });
    }
    Ok(PushSearchItemsResult {
        pushed: true,
        status_message: None,
    })
}
