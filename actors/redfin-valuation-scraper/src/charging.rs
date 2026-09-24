use crate::apify::ApifyRuntime;
use serde_json::Value;
use std::collections::HashMap;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub const REDFIN_VALUATION_RESULT_CHARGE_EVENT: &str = "valuation-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

#[derive(Clone, Debug)]
struct EventPrice {
    price_usd: f64,
    title: String,
}

#[derive(Clone, Debug)]
pub struct ChargingManager {
    is_at_home: bool,
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    use_charging_log_dataset: bool,
    event_prices: HashMap<String, EventPrice>,
    charged_event_counts: HashMap<String, u64>,
}

#[derive(Debug, PartialEq)]
pub struct PushResult {
    pub saved: bool,
    pub status_message: Option<String>,
}

#[derive(Debug)]
struct ChargeResult {
    charged_count: u64,
    event_charge_limit_reached: bool,
    event_name: String,
    event_title: String,
    event_price_usd: f64,
    send_to_platform: bool,
}

impl ChargingManager {
    pub fn from_environment(is_at_home: bool, current_run: &Value) -> Result<Self, String> {
        let test_pay_per_event = env_bool("ACTOR_TEST_PAY_PER_EVENT");
        let use_charging_log_dataset = env_bool("ACTOR_USE_CHARGING_LOG_DATASET");
        if is_at_home && test_pay_per_event {
            return Err("Using the ACTOR_TEST_PAY_PER_EVENT environment variable is only supported in a local development environment".to_owned());
        }
        if is_at_home && use_charging_log_dataset {
            return Err("Using the ACTOR_USE_CHARGING_LOG_DATASET environment variable is only supported in a local development environment".to_owned());
        }

        let env_pricing_info = std::env::var("APIFY_ACTOR_PRICING_INFO").ok();
        let env_charged_counts = std::env::var("APIFY_CHARGED_ACTOR_EVENT_COUNTS").ok();
        let (pricing_info, charged_counts, max_total_charge_usd) =
            match (env_pricing_info, env_charged_counts) {
                (Some(pricing_info), Some(charged_counts)) => {
                    let pricing_info = serde_json::from_str(&pricing_info).map_err(|error| {
                        format!("Invalid APIFY_ACTOR_PRICING_INFO JSON: {error}")
                    })?;
                    let charged_counts =
                        serde_json::from_str(&charged_counts).map_err(|error| {
                            format!("Invalid APIFY_CHARGED_ACTOR_EVENT_COUNTS JSON: {error}")
                        })?;
                    (pricing_info, charged_counts, max_charge_from_env())
                }
                _ if is_at_home => {
                    let pricing_info = current_run
                        .get("pricingInfo")
                        .cloned()
                        .unwrap_or(Value::Null);
                    let charged_counts = current_run
                        .get("chargedEventCounts")
                        .cloned()
                        .unwrap_or(Value::Null);
                    let max_total_charge_usd = current_run["options"]["maxTotalChargeUsd"]
                        .as_f64()
                        .filter(|value| *value != 0.0)
                        .unwrap_or(f64::INFINITY);
                    (pricing_info, charged_counts, max_total_charge_usd)
                }
                _ => (Value::Null, Value::Null, max_charge_from_env()),
            };

        Self::from_values(
            is_at_home,
            test_pay_per_event,
            use_charging_log_dataset,
            &pricing_info,
            &charged_counts,
            max_total_charge_usd,
        )
    }

    pub(crate) fn from_values(
        is_at_home: bool,
        test_pay_per_event: bool,
        use_charging_log_dataset: bool,
        pricing_info: &Value,
        charged_counts: &Value,
        max_total_charge_usd: f64,
    ) -> Result<Self, String> {
        let is_pay_per_event =
            test_pay_per_event || pricing_info["pricingModel"] == "PAY_PER_EVENT";
        let mut event_prices = HashMap::new();
        if pricing_info["pricingModel"] == "PAY_PER_EVENT" {
            if let Some(events) = pricing_info["pricingPerEvent"]["actorChargeEvents"].as_object() {
                for (event_name, event) in events {
                    if let Some(price_usd) = event["eventPriceUsd"].as_f64() {
                        event_prices.insert(
                            event_name.clone(),
                            EventPrice {
                                price_usd,
                                title: event["eventTitle"].as_str().unwrap_or("").to_owned(),
                            },
                        );
                    }
                }
            }
        }

        let charged_event_counts = charged_counts
            .as_object()
            .map(|counts| {
                counts
                    .iter()
                    .filter_map(|(name, count)| count.as_u64().map(|count| (name.clone(), count)))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            is_at_home,
            is_pay_per_event,
            max_total_charge_usd: if max_total_charge_usd == 0.0 {
                f64::INFINITY
            } else {
                max_total_charge_usd
            },
            use_charging_log_dataset,
            event_prices,
            charged_event_counts,
        })
    }

    pub fn get_charge_limit_status(
        &self,
        total_results: usize,
        request_index: usize,
    ) -> Option<String> {
        if !self.is_pay_per_event {
            return None;
        }
        if self.calculate_max_event_charge_count_within_limit(REDFIN_VALUATION_RESULT_CHARGE_EVENT)
            > 0.0
        {
            return None;
        }
        Some(format!(
            "Charge limit reached before fetching Redfin valuation {}; {total_results} valuation result(s) were saved.",
            request_index + 1,
        ))
    }

    fn total_charged_amount(&self) -> f64 {
        let amount = self
            .charged_event_counts
            .iter()
            .map(|(event_name, count)| *count as f64 * self.event_price(event_name))
            .sum::<f64>();
        (amount * 1_000_000.0).round() / 1_000_000.0
    }

    fn event_price(&self, event_name: &str) -> f64 {
        if self.is_at_home {
            self.event_prices
                .get(event_name)
                .map(|price| price.price_usd)
                .unwrap_or(0.0)
        } else {
            1.0
        }
    }

    pub(crate) fn calculate_max_event_charge_count_within_limit(&self, event_name: &str) -> f64 {
        let price = self.event_price(event_name);
        if price == 0.0 {
            return f64::INFINITY;
        }
        let unrounded = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !unrounded.is_finite() {
            return f64::INFINITY;
        }
        ((unrounded * 10_000.0).round() / 10_000.0).floor().max(0.0)
    }

    fn can_push_default_dataset_item(&self) -> bool {
        if !self.is_pay_per_event {
            return true;
        }
        let item_price = self.event_price(REDFIN_VALUATION_RESULT_CHARGE_EVENT)
            + self.event_price(DEFAULT_DATASET_ITEM_EVENT);
        if item_price <= 0.0 {
            return true;
        }
        let unrounded = (self.max_total_charge_usd - self.total_charged_amount()) / item_price;
        if !unrounded.is_finite() {
            return true;
        }
        let max_count = ((unrounded * 10_000.0).round() / 10_000.0).floor().max(0.0);
        max_count >= 1.0
            || (max_count == 0.0 && self.total_charged_amount() <= self.max_total_charge_usd)
    }

    fn record_charge(&mut self, event_name: &str) -> ChargeResult {
        let max_count = self.calculate_max_event_charge_count_within_limit(event_name);
        let total_amount = self.total_charged_amount();
        let charged_count = if max_count >= 1.0 {
            1
        } else if total_amount <= self.max_total_charge_usd {
            1
        } else {
            0
        };
        let price = self.event_price(event_name);
        let price_for_log = if self.is_at_home { price } else { 1.0 };
        if charged_count > 0 {
            *self
                .charged_event_counts
                .entry(event_name.to_owned())
                .or_default() += charged_count;
        }
        let event_limit_reached =
            self.calculate_max_event_charge_count_within_limit(event_name) <= 0.0;
        let event_title = self
            .event_prices
            .get(event_name)
            .map(|price| price.title.clone())
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| format!("Unknown event '{event_name}'"));
        ChargeResult {
            charged_count,
            event_charge_limit_reached: event_limit_reached,
            event_name: event_name.to_owned(),
            event_title,
            event_price_usd: price_for_log,
            send_to_platform: self.is_at_home
                && self.event_prices.contains_key(event_name)
                && !event_name.starts_with("apify-"),
        }
    }

    async fn charge(
        &mut self,
        runtime: &ApifyRuntime,
        event_name: &str,
        idempotency_key: &str,
    ) -> Result<ChargeResult, String> {
        let charge = self.record_charge(event_name);
        if charge.charged_count == 0 {
            return Ok(charge);
        }
        if charge.send_to_platform {
            runtime
                .charge_event(event_name, charge.charged_count, idempotency_key)
                .await?;
        }
        if self.use_charging_log_dataset {
            let timestamp = OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_default();
            let record = serde_json::json!({
                "eventName": charge.event_name,
                "eventTitle": charge.event_title,
                "eventPriceUsd": charge.event_price_usd,
                "chargedCount": charge.charged_count,
                "timestamp": timestamp,
            });
            runtime
                .push_named_dataset_item("charging_log", &record)
                .await?;
        }
        Ok(charge)
    }
}

pub async fn push_charged_valuation(
    runtime: &ApifyRuntime,
    manager: &mut ChargingManager,
    item: &Value,
    request_index: usize,
) -> Result<PushResult, String> {
    if !manager.is_pay_per_event {
        runtime.push_dataset_item(item).await?;
        return Ok(PushResult {
            saved: true,
            status_message: None,
        });
    }

    if !manager.can_push_default_dataset_item() {
        return Ok(PushResult {
            saved: false,
            status_message: Some(
                "Charge limit reached before saving the next Redfin valuation result.".to_owned(),
            ),
        });
    }

    runtime.push_dataset_item(item).await?;
    let run_id = runtime.actor_run_id().unwrap_or("local");
    let events = [
        REDFIN_VALUATION_RESULT_CHARGE_EVENT,
        DEFAULT_DATASET_ITEM_EVENT,
    ];
    let mut charged_count = 0;
    let mut event_charge_limit_reached = false;
    for event_name in events {
        let idempotency_key = format!("redfin-valuation-{run_id}-{request_index}-{event_name}");
        let result = manager
            .charge(runtime, event_name, &idempotency_key)
            .await?;
        charged_count += result.charged_count;
        event_charge_limit_reached |= result.event_charge_limit_reached;
    }

    if event_charge_limit_reached && charged_count < 1 {
        return Ok(PushResult {
            saved: false,
            status_message: Some(
                "Charge limit reached before saving the next Redfin valuation result.".to_owned(),
            ),
        });
    }
    Ok(PushResult {
        saved: charged_count >= 1,
        status_message: None,
    })
}

fn env_bool(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true"))
        .unwrap_or(false)
}

fn max_charge_from_env() -> f64 {
    std::env::var("ACTOR_MAX_TOTAL_CHARGE_USD")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value != 0.0)
        .unwrap_or(f64::INFINITY)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_with_budget(budget: f64, charged: u64) -> Value {
        serde_json::json!({
            "pricingInfo": {
                "pricingModel":"PAY_PER_EVENT",
                "pricingPerEvent":{"actorChargeEvents":{
                    "valuation-result":{"eventPriceUsd":0.0005,"eventTitle":"Valuation result"},
                    "apify-default-dataset-item":{"eventPriceUsd":0.0002,"eventTitle":"Default dataset item"}
                }}
            },
            "chargedEventCounts":{"valuation-result":charged},
            "options":{"maxTotalChargeUsd":budget}
        })
    }

    #[test]
    fn detects_exhausted_budget_before_fetching_and_includes_saved_count() {
        let run = run_with_budget(0.001, 2);
        let manager =
            ChargingManager::from_values(false, false, false, &Value::Null, &Value::Null, 10.0)
                .unwrap();
        assert!(!manager.is_pay_per_event);
        assert_eq!(manager.get_charge_limit_status(4, 1), None);

        let manager = ChargingManager::from_values(
            true,
            false,
            false,
            &run["pricingInfo"],
            &run["chargedEventCounts"],
            0.001,
        )
        .unwrap();
        assert_eq!(
            manager.get_charge_limit_status(4, 1).as_deref(),
            Some("Charge limit reached before fetching Redfin valuation 2; 4 valuation result(s) were saved."),
        );
    }

    #[test]
    fn counts_existing_event_spend_and_default_dataset_event_costs() {
        let run = run_with_budget(0.003, 2);
        let mut manager = ChargingManager::from_values(
            true,
            false,
            false,
            &run["pricingInfo"],
            &run["chargedEventCounts"],
            0.003,
        )
        .unwrap();
        assert_eq!(
            manager.calculate_max_event_charge_count_within_limit(
                REDFIN_VALUATION_RESULT_CHARGE_EVENT
            ),
            4.0
        );
        assert!(manager.can_push_default_dataset_item());
        let result = manager.record_charge(REDFIN_VALUATION_RESULT_CHARGE_EVENT);
        assert_eq!(result.charged_count, 1);
        assert_eq!(result.event_name, REDFIN_VALUATION_RESULT_CHARGE_EVENT);
        assert_eq!(
            manager.calculate_max_event_charge_count_within_limit(
                REDFIN_VALUATION_RESULT_CHARGE_EVENT
            ),
            3.0
        );
    }

    #[test]
    fn uses_local_one_dollar_test_events_and_preserves_non_ppe_mode() {
        let mut manager =
            ChargingManager::from_values(false, true, false, &Value::Null, &Value::Null, 3.0)
                .unwrap();
        assert!(manager.is_pay_per_event);
        assert_eq!(
            manager.calculate_max_event_charge_count_within_limit(
                REDFIN_VALUATION_RESULT_CHARGE_EVENT
            ),
            3.0
        );
        assert_eq!(
            manager
                .record_charge(REDFIN_VALUATION_RESULT_CHARGE_EVENT)
                .charged_count,
            1
        );
        assert_eq!(
            manager
                .record_charge(DEFAULT_DATASET_ITEM_EVENT)
                .charged_count,
            1
        );
        assert_eq!(
            manager.calculate_max_event_charge_count_within_limit(
                REDFIN_VALUATION_RESULT_CHARGE_EVENT
            ),
            1.0
        );
    }

    #[test]
    fn reads_run_pricing_and_charged_counts() {
        let run = run_with_budget(0.003, 2);
        let manager = ChargingManager::from_values(
            true,
            false,
            false,
            &run["pricingInfo"],
            &run["chargedEventCounts"],
            run["options"]["maxTotalChargeUsd"].as_f64().unwrap(),
        )
        .unwrap();
        assert!(manager.is_pay_per_event);
        assert_eq!(manager.total_charged_amount(), 0.001);
    }
}
