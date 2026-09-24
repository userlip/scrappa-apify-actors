use std::collections::HashMap;

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::apify::ApifyClient;

pub const BOOKING_HOTEL_RESULT_CHARGE_EVENT: &str = "hotel-result";
pub const DEFAULT_DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";

#[derive(Debug, Clone)]
pub struct ChargingManager {
    is_pay_per_event: bool,
    is_at_home: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    event_counts: HashMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChargeResult {
    pub charged_count: u64,
    pub event_charge_limit_reached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushPlan {
    pub items_to_keep: usize,
    pub events_to_charge: Vec<(String, usize)>,
}

impl ChargingManager {
    pub fn from_run(run: &Value, is_at_home: bool) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_info = data.get("pricingInfo").unwrap_or(&Value::Null);
        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|value| *value != 0.0)
            .unwrap_or(f64::INFINITY);
        Self::from_values(
            pricing_info,
            &Value::Object(charged_event_counts),
            max_total_charge_usd,
            is_at_home,
        )
    }

    pub fn from_values(
        pricing_info: &Value,
        charged_event_counts: &Value,
        max_total_charge_usd: f64,
        is_at_home: bool,
    ) -> Result<Self> {
        let is_pay_per_event =
            pricing_info.get("pricingModel").and_then(Value::as_str) == Some("PAY_PER_EVENT");
        let mut event_prices = HashMap::new();
        if let Some(events) = pricing_info
            .pointer("/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
        {
            for (event_name, event) in events {
                if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
                    if !price.is_finite() || price < 0.0 {
                        return Err(anyhow!("Apify run returned invalid price for {event_name}"));
                    }
                    event_prices.insert(event_name.clone(), price);
                }
            }
        }

        let mut event_counts = HashMap::new();
        if let Some(counts) = charged_event_counts.as_object() {
            for (event_name, count) in counts {
                let count = count
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
                event_counts.insert(event_name.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event,
            is_at_home,
            max_total_charge_usd: if max_total_charge_usd == 0.0 {
                f64::INFINITY
            } else {
                max_total_charge_usd
            },
            event_prices,
            event_counts,
        })
    }

    pub fn is_pay_per_event(&self) -> bool {
        self.is_pay_per_event
    }

    pub fn hotel_charge_limit_status(
        &self,
        saved_results: usize,
        request_index: usize,
    ) -> Option<String> {
        if !self.is_pay_per_event
            || self
                .max_event_charge_count_within_limit(BOOKING_HOTEL_RESULT_CHARGE_EVENT)
                .is_none_or(|count| count > 0)
        {
            return None;
        }
        Some(format!(
            "Charge limit reached before fetching Booking.com hotel detail request {}; {saved_results} hotel detail result(s) were saved.",
            request_index + 1
        ))
    }

    pub fn plan_dataset_push(
        &self,
        item_count: usize,
        event_name: Option<&str>,
        is_default_dataset: bool,
    ) -> PushPlan {
        if !self.is_pay_per_event {
            return PushPlan {
                items_to_keep: item_count,
                events_to_charge: Vec::new(),
            };
        }

        let event_price = event_name
            .map(|name| self.event_price_for_limit(name))
            .unwrap_or(0.0);
        let dataset_price = if is_default_dataset {
            self.event_price_for_limit(DEFAULT_DATASET_ITEM_CHARGE_EVENT)
        } else {
            0.0
        };
        let item_price = event_price + dataset_price;
        let max_charged_count = if item_price > 0.0 {
            self.max_charges_by_price(item_price)
        } else {
            None
        };
        let items_to_keep = match max_charged_count {
            None => item_count,
            Some(max_count) if max_count >= item_count => item_count,
            Some(0)
                if item_count > 0 && self.total_charged_amount() <= self.max_total_charge_usd =>
            {
                1
            }
            Some(max_count) => max_count.min(item_count),
        };

        let mut events_to_charge = Vec::new();
        if items_to_keep > 0 {
            if let Some(event_name) = event_name {
                events_to_charge.push((event_name.to_owned(), items_to_keep));
            }
            if is_default_dataset {
                events_to_charge
                    .push((DEFAULT_DATASET_ITEM_CHARGE_EVENT.to_owned(), items_to_keep));
            }
        }
        PushPlan {
            items_to_keep,
            events_to_charge,
        }
    }

    pub async fn charge(
        &mut self,
        apify: &ApifyClient,
        event_name: &str,
        count: usize,
        idempotency_key: &str,
    ) -> Result<ChargeResult> {
        if !self.is_pay_per_event {
            return Ok(ChargeResult {
                charged_count: 0,
                event_charge_limit_reached: false,
            });
        }

        let max_event_count = self
            .max_event_charge_count_within_limit(event_name)
            .unwrap_or(usize::MAX);
        let charged_count = if count <= max_event_count {
            count
        } else if self.total_charged_amount() <= self.max_total_charge_usd {
            max_event_count.saturating_add(1)
        } else {
            0
        };

        if charged_count == 0 {
            return Ok(ChargeResult {
                charged_count: 0,
                event_charge_limit_reached: count > 0,
            });
        }

        let current_count = self.event_counts.entry(event_name.to_owned()).or_default();
        *current_count = current_count.saturating_add(charged_count as u64);

        if self.is_at_home
            && !event_name.starts_with("apify-")
            && self.event_prices.contains_key(event_name)
        {
            apify
                .charge_event(event_name, charged_count, idempotency_key)
                .await?;
        }

        Ok(ChargeResult {
            charged_count: charged_count as u64,
            event_charge_limit_reached: self
                .max_event_charge_count_within_limit(event_name)
                .is_some_and(|count| count == 0),
        })
    }

    fn event_price_for_limit(&self, event_name: &str) -> f64 {
        if self.is_at_home {
            self.event_prices.get(event_name).copied().unwrap_or(0.0)
        } else {
            1.0
        }
    }

    fn max_event_charge_count_within_limit(&self, event_name: &str) -> Option<usize> {
        let price = if self.is_at_home {
            self.event_prices.get(event_name).copied()
        } else {
            Some(1.0)
        }?;
        self.max_charges_by_price(price)
    }

    fn max_charges_by_price(&self, price: f64) -> Option<usize> {
        if price == 0.0 {
            return None;
        }
        let remaining = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !remaining.is_finite() {
            return None;
        }
        let rounded = (remaining * 10_000.0).round() / 10_000.0;
        Some(rounded.floor().max(0.0) as usize)
    }

    fn total_charged_amount(&self) -> f64 {
        self.event_counts
            .iter()
            .map(|(event_name, count)| {
                *count as f64 * self.event_prices.get(event_name).copied().unwrap_or(0.0)
            })
            .sum()
    }
}

pub fn merge_charge_results(left: ChargeResult, right: ChargeResult) -> ChargeResult {
    ChargeResult {
        charged_count: left.charged_count.saturating_add(right.charged_count),
        event_charge_limit_reached: left.event_charge_limit_reached
            || right.event_charge_limit_reached,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manager(max_total: f64, counts: Value) -> ChargingManager {
        ChargingManager::from_values(
            &json!({
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "hotel-result": {"eventPriceUsd": 0.1},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.02},
                    "other-event": {"eventPriceUsd": 0.03}
                }}
            }),
            &counts,
            max_total,
            true,
        )
        .unwrap()
    }

    #[test]
    fn checks_custom_event_capacity_before_fetch() {
        let available = manager(0.32, json!({"hotel-result": 1, "other-event": 2}));
        assert_eq!(available.hotel_charge_limit_status(1, 2), None);
        let depleted = manager(0.25, json!({"hotel-result": 1, "other-event": 2}));
        assert_eq!(
            depleted.hotel_charge_limit_status(4, 5).as_deref(),
            Some("Charge limit reached before fetching Booking.com hotel detail request 6; 4 hotel detail result(s) were saved.")
        );
        let free = ChargingManager::from_values(
            &json!({"pricingModel": "PAY_PER_EVENT"}),
            &json!({}),
            0.0,
            true,
        )
        .unwrap();
        assert_eq!(free.hotel_charge_limit_status(0, 0), None);
    }

    #[test]
    fn combines_custom_and_default_dataset_prices_when_limiting_a_write() {
        let billing = manager(0.05, json!({"other-event": 2}));
        let plan = billing.plan_dataset_push(1, Some(BOOKING_HOTEL_RESULT_CHARGE_EVENT), true);
        assert_eq!(plan.items_to_keep, 0);
        assert!(plan.events_to_charge.is_empty());

        let at_limit = manager(0.14, json!({"other-event": 2}));
        let plan = at_limit.plan_dataset_push(1, Some(BOOKING_HOTEL_RESULT_CHARGE_EVENT), true);
        assert_eq!(plan.items_to_keep, 1);
        assert_eq!(
            plan.events_to_charge,
            vec![
                ("hotel-result".to_owned(), 1),
                ("apify-default-dataset-item".to_owned(), 1)
            ]
        );
    }

    #[test]
    fn retains_one_item_at_the_budget_edge_to_match_sdk_overcharge_handling() {
        let billing = manager(0.10, json!({}));
        let plan = billing.plan_dataset_push(1, Some(BOOKING_HOTEL_RESULT_CHARGE_EVENT), true);
        assert_eq!(plan.items_to_keep, 1);
    }

    #[test]
    fn push_data_result_combines_custom_and_default_dataset_charges() {
        let result = merge_charge_results(
            ChargeResult {
                charged_count: 1,
                event_charge_limit_reached: false,
            },
            ChargeResult {
                charged_count: 1,
                event_charge_limit_reached: true,
            },
        );
        assert_eq!(
            result,
            ChargeResult {
                charged_count: 2,
                event_charge_limit_reached: true,
            }
        );
    }

    #[test]
    fn non_pay_per_event_runs_keep_unpriced_dataset_rows() {
        let billing =
            ChargingManager::from_values(&json!({"pricingModel": "FREE"}), &json!({}), 0.0, true)
                .unwrap();
        assert!(!billing.is_pay_per_event());
        let plan = billing.plan_dataset_push(1, Some(BOOKING_HOTEL_RESULT_CHARGE_EVENT), true);
        assert_eq!(plan.items_to_keep, 1);
        assert!(plan.events_to_charge.is_empty());
    }
}
