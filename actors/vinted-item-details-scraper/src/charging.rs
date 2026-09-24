use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub const VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT: &str = "item-detail-result";
pub const DEFAULT_DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";

#[derive(Debug, Clone)]
pub struct ChargeBudget {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    configured_events: HashSet<String>,
    charged_event_counts: HashMap<String, u64>,
}

impl ChargeBudget {
    pub fn from_run(run: &Value) -> Result<Self> {
        let data = run.get("data").context("Apify run pricing is missing")?;
        let is_pay_per_event = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");

        if !is_pay_per_event {
            return Ok(Self {
                is_pay_per_event,
                max_total_charge_usd: f64::INFINITY,
                event_prices: HashMap::new(),
                configured_events: HashSet::new(),
                charged_event_counts: HashMap::new(),
            });
        }

        let pricing_events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        let mut configured_events = HashSet::new();
        for (event_name, event) in pricing_events {
            configured_events.insert(event_name.clone());
            if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
                if !price.is_finite() || price < 0.0 {
                    bail!("Invalid price for charged event {event_name}");
                }
                event_prices.insert(event_name.clone(), price);
            } else {
                event_prices.insert(event_name.clone(), 0.0);
            }
        }

        let configured_limit = match data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
        {
            Some(value) if !value.is_finite() || value < 0.0 => {
                bail!("Apify run returned an invalid spending limit");
            }
            Some(value) if value > 0.0 => value,
            _ => f64::INFINITY,
        };

        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_event_counts = HashMap::new();
        for (event_name, count) in counts {
            let count = count
                .as_u64()
                .or_else(|| {
                    count
                        .as_i64()
                        .filter(|count| *count >= 0)
                        .map(|count| count as u64)
                })
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            charged_event_counts.insert(event_name.clone(), count);
        }

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd: configured_limit,
            event_prices,
            configured_events,
            charged_event_counts,
        })
    }

    pub fn is_pay_per_event(&self) -> bool {
        self.is_pay_per_event
    }

    pub fn has_remaining_event_capacity(&self, event_name: &str) -> bool {
        !self.is_pay_per_event || self.max_event_charge_count(event_name) > 0
    }

    pub fn should_push_success_item(&self) -> bool {
        self.should_push_item(Some(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT))
    }

    pub fn should_push_error_item(&self) -> bool {
        self.should_push_item(None)
    }

    pub fn is_event_configured(&self, event_name: &str) -> bool {
        self.configured_events.contains(event_name)
    }

    pub fn record_charge(&mut self, event_name: &str, requested: u64) -> u64 {
        if !self.is_pay_per_event || requested == 0 {
            return 0;
        }

        let max_charge_count = self.max_event_charge_count(event_name);
        let charged_count = if requested <= max_charge_count {
            requested
        } else if self.total_charged_amount() <= self.max_total_charge_usd {
            max_charge_count.saturating_add(1)
        } else {
            0
        };
        if charged_count > 0 {
            *self
                .charged_event_counts
                .entry(event_name.to_owned())
                .or_default() += charged_count;
        }
        charged_count
    }

    pub fn event_charge_limit_reached(&self, event_name: &str) -> bool {
        self.is_pay_per_event && self.max_event_charge_count(event_name) == 0
    }

    fn should_push_item(&self, event_name: Option<&str>) -> bool {
        if !self.is_pay_per_event {
            return true;
        }

        let item_price = event_name
            .and_then(|event_name| self.event_prices.get(event_name))
            .copied()
            .unwrap_or_default()
            + self
                .event_prices
                .get(DEFAULT_DATASET_ITEM_CHARGE_EVENT)
                .copied()
                .unwrap_or_default();
        if item_price <= 0.0 {
            return true;
        }

        let max_items = self.max_charge_count_for_price(item_price);
        if max_items > 0 {
            return true;
        }
        // Match the SDK behavior: let one last row through when the budget is at the limit so
        // the platform can record the final event and terminate the run cleanly.
        self.total_charged_amount() <= self.max_total_charge_usd
    }

    fn max_event_charge_count(&self, event_name: &str) -> u64 {
        let price = self
            .event_prices
            .get(event_name)
            .copied()
            .unwrap_or_default();
        if price <= 0.0 {
            return u64::MAX;
        }
        self.max_charge_count_for_price(price)
    }

    fn max_charge_count_for_price(&self, price: f64) -> u64 {
        if price <= 0.0 || self.max_total_charge_usd.is_infinite() {
            return u64::MAX;
        }
        let raw_count = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !raw_count.is_finite() {
            return u64::MAX;
        }
        let rounded_count = (raw_count * 10_000.0).round() / 10_000.0;
        rounded_count.floor().max(0.0) as u64
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_event_counts
            .iter()
            .map(|(event_name, count)| {
                self.event_prices
                    .get(event_name)
                    .copied()
                    .unwrap_or_default()
                    * *count as f64
            })
            .sum::<f64>();
        (total * 1_000_000.0).round() / 1_000_000.0
    }
}

pub fn charge_limit_status(saved_results: usize, request_index: usize) -> String {
    format!(
        "Charge limit reached before fetching Vinted item detail request {}; {} item detail result(s) were saved.",
        request_index + 1,
        saved_results
    )
}

pub fn successful_charge_status(saved: bool, request_index: usize) -> String {
    let timing = if saved { "after" } else { "before" };
    format!(
        "Charge limit reached {timing} saving Vinted item detail result {}.",
        request_index + 1
    )
}

#[cfg(test)]
mod tests {
    use super::{
        charge_limit_status, successful_charge_status, ChargeBudget,
        DEFAULT_DATASET_ITEM_CHARGE_EVENT, VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT,
    };
    use serde_json::{json, Value};

    fn priced_run(max_charge: f64, charged_counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "item-detail-result": {"eventPriceUsd": 0.00025},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                        "other-event": {"eventPriceUsd": 0.0005}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": charged_counts
            }
        })
    }

    #[test]
    fn detects_ppe_and_gates_on_the_named_event_budget() {
        let mut budget = ChargeBudget::from_run(&priced_run(0.0005, json!({}))).unwrap();
        assert!(budget.is_pay_per_event());
        assert!(budget.has_remaining_event_capacity(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT));
        assert!(budget.has_remaining_event_capacity("other-event"));

        budget.record_charge("other-event", 1);
        assert!(!budget.has_remaining_event_capacity(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT));
        assert_eq!(
            charge_limit_status(4, 5),
            "Charge limit reached before fetching Vinted item detail request 6; 4 item detail result(s) were saved."
        );
    }

    #[test]
    fn accounts_for_default_dataset_cost_and_sdk_final_row_behavior() {
        let budget = ChargeBudget::from_run(&priced_run(0.00035, json!({}))).unwrap();
        assert!(budget.should_push_success_item());
        assert!(budget.should_push_error_item());

        let mut full_budget = ChargeBudget::from_run(&priced_run(0.00035, json!({}))).unwrap();
        full_budget.record_charge(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT, 1);
        full_budget.record_charge(DEFAULT_DATASET_ITEM_CHARGE_EVENT, 1);
        assert!(full_budget.should_push_success_item());
        assert!(full_budget.event_charge_limit_reached(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT));
        full_budget.record_charge(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT, 1);
        assert!(!full_budget.should_push_success_item());
        assert!(!full_budget.has_remaining_event_capacity(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT));
    }

    #[test]
    fn records_charges_and_reports_limit_after_the_last_result() {
        let mut budget = ChargeBudget::from_run(&priced_run(0.00035, json!({}))).unwrap();
        assert_eq!(
            budget.record_charge(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT, 1),
            1
        );
        assert_eq!(
            budget.record_charge(DEFAULT_DATASET_ITEM_CHARGE_EVENT, 1),
            1
        );
        assert!(budget.event_charge_limit_reached(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT));
        assert_eq!(
            successful_charge_status(true, 0),
            "Charge limit reached after saving Vinted item detail result 1."
        );
        assert_eq!(
            successful_charge_status(false, 0),
            "Charge limit reached before saving Vinted item detail result 1."
        );
    }

    #[test]
    fn uses_unlimited_budget_when_run_has_no_nonzero_limit() {
        let mut run = priced_run(0.0, json!({}));
        run["data"]["options"]
            .as_object_mut()
            .unwrap()
            .remove("maxTotalChargeUsd");
        let mut budget = ChargeBudget::from_run(&run).unwrap();
        for _ in 0..100 {
            assert!(budget.has_remaining_event_capacity(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT));
            assert!(budget.should_push_success_item());
            budget.record_charge(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT, 1);
            budget.record_charge(DEFAULT_DATASET_ITEM_CHARGE_EVENT, 1);
        }
    }

    #[test]
    fn non_ppe_run_does_not_require_event_pricing() {
        let run = json!({"data": {"pricingInfo": {"pricingModel": "FREE"}}});
        let budget = ChargeBudget::from_run(&run).unwrap();
        assert!(!budget.is_pay_per_event());
        assert!(budget.has_remaining_event_capacity(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT));
        assert!(budget.should_push_success_item());
    }
}
