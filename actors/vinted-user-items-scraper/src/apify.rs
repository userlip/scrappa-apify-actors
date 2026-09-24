use std::{collections::HashMap, env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use url::Url;

pub const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const STATUS_MESSAGE_TIMEOUT: Duration = Duration::from_secs(1);

pub struct ActorConfig {
    pub apify_token: String,
    pub key_value_store_id: String,
    pub dataset_id: String,
    pub actor_run_id: String,
    pub input_key: String,
    pub scrappa_api_key: String,
    pub apify_base: String,
    pub scrappa_base: String,
}

impl ActorConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|key| !key.is_empty())
                .unwrap_or_else(|| "INPUT".to_owned()),
            scrappa_api_key: required_env("SCRAPPA_API_KEY")?,
            apify_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", "https://api.apify.com"),
            scrappa_base: env_or_default("SCRAPPA_API_BASE_URL", "https://scrappa.co/api"),
        })
    }
}

pub struct ApifyClient {
    client: Client,
    config: ActorConfig,
}

impl ApifyClient {
    pub fn new(config: ActorConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .context("Could not create Apify HTTP client")?;
        Ok(Self { client, config })
    }

    pub async fn read_input(&self) -> Result<Value> {
        let url = apify_url(
            &self.config.apify_base,
            &[
                "v2",
                "key-value-stores",
                &self.config.key_value_store_id,
                "records",
                &self.config.input_key,
            ],
        )?;
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.config.apify_token)
            .send()
            .await
            .context("Could not read Apify INPUT")?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Value::Null);
        }

        ensure_success(response, "Apify INPUT")
            .await?
            .json()
            .await
            .context("Could not parse Apify INPUT JSON")
    }

    pub async fn read_pricing(&self) -> Result<ActorPricing> {
        let url = apify_url(
            &self.config.apify_base,
            &["v2", "actor-runs", &self.config.actor_run_id],
        )?;
        let run = read_json_record(
            &self.client,
            &url,
            &self.config.apify_token,
            "Apify run pricing",
        )
        .await?;
        ActorPricing::from_run(&run)
    }

    pub async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = apify_url(
            &self.config.apify_base,
            &["v2", "datasets", &self.config.dataset_id, "items"],
        )?;
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .json(items)
            .send()
            .await
            .context("Could not publish Apify dataset items")?;
        ensure_success(response, "Apify dataset item publication")
            .await?
            .bytes()
            .await
            .context("Could not finish Apify dataset item publication")?;
        Ok(())
    }

    pub async fn charge_event(
        &self,
        event_name: &str,
        count: usize,
        idempotency_key: &str,
    ) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let url = apify_url(
            &self.config.apify_base,
            &["v2", "actor-runs", &self.config.actor_run_id, "charge"],
        )?;
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header("idempotency-key", idempotency_key)
            .json(&serde_json::json!({
                "eventName": event_name,
                "count": count,
            }))
            .send()
            .await
            .context("Could not charge Apify event")?;
        ensure_success(response, "Apify event charge")
            .await?
            .bytes()
            .await
            .context("Could not finish Apify event charge")?;
        Ok(())
    }

    pub async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = apify_url(
            &self.config.apify_base,
            &["v2", "actor-runs", &self.config.actor_run_id],
        )?;
        let response = self
            .client
            .put(url)
            .bearer_auth(&self.config.apify_token)
            .json(&serde_json::json!({
                "runId": self.config.actor_run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true,
            }))
            .timeout(STATUS_MESSAGE_TIMEOUT)
            .send()
            .await
            .context("Could not set Apify Actor status message")?;
        ensure_success(response, "Apify Actor status message update")
            .await?
            .bytes()
            .await
            .context("Could not finish Apify Actor status message update")?;
        Ok(())
    }

    pub fn actor_run_id(&self) -> &str {
        &self.config.actor_run_id
    }

    pub fn scrappa_base(&self) -> &str {
        &self.config.scrappa_base
    }

    pub fn scrappa_api_key(&self) -> &str {
        &self.config.scrappa_api_key
    }
}

pub async fn set_failure_status_message_from_env(message: &str) -> Result<()> {
    let (Ok(token), Ok(actor_run_id)) = (env::var("APIFY_TOKEN"), env::var("ACTOR_RUN_ID")) else {
        return Ok(());
    };
    let apify_base = env_or_default("APIFY_API_PUBLIC_BASE_URL", "https://api.apify.com");
    let url = apify_url(&apify_base, &["v2", "actor-runs", &actor_run_id])?;
    let client = Client::builder()
        .timeout(STATUS_MESSAGE_TIMEOUT)
        .build()
        .context("Could not create Apify HTTP client for failure status message")?;
    let response = client
        .put(url)
        .bearer_auth(token)
        .json(&serde_json::json!({
            "runId": actor_run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true,
        }))
        .send()
        .await
        .context("Could not set Apify Actor failure status message")?;
    ensure_success(response, "Apify Actor failure status message update")
        .await?
        .bytes()
        .await
        .context("Could not finish Apify Actor failure status message update")?;
    Ok(())
}

#[derive(Debug)]
pub struct ActorPricing {
    pub is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    charged_event_counts: HashMap<String, u64>,
}

impl ActorPricing {
    pub fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let is_pay_per_event = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self {
                is_pay_per_event,
                max_total_charge_usd: f64::INFINITY,
                event_prices: HashMap::new(),
                charged_event_counts: HashMap::new(),
            });
        }

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|amount| amount.is_finite() && *amount >= 0.0)
            .unwrap_or(f64::INFINITY);
        let event_prices = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .map(|events| {
                events
                    .iter()
                    .filter_map(|(name, event)| {
                        event
                            .get("eventPriceUsd")
                            .and_then(Value::as_f64)
                            .filter(|price| price.is_finite() && *price >= 0.0)
                            .map(|price| (name.clone(), price))
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .map(|counts| {
                counts
                    .iter()
                    .filter_map(|(name, count)| count.as_u64().map(|count| (name.clone(), count)))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        })
    }

    pub fn limit_dataset_items(&self, requested: usize, event_name: &str) -> usize {
        if !self.is_pay_per_event || requested == 0 {
            return requested;
        }
        let combined_price =
            self.event_price(event_name) + self.event_price(DEFAULT_DATASET_ITEM_EVENT);
        if combined_price <= 0.0 {
            return requested;
        }

        let available = self.max_charges_by_price(combined_price);
        if available >= requested {
            return requested;
        }
        available
    }

    pub fn record_dataset_items(&mut self, count: usize) {
        if self.is_pay_per_event {
            *self
                .charged_event_counts
                .entry(DEFAULT_DATASET_ITEM_EVENT.to_owned())
                .or_default() += count as u64;
        }
    }

    pub fn record_event_charge(&mut self, event_name: &str, count: usize) {
        if self.is_pay_per_event {
            *self
                .charged_event_counts
                .entry(event_name.to_owned())
                .or_default() += count as u64;
        }
    }

    pub fn event_is_exhausted(&self, event_name: &str) -> bool {
        self.is_pay_per_event && self.max_charges_by_price(self.event_price(event_name)) == 0
    }

    pub fn event_is_configured(&self, event_name: &str) -> bool {
        self.event_prices.contains_key(event_name)
    }

    fn event_price(&self, event_name: &str) -> f64 {
        self.event_prices.get(event_name).copied().unwrap_or(0.0)
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_event_counts
            .iter()
            .map(|(event_name, count)| self.event_price(event_name) * *count as f64)
            .sum::<f64>();
        (total * 1_000_000.0).round() / 1_000_000.0
    }

    fn max_charges_by_price(&self, price: f64) -> usize {
        if price <= 0.0 || self.max_total_charge_usd.is_infinite() {
            return usize::MAX;
        }
        let unrounded = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        let rounded = (unrounded * 10_000.0).round() / 10_000.0;
        if !rounded.is_finite() || rounded <= 0.0 {
            return 0;
        }
        rounded.floor().min(usize::MAX as f64) as usize
    }

    #[cfg(test)]
    fn current_total(&self) -> f64 {
        self.total_charged_amount()
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{name} environment variable is not set"))
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn apify_url(base: &str, segments: &[&str]) -> Result<Url> {
    let mut url = Url::parse(base.trim_end_matches('/'))
        .with_context(|| format!("Invalid API base URL: {base}"))?;
    url.set_query(None);
    url.set_fragment(None);
    let mut path = url
        .path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot be a base: {base}"))?;
    path.pop_if_empty();
    for segment in segments {
        path.push(segment);
    }
    drop(path);
    Ok(url)
}

async fn read_json_record(client: &Client, url: &Url, token: &str, label: &str) -> Result<Value> {
    let response = client.get(url.clone()).bearer_auth(token).send().await?;
    ensure_success(response, label)
        .await?
        .json()
        .await
        .with_context(|| format!("Could not parse {label} JSON"))
}

async fn ensure_success(response: Response, label: &str) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    let detail = if body.is_empty() {
        status.to_string()
    } else {
        format!("{status}: {body}")
    };
    bail!("{label} failed: {detail}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_pay_per_event_prices_spend_and_limit() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "user-item-result": {"eventPriceUsd": 0.00025},
                            "apify-default-dataset-item": {"eventPriceUsd": 0.00005},
                            "apify-actor-start": {"eventPriceUsd": 0.0001}
                        }
                    }
                },
                "chargedEventCounts": {
                    "apify-actor-start": 1,
                    "apify-default-dataset-item": 0
                },
                "options": {"maxTotalChargeUsd": 0.0007}
            }
        });
        let mut pricing = ActorPricing::from_run(&run).unwrap();
        assert!(pricing.is_pay_per_event);
        assert_eq!(pricing.current_total(), 0.0001);
        assert_eq!(pricing.limit_dataset_items(5, "user-item-result"), 2);

        pricing.record_dataset_items(2);
        pricing.record_event_charge("user-item-result", 2);
        assert_eq!(pricing.current_total(), 0.0007);
        assert!(pricing.event_is_exhausted("user-item-result"));
        assert!(pricing.event_is_exhausted(DEFAULT_DATASET_ITEM_EVENT));
    }

    #[test]
    fn allows_all_rows_for_non_ppe_and_zero_price_events() {
        let standard = ActorPricing::from_run(&json!({
            "data":{"pricingInfo":{"pricingModel":"PAY_PER_RESULT"}}
        }))
        .unwrap();
        assert!(!standard.is_pay_per_event);
        assert_eq!(standard.limit_dataset_items(20, "user-item-result"), 20);

        let free_ppe = ActorPricing::from_run(&json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {}}
                },
                "chargedEventCounts": {},
                "options": {"maxTotalChargeUsd": 0.0001}
            }
        }))
        .unwrap();
        assert_eq!(free_ppe.limit_dataset_items(20, "user-item-result"), 20);
        assert!(!free_ppe.event_is_exhausted("user-item-result"));
    }

    #[test]
    fn trims_a_batch_to_available_budget_without_an_overlimit_probe_row() {
        let mut pricing = ActorPricing::from_run(&json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "user-item-result": {"eventPriceUsd": 0.25},
                            "apify-default-dataset-item": {"eventPriceUsd": 0.05}
                        }
                    }
                },
                "chargedEventCounts": {},
                "options": {"maxTotalChargeUsd": 0.35}
            }
        }))
        .unwrap();
        assert_eq!(pricing.limit_dataset_items(4, "user-item-result"), 1);
        pricing.record_dataset_items(1);
        pricing.record_event_charge("user-item-result", 1);
        assert_eq!(pricing.limit_dataset_items(4, "user-item-result"), 0);
    }

    #[test]
    fn zero_spending_cap_allows_no_priced_dataset_rows() {
        let pricing = ActorPricing::from_run(&json!({
            "data": {
                "pricingInfo": {
                    "pricingModel":"PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "user-item-result": {"eventPriceUsd":0.001},
                            "apify-default-dataset-item": {"eventPriceUsd":0.0003}
                        }
                    }
                },
                "chargedEventCounts": {},
                "options": {"maxTotalChargeUsd": 0}
            }
        }))
        .unwrap();
        assert_eq!(pricing.limit_dataset_items(20, "user-item-result"), 0);
    }

    #[test]
    fn fully_spent_start_charge_leaves_no_dataset_capacity() {
        let pricing = ActorPricing::from_run(&json!({
            "data": {
                "pricingInfo": {
                    "pricingModel":"PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "user-item-result": {"eventPriceUsd":0.001},
                            "apify-default-dataset-item": {"eventPriceUsd":0.0003},
                            "apify-actor-start": {"eventPriceUsd":0.0015}
                        }
                    }
                },
                "chargedEventCounts": {"apify-actor-start":1},
                "options": {"maxTotalChargeUsd":0.0015}
            }
        }))
        .unwrap();
        assert_eq!(pricing.limit_dataset_items(20, "user-item-result"), 0);
    }
}
