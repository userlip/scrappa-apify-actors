use std::{collections::HashMap, env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Method, Response, StatusCode};
use serde_json::{json, Value};
use url::Url;

use crate::batch_runner::{RouteSaveResult, RouteWriter};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const ROUTE_RESULT_EVENT: &str = "route-result";
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub struct ApifyConfig {
    pub api_base: Url,
    pub token: String,
    pub run_id: String,
    pub key_value_store_id: String,
    pub dataset_id: String,
    pub input_key: String,
}

impl ApifyConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            token: required_env("APIFY_TOKEN")?,
            run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
        })
    }
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
        endpoint_url(&self.config.api_base, segments)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.http
            .request(method, url)
            .bearer_auth(&self.config.token)
            .header(header::ACCEPT, "application/json")
            .header(
                header::USER_AGENT,
                "thescrappa-google-maps-directions-scraper/1.0",
            )
    }

    pub async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["actor-runs", &self.config.run_id])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .context("Apify run pricing request failed")?;
        response_json(response, "Apify run pricing request").await
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
            .context("Failed to retrieve Actor input from the default key-value store")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Ok(Some(
            response_json(response, "Apify input retrieval").await?,
        ))
    }

    async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(&json!([item]))
            .send()
            .await
            .context("Failed to publish dataset item to Apify")?;
        successful_response(response, "dataset item publication").await?;
        Ok(())
    }

    async fn charge_route_result(&self, idempotency_key: &str) -> Result<()> {
        let url = self.endpoint(&["actor-runs", &self.config.run_id, "charge"])?;
        let response = self
            .request(Method::POST, url)
            .header("idempotency-key", idempotency_key)
            .json(&json!({ "eventName": ROUTE_RESULT_EVENT, "count": 1 }))
            .send()
            .await
            .context("Apify route-result charge request failed")?;
        successful_response(response, "route-result charge").await?;
        Ok(())
    }

    pub async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["actor-runs", &self.config.run_id])?;
        let response = self
            .request(Method::PUT, url)
            .json(&json!({
                "statusMessage": message,
                "isStatusMessageTerminal": true
            }))
            .send()
            .await
            .context("Apify status message update failed")?;
        successful_response(response, "status message update").await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct PpeBudget {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    event_counts: HashMap<String, u64>,
}

impl PpeBudget {
    pub fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing = data.get("pricingInfo");
        let is_pay_per_event = pricing
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self {
                is_pay_per_event: false,
                max_total_charge_usd: f64::INFINITY,
                event_prices: HashMap::new(),
                event_counts: HashMap::new(),
            });
        }

        let events = pricing
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (name, event) in events {
            let price = event
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    anyhow!("Apify run did not provide a price for charged event {name}")
                })?;
            if !price.is_finite() || price < 0.0 {
                bail!("Invalid price for charged event {name}");
            }
            event_prices.insert(name.clone(), price);
        }
        if !event_prices.contains_key(ROUTE_RESULT_EVENT) {
            bail!("Apify run did not provide the {ROUTE_RESULT_EVENT} event price");
        }

        let max_total_charge_usd = match data.pointer("/options/maxTotalChargeUsd") {
            Some(Value::Number(value)) => {
                let value = value
                    .as_f64()
                    .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
                if value == 0.0 {
                    f64::INFINITY
                } else {
                    value
                }
            }
            Some(Value::Null) | None => f64::INFINITY,
            Some(_) => bail!("Apify run returned an invalid spending limit"),
        };
        if !max_total_charge_usd.is_finite() && max_total_charge_usd != f64::INFINITY {
            bail!("Apify run returned an invalid spending limit");
        }
        if max_total_charge_usd < 0.0 {
            bail!("Apify run returned an invalid spending limit");
        }

        let mut event_counts = HashMap::new();
        if let Some(counts) = data.get("chargedEventCounts") {
            let counts = counts
                .as_object()
                .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
            for (name, count) in counts {
                let count = count
                    .as_u64()
                    .ok_or_else(|| anyhow!("Invalid charged event count for {name}"))?;
                if count > 0 && !event_prices.contains_key(name) {
                    bail!("Apify run did not provide a price for charged event {name}");
                }
                event_counts.insert(name.clone(), count);
            }
        }

        let budget = Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            event_counts,
        };
        if !budget.total_charged_amount().is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        Ok(budget)
    }

    pub fn is_pay_per_event(&self) -> bool {
        self.is_pay_per_event
    }

    fn can_charge_event(&self, event: &str) -> bool {
        self.max_event_charge_count(event) > 0
    }

    fn can_push_one_item(&self) -> bool {
        let item_price =
            self.event_price(ROUTE_RESULT_EVENT) + self.event_price(DATASET_ITEM_EVENT);
        self.max_charges_by_price(item_price) > 0
    }

    fn apply_event_charge(&mut self, event: &str) -> EventChargeResult {
        let max_count = self.max_event_charge_count(event);
        let charged_count = usize::from(max_count > 0);
        if charged_count > 0 {
            *self.event_counts.entry(event.to_owned()).or_default() += charged_count as u64;
        }

        EventChargeResult {
            charged_count,
            event_limit_reached: self.max_event_charge_count(event) == 0,
            should_send_to_apify: charged_count > 0
                && !event.starts_with("apify-")
                && self.event_prices.contains_key(event),
        }
    }

    fn max_event_charge_count(&self, event: &str) -> usize {
        self.max_charges_by_price(self.event_price(event))
    }

    fn max_charges_by_price(&self, price: f64) -> usize {
        if price == 0.0 || self.max_total_charge_usd == f64::INFINITY {
            return usize::MAX;
        }
        if price < 0.0 || !price.is_finite() {
            return 0;
        }
        let spent = self.total_charged_amount();
        let tolerance = f64::EPSILON * self.max_total_charge_usd.max(spent).max(1.0);
        let available = (self.max_total_charge_usd - spent + tolerance) / price;
        if available <= 0.0 {
            return 0;
        }
        if !available.is_finite() {
            return usize::MAX;
        }
        available.floor().max(0.0).min(usize::MAX as f64) as usize
    }

    fn event_price(&self, event: &str) -> f64 {
        self.event_prices.get(event).copied().unwrap_or(0.0)
    }

    fn total_charged_amount(&self) -> f64 {
        let total: f64 = self
            .event_counts
            .iter()
            .map(|(event, count)| self.event_price(event) * *count as f64)
            .sum();
        total
    }
}

#[derive(Clone, Copy)]
struct EventChargeResult {
    charged_count: usize,
    event_limit_reached: bool,
    should_send_to_apify: bool,
}

pub struct ApifyRouteWriter<'a> {
    apify: &'a ApifyClient,
    budget: &'a mut PpeBudget,
    run_id: &'a str,
}

impl<'a> ApifyRouteWriter<'a> {
    pub fn new(apify: &'a ApifyClient, budget: &'a mut PpeBudget, run_id: &'a str) -> Self {
        Self {
            apify,
            budget,
            run_id,
        }
    }
}

impl RouteWriter for ApifyRouteWriter<'_> {
    fn can_save(&self) -> bool {
        !self.budget.is_pay_per_event() || self.budget.can_charge_event(ROUTE_RESULT_EVENT)
    }

    async fn save(&mut self, item: &Value) -> Result<RouteSaveResult> {
        if !self.budget.is_pay_per_event() {
            self.apify.push_dataset_item(item).await?;
            return Ok(RouteSaveResult {
                saved: true,
                charged_count: 1,
                charge_limit_reached: false,
            });
        }

        if !self.budget.can_push_one_item() {
            return Ok(RouteSaveResult {
                saved: false,
                charged_count: 0,
                charge_limit_reached: true,
            });
        }

        self.apify.push_dataset_item(item).await?;

        // Match the SDK's PPE-aware dataset write: reserve each event locally first,
        // then submit registered custom events to the Apify charge endpoint. Apify
        // charges the synthetic dataset-item event from the successful dataset write.
        let route_charge = self.budget.apply_event_charge(ROUTE_RESULT_EVENT);
        let dataset_charge = self.budget.apply_event_charge(DATASET_ITEM_EVENT);
        if route_charge.should_send_to_apify {
            let request_index = item
                .get("request_index")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let alternative_index = item
                .get("alternative_index")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let idempotency_key = format!(
                "{}-route-result-{request_index}-{alternative_index}",
                self.run_id
            );
            self.apify.charge_route_result(&idempotency_key).await?;
        }

        let charged_count = route_charge.charged_count + dataset_charge.charged_count;
        Ok(RouteSaveResult {
            saved: charged_count > 0,
            charged_count: usize::from(charged_count > 0),
            charge_limit_reached: route_charge.event_limit_reached
                || dataset_charge.event_limit_reached
                || charged_count == 0,
        })
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env_or_default(name, default);
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("Apify API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(["v2"].into_iter().chain(segments.iter().copied()));
    Ok(url)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let response = successful_response(response, operation).await?;
    response
        .json::<Value>()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let status_code = status.as_u16();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        format!("HTTP {status_code}")
    } else {
        body
    };
    bail!("Apify API error ({status_code}) while trying to {operation}: {detail}");
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    use super::*;
    use serde_json::json;

    fn run(max_charge: f64, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "route-result": { "eventPriceUsd": 0.002 },
                        "apify-default-dataset-item": { "eventPriceUsd": 0.001 },
                        "apify-actor-start": { "eventPriceUsd": 0.0002 }
                    }}
                },
                "chargedEventCounts": counts,
                "options": { "maxTotalChargeUsd": max_charge }
            }
        })
    }

    fn decimal_run(max_charge: f64, dataset_price: f64, counts: Value) -> Value {
        let mut run = run(max_charge, counts);
        let events = &mut run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"];
        events["route-result"]["eventPriceUsd"] = json!(0.1);
        events["apify-default-dataset-item"]["eventPriceUsd"] = json!(dataset_price);
        events["apify-actor-start"]["eventPriceUsd"] = json!(0.2);
        run
    }

    #[test]
    fn reads_ppe_budget_and_accounts_for_custom_and_dataset_event_prices() {
        let mut budget =
            PpeBudget::from_run(&run(0.007, json!({ "apify-actor-start": 1 }))).unwrap();

        assert!(budget.is_pay_per_event());
        assert!(budget.can_charge_event(ROUTE_RESULT_EVENT));
        assert!(budget.can_push_one_item());

        let route = budget.apply_event_charge(ROUTE_RESULT_EVENT);
        let dataset = budget.apply_event_charge(DATASET_ITEM_EVENT);
        assert_eq!(route.charged_count, 1);
        assert_eq!(dataset.charged_count, 1);
        assert!(!route.event_limit_reached);
        assert!(!dataset.event_limit_reached);
        assert!(route.should_send_to_apify);
        assert!(!dataset.should_send_to_apify);
        assert_eq!(budget.event_counts[ROUTE_RESULT_EVENT], 1);
        assert_eq!(budget.event_counts[DATASET_ITEM_EVENT], 1);
    }

    #[test]
    fn refuses_an_item_only_after_the_run_is_already_over_budget() {
        let budget = PpeBudget::from_run(&run(0.001, json!({ "apify-actor-start": 10 }))).unwrap();

        assert!(!budget.can_push_one_item());
        assert!(!budget.can_charge_event(ROUTE_RESULT_EVENT));
    }

    #[test]
    fn refuses_an_item_when_the_limit_is_fractionally_below_its_combined_price() {
        let budget = PpeBudget::from_run(&run(0.002999999, json!({}))).unwrap();

        assert!(!budget.can_push_one_item());
    }

    #[test]
    fn preserves_sub_microdollar_charges_when_computing_remaining_budget() {
        let mut run = run(0.00300048, json!({ "apify-actor-start": 1 }));
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]["apify-actor-start"]
            ["eventPriceUsd"] = json!(0.00000049);
        let budget = PpeBudget::from_run(&run).unwrap();

        assert!(!budget.can_push_one_item());
    }

    #[test]
    fn admits_exact_decimal_boundaries_for_combined_and_prior_charges() {
        let combined = PpeBudget::from_run(&decimal_run(0.3, 0.2, json!({}))).unwrap();
        assert!(combined.can_push_one_item());

        let prior_charge =
            PpeBudget::from_run(&decimal_run(0.3, 0.0, json!({ "apify-actor-start": 1 }))).unwrap();
        assert!(prior_charge.can_push_one_item());
    }

    #[test]
    fn rejects_a_decimal_result_that_exceeds_the_budget() {
        let budget = PpeBudget::from_run(&decimal_run(0.299999999, 0.2, json!({}))).unwrap();

        assert!(!budget.can_push_one_item());
    }

    #[test]
    fn allows_non_ppe_dataset_output_without_custom_charging() {
        let budget =
            PpeBudget::from_run(&json!({ "data": { "pricingInfo": { "pricingModel": "FREE" } } }))
                .unwrap();

        assert!(!budget.is_pay_per_event());
    }

    #[test]
    fn zero_run_spending_limit_matches_actor_sdk_unlimited_default() {
        let budget = PpeBudget::from_run(&run(0.0, json!({}))).unwrap();

        assert!(budget.can_charge_event(ROUTE_RESULT_EVENT));
        assert!(budget.can_push_one_item());
    }

    #[test]
    fn rejects_invalid_ppe_pricing_and_charge_counts() {
        let missing_events =
            json!({ "data": { "pricingInfo": { "pricingModel": "PAY_PER_EVENT" } } });
        assert!(PpeBudget::from_run(&missing_events)
            .unwrap_err()
            .to_string()
            .contains("event prices"));
        let invalid_count = run(1.0, json!({ "apify-actor-start": -1 }));
        assert!(PpeBudget::from_run(&invalid_count)
            .unwrap_err()
            .to_string()
            .contains("Invalid charged event count"));
    }

    #[test]
    fn requires_the_configured_route_event_and_prices_for_existing_charges() {
        let missing_route_event = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "apify-default-dataset-item": { "eventPriceUsd": 0.001 }
                    }}
                }
            }
        });
        assert!(PpeBudget::from_run(&missing_route_event)
            .unwrap_err()
            .to_string()
            .contains("route-result"));

        let unpriced_existing_event = run(1.0, json!({ "legacy-event": 1 }));
        assert!(PpeBudget::from_run(&unpriced_existing_event)
            .unwrap_err()
            .to_string()
            .contains("price for charged event legacy-event"));
    }

    #[tokio::test]
    async fn reads_input_saves_the_dataset_row_charges_it_and_sets_terminal_status() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "route-result": { "eventPriceUsd": 0.002 },
                        "apify-default-dataset-item": { "eventPriceUsd": 0.001 },
                        "apify-actor-start": { "eventPriceUsd": 0.0002 }
                    }}
                },
                "chargedEventCounts": { "apify-actor-start": 1 },
                "options": { "maxTotalChargeUsd": 0.0032 }
            }
        });
        let input = json!({ "origin": "Times Square", "destination": "Central Park" });
        let responses = vec![
            (200, run.to_string()),
            (200, input.to_string()),
            (201, "{}".to_owned()),
            (201, "{}".to_owned()),
            (200, "{}".to_owned()),
        ];
        let (api_base, server) = mock_server(responses);
        let apify = client(&api_base);
        let run_info = apify.get_run().await.unwrap();
        let mut budget = PpeBudget::from_run(&run_info).unwrap();

        assert_eq!(apify.get_input().await.unwrap(), Some(input));
        let row = json!({ "request_index": 0, "alternative_index": 0, "distance": 10 });
        let mut writer = ApifyRouteWriter::new(&apify, &mut budget, "run");
        assert!(writer.can_save());
        let saved = writer.save(&row).await.unwrap();
        assert_eq!(
            saved,
            RouteSaveResult {
                saved: true,
                charged_count: 1,
                charge_limit_reached: true,
            }
        );
        assert!(!writer.can_save());
        apify
            .set_status_message("Charge limit reached after saving 1 route alternative(s).")
            .await
            .unwrap();

        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 5);
        assert!(requests[0].starts_with("GET /v2/actor-runs/run HTTP/1.1\r\n"));
        assert!(
            requests[1].starts_with("GET /v2/key-value-stores/store/records/INPUT HTTP/1.1\r\n")
        );
        assert!(requests[2].starts_with("POST /v2/datasets/dataset/items HTTP/1.1\r\n"));
        assert_eq!(request_body(&requests[2]), json!([row]));
        assert!(requests[3].starts_with("POST /v2/actor-runs/run/charge HTTP/1.1\r\n"));
        assert!(has_header(
            &requests[3],
            "idempotency-key",
            "run-route-result-0-0"
        ));
        assert_eq!(
            request_body(&requests[3]),
            json!({ "eventName": "route-result", "count": 1 })
        );
        assert!(requests[4].starts_with("PUT /v2/actor-runs/run HTTP/1.1\r\n"));
        assert_eq!(
            request_body(&requests[4]),
            json!({
                "statusMessage": "Charge limit reached after saving 1 route alternative(s).",
                "isStatusMessageTerminal": true
            })
        );
        for request in requests {
            assert!(has_header(&request, "authorization", "Bearer test-token"));
        }
    }

    fn client(api_base: &str) -> ApifyClient {
        let config = ApifyConfig {
            api_base: Url::parse(api_base).unwrap(),
            token: "test-token".to_owned(),
            run_id: "run".to_owned(),
            key_value_store_id: "store".to_owned(),
            dataset_id: "dataset".to_owned(),
            input_key: "INPUT".to_owned(),
        };
        ApifyClient::new(config).unwrap()
    }

    fn mock_server(responses: Vec<(u16, String)>) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_request(&mut stream);
                requests.push(request);
                let reason = if status < 300 { "OK" } else { "Error" };
                write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
                stream.flush().unwrap();
            }
            requests
        });
        (format!("http://{address}"), handle)
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut reader = BufReader::new(stream);
        let mut request = String::new();
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap() > 0 {
            let done = line == "\r\n";
            request.push_str(&line);
            line.clear();
            if done {
                break;
            }
        }
        let content_length = request
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|value| value.trim().parse::<usize>().ok())
            })
            .unwrap_or(0);
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        request.push_str(&String::from_utf8_lossy(&body));
        request
    }

    fn request_body(request: &str) -> Value {
        let body = request.split_once("\r\n\r\n").unwrap().1;
        serde_json::from_str(body).unwrap()
    }

    fn has_header(request: &str, expected_name: &str, expected_value: &str) -> bool {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .skip(1)
            .any(|line| {
                let Some((name, value)) = line.split_once(':') else {
                    return false;
                };
                name.eq_ignore_ascii_case(expected_name) && value.trim() == expected_value
            })
    }
}
