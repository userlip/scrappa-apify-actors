use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Method, Response, StatusCode};
use serde_json::{json, Value};
use url::Url;

use crate::{
    request_params::VintedUserProfileRequest,
    runtime_budget::{retry_delay_ms, APIFY_CHARGE_MAX_ATTEMPTS, APIFY_REQUEST_TIMEOUT_MS},
};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const VINTED_USER_PROFILE_RESULT_CHARGE_EVENT: &str = "user-profile-result";
const DEFAULT_DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";
pub struct ActorConfig {
    pub apify_api_base: Url,
    pub scrappa_api_base: Option<String>,
    pub apify_token: String,
    pub actor_run_id: String,
    pub key_value_store_id: String,
    pub dataset_id: String,
    pub input_key: String,
    pub scrappa_api_key: String,
}

impl ActorConfig {
    pub fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        let apify_api_base = Url::parse(&env_or_default(
            "APIFY_API_PUBLIC_BASE_URL",
            APIFY_API_DEFAULT,
        ))
        .context("APIFY_API_PUBLIC_BASE_URL must be a valid absolute URL")?;

        Ok(Self {
            apify_api_base,
            scrappa_api_base: env::var("SCRAPPA_API_BASE_URL").ok(),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChargeBudget {
    is_pay_per_event: bool,
    event_price_usd: Option<f64>,
    remaining_event_count: Option<usize>,
}

impl ChargeBudget {
    pub fn capacity(&self) -> Option<usize> {
        self.remaining_event_count
    }

    pub fn charges_profile_results(&self) -> bool {
        self.is_pay_per_event && self.event_price_usd.is_some()
    }

    pub fn event_charge_succeeded(&mut self) {
        if let Some(remaining) = &mut self.remaining_event_count {
            *remaining = remaining.saturating_sub(1);
        }
    }
}

pub fn charge_budget_from_run(run: &Value) -> Result<ChargeBudget> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let is_pay_per_event = data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        == Some("PAY_PER_EVENT");
    if !is_pay_per_event {
        return Ok(ChargeBudget {
            is_pay_per_event: false,
            event_price_usd: None,
            remaining_event_count: None,
        });
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let event_price = events
        .get(VINTED_USER_PROFILE_RESULT_CHARGE_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| {
            anyhow!(
                "Apify run did not provide the {VINTED_USER_PROFILE_RESULT_CHARGE_EVENT} event price"
            )
        })?;
    let default_dataset_item_price = events
        .get(DEFAULT_DATASET_ITEM_CHARGE_EVENT)
        .map(|event| {
            event
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    anyhow!(
                        "Apify run did not provide the {DEFAULT_DATASET_ITEM_CHARGE_EVENT} event price"
                    )
                })
        })
        .transpose()?
        .unwrap_or(0.0);
    let result_price = event_price + default_dataset_item_price;
    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .filter(|value| !value.is_null())
        .and_then(Value::as_f64)
        .unwrap_or(f64::INFINITY);
    if !event_price.is_finite() || event_price < 0.0 || max_charge.is_nan() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    if !default_dataset_item_price.is_finite()
        || default_dataset_item_price < 0.0
        || !result_price.is_finite()
    {
        bail!("Apify run returned invalid charging values");
    }

    let events_by_name = events;
    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut spent = 0.0;
    for (event_name, count) in &counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if count == 0 {
            continue;
        }
        let price = events_by_name
            .get(event_name)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }

    let remaining_event_count = if result_price == 0.0 || max_charge.is_infinite() {
        None
    } else {
        let tolerance = f64::EPSILON * max_charge.max(1.0);
        let available_charge = (max_charge - spent + tolerance).max(0.0);
        let affordable_events = (available_charge / result_price).floor();
        Some(if affordable_events.is_finite() {
            affordable_events.min(usize::MAX as f64) as usize
        } else {
            usize::MAX
        })
    };

    Ok(ChargeBudget {
        is_pay_per_event,
        event_price_usd: Some(event_price),
        remaining_event_count,
    })
}

#[derive(Clone)]
pub struct ApifyClient {
    http: Client,
    base_url: Url,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    pub fn new(config: &ActorConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_millis(APIFY_REQUEST_TIMEOUT_MS))
            .build()
            .context("Could not create Apify HTTP client")?;

        Ok(Self {
            http,
            base_url: config.apify_api_base.clone(),
            token: config.apify_token.clone(),
            actor_run_id: config.actor_run_id.clone(),
            key_value_store_id: config.key_value_store_id.clone(),
            dataset_id: config.dataset_id.clone(),
            input_key: config.input_key.clone(),
        })
    }

    fn endpoint(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot be a base URL"))?;
        path.pop_if_empty();
        path.extend(["v2"].into_iter().chain(resource.iter().copied()));
        drop(path);
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.http
            .request(method, url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .context("Failed to retrieve actor input from Apify API")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let response = require_success(response, "input retrieval").await?;
        response
            .json::<Value>()
            .await
            .context("Apify input record was not valid JSON")
            .map(Some)
    }

    pub async fn get_charge_budget(&self) -> Result<ChargeBudget> {
        let url = self.endpoint(&["actor-runs", &self.actor_run_id])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let response = require_success(response, "run pricing request").await?;
        let run = response
            .json::<Value>()
            .await
            .context("Apify run pricing response was not valid JSON")?;
        charge_budget_from_run(&run)
    }

    pub async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["datasets", &self.dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(item)
            .send()
            .await
            .context("Failed to publish dataset item to Apify API")?;
        require_success(response, "dataset item publication").await?;
        Ok(())
    }

    pub async fn charge_user_profile_result(
        &self,
        request: &VintedUserProfileRequest,
    ) -> Result<()> {
        let url = self.endpoint(&["actor-runs", &self.actor_run_id, "charge"])?;
        let idempotency_key = format!(
            "{}-{VINTED_USER_PROFILE_RESULT_CHARGE_EVENT}-{}",
            self.actor_run_id, request.index
        );

        for attempt in 0..APIFY_CHARGE_MAX_ATTEMPTS {
            let response = self
                .request(Method::POST, url.clone())
                .header("idempotency-key", &idempotency_key)
                .json(&json!({
                    "eventName": VINTED_USER_PROFILE_RESULT_CHARGE_EVENT,
                    "count": 1
                }))
                .send()
                .await;

            match response {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response)
                    if attempt + 1 < APIFY_CHARGE_MAX_ATTEMPTS
                        && retryable_apify_status(response.status()) =>
                {
                    drop(response);
                    tokio::time::sleep(Duration::from_millis(retry_delay_ms(attempt))).await;
                }
                Ok(response) => {
                    return Err(apify_error(response, "profile result charge").await);
                }
                Err(error)
                    if attempt + 1 < APIFY_CHARGE_MAX_ATTEMPTS
                        && (error.is_timeout() || error.is_connect()) =>
                {
                    tokio::time::sleep(Duration::from_millis(retry_delay_ms(attempt))).await;
                }
                Err(error) => {
                    return Err(anyhow!(
                        "Apify profile result charge request failed: {error}"
                    ));
                }
            }
        }

        unreachable!("at least one Apify charge attempt is configured")
    }

    pub async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["actor-runs", &self.actor_run_id])?;
        let response = self
            .request(Method::PUT, url)
            .json(&json!({"statusMessage": message}))
            .send()
            .await
            .context("Apify run status update failed")?;
        require_success(response, "run status update").await?;
        Ok(())
    }
}

fn retryable_apify_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
}

async fn require_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    Err(apify_error(response, operation).await)
}

async fn apify_error(response: Response, operation: &str) -> anyhow::Error {
    let status = response.status();
    let status_code = status.as_u16();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.is_empty() {
        format!("HTTP {status_code}")
    } else {
        body
    };
    anyhow!("Apify API error ({status_code}) while trying to {operation}: {detail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(max_charge: Value, charged: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "user-profile-result": {"eventPriceUsd": 0.0005},
                            "apify-actor-start": {"eventPriceUsd": 0.01}
                        }
                    }
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": charged
            }
        })
    }

    #[test]
    fn calculates_event_budget_from_all_charged_events() {
        let budget =
            charge_budget_from_run(&run(json!(0.021), json!({"apify-actor-start": 1}))).unwrap();
        assert!(budget.is_pay_per_event);
        assert_eq!(budget.capacity(), Some(22));
        let mut budget = budget;
        budget.event_charge_succeeded();
        assert_eq!(budget.capacity(), Some(21));
    }

    #[test]
    fn handles_non_ppe_runs_and_unbounded_charge_limits() {
        let free = json!({"data": {"pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM"}}});
        let free_budget = charge_budget_from_run(&free).unwrap();
        assert!(!free_budget.is_pay_per_event);
        assert_eq!(free_budget.capacity(), None);
        assert!(!free_budget.charges_profile_results());

        let unbounded = charge_budget_from_run(&run(Value::Null, json!({}))).unwrap();
        assert_eq!(unbounded.capacity(), None);
        assert!(unbounded.charges_profile_results());
    }

    #[test]
    fn accounts_for_default_dataset_item_charges_with_custom_result_charges() {
        let mut run = run(json!(0.021), json!({"apify-actor-start": 1}));
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            ["apify-default-dataset-item"] = json!({"eventPriceUsd": 0.001});

        let mut budget = charge_budget_from_run(&run).unwrap();
        assert_eq!(budget.capacity(), Some(7));
        budget.event_charge_succeeded();
        assert_eq!(budget.capacity(), Some(6));
    }

    #[test]
    fn rejects_incomplete_ppe_pricing_and_invalid_counts() {
        let missing_event = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {}}
                }
            }
        });
        assert!(charge_budget_from_run(&missing_event).is_err());
        assert!(
            charge_budget_from_run(&run(json!(1.0), json!({"apify-actor-start": -1}))).is_err()
        );
    }
}
