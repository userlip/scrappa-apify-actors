use anyhow::{Context, Result, anyhow};
use serde_json::Value;
use std::collections::HashMap;

pub const BUSINESS_RESULT_CHARGE_EVENT: &str = "business-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

#[derive(Debug)]
pub struct PpeBudget {
    pub is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    event_counts: HashMap<String, u64>,
}

#[derive(Debug, PartialEq)]
pub struct PushResult {
    pub saved_count: usize,
    pub status_message: Option<String>,
}

impl PpeBudget {
    pub fn from_run(run: &Value) -> Result<Self> {
        let data = run.get("data").unwrap_or(run);
        let pricing_info = data.get("pricingInfo");
        let is_pay_per_event = pricing_info
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        let Some(pricing_info) = pricing_info.filter(|_| is_pay_per_event) else {
            return Ok(Self {
                is_pay_per_event: false,
                max_total_charge_usd: f64::INFINITY,
                event_prices: HashMap::new(),
                event_counts: HashMap::new(),
            });
        };

        let events = pricing_info
            .pointer("/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .context("Apify run did not provide pay-per-event prices")?;
        let mut event_prices = HashMap::new();
        for (event, definition) in events {
            let price = definition
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if !price.is_finite() || price < 0.0 {
                return Err(anyhow!(
                    "Apify run returned an invalid price for event {event}"
                ));
            }
            event_prices.insert(event.clone(), price);
        }

        let configured_max = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let max_total_charge_usd = if configured_max == 0.0 {
            f64::INFINITY
        } else if configured_max.is_finite() && configured_max > 0.0 {
            configured_max
        } else {
            return Err(anyhow!("Apify run returned an invalid spending limit"));
        };
        let mut event_counts = HashMap::new();
        if let Some(counts) = data.get("chargedEventCounts").and_then(Value::as_object) {
            for (event, count) in counts {
                let count = count
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {event}"))?;
                event_counts.insert(event.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            event_counts,
        })
    }

    pub fn dataset_item_limit(&self, requested: usize) -> usize {
        if !self.is_pay_per_event || requested == 0 {
            return requested;
        }
        let item_price = self
            .event_prices
            .get(BUSINESS_RESULT_CHARGE_EVENT)
            .copied()
            .unwrap_or(0.0)
            + self
                .event_prices
                .get(DEFAULT_DATASET_ITEM_EVENT)
                .copied()
                .unwrap_or(0.0);
        let max_count = if item_price == 0.0 {
            usize::MAX
        } else {
            self.max_charges_by_price(item_price)
        };
        if max_count >= requested {
            requested
        } else if max_count == 0 && self.total_charged_amount() <= self.max_total_charge_usd {
            1
        } else {
            max_count.min(requested)
        }
    }

    pub fn finish_dataset_push(
        &mut self,
        saved_items: usize,
        requested_items: usize,
    ) -> PushResult {
        if !self.is_pay_per_event {
            return PushResult {
                saved_count: saved_items,
                status_message: None,
            };
        }
        if saved_items == 0 {
            return self.limit_reached_result(0, requested_items, true);
        }

        let count = saved_items as u64;
        *self
            .event_counts
            .entry(BUSINESS_RESULT_CHARGE_EVENT.into())
            .or_default() += count;
        *self
            .event_counts
            .entry(DEFAULT_DATASET_ITEM_EVENT.into())
            .or_default() += count;

        let limit_reached = [BUSINESS_RESULT_CHARGE_EVENT, DEFAULT_DATASET_ITEM_EVENT]
            .into_iter()
            .any(|event| {
                self.max_charges_by_price(self.event_prices.get(event).copied().unwrap_or(0.0)) == 0
            });
        self.limit_reached_result(saved_items, requested_items, limit_reached)
    }

    pub fn should_charge_business_result(&self) -> bool {
        self.event_prices.contains_key(BUSINESS_RESULT_CHARGE_EVENT)
    }

    fn limit_reached_result(
        &self,
        saved_items: usize,
        requested_items: usize,
        limit_reached: bool,
    ) -> PushResult {
        if !limit_reached {
            return PushResult {
                saved_count: requested_items,
                status_message: None,
            };
        }
        let saved_count = saved_items.min(requested_items);
        PushResult {
            saved_count,
            status_message: Some(format!(
                "Charge limit reached after saving {saved_count} of {requested_items} Trustpilot business results on the current page."
            )),
        }
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self.event_counts.iter().fold(0.0, |total, (event, count)| {
            total + self.event_prices.get(event).copied().unwrap_or(0.0) * *count as f64
        });
        (total * 1_000_000.0).round() / 1_000_000.0
    }

    fn max_charges_by_price(&self, price: f64) -> usize {
        if price == 0.0 {
            return usize::MAX;
        }
        let available = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !available.is_finite() {
            return if available.is_sign_positive() {
                usize::MAX
            } else {
                0
            };
        }
        let rounded = format!("{available:.4}").parse::<f64>().unwrap_or(0.0);
        rounded.floor().max(0.0).min(usize::MAX as f64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(max_total: Value, charged: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{
                    "business-result":{"eventPriceUsd":0.1},
                    "apify-default-dataset-item":{"eventPriceUsd":0.05}
                }}},
                "options":{"maxTotalChargeUsd":max_total},
                "chargedEventCounts":charged
            }
        })
    }

    #[test]
    fn caps_dataset_rows_by_the_combined_event_price() {
        let mut budget = PpeBudget::from_run(&run(json!(0.3), json!({}))).unwrap();
        let rows = budget.dataset_item_limit(4);
        assert_eq!(rows, 2);
        assert_eq!(budget.finish_dataset_push(rows, 4), PushResult {
            saved_count: 2,
            status_message: Some("Charge limit reached after saving 2 of 4 Trustpilot business results on the current page.".into())
        });
    }

    #[test]
    fn leaves_one_result_for_the_sdk_limit_signal_when_the_budget_is_exactly_spent() {
        let mut budget = PpeBudget::from_run(&run(json!(0.15), json!({}))).unwrap();
        assert_eq!(budget.dataset_item_limit(2), 1);
        let result = budget.finish_dataset_push(1, 2);
        assert_eq!(result.saved_count, 1);
        assert!(result.status_message.unwrap().contains("saving 1 of 2"));
    }

    #[test]
    fn allows_all_results_outside_pay_per_event_pricing() {
        let mut budget =
            PpeBudget::from_run(&json!({"data":{"pricingInfo":{"pricingModel":"PAY_PER_USAGE"}}}))
                .unwrap();
        assert!(!budget.is_pay_per_event);
        assert_eq!(budget.dataset_item_limit(5), 5);
        assert_eq!(
            budget.finish_dataset_push(4, 5),
            PushResult {
                saved_count: 4,
                status_message: None
            }
        );
    }

    #[test]
    fn accounts_for_prior_events_and_treats_zero_limit_as_unlimited() {
        let budget = PpeBudget::from_run(&run(
            json!(0.25),
            json!({"business-result":1,"apify-default-dataset-item":1}),
        ))
        .unwrap();
        assert_eq!(budget.dataset_item_limit(5), 1);
        let unlimited = PpeBudget::from_run(&run(json!(0), json!({}))).unwrap();
        assert_eq!(unlimited.dataset_item_limit(5), 5);
    }
}
