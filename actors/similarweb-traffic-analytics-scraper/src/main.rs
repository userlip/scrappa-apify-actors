mod request_params;
mod response_utils;
mod scrappa_client;

#[cfg(test)]
mod test_support;

use std::{
    collections::{HashMap, HashSet},
    env,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Value};
use url::Url;

use request_params::{
    build_similarweb_traffic_requests, describe_similarweb_traffic_requests, input_required_error,
    SimilarwebTrafficRequest,
};
use response_utils::{build_similarweb_dataset_item, has_similarweb_traffic_data};
use scrappa_client::{is_scrappa_not_found, ScrappaClient, ScrappaTimeoutError};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const INPUT_KEY_DEFAULT: &str = "INPUT";
const OUTPUT_KEY: &str = "OUTPUT";
const DOMAIN_RESULT_CHARGE_EVENT: &str = "domain-result";
const DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const ACTOR_TIMEOUT_STATUS_UPDATE: Duration = Duration::from_secs(1);

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
    scrappa_api_key: Option<String>,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|key| !key.is_empty())
                .unwrap_or_else(|| INPUT_KEY_DEFAULT.to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key: env::var("SCRAPPA_API_KEY")
                .ok()
                .filter(|key| !key.is_empty()),
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.trim().is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

async fn run_actor(http: &Client, config: &ActorConfig) -> Result<()> {
    let run = get_run(http, config).await?;
    let mut budget = ChargeBudget::from_run(&run)?;
    let api_key = config
        .scrappa_api_key
        .as_deref()
        .ok_or_else(|| anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."))?;
    let input = get_input(http, config)
        .await?
        .ok_or_else(input_required_error)?;
    let requests = build_similarweb_traffic_requests(&input)?;
    println!(
        "Fetching Similarweb traffic analytics for {}",
        describe_similarweb_traffic_requests(&requests)
    );

    let scrappa = ScrappaClient::new(
        http.clone(),
        config.scrappa_api_base_url.clone(),
        api_key.to_owned(),
        SCRAPPA_REQUEST_TIMEOUT,
    );
    run_actor_requests(http, config, &mut budget, &scrappa, &requests).await
}

async fn run_actor_requests(
    http: &Client,
    config: &ActorConfig,
    budget: &mut ChargeBudget,
    scrappa: &ScrappaClient,
    requests: &[SimilarwebTrafficRequest],
) -> Result<()> {
    let mut processed = 0;
    let mut successful = 0;
    let mut no_data = 0;
    let mut status_message = None;

    for (index, request) in requests.iter().enumerate() {
        if budget.charge_limit_reached_before_fetch() {
            let message = "Charge limit reached before fetching the next Similarweb domain result; no more Scrappa requests will be made.";
            println!(
                "{} {}",
                message,
                json!({
                    "event": DOMAIN_RESULT_CHARGE_EVENT,
                    "processed": processed,
                    "requested": requests.len(),
                })
            );
            status_message = Some(message.to_owned());
            break;
        }

        println!(
            "Fetching Similarweb traffic analytics for {}",
            request.domain
        );
        let response = match scrappa.get_similarweb(&request.domain).await {
            Ok(response) => response,
            Err(error) if is_scrappa_not_found(&error) => {
                let item = json!({
                    "success": false,
                    "domain": request.domain,
                    "input_domain": request.input_domain,
                    "request_domain": request.domain,
                    "status_code": 404,
                    "error": "No traffic data available",
                });
                let result = push_charged_item(http, config, budget, &item, index).await?;
                if !result.saved {
                    status_message = result.status_message;
                    break;
                }

                processed += 1;
                no_data += 1;
                if result.status_message.is_some() {
                    status_message = result.status_message;
                    break;
                }
                println!(
                    "No Similarweb traffic data available for {}",
                    request.domain
                );
                continue;
            }
            Err(error) => return Err(error),
        };

        let (item, is_no_data) = if has_similarweb_traffic_data(&response) {
            (build_similarweb_dataset_item(&response, request), false)
        } else {
            (
                json!({
                    "success": false,
                    "domain": request.domain,
                    "input_domain": request.input_domain,
                    "request_domain": request.domain,
                    "error": "No traffic data returned",
                }),
                true,
            )
        };
        let result = push_charged_item(http, config, budget, &item, index).await?;
        if !result.saved {
            status_message = result.status_message;
            break;
        }

        processed += 1;
        if is_no_data {
            no_data += 1;
            println!("No Similarweb traffic data returned for {}", request.domain);
        } else {
            successful += 1;
        }
        if result.status_message.is_some() {
            status_message = result.status_message;
            break;
        }
    }

    let output = json!({
        "requested": requests.len(),
        "processed": processed,
        "successful": successful,
        "no_data": no_data,
        "status_message": status_message,
    });
    put_output(http, config, &output).await?;

    println!("Similarweb traffic analytics completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "requested": output["requested"],
            "processed": output["processed"],
            "successful": output["successful"],
            "no_data": output["no_data"],
        })
    );

    if let Some(message) = status_message {
        write_status_message_best_effort(http, config, &message).await;
    }
    Ok(())
}

async fn get_run(http: &Client, config: &ActorConfig) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = http
        .get(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .context("Apify run pricing request failed")?;
    response_json(response, "Apify run pricing request").await
}

async fn get_input(http: &Client, config: &ActorConfig) -> Result<Option<Value>> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let response = http
        .get(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .context("Apify INPUT request failed")?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response_json(response, "Apify INPUT request")
        .await
        .map(Some)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .bytes()
        .await
        .with_context(|| format!("{operation} response could not be read"))?;
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = String::from_utf8_lossy(&body);
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
    serde_json::from_slice(&body).with_context(|| format!("{operation} returned invalid JSON"))
}

async fn put_output(http: &Client, config: &ActorConfig, output: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            OUTPUT_KEY,
        ],
    )?;
    let response = http
        .put(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .json(output)
        .send()
        .await
        .context("Apify OUTPUT write failed")?;
    ensure_apify_success(response, "OUTPUT write").await
}

async fn push_dataset_item(http: &Client, config: &ActorConfig, item: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.dataset_id, "items"],
    )?;
    let response = http
        .post(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .json(item)
        .send()
        .await
        .context("Apify dataset write failed")?;
    ensure_apify_success(response, "dataset write").await
}

async fn post_charge_event(
    http: &Client,
    config: &ActorConfig,
    event_name: &str,
    index: usize,
) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id, "charge"],
    )?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let idempotency_key = format!("{}-{event_name}-{index}-{timestamp}", config.actor_run_id);
    let response = http
        .post(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .header("idempotency-key", idempotency_key)
        .json(&json!({"eventName": event_name, "count": 1}))
        .send()
        .await
        .context("Apify charge request failed")?;
    ensure_apify_success(response, "charge request").await
}

async fn set_status_message(http: &Client, config: &ActorConfig, message: &str) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = http
        .put(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .json(&json!({
            "runId": &config.actor_run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true,
        }))
        .send()
        .await
        .context("Apify status message request failed")?;
    ensure_apify_success(response, "status message request").await
}

async fn write_status_message_best_effort(http: &Client, config: &ActorConfig, message: &str) {
    eprintln!("[Status message]: {message}");
    match tokio::time::timeout(
        ACTOR_TIMEOUT_STATUS_UPDATE,
        set_status_message(http, config, message),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => eprintln!("Could not update the Apify status message: {error}"),
        Err(_) => eprintln!("Setting status message timed out after 1s"),
    }
}

async fn ensure_apify_success(response: Response, operation: &str) -> Result<()> {
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    bail!(
        "Apify {operation} failed with {} {reason}{detail}",
        status.as_u16()
    );
}

struct ChargeBudget {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    prices: HashMap<String, f64>,
    registered_events: HashSet<String>,
    charged_counts: HashMap<String, u64>,
}

impl ChargeBudget {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        if data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(Self {
                is_pay_per_event: false,
                max_total_charge_usd: f64::INFINITY,
                prices: HashMap::new(),
                registered_events: HashSet::new(),
                charged_counts: HashMap::new(),
            });
        }

        let event_data = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut prices = HashMap::new();
        let mut registered_events = HashSet::new();
        for (name, event) in event_data {
            registered_events.insert(name.clone());
            let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) else {
                continue;
            };
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned an invalid price for event {name}");
            }
            prices.insert(name.clone(), price);
        }

        let max_total_charge_usd = match env::var("ACTOR_MAX_TOTAL_CHARGE_USD") {
            Ok(value) if !value.is_empty() => {
                let value = value
                    .parse::<f64>()
                    .context("ACTOR_MAX_TOTAL_CHARGE_USD must be numeric")?;
                if !value.is_finite() || value < 0.0 {
                    bail!("ACTOR_MAX_TOTAL_CHARGE_USD must be a finite non-negative number");
                }
                if value == 0.0 {
                    f64::INFINITY
                } else {
                    value
                }
            }
            _ => {
                let value = data
                    .pointer("/options/maxTotalChargeUsd")
                    .and_then(Value::as_f64)
                    .filter(|value| *value > 0.0)
                    .unwrap_or(f64::INFINITY);
                if !value.is_finite() && !value.is_infinite() {
                    bail!("Apify run returned an invalid spending limit");
                }
                value
            }
        };

        let mut charged_counts = HashMap::new();
        if let Some(counts) = data.get("chargedEventCounts").and_then(Value::as_object) {
            for (event, count) in counts {
                let count = count
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {event}"))?;
                charged_counts.insert(event.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event: true,
            max_total_charge_usd,
            prices,
            registered_events,
            charged_counts,
        })
    }

    fn charge_limit_reached_before_fetch(&self) -> bool {
        self.is_pay_per_event && self.max_event_charge_count(DOMAIN_RESULT_CHARGE_EVENT) == 0
    }

    fn can_push_result(&self) -> bool {
        let item_price = self.event_price(DOMAIN_RESULT_CHARGE_EVENT)
            + self.event_price(DATASET_ITEM_CHARGE_EVENT);
        let maximum_items = if item_price > 0.0 {
            self.max_charge_count_by_price(item_price)
        } else {
            usize::MAX
        };
        maximum_items > 0
            || (maximum_items == 0 && self.total_charged_amount() <= self.max_total_charge_usd)
    }

    fn charge_push_events(&mut self) -> (u64, bool, u64) {
        let total_before_charge = self.total_charged_amount();
        let event_names = [DOMAIN_RESULT_CHARGE_EVENT, DATASET_ITEM_CHARGE_EVENT];
        let mut charge_counts = [0_u64; 2];

        for (index, event_name) in event_names.iter().enumerate() {
            let maximum = self.max_event_charge_count(event_name);
            charge_counts[index] = if maximum > 0 {
                1
            } else if total_before_charge <= self.max_total_charge_usd {
                1
            } else {
                0
            };
        }

        for (index, event_name) in event_names.iter().enumerate() {
            if charge_counts[index] > 0 {
                let count = self
                    .charged_counts
                    .entry((*event_name).to_owned())
                    .or_default();
                *count = count.saturating_add(charge_counts[index]);
            }
        }

        let charge_limit_reached = event_names
            .iter()
            .any(|event_name| self.max_event_charge_count(event_name) == 0);
        (
            charge_counts.iter().sum(),
            charge_limit_reached,
            charge_counts[0],
        )
    }

    fn total_charged_amount(&self) -> f64 {
        let amount = self
            .charged_counts
            .iter()
            .map(|(event, count)| {
                self.prices.get(event).copied().unwrap_or_default() * *count as f64
            })
            .sum::<f64>();
        round_to_fixed(amount, 6)
    }

    fn max_event_charge_count(&self, event_name: &str) -> usize {
        self.prices
            .get(event_name)
            .map(|price| self.max_charge_count_by_price(*price))
            .unwrap_or(usize::MAX)
    }

    fn max_charge_count_by_price(&self, price: f64) -> usize {
        if price <= 0.0 {
            return usize::MAX;
        }
        let remaining = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if remaining.is_infinite() && remaining.is_sign_positive() {
            return usize::MAX;
        }
        if !remaining.is_finite() {
            return 0;
        }
        let rounded = round_to_fixed(remaining, 4);
        if rounded <= 0.0 {
            return 0;
        }
        rounded.floor().min(usize::MAX as f64) as usize
    }

    fn event_price(&self, event_name: &str) -> f64 {
        self.prices.get(event_name).copied().unwrap_or_default()
    }
}

fn round_to_fixed(value: f64, decimal_places: usize) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let rounded = match decimal_places {
        4 => format!("{value:.4}"),
        6 => format!("{value:.6}"),
        _ => return value,
    };
    rounded.parse().unwrap_or(value)
}

struct PushChargedItemResult {
    saved: bool,
    status_message: Option<String>,
}

async fn push_charged_item(
    http: &Client,
    config: &ActorConfig,
    budget: &mut ChargeBudget,
    item: &Value,
    index: usize,
) -> Result<PushChargedItemResult> {
    if !budget.is_pay_per_event {
        push_dataset_item(http, config, item).await?;
        return Ok(PushChargedItemResult {
            saved: true,
            status_message: None,
        });
    }

    if !budget.can_push_result() {
        let message = format!(
            "Charge limit reached before saving Similarweb result for {}.",
            item.get("domain")
                .map(|domain| domain
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| domain.to_string()))
                .unwrap_or_default()
        );
        return Ok(PushChargedItemResult {
            saved: false,
            status_message: Some(message),
        });
    }

    push_dataset_item(http, config, item).await?;
    let (charged_count, event_charge_limit_reached, custom_event_charge_count) =
        budget.charge_push_events();

    if custom_event_charge_count > 0
        && budget
            .registered_events
            .contains(DOMAIN_RESULT_CHARGE_EVENT)
    {
        post_charge_event(http, config, DOMAIN_RESULT_CHARGE_EVENT, index).await?;
    }

    if event_charge_limit_reached {
        let saved = charged_count >= 1;
        let message = if saved {
            format!(
                "Charge limit reached after saving Similarweb result for {}.",
                item.get("domain")
                    .map(|domain| domain
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| domain.to_string()))
                    .unwrap_or_default()
            )
        } else {
            format!(
                "Charge limit reached before saving Similarweb result for {}.",
                item.get("domain")
                    .map(|domain| domain
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| domain.to_string()))
                    .unwrap_or_default()
            )
        };
        println!(
            "{} {}",
            message,
            json!({
                "event": DOMAIN_RESULT_CHARGE_EVENT,
                "charged_count": charged_count,
            })
        );
        return Ok(PushChargedItemResult {
            saved,
            status_message: Some(message),
        });
    }

    Ok(PushChargedItemResult {
        saved: charged_count >= 1,
        status_message: None,
    })
}

fn failure_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return format!(
            "{error}. The Similarweb traffic request exceeded the {}s Scrappa API timeout. Try fewer domains or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        );
    }
    error.to_string()
}

#[tokio::main]
async fn main() {
    let config = match ActorConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {}", failure_message(&error));
            std::process::exit(1);
        }
    };
    let http = match Client::builder().build() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Actor failed: Could not create HTTP client: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = run_actor(&http, &config).await {
        let message = failure_message(&error);
        eprintln!("Actor failed: {message}");
        write_status_message_best_effort(&http, &config, &message).await;
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::*;
    use crate::test_support::{request_header, request_parts, MockResponse, MockServer};

    fn config(base_url: &Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: base_url.clone(),
            key_value_store_id: "test-store".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
            scrappa_api_key: Some("scrappa-test-key".to_owned()),
        }
    }

    fn client() -> Client {
        Client::builder().build().unwrap()
    }

    fn free_run() -> MockResponse {
        MockResponse::json(200, r#"{"data":{"pricingInfo":{"pricingModel":"FREE"}}}"#)
    }

    fn ppe_run(max_total_charge_usd: f64, domain_count: u64, dataset_count: u64) -> MockResponse {
        MockResponse::json(
            200,
            &json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "domain-result": {"eventPriceUsd": 0.01},
                            "apify-default-dataset-item": {"eventPriceUsd": 0.001}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_total_charge_usd},
                    "chargedEventCounts": {
                        "domain-result": domain_count,
                        "apify-default-dataset-item": dataset_count
                    }
                }
            })
            .to_string(),
        )
    }

    fn input(body: &str) -> MockResponse {
        MockResponse::json(200, body)
    }

    fn similarweb(body: &str) -> MockResponse {
        MockResponse::json(200, body)
    }

    fn successful_response() -> &'static str {
        r#"{"domain":"google.com","site_name":"google.com","global_rank":{"Rank":1},"engagement":{"visits":"1000"},"monthly_visits":{"2026-01-01":"1000"}}"#
    }

    #[test]
    fn ppe_budget_limits_a_result_by_custom_and_dataset_event_prices() {
        let run = json!({
            "data": {
                "pricingInfo": {"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{
                    "domain-result":{"eventPriceUsd":0.01},
                    "apify-default-dataset-item":{"eventPriceUsd":0.001},
                    "apify-actor-start":{"eventPriceUsd":0.00005}
                }}},
                "options":{"maxTotalChargeUsd":0.01105},
                "chargedEventCounts":{"apify-actor-start":1}
            }
        });
        let budget = ChargeBudget::from_run(&run).unwrap();
        assert!(!budget.charge_limit_reached_before_fetch());
        assert!(budget.can_push_result());

        let run = json!({
            "data": {
                "pricingInfo": {"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{
                    "domain-result":{"eventPriceUsd":0.01},
                    "apify-default-dataset-item":{"eventPriceUsd":0.001}
                }}},
                "options":{"maxTotalChargeUsd":0.01},
                "chargedEventCounts":{}
            }
        });
        let budget = ChargeBudget::from_run(&run).unwrap();
        assert!(!budget.charge_limit_reached_before_fetch());
        assert!(budget.can_push_result());
    }

    #[test]
    fn ppe_preflight_stops_when_the_custom_event_has_no_remaining_budget() {
        let run = json!({
            "data": {
                "pricingInfo": {"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{
                    "domain-result":{"eventPriceUsd":0.01},
                    "apify-default-dataset-item":{"eventPriceUsd":0.001}
                }}},
                "options":{"maxTotalChargeUsd":0.01},
                "chargedEventCounts":{"domain-result":1}
            }
        });
        let budget = ChargeBudget::from_run(&run).unwrap();
        assert!(budget.charge_limit_reached_before_fetch());
    }

    #[tokio::test]
    async fn run_reads_input_calls_scrappa_and_writes_dataset_and_output() {
        let server = MockServer::start(vec![
            free_run(),
            input(r#"{"domain":" https://www.Google.com/search ","domains":["google.com","github.com/features"]}"#),
            similarweb(successful_response()),
            MockResponse::json(201, ""),
            similarweb(r#"{"domain":"github.com","global_rank":321}"#),
            MockResponse::json(201, ""),
            MockResponse::json(201, ""),
        ])
        .await;
        let config = config(&server.base_url);

        run_actor(&client(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 7);
        assert_eq!(request_parts(&requests[0]).1, "/v2/actor-runs/test-run");
        assert_eq!(
            request_parts(&requests[1]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert_eq!(
            request_header(&requests[0], "authorization"),
            Some("Bearer test-token-not-a-real-credential")
        );
        assert_eq!(
            request_header(&requests[2], "x-api-key"),
            Some("scrappa-test-key")
        );
        assert_eq!(
            request_header(&requests[2], "user-agent"),
            Some("thescrappa-similarweb-traffic-analytics-scraper/1.0")
        );
        assert!(request_parts(&requests[2])
            .1
            .starts_with("/similarweb?domain=google.com"));

        let (method, path, first_item) = request_parts(&requests[3]);
        assert_eq!(method, "POST");
        assert_eq!(path, "/v2/datasets/test-dataset/items");
        let first_item: Value = serde_json::from_str(first_item).unwrap();
        assert_eq!(first_item["input_domain"], "https://www.Google.com/search");
        assert_eq!(first_item["request_domain"], "google.com");
        assert_eq!(request_parts(&requests[5]).0, "POST");
        assert_eq!(
            request_parts(&requests[5]).1,
            "/v2/datasets/test-dataset/items"
        );

        let (method, path, output) = request_parts(&requests[6]);
        assert_eq!(method, "PUT");
        assert_eq!(path, "/v2/key-value-stores/test-store/records/OUTPUT");
        let output: Value = serde_json::from_str(output).unwrap();
        assert_eq!(output["requested"], 2);
        assert_eq!(output["processed"], 2);
        assert_eq!(output["successful"], 2);
        assert_eq!(output["no_data"], 0);
        assert_eq!(output["status_message"], Value::Null);
    }

    #[tokio::test]
    async fn not_found_and_empty_responses_create_billable_no_data_items() {
        let server = MockServer::start(vec![
            free_run(),
            input(r#"{"domains":["missing.example","empty.example"]}"#),
            MockResponse::json(404, r#"{"message":"No traffic data available"}"#),
            MockResponse::json(201, ""),
            similarweb(r#"{"domain":"empty.example","site_name":"Empty"}"#),
            MockResponse::json(201, ""),
            MockResponse::json(201, ""),
        ])
        .await;

        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();

        let requests = server.requests();
        let first: Value = serde_json::from_str(request_parts(&requests[3]).2).unwrap();
        assert_eq!(first["success"], false);
        assert_eq!(first["status_code"], 404);
        assert_eq!(first["error"], "No traffic data available");
        let second: Value = serde_json::from_str(request_parts(&requests[5]).2).unwrap();
        assert_eq!(second["success"], false);
        assert_eq!(second["error"], "No traffic data returned");
        let output: Value = serde_json::from_str(request_parts(&requests[6]).2).unwrap();
        assert_eq!(output["processed"], 2);
        assert_eq!(output["successful"], 0);
        assert_eq!(output["no_data"], 2);
    }

    #[tokio::test]
    async fn retries_transient_upstream_failures_with_the_same_domain_request() {
        let server = MockServer::start(vec![
            free_run(),
            input(r#"{"domain":"google.com"}"#),
            MockResponse::json(503, r#"{"message":"Unavailable"}"#),
            similarweb(successful_response()),
            MockResponse::json(201, ""),
            MockResponse::json(201, ""),
        ])
        .await;
        let config = config(&server.base_url);

        let result = run_actor_with_scrappa_retry_delay(&client(), &config, Duration::ZERO).await;
        result.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 6);
        assert!(request_parts(&requests[2])
            .1
            .starts_with("/similarweb?domain=google.com"));
        assert!(request_parts(&requests[3])
            .1
            .starts_with("/similarweb?domain=google.com"));
    }

    async fn run_actor_with_scrappa_retry_delay(
        http: &Client,
        config: &ActorConfig,
        retry_delay: Duration,
    ) -> Result<()> {
        let run = get_run(http, config).await?;
        let mut budget = ChargeBudget::from_run(&run)?;
        let api_key = config.scrappa_api_key.as_deref().ok_or_else(|| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        let input = get_input(http, config)
            .await?
            .ok_or_else(input_required_error)?;
        let requests = build_similarweb_traffic_requests(&input)?;
        let scrappa = ScrappaClient::new(
            http.clone(),
            config.scrappa_api_base_url.clone(),
            api_key.to_owned(),
            SCRAPPA_REQUEST_TIMEOUT,
        )
        .with_retry_delay(retry_delay);
        run_actor_requests(http, config, &mut budget, &scrappa, &requests).await
    }

    #[tokio::test]
    async fn ppe_push_charges_custom_event_and_stops_at_the_budget() {
        let server = MockServer::start(vec![
            ppe_run(0.011, 0, 0),
            input(r#"{"domains":["google.com","github.com"]}"#),
            similarweb(successful_response()),
            MockResponse::json(201, ""),
            MockResponse::json(201, ""),
            MockResponse::json(201, ""),
            MockResponse::json(201, ""),
        ])
        .await;

        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 7);
        assert_eq!(
            request_parts(&requests[3]).1,
            "/v2/datasets/test-dataset/items"
        );
        assert_eq!(
            request_parts(&requests[4]).1,
            "/v2/actor-runs/test-run/charge"
        );
        assert_eq!(
            request_header(&requests[4], "idempotency-key").is_some(),
            true
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[4]).2).unwrap(),
            json!({"eventName":"domain-result","count":1})
        );
        assert_eq!(
            request_parts(&requests[5]).1,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
        let output: Value = serde_json::from_str(request_parts(&requests[5]).2).unwrap();
        assert_eq!(output["requested"], 2);
        assert_eq!(output["processed"], 1);
        assert_eq!(output["successful"], 1);
        assert_eq!(
            output["status_message"],
            "Charge limit reached after saving Similarweb result for google.com."
        );
        assert_eq!(request_parts(&requests[6]).0, "PUT");
        assert_eq!(request_parts(&requests[6]).1, "/v2/actor-runs/test-run");
    }

    #[tokio::test]
    async fn ppe_preflight_writes_summary_without_making_scrappa_requests() {
        let server = MockServer::start(vec![
            ppe_run(0.01, 1, 0),
            input(r#"{"domain":"google.com"}"#),
            MockResponse::json(201, ""),
            MockResponse::json(201, ""),
        ])
        .await;

        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests
            .iter()
            .all(|request| !request_parts(request).1.starts_with("/similarweb")));
        let output: Value = serde_json::from_str(request_parts(&requests[2]).2).unwrap();
        assert_eq!(output["processed"], 0);
        assert_eq!(output["status_message"], "Charge limit reached before fetching the next Similarweb domain result; no more Scrappa requests will be made.");
        assert_eq!(request_parts(&requests[3]).0, "PUT");
    }

    #[tokio::test]
    async fn scrappa_timeout_is_retryable_and_deadline_message_keeps_the_60_second_timeout() {
        let server = MockServer::start(vec![MockResponse::delayed_json(
            200,
            successful_response(),
            Duration::from_millis(100),
        )])
        .await;
        let scrappa = ScrappaClient::new(
            client(),
            server.base_url.clone(),
            "test-key".to_owned(),
            Duration::from_millis(10),
        )
        .with_retry_delay(Duration::ZERO);
        let error = scrappa.get_similarweb("google.com").await.unwrap_err();
        assert!(error.to_string().contains("timed out"));
        assert!(failure_message(&error).contains("60s Scrappa API timeout"));
    }

    #[tokio::test]
    async fn upstream_non_retryable_errors_fail_without_writing_output() {
        let server = MockServer::start(vec![
            free_run(),
            input(r#"{"domain":"google.com"}"#),
            MockResponse::json(
                400,
                r#"{"message":"Bad request","errors":{"domain":["invalid"]}}"#,
            ),
        ])
        .await;
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (400): Bad request - domain: invalid"
        );
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(
            request_parts(&requests[2]).1.starts_with("/similarweb"),
            true
        );
    }

    #[tokio::test]
    async fn missing_input_fails_with_the_original_actor_error() {
        let server = MockServer::start(vec![free_run(), MockResponse::json(404, "")]).await;
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "Input is required");
    }

    #[test]
    fn charge_budget_uses_the_run_event_counts_and_rejects_invalid_counts() {
        let run = json!({
            "data": {
                "pricingInfo": {"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{
                    "domain-result":{"eventPriceUsd":0.01},
                    "apify-default-dataset-item":{"eventPriceUsd":0.001},
                    "actor-start":{"eventPriceUsd":0.00005}
                }}},
                "options":{"maxTotalChargeUsd":1.0},
                "chargedEventCounts":{"actor-start":1}
            }
        });
        let budget = ChargeBudget::from_run(&run).unwrap();
        assert_eq!(budget.total_charged_amount(), 0.00005);
        assert_eq!(
            budget.max_event_charge_count(DOMAIN_RESULT_CHARGE_EVENT),
            99
        );

        let invalid = json!({"data": {
            "pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{}}},
            "options":{"maxTotalChargeUsd":1.0},
            "chargedEventCounts":{"domain-result":"one"}
        }});
        assert!(ChargeBudget::from_run(&invalid).is_err());
    }
}
