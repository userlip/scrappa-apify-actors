use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};

use crate::apify::ApifyClient;

pub(crate) const REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT: &str = "property-result";
const DEFAULT_DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";

pub(crate) struct PpeBudget {
    is_pay_per_event: bool,
    event_prices: HashMap<String, Option<f64>>,
    event_counts: HashMap<String, u64>,
    max_total_charge_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PushChargedPropertyResult {
    pub(crate) saved: bool,
    pub(crate) status_message: Option<String>,
    pub(crate) charged_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EventChargeResult {
    charged_count: u64,
    event_charge_limit_reached: bool,
}

impl PpeBudget {
    pub(crate) fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_info = data.get("pricingInfo");
        if pricing_info
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(Self {
                is_pay_per_event: false,
                event_prices: HashMap::new(),
                event_counts: HashMap::new(),
                max_total_charge_usd: f64::INFINITY,
            });
        }

        let events = pricing_info
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (name, event) in events {
            let price = event.get("eventPriceUsd").and_then(Value::as_f64);
            if price.is_some_and(|price| !price.is_finite() || price < 0.0) {
                bail!("Apify run returned invalid price for charged event {name}");
            }
            event_prices.insert(name.clone(), price);
        }

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|value| *value != 0.0)
            .unwrap_or(f64::INFINITY);
        if !max_total_charge_usd.is_finite() && max_total_charge_usd != f64::INFINITY {
            bail!("Apify run returned an invalid spending limit");
        }

        let mut event_counts = HashMap::new();
        if let Some(counts) = data.get("chargedEventCounts").and_then(Value::as_object) {
            for (name, count) in counts {
                let count = count
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {name}"))?;
                event_counts.insert(name.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event: true,
            event_prices,
            event_counts,
            max_total_charge_usd,
        })
    }

    pub(crate) fn charge_limit_status(
        &self,
        total_results: usize,
        property_index: usize,
    ) -> Option<String> {
        if !self.is_pay_per_event
            || self.calculate_max_event_charge_count(REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT)
                > 0
        {
            return None;
        }

        Some(format!(
            "Charge limit reached before fetching Redfin property {}; {} property detail result(s) were saved.",
            property_index + 1,
            total_results
        ))
    }

    pub(crate) async fn push_property(
        &mut self,
        apify: &ApifyClient,
        property: &Value,
        property_index: usize,
    ) -> Result<PushChargedPropertyResult> {
        if !self.is_pay_per_event {
            apify.push_dataset_item(property).await?;
            return Ok(PushChargedPropertyResult {
                saved: true,
                status_message: None,
                charged_count: 0,
            });
        }

        self.push_dataset_data(
            apify,
            property,
            Some(REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT),
            property_index,
        )
        .await
    }

    pub(crate) async fn push_error_item(
        &mut self,
        apify: &ApifyClient,
        item: &Value,
    ) -> Result<()> {
        self.push_dataset_data(apify, item, None, 0).await?;
        Ok(())
    }

    async fn push_dataset_data(
        &mut self,
        apify: &ApifyClient,
        item: &Value,
        event_name: Option<&str>,
        property_index: usize,
    ) -> Result<PushChargedPropertyResult> {
        let keep_item = self.calculate_push_data_limit(event_name) > 0;
        if !keep_item {
            let status_message = event_name.map(|_| {
                format!(
                    "Charge limit reached before saving Redfin property detail result {}.",
                    property_index + 1
                )
            });
            return Ok(PushChargedPropertyResult {
                saved: false,
                status_message,
                charged_count: 0,
            });
        }

        apify.push_dataset_item(item).await?;

        let mut charged_count = 0;
        let mut event_charge_limit_reached = false;
        if let Some(event_name) = event_name {
            let result = self
                .charge_event(
                    apify,
                    event_name,
                    1,
                    &format!(
                        "redfin-property-result-{}-{property_index}",
                        apify.actor_run_id
                    ),
                )
                .await?;
            charged_count += result.charged_count;
            event_charge_limit_reached |= result.event_charge_limit_reached;
        }

        if self.is_pay_per_event {
            let default_result = self
                .charge_event(
                    apify,
                    DEFAULT_DATASET_ITEM_CHARGE_EVENT,
                    1,
                    &format!(
                        "redfin-default-dataset-item-{}-{property_index}",
                        apify.actor_run_id
                    ),
                )
                .await?;
            charged_count += default_result.charged_count;
            event_charge_limit_reached |= default_result.event_charge_limit_reached;
        }

        if event_name.is_none() {
            return Ok(PushChargedPropertyResult {
                saved: true,
                status_message: None,
                charged_count,
            });
        }

        let status_message = if event_charge_limit_reached {
            let message = if charged_count >= 1 {
                format!(
                    "Charge limit reached after saving Redfin property detail result {}.",
                    property_index + 1
                )
            } else {
                format!(
                    "Charge limit reached before saving Redfin property detail result {}.",
                    property_index + 1
                )
            };
            println!(
                "{} {}",
                message,
                json!({
                    "event": REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT,
                    "charged_count": charged_count,
                    "property_index": property_index,
                })
            );
            Some(message)
        } else {
            None
        };

        Ok(PushChargedPropertyResult {
            saved: charged_count >= 1,
            status_message,
            charged_count,
        })
    }

    fn calculate_push_data_limit(&self, event_name: Option<&str>) -> u64 {
        let mut item_price = event_name
            .and_then(|name| self.event_prices.get(name).copied().flatten())
            .unwrap_or(0.0);
        item_price += self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_CHARGE_EVENT)
            .copied()
            .flatten()
            .unwrap_or(0.0);

        let max_count = if item_price > 0.0 {
            self.calculate_max_charges_by_price(item_price)
        } else {
            u64::MAX
        };
        if max_count >= 1 {
            return 1;
        }

        if self.total_charged_amount() <= self.max_total_charge_usd {
            1
        } else {
            0
        }
    }

    async fn charge_event(
        &mut self,
        apify: &ApifyClient,
        event_name: &str,
        requested_count: u64,
        idempotency_key: &str,
    ) -> Result<EventChargeResult> {
        let max_event_charge_count = self.calculate_max_event_charge_count(event_name);
        let total_charged = self.total_charged_amount();
        let charged_count = if requested_count <= max_event_charge_count {
            requested_count
        } else if total_charged <= self.max_total_charge_usd {
            max_event_charge_count.saturating_add(1)
        } else {
            0
        };

        if charged_count == 0 {
            return Ok(EventChargeResult {
                charged_count: 0,
                event_charge_limit_reached: requested_count > 0,
            });
        }

        let count = self.event_counts.entry(event_name.to_owned()).or_default();
        *count = count.saturating_add(charged_count);

        if !event_name.starts_with("apify-") && self.event_prices.contains_key(event_name) {
            apify
                .charge_event(event_name, charged_count, idempotency_key)
                .await?;
        }

        Ok(EventChargeResult {
            charged_count,
            event_charge_limit_reached: self.calculate_max_event_charge_count(event_name) == 0,
        })
    }

    fn calculate_max_event_charge_count(&self, event_name: &str) -> u64 {
        let Some(price) = self
            .event_prices
            .get(event_name)
            .copied()
            .flatten()
            .filter(|price| *price != 0.0)
        else {
            return u64::MAX;
        };
        self.calculate_max_charges_by_price(price)
    }

    fn calculate_max_charges_by_price(&self, price: f64) -> u64 {
        let unrounded = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !unrounded.is_finite() {
            return if unrounded.is_sign_positive() {
                u64::MAX
            } else {
                0
            };
        }

        let rounded = (unrounded * 10_000.0).round() / 10_000.0;
        rounded.floor().max(0.0).min(u64::MAX as f64) as u64
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .event_counts
            .iter()
            .map(|(name, count)| {
                self.event_prices
                    .get(name)
                    .copied()
                    .flatten()
                    .unwrap_or(0.0)
                    * *count as f64
            })
            .sum::<f64>();
        if total.is_finite() {
            (total * 1_000_000.0).round() / 1_000_000.0
        } else {
            total
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_per_event_budget_and_stops_before_the_next_property() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "property-result": {"eventPriceUsd": 0.0005}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.0005},
                "chargedEventCounts": {}
            }
        });
        let mut budget = PpeBudget::from_run(&run).unwrap();
        assert!(budget.charge_limit_status(0, 0).is_none());
        assert_eq!(budget.calculate_push_data_limit(Some("property-result")), 1);

        *budget
            .event_counts
            .entry("property-result".to_owned())
            .or_default() = 1;
        assert_eq!(
            budget.charge_limit_status(1, 1),
            Some("Charge limit reached before fetching Redfin property 2; 1 property detail result(s) were saved.".to_owned())
        );
    }

    #[test]
    fn leaves_non_ppe_runs_unlimited_and_handles_default_dataset_price() {
        let free_run = json!({"data": {"pricingInfo": {"pricingModel": "FREE"}}});
        let free = PpeBudget::from_run(&free_run).unwrap();
        assert!(!free.is_pay_per_event);
        assert!(free.charge_limit_status(0, 0).is_none());

        let paid_run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "property-result": {"eventPriceUsd": 0.0004},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.0005},
                "chargedEventCounts": {}
            }
        });
        let budget = PpeBudget::from_run(&paid_run).unwrap();
        assert_eq!(budget.calculate_push_data_limit(Some("property-result")), 1);
        assert_eq!(budget.calculate_push_data_limit(None), 1);
    }
}
