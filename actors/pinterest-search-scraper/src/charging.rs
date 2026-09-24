use anyhow::{anyhow, bail, Result};
use serde_json::{json, Map, Value};

use crate::apify_client::ApifyClient;

pub(crate) const PIN_RESULT_CHARGE_EVENT: &str = "pin-result";
pub(crate) const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub(crate) struct ChargeBudget {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    pub(crate) event_prices: Map<String, Value>,
    charged_event_counts: Map<String, Value>,
}

impl ChargeBudget {
    pub(crate) fn from_actor_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_info = data.get("pricingInfo");
        let is_pay_per_event = pricing_info
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self {
                is_pay_per_event: false,
                max_total_charge_usd: f64::INFINITY,
                event_prices: Map::new(),
                charged_event_counts: Map::new(),
            });
        }

        let event_prices = pricing_info
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?
            .iter()
            .map(|(name, event)| {
                let price = event
                    .get("eventPriceUsd")
                    .and_then(Value::as_f64)
                    .filter(|price| price.is_finite() && *price >= 0.0)
                    .ok_or_else(|| anyhow!("Apify run did not provide a valid price for {name}"))?;
                Ok((name.clone(), json!(price)))
            })
            .collect::<Result<Map<String, Value>>>()?;

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|amount| amount.is_finite() && *amount >= 0.0)
            .filter(|amount| *amount != 0.0)
            .unwrap_or(f64::INFINITY);
        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (name, count) in &charged_event_counts {
            if count.as_u64().is_none() {
                bail!("Invalid charged event count for {name}");
            }
        }

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

    pub(crate) fn price(&self, event_name: &str) -> Option<f64> {
        self.event_prices.get(event_name).and_then(Value::as_f64)
    }

    pub(crate) fn charged_count(&self, event_name: &str) -> u64 {
        self.charged_event_counts
            .get(event_name)
            .and_then(Value::as_u64)
            .unwrap_or(0)
    }

    pub(crate) fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_event_counts
            .iter()
            .map(|(event_name, count)| {
                self.price(event_name).unwrap_or(0.0) * count.as_u64().unwrap_or(0) as f64
            })
            .sum::<f64>();
        if total.is_finite() {
            format!("{total:.6}").parse().unwrap_or(total)
        } else {
            total
        }
    }

    pub(crate) fn max_event_charge_count_within_limit(&self, event_name: &str) -> usize {
        let Some(price) = self.price(event_name) else {
            return usize::MAX;
        };
        if price == 0.0 {
            return usize::MAX;
        }
        self.max_charge_count_by_price(price)
    }

    pub(crate) fn max_charge_count_by_price(&self, price: f64) -> usize {
        if price <= 0.0 || self.max_total_charge_usd.is_infinite() {
            return usize::MAX;
        }
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
        if rounded <= 0.0 {
            0
        } else {
            rounded.floor().min(usize::MAX as f64) as usize
        }
    }

    pub(crate) fn chargeable_pin_capacity(&self) -> usize {
        self.max_event_charge_count_within_limit(PIN_RESULT_CHARGE_EVENT)
    }

    pub(crate) fn pushable_pin_count(&self, requested_count: usize) -> usize {
        if !self.is_pay_per_event || requested_count == 0 {
            return requested_count;
        }

        let item_price = self.price(PIN_RESULT_CHARGE_EVENT).unwrap_or(0.0)
            + self.price(DEFAULT_DATASET_ITEM_EVENT).unwrap_or(0.0);
        let max_charged_count = if item_price > 0.0 {
            self.max_charge_count_by_price(item_price)
        } else {
            usize::MAX
        };
        if max_charged_count >= requested_count {
            return requested_count;
        }
        if max_charged_count == 0 && self.total_charged_amount() <= self.max_total_charge_usd {
            return 1;
        }
        max_charged_count
    }

    pub(crate) fn prepare_event_charge(
        &mut self,
        event_name: &str,
        requested_count: usize,
    ) -> usize {
        if !self.is_pay_per_event {
            return 0;
        }

        let max_event_count = self.max_event_charge_count_within_limit(event_name);
        let charged_count = if requested_count <= max_event_count {
            requested_count
        } else if self.total_charged_amount() <= self.max_total_charge_usd {
            max_event_count.saturating_add(1)
        } else {
            0
        };
        if charged_count > 0 {
            let new_count = self
                .charged_count(event_name)
                .saturating_add(charged_count as u64);
            self.charged_event_counts
                .insert(event_name.to_owned(), json!(new_count));
        }
        charged_count
    }

    pub(crate) fn event_charge_limit_reached(&self, event_name: &str) -> bool {
        self.max_event_charge_count_within_limit(event_name) == 0
    }
}

#[derive(Debug)]
pub(crate) struct PinterestChargedSaveResult {
    pub(crate) saved_count: usize,
    charge_limit_reached: bool,
}

pub(crate) fn pinterest_charged_save_result(
    charged_count: usize,
    event_charge_limit_reached: bool,
    requested_count: usize,
) -> PinterestChargedSaveResult {
    let saved_count = charged_count.min(requested_count);
    PinterestChargedSaveResult {
        saved_count,
        charge_limit_reached: event_charge_limit_reached || saved_count < requested_count,
    }
}

#[derive(Debug)]
pub(crate) struct PushChargedItemsResult {
    pub(crate) saved_count: usize,
    pub(crate) status_message: Option<String>,
}

pub(crate) async fn push_charged_pins(
    apify: &ApifyClient,
    budget: &mut ChargeBudget,
    items: &[Value],
    query: &str,
) -> Result<PushChargedItemsResult> {
    if items.is_empty() {
        return Ok(PushChargedItemsResult {
            saved_count: 0,
            status_message: None,
        });
    }

    if !budget.is_pay_per_event() {
        apify.push_dataset_items(items).await?;
        return Ok(PushChargedItemsResult {
            saved_count: items.len(),
            status_message: None,
        });
    }

    let pushed_count = budget.pushable_pin_count(items.len());
    let pushed_items = &items[..pushed_count];
    if !pushed_items.is_empty() {
        apify.push_dataset_items(pushed_items).await?;
    }

    let pin_charged_count = budget.prepare_event_charge(PIN_RESULT_CHARGE_EVENT, pushed_count);
    let dataset_charged_count =
        budget.prepare_event_charge(DEFAULT_DATASET_ITEM_EVENT, pushed_count);
    if budget.event_prices.contains_key(PIN_RESULT_CHARGE_EVENT) && pin_charged_count > 0 {
        apify
            .charge_event(PIN_RESULT_CHARGE_EVENT, pin_charged_count)
            .await?;
    }

    let charge_limit_reached = if pushed_count == 0 {
        true
    } else {
        budget.event_charge_limit_reached(PIN_RESULT_CHARGE_EVENT)
            || budget.event_charge_limit_reached(DEFAULT_DATASET_ITEM_EVENT)
    };
    let charge_result = pinterest_charged_save_result(
        pin_charged_count.saturating_add(dataset_charged_count),
        charge_limit_reached,
        items.len(),
    );

    let status_message = if charge_result.charge_limit_reached {
        let message = format!(
            "Charge limit reached after saving {} of {} Pinterest pin result(s) for \"{query}\".",
            charge_result.saved_count,
            items.len()
        );
        eprintln!(
            "{message} {}",
            json!({
                "event": PIN_RESULT_CHARGE_EVENT,
                "charged_count": charge_result.saved_count,
                "requested_count": items.len(),
                "saved_count": charge_result.saved_count,
                "query": query,
            })
        );
        Some(message)
    } else {
        None
    };

    Ok(PushChargedItemsResult {
        saved_count: charge_result.saved_count,
        status_message,
    })
}
