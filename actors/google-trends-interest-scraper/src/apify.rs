use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Value};

use crate::config::{endpoint_url, Config};

pub const TIMELINE_POINT_CHARGE_EVENT: &str = "timeline-point";
pub const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("Failed to read {operation} response"))?;
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "{operation} failed with {} {reason}{detail}",
            status.as_u16()
        );
    }
    serde_json::from_str(&body).with_context(|| format!("{operation} returned invalid JSON"))
}

async fn ensure_success(response: Response, operation: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    bail!(
        "{operation} failed with {} {reason}{detail}",
        status.as_u16()
    );
}

pub async fn get_input(client: &Client, config: &Config) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base,
        &[
            "v2",
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .context("Apify INPUT request failed")?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(Value::Null);
    }
    response_json(response, "Apify INPUT request").await
}

pub async fn get_actor_run(client: &Client, config: &Config) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .context("Apify actor run pricing request failed")?;
    response_json(response, "Apify actor run pricing request").await
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChargeResult {
    pub event_charge_limit_reached: bool,
    pub charged_count: usize,
}

impl ChargeResult {
    pub fn merge(self, other: Self) -> Self {
        Self {
            event_charge_limit_reached: self.event_charge_limit_reached
                || other.event_charge_limit_reached,
            charged_count: self.charged_count.saturating_add(other.charged_count),
        }
    }
}

pub struct PpeBudget {
    pub prices: HashMap<String, f64>,
    pub charged_counts: HashMap<String, u64>,
    pub max_total_charge_usd: f64,
}

impl PpeBudget {
    pub fn from_actor_run(run_response: &Value) -> Result<Option<Self>> {
        let run = run_response.get("data").unwrap_or(run_response);
        let Some(pricing_info) = run.get("pricingInfo") else {
            return Ok(None);
        };
        if pricing_info.get("pricingModel").and_then(Value::as_str) != Some("PAY_PER_EVENT") {
            return Ok(None);
        }

        let events = pricing_info
            .pointer("/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .context("Actor run charge-event pricing is missing or invalid")?;
        let mut prices = HashMap::new();
        for (name, event) in events {
            let price = event
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if !price.is_finite() || price < 0.0 {
                bail!("Actor run charge-event price is invalid for {name}");
            }
            prices.insert(name.clone(), price);
        }

        let charged_counts = run
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .context("Actor run charged event counts are missing or invalid")?
            .iter()
            .map(|(name, count)| {
                count
                    .as_u64()
                    .map(|count| (name.clone(), count))
                    .with_context(|| format!("Actor run charged event count is invalid for {name}"))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        let configured_limit = run
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|limit| *limit != 0.0)
            .unwrap_or(f64::INFINITY);
        if configured_limit < 0.0 || configured_limit.is_nan() {
            bail!("Actor run maxTotalChargeUsd is invalid");
        }

        Ok(Some(Self {
            prices,
            charged_counts,
            max_total_charge_usd: configured_limit,
        }))
    }

    fn price(&self, event_name: &str) -> f64 {
        self.prices.get(event_name).copied().unwrap_or(0.0)
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_counts
            .iter()
            .map(|(name, count)| self.price(name) * *count as f64)
            .sum::<f64>();
        (total * 1_000_000.0).round() / 1_000_000.0
    }

    fn max_charges_by_price(&self, price: f64) -> usize {
        if price == 0.0 {
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
        let rounded = (unrounded * 10_000.0).round() / 10_000.0;
        if rounded <= 0.0 {
            return 0;
        }
        rounded.floor().min(usize::MAX as f64) as usize
    }

    fn max_event_charge_count(&self, event_name: &str) -> usize {
        let price = self.price(event_name);
        if price == 0.0 {
            usize::MAX
        } else {
            self.max_charges_by_price(price)
        }
    }

    pub fn dataset_item_limit(&self, requested: usize) -> usize {
        let price_per_item =
            self.price(TIMELINE_POINT_CHARGE_EVENT) + self.price(DEFAULT_DATASET_ITEM_EVENT);
        let affordable = if price_per_item > 0.0 {
            self.max_charges_by_price(price_per_item)
        } else {
            usize::MAX
        };
        if affordable >= requested {
            return requested;
        }
        if requested > 0
            && affordable == 0
            && self.total_charged_amount() <= self.max_total_charge_usd
        {
            return 1;
        }
        affordable.min(requested)
    }

    fn charge(&mut self, event_name: &str, requested: usize) -> ChargeResult {
        let max_count = self.max_event_charge_count(event_name);
        let charged_count = if requested <= max_count {
            requested
        } else if self.total_charged_amount() <= self.max_total_charge_usd {
            max_count.saturating_add(1)
        } else {
            0
        };
        if charged_count == 0 {
            return ChargeResult {
                event_charge_limit_reached: requested > 0,
                charged_count: 0,
            };
        }
        *self
            .charged_counts
            .entry(event_name.to_owned())
            .or_default() += charged_count as u64;
        let limit_reached = self.max_event_charge_count(event_name) == 0;
        ChargeResult {
            event_charge_limit_reached: limit_reached,
            charged_count,
        }
    }
}

pub fn ppe_items_result(
    budget: &mut PpeBudget,
    requested: usize,
) -> (usize, ChargeResult, ChargeResult) {
    let kept = budget.dataset_item_limit(requested);
    if kept == 0 {
        return (
            kept,
            ChargeResult {
                event_charge_limit_reached: requested > 0,
                charged_count: 0,
            },
            ChargeResult::default(),
        );
    }
    let custom_event = budget.charge(TIMELINE_POINT_CHARGE_EVENT, kept);
    let dataset_event = budget.charge(DEFAULT_DATASET_ITEM_EVENT, kept);
    (kept, custom_event, dataset_event)
}

pub async fn push_dataset_items(client: &Client, config: &Config, items: &[Value]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let url = endpoint_url(
        &config.apify_api_base,
        &["v2", "datasets", &config.dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .json(items)
        .send()
        .await
        .context("Apify dataset write failed")?;
    ensure_success(response, "Apify dataset write").await
}

pub async fn charge_timeline_points(client: &Client, config: &Config, count: usize) -> Result<()> {
    if count == 0 {
        return Ok(());
    }
    let url = endpoint_url(
        &config.apify_api_base,
        &["v2", "actor-runs", &config.actor_run_id, "charge"],
    )?;
    let idempotency_key = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .header(
            "Idempotency-Key",
            format!(
                "{}-{}-{idempotency_key}",
                config.actor_run_id, TIMELINE_POINT_CHARGE_EVENT
            ),
        )
        .json(&json!({
            "eventName": TIMELINE_POINT_CHARGE_EVENT,
            "count": count,
        }))
        .send()
        .await
        .context("Apify timeline-point charge request failed")?;
    ensure_success(response, "Apify timeline-point charge request").await
}

pub async fn put_output(client: &Client, config: &Config, output: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base,
        &[
            "v2",
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            "OUTPUT",
        ],
    )?;
    let response = client
        .put(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .json(output)
        .send()
        .await
        .context("Apify OUTPUT write failed")?;
    ensure_success(response, "Apify OUTPUT write").await
}

pub async fn put_terminal_status_message(
    client: &Client,
    config: &Config,
    status_message: &str,
) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .put(url)
        .timeout(Duration::from_secs(1))
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .json(&json!({
            "runId": config.actor_run_id,
            "statusMessage": status_message,
            "isStatusMessageTerminal": true,
        }))
        .send()
        .await
        .context("Apify status-message update failed")?;
    ensure_success(response, "Apify status-message update").await
}
