use serde_json::Value;
use std::collections::HashMap;

pub const SHOP_PROFILE_RESULT_CHARGE_EVENT: &str = "shop-profile-result";
pub const DEFAULT_DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";

#[derive(Clone, Debug, PartialEq)]
pub struct ChargeResult {
    pub event_charge_limit_reached: bool,
    pub charged_count: usize,
}

#[derive(Clone, Debug)]
pub struct ChargingManager {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices_usd: HashMap<String, f64>,
    charged_event_counts: HashMap<String, u64>,
}

impl ChargingManager {
    pub fn from_run(run: &Value) -> Self {
        let data = run.get("data").unwrap_or(run);
        Self::from_parts(
            data.get("pricingInfo"),
            data.get("chargedEventCounts"),
            data.pointer("/options/maxTotalChargeUsd")
                .and_then(Value::as_f64),
        )
    }

    pub fn from_environment(
        pricing_info: &Value,
        charged_event_counts: &Value,
        max_total_charge_usd: Option<f64>,
    ) -> Self {
        Self::from_parts(
            Some(pricing_info),
            Some(charged_event_counts),
            max_total_charge_usd,
        )
    }

    pub fn free() -> Self {
        Self {
            is_pay_per_event: false,
            max_total_charge_usd: f64::INFINITY,
            event_prices_usd: HashMap::new(),
            charged_event_counts: HashMap::new(),
        }
    }

    fn from_parts(
        pricing_info: Option<&Value>,
        charged_event_counts: Option<&Value>,
        max_total_charge_usd: Option<f64>,
    ) -> Self {
        let is_pay_per_event = pricing_info
            .and_then(|info| info.get("pricingModel"))
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        let mut event_prices_usd = HashMap::new();
        if let Some(events) = pricing_info
            .and_then(|info| info.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
        {
            for (name, event) in events {
                if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64)
                    && price.is_finite()
                    && price >= 0.0
                {
                    event_prices_usd.insert(name.clone(), price);
                }
            }
        }

        let charged_event_counts = charged_event_counts
            .and_then(Value::as_object)
            .map(|counts| {
                counts
                    .iter()
                    .filter_map(|(name, value)| value.as_u64().map(|count| (name.clone(), count)))
                    .collect()
            })
            .unwrap_or_default();

        let max_total_charge_usd = max_total_charge_usd
            .filter(|value| value.is_finite() && *value > 0.0)
            .unwrap_or(f64::INFINITY);

        Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices_usd,
            charged_event_counts,
        }
    }

    pub fn is_pay_per_event(&self) -> bool {
        self.is_pay_per_event
    }

    pub fn push_limit(&self, item_count: usize) -> usize {
        if !self.is_pay_per_event || item_count == 0 {
            return item_count;
        }

        let item_price = self.event_price(SHOP_PROFILE_RESULT_CHARGE_EVENT)
            + self.event_price(DEFAULT_DATASET_ITEM_CHARGE_EVENT);
        if item_price <= 0.0 {
            return item_count;
        }

        let available_count = self.max_event_count(item_price);
        if available_count >= item_count {
            return item_count;
        }
        if available_count == 0 && self.total_charged_usd() <= self.max_total_charge_usd {
            return 1;
        }
        available_count
    }

    pub fn charge_event(&mut self, event_name: &str) -> ChargeResult {
        if !self.is_pay_per_event {
            return ChargeResult {
                event_charge_limit_reached: false,
                charged_count: 0,
            };
        }

        let price = self.event_price(event_name);
        let maximum_count = if price <= 0.0 {
            usize::MAX
        } else {
            self.max_event_count(price)
        };
        let charged_count = if maximum_count > 0 {
            1
        } else if self.total_charged_usd() <= self.max_total_charge_usd {
            1
        } else {
            0
        };

        if charged_count > 0 {
            *self
                .charged_event_counts
                .entry(event_name.to_owned())
                .or_default() += charged_count as u64;
        }

        ChargeResult {
            event_charge_limit_reached: price > 0.0 && self.max_event_count(price) == 0,
            charged_count,
        }
    }

    pub fn is_priced_event(&self, event_name: &str) -> bool {
        self.event_prices_usd.contains_key(event_name)
    }

    fn event_price(&self, event_name: &str) -> f64 {
        self.event_prices_usd
            .get(event_name)
            .copied()
            .unwrap_or(0.0)
    }

    fn total_charged_usd(&self) -> f64 {
        let total = self
            .charged_event_counts
            .iter()
            .map(|(name, count)| self.event_price(name) * *count as f64)
            .sum::<f64>();
        (total * 1_000_000.0).round() / 1_000_000.0
    }

    fn max_event_count(&self, event_price_usd: f64) -> usize {
        let count = (self.max_total_charge_usd - self.total_charged_usd()) / event_price_usd;
        if count.is_infinite() && count.is_sign_positive() {
            return usize::MAX;
        }
        if !count.is_finite() || count <= 0.0 {
            return 0;
        }

        let rounded = (count * 10_000.0).round() / 10_000.0;
        rounded.floor() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pay_per_event_budget(max_total: f64, charged: Value) -> ChargingManager {
        ChargingManager::from_environment(
            &json!({
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": { "actorChargeEvents": {
                    "shop-profile-result": { "eventPriceUsd": 0.10 },
                    "apify-default-dataset-item": { "eventPriceUsd": 0.05 },
                    "other-event": { "eventPriceUsd": 0.03 }
                }}
            }),
            &charged,
            Some(max_total),
        )
    }

    #[test]
    fn free_runs_do_not_limit_or_charge_dataset_items() {
        let mut budget = ChargingManager::free();
        assert!(!budget.is_pay_per_event());
        assert_eq!(budget.push_limit(1), 1);
        assert_eq!(
            budget.charge_event(SHOP_PROFILE_RESULT_CHARGE_EVENT),
            ChargeResult {
                event_charge_limit_reached: false,
                charged_count: 0,
            }
        );
    }

    #[test]
    fn limits_pushes_using_explicit_and_default_dataset_event_prices() {
        let budget = pay_per_event_budget(0.30, json!({"other-event": 1}));
        assert_eq!(budget.push_limit(3), 1);
    }

    #[test]
    fn permits_one_item_to_reach_the_budget_then_stops_subsequent_results() {
        let mut budget = pay_per_event_budget(0.15, json!({}));
        assert_eq!(budget.push_limit(1), 1);

        let explicit = budget.charge_event(SHOP_PROFILE_RESULT_CHARGE_EVENT);
        let dataset = budget.charge_event(DEFAULT_DATASET_ITEM_CHARGE_EVENT);
        assert_eq!(explicit.charged_count, 1);
        assert_eq!(dataset.charged_count, 1);
        assert!(dataset.event_charge_limit_reached);
        assert_eq!(budget.push_limit(1), 1);
    }

    #[test]
    fn accepts_charged_counts_from_prior_operations_in_the_run() {
        let budget = pay_per_event_budget(
            0.15,
            json!({
                "shop-profile-result": 1,
                "apify-default-dataset-item": 1,
            }),
        );
        assert_eq!(budget.push_limit(1), 1); // one overflow event matches the SDK behavior

        let over_budget = pay_per_event_budget(
            0.15,
            json!({
                "shop-profile-result": 2,
                "apify-default-dataset-item": 1,
            }),
        );
        assert_eq!(over_budget.push_limit(1), 0);
    }

    #[test]
    fn recognizes_only_configured_events_as_platform_charge_requests() {
        let budget = pay_per_event_budget(1.0, json!({}));
        assert!(budget.is_priced_event(SHOP_PROFILE_RESULT_CHARGE_EVENT));
        assert!(!budget.is_priced_event("unknown-event"));
    }
}
