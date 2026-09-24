use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ChargeBudget {
    pub is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    charged_counts: HashMap<String, f64>,
}

impl ChargeBudget {
    pub fn from_run(run: &Value) -> Result<Self> {
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
                charged_counts: HashMap::new(),
            });
        }

        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (event_name, event) in events {
            let price = event
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    anyhow!("Apify run did not provide the price for event {event_name}")
                })?;
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned an invalid price for event {event_name}");
            }
            event_prices.insert(event_name.clone(), price);
        }

        let raw_limit = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64);
        let max_total_charge_usd = match raw_limit {
            Some(limit) if limit != 0.0 => limit,
            _ => f64::INFINITY,
        };
        if max_total_charge_usd.is_finite() && max_total_charge_usd < 0.0 {
            bail!("Apify run returned an invalid spending limit");
        }

        let mut charged_counts = HashMap::new();
        if let Some(counts) = data.get("chargedEventCounts").and_then(Value::as_object) {
            for (event_name, count) in counts {
                let count = count
                    .as_f64()
                    .filter(|count| count.is_finite() && *count >= 0.0)
                    .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
                charged_counts.insert(event_name.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_counts,
        })
    }

    fn charged_total(&self) -> f64 {
        let total = self
            .charged_counts
            .iter()
            .map(|(event_name, count)| {
                self.event_prices.get(event_name).copied().unwrap_or(0.0) * count
            })
            .sum::<f64>();
        (total * 1_000_000.0).round() / 1_000_000.0
    }

    fn event_price_for_limit(&self, event_name: &str) -> f64 {
        self.event_prices.get(event_name).copied().unwrap_or(0.0)
    }

    fn max_count_for_price(&self, price: f64) -> usize {
        if price == 0.0 {
            return usize::MAX;
        }
        let remaining = (self.max_total_charge_usd - self.charged_total()) / price;
        if !remaining.is_finite() {
            return usize::MAX;
        }
        let rounded = format!("{remaining:.4}")
            .parse::<f64>()
            .unwrap_or(remaining);
        rounded.floor().max(0.0).min(usize::MAX as f64) as usize
    }

    pub fn chargeable_event_capacity(&self, event_name: &str) -> usize {
        if !self.is_pay_per_event {
            return usize::MAX;
        }
        self.max_count_for_price(self.event_price_for_limit(event_name))
    }

    fn push_capacity(&self, event_name: &str) -> usize {
        if !self.is_pay_per_event {
            return usize::MAX;
        }
        let item_price = self.event_price_for_limit(event_name)
            + self.event_price_for_limit("apify-default-dataset-item");
        self.max_count_for_price(item_price)
    }

    pub(crate) fn item_count_to_push(&self, requested: usize, event_name: &str) -> usize {
        if requested == 0 {
            return 0;
        }
        let max_count = self.push_capacity(event_name);
        if max_count >= requested {
            return requested;
        }
        if max_count == 0 && self.charged_total() <= self.max_total_charge_usd {
            return 1;
        }
        max_count
    }

    pub(crate) fn charged_count(&self, requested: usize, event_name: &str) -> usize {
        let max_count = self.chargeable_event_capacity(event_name);
        if requested <= max_count {
            return requested;
        }
        if self.charged_total() <= self.max_total_charge_usd {
            return max_count.saturating_add(1);
        }
        0
    }

    pub(crate) fn apply_charges(&mut self, event_name: &str, count: usize) {
        *self
            .charged_counts
            .entry(event_name.to_owned())
            .or_default() += count as f64;
        if self.event_prices.contains_key("apify-default-dataset-item") {
            *self
                .charged_counts
                .entry("apify-default-dataset-item".to_owned())
                .or_default() += count as f64;
        }
    }

    pub(crate) fn event_charge_limit_reached(&self, event_name: &str) -> bool {
        self.chargeable_event_capacity(event_name) == 0
            || (self.event_prices.contains_key("apify-default-dataset-item")
                && self.chargeable_event_capacity("apify-default-dataset-item") == 0)
    }

    pub(crate) fn is_configured_event(&self, event_name: &str) -> bool {
        self.event_prices.contains_key(event_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn respects_custom_event_budget_and_default_dataset_event_costs() {
        let mut run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "challenge-result": { "eventPriceUsd": 0.00025 },
                        "apify-actor-start": { "eventPriceUsd": 0.0001 }
                    }}
                },
                "options": { "maxTotalChargeUsd": 0.0012 },
                "chargedEventCounts": { "apify-actor-start": 1 }
            }
        });
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            ["apify-default-dataset-item"] = json!({ "eventPriceUsd": 0.0001 });

        let mut budget = ChargeBudget::from_run(&run).unwrap();
        assert_eq!(budget.chargeable_event_capacity("challenge-result"), 4);
        assert_eq!(budget.item_count_to_push(5, "challenge-result"), 3);
        budget.apply_charges("challenge-result", 3);
        assert!(budget.event_charge_limit_reached("challenge-result"));
        assert!(budget.is_pay_per_event);
    }
}
