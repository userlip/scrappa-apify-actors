use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const APIFY_API_TIMEOUT: Duration = Duration::from_secs(60);

pub struct ApifyConfig {
    pub api_base_url: String,
    pub token: String,
    pub run_id: String,
    pub store_id: String,
    pub dataset_id: String,
    pub input_key: String,
    pub at_home: bool,
}

impl ApifyConfig {
    pub fn from_env() -> Result<Self> {
        let api_base_url =
            std::env::var("APIFY_API_PUBLIC_BASE_URL").unwrap_or_else(|_| APIFY_API_DEFAULT.into());
        let token = required_env("APIFY_TOKEN")?;
        let run_id = required_env("ACTOR_RUN_ID")?;
        let store_id = required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?;
        let dataset_id = required_env("ACTOR_DEFAULT_DATASET_ID")?;
        let input_key = std::env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".into());
        let at_home = std::env::var("APIFY_IS_AT_HOME")
            .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));

        Ok(Self {
            api_base_url,
            token,
            run_id,
            store_id,
            dataset_id,
            input_key,
            at_home,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = std::env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

pub struct ApifyClient {
    http: Client,
    base_url: Url,
    config: ApifyConfig,
}

impl ApifyClient {
    pub fn new(config: ApifyConfig) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(APIFY_API_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(&config.api_base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            config,
        })
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
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
            .bearer_auth(&self.config.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.config.store_id,
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
        let response = successful_response(response, "fetch Actor input").await?;
        let input = response
            .json()
            .await
            .context("Actor input record is not valid JSON")?;
        Ok(Some(input))
    }

    pub async fn get_pricing(&self) -> Result<ActorPricing> {
        if !self.config.at_home {
            return Ok(ActorPricing::not_pay_per_event());
        }
        let url = self.resource_url(&["actor-runs", &self.config.run_id])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = successful_response(response, "fetch Actor run pricing")
            .await?
            .json::<Value>()
            .await
            .context("Apify run pricing response is not valid JSON")?;
        let data = run.get("data").unwrap_or(&run);
        ActorPricing::from_run_data(data)
    }

    pub async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.resource_url(&["datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(item)
            .send()
            .await
            .context("Failed to store quote item in the default dataset")?;
        successful_response(response, "store quote item").await?;
        Ok(())
    }

    pub async fn charge_quote_result(&self) -> Result<()> {
        let url = self.resource_url(&["actor-runs", &self.config.run_id, "charge"])?;
        let response = self
            .request(Method::POST, url)
            .header(
                "idempotency-key",
                format!("{}-google-finance-quote-result", self.config.run_id),
            )
            .json(&json!({"eventName": "quote-result", "count": 1}))
            .send()
            .await
            .context("Apify quote-result charge request failed")?;
        successful_response(response, "charge quote-result event").await?;
        Ok(())
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.config.store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .request(Method::PUT, url)
            .json(output)
            .send()
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }
}

#[derive(Default)]
pub struct ActorPricing {
    pub is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    configured_events: HashSet<String>,
    charged_event_counts: HashMap<String, u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DatasetItemChargePlan {
    pub keep_item: bool,
    pub charge_quote_result: bool,
    pub expected_charged_count: u64,
}

impl ActorPricing {
    fn not_pay_per_event() -> Self {
        Self {
            max_total_charge_usd: f64::INFINITY,
            ..Self::default()
        }
    }

    fn from_run_data(data: &Value) -> Result<Self> {
        let pricing_info = data.get("pricingInfo");
        if pricing_info
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(Self::not_pay_per_event());
        }

        let event_definitions = pricing_info
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        let mut configured_events = HashSet::new();
        for (name, event) in event_definitions {
            configured_events.insert(name.clone());
            if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
                if !price.is_finite() || price < 0.0 {
                    bail!("Apify run returned an invalid price for event {name}");
                }
                event_prices.insert(name.clone(), price);
            }
        }

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|value| *value != 0.0)
            .unwrap_or(f64::INFINITY);
        if max_total_charge_usd.is_nan() || max_total_charge_usd < 0.0 {
            bail!("Apify run returned an invalid spending limit");
        }

        let mut charged_event_counts = HashMap::new();
        if let Some(counts) = data.get("chargedEventCounts").and_then(Value::as_object) {
            for (event, count) in counts {
                let count = count
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {event}"))?;
                charged_event_counts.insert(event.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event: true,
            max_total_charge_usd,
            event_prices,
            configured_events,
            charged_event_counts,
        })
    }

    pub fn plan_default_dataset_item(&self, event_name: &str) -> DatasetItemChargePlan {
        if !self.is_pay_per_event {
            return DatasetItemChargePlan {
                keep_item: true,
                charge_quote_result: false,
                expected_charged_count: 0,
            };
        }

        let item_price = self.event_prices.get(event_name).copied().unwrap_or(0.0)
            + self
                .event_prices
                .get(DEFAULT_DATASET_ITEM_EVENT)
                .copied()
                .unwrap_or(0.0);
        let total_charged = self.total_charged_amount();
        let affordable_count = if item_price > 0.0 {
            self.max_charges_by_price(item_price, total_charged)
        } else {
            u64::MAX
        };
        let keep_item = affordable_count >= 1
            || (affordable_count == 0 && total_charged <= self.max_total_charge_usd);

        DatasetItemChargePlan {
            keep_item,
            charge_quote_result: keep_item && self.configured_events.contains(event_name),
            expected_charged_count: if keep_item { 2 } else { 0 },
        }
    }

    fn total_charged_amount(&self) -> f64 {
        let total = self
            .charged_event_counts
            .iter()
            .map(|(event, count)| {
                self.event_prices.get(event).copied().unwrap_or(0.0) * *count as f64
            })
            .sum::<f64>();
        if total.is_finite() {
            (total * 1_000_000.0).round() / 1_000_000.0
        } else {
            total
        }
    }

    fn max_charges_by_price(&self, price: f64, total_charged: f64) -> u64 {
        let count = (self.max_total_charge_usd - total_charged) / price;
        if !count.is_finite() {
            return if count.is_sign_positive() {
                u64::MAX
            } else {
                0
            };
        }
        let rounded = (count * 10_000.0).round() / 10_000.0;
        rounded.floor().max(0.0).min(u64::MAX as f64) as u64
    }
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let status_code = status.as_u16();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.is_empty() {
        format!("HTTP {status_code}")
    } else {
        body
    };
    bail!("Apify API error ({status_code}) while trying to {operation}: {detail}");
}

#[cfg(test)]
mod tests {
    use super::{ActorPricing, DatasetItemChargePlan};
    use serde_json::json;

    fn pricing(max_total: f64, counts: serde_json::Value) -> ActorPricing {
        ActorPricing::from_run_data(&json!({
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "quote-result": {"eventPriceUsd": 0.001},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0002},
                    "other-result": {"eventPriceUsd": 0.01}
                }}
            },
            "chargedEventCounts": counts,
            "options": {"maxTotalChargeUsd": max_total}
        }))
        .unwrap()
    }

    #[test]
    fn allows_a_result_when_the_combined_custom_and_dataset_charges_fit() {
        let plan =
            pricing(0.01, json!({"other-result": 1})).plan_default_dataset_item("quote-result");
        assert_eq!(
            plan,
            DatasetItemChargePlan {
                keep_item: true,
                charge_quote_result: true,
                expected_charged_count: 2,
            }
        );
    }

    #[test]
    fn refuses_a_result_when_the_run_is_already_over_budget() {
        let plan =
            pricing(0.001, json!({"other-result": 1})).plan_default_dataset_item("quote-result");
        assert_eq!(
            plan,
            DatasetItemChargePlan {
                keep_item: false,
                charge_quote_result: false,
                expected_charged_count: 0,
            }
        );
    }

    #[test]
    fn mirrors_the_sdk_boundary_rule_when_no_event_is_affordable_yet_spend_is_at_limit() {
        let plan =
            pricing(0.0012, json!({"quote-result": 1})).plan_default_dataset_item("quote-result");
        assert!(plan.keep_item);
        assert!(plan.charge_quote_result);
        assert_eq!(plan.expected_charged_count, 2);
    }

    #[test]
    fn writes_items_without_custom_charges_for_non_ppe_runs() {
        let pricing = ActorPricing::from_run_data(&json!({
            "pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM"}
        }))
        .unwrap();
        assert_eq!(
            pricing.plan_default_dataset_item("quote-result"),
            DatasetItemChargePlan {
                keep_item: true,
                charge_quote_result: false,
                expected_charged_count: 0,
            }
        );
    }
}
