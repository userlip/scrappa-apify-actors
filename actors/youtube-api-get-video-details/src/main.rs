use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, Url};
use serde_json::Value;
use std::{env, time::Duration};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://ytapi.scrappa.co";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
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

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

fn build_video_details_url(input: &Value, api_base_url: &Url) -> Result<Url> {
    let id = input
        .as_object()
        .and_then(|object| object.get("id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| anyhow!("YouTube video ID \"id\" is required."))?;

    let mut url = endpoint_url(api_base_url, &["videos"])?;
    url.query_pairs_mut().append_pair("id", id);
    Ok(url)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
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

    response
        .json()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

async fn get_input(client: &Client, config: &ActorConfig) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify INPUT request failed")?;
    response_json(response, "Apify INPUT request").await
}

async fn run_dataset_capacity(
    client: &Client,
    config: &ActorConfig,
    requested: usize,
) -> Result<usize> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify run pricing request failed")?;
    let run = response_json(response, "Apify run pricing request").await?;
    affordable_dataset_items(&run, requested)
}

fn affordable_dataset_items(run: &Value, requested: usize) -> Result<usize> {
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
        return Ok(requested);
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .count())
}

async fn fetch_video_details(client: &Client, url: &Url) -> Result<Value> {
    let response = client
        .get(url.clone())
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "Scrappa API request timed out after {}s",
                    SCRAPPA_REQUEST_TIMEOUT.as_secs()
                )
            } else {
                anyhow!("Scrappa API request failed: {error}")
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        bail!(
            "Scrappa API request failed with {} {reason}",
            status.as_u16()
        );
    }

    response
        .json()
        .await
        .context("Scrappa API response was not valid JSON")
}

async fn push_dataset_data(client: &Client, config: &ActorConfig, data: &Value) -> Result<usize> {
    if !data.is_array() && !data.is_object() {
        bail!("Scrappa API response must be an object or array");
    }
    let requested = data.as_array().map_or(1, Vec::len);
    if requested == 0 {
        return Ok(0);
    }

    let limit = run_dataset_capacity(client, config, requested).await?;
    if limit == 0 {
        return Ok(0);
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let request = client.post(url).bearer_auth(&config.apify_token);
    let request = match data.as_array() {
        Some(items) => request.json(&items[..limit]),
        None => request.json(data),
    };
    let response = request.send().await.context("Apify dataset write failed")?;
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = response.text().await.unwrap_or_default();
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "Apify dataset write failed with {} {reason}{detail}",
            status.as_u16()
        );
    }

    Ok(limit)
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    js_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let url = build_video_details_url(&input, &config.scrappa_api_base_url)?;
    println!("Fetching from: {url}");

    let data = fetch_video_details(client, &url).await?;
    let saved_count = push_dataset_data(client, config, &data).await?;

    let result_count = data.as_array().map_or(1, Vec::len);
    let id = input
        .get("id")
        .map(js_string)
        .unwrap_or_else(|| "undefined".to_owned());
    println!(
        "Successfully fetched {result_count} video detail result(s) for id: {id}; saved {saved_count} dataset item(s)"
    );

    if let Some(continuation) = data.get("continuation").filter(|value| js_truthy(value)) {
        println!(
            "Continuation token available for next page: {}",
            js_string(continuation)
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube video details: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::builder()
        .build()
        .context("Could not create HTTP client")?;
    run_actor(&client, &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver},
            Arc,
        },
        thread::{self, JoinHandle},
        time::Instant,
    };

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: Url,
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
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
                        504 => "Gateway Timeout",
                        _ => "Mock Response",
                    };
                    let message = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    if stream.write_all(message.as_bytes()).is_err() {
                        return;
                    }
                }
            });

            Self {
                base_url: Url::parse(&format!("http://{address}")).unwrap(),
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
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
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
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    fn pricing_response(max_charge: f64, charged_event_counts: Value) -> MockResponse {
        response(
            200,
            &serde_json::json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            "apify-actor-start": {"eventPriceUsd": 0.0001}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_charge},
                    "chargedEventCounts": charged_event_counts
                }
            })
            .to_string(),
        )
    }

    fn config(base_url: &Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: base_url.clone(),
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
        }
    }

    fn client() -> Client {
        Client::builder().build().unwrap()
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
                name.eq_ignore_ascii_case("authorization")
                    && value.trim() == "Bearer test-token-not-a-real-credential"
            })
    }

    async fn assert_dataset_preserves_response(data: Value) {
        let input = serde_json::json!({"id":" video-id "});
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(200, &data.to_string()),
            pricing_response(1.0, serde_json::json!({})),
            response(201, ""),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(has_test_bearer_token(&requests[0]));
        assert!(!has_test_bearer_token(&requests[1]));
        assert!(has_test_bearer_token(&requests[2]));
        assert!(has_test_bearer_token(&requests[3]));
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert_eq!(request_parts(&requests[1]).1, "/videos?id=video-id");
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
        let (method, path, body) = request_parts(&requests[3]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert_eq!(serde_json::from_str::<Value>(body).unwrap(), data);
    }

    #[test]
    fn builds_url_from_trimmed_video_id_and_rejects_missing_id() {
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let url =
            build_video_details_url(&serde_json::json!({"id":" dQw4w9WgXcQ "}), &base).unwrap();
        assert_eq!(url.path(), "/videos");
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "id").unwrap().1,
            "dQw4w9WgXcQ"
        );
        for input in [
            serde_json::json!({}),
            serde_json::json!({"id":"   "}),
            serde_json::json!({"id":7}),
        ] {
            assert!(build_video_details_url(&input, &base)
                .unwrap_err()
                .to_string()
                .contains("id"));
        }
    }

    #[test]
    fn qa_prefill_is_accepted_by_the_rust_url_builder() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let prefill = schema["properties"]["id"]["prefill"].as_str().unwrap();
        let url = build_video_details_url(
            &serde_json::json!({"id":prefill}),
            &Url::parse(SCRAPPA_API_BASE_URL).unwrap(),
        )
        .unwrap();
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "id").unwrap().1,
            prefill
        );
    }

    #[tokio::test]
    async fn local_storage_mock_preserves_object_and_array_dataset_rows() {
        assert_dataset_preserves_response(serde_json::json!({
            "videoId":"video-id",
            "title":"Video",
            "continuation":"next-page"
        }))
        .await;
        assert_dataset_preserves_response(serde_json::json!([
            {"videoId":"video-one"},
            {"videoId":"video-two"}
        ]))
        .await;
    }

    #[tokio::test]
    async fn one_result_budget_posts_only_the_first_row() {
        let rows = serde_json::json!([
            {"videoId":"video-one"},
            {"videoId":"video-two"}
        ]);
        let server = MockServer::start(vec![
            response(200, r#"{"id":"video-one,video-two"}"#),
            response(200, &rows.to_string()),
            pricing_response(0.0003, serde_json::json!({})),
            response(201, ""),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
        assert!(has_test_bearer_token(&requests[2]));
        let (method, path, body) = request_parts(&requests[3]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::json!([{"videoId":"video-one"}])
        );
    }

    #[tokio::test]
    async fn zero_budget_skips_the_dataset_post() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"video-one"}"#),
            response(200, r#"[{"videoId":"video-one"}]"#),
            pricing_response(0.0, serde_json::json!({})),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));
    }

    #[test]
    fn all_existing_charged_events_reduce_affordable_dataset_rows() {
        let run = serde_json::json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                "apify-actor-start": {"eventPriceUsd": 0.0001}
            }}},
            "options": {"maxTotalChargeUsd": 0.0005},
            "chargedEventCounts": {"apify-actor-start": 1}
        }});
        assert_eq!(affordable_dataset_items(&run, 2).unwrap(), 1);
        assert!(affordable_dataset_items(
            &serde_json::json!({"data": {
                "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
                }}},
                "options": {"maxTotalChargeUsd": 1.0}
            }}),
            2
        )
        .is_err());
        let mut missing_limit = serde_json::json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
            }}},
            "options": {"maxTotalChargeUsd": null},
            "chargedEventCounts": {}
        }});
        assert!(affordable_dataset_items(&missing_limit, 2).is_err());
        missing_limit["data"]["options"] = serde_json::json!({});
        assert!(affordable_dataset_items(&missing_limit, 2).is_err());
    }

    #[tokio::test]
    async fn pricing_and_storage_errors_propagate_without_followup_writes() {
        let pricing_error = MockServer::start(vec![
            response(200, r#"{"id":"video-one"}"#),
            response(200, r#"[{"videoId":"video-one"}]"#),
            response(500, "pricing unavailable"),
        ]);
        let error = run_actor(&client(), &config(&pricing_error.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("500 Internal Server Error"));
        let requests = pricing_error.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));

        let storage_error = MockServer::start(vec![
            response(200, r#"{"id":"video-one"}"#),
            response(200, r#"[{"videoId":"video-one"}]"#),
            pricing_response(1.0, serde_json::json!({})),
            response(500, "storage unavailable"),
        ]);
        let error = run_actor(&client(), &config(&storage_error.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("500 Internal Server Error"));
        let requests = storage_error.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests[3].starts_with("POST /v2/datasets/test-dataset/items"));
    }

    #[tokio::test]
    async fn upstream_and_apify_errors_fail_without_fabricating_rows() {
        let input = r#"{"id":"video-id"}"#;
        let upstream_error = MockServer::start(vec![response(200, input), response(400, "{}")]);
        let error = run_actor(&client(), &config(&upstream_error.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("400 Bad Request"));
        let requests = upstream_error.requests();
        assert_eq!(requests.len(), 2);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));

        let apify_error = MockServer::start(vec![response(401, "unauthorized")]);
        let error = run_actor(&client(), &config(&apify_error.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("401 Unauthorized"));
        assert_eq!(apify_error.requests().len(), 1);
    }

    #[test]
    fn continuation_uses_javascript_truthiness_and_string_coercion() {
        assert!(!js_truthy(&Value::Null));
        assert!(!js_truthy(&serde_json::json!(0)));
        assert!(!js_truthy(&serde_json::json!("")));
        assert!(js_truthy(&serde_json::json!([])));
        assert!(js_truthy(&serde_json::json!({})));
        assert_eq!(js_string(&serde_json::json!(["next", null, 2])), "next,,2");
        assert_eq!(js_string(&serde_json::json!({})), "[object Object]");
    }
}
