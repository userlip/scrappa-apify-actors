use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, Url};
use serde_json::{json, Value};
use std::{collections::HashMap, env, time::Duration};
use tokio::time::sleep;

pub const COMPANY_DETAIL_RESULT_EVENT: &str = "company-detail-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const APIFY_HTTP_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_MAX_ATTEMPTS: usize = 3;

#[derive(Clone)]
pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
    pub key_value_store_id: String,
    pub dataset_id: String,
    pub actor_run_id: String,
    pub input_key: String,
}

pub struct ChargeBudget {
    event_prices: HashMap<String, f64>,
    charged_event_counts: HashMap<String, u64>,
    max_total_charge_usd: Option<f64>,
}

#[derive(Debug, PartialEq)]
pub struct PushResult {
    pub saved_count: usize,
    pub status_message: Option<String>,
}

impl ApifyClient {
    pub fn from_env() -> Result<Self> {
        let token = required_env("APIFY_TOKEN")?;
        let key_value_store_id = required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?;
        let dataset_id = required_env("ACTOR_DEFAULT_DATASET_ID")?;
        let actor_run_id = required_env("ACTOR_RUN_ID")?;
        let input_key = env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".into());
        let base_url =
            env::var("APIFY_API_PUBLIC_BASE_URL").unwrap_or_else(|_| APIFY_API_DEFAULT.into());
        Self::new(
            &base_url,
            token,
            key_value_store_id,
            dataset_id,
            actor_run_id,
            input_key,
        )
    }

    pub fn new(
        base_url: &str,
        token: String,
        key_value_store_id: String,
        dataset_id: String,
        actor_run_id: String,
        input_key: String,
    ) -> Result<Self> {
        let base_url =
            Url::parse(base_url).context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?;
        let client = Client::builder()
            .timeout(APIFY_HTTP_TIMEOUT)
            .build()
            .context("Could not create Apify HTTP client")?;
        Ok(Self {
            client,
            base_url,
            token,
            key_value_store_id,
            dataset_id,
            actor_run_id,
            input_key,
        })
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(resource.iter().copied()));
        Ok(url)
    }

    fn record_url(&self, store_id: &str, record_key: &str) -> Result<Url> {
        self.resource_url(&["key-value-stores", store_id, "records", record_key])
    }

    fn request(&self, method: Method, url: Url) -> RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    async fn retrying_request<F>(&self, mut request: F, operation: &str) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        for attempt in 1..=APIFY_MAX_ATTEMPTS {
            match request().send().await {
                Ok(response)
                    if attempt < APIFY_MAX_ATTEMPTS && retryable_status(response.status()) =>
                {
                    let status = response.status();
                    let _ = response.bytes().await;
                    let delay_ms = 250 * 2_u64.pow((attempt - 1) as u32);
                    eprintln!(
                        "Apify {operation} request failed with HTTP {status}. Retrying attempt {}/{APIFY_MAX_ATTEMPTS} in {delay_ms}ms.",
                        attempt + 1
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                }
                Ok(response) => return Ok(response),
                Err(error)
                    if attempt < APIFY_MAX_ATTEMPTS
                        && (error.is_timeout() || error.is_connect()) =>
                {
                    let delay_ms = 250 * 2_u64.pow((attempt - 1) as u32);
                    eprintln!(
                        "Apify {operation} request failed ({error}). Retrying attempt {}/{APIFY_MAX_ATTEMPTS} in {delay_ms}ms.",
                        attempt + 1
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify {operation} request failed"));
                }
            }
        }
        unreachable!("the retry loop always returns on its final attempt")
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.record_url(&self.key_value_store_id, &self.input_key)?;
        let response = self
            .retrying_request(
                || self.request(Method::GET, url.clone()),
                "fetch Actor input",
            )
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        successful_response(response, "fetch Actor input")
            .await?
            .json()
            .await
            .map(Some)
            .context("Actor input record is not valid JSON")
    }

    pub async fn get_run_pricing(&self) -> Result<Value> {
        let url = self.resource_url(&["actor-runs", &self.actor_run_id])?;
        successful_response(
            self.retrying_request(
                || self.request(Method::GET, url.clone()),
                "fetch Actor run pricing",
            )
            .await?,
            "fetch Actor run pricing",
        )
        .await?
        .json()
        .await
        .context("Actor run pricing response is not valid JSON")
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let url = self.record_url(&self.key_value_store_id, "OUTPUT")?;
        let response = self
            .retrying_request(
                || self.request(Method::PUT, url.clone()).json(output),
                "write OUTPUT",
            )
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.resource_url(&["actor-runs", &self.actor_run_id])?;
        let body = json!({
            "runId": self.actor_run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true,
        });
        let response = self
            .retrying_request(
                || self.request(Method::PUT, url.clone()).json(&body),
                "set Actor status message",
            )
            .await?;
        successful_response(response, "set Actor status message").await?;
        Ok(())
    }

    pub async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.resource_url(&["datasets", &self.dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(&[item])
            .send()
            .await
            .context("Failed to store item in the default dataset")?;
        successful_response(response, "store item in the default dataset").await?;
        Ok(())
    }

    pub async fn push_charged_item(
        &self,
        item: &Value,
        budget: &mut ChargeBudget,
        item_index: usize,
    ) -> Result<PushResult> {
        if !budget.can_charge_next_item()? {
            return Ok(charge_limit_result(0));
        }

        let url = self.resource_url(&["actor-runs", &self.actor_run_id, "charge"])?;
        let idempotency_key = format!("{}-company-detail-result-{item_index}", self.actor_run_id);
        let body = json!({"eventName": COMPANY_DETAIL_RESULT_EVENT, "count": 1});
        let response = self
            .retrying_request(
                || {
                    self.request(Method::POST, url.clone())
                        .header("idempotency-key", &idempotency_key)
                        .json(&body)
                },
                "charge company detail result",
            )
            .await?;
        successful_response(response, "charge company detail result").await?;
        budget.record_charge(COMPANY_DETAIL_RESULT_EVENT)?;

        self.push_dataset_item(item).await?;
        budget.record_charge(DEFAULT_DATASET_ITEM_EVENT)?;

        let status_message = if !budget.can_charge_next_item()? {
            log_charge_limit(1, 1);
            Some(charge_limit_message(1))
        } else {
            None
        };
        Ok(PushResult {
            saved_count: 1,
            status_message,
        })
    }
}

impl ChargeBudget {
    pub fn from_run(run: &Value) -> Result<Self> {
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

        let mut event_prices = HashMap::new();
        for (name, event) in events {
            let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) else {
                continue;
            };
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned an invalid price for charged event {name}");
            }
            event_prices.insert(name.clone(), price);
        }
        if !event_prices.contains_key(COMPANY_DETAIL_RESULT_EVENT) {
            bail!("Apify run did not provide the {COMPANY_DETAIL_RESULT_EVENT} charge event price");
        }

        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let charged_event_counts = counts
            .iter()
            .map(|(name, count)| {
                count
                    .as_u64()
                    .map(|count| (name.clone(), count))
                    .ok_or_else(|| anyhow!("Invalid charged event count for {name}"))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64);
        if max_total_charge_usd.is_some_and(|limit| !limit.is_finite() || limit < 0.0) {
            bail!("Apify run returned an invalid spending limit");
        }

        Ok(Self {
            event_prices,
            charged_event_counts,
            max_total_charge_usd,
        })
    }

    pub fn can_charge_next_item(&self) -> Result<bool> {
        Ok(self.affordable_item_count()? > 0)
    }

    pub fn record_charge(&mut self, event_name: &str) -> Result<()> {
        let count = self
            .charged_event_counts
            .entry(event_name.into())
            .or_default();
        *count = count
            .checked_add(1)
            .ok_or_else(|| anyhow!("Charged event count overflowed for {event_name}"))?;
        Ok(())
    }

    pub fn affordable_item_count(&self) -> Result<usize> {
        let Some(max_total_charge_usd) = self.max_total_charge_usd else {
            return Ok(usize::MAX);
        };
        let detail_price = *self
            .event_prices
            .get(COMPANY_DETAIL_RESULT_EVENT)
            .ok_or_else(|| anyhow!("Apify run did not provide the {COMPANY_DETAIL_RESULT_EVENT} charge event price"))?;
        let dataset_item_price = self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or_default();
        let item_price = detail_price + dataset_item_price;
        if !item_price.is_finite() {
            bail!("Apify run returned invalid charging values");
        }

        let spent = self
            .charged_event_counts
            .iter()
            .try_fold(0.0, |spent, (name, count)| {
                let price = self.event_prices.get(name).copied().unwrap_or_default();
                let next_spent = spent + price * (*count as f64);
                if next_spent.is_finite() {
                    Ok(next_spent)
                } else {
                    Err(anyhow!("Apify run returned invalid charged totals"))
                }
            })?;
        if item_price == 0.0 {
            return Ok(usize::MAX);
        }

        let tolerance = f64::EPSILON * max_total_charge_usd.max(1.0);
        let available_charge = (max_total_charge_usd - spent + tolerance).max(0.0);
        let affordable_items = (available_charge / item_price).floor();
        Ok(if affordable_items.is_finite() {
            affordable_items as usize
        } else {
            usize::MAX
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is missing"))
}

fn retryable_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
}

fn charge_limit_message(saved_count: usize) -> String {
    format!(
        "Charge limit reached after saving {saved_count} of 1 Trustpilot company detail results."
    )
}

fn charge_limit_result(saved_count: usize) -> PushResult {
    let status_message = charge_limit_message(saved_count);
    log_charge_limit(saved_count, saved_count);
    PushResult {
        saved_count,
        status_message: Some(status_message),
    }
}

fn log_charge_limit(saved_count: usize, charged_count: usize) {
    eprintln!(
        "{} {{\"event\":\"{COMPANY_DETAIL_RESULT_EVENT}\",\"charged_count\":{charged_count},\"requested_count\":1,\"saved_count\":{saved_count}}}",
        charge_limit_message(saved_count)
    );
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
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
        thread::{self, JoinHandle},
        time::Duration,
    };

    struct MockServer {
        base_url: String,
        requests: Receiver<String>,
        thread: JoinHandle<()>,
    }

    impl MockServer {
        fn start(expected_requests: usize) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                for _ in 0..expected_requests {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_request(&mut stream);
                    sender.send(request).unwrap();
                    stream
                        .write_all(
                            b"HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                        )
                        .unwrap();
                }
            });
            Self {
                base_url: format!("http://{address}"),
                requests,
                thread,
            }
        }

        fn request(&self) -> String {
            self.requests.recv_timeout(Duration::from_secs(5)).unwrap()
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let bytes_read = stream.read(&mut buffer).unwrap();
            if bytes_read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..bytes_read]);
            let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n")
            else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or_default();
            if request.len() >= header_end + 4 + content_length {
                break;
            }
        }
        String::from_utf8(request).unwrap()
    }

    fn request_body(request: &str) -> Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    fn pricing_run(max_charge: f64, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "company-detail-result": {"eventPriceUsd": 0.10},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.05},
                        "other-event": {"eventPriceUsd": 0.025}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": counts
            }
        })
    }

    #[test]
    fn budget_counts_custom_dataset_and_other_charged_events() {
        let run = pricing_run(0.30, json!({"other-event": 2}));
        let mut budget = ChargeBudget::from_run(&run).unwrap();
        assert_eq!(budget.affordable_item_count().unwrap(), 1);

        budget.record_charge(COMPANY_DETAIL_RESULT_EVENT).unwrap();
        budget.record_charge(DEFAULT_DATASET_ITEM_EVENT).unwrap();
        assert!(!budget.can_charge_next_item().unwrap());
    }

    #[test]
    fn rejects_missing_ppe_pricing_and_preserves_unbounded_runs() {
        assert!(ChargeBudget::from_run(&json!({"data":{}})).is_err());
        assert!(ChargeBudget::from_run(&json!({
            "data":{"pricingInfo":{"pricingModel":"FREE"}}
        }))
        .is_err());

        let mut run = pricing_run(1.0, json!({}));
        run["data"]["options"] = json!({});
        assert_eq!(
            ChargeBudget::from_run(&run)
                .unwrap()
                .affordable_item_count()
                .unwrap(),
            usize::MAX
        );
    }

    #[test]
    fn returns_a_partial_result_when_the_user_budget_is_exhausted() {
        assert_eq!(
            charge_limit_result(0),
            PushResult {
                saved_count: 0,
                status_message: Some(
                    "Charge limit reached after saving 0 of 1 Trustpilot company detail results."
                        .into()
                )
            }
        );
    }

    #[tokio::test]
    async fn charges_the_custom_event_then_writes_only_the_budgeted_dataset_item() {
        let server = MockServer::start(2);
        let client = ApifyClient::new(
            &server.base_url,
            "test-token".into(),
            "test-store".into(),
            "test-dataset".into(),
            "test-run".into(),
            "INPUT".into(),
        )
        .unwrap();
        let mut budget = ChargeBudget::from_run(&pricing_run(0.15, json!({}))).unwrap();

        let result = client
            .push_charged_item(&json!({"company_domain":"example.com"}), &mut budget, 7)
            .await
            .unwrap();

        assert_eq!(result.saved_count, 1);
        assert_eq!(
            result.status_message.as_deref(),
            Some("Charge limit reached after saving 1 of 1 Trustpilot company detail results.")
        );
        assert!(!budget.can_charge_next_item().unwrap());

        let charge_request = server.request();
        let charge_headers = charge_request.to_ascii_lowercase();
        assert!(charge_request.starts_with("POST /v2/actor-runs/test-run/charge HTTP/1.1"));
        assert!(charge_headers.contains("authorization: bearer test-token"));
        assert!(charge_headers.contains("idempotency-key: test-run-company-detail-result-7"));
        assert_eq!(
            request_body(&charge_request),
            json!({"eventName":"company-detail-result", "count":1})
        );

        let dataset_request = server.request();
        assert!(dataset_request.starts_with("POST /v2/datasets/test-dataset/items HTTP/1.1"));
        assert_eq!(
            request_body(&dataset_request),
            json!([{"company_domain":"example.com"}])
        );
        server.thread.join().unwrap();
    }
}
