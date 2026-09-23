use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::Value;
use std::time::Duration;

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub fn new(base_url: &str, token: String) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token,
        })
    }

    fn record_url(&self, store_id: &str, record_key: &str) -> Result<Url> {
        self.resource_url(&["key-value-stores", store_id, "records", record_key])
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
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    pub async fn get_input(&self, store_id: &str, record_key: &str) -> Result<Option<Value>> {
        let response = self
            .request(Method::GET, self.record_url(store_id, record_key)?)
            .send()
            .await
            .context("Failed to fetch Actor input from the default key-value store")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "fetch Actor input").await?;
        Ok(Some(
            response
                .json()
                .await
                .context("Actor input record is not valid JSON")?,
        ))
    }

    pub async fn set_output(&self, store_id: &str, output: &Value) -> Result<()> {
        let response = self
            .request(Method::PUT, self.record_url(store_id, "OUTPUT")?)
            .json(output)
            .send()
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn dataset_item_budget(&self, actor_run_id: &str) -> Result<usize> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&["actor-runs", actor_run_id])?,
            )
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = successful_response(response, "fetch Actor run pricing")
            .await?
            .json()
            .await
            .context("Actor run pricing response is not valid JSON")?;
        affordable_dataset_items(&run)
    }

    pub async fn push_data(
        &self,
        dataset_id: &str,
        items: &[Value],
        remaining_budget: &mut usize,
    ) -> Result<()> {
        let items = &items[..items.len().min(*remaining_budget)];
        if items.is_empty() {
            return Ok(());
        }
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["datasets", dataset_id, "items"])?,
            )
            .json(items)
            .send()
            .await
            .context("Failed to store review items in the default dataset")?;
        successful_response(response, "store dataset items").await?;
        *remaining_budget -= items.len();
        Ok(())
    }
}

fn affordable_dataset_items(run: &Value) -> Result<usize> {
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
    let item_price = events
        .get("apify-default-dataset-item")
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
    if !item_price.is_finite() || item_price < 0.0 || !max_charge.is_finite() || max_charge < 0.0 {
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
        if count == 0 {
            continue;
        }
        let price = events
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
    if item_price == 0.0 {
        return Ok(usize::MAX);
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let available_charge = (max_charge - spent + tolerance).max(0.0);
    let affordable_items = (available_charge / item_price).floor();
    Ok(if affordable_items.is_finite() {
        affordable_items as usize
    } else {
        usize::MAX
    })
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
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver},
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: String,
        requests: Receiver<String>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (request_sender, requests) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                for response in responses {
                    let (mut stream, _) = loop {
                        if stopped.load(Ordering::Relaxed) || Instant::now() >= deadline {
                            return;
                        }
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(_) => return,
                        }
                    };
                    let request = read_request(&mut stream).unwrap_or_default();
                    if request_sender.send(request).is_err() {
                        return;
                    }
                    let reason = match response.status {
                        200 => "OK",
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
                    if stream.write_all(reply.as_bytes()).is_err() {
                        return;
                    }
                }
            });
            Self {
                base_url: format!("http://{address}"),
                requests,
                stop,
                thread: Some(thread),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.try_iter().collect()
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        let mut content_length = 0;
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                if content_length == 0 {
                    let headers = String::from_utf8_lossy(&bytes[..header_end]);
                    content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                }
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn response(status: u16, body: Value) -> MockResponse {
        MockResponse {
            status,
            body: body.to_string(),
        }
    }

    fn pricing_response(max_charge: f64, charged_counts: Value) -> MockResponse {
        response(
            200,
            json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                            "other-event": {"eventPriceUsd": 0.05}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_charge},
                    "chargedEventCounts": charged_counts
                }
            }),
        )
    }

    fn client(server: &MockServer) -> ApifyClient {
        ApifyClient::new(&server.base_url, "test-token".to_owned()).unwrap()
    }

    fn rows(ids: &[&str]) -> Vec<Value> {
        ids.iter().map(|id| json!({"id": id})).collect()
    }

    fn request_parts(request: &str) -> (&str, &str, &str) {
        let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
        let mut parts = headers
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace();
        (
            parts.next().unwrap_or_default(),
            parts.next().unwrap_or_default(),
            body,
        )
    }

    fn has_test_bearer_token(request: &str) -> bool {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .any(|line| {
                let Some((name, value)) = line.split_once(':') else {
                    return false;
                };
                name.eq_ignore_ascii_case("authorization") && value.trim() == "Bearer test-token"
            })
    }

    #[tokio::test]
    async fn one_result_budget_posts_only_the_first_of_two_reviews() {
        let server = MockServer::start(vec![
            pricing_response(0.1, json!({})),
            response(200, json!({})),
        ]);
        let apify = client(&server);
        let rows = rows(&["first", "second"]);
        let mut budget = apify.dataset_item_budget("test-run").await.unwrap();
        assert_eq!(budget, 1);

        apify
            .push_data("test-dataset", &rows, &mut budget)
            .await
            .unwrap();

        assert_eq!(budget, 0);
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_parts(&requests[0]).1, "/v2/actor-runs/test-run");
        assert!(has_test_bearer_token(&requests[0]));
        let (method, path, body) = request_parts(&requests[1]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert!(has_test_bearer_token(&requests[1]));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{"id":"first"}])
        );
    }

    #[tokio::test]
    async fn zero_budget_skips_dataset_post() {
        let server = MockServer::start(vec![pricing_response(0.0, json!({}))]);
        let apify = client(&server);
        let mut budget = apify.dataset_item_budget("test-run").await.unwrap();
        assert_eq!(budget, 0);

        apify
            .push_data("test-dataset", &rows(&["first", "second"]), &mut budget)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(request_parts(&requests[0]).1, "/v2/actor-runs/test-run");
    }

    #[tokio::test]
    async fn generous_numeric_budget_posts_all_reviews() {
        let server = MockServer::start(vec![
            pricing_response(1.0, json!({})),
            response(200, json!({})),
        ]);
        let apify = client(&server);
        let rows = rows(&["first", "second"]);
        let mut budget = apify.dataset_item_budget("test-run").await.unwrap();

        apify
            .push_data("test-dataset", &rows, &mut budget)
            .await
            .unwrap();

        assert_eq!(budget, 8);
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[1]).2).unwrap(),
            json!([{"id":"first"}, {"id":"second"}])
        );
    }

    #[tokio::test]
    async fn multiple_page_posts_share_the_initial_budget_for_all_events() {
        let server = MockServer::start(vec![
            pricing_response(
                0.5,
                json!({"apify-default-dataset-item": 1, "other-event": 2}),
            ),
            response(200, json!({})),
            response(200, json!({})),
        ]);
        let apify = client(&server);
        let mut budget = apify.dataset_item_budget("test-run").await.unwrap();
        assert_eq!(budget, 3);

        apify
            .push_data("test-dataset", &rows(&["first", "second"]), &mut budget)
            .await
            .unwrap();
        apify
            .push_data("test-dataset", &rows(&["third", "fourth"]), &mut budget)
            .await
            .unwrap();

        assert_eq!(budget, 0);
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_parts(&requests[0]).1, "/v2/actor-runs/test-run");
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[1]).2).unwrap(),
            json!([{"id":"first"}, {"id":"second"}])
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[2]).2).unwrap(),
            json!([{"id":"third"}])
        );
    }

    #[tokio::test]
    async fn missing_spending_limit_fails_before_any_dataset_post() {
        let server = MockServer::start(vec![response(
            200,
            json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.1}
                        }}
                    },
                    "chargedEventCounts": {}
                }
            }),
        )]);
        let apify = client(&server);

        assert!(apify.dataset_item_budget("test-run").await.is_err());
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn missing_charged_event_counts_cannot_authorize_reviews() {
        let mut run: Value = serde_json::from_str(&pricing_response(1.0, json!({})).body).unwrap();
        run["data"]
            .as_object_mut()
            .unwrap()
            .remove("chargedEventCounts");
        assert!(affordable_dataset_items(&run).is_err());
    }

    #[tokio::test]
    async fn pricing_http_error_fails_before_any_dataset_post() {
        let server =
            MockServer::start(vec![response(500, json!({"error": "pricing unavailable"}))]);
        let apify = client(&server);

        assert!(apify.dataset_item_budget("test-run").await.is_err());
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn storage_error_is_returned_without_spending_local_budget() {
        let server = MockServer::start(vec![
            pricing_response(0.2, json!({})),
            response(500, json!({"error": "storage unavailable"})),
        ]);
        let apify = client(&server);
        let mut budget = apify.dataset_item_budget("test-run").await.unwrap();

        assert!(
            apify
                .push_data("test-dataset", &rows(&["first", "second"]), &mut budget)
                .await
                .is_err()
        );

        assert_eq!(budget, 2);
        assert_eq!(server.requests().len(), 2);
    }
}
