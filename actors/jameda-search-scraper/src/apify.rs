use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Method, Response, StatusCode};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    env,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
pub const DOCTOR_RESULT_CHARGE_EVENT: &str = "doctor-result";
pub const DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";
static CHARGE_ID_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub struct ApifyConfig {
    pub apify_api_base: Url,
    pub apify_token: String,
    pub key_value_store_id: String,
    pub dataset_id: String,
    pub actor_run_id: String,
    pub input_key: String,
}

impl ApifyConfig {
    pub fn from_env() -> Result<Self> {
        let apify_api_base = base_url_from_env(
            "APIFY_API_PUBLIC_BASE_URL",
            &env::var("APIFY_API_BASE_URL").unwrap_or_else(|_| APIFY_API_DEFAULT.to_owned()),
        )?;
        Ok(Self {
            apify_api_base,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(std::iter::once("v2").chain(segments.iter().copied()));
    Ok(url)
}

pub struct ApifyClient {
    http: Client,
    config: ApifyConfig,
}

impl ApifyClient {
    pub fn new(config: ApifyConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("Could not create Apify HTTP client")?;
        Ok(Self { http, config })
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.http
            .request(method, url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "Apify INPUT request").await?;
        Ok(Some(
            response
                .json()
                .await
                .context("Apify INPUT record is not valid JSON")?,
        ))
    }

    pub async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["actor-runs", &self.config.actor_run_id])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let response = successful_response(response, "Apify run pricing request").await?;
        response
            .json()
            .await
            .context("Apify run pricing response is not valid JSON")
    }

    pub async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        successful_response(response, "Apify dataset write").await?;
        Ok(())
    }

    pub async fn push_charged_items(
        &self,
        items: &[Value],
        budget: &mut PpeBudget,
    ) -> Result<PushChargedItemsResult> {
        if items.is_empty() {
            return Ok(PushChargedItemsResult {
                saved_count: 0,
                event_charge_limit_reached: false,
            });
        }
        let decision = budget.plan_charge(DOCTOR_RESULT_CHARGE_EVENT, items.len())?;
        if decision.charged_count == 0 {
            return Ok(PushChargedItemsResult {
                saved_count: 0,
                event_charge_limit_reached: decision.event_charge_limit_reached,
            });
        }

        self.push_dataset_items(&items[..decision.charged_count])
            .await?;
        budget.record_charge(DATASET_ITEM_CHARGE_EVENT, decision.charged_count)?;

        let url = self.endpoint(&["actor-runs", &self.config.actor_run_id, "charge"])?;
        let idempotency_key = charge_idempotency_key(&self.config.actor_run_id);
        let response = self
            .request(Method::POST, url)
            .header("idempotency-key", idempotency_key)
            .json(&json!({
                "eventName": DOCTOR_RESULT_CHARGE_EVENT,
                "count": decision.charged_count
            }))
            .send()
            .await
            .context("Apify result charge request failed")?;
        successful_response(response, "Apify result charge request").await?;

        budget.record_charge(DOCTOR_RESULT_CHARGE_EVENT, decision.charged_count)?;
        Ok(PushChargedItemsResult {
            saved_count: decision.charged_count,
            event_charge_limit_reached: decision.event_charge_limit_reached,
        })
    }

    pub async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .request(Method::PUT, url)
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        successful_response(response, "Apify OUTPUT write").await?;
        Ok(())
    }

    pub async fn set_terminal_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["actor-runs", &self.config.actor_run_id])?;
        let response = self
            .request(Method::PUT, url)
            .json(&json!({
                "runId": self.config.actor_run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true
            }))
            .send()
            .await
            .context("Apify terminal status update failed")?;
        successful_response(response, "Apify terminal status update").await?;
        Ok(())
    }
}

fn charge_idempotency_key(actor_run_id: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let sequence = CHARGE_ID_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{actor_run_id}-{timestamp}-{sequence}")
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.is_empty() {
        format!("HTTP {}", status.as_u16())
    } else {
        body
    };
    bail!("{operation} failed ({}): {detail}", status.as_u16());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PushChargedItemsResult {
    pub saved_count: usize,
    pub event_charge_limit_reached: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChargeDecision {
    pub charged_count: usize,
    pub event_charge_limit_reached: bool,
}

#[derive(Debug)]
pub struct PpeBudget {
    event_prices: HashMap<String, f64>,
    charged_event_counts: HashMap<String, u64>,
    max_total_charge_usd: Option<f64>,
    local_event_counts: HashMap<String, u64>,
}

pub fn is_pay_per_event(run: &Value) -> Result<bool> {
    let model = run
        .pointer("/data/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Apify run pricing model is missing"))?;
    Ok(model == "PAY_PER_EVENT")
}

impl PpeBudget {
    pub fn from_run(run: &Value) -> Result<Self> {
        if !is_pay_per_event(run)? {
            bail!("Apify run is not configured for pay-per-event pricing");
        }
        let events = run
            .pointer("/data/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::with_capacity(events.len());
        for (event_name, event) in events {
            let price = event
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .ok_or_else(|| anyhow!("Apify run did not provide the {event_name} event price"))?;
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned an invalid price for charged event {event_name}");
            }
            event_prices.insert(event_name.clone(), price);
        }

        let counts = run
            .pointer("/data/chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_event_counts = HashMap::with_capacity(counts.len());
        for (event_name, count) in counts {
            let count = count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            charged_event_counts.insert(event_name.clone(), count);
        }

        let run_charge_limit = run
            .pointer("/data/options/maxTotalChargeUsd")
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_f64()
                    .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))
            })
            .transpose()?;
        let environment_charge_limit = env::var("ACTOR_MAX_TOTAL_CHARGE_USD")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|value| {
                value
                    .parse::<f64>()
                    .with_context(|| "ACTOR_MAX_TOTAL_CHARGE_USD must be a valid number")
            })
            .transpose()?;
        if run_charge_limit
            .into_iter()
            .chain(environment_charge_limit)
            .any(|limit| !limit.is_finite() || limit < 0.0)
        {
            bail!("Apify run returned an invalid spending limit");
        }
        let run_charge_limit = run_charge_limit.filter(|limit| *limit > 0.0);
        let environment_charge_limit = environment_charge_limit.filter(|limit| *limit > 0.0);
        let max_total_charge_usd = match (run_charge_limit, environment_charge_limit) {
            (Some(run_limit), Some(environment_limit)) => Some(run_limit.min(environment_limit)),
            (Some(limit), None) | (None, Some(limit)) => Some(limit),
            (None, None) => None,
        };
        if !event_prices.contains_key(DOCTOR_RESULT_CHARGE_EVENT) {
            bail!("Apify run did not provide the {DOCTOR_RESULT_CHARGE_EVENT} event price");
        }
        if !event_prices.contains_key(DATASET_ITEM_CHARGE_EVENT) {
            bail!("Apify run did not provide the {DATASET_ITEM_CHARGE_EVENT} event price");
        }

        Ok(Self {
            event_prices,
            charged_event_counts,
            max_total_charge_usd,
            local_event_counts: HashMap::new(),
        })
    }

    pub fn plan_charge(&self, event_name: &str, requested: usize) -> Result<ChargeDecision> {
        let event_price = *self
            .event_prices
            .get(event_name)
            .ok_or_else(|| anyhow!("Apify run did not provide the {event_name} event price"))?;
        let dataset_item_price = *self
            .event_prices
            .get(DATASET_ITEM_CHARGE_EVENT)
            .ok_or_else(|| anyhow!("Apify run did not provide the {DATASET_ITEM_CHARGE_EVENT} event price"))?;
        let row_price = event_price + dataset_item_price;
        if !row_price.is_finite() {
            bail!("Apify run returned invalid per-row charges");
        }
        let Some(max_total_charge_usd) = self.max_total_charge_usd else {
            return Ok(ChargeDecision {
                charged_count: requested,
                event_charge_limit_reached: false,
            });
        };

        let mut total_spent = 0.0;
        for (name, initial_count) in &self.charged_event_counts {
            let local_count = self.local_event_counts.get(name).copied().unwrap_or(0);
            let count = initial_count
                .checked_add(local_count)
                .ok_or_else(|| anyhow!("Charged event count overflowed for {name}"))?;
            if count == 0 {
                continue;
            }
            let price = self
                .event_prices
                .get(name)
                .ok_or_else(|| anyhow!("Missing price for charged event {name}"))?;
            total_spent += price * count as f64;
        }
        for (name, local_count) in &self.local_event_counts {
            if self.charged_event_counts.contains_key(name) || *local_count == 0 {
                continue;
            }
            let price = self
                .event_prices
                .get(name)
                .ok_or_else(|| anyhow!("Missing price for charged event {name}"))?;
            total_spent += price * *local_count as f64;
        }
        if !total_spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }
        if row_price == 0.0 {
            return Ok(ChargeDecision {
                charged_count: requested,
                event_charge_limit_reached: false,
            });
        }

        let tolerance = f64::EPSILON * max_total_charge_usd.max(1.0);
        let remaining = (max_total_charge_usd - total_spent + tolerance).max(0.0);
        let affordable = (remaining / row_price).floor();
        let affordable = if affordable.is_finite() && affordable < usize::MAX as f64 {
            affordable as usize
        } else {
            usize::MAX
        };
        let charged_count = requested.min(affordable);
        let next_event_spend = total_spent + (charged_count as f64 + 1.0) * row_price;
        let event_charge_limit_reached = next_event_spend > max_total_charge_usd + tolerance;
        Ok(ChargeDecision {
            charged_count,
            event_charge_limit_reached,
        })
    }

    pub fn record_charge(&mut self, event_name: &str, count: usize) -> Result<()> {
        let count = u64::try_from(count).context("Charge count is too large")?;
        let current = self
            .local_event_counts
            .entry(event_name.to_owned())
            .or_default();
        *current = current
            .checked_add(count)
            .ok_or_else(|| anyhow!("Charged event count overflowed for {event_name}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ppe_run(max_charge: Value, initial_counts: Value) -> Value {
        json!({"data": {
            "pricingInfo": {"pricingModel":"PAY_PER_EVENT", "pricingPerEvent":{"actorChargeEvents":{
                "doctor-result":{"eventPriceUsd":0.10},
                "apify-default-dataset-item":{"eventPriceUsd":0.03},
                "apify-actor-start":{"eventPriceUsd":0.05},
                "other-result":{"eventPriceUsd":0.20}
            }}},
            "options":{"maxTotalChargeUsd":max_charge},
            "chargedEventCounts":initial_counts
        }})
    }

    #[test]
    fn pay_per_event_budget_accounts_for_all_charges_and_partial_results() {
        let run = ppe_run(
            json!(0.52),
            json!({
                "apify-actor-start":1,
                "apify-default-dataset-item":1,
                "other-result":1
            }),
        );
        assert!(is_pay_per_event(&run).unwrap());
        let mut budget = PpeBudget::from_run(&run).unwrap();
        assert_eq!(
            budget.plan_charge(DOCTOR_RESULT_CHARGE_EVENT, 5).unwrap(),
            ChargeDecision {
                charged_count: 1,
                event_charge_limit_reached: true
            }
        );
        budget.record_charge(DOCTOR_RESULT_CHARGE_EVENT, 1).unwrap();
        budget
            .record_charge(DATASET_ITEM_CHARGE_EVENT, 1)
            .unwrap();
        assert_eq!(
            budget.plan_charge(DOCTOR_RESULT_CHARGE_EVENT, 1).unwrap(),
            ChargeDecision {
                charged_count: 0,
                event_charge_limit_reached: true
            }
        );
    }

    #[test]
    fn exact_budget_exhaustion_is_reported_even_when_every_requested_item_fits() {
        let run = ppe_run(json!(0.31), json!({"apify-actor-start":1}));
        let budget = PpeBudget::from_run(&run).unwrap();
        assert_eq!(
            budget.plan_charge(DOCTOR_RESULT_CHARGE_EVENT, 3).unwrap(),
            ChargeDecision {
                charged_count: 2,
                event_charge_limit_reached: true
            }
        );
    }

    #[test]
    fn absent_null_and_zero_run_limits_are_unlimited() {
        let mut runs = vec![
            ppe_run(Value::Null, json!({})),
            ppe_run(json!(0.0), json!({})),
        ];
        let mut absent_limit = ppe_run(json!(1.0), json!({}));
        absent_limit["data"]["options"]
            .as_object_mut()
            .unwrap()
            .remove("maxTotalChargeUsd");
        runs.push(absent_limit);

        for run in runs {
            let budget = PpeBudget::from_run(&run).unwrap();
            assert_eq!(
                budget.plan_charge(DOCTOR_RESULT_CHARGE_EVENT, 4).unwrap(),
                ChargeDecision {
                    charged_count: 4,
                    event_charge_limit_reached: false
                }
            );
        }
    }

    #[test]
    fn zero_price_events_and_unlimited_runs_charge_all_requested_items() {
        let mut free_run = ppe_run(Value::Null, json!({}));
        free_run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]["doctor-result"]
            ["eventPriceUsd"] = json!(0.0);
        let free_budget = PpeBudget::from_run(&free_run).unwrap();
        assert_eq!(
            free_budget
                .plan_charge(DOCTOR_RESULT_CHARGE_EVENT, 4)
                .unwrap(),
            ChargeDecision {
                charged_count: 4,
                event_charge_limit_reached: false
            }
        );

        let unlimited_budget = PpeBudget::from_run(&ppe_run(Value::Null, json!({}))).unwrap();
        assert_eq!(
            unlimited_budget
                .plan_charge(DOCTOR_RESULT_CHARGE_EVENT, 4)
                .unwrap(),
            ChargeDecision {
                charged_count: 4,
                event_charge_limit_reached: false
            }
        );
    }

    #[test]
    fn invalid_pricing_is_rejected_before_charging() {
        let mut run = ppe_run(json!(1.0), json!({}));
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove(DOCTOR_RESULT_CHARGE_EVENT);
        assert!(PpeBudget::from_run(&run)
            .unwrap_err()
            .to_string()
            .contains("doctor-result"));

        let mut missing_dataset_item_price = ppe_run(json!(1.0), json!({}));
        missing_dataset_item_price["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove("apify-default-dataset-item");
        assert!(PpeBudget::from_run(&missing_dataset_item_price)
            .unwrap_err()
            .to_string()
            .contains("apify-default-dataset-item"));
        assert!(!is_pay_per_event(
            &json!({"data":{"pricingInfo":{"pricingModel":"PRICE_PER_DATASET_ITEM"}}})
        )
        .unwrap());
    }
}
