use std::{collections::BTreeMap, env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::{json, Value};

pub const INDEX_RESULT_CHARGE_EVENT: &str = "index-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const DEFAULT_APIFY_API_BASE: &str = "https://api.apify.com";

#[derive(Clone)]
pub struct ActorConfig {
    pub apify_api_base: Url,
    pub apify_token: String,
    pub key_value_store_id: String,
    pub dataset_id: String,
    pub actor_run_id: String,
    pub input_key: String,
    pub scrappa_api_key: String,
    pub scrappa_api_base: Option<String>,
}

impl ActorConfig {
    pub fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        let apify_base = env::var("APIFY_API_PUBLIC_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_APIFY_API_BASE.to_owned());
        Ok(Self {
            apify_api_base: Url::parse(&apify_base).with_context(|| {
                format!("APIFY_API_PUBLIC_BASE_URL must be a valid URL: {apify_base}")
            })?,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key,
            scrappa_api_base: env::var("SCRAPPA_API_BASE_URL").ok(),
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

#[derive(Default)]
struct ChargeBudget {
    initial_counts: Option<BTreeMap<String, u64>>,
    local_event_charges: BTreeMap<String, u64>,
    is_ppe: Option<bool>,
    run_snapshot: Option<Value>,
}

pub struct ApifyClient {
    http: Client,
    config: ActorConfig,
    budget: ChargeBudget,
}

impl ApifyClient {
    pub fn new(config: ActorConfig) -> Result<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(15))
            .build()
            .context("Could not create Apify HTTP client")?;
        Ok(Self {
            http,
            config,
            budget: ChargeBudget::default(),
        })
    }

    pub async fn get_input(&self) -> Result<Value> {
        let url = self.endpoint(&[
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .send(Method::GET, url)
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Value::Null);
        }
        response_json(response, "Apify INPUT request").await
    }

    pub async fn get_capacity(&mut self) -> Result<usize> {
        let run = match &self.budget.run_snapshot {
            Some(run) => run.clone(),
            None => {
                let run = self.get_run().await?;
                self.budget.run_snapshot = Some(run.clone());
                run
            }
        };
        let Some(capacity) =
            affordable_row_count(&run, INDEX_RESULT_CHARGE_EVENT, &mut self.budget)?
        else {
            return Ok(usize::MAX);
        };
        Ok(capacity)
    }

    pub async fn save_index(&mut self, item: &Value, capacity: usize) -> Result<SaveResult> {
        if capacity == 0 {
            return Ok(SaveResult {
                saved_count: 0,
                charge_limit_reached: true,
            });
        }

        self.push_dataset_item(item).await?;
        if self.budget.is_ppe == Some(true) {
            self.charge_index_result(item).await?;
            self.record_local_event_charge(INDEX_RESULT_CHARGE_EVENT)?;
            let charge_limit_reached = self.get_capacity().await? == 0;
            return Ok(SaveResult {
                saved_count: 1,
                charge_limit_reached,
            });
        }

        Ok(SaveResult {
            saved_count: 1,
            charge_limit_reached: false,
        })
    }

    fn record_local_event_charge(&mut self, event_name: &str) -> Result<()> {
        let count = self
            .budget
            .local_event_charges
            .entry(event_name.to_owned())
            .or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| anyhow!("Charged {event_name} count overflowed"))?;
        Ok(())
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
    }

    async fn send(&self, method: Method, url: Url) -> Result<Response> {
        self.http
            .request(method, url)
            .bearer_auth(&self.config.apify_token)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify API request failed")
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["actor-runs", &self.config.actor_run_id])?;
        let response = self
            .send(Method::GET, url)
            .await
            .context("Apify run pricing request failed")?;
        response_json(response, "Apify run pricing request").await
    }

    async fn push_dataset_item(&mut self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(item)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await?;
        if self.budget.is_ppe == Some(true) {
            self.record_local_event_charge(DEFAULT_DATASET_ITEM_EVENT)?;
        }
        Ok(())
    }

    async fn charge_index_result(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["actor-runs", &self.config.actor_run_id, "charge"])?;
        let result_id = item
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Index result is missing its id"))?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                "idempotency-key",
                format!("{}:index-result:{result_id}", self.config.actor_run_id),
            )
            .json(&json!({ "eventName": INDEX_RESULT_CHARGE_EVENT, "count": 1 }))
            .send()
            .await
            .context("Apify index-result charge request failed")?;
        ensure_success(response, "Apify index-result charge").await?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveResult {
    pub saved_count: usize,
    pub charge_limit_reached: bool,
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(["v2"].into_iter().chain(segments.iter().copied()));
    Ok(url)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read Apify API response")?;
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
    let body = response.text().await.unwrap_or_default();
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

fn charged_counts(run: &Value) -> Result<BTreeMap<String, u64>> {
    let counts = run
        .pointer("/data/chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    counts
        .iter()
        .map(|(name, count)| {
            count
                .as_u64()
                .map(|count| (name.clone(), count))
                .ok_or_else(|| anyhow!("Invalid charged event count for {name}"))
        })
        .collect()
}

fn affordable_row_count(
    run: &Value,
    event_name: &str,
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
        budget.is_ppe = Some(false);
        return Ok(None);
    }
    budget.is_ppe = Some(true);
    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let result_event_price = events
        .get(event_name)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the {event_name} event price"))?;
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => return Ok(Some(usize::MAX)),
        Some(value) => value
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run provided an invalid spending limit"))?,
    };
    if !result_event_price.is_finite()
        || result_event_price < 0.0
        || !max_charge.is_finite()
        || max_charge < 0.0
    {
        bail!("Apify run returned invalid charging values");
    }
    if max_charge == 0.0 {
        return Ok(Some(usize::MAX));
    }

    let dataset_item_price = events
        .get(DEFAULT_DATASET_ITEM_EVENT)
        .map(|event| {
            event
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    anyhow!(
                        "Apify run did not provide the {DEFAULT_DATASET_ITEM_EVENT} event price"
                    )
                })
        })
        .transpose()?
        .unwrap_or(0.0);
    if !dataset_item_price.is_finite() || dataset_item_price < 0.0 {
        bail!("Apify run returned an invalid dataset item price");
    }
    let row_price = result_event_price + dataset_item_price;
    if !row_price.is_finite() {
        bail!("Apify run returned invalid per-row pricing");
    }

    let current_counts = charged_counts(run)?;
    let initial_counts = budget
        .initial_counts
        .get_or_insert_with(|| current_counts.clone());
    let mut spent = 0.0;
    for (name, event) in events {
        let price = event
            .get("eventPriceUsd")
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Missing price for charged event {name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {name}");
        }
        let current = current_counts.get(name).copied().unwrap_or(0);
        let initial = initial_counts.get(name).copied().unwrap_or(0);
        let local = budget.local_event_charges.get(name).copied().unwrap_or(0);
        let count = initial
            .checked_add(local)
            .ok_or_else(|| anyhow!("Charged event count overflowed for {name}"))?
            .max(current);
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if row_price == 0.0 {
        return Ok(Some(usize::MAX));
    }
    let remaining = (max_charge - spent).max(0.0);
    let count = (remaining / row_price).floor();
    let mut count = if count.is_finite() {
        count as usize
    } else {
        usize::MAX
    };
    if spent + row_price * count as f64 > max_charge {
        count = count.saturating_sub(1);
    }
    Ok(Some(count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockResponse, MockServer};
    use serde_json::json;

    fn actor_config(base_url: &str) -> ActorConfig {
        ActorConfig {
            apify_api_base: Url::parse(base_url).unwrap(),
            apify_token: "apify-test-token".to_owned(),
            key_value_store_id: "store-1".to_owned(),
            dataset_id: "dataset-1".to_owned(),
            actor_run_id: "run-1".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
            scrappa_api_base: None,
        }
    }

    fn run(max_charge: f64, counts: Value) -> Value {
        json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "index-result": {"eventPriceUsd": 0.00025},
                "apify-actor-start": {"eventPriceUsd": 0.00005}
            } } },
            "options": {"maxTotalChargeUsd": max_charge},
            "chargedEventCounts": counts
        }})
    }

    fn run_with_dataset_item_price(max_charge: f64, counts: Value) -> Value {
        let mut run = run(max_charge, counts);
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]["apify-default-dataset-item"] =
            json!({"eventPriceUsd": 0.0001});
        run
    }

    #[test]
    fn computes_result_capacity_after_all_existing_event_charges() {
        let mut budget = ChargeBudget::default();
        assert_eq!(
            affordable_row_count(
                &run(0.00055, json!({"apify-actor-start": 1})),
                INDEX_RESULT_CHARGE_EVENT,
                &mut budget
            )
            .unwrap(),
            Some(2)
        );
        budget
            .local_event_charges
            .insert(INDEX_RESULT_CHARGE_EVENT.to_owned(), 1);
        assert_eq!(
            affordable_row_count(
                &run(0.00055, json!({"apify-actor-start": 1})),
                INDEX_RESULT_CHARGE_EVENT,
                &mut budget
            )
            .unwrap(),
            Some(1)
        );
        assert_eq!(
            affordable_row_count(
                &run(0.00055, json!({"index-result": 1, "apify-actor-start": 1})),
                INDEX_RESULT_CHARGE_EVENT,
                &mut budget
            )
            .unwrap(),
            Some(1)
        );
    }

    #[test]
    fn computes_capacity_from_custom_and_default_dataset_prices_per_row() {
        let mut budget = ChargeBudget::default();
        assert_eq!(
            affordable_row_count(
                &run_with_dataset_item_price(0.0006, json!({"apify-actor-start": 1})),
                INDEX_RESULT_CHARGE_EVENT,
                &mut budget
            )
            .unwrap(),
            Some(1)
        );

        budget
            .local_event_charges
            .insert(INDEX_RESULT_CHARGE_EVENT.to_owned(), 1);
        assert_eq!(
            affordable_row_count(
                &run_with_dataset_item_price(0.0006, json!({"apify-actor-start": 1})),
                INDEX_RESULT_CHARGE_EVENT,
                &mut budget
            )
            .unwrap(),
            Some(0)
        );
    }

    #[test]
    fn dataset_only_rows_consume_capacity_when_the_default_event_is_priced() {
        let mut budget = ChargeBudget::default();
        assert_eq!(
            affordable_row_count(
                &run_with_dataset_item_price(
                    0.00045,
                    json!({"apify-actor-start": 1, "apify-default-dataset-item": 1})
                ),
                INDEX_RESULT_CHARGE_EVENT,
                &mut budget
            )
            .unwrap(),
            Some(0)
        );
    }

    #[test]
    fn validates_prices_and_limits_but_allows_unlimited_and_non_ppe_runs() {
        let mut budget = ChargeBudget::default();
        let mut no_event = run(1.0, json!({}));
        no_event["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"] = json!({});
        assert!(affordable_row_count(&no_event, INDEX_RESULT_CHARGE_EVENT, &mut budget).is_err());

        let mut unlimited = run(1.0, json!({}));
        unlimited["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
        assert_eq!(
            affordable_row_count(&unlimited, INDEX_RESULT_CHARGE_EVENT, &mut budget).unwrap(),
            Some(usize::MAX)
        );

        let mut zero_limit = run(0.0, json!({}));
        zero_limit["data"]["options"]["maxTotalChargeUsd"] = json!(0);
        assert_eq!(
            affordable_row_count(&zero_limit, INDEX_RESULT_CHARGE_EVENT, &mut budget).unwrap(),
            Some(usize::MAX)
        );

        let mut missing_limit = run(1.0, json!({}));
        missing_limit["data"]["options"] = json!({});
        assert_eq!(
            affordable_row_count(
                &missing_limit,
                INDEX_RESULT_CHARGE_EVENT,
                &mut budget
            )
            .unwrap(),
            Some(usize::MAX)
        );

        let mut invalid_limit = run(1.0, json!({}));
        invalid_limit["data"]["options"]["maxTotalChargeUsd"] = json!("unlimited");
        assert!(
            affordable_row_count(&invalid_limit, INDEX_RESULT_CHARGE_EVENT, &mut budget).is_err()
        );

        let non_ppe = json!({"data":{"pricingInfo":{"pricingModel":"FLAT_PRICE"}}});
        assert_eq!(
            affordable_row_count(&non_ppe, INDEX_RESULT_CHARGE_EVENT, &mut budget).unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn reads_input_from_the_default_key_value_store_with_apify_auth() {
        let input = json!({"indices":[".INX"],"hl":"en","gl":"us"});
        let server = MockServer::start(vec![MockResponse::json(200, &input.to_string())]);
        let client = ApifyClient::new(actor_config(&server.base_url)).unwrap();

        assert_eq!(client.get_input().await.unwrap(), input);
        let requests = server.finish();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(
            requests[0].target,
            "/v2/key-value-stores/store-1/records/INPUT"
        );
        assert!(requests[0]
            .headers
            .to_ascii_lowercase()
            .contains("authorization: bearer apify-test-token"));
    }

    #[tokio::test]
    async fn missing_input_record_is_treated_as_empty_input() {
        let server = MockServer::start(vec![MockResponse::json(404, "{}")]);
        let client = ApifyClient::new(actor_config(&server.base_url)).unwrap();

        assert!(client.get_input().await.unwrap().is_null());
        assert_eq!(server.finish().len(), 1);
    }

    #[tokio::test]
    async fn writes_only_saved_dataset_rows_and_charges_the_custom_event_with_idempotency() {
        let start_run = run(0.00025, json!({}));
        let server = MockServer::start(vec![
            MockResponse::json(200, &start_run.to_string()),
            MockResponse::json(200, "{}"),
            MockResponse::json(201, "{}"),
        ]);
        let mut client = ApifyClient::new(actor_config(&server.base_url)).unwrap();
        let item = json!({"id":"INDEXSP:.INX","symbol":".INX"});

        let capacity = client.get_capacity().await.unwrap();
        assert_eq!(
            client.save_index(&item, capacity).await.unwrap(),
            SaveResult {
                saved_count: 1,
                charge_limit_reached: true
            }
        );
        assert_eq!(client.get_capacity().await.unwrap(), 0);
        let requests = server.finish();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].target, "/v2/actor-runs/run-1");
        assert_eq!(requests[1].method, "POST");
        assert_eq!(requests[1].target, "/v2/datasets/dataset-1/items");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[1].body).unwrap(),
            item
        );
        assert_eq!(requests[2].method, "POST");
        assert_eq!(requests[2].target, "/v2/actor-runs/run-1/charge");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[2].body).unwrap(),
            json!({"eventName":"index-result","count":1})
        );
        let charge_headers = requests[2].headers.to_ascii_lowercase();
        assert!(charge_headers.contains("authorization: bearer apify-test-token"));
        assert!(charge_headers.contains("idempotency-key: run-1:index-result:indexsp:.inx"));
        assert!(requests
            .iter()
            .all(|request| !request.target.ends_with("/records/OUTPUT")));
    }

    #[tokio::test]
    async fn reuses_run_pricing_and_charge_counts_for_each_capacity_check() {
        let server = MockServer::start(vec![MockResponse::json(
            200,
            &run(0.0005, json!({})).to_string(),
        )]);
        let mut client = ApifyClient::new(actor_config(&server.base_url)).unwrap();

        assert_eq!(client.get_capacity().await.unwrap(), 2);
        assert_eq!(client.get_capacity().await.unwrap(), 2);
        assert_eq!(server.finish().len(), 1);
    }

    #[tokio::test]
    async fn stops_after_one_row_when_custom_and_default_events_reach_a_tight_cap() {
        let start_run = run_with_dataset_item_price(0.0006, json!({"apify-actor-start": 1}));
        let server = MockServer::start(vec![
            MockResponse::json(200, &start_run.to_string()),
            MockResponse::json(201, "{}"),
            MockResponse::json(200, "{}"),
        ]);
        let mut client = ApifyClient::new(actor_config(&server.base_url)).unwrap();
        let item = json!({"id":"INDEXSP:.INX","symbol":".INX"});

        let capacity = client.get_capacity().await.unwrap();
        assert_eq!(capacity, 1);
        assert_eq!(
            client.save_index(&item, capacity).await.unwrap(),
            SaveResult {
                saved_count: 1,
                charge_limit_reached: true
            }
        );
        assert_eq!(client.get_capacity().await.unwrap(), 0);

        let requests = server.finish();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[1].target, "/v2/datasets/dataset-1/items");
        assert_eq!(requests[2].target, "/v2/actor-runs/run-1/charge");
        assert!(requests
            .iter()
            .all(|request| !request.target.ends_with("/records/OUTPUT")));
    }

    #[tokio::test]
    async fn a_dataset_only_failure_row_uses_default_item_capacity_without_custom_charge() {
        let start_run = run_with_dataset_item_price(0.00045, json!({"apify-actor-start": 1}));
        let server = MockServer::start(vec![
            MockResponse::json(200, &start_run.to_string()),
            MockResponse::json(201, "{}"),
        ]);
        let mut client = ApifyClient::new(actor_config(&server.base_url)).unwrap();

        assert_eq!(client.get_capacity().await.unwrap(), 1);
        client
            .push_dataset_item(&json!({"status":"failed","error":"upstream"}))
            .await
            .unwrap();
        assert_eq!(client.get_capacity().await.unwrap(), 0);

        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].target, "/v2/datasets/dataset-1/items");
    }

    #[tokio::test]
    async fn does_not_charge_when_the_dataset_write_fails() {
        let server = MockServer::start(vec![
            MockResponse::json(200, &run(0.00025, json!({})).to_string()),
            MockResponse::json(500, "storage unavailable"),
        ]);
        let mut client = ApifyClient::new(actor_config(&server.base_url)).unwrap();
        let capacity = client.get_capacity().await.unwrap();

        assert!(client
            .save_index(&json!({"id":"INDEXSP:.INX"}), capacity)
            .await
            .is_err());
        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].target, "/v2/datasets/dataset-1/items");
    }

    #[tokio::test]
    async fn non_ppe_runs_keep_dataset_output_without_a_custom_charge() {
        let non_ppe = json!({"data":{"pricingInfo":{"pricingModel":"FLAT_PRICE"}}});
        let item = json!({"id":"INDEXSP:.INX"});
        let server = MockServer::start(vec![
            MockResponse::json(200, &non_ppe.to_string()),
            MockResponse::json(200, "{}"),
        ]);
        let mut client = ApifyClient::new(actor_config(&server.base_url)).unwrap();

        let capacity = client.get_capacity().await.unwrap();
        assert_eq!(
            client.save_index(&item, capacity).await.unwrap(),
            SaveResult {
                saved_count: 1,
                charge_limit_reached: false
            }
        );
        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].target, "/v2/datasets/dataset-1/items");
    }
}
