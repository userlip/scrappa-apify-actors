use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;

pub(crate) const INTRADAY_PRICE_POINT_CHARGE_EVENT: &str = "intraday-price-point";

#[derive(Default)]
pub(crate) struct ChargeBudget {
    initial_point_charges: Option<u64>,
    confirmed_point_charges: u64,
}

impl ChargeBudget {
    #[cfg(test)]
    pub(crate) fn confirmed_point_charges(&self) -> u64 {
        self.confirmed_point_charges
    }

    pub(crate) fn confirm_point_charges(&mut self, count: usize) -> Result<()> {
        let count =
            u64::try_from(count).context("Intraday price point charge count is too large")?;
        self.confirmed_point_charges = self
            .confirmed_point_charges
            .checked_add(count)
            .ok_or_else(|| anyhow!("Intraday price point charge count overflowed"))?;
        Ok(())
    }
}

pub(crate) fn affordable_point_count(
    run: &Value,
    requested: usize,
    budget: &mut ChargeBudget,
) -> Result<Option<usize>> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        return Ok(None);
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let point_price = events
        .get(INTRADAY_PRICE_POINT_CHARGE_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the intraday price point event price"))?;
    if !point_price.is_finite() || point_price < 0.0 {
        bail!("Apify run returned an invalid intraday price point event price");
    }
    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .filter(|value| !value.is_null())
        .map(|value| {
            value
                .as_f64()
                .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))
        })
        .transpose()?;
    if max_charge.is_some_and(|max_charge| !max_charge.is_finite() || max_charge < 0.0) {
        bail!("Apify run returned an invalid spending limit");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let reported_point_charges = counts
        .get(INTRADAY_PRICE_POINT_CHARGE_EVENT)
        .map(|count| {
            count.as_u64().ok_or_else(|| {
                anyhow!("Invalid charged event count for {INTRADAY_PRICE_POINT_CHARGE_EVENT}")
            })
        })
        .transpose()?
        .unwrap_or(0);
    let initial_point_charges = *budget
        .initial_point_charges
        .get_or_insert(reported_point_charges);
    let locally_confirmed = initial_point_charges
        .checked_add(budget.confirmed_point_charges)
        .ok_or_else(|| anyhow!("Intraday price point charge count overflowed"))?;

    let mut spent = 0.0;
    let mut saw_point_count = false;
    for (event_name, count) in &counts {
        let mut count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if event_name == INTRADAY_PRICE_POINT_CHARGE_EVENT {
            saw_point_count = true;
            count = count.max(locally_confirmed);
        }
        if count == 0 {
            continue;
        }
        let price = events
            .get(event_name)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        spent += price * count as f64;
    }
    if !saw_point_count && locally_confirmed > 0 {
        spent += point_price * locally_confirmed as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    let Some(max_charge) = max_charge else {
        return Ok(Some(requested));
    };
    if point_price == 0.0 {
        return Ok(Some(requested));
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let affordable = (1..=requested)
        .take_while(|count| spent + *count as f64 * point_price <= max_charge + tolerance)
        .count();
    Ok(Some(affordable))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn respects_existing_event_charges_and_partial_spending_limits() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "intraday-price-point": { "eventPriceUsd": 0.2 },
                        "another-event": { "eventPriceUsd": 0.1 }
                    }}
                },
                "chargedEventCounts": {
                    "intraday-price-point": 1,
                    "another-event": 1
                },
                "options": { "maxTotalChargeUsd": 0.75 }
            }
        });
        let mut budget = ChargeBudget::default();
        assert_eq!(
            affordable_point_count(&run, 5, &mut budget).unwrap(),
            Some(2)
        );
        budget.confirmed_point_charges = 2;
        let mut next_run = run.clone();
        next_run["data"]["chargedEventCounts"]["intraday-price-point"] = json!(1);
        assert_eq!(
            affordable_point_count(&next_run, 2, &mut budget).unwrap(),
            Some(0)
        );
    }

    #[test]
    fn skips_event_charging_during_non_pay_per_event_pricing() {
        let run =
            json!({ "data": { "pricingInfo": { "pricingModel": "PRICE_PER_DATASET_ITEM" } } });
        assert_eq!(
            affordable_point_count(&run, 3, &mut ChargeBudget::default()).unwrap(),
            None
        );
    }
}
