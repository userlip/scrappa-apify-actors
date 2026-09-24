use std::collections::HashMap;

use anyhow::{anyhow, Result};
use serde_json::Value;

pub(crate) const PROPERTY_RESULT_EVENT: &str = "property-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

#[derive(Debug, Clone)]
pub(crate) struct BillingState {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    charged_event_counts: HashMap<String, u64>,
}

impl BillingState {
    pub(crate) fn from_run(run: &Value) -> Result<Self> {
        let data = run.get("data").unwrap_or(run);
        let pricing_info = data
            .get("pricingInfo")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|value| *value > 0.0)
            .unwrap_or(f64::INFINITY);
        Self::from_parts(pricing_info, charged_event_counts, max_total_charge_usd)
    }

    pub(crate) fn from_environment() -> Result<Option<Self>> {
        let (Some(pricing), Some(charged_counts)) = (
            std::env::var("APIFY_ACTOR_PRICING_INFO").ok(),
            std::env::var("APIFY_CHARGED_ACTOR_EVENT_COUNTS").ok(),
        ) else {
            return Ok(None);
        };
        let pricing_info: Value = serde_json::from_str(&pricing)
            .map_err(|error| anyhow!("APIFY_ACTOR_PRICING_INFO was not valid JSON: {error}"))?;
        let charged_counts: Value = serde_json::from_str(&charged_counts).map_err(|error| {
            anyhow!("APIFY_CHARGED_ACTOR_EVENT_COUNTS was not valid JSON: {error}")
        })?;
        let charged_counts = charged_counts
            .as_object()
            .ok_or_else(|| anyhow!("APIFY_CHARGED_ACTOR_EVENT_COUNTS must be a JSON object"))?;
        let max_total_charge_usd = std::env::var("ACTOR_MAX_TOTAL_CHARGE_USD")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|value| {
                value
                    .parse::<f64>()
                    .map_err(|_| anyhow!("ACTOR_MAX_TOTAL_CHARGE_USD must be a number"))
            })
            .transpose()?
            .filter(|value| *value > 0.0)
            .unwrap_or(f64::INFINITY);
        Self::from_parts(&pricing_info, charged_counts, max_total_charge_usd).map(Some)
    }

    fn from_parts(
        pricing_info: &Value,
        charged_event_counts: &serde_json::Map<String, Value>,
        max_total_charge_usd: f64,
    ) -> Result<Self> {
        let is_pay_per_event =
            pricing_info.get("pricingModel").and_then(Value::as_str) == Some("PAY_PER_EVENT");
        let mut event_prices = HashMap::new();
        if is_pay_per_event {
            let events = pricing_info
                .pointer("/pricingPerEvent/actorChargeEvents")
                .and_then(Value::as_object)
                .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
            for (event_name, event) in events {
                let price = match event.get("eventPriceUsd").and_then(Value::as_f64) {
                    Some(price) => price,
                    None => highest_tier_event_price(event, event_name)?,
                };
                if !price.is_finite() || price < 0.0 {
                    return Err(anyhow!(
                        "Apify run returned an invalid price for event {event_name}"
                    ));
                }
                event_prices.insert(event_name.clone(), price);
            }
        }
        let charged_event_counts = charged_event_counts
            .iter()
            .map(|(event_name, count)| {
                let count = count
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
                Ok((event_name.clone(), count))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        })
    }

    pub(crate) fn is_pay_per_event(&self) -> bool {
        self.is_pay_per_event
    }

    pub(crate) fn charge_limit_status(
        &self,
        total_results: usize,
        search_index: usize,
    ) -> Option<String> {
        if !self.is_pay_per_event
            || self
                .max_charges_within_limit(PROPERTY_RESULT_EVENT)
                .is_none_or(|count| count > 0)
        {
            return None;
        }
        Some(format!(
            "Charge limit reached before fetching Redfin search {}; {total_results} property result(s) were saved.",
            search_index + 1,
        ))
    }

    pub(crate) fn can_write_property_result(&self) -> bool {
        if !self.is_pay_per_event {
            return true;
        }
        let event_price = self
            .event_prices
            .get(PROPERTY_RESULT_EVENT)
            .copied()
            .unwrap_or(0.0);
        let dataset_price = self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0.0);
        self.max_charges_at_price(event_price + dataset_price)
            .is_none_or(|count| count > 0)
    }

    pub(crate) fn record_dataset_item(&mut self) {
        self.record_event(DEFAULT_DATASET_ITEM_EVENT);
    }

    pub(crate) fn should_charge_property_result(&self) -> bool {
        self.event_prices.contains_key(PROPERTY_RESULT_EVENT)
    }

    pub(crate) fn record_property_result_charge(&mut self) {
        self.record_event(PROPERTY_RESULT_EVENT);
    }

    pub(crate) fn charge_limit_reached(&self) -> bool {
        [PROPERTY_RESULT_EVENT, DEFAULT_DATASET_ITEM_EVENT]
            .iter()
            .filter(|event| self.event_prices.contains_key(**event))
            .any(|event| {
                self.max_charges_within_limit(event)
                    .is_some_and(|count| count == 0)
            })
    }

    fn record_event(&mut self, event_name: &str) {
        *self
            .charged_event_counts
            .entry(event_name.to_owned())
            .or_default() += 1;
    }

    fn max_charges_within_limit(&self, event_name: &str) -> Option<usize> {
        self.max_charges_at_price(*self.event_prices.get(event_name)?)
    }

    fn max_charges_at_price(&self, price: f64) -> Option<usize> {
        if price <= 0.0 || self.max_total_charge_usd.is_infinite() {
            return None;
        }
        let remaining = self.max_total_charge_usd - self.total_charged_amount();
        let raw_count = remaining / price;
        let rounded_count = ((raw_count * 10_000.0).round() / 10_000.0).floor();
        Some(if rounded_count.is_finite() && rounded_count > 0.0 {
            rounded_count.min(usize::MAX as f64) as usize
        } else {
            0
        })
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_event_counts
            .iter()
            .map(|(event_name, count)| {
                self.event_prices.get(event_name).copied().unwrap_or(0.0) * *count as f64
            })
            .sum::<f64>();
        (total * 1_000_000.0).round() / 1_000_000.0
    }
}

fn highest_tier_event_price(event: &Value, event_name: &str) -> Result<f64> {
    let tiers = event
        .get("eventTieredPricingUsd")
        .and_then(Value::as_object)
        .filter(|tiers| !tiers.is_empty())
        .ok_or_else(|| anyhow!("Apify run did not provide the price for event {event_name}"))?;

    // The run pricing table doesn't identify the current user's tier, so use the highest
    // configured rate to keep the local budget guard from underestimating any tier's charge.
    tiers
        .iter()
        .map(|(tier, pricing)| {
            let price = pricing
                .get("tieredEventPriceUsd")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    anyhow!(
                        "Apify run did not provide the {tier} tier price for event {event_name}"
                    )
                })?;
            if !price.is_finite() || price < 0.0 {
                return Err(anyhow!(
                    "Apify run returned an invalid {tier} tier price for event {event_name}"
                ));
            }
            Ok(price)
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .max_by(f64::total_cmp)
        .ok_or_else(|| anyhow!("Apify run did not provide the price for event {event_name}"))
}

pub(crate) fn charge_limit_message(
    search_index: usize,
    saved: usize,
    requested: usize,
    charged: usize,
) -> String {
    if charged >= 1 {
        format!(
            "Charge limit reached after saving {saved} of {requested} Redfin property result(s) for search {}.",
            search_index + 1,
        )
    } else {
        format!(
            "Charge limit reached before saving the next Redfin property result for search {}.",
            search_index + 1,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ppe_run(max_total: f64, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel":"PAY_PER_EVENT",
                    "pricingPerEvent":{"actorChargeEvents":{
                        "property-result":{"eventPriceUsd":0.30},
                        "apify-default-dataset-item":{"eventPriceUsd":0.0}
                    }}
                },
                "chargedEventCounts":counts,
                "options":{"maxTotalChargeUsd":max_total}
            }
        })
    }

    fn tiered_ppe_run(max_total: f64, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel":"PAY_PER_EVENT",
                    "pricingPerEvent":{"actorChargeEvents":{
                        "property-result":{"eventTieredPricingUsd":{
                            "FREE":{"tieredEventPriceUsd":0.30},
                            "BRONZE":{"tieredEventPriceUsd":0.25},
                            "SILVER":{"tieredEventPriceUsd":0.22},
                            "GOLD":{"tieredEventPriceUsd":0.20}
                        }},
                        "apify-default-dataset-item":{"eventTieredPricingUsd":{
                            "FREE":{"tieredEventPriceUsd":0.05},
                            "BRONZE":{"tieredEventPriceUsd":0.04},
                            "SILVER":{"tieredEventPriceUsd":0.03},
                            "GOLD":{"tieredEventPriceUsd":0.02}
                        }}
                    }}
                },
                "chargedEventCounts":counts,
                "options":{"maxTotalChargeUsd":max_total}
            }
        })
    }

    #[test]
    fn tracks_remaining_charge_budget_across_custom_and_dataset_events() {
        let mut billing =
            BillingState::from_run(&ppe_run(0.60, json!({"property-result":1}))).unwrap();
        assert_eq!(billing.charge_limit_status(2, 1), None);
        assert!(billing.can_write_property_result());
        billing.record_dataset_item();
        billing.record_property_result_charge();
        assert_eq!(billing.charged_event_counts[PROPERTY_RESULT_EVENT], 2);
        assert_eq!(billing.charged_event_counts.values().sum::<u64>(), 3);
        assert!(billing.charge_limit_reached());
        assert!(billing.charge_limit_status(2, 1).is_some());
    }

    #[test]
    fn accounts_for_dataset_event_price_when_limiting_a_property_write() {
        let run = json!({
            "data": {
                "pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{
                    "property-result":{"eventPriceUsd":0.20},
                    "apify-default-dataset-item":{"eventPriceUsd":0.10}
                }}},
                "chargedEventCounts":{},"options":{"maxTotalChargeUsd":0.29}
            }
        });
        let billing = BillingState::from_run(&run).unwrap();
        assert!(billing.charge_limit_status(0, 0).is_none());
        assert!(!billing.can_write_property_result());
    }

    #[test]
    fn accepts_tiered_event_prices_and_guards_budget_with_highest_tier_rate() {
        let billing = BillingState::from_run(&tiered_ppe_run(0.349, json!({}))).unwrap();

        assert!(billing.is_pay_per_event());
        assert!(billing.should_charge_property_result());
        assert_eq!(billing.event_prices[PROPERTY_RESULT_EVENT], 0.30);
        assert_eq!(billing.event_prices[DEFAULT_DATASET_ITEM_EVENT], 0.05);
        assert!(!billing.can_write_property_result());
    }

    #[test]
    fn non_ppe_runs_have_no_charge_gate() {
        let billing = BillingState::from_run(&json!({
            "data":{"pricingInfo":{"pricingModel":"FLAT_PRICE_PER_MONTH"},"chargedEventCounts":{},"options":{}}
        })).unwrap();
        assert!(!billing.is_pay_per_event());
        assert!(billing.charge_limit_status(0, 0).is_none());
        assert!(billing.can_write_property_result());
    }
}
