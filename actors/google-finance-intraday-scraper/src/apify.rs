use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Value};
use url::Url;

use crate::{
    config::{base_url_from_env, endpoint_url, required_env, Config, APIFY_API_DEFAULT},
    pricing::{affordable_point_count, ChargeBudget, INTRADAY_PRICE_POINT_CHARGE_EVENT},
};

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

async fn update_terminal_status_message(
    http: &Client,
    apify_api_base_url: &Url,
    apify_token: &str,
    actor_run_id: &str,
    status_message: &str,
) -> Result<()> {
    let url = endpoint_url(apify_api_base_url, &["v2", "actor-runs", actor_run_id])?;
    let response = http
        .put(url)
        .bearer_auth(apify_token)
        .header(header::ACCEPT, "application/json")
        .json(&json!({
            "runId": actor_run_id,
            "statusMessage": status_message,
            "isStatusMessageTerminal": true,
        }))
        .send()
        .await
        .context("Apify run status update failed")?;
    ensure_success(response, "Apify run status update").await
}

pub(crate) async fn update_terminal_status_message_from_env(status_message: &str) -> Result<()> {
    let apify_api_base_url = base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?;
    let apify_token = required_env("APIFY_TOKEN")?;
    let actor_run_id = required_env("ACTOR_RUN_ID")?;
    let http = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("Failed to create the Apify API client for the status message")?;
    update_terminal_status_message(
        &http,
        &apify_api_base_url,
        &apify_token,
        &actor_run_id,
        status_message,
    )
    .await
}

pub(crate) struct DatasetPushResult {
    pub(crate) saved_count: usize,
    pub(crate) charge_limit_reached: bool,
}

pub(crate) struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
    charge_sequence: u64,
    charge_budget: ChargeBudget,
}

impl<'a> ApifyClient<'a> {
    pub(crate) fn new(http: &'a Client, config: &'a Config) -> Self {
        Self {
            http,
            config,
            charge_sequence: 0,
            charge_budget: ChargeBudget::default(),
        }
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base_url, segments)
    }

    pub(crate) async fn get_input(&self) -> Result<Option<Value>> {
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
            .bearer_auth(&self.config.apify_token)
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

    pub(crate) async fn put_output(&self, output: &Value) -> Result<()> {
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
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }

    pub(crate) async fn set_terminal_status_message(&self, status_message: &str) -> Result<()> {
        update_terminal_status_message(
            self.http,
            &self.config.apify_api_base_url,
            &self.config.apify_token,
            &self.config.actor_run_id,
            status_message,
        )
        .await
    }

    pub(crate) async fn push_dataset_items(
        &mut self,
        items: &[Value],
    ) -> Result<DatasetPushResult> {
        if items.is_empty() {
            return Ok(DatasetPushResult {
                saved_count: 0,
                charge_limit_reached: false,
            });
        }

        let run_url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let run_response = self
            .http
            .get(run_url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = response_json(run_response, "Apify run pricing request").await?;
        let charge_count = affordable_point_count(&run, items.len(), &mut self.charge_budget)?;

        let Some(charge_count) = charge_count else {
            self.store_dataset_items(items).await?;
            return Ok(DatasetPushResult {
                saved_count: items.len(),
                charge_limit_reached: false,
            });
        };

        let charge_limit_reached = charge_count < items.len();
        if charge_count == 0 {
            return Ok(DatasetPushResult {
                saved_count: 0,
                charge_limit_reached,
            });
        }
        self.store_dataset_items(&items[..charge_count]).await?;
        self.charge_points(charge_count).await?;
        Ok(DatasetPushResult {
            saved_count: charge_count,
            charge_limit_reached,
        })
    }

    async fn charge_points(&mut self, count: usize) -> Result<()> {
        self.charge_sequence = self
            .charge_sequence
            .checked_add(1)
            .ok_or_else(|| anyhow!("Apify charge request count overflowed"))?;
        let idempotency_key = format!(
            "{}-{}-{}",
            self.config.actor_run_id,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            self.charge_sequence,
        );
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .header("idempotency-key", idempotency_key)
            .json(&json!({
                "eventName": INTRADAY_PRICE_POINT_CHARGE_EVENT,
                "count": count,
            }))
            .send()
            .await
            .context("Apify intraday price point charge request failed")?;
        ensure_success(response, "Apify intraday price point charge request").await?;
        self.charge_budget.confirm_point_charges(count)?;
        Ok(())
    }

    async fn store_dataset_items(&self, items: &[Value]) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    fn read_http_request(stream: &mut TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut byte = [0; 1];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }

        let headers = String::from_utf8_lossy(&request);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        let body_start = request.len();
        request.resize(body_start + content_length, 0);
        stream.read_exact(&mut request[body_start..]).unwrap();
        request
    }

    fn mock_apify_server(
        responses: Vec<(u16, &'static str, String)>,
    ) -> (Url, thread::JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            responses
                .into_iter()
                .map(|(status, reason, body)| {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_http_request(&mut stream);
                    write!(
                        stream,
                        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                    .unwrap();
                    request
                })
                .collect()
        });
        (Url::parse(&format!("http://{address}")).unwrap(), server)
    }

    fn test_apify_config(api_base_url: Url) -> Config {
        Config {
            apify_api_base_url: api_base_url,
            scrappa_api_base_url: Url::parse("https://scrappa.co/api").unwrap(),
            apify_token: "test-apify-token".to_owned(),
            key_value_store_id: "test-store".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    fn intraday_pricing_response() -> String {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "intraday-price-point": { "eventPriceUsd": 0.2 },
                        "another-event": { "eventPriceUsd": 0.1 }
                    }}
                },
                "chargedEventCounts": {
                    "intraday-price-point": 1,
                    "another-event": 1
                },
                "options": { "maxTotalChargeUsd": 0.75 }
            }
        })
        .to_string()
    }

    fn request_path(request: &[u8]) -> &str {
        std::str::from_utf8(request)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
    }

    fn request_body(request: &[u8]) -> Value {
        let body_start = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
            .unwrap();
        serde_json::from_slice(&request[body_start..]).unwrap()
    }

    #[tokio::test]
    async fn failed_dataset_write_does_not_charge() {
        let (api_base_url, server) = mock_apify_server(vec![
            (200, "OK", intraday_pricing_response()),
            (
                503,
                "Service Unavailable",
                r#"{"error":"temporary dataset failure"}"#.to_owned(),
            ),
        ]);
        let http = Client::new();
        let config = test_apify_config(api_base_url);
        let mut apify = ApifyClient::new(&http, &config);
        let items = [json!({ "price": 198.42 }), json!({ "price": 199.01 })];

        let error = match apify.push_dataset_items(&items).await {
            Ok(_) => panic!("failed dataset write unexpectedly succeeded"),
            Err(error) => error,
        };

        assert!(error
            .to_string()
            .contains("Apify dataset write failed with 503 Service Unavailable"));
        assert_eq!(apify.charge_budget.confirmed_point_charges(), 0);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_path(&requests[0]), "/v2/actor-runs/test-run");
        assert_eq!(
            request_path(&requests[1]),
            "/v2/datasets/test-dataset/items"
        );
    }

    #[tokio::test]
    async fn successful_dataset_write_is_charged_after_saving_affordable_points() {
        let (api_base_url, server) = mock_apify_server(vec![
            (200, "OK", intraday_pricing_response()),
            (201, "Created", String::new()),
            (200, "OK", String::new()),
        ]);
        let http = Client::new();
        let config = test_apify_config(api_base_url);
        let mut apify = ApifyClient::new(&http, &config);
        let items = [
            json!({ "price": 198.42 }),
            json!({ "price": 199.01 }),
            json!({ "price": 199.5 }),
        ];

        let result = apify.push_dataset_items(&items).await.unwrap();

        assert_eq!(result.saved_count, 2);
        assert!(result.charge_limit_reached);
        assert_eq!(apify.charge_budget.confirmed_point_charges(), 2);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_path(&requests[0]), "/v2/actor-runs/test-run");
        assert_eq!(
            request_path(&requests[1]),
            "/v2/datasets/test-dataset/items"
        );
        assert_eq!(
            request_body(&requests[1]),
            json!([{ "price": 198.42 }, { "price": 199.01 }])
        );
        assert_eq!(request_path(&requests[2]), "/v2/actor-runs/test-run/charge");
        assert_eq!(
            request_body(&requests[2]),
            json!({ "eventName": "intraday-price-point", "count": 2 })
        );
    }

    #[tokio::test]
    async fn non_pay_per_event_dataset_write_skips_custom_charging() {
        let non_pay_per_event = json!({
            "data": { "pricingInfo": { "pricingModel": "PRICE_PER_DATASET_ITEM" } }
        })
        .to_string();
        let (api_base_url, server) = mock_apify_server(vec![
            (200, "OK", non_pay_per_event),
            (201, "Created", String::new()),
        ]);
        let http = Client::new();
        let config = test_apify_config(api_base_url);
        let mut apify = ApifyClient::new(&http, &config);
        let items = [json!({ "price": 198.42 })];

        let result = apify.push_dataset_items(&items).await.unwrap();

        assert_eq!(result.saved_count, 1);
        assert!(!result.charge_limit_reached);
        assert_eq!(apify.charge_budget.confirmed_point_charges(), 0);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_path(&requests[0]), "/v2/actor-runs/test-run");
        assert_eq!(
            request_path(&requests[1]),
            "/v2/datasets/test-dataset/items"
        );
    }
}
