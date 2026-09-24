use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use std::collections::HashMap;

pub(crate) const PROPERTY_RESULT_CHARGE_EVENT: &str = "property-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub(crate) struct ChargePricing {
    pub(crate) is_pay_per_event: bool,
    max_total_charge: f64,
    spent: f64,
    event_prices: HashMap<String, f64>,
    configured_events: Vec<String>,
}

impl ChargePricing {
    pub(crate) fn from_run(run: &Value) -> Result<Self> {
        let data = run.get("data").unwrap_or(run);
        let pricing_info = data.get("pricingInfo");
        if pricing_info
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(Self {
                is_pay_per_event: false,
                max_total_charge: f64::INFINITY,
                spent: 0.0,
                event_prices: HashMap::new(),
                configured_events: Vec::new(),
            });
        }

        let configured = pricing_info
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        let mut configured_events = Vec::new();
        for (name, event) in configured {
            configured_events.push(name.clone());
            if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
                if !price.is_finite() || price < 0.0 {
                    bail!("Apify run returned invalid charging values");
                }
                event_prices.insert(name.clone(), price);
            }
        }

        let max_total_charge = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|amount| amount.is_finite() && *amount != 0.0)
            .unwrap_or(f64::INFINITY);
        let counts = data.get("chargedEventCounts").and_then(Value::as_object);
        let mut spent = 0.0;
        if let Some(counts) = counts {
            for (event_name, count) in counts {
                let count = count
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
                if let Some(price) = event_prices.get(event_name) {
                    spent += price * count as f64;
                }
            }
        }
        if !spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        Ok(Self {
            is_pay_per_event: true,
            max_total_charge,
            spent: round_to_six_decimals(spent),
            event_prices,
            configured_events,
        })
    }

    pub(crate) fn plan_dataset_push(&self, requested: usize) -> PushPlan {
        if !self.is_pay_per_event {
            return PushPlan {
                items_to_push: requested,
                custom_event_charge_count: 0,
                limit_reached: false,
                charged_event_count: 0,
                should_charge_custom_event: false,
            };
        }

        let custom_price = self
            .event_prices
            .get(PROPERTY_RESULT_CHARGE_EVENT)
            .copied()
            .unwrap_or(0.0);
        let default_item_price = self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0.0);
        let item_price = custom_price + default_item_price;
        let remaining_count = self.available_count(item_price);
        let items_to_push = if remaining_count >= requested {
            requested
        } else if remaining_count == 0 && self.spent <= self.max_total_charge {
            usize::from(requested > 0)
        } else {
            remaining_count
        };
        let items_to_push = items_to_push.min(requested.max(usize::from(requested > 0)));
        if items_to_push == 0 {
            return PushPlan {
                items_to_push,
                custom_event_charge_count: 0,
                limit_reached: requested > 0,
                charged_event_count: 0,
                should_charge_custom_event: false,
            };
        }

        let mut spent = self.spent;
        let custom_count =
            self.charge_count(PROPERTY_RESULT_CHARGE_EVENT, items_to_push, &mut spent);
        let default_count =
            self.charge_count(DEFAULT_DATASET_ITEM_EVENT, items_to_push, &mut spent);
        let limit_reached = self.event_limit_reached(PROPERTY_RESULT_CHARGE_EVENT, spent)
            || self.event_limit_reached(DEFAULT_DATASET_ITEM_EVENT, spent);

        PushPlan {
            items_to_push,
            custom_event_charge_count: custom_count,
            limit_reached,
            charged_event_count: custom_count + default_count,
            should_charge_custom_event: self
                .configured_events
                .iter()
                .any(|event| event == PROPERTY_RESULT_CHARGE_EVENT),
        }
    }

    fn charge_count(&self, event_name: &str, count: usize, spent: &mut f64) -> usize {
        let price = self.event_prices.get(event_name).copied().unwrap_or(0.0);
        let available = self.available_count_for_price(price, *spent);
        let charged = if count <= available {
            count
        } else if *spent <= self.max_total_charge {
            available.saturating_add(1)
        } else {
            0
        };
        *spent = round_to_six_decimals(*spent + price * charged as f64);
        charged
    }

    fn event_limit_reached(&self, event_name: &str, spent: f64) -> bool {
        self.event_prices
            .get(event_name)
            .is_some_and(|price| *price > 0.0 && self.available_count_for_price(*price, spent) == 0)
    }

    fn available_count(&self, item_price: f64) -> usize {
        self.available_count_for_price(item_price, self.spent)
    }

    fn available_count_for_price(&self, price: f64, spent: f64) -> usize {
        if price <= 0.0 {
            return usize::MAX;
        }
        if self.max_total_charge.is_infinite() {
            return usize::MAX;
        }
        let amount = ((self.max_total_charge - spent) / price * 10_000.0).round() / 10_000.0;
        if amount <= 0.0 || !amount.is_finite() {
            0
        } else {
            amount.floor().min(usize::MAX as f64) as usize
        }
    }
}

fn round_to_six_decimals(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PushPlan {
    pub(crate) items_to_push: usize,
    pub(crate) custom_event_charge_count: usize,
    pub(crate) limit_reached: bool,
    pub(crate) charged_event_count: usize,
    pub(crate) should_charge_custom_event: bool,
}
