use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use serde_json::Value;

pub(crate) const JOB_RESULT_CHARGE_EVENT: &str = "job-result";
pub(crate) const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

#[derive(Debug)]
pub(crate) struct ChargeRecord {
    pub(crate) event_name: String,
    pub(crate) charged_count: usize,
    pub(crate) should_call_api: bool,
}

pub(crate) struct PricingState {
    pub(crate) is_pay_per_event: bool,
    pub(crate) max_total_charge_usd: f64,
    pub(crate) event_prices: HashMap<String, f64>,
    pub(crate) charged_event_counts: HashMap<String, usize>,
}

impl PricingState {
    pub(crate) fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let is_pay_per_event = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self {
                is_pay_per_event: false,
                max_total_charge_usd: f64::INFINITY,
                event_prices: HashMap::new(),
                charged_event_counts: HashMap::new(),
            });
        }

        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (event_name, event) in events {
            let price = event_price_for_budget(event_name, event)?;
            event_prices.insert(event_name.clone(), price);
        }

        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_event_counts = HashMap::new();
        for (event_name, count) in counts {
            let count = count
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            charged_event_counts.insert(event_name.clone(), count);
        }

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .unwrap_or(f64::INFINITY);
        if max_total_charge_usd.is_nan() || max_total_charge_usd < 0.0 {
            bail!("Apify run returned an invalid spending limit");
        }
        // Apify's SDK resolves its default zero spending limit to no limit.
        let max_total_charge_usd = if max_total_charge_usd == 0.0 {
            f64::INFINITY
        } else {
            max_total_charge_usd
        };

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        })
    }

    pub(crate) fn event_price(&self, event_name: &str) -> f64 {
        self.event_prices.get(event_name).copied().unwrap_or(0.0)
    }

    pub(crate) fn total_charged_amount(&self) -> f64 {
        let amount = self
            .charged_event_counts
            .iter()
            .map(|(event_name, count)| self.event_price(event_name) * *count as f64)
            .sum::<f64>();
        if amount.is_finite() {
            (amount * 1_000_000.0).round() / 1_000_000.0
        } else {
            amount
        }
    }

    pub(crate) fn max_charges_for_price(&self, price: f64) -> Option<usize> {
        if price <= 0.0 {
            return None;
        }
        let remaining = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !remaining.is_finite() {
            return None;
        }
        let rounded = (remaining * 10_000.0).round() / 10_000.0;
        Some(rounded.floor().max(0.0).min(usize::MAX as f64) as usize)
    }

    pub(crate) fn max_event_charges_within_limit(&self, event_name: &str) -> Option<usize> {
        self.max_charges_for_price(self.event_price(event_name))
    }

    pub(crate) fn item_limit(&self, event_name: Option<&str>) -> usize {
        if !self.is_pay_per_event {
            return usize::MAX;
        }

        let explicit_event_price = event_name.map_or(0.0, |name| self.event_price(name));
        let default_item_price = self.event_price(DEFAULT_DATASET_ITEM_EVENT);
        self.max_charges_for_price(explicit_event_price + default_item_price)
            .unwrap_or(usize::MAX)
    }

    pub(crate) fn should_push_item(&self, event_name: Option<&str>) -> bool {
        let max_charges = self.item_limit(event_name);
        max_charges >= 1
            || (max_charges == 0 && self.total_charged_amount() <= self.max_total_charge_usd)
    }

    pub(crate) fn register_charge(&mut self, event_name: &str, count: usize) -> ChargeRecord {
        let max_charges = self.max_event_charges_within_limit(event_name);
        let total_before_charge = self.total_charged_amount();
        let charged_count = match max_charges {
            None => count,
            Some(max_charges) if count <= max_charges => count,
            Some(max_charges) if total_before_charge <= self.max_total_charge_usd => {
                max_charges.saturating_add(1)
            }
            Some(_) => 0,
        };

        if charged_count > 0 {
            let event_count = self
                .charged_event_counts
                .entry(event_name.to_owned())
                .or_default();
            *event_count = event_count.saturating_add(charged_count);
        }

        ChargeRecord {
            event_name: event_name.to_owned(),
            charged_count,
            should_call_api: charged_count > 0
                && !event_name.starts_with("apify-")
                && self.event_prices.contains_key(event_name),
        }
    }

    pub(crate) fn event_charge_limit_reached(&self, event_name: &str) -> bool {
        self.max_event_charges_within_limit(event_name)
            .is_some_and(|remaining| remaining == 0)
    }
}

fn event_price_for_budget(event_name: &str, event: &Value) -> Result<f64> {
    if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
        if !price.is_finite() || price < 0.0 {
            bail!("Apify run returned an invalid price for event {event_name}");
        }
        return Ok(price);
    }

    let tier_prices = event
        .get("eventTieredPricingUsd")
        .and_then(Value::as_object)
        .filter(|prices| !prices.is_empty())
        .ok_or_else(|| {
            anyhow!("Apify run did not provide a usable price for event {event_name}")
        })?;

    // The run response includes the tier schedule but not the user's active tier. Use the
    // schedule's highest rate as a safe budget estimate; the charge API applies the real tier.
    tier_prices
        .iter()
        .map(|(tier, entry)| {
            let price = entry
                .get("tieredEventPriceUsd")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    anyhow!(
                        "Apify run did not provide a price for {tier} tier of event {event_name}"
                    )
                })?;
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned an invalid price for {tier} tier of event {event_name}");
            }
            Ok(price)
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .reduce(f64::max)
        .ok_or_else(|| anyhow!("Apify run did not provide a usable price for event {event_name}"))
}
