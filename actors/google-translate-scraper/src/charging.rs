use std::collections::BTreeMap;

use serde_json::Value;

pub const TRANSLATION_RESULT_CHARGE_EVENT: &str = "translation-result";
pub const DEFAULT_DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushPlan {
    pub keep_item: bool,
    pub events_to_charge: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ChargingManager {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: BTreeMap<String, f64>,
    charged_event_counts: BTreeMap<String, u64>,
}

impl ChargingManager {
    pub fn from_run(run: &Value) -> Result<Self, String> {
        let data = run.get("data").unwrap_or(run);
        let pricing_info = data.get("pricingInfo");
        let is_pay_per_event = pricing_info
            .and_then(|info| info.get("pricingModel"))
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");

        let mut event_prices = BTreeMap::new();
        if is_pay_per_event {
            let events = pricing_info
                .and_then(|info| info.pointer("/pricingPerEvent/actorChargeEvents"))
                .and_then(Value::as_object)
                .ok_or_else(|| "Apify run did not provide event prices".to_owned())?;
            for (event_name, event) in events {
                let price = event
                    .get("eventPriceUsd")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| {
                        format!("Apify run did not provide the price for {event_name}")
                    })?;
                if !price.is_finite() || price < 0.0 {
                    return Err(format!(
                        "Apify run returned an invalid price for {event_name}"
                    ));
                }
                event_prices.insert(event_name.clone(), price);
            }
        }

        let max_total_charge_usd = if is_pay_per_event {
            match data
                .pointer("/options/maxTotalChargeUsd")
                .and_then(Value::as_f64)
            {
                Some(value) if !value.is_finite() || value < 0.0 => {
                    return Err("Apify run returned an invalid spending limit".to_owned());
                }
                Some(0.0) | None => f64::INFINITY,
                Some(value) => value,
            }
        } else {
            f64::INFINITY
        };

        let mut charged_event_counts = BTreeMap::new();
        if let Some(counts) = data.get("chargedEventCounts").and_then(Value::as_object) {
            for (event_name, count) in counts {
                let count = count
                    .as_u64()
                    .ok_or_else(|| format!("Invalid charged event count for {event_name}"))?;
                charged_event_counts.insert(event_name.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        })
    }

    pub fn is_pay_per_event(&self) -> bool {
        self.is_pay_per_event
    }

    pub fn has_event_price(&self, event_name: &str) -> bool {
        self.event_prices.contains_key(event_name)
    }

    pub fn charge_limit_status(&self, processed: usize, requested: usize) -> Option<String> {
        if !self.is_pay_per_event
            || self.max_event_charge_count(TRANSLATION_RESULT_CHARGE_EVENT) > 0
        {
            return None;
        }

        Some(format!(
            "Charge limit reached before fetching the next translation; {processed} of {requested} translation item(s) were processed."
        ))
    }

    pub fn plan_dataset_push(&self, successful_translation: bool) -> PushPlan {
        if !self.is_pay_per_event {
            return PushPlan {
                keep_item: true,
                events_to_charge: Vec::new(),
            };
        }

        let custom_price = if successful_translation {
            self.event_prices
                .get(TRANSLATION_RESULT_CHARGE_EVENT)
                .copied()
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let dataset_price = self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_CHARGE_EVENT)
            .copied()
            .unwrap_or(0.0);
        let item_price = custom_price + dataset_price;
        let item_capacity = if item_price > 0.0 {
            self.max_charges_by_price(item_price)
        } else {
            usize::MAX
        };
        let keep_item =
            item_capacity > 0 || self.total_charged_amount() <= self.max_total_charge_usd;

        let mut events_to_charge = Vec::new();
        if keep_item {
            if successful_translation {
                events_to_charge.push(TRANSLATION_RESULT_CHARGE_EVENT.to_owned());
            }
            // Dataset writes charge this synthetic event automatically on Apify. Keep it in
            // local budget state because the SDK includes it in its spending-limit checks.
            events_to_charge.push(DEFAULT_DATASET_ITEM_CHARGE_EVENT.to_owned());
        }

        PushPlan {
            keep_item,
            events_to_charge,
        }
    }

    pub fn record_charge(&mut self, event_name: &str, count: u64) -> Result<(), String> {
        let event_count = self
            .charged_event_counts
            .entry(event_name.to_owned())
            .or_default();
        *event_count = event_count
            .checked_add(count)
            .ok_or_else(|| "Charged event count overflowed".to_owned())?;
        Ok(())
    }

    pub fn event_limit_reached(&self, events: &[String]) -> bool {
        events
            .iter()
            .any(|event_name| self.max_event_charge_count(event_name) == 0)
    }

    fn max_event_charge_count(&self, event_name: &str) -> usize {
        let Some(price) = self.event_prices.get(event_name).copied() else {
            return usize::MAX;
        };
        if price == 0.0 {
            return usize::MAX;
        }
        self.max_charges_by_price(price)
    }

    fn max_charges_by_price(&self, price: f64) -> usize {
        let unrounded = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !unrounded.is_finite() {
            return if unrounded.is_sign_positive() {
                usize::MAX
            } else {
                0
            };
        }
        let rounded = format!("{unrounded:.4}")
            .parse::<f64>()
            .unwrap_or(unrounded);
        rounded.floor().max(0.0).min(usize::MAX as f64) as usize
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_event_counts
            .iter()
            .map(|(event_name, count)| {
                self.event_prices.get(event_name).copied().unwrap_or(0.0) * *count as f64
            })
            .sum::<f64>();
        format!("{total:.6}").parse().unwrap_or(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(pricing_model: &str, max_total: f64, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": pricing_model,
                    "pricingPerEvent": {"actorChargeEvents": {
                        "translation-result": {"eventPriceUsd": 0.00025},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                        "other-event": {"eventPriceUsd": 0.0003}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_total},
                "chargedEventCounts": counts
            }
        })
    }

    #[test]
    fn ignores_charge_limits_for_non_ppe_runs() {
        let manager =
            ChargingManager::from_run(&run("PRICE_PER_RESULT", 0.001, json!({}))).unwrap();

        assert!(!manager.is_pay_per_event());
        assert_eq!(manager.charge_limit_status(2, 4), None);
        assert_eq!(
            manager.plan_dataset_push(true),
            PushPlan {
                keep_item: true,
                events_to_charge: Vec::new(),
            }
        );
    }

    #[test]
    fn checks_the_translation_event_against_all_existing_run_charges() {
        let manager = ChargingManager::from_run(&run(
            "PAY_PER_EVENT",
            0.001,
            json!({
                "translation-result": 2,
                "apify-default-dataset-item": 1,
                "other-event": 1
            }),
        ))
        .unwrap();

        assert_eq!(
            manager.charge_limit_status(2, 4).as_deref(),
            Some(
                "Charge limit reached before fetching the next translation; 2 of 4 translation item(s) were processed."
            )
        );
    }

    #[test]
    fn success_plans_charge_the_result_and_default_dataset_events() {
        let manager = ChargingManager::from_run(&run("PAY_PER_EVENT", 0.001, json!({}))).unwrap();

        assert_eq!(
            manager.plan_dataset_push(true),
            PushPlan {
                keep_item: true,
                events_to_charge: vec![
                    "translation-result".to_owned(),
                    "apify-default-dataset-item".to_owned(),
                ],
            }
        );
        assert_eq!(
            manager.plan_dataset_push(false),
            PushPlan {
                keep_item: true,
                events_to_charge: vec!["apify-default-dataset-item".to_owned()],
            }
        );
    }

    #[test]
    fn trims_items_at_the_combined_event_and_dataset_budget() {
        let mut manager = ChargingManager::from_run(&run(
            "PAY_PER_EVENT",
            0.0006,
            json!({"translation-result": 1}),
        ))
        .unwrap();

        assert_eq!(manager.charge_limit_status(1, 3), None);
        assert!(manager.plan_dataset_push(true).keep_item);
        manager.record_charge("translation-result", 1).unwrap();
        manager
            .record_charge("apify-default-dataset-item", 1)
            .unwrap();
        // Match the SDK's one-item overage when spend is exactly at the configured limit.
        assert!(manager.plan_dataset_push(true).keep_item);
        manager.record_charge("translation-result", 1).unwrap();
        manager
            .record_charge("apify-default-dataset-item", 1)
            .unwrap();
        assert!(!manager.plan_dataset_push(true).keep_item);
        assert!(!manager.plan_dataset_push(false).keep_item);
    }

    #[test]
    fn rejects_invalid_run_price_and_charge_count_data() {
        let mut invalid_price = run("PAY_PER_EVENT", 0.001, json!({}));
        invalid_price["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]["translation-result"]
            ["eventPriceUsd"] = json!(-1);
        assert_eq!(
            ChargingManager::from_run(&invalid_price).unwrap_err(),
            "Apify run returned an invalid price for translation-result"
        );

        let invalid_count = run("PAY_PER_EVENT", 0.001, json!({"translation-result": -1}));
        assert_eq!(
            ChargingManager::from_run(&invalid_count).unwrap_err(),
            "Invalid charged event count for translation-result"
        );
    }
}
