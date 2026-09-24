use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde_json::{Map, Value};

use crate::{apify::ApifyClient, config::Config};

pub(crate) const SEARCH_EVENT: &str = "search";
pub(crate) const RESULT_EVENT: &str = "result";
pub(crate) const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub(crate) struct ChargeBudget {
    pub(crate) event_prices: BTreeMap<String, f64>,
    pub(crate) max_total_charge_usd: f64,
    pub(crate) charged_event_counts: BTreeMap<String, u64>,
}

impl ChargeBudget {
    pub(crate) fn from_metadata(
        pricing_info: &Value,
        charged_event_counts: &Value,
        max_total_charge_usd: f64,
    ) -> Result<Self> {
        if pricing_info.get("pricingModel").and_then(Value::as_str) != Some("PAY_PER_EVENT") {
            bail!("Apify run is not configured for pay-per-event pricing");
        }

        if max_total_charge_usd.is_nan() || max_total_charge_usd < 0.0 {
            bail!("Apify run returned invalid charging values");
        }

        let event_prices_json = pricing_info
            .pointer("/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = BTreeMap::new();
        for (event_name, event) in event_prices_json {
            let price = event
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    anyhow!("Apify run did not provide a price for event {event_name}")
                })?;
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned invalid price for event {event_name}");
            }
            event_prices.insert(event_name.clone(), price);
        }

        let counts_json = match charged_event_counts.get("chargedEventCounts") {
            Some(counts) => counts,
            None => charged_event_counts,
        };
        let counts_json = counts_json
            .as_object()
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_counts = BTreeMap::new();
        for (event_name, count) in counts_json {
            let count = count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            charged_counts.insert(event_name.clone(), count);
        }

        Ok(Self {
            event_prices,
            max_total_charge_usd,
            charged_event_counts: charged_counts,
        })
    }

    pub(crate) fn affordable_count(&self, event_name: &str, requested: usize) -> Result<usize> {
        let item_price =
            self.event_prices.get(event_name).copied().ok_or_else(|| {
                anyhow!("Apify run did not provide the price for event {event_name}")
            })?;
        self.affordable_count_at_price(item_price, requested)
    }

    pub(crate) fn affordable_dataset_item_count(
        &self,
        event_name: &str,
        requested: usize,
    ) -> Result<usize> {
        let event_price =
            self.event_prices.get(event_name).copied().ok_or_else(|| {
                anyhow!("Apify run did not provide the price for event {event_name}")
            })?;
        let dataset_item_price = self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0.0);
        let item_price = event_price + dataset_item_price;
        if !item_price.is_finite() {
            bail!("Apify run returned invalid combined dataset item price");
        }
        self.affordable_count_at_price(item_price, requested)
    }

    fn affordable_count_at_price(&self, item_price: f64, requested: usize) -> Result<usize> {
        if !item_price.is_finite() || item_price < 0.0 {
            bail!("Apify run returned invalid charging values");
        }

        let mut spent = 0.0;
        for (charged_event_name, count) in &self.charged_event_counts {
            if *count == 0 {
                continue;
            }
            let price = self
                .event_prices
                .get(charged_event_name)
                .copied()
                .unwrap_or(0.0);
            spent += price * *count as f64;
        }
        if !spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        if item_price == 0.0 || self.max_total_charge_usd == f64::INFINITY {
            return Ok(requested);
        }

        let tolerance = f64::EPSILON * self.max_total_charge_usd.max(1.0);
        Ok((1..=requested)
            .take_while(|count| {
                spent + *count as f64 * item_price <= self.max_total_charge_usd + tolerance
            })
            .count())
    }

    pub(crate) async fn charge_event(
        &mut self,
        apify: &ApifyClient<'_>,
        event_name: &str,
        requested: usize,
    ) -> Result<ChargeResult> {
        if requested == 0 {
            return Ok(ChargeResult {
                charged: 0,
                event_charge_limit_reached: false,
            });
        }

        let affordable = self.affordable_count(event_name, requested)?;
        if affordable == 0 {
            return Ok(ChargeResult {
                charged: 0,
                event_charge_limit_reached: true,
            });
        }

        apify.charge(event_name, affordable).await?;
        let affordable = u64::try_from(affordable).context("Charge event count is too large")?;
        let total = self
            .charged_event_counts
            .get(event_name)
            .copied()
            .unwrap_or(0)
            .checked_add(affordable)
            .ok_or_else(|| anyhow!("Charged event count overflowed"))?;
        self.charged_event_counts
            .insert(event_name.to_owned(), total);
        let event_charge_limit_reached = self.affordable_count(event_name, 1)? == 0;

        Ok(ChargeResult {
            charged: usize::try_from(affordable).context("Charge event count is too large")?,
            event_charge_limit_reached,
        })
    }

    pub(crate) fn record_default_dataset_items(&mut self, count: usize) -> Result<()> {
        if !self.event_prices.contains_key(DEFAULT_DATASET_ITEM_EVENT) || count == 0 {
            return Ok(());
        }
        let count = u64::try_from(count).context("Dataset item count is too large")?;
        let total = self
            .charged_event_counts
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0)
            .checked_add(count)
            .ok_or_else(|| anyhow!("Charged dataset item count overflowed"))?;
        self.charged_event_counts
            .insert(DEFAULT_DATASET_ITEM_EVENT.to_owned(), total);
        Ok(())
    }
}

pub(crate) struct ChargeResult {
    pub(crate) charged: usize,
    pub(crate) event_charge_limit_reached: bool,
}

pub(crate) async fn load_charge_budget(http: &Client, config: &Config) -> Result<ChargeBudget> {
    if let (Some(pricing_info), Some(charged_counts)) = (
        config.pricing_info.as_ref(),
        config.charged_event_counts.as_ref(),
    ) {
        return ChargeBudget::from_metadata(
            pricing_info,
            charged_counts,
            config.max_total_charge_usd.unwrap_or(f64::INFINITY),
        );
    }

    let apify = ApifyClient::new(http, config);
    let run = apify.get_run().await?;
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let pricing_info = data
        .get("pricingInfo")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let empty_counts = Value::Object(Map::new());
    let charged_counts = config
        .charged_event_counts
        .as_ref()
        .or_else(|| data.get("chargedEventCounts"))
        .unwrap_or(&empty_counts);
    let max_charge = config
        .max_total_charge_usd
        .or_else(|| {
            data.pointer("/options/maxTotalChargeUsd")
                .and_then(Value::as_f64)
        })
        .unwrap_or(f64::INFINITY);

    ChargeBudget::from_metadata(pricing_info, charged_counts, max_charge)
}
