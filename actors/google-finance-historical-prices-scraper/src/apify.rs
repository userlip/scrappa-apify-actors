use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, RequestBuilder, Response};
use serde_json::{json, Value};
use url::Url;
use uuid::Uuid;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const PRICE_POINT_EVENT: &str = "price-point";
const DEFAULT_DATASET_EVENT: &str = "apify-default-dataset-item";
const APIFY_API_MAX_RETRIES: u32 = 8;
const APIFY_API_RETRY_BASE_DELAY: Duration = Duration::from_millis(500);

#[derive(Clone)]
pub struct ActorConfig {
    pub apify_api_base_url: Url,
    pub scrappa_api_base_url: Url,
    pub scrappa_api_key: Option<String>,
    apify_token: String,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
}

impl ActorConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            scrappa_api_key: env::var("SCRAPPA_API_KEY").ok(),
            apify_token: required_env("APIFY_TOKEN")?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|key| !key.is_empty())
                .unwrap_or_else(|| "INPUT".to_owned()),
        })
    }
}

pub struct ActorApi {
    config: ActorConfig,
    http: Client,
    retry_base_delay: Duration,
}

impl ActorApi {
    pub fn new(config: ActorConfig) -> Result<Self> {
        Self::with_retry_base_delay(config, APIFY_API_RETRY_BASE_DELAY)
    }

    fn with_retry_base_delay(config: ActorConfig, retry_base_delay: Duration) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to initialize Apify API client")?;
        Ok(Self {
            config,
            http,
            retry_base_delay,
        })
    }

    pub async fn get_input(&self) -> Result<Value> {
        let url = self.key_value_record_url(&self.config.input_key)?;
        let response = self
            .send_with_retries("Apify INPUT request", || {
                self.http
                    .get(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
            })
            .await?;
        response_json(response, "Apify INPUT request").await
    }

    pub async fn set_value(&self, key: &str, value: &Value) -> Result<()> {
        let url = self.key_value_record_url(key)?;
        let response = self
            .send_with_retries("Apify key-value store write", || {
                self.http
                    .put(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
                    .json(value)
            })
            .await?;
        ensure_success(response, "Apify key-value store write").await
    }

    pub async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.api_url(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .send_with_retries("Apify status message update", || {
                self.http
                    .put(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .json(&json!({
                        "runId": self.config.actor_run_id,
                        "statusMessage": message,
                        "isStatusMessageTerminal": true,
                    }))
            })
            .await?;
        ensure_success(response, "Apify status message update").await
    }

    pub async fn push_dataset_items(&self, items: &[Value]) -> Result<DatasetWriteResult> {
        if items.is_empty() {
            return Ok(DatasetWriteResult::default());
        }

        let run = self.get_run().await?;
        let Some(budget) = PpeBudget::from_run(&run)? else {
            self.write_dataset_items(items).await?;
            return Ok(DatasetWriteResult {
                charged_count: items.len(),
                event_charge_limit_reached: false,
            });
        };

        let allowed = budget.affordable_item_count(items.len());
        if allowed == 0 {
            return Ok(DatasetWriteResult {
                charged_count: 0,
                event_charge_limit_reached: true,
            });
        }

        self.write_dataset_items(&items[..allowed]).await?;
        self.charge_price_points(allowed).await?;

        Ok(DatasetWriteResult {
            charged_count: allowed,
            event_charge_limit_reached: allowed < items.len(),
        })
    }

    async fn write_dataset_items(&self, items: &[Value]) -> Result<()> {
        let url = self.api_url(&["v2", "datasets", &self.config.default_dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }

    async fn charge_price_points(&self, count: usize) -> Result<()> {
        let url = self.api_url(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        let idempotency_key = format!(
            "{}-{PRICE_POINT_EVENT}-{}",
            self.config.actor_run_id,
            Uuid::new_v4()
        );
        let response = self
            .send_with_retries("Apify price-point charge", || {
                self.http
                    .post(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header("idempotency-key", &idempotency_key)
                    .json(&json!({ "eventName": PRICE_POINT_EVENT, "count": count }))
            })
            .await?;
        ensure_success(response, "Apify price-point charge").await
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.api_url(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .send_with_retries("Apify run pricing request", || {
                self.http
                    .get(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
            })
            .await?;
        response_json(response, "Apify run pricing request").await
    }

    async fn send_with_retries<F>(&self, operation: &str, mut build_request: F) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        for attempt in 0..=APIFY_API_MAX_RETRIES {
            match build_request().send().await {
                Ok(response)
                    if attempt < APIFY_API_MAX_RETRIES
                        && is_retryable_status(response.status()) =>
                {
                    let delay = self.retry_delay(attempt);
                    eprintln!(
                        "{operation} failed with HTTP {}; retrying in {}ms.",
                        response.status().as_u16(),
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Ok(response) => return Ok(response),
                Err(error)
                    if attempt < APIFY_API_MAX_RETRIES && is_retryable_transport_error(&error) =>
                {
                    let delay = self.retry_delay(attempt);
                    eprintln!(
                        "{operation} failed ({error}); retrying in {}ms.",
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error).with_context(|| format!("{operation} failed")),
            }
        }

        unreachable!("retry loop returns after its final attempt")
    }

    fn retry_delay(&self, retry_number: u32) -> Duration {
        let multiplier = 1_u32 << retry_number.min(7);
        self.retry_base_delay
            .saturating_mul(multiplier)
            .min(Duration::from_secs(60))
    }

    fn key_value_record_url(&self, key: &str) -> Result<Url> {
        self.api_url(&[
            "v2",
            "key-value-stores",
            &self.config.default_key_value_store_id,
            "records",
            key,
        ])
    }

    fn api_url(&self, segments: &[&str]) -> Result<Url> {
        let mut url = self.config.apify_api_base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("Apify API base URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(segments.iter().copied());
        Ok(url)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct DatasetWriteResult {
    pub charged_count: usize,
    pub event_charge_limit_reached: bool,
}

struct PpeBudget {
    event_prices: serde_json::Map<String, Value>,
    charged_amount: f64,
    max_total_charge: Option<f64>,
}

impl PpeBudget {
    fn from_run(run: &Value) -> Result<Option<Self>> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        if data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(None);
        }

        let event_prices = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?
            .clone();
        let price_point = event_price(&event_prices, PRICE_POINT_EVENT)?
            .ok_or_else(|| anyhow!("Apify run did not provide the price-point event price"))?;
        let dataset_item_price = event_price(&event_prices, DEFAULT_DATASET_EVENT)?.unwrap_or(0.0);

        let options = data
            .get("options")
            .ok_or_else(|| anyhow!("Apify run did not provide run options"))?;
        let max_total_charge = match options.get("maxTotalChargeUsd") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                value
                    .as_f64()
                    .filter(|value| value.is_finite() && *value >= 0.0)
                    .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?,
            ),
        };

        let charged_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_amount = 0.0;
        for (event_name, count) in charged_counts {
            let count = count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            let price = match event_price(&event_prices, event_name)? {
                Some(price) => price,
                None => 0.0,
            };
            charged_amount += price * count as f64;
        }

        if !price_point.is_finite()
            || price_point < 0.0
            || !dataset_item_price.is_finite()
            || dataset_item_price < 0.0
            || !charged_amount.is_finite()
        {
            bail!("Apify run returned invalid charging values");
        }

        Ok(Some(Self {
            event_prices,
            charged_amount: round_to_six_decimals(charged_amount),
            max_total_charge,
        }))
    }

    fn affordable_item_count(&self, requested: usize) -> usize {
        let Some(max_total_charge) = self.max_total_charge else {
            return requested;
        };
        let price_point = event_price(&self.event_prices, PRICE_POINT_EVENT)
            .ok()
            .flatten()
            .unwrap_or(0.0);
        let dataset_item = event_price(&self.event_prices, DEFAULT_DATASET_EVENT)
            .ok()
            .flatten()
            .unwrap_or(0.0);
        let combined_price = price_point + dataset_item;
        if combined_price == 0.0 {
            return requested;
        }

        let remaining = max_total_charge - self.charged_amount;
        if remaining <= 0.0 {
            return 0;
        }
        let possible = (((remaining / combined_price) * 10_000.0).round() / 10_000.0).floor();
        if !possible.is_finite() || possible <= 0.0 {
            return 0;
        }
        requested.min(possible as usize)
    }
}

fn event_price(events: &serde_json::Map<String, Value>, name: &str) -> Result<Option<f64>> {
    let Some(event) = events.get(name) else {
        return Ok(None);
    };
    let Some(value) = event.get("eventPriceUsd") else {
        return Ok(None);
    };
    let price = value
        .as_f64()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| anyhow!("Invalid price for charge event {name}"))?;
    Ok(Some(price))
}

fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    status.as_u16() == 429 || status.is_server_error()
}

fn is_retryable_transport_error(error: &reqwest::Error) -> bool {
    error.is_connect() || error.is_timeout() || error.is_body() || error.is_request()
}

fn round_to_six_decimals(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
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

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("{operation} response could not be read"))?;
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
    use std::time::Duration;

    use serde_json::{json, Value};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };
    use url::Url;

    use super::{ActorApi, ActorConfig, PpeBudget};

    fn actor_api(server_address: std::net::SocketAddr, retry_delay: Duration) -> ActorApi {
        ActorApi::with_retry_base_delay(
            ActorConfig {
                apify_api_base_url: Url::parse(&format!("http://{server_address}")).unwrap(),
                scrappa_api_base_url: Url::parse("https://scrappa.test/api").unwrap(),
                scrappa_api_key: Some("scrappa-key".to_owned()),
                apify_token: "apify-token".to_owned(),
                default_key_value_store_id: "store-id".to_owned(),
                default_dataset_id: "dataset-id".to_owned(),
                actor_run_id: "run-id".to_owned(),
                input_key: "INPUT".to_owned(),
            },
            retry_delay,
        )
        .unwrap()
    }

    async fn read_http_request(stream: &mut TcpStream) -> (String, Vec<u8>) {
        let mut headers = Vec::new();
        let mut byte = [0_u8; 1];
        loop {
            stream.read_exact(&mut byte).await.unwrap();
            headers.push(byte[0]);
            if headers.ends_with(b"\r\n\r\n") {
                break;
            }
        }

        let headers = String::from_utf8(headers).unwrap();
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        let mut body = vec![0; content_length];
        stream.read_exact(&mut body).await.unwrap();
        (headers, body)
    }

    async fn write_http_response(stream: &mut TcpStream, status: u16, body: &str) {
        let reason = match status {
            200 => "OK",
            201 => "Created",
            503 => "Service Unavailable",
            _ => "Unknown",
        };
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    }

    #[tokio::test]
    async fn caps_ppe_dataset_items_and_charges_price_point_events() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (headers, _) = read_http_request(&mut stream).await;
            let headers = headers.to_ascii_lowercase();
            assert!(headers.starts_with("get /v2/actor-runs/run-id "));
            assert!(headers.contains("authorization: bearer apify-token"));
            write_http_response(
                &mut stream,
                200,
                &serde_json::to_string(&run(json!(0.00075), json!({"apify-actor-start": 1})))
                    .unwrap(),
            )
            .await;

            let (mut stream, _) = listener.accept().await.unwrap();
            let (headers, body) = read_http_request(&mut stream).await;
            assert!(headers
                .to_ascii_lowercase()
                .starts_with("post /v2/datasets/dataset-id/items "));
            assert_eq!(
                serde_json::from_slice::<Value>(&body)
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .len(),
                2
            );
            write_http_response(&mut stream, 201, "{}").await;

            let (mut stream, _) = listener.accept().await.unwrap();
            let (headers, body) = read_http_request(&mut stream).await;
            let headers = headers.to_ascii_lowercase();
            assert!(headers.starts_with("post /v2/actor-runs/run-id/charge "));
            assert!(headers.contains("idempotency-key: run-id-price-point-"));
            assert_eq!(
                serde_json::from_slice::<Value>(&body).unwrap(),
                json!({ "eventName": "price-point", "count": 2 })
            );
            write_http_response(&mut stream, 200, "{}").await;
        });

        let api = actor_api(address, Duration::ZERO);
        let items = (0..5)
            .map(|index| json!({"position": index + 1}))
            .collect::<Vec<_>>();
        let result = api.push_dataset_items(&items).await.unwrap();
        assert_eq!(result.charged_count, 2);
        assert!(result.event_charge_limit_reached);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn retries_price_point_charge_with_the_same_idempotency_key() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = read_http_request(&mut stream).await;
            write_http_response(
                &mut stream,
                200,
                &serde_json::to_string(&run(serde_json::Value::Null, json!({}))).unwrap(),
            )
            .await;

            let (mut stream, _) = listener.accept().await.unwrap();
            let (headers, _) = read_http_request(&mut stream).await;
            assert!(headers.starts_with("POST /v2/datasets/dataset-id/items "));
            write_http_response(&mut stream, 201, "{}").await;

            let (mut stream, _) = listener.accept().await.unwrap();
            let (headers, body) = read_http_request(&mut stream).await;
            assert!(headers.starts_with("POST /v2/actor-runs/run-id/charge "));
            let idempotency_key = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("idempotency-key")
                        .then(|| value.trim().to_owned())
                })
                .unwrap();
            let charge = serde_json::from_slice::<Value>(&body).unwrap();
            assert_eq!(charge, json!({"eventName": "price-point", "count": 1}));
            write_http_response(&mut stream, 503, "temporary error").await;

            let (mut stream, _) = listener.accept().await.unwrap();
            let (headers, body) = read_http_request(&mut stream).await;
            assert!(headers.starts_with("POST /v2/actor-runs/run-id/charge "));
            let retry_key = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("idempotency-key")
                        .then(|| value.trim().to_owned())
                })
                .unwrap();
            assert_eq!(retry_key, idempotency_key);
            assert_eq!(serde_json::from_slice::<Value>(&body).unwrap(), charge);
            write_http_response(&mut stream, 200, "{}").await;
        });

        let api = actor_api(address, Duration::ZERO);
        let result = api
            .push_dataset_items(&[json!({"symbol": "AAPL", "date": 1})])
            .await
            .unwrap();

        assert_eq!(result.charged_count, 1);
        assert!(!result.event_charge_limit_reached);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn retries_apify_api_server_errors_before_writing_non_ppe_results() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let (headers, _) = read_http_request(&mut stream).await;
            assert!(headers.starts_with("GET /v2/actor-runs/run-id "));
            write_http_response(&mut stream, 503, "temporary error").await;

            let (mut stream, _) = listener.accept().await.unwrap();
            let (headers, _) = read_http_request(&mut stream).await;
            assert!(headers.starts_with("GET /v2/actor-runs/run-id "));
            write_http_response(
                &mut stream,
                200,
                r#"{"data":{"pricingInfo":{"pricingModel":"FREE"}}}"#,
            )
            .await;

            let (mut stream, _) = listener.accept().await.unwrap();
            let (headers, body) = read_http_request(&mut stream).await;
            assert!(headers.starts_with("POST /v2/datasets/dataset-id/items "));
            assert_eq!(
                serde_json::from_slice::<Value>(&body)
                    .unwrap()
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            write_http_response(&mut stream, 201, "{}").await;
        });

        let api = actor_api(address, Duration::ZERO);
        let result = api
            .push_dataset_items(&[json!({"symbol": "AAPL"})])
            .await
            .unwrap();
        assert_eq!(result.charged_count, 1);
        assert!(!result.event_charge_limit_reached);
        server.await.unwrap();
    }

    #[tokio::test]
    async fn does_not_retry_dataset_append_after_lost_response() {
        assert_dataset_append_is_not_retried(None).await;
    }

    #[tokio::test]
    async fn does_not_retry_dataset_append_after_server_error() {
        assert_dataset_append_is_not_retried(Some(503)).await;
    }

    async fn assert_dataset_append_is_not_retried(response_status: Option<u16>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let items = [json!({"symbol": "AAPL", "date": 1})];
        let api = actor_api(address, Duration::ZERO);
        let mut write = tokio::spawn(async move { api.push_dataset_items(&items).await });

        let (mut stream, _) = listener.accept().await.unwrap();
        let _ = read_http_request(&mut stream).await;
        write_http_response(
            &mut stream,
            200,
            r#"{"data":{"pricingInfo":{"pricingModel":"FREE"}}}"#,
        )
        .await;

        let (mut stream, _) = listener.accept().await.unwrap();
        let (headers, body) = read_http_request(&mut stream).await;
        assert!(headers.starts_with("POST /v2/datasets/dataset-id/items "));
        let mut persisted_rows = serde_json::from_slice::<Vec<Value>>(&body).unwrap();
        if let Some(status) = response_status {
            write_http_response(&mut stream, status, "temporary error").await;
        } else {
            drop(stream);
        }

        let retried = tokio::select! {
            result = &mut write => {
                assert!(
                    result.unwrap().is_err(),
                    "an ambiguous append failure must remain an error"
                );
                false
            }
            retry = listener.accept() => {
                let (mut stream, _) = retry.unwrap();
                let (_, body) = read_http_request(&mut stream).await;
                persisted_rows.extend(serde_json::from_slice::<Vec<Value>>(&body).unwrap());
                write_http_response(&mut stream, 201, "{}").await;
                true
            }
        };

        if retried {
            write.await.unwrap().unwrap();
        }

        assert!(!retried, "an ambiguous append must not be replayed");
        assert_eq!(persisted_rows.len(), 1, "the batch was persisted only once");
    }

    fn run(
        max_total_charge: serde_json::Value,
        charged_counts: serde_json::Value,
    ) -> serde_json::Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "price-point": {"eventPriceUsd": 0.0002},
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                            "apify-actor-start": {"eventPriceUsd": 0.00005}
                        }
                    }
                },
                "chargedEventCounts": charged_counts,
                "options": {"maxTotalChargeUsd": max_total_charge}
            }
        })
    }

    #[test]
    fn budget_counts_custom_result_and_default_dataset_events() {
        let run = run(json!(0.00075), json!({"apify-actor-start": 1}));
        let budget = PpeBudget::from_run(&run).unwrap().unwrap();
        assert_eq!(budget.affordable_item_count(10), 2);
    }

    #[test]
    fn budget_includes_existing_result_charges_and_stops_at_zero() {
        let run = run(
            json!(0.0006),
            json!({
                "price-point": 1,
                "apify-default-dataset-item": 1,
                "apify-actor-start": 1
            }),
        );
        let budget = PpeBudget::from_run(&run).unwrap().unwrap();
        assert_eq!(budget.affordable_item_count(10), 0);
    }

    #[test]
    fn budget_without_a_total_cap_allows_all_requested_rows() {
        let run = run(serde_json::Value::Null, json!({}));
        let budget = PpeBudget::from_run(&run).unwrap().unwrap();
        assert_eq!(budget.affordable_item_count(7), 7);
    }

    #[test]
    fn non_ppe_run_uses_unmetered_dataset_writes() {
        let run = json!({"data": {"pricingInfo": {"pricingModel": "FREE"}}});
        assert!(PpeBudget::from_run(&run).unwrap().is_none());
    }
}
