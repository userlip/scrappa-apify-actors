use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::{json, Value};
use std::{
    env,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const PROPERTY_RESULT_EVENT: &str = "property-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

static IDEMPOTENCY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct ActorConfig {
    pub apify_api_base_url: Url,
    pub scrappa_api_base_url: String,
    pub default_key_value_store_id: String,
    pub default_dataset_id: String,
    pub actor_run_id: String,
    pub input_key: String,
    pub apify_token: String,
    pub scrappa_api_key: String,
}

impl ActorConfig {
    pub fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").unwrap_or_default();
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: env::var("SCRAPPA_API_BASE_URL")
                .unwrap_or_else(|_| "https://scrappa.co/api".to_owned()),
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
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
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub fn new(base_url: Url, token: String) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url,
            token,
        })
    }

    pub async fn get_input(&self, store_id: &str, input_key: &str) -> Result<Option<Value>> {
        let response = self
            .request(Method::GET, self.record_url(store_id, input_key)?)
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "read INPUT").await?;
        Ok(Some(
            response
                .json()
                .await
                .context("Apify INPUT is not valid JSON")?,
        ))
    }

    pub async fn push_data(
        &self,
        run_id: &str,
        dataset_id: &str,
        items: &[Value],
    ) -> Result<usize> {
        if items.is_empty() {
            return Ok(0);
        }

        let run = self.get_run(run_id).await?;
        if !is_pay_per_event(&run) {
            self.write_dataset(dataset_id, items).await?;
            return Ok(items.len());
        }

        let limit = affordable_property_results(&run, items.len())?;
        if limit == 0 {
            return Ok(0);
        }

        self.charge_event(run_id, PROPERTY_RESULT_EVENT, limit)
            .await?;
        self.write_dataset(dataset_id, &items[..limit]).await?;
        Ok(limit)
    }

    pub async fn set_output(&self, store_id: &str, output: &Value) -> Result<()> {
        let response = self
            .request(Method::PUT, self.record_url(store_id, "OUTPUT")?)
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT request failed")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn set_status_message(&self, run_id: &str, message: &str) -> Result<()> {
        let response = self
            .request(Method::PUT, self.resource_url(&["actor-runs", run_id])?)
            .json(&json!({
                "runId": run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true
            }))
            .send()
            .await
            .context("Apify run status update failed")?;
        successful_response(response, "set run status message").await?;
        Ok(())
    }

    async fn get_run(&self, run_id: &str) -> Result<Value> {
        let response = self
            .request(Method::GET, self.resource_url(&["actor-runs", run_id])?)
            .send()
            .await
            .context("Apify run pricing request failed")?;
        successful_response(response, "read run pricing")
            .await?
            .json()
            .await
            .context("Apify run pricing response is not valid JSON")
    }

    async fn charge_event(&self, run_id: &str, event_name: &str, count: usize) -> Result<()> {
        let idempotency_key = idempotency_key(run_id, event_name);
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["actor-runs", run_id, "charge"])?,
            )
            .header("idempotency-key", idempotency_key)
            .json(&json!({"eventName": event_name, "count": count}))
            .send()
            .await
            .context("Apify property-result charge request failed")?;
        successful_response(response, "charge property-result events").await?;
        Ok(())
    }

    async fn write_dataset(&self, dataset_id: &str, items: &[Value]) -> Result<()> {
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["datasets", dataset_id, "items"])?,
            )
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        successful_response(response, "write dataset items").await?;
        Ok(())
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    fn record_url(&self, store_id: &str, record_key: &str) -> Result<Url> {
        self.resource_url(&["key-value-stores", store_id, "records", record_key])
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(resource.iter().copied()));
        Ok(url)
    }
}

pub fn is_pay_per_event(run: &Value) -> bool {
    run.pointer("/data/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        == Some("PAY_PER_EVENT")
}

pub fn affordable_property_results(run: &Value, requested: usize) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let property_result_price = event_price(events, PROPERTY_RESULT_EVENT)?
        .ok_or_else(|| anyhow!("Apify run did not provide the property-result event price"))?;
    let dataset_item_price = event_price(events, DEFAULT_DATASET_ITEM_EVENT)?.unwrap_or(0.0);
    let Some(max_charge) = data
        .pointer("/options/maxTotalChargeUsd")
        .filter(|value| !value.is_null())
    else {
        return Ok(requested);
    };
    let max_charge = max_charge
        .as_f64()
        .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
    if !property_result_price.is_finite()
        || property_result_price < 0.0
        || !dataset_item_price.is_finite()
        || dataset_item_price < 0.0
        || !max_charge.is_finite()
        || max_charge < 0.0
    {
        bail!("Apify run returned invalid charging values");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    for (event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count"))?;
        let price = event_price(events, event_name)?.unwrap_or(0.0);
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }

    let per_result_price = property_result_price + dataset_item_price;
    if per_result_price == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let available = ((max_charge - spent + tolerance) / per_result_price).floor();
    if !available.is_finite() || available < 1.0 {
        return Ok(0);
    }
    Ok(requested.min(available as usize))
}

fn event_price(events: &serde_json::Map<String, Value>, event_name: &str) -> Result<Option<f64>> {
    let Some(event) = events.get(event_name) else {
        return Ok(None);
    };
    let Some(price) = event.get("eventPriceUsd") else {
        return Ok(None);
    };
    let price = price
        .as_f64()
        .ok_or_else(|| anyhow!("Invalid price for charged event {event_name}"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("Invalid price for charged event {event_name}");
    }
    Ok(Some(price))
}

fn idempotency_key(run_id: &str, event_name: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = IDEMPOTENCY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{run_id}-{event_name}-{nanos}-{sequence}")
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
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
        thread::{self, JoinHandle},
    };

    fn pay_per_event_run(limit: f64, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "property-result": {"eventPriceUsd": 0.0003},
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                            "apify-actor-start": {"eventPriceUsd": 0.0001}
                        }
                    }
                },
                "options": {"maxTotalChargeUsd": limit},
                "chargedEventCounts": counts
            }
        })
    }

    #[test]
    fn combines_result_and_dataset_charges_with_prior_run_spend() {
        let run = pay_per_event_run(0.0005, json!({"apify-actor-start": 1}));
        assert_eq!(affordable_property_results(&run, 20).unwrap(), 1);
    }

    #[test]
    fn allows_rows_up_to_budget_and_zero_price_events() {
        let run = pay_per_event_run(1.0, json!({"apify-actor-start": 1}));
        assert_eq!(affordable_property_results(&run, 2).unwrap(), 2);

        let mut free = run;
        free["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]["property-result"]
            ["eventPriceUsd"] = json!(0);
        free["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            ["apify-default-dataset-item"]["eventPriceUsd"] = json!(0);
        assert_eq!(affordable_property_results(&free, 20).unwrap(), 20);
    }

    #[test]
    fn treats_an_unset_spending_limit_as_unlimited() {
        let mut run = pay_per_event_run(1.0, json!({}));
        run["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
        assert_eq!(affordable_property_results(&run, 7).unwrap(), 7);
    }

    #[test]
    fn blocks_rows_when_spending_limit_is_already_reached() {
        let run = pay_per_event_run(0.0001, json!({"apify-actor-start": 1}));
        assert_eq!(affordable_property_results(&run, 20).unwrap(), 0);
    }

    #[test]
    fn rejects_missing_or_invalid_pricing_metadata() {
        assert!(affordable_property_results(&json!({}), 1).is_err());
        assert!(affordable_property_results(
            &json!({"data": {"pricingInfo": {"pricingPerEvent": {"actorChargeEvents": {}}}}}),
            1
        )
        .is_err());
    }

    #[test]
    fn detects_only_pay_per_event_runs() {
        assert!(is_pay_per_event(&pay_per_event_run(1.0, json!({}))));
        assert!(!is_pay_per_event(
            &json!({"data": {"pricingInfo": {"pricingModel": "FLAT_RATE"}}})
        ));
    }

    #[tokio::test]
    async fn non_ppe_run_writes_all_rows_without_explicit_charge() {
        let server = MockServer::start(vec![
            response(200, r#"{"data":{"pricingInfo":{"pricingModel":"FREE"}}}"#),
            response(201, ""),
        ]);
        let client = ApifyClient::new(server.base_url.clone(), "test-token".into()).unwrap();
        let items = vec![json!({"id": 1}), json!({"id": 2})];

        assert_eq!(
            client
                .push_data("test-run", "test-dataset", &items)
                .await
                .unwrap(),
            2
        );
        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            request_line(&requests[0]),
            "GET /v2/actor-runs/test-run HTTP/1.1"
        );
        assert_eq!(
            request_line(&requests[1]),
            "POST /v2/datasets/test-dataset/items HTTP/1.1"
        );
        assert!(requests
            .iter()
            .all(|request| has_test_bearer_token(request)));
        assert_eq!(
            serde_json::from_str::<Value>(request_body(&requests[1])).unwrap(),
            json!([{"id": 1}, {"id": 2}])
        );
    }

    #[tokio::test]
    async fn ppe_run_charges_only_rows_that_fit_the_remaining_budget() {
        let run = pay_per_event_run(0.0005, json!({"apify-actor-start": 1})).to_string();
        let server = MockServer::start(vec![
            response(200, &run),
            response(201, ""),
            response(201, ""),
        ]);
        let client = ApifyClient::new(server.base_url.clone(), "test-token".into()).unwrap();
        let items = vec![json!({"id": 1}), json!({"id": 2})];

        assert_eq!(
            client
                .push_data("test-run", "test-dataset", &items)
                .await
                .unwrap(),
            1
        );
        let requests = server.finish();
        assert_eq!(requests.len(), 3);
        assert_eq!(
            request_line(&requests[1]),
            "POST /v2/actor-runs/test-run/charge HTTP/1.1"
        );
        assert!(header(&requests[1], "idempotency-key").is_some());
        let charge = serde_json::from_str::<Value>(request_body(&requests[1])).unwrap();
        assert_eq!(charge, json!({"eventName": "property-result", "count": 1}));
        assert_eq!(
            request_line(&requests[2]),
            "POST /v2/datasets/test-dataset/items HTTP/1.1"
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_body(&requests[2])).unwrap(),
            json!([{"id": 1}])
        );
        assert!(requests
            .iter()
            .all(|request| has_test_bearer_token(request)));
    }

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: Url,
        requests: Receiver<String>,
        thread: JoinHandle<()>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                for response in responses {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_request(&mut stream);
                    sender.send(request).unwrap();
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        500 => "Internal Server Error",
                        _ => "Mock Response",
                    };
                    let reply = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    stream.write_all(reply.as_bytes()).unwrap();
                }
            });
            Self {
                base_url: Url::parse(&format!("http://{address}")).unwrap(),
                requests,
                thread,
            }
        }

        fn finish(self) -> Vec<String> {
            self.thread.join().unwrap();
            self.requests.into_iter().collect()
        }
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn request_line(request: &str) -> &str {
        request.lines().next().unwrap_or_default()
    }

    fn request_body(request: &str) -> &str {
        request.split_once("\r\n\r\n").map_or("", |(_, body)| body)
    }

    fn header<'a>(request: &'a str, wanted: &str) -> Option<&'a str> {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .skip(1)
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case(wanted).then_some(value.trim())
            })
    }

    fn has_test_bearer_token(request: &str) -> bool {
        header(request, "authorization") == Some("Bearer test-token")
    }
}
