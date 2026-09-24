use anyhow::{anyhow, bail, Result};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub const BOOKING_RESULT_CHARGE_EVENT: &str = "booking-result";
pub const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

fn run_data(run: &Value) -> &Value {
    run.get("data").unwrap_or(run)
}

pub fn is_pay_per_event(run: &Value) -> bool {
    run_data(run)
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        == Some("PAY_PER_EVENT")
}

#[derive(Debug, PartialEq)]
pub struct ChargePlan {
    pub charged_count: usize,
    pub event_charge_limit_reached: bool,
}

#[derive(Default)]
pub struct ChargeBudget {
    initial_charged_counts: Option<BTreeMap<String, u64>>,
    saved_dataset_items: u64,
    charged_events: BTreeMap<String, u64>,
}

fn priced_event(events: &Map<String, Value>, name: &str, required: bool) -> Result<Option<f64>> {
    let Some(event) = events.get(name) else {
        if required {
            bail!("Apify run did not provide the {name} event price");
        }
        return Ok(None);
    };
    let price = event
        .get("eventPriceUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run returned an invalid {name} event price"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned an invalid {name} event price");
    }
    Ok(Some(price))
}

pub fn charge_plan(
    run: &Value,
    event_name: &str,
    requested: usize,
    budget: &mut ChargeBudget,
) -> Result<ChargePlan> {
    let data = run_data(run);
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
    let event_price = priced_event(events, event_name, true)?.unwrap_or_default();
    let dataset_item_price =
        priced_event(events, DEFAULT_DATASET_ITEM_EVENT, false)?.unwrap_or_default();
    let per_item_price = event_price + dataset_item_price;

    let reported_counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .map(|counts| {
            counts
                .iter()
                .map(|(name, count)| {
                    Ok((
                        name.clone(),
                        count.as_u64().ok_or_else(|| {
                            anyhow!("Apify run returned an invalid charged event count for {name}")
                        })?,
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let initial_counts = budget
        .initial_charged_counts
        .get_or_insert_with(|| reported_counts.clone());
    let mut counts = reported_counts;
    if events.contains_key(DEFAULT_DATASET_ITEM_EVENT) {
        let expected_dataset_items = initial_counts
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0)
            .checked_add(budget.saved_dataset_items)
            .ok_or_else(|| anyhow!("Dataset row count overflowed"))?;
        counts
            .entry(DEFAULT_DATASET_ITEM_EVENT.to_owned())
            .and_modify(|count| *count = (*count).max(expected_dataset_items))
            .or_insert(expected_dataset_items);
    }
    for (name, locally_charged) in &budget.charged_events {
        let expected_count = initial_counts
            .get(name)
            .copied()
            .unwrap_or(0)
            .checked_add(*locally_charged)
            .ok_or_else(|| anyhow!("Charged event count overflowed for {name}"))?;
        counts
            .entry(name.clone())
            .and_modify(|count| *count = (*count).max(expected_count))
            .or_insert(expected_count);
    }
    let mut spent = 0.0;
    for (name, count) in &counts {
        if *count == 0 {
            continue;
        }
        let price = priced_event(events, name, true)?.unwrap_or_default();
        spent += price * *count as f64;
        if !spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }
    }

    let limit_value = data.pointer("/options/maxTotalChargeUsd");
    let max_charge = match limit_value {
        None | Some(Value::Null) => None,
        Some(value) => {
            let limit = value
                .as_f64()
                .filter(|limit| limit.is_finite() && *limit >= 0.0)
                .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
            Some(limit)
        }
    };

    let Some(max_charge) = max_charge else {
        return Ok(ChargePlan {
            charged_count: requested,
            event_charge_limit_reached: false,
        });
    };
    if per_item_price == 0.0 {
        return Ok(ChargePlan {
            charged_count: requested,
            event_charge_limit_reached: false,
        });
    }

    let remaining = max_charge - spent;
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let affordable = if remaining < -tolerance {
        0
    } else {
        let count = ((remaining.max(0.0) + tolerance) / per_item_price).floor();
        if count >= usize::MAX as f64 {
            usize::MAX
        } else {
            count as usize
        }
    };
    Ok(ChargePlan {
        charged_count: requested.min(affordable),
        event_charge_limit_reached: affordable <= requested,
    })
}

impl ChargeBudget {
    pub fn record_dataset_items_saved(&mut self, count: usize) -> Result<()> {
        self.saved_dataset_items = self
            .saved_dataset_items
            .checked_add(count as u64)
            .ok_or_else(|| anyhow!("Dataset row count overflowed"))?;
        Ok(())
    }

    pub fn record_charged_event(&mut self, name: &str, count: usize) -> Result<()> {
        let charged_count = self.charged_events.entry(name.to_owned()).or_default();
        *charged_count = charged_count
            .checked_add(count as u64)
            .ok_or_else(|| anyhow!("Charged event count overflowed for {name}"))?;
        Ok(())
    }
}
