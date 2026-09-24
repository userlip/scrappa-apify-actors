use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response};
use serde_json::{json, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const API_TIMEOUT: Duration = Duration::from_secs(60);
pub const FLIGHT_RESULT_CHARGE_EVENT: &str = "flight-result";
const DEFAULT_DATASET_ITEM_CHARGE_EVENT: &str = "apify-default-dataset-item";

pub struct Config {
    apify_api_base: Url,
    scrappa_api_base: Url,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    pub scrappa_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        let apify_base = env::var("APIFY_API_PUBLIC_BASE_URL")
            .or_else(|_| env::var("APIFY_API_BASE_URL"))
            .unwrap_or_else(|_| APIFY_API_DEFAULT.to_owned());
        let scrappa_base = env::var("SCRAPPA_API_BASE_URL")
            .unwrap_or_else(|_| "https://scrappa.co/api".to_owned());
        Ok(Self {
            apify_api_base: Url::parse(&apify_base)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid absolute URL")?,
            scrappa_api_base: Url::parse(&scrappa_base)
                .context("SCRAPPA_API_BASE_URL must be a valid absolute URL")?,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key,
        })
    }

    pub fn scrappa_api_base(&self) -> &Url {
        &self.scrappa_api_base
    }
}

pub struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

pub struct PushDataResult {
    pub saved_count: usize,
    pub event_charge_limit_reached: bool,
}

impl<'a> ApifyClient<'a> {
    pub fn new(http: &'a Client, config: &'a Config) -> Self {
        Self { http, config }
    }

    pub async fn get_input(&self) -> Result<Value> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .http
            .get(url)
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        response_json(response, "Apify INPUT request").await
    }

    pub async fn push_dataset_items(&self, items: &[Value]) -> Result<PushDataResult> {
        if items.is_empty() {
            return Ok(PushDataResult {
                saved_count: 0,
                event_charge_limit_reached: false,
            });
        }
        let run = self.get_run().await?;
        let is_pay_per_event = run
            .pointer("/data/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        let saved_count = if is_pay_per_event {
            let charge_count =
                affordable_result_count(&run, FLIGHT_RESULT_CHARGE_EVENT, items.len())?;
            if charge_count == 0 {
                return Ok(PushDataResult {
                    saved_count: 0,
                    event_charge_limit_reached: true,
                });
            }
            let idempotency_key = format!(
                "{}:{FLIGHT_RESULT_CHARGE_EVENT}:results:1:count:{charge_count}",
                self.config.actor_run_id
            );
            self.charge_events(FLIGHT_RESULT_CHARGE_EVENT, charge_count, &idempotency_key)
                .await?;
            self.store_dataset_items(&items[..charge_count]).await?;
            charge_count
        } else {
            self.store_dataset_items(items).await?;
            items.len()
        };

        Ok(PushDataResult {
            saved_count,
            event_charge_limit_reached: is_pay_per_event && saved_count < items.len(),
        })
    }

    pub async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .http
            .put(url)
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }

    pub async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .put(url)
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(&json!({
                "runId": self.config.actor_run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true,
            }))
            .send()
            .await
            .context("Apify run status message request failed")?;
        ensure_success(response, "Apify run status message request").await
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .get(url)
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        response_json(response, "Apify run pricing request").await
    }

    async fn charge_events(
        &self,
        event_name: &str,
        count: usize,
        idempotency_key: &str,
    ) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        let response = self
            .http
            .post(url)
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .header("idempotency-key", idempotency_key)
            .json(&json!({
                "eventName": event_name,
                "count": count,
            }))
            .send()
            .await
            .context("Apify event charge request failed")?;
        ensure_success(response, "Apify event charge request").await
    }

    async fn store_dataset_items(&self, items: &[Value]) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .timeout(API_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
    }
}

pub fn affordable_result_count(run: &Value, event_name: &str, requested: usize) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        bail!("Apify run is not configured for pay-per-event pricing");
    }
    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let result_event_price = required_event_price(events, event_name)?;
    let dataset_item_price = optional_event_price(events, DEFAULT_DATASET_ITEM_CHARGE_EVENT)?;
    let price_per_result = result_event_price + dataset_item_price;
    if !price_per_result.is_finite() {
        bail!("Apify run returned invalid charging values");
    }
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => f64::INFINITY,
        Some(value) => value
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?,
    };
    if max_charge.is_nan() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    let empty_counts = serde_json::Map::new();
    let counts = match data.get("chargedEventCounts") {
        None | Some(Value::Null) => &empty_counts,
        Some(Value::Object(counts)) => counts,
        Some(_) => bail!("Apify run returned invalid charged event counts"),
    };
    let mut spent = 0.0;
    for (charged_event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {charged_event_name}"))?;
        if count == 0 {
            continue;
        }
        let price = optional_event_price(events, charged_event_name)?;
        if !price.is_finite() {
            bail!("Invalid price for charged event {charged_event_name}");
        }
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if price_per_result == 0.0 || max_charge.is_infinite() {
        return Ok(requested);
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * price_per_result <= max_charge + tolerance)
        .count())
}

fn required_event_price(events: &serde_json::Map<String, Value>, event_name: &str) -> Result<f64> {
    let event = events
        .get(event_name)
        .ok_or_else(|| anyhow!("Apify run did not provide the {event_name} event price"))?;
    let price = event
        .get("eventPriceUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the {event_name} event price"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned an invalid price for {event_name}");
    }
    Ok(price)
}

fn optional_event_price(events: &serde_json::Map<String, Value>, event_name: &str) -> Result<f64> {
    let Some(event) = events.get(event_name) else {
        return Ok(0.0);
    };
    let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) else {
        return Ok(0.0);
    };
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned an invalid price for {event_name}");
    }
    Ok(price)
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read API response")?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn run(max_charge: Value, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "flight-result": {"eventPriceUsd": 0.2},
                            "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                            "actor-start": {"eventPriceUsd": 0.1}
                        }
                    }
                },
                "chargedEventCounts": counts,
                "options": {"maxTotalChargeUsd": max_charge}
            }
        })
    }

    #[test]
    fn budget_counts_prior_result_charges_and_other_events() {
        let run = run(json!(1.0), json!({"flight-result": 1, "actor-start": 1}));
        assert_eq!(
            affordable_result_count(&run, FLIGHT_RESULT_CHARGE_EVENT, 10).unwrap(),
            2
        );
    }

    #[test]
    fn missing_budget_and_charge_counts_mean_unlimited_and_unspent() {
        let run = json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "flight-result": {"eventPriceUsd": 0.2},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.1}
                }}
            },
            "options": {"maxTotalChargeUsd": null}
        }});
        assert_eq!(
            affordable_result_count(&run, FLIGHT_RESULT_CHARGE_EVENT, 10).unwrap(),
            10
        );
    }

    #[test]
    fn missing_prices_and_charge_counts_fail_closed() {
        let mut missing_event = run(json!(1.0), json!({}));
        missing_event["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove(FLIGHT_RESULT_CHARGE_EVENT);
        assert!(affordable_result_count(&missing_event, FLIGHT_RESULT_CHARGE_EVENT, 1).is_err());

        let mut invalid_counts = run(json!(1.0), json!({}));
        invalid_counts["data"]["chargedEventCounts"] = json!("invalid");
        assert!(affordable_result_count(&invalid_counts, FLIGHT_RESULT_CHARGE_EVENT, 1).is_err());
    }

    #[tokio::test]
    async fn input_and_output_use_the_default_key_value_store_records() {
        let input = json!({
            "trip_type": "one_way",
            "origin": "JFK",
            "destination": "LAX",
            "departure_date": "2026-09-15"
        });
        let output = json!({"flights": [], "provider_field": "preserved"});
        let (base_url, server) = mock_server(vec![(200, input.to_string()), (201, String::new())]);
        let config = test_config(&base_url);
        let http = Client::new();
        let apify = ApifyClient::new(&http, &config);

        assert_eq!(apify.get_input().await.unwrap(), input);
        apify.put_output(&output).await.unwrap();

        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(
            requests[0].target,
            "/v2/key-value-stores/store-id/records/INPUT"
        );
        assert_eq!(requests[1].method, "PUT");
        assert_eq!(
            requests[1].target,
            "/v2/key-value-stores/store-id/records/OUTPUT"
        );
        assert_eq!(
            serde_json::from_str::<Value>(&requests[1].body).unwrap(),
            output
        );
        assert_authorized(&requests);
    }

    #[tokio::test]
    async fn pay_per_event_storage_charges_and_writes_only_affordable_results() {
        let run = json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "flight-result": {"eventPriceUsd": 0.2},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                    "apify-actor-start": {"eventPriceUsd": 0.05}
                }}
            },
            "chargedEventCounts": {"apify-actor-start": 1},
            "options": {"maxTotalChargeUsd": 0.75}
        }});
        let (base_url, server) = mock_server(vec![
            (200, run.to_string()),
            (200, "{}".to_owned()),
            (201, String::new()),
        ]);
        let config = test_config(&base_url);
        let http = Client::new();
        let apify = ApifyClient::new(&http, &config);
        let items = vec![
            json!({"position": 1}),
            json!({"position": 2}),
            json!({"position": 3}),
        ];

        let result = apify.push_dataset_items(&items).await.unwrap();

        assert_eq!(result.saved_count, 2);
        assert!(result.event_charge_limit_reached);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].target, "/v2/actor-runs/run-id");
        assert_eq!(requests[1].method, "POST");
        assert_eq!(requests[1].target, "/v2/actor-runs/run-id/charge");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[1].body).unwrap(),
            json!({"eventName": "flight-result", "count": 2})
        );
        assert!(requests[1].headers.contains_key("idempotency-key"));
        assert_eq!(requests[2].method, "POST");
        assert_eq!(requests[2].target, "/v2/datasets/dataset-id/items");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[2].body).unwrap(),
            json!([{"position": 1}, {"position": 2}])
        );
        assert_authorized(&requests);
    }

    #[tokio::test]
    async fn pay_per_event_budget_exhaustion_skips_charge_and_dataset_write() {
        let run = json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "flight-result": {"eventPriceUsd": 0.2},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                    "apify-actor-start": {"eventPriceUsd": 0.05}
                }}
            },
            "chargedEventCounts": {"apify-actor-start": 1},
            "options": {"maxTotalChargeUsd": 0.05}
        }});
        let (base_url, server) = mock_server(vec![(200, run.to_string())]);
        let config = test_config(&base_url);
        let http = Client::new();
        let apify = ApifyClient::new(&http, &config);

        let result = apify
            .push_dataset_items(&[json!({"position": 1})])
            .await
            .unwrap();

        assert_eq!(result.saved_count, 0);
        assert!(result.event_charge_limit_reached);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].target, "/v2/actor-runs/run-id");
    }

    fn test_config(api_base: &str) -> Config {
        Config {
            apify_api_base: Url::parse(api_base).unwrap(),
            scrappa_api_base: Url::parse(api_base).unwrap(),
            apify_token: "apify-test-token".to_owned(),
            key_value_store_id: "store-id".to_owned(),
            dataset_id: "dataset-id".to_owned(),
            actor_run_id: "run-id".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
        }
    }

    fn assert_authorized(requests: &[CapturedRequest]) {
        assert!(requests.iter().all(|request| {
            request.headers.get("authorization").map(String::as_str)
                == Some("Bearer apify-test-token")
        }));
    }

    struct CapturedRequest {
        method: String,
        target: String,
        headers: std::collections::HashMap<String, String>,
        body: String,
    }

    fn mock_server(
        responses: Vec<(u16, String)>,
    ) -> (String, std::thread::JoinHandle<Vec<CapturedRequest>>) {
        use std::io::{BufRead, BufReader, Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let mut requests = Vec::with_capacity(responses.len());
            for (status, response_body) in responses {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream);
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or_default().to_owned();
                let target = parts.next().unwrap_or_default().to_owned();
                let mut headers = std::collections::HashMap::new();
                let mut content_length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':') {
                        let name = name.trim().to_ascii_lowercase();
                        let value = value.trim().to_owned();
                        if name == "content-length" {
                            content_length = value.parse().unwrap_or_default();
                        }
                        headers.insert(name, value);
                    }
                }
                let mut body = vec![0; content_length];
                reader.read_exact(&mut body).unwrap();
                let body = String::from_utf8(body).unwrap();
                let mut stream = reader.into_inner();
                let reason = if status == 201 { "Created" } else { "OK" };
                write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                )
                .unwrap();
                requests.push(CapturedRequest {
                    method,
                    target,
                    headers,
                    body,
                });
            }
            requests
        });
        (format!("http://{address}"), server)
    }
}
