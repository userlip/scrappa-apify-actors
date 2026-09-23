use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://ytapi.scrappa.co/playlists";
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

fn string_value(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[derive(Debug)]
struct PlaylistDetailsRequest {
    id: String,
    url: Url,
}

fn build_playlist_details_request(
    input: &Value,
    api_base_url: &Url,
) -> Result<PlaylistDetailsRequest> {
    let id = string_value(input.get("id"))
        .ok_or_else(|| anyhow!("YouTube playlist ID \"id\" is required."))?;

    let mut url = api_base_url.clone();
    url.set_query(None);
    url.query_pairs_mut().append_pair("id", &id);
    Ok(PlaylistDetailsRequest { id, url })
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

async fn fetch_playlist_details(client: &Client, url: &Url) -> Result<Value> {
    let response = client
        .get(url.clone())
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() || error.to_string().contains("aborted") {
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

fn dataset_item_count(data: &Value) -> usize {
    data.as_array().map_or(1, Vec::len)
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
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
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

async fn push_dataset_data(client: &Client, config: &ActorConfig, data: &Value) -> Result<usize> {
    let requested = dataset_item_count(data);
    if requested == 0 {
        return Ok(0);
    }
    let capacity = run_dataset_capacity(client, config, requested).await?;
    let saved_rows = requested.min(capacity);
    if saved_rows == 0 {
        return Ok(0);
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let request = client.post(url).bearer_auth(&config.apify_token);
    let request = if let Some(items) = data.as_array() {
        request.json(&items[..saved_rows])
    } else {
        request.json(&[data])
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
    Ok(saved_rows)
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_none_or(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let request = build_playlist_details_request(&input, &config.scrappa_api_base_url)?;
    println!("Fetching from: {}", request.url);

    let data = fetch_playlist_details(client, &request.url).await?;
    let saved_rows = push_dataset_data(client, config, &data).await?;
    println!(
        "Successfully fetched {} playlist detail result(s) for id: {} (saved {saved_rows})",
        dataset_item_count(&data),
        request.id
    );

    if let Some(continuation) = data.get("continuation").filter(|value| is_truthy(value)) {
        println!("Continuation token available for next page: {continuation}");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube playlist details: {error:#}");
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
            mpsc, Arc,
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
        requests: mpsc::Receiver<String>,
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
                    let _ = request_sender.send(request);
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
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

    fn pricing_response(max_charge: f64, charged_dataset_items: u64) -> MockResponse {
        response(
            200,
            &serde_json::json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            "apify-actor-start": {"eventPriceUsd": 0.00005}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_charge},
                    "chargedEventCounts": {
                        "apify-default-dataset-item": charged_dataset_items,
                        "apify-actor-start": 1
                    }
                }
            })
            .to_string(),
        )
    }

    fn run_input() -> MockResponse {
        response(200, r#"{"id":"playlist-id"}"#)
    }

    fn playlist_rows() -> MockResponse {
        response(200, r#"[{"id":"first"},{"id":"second"}]"#)
    }

    fn config(base_url: &Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: base_url.join("playlists").unwrap(),
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

    #[test]
    fn schema_keeps_the_playlist_id_prefill_and_required_field() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["id"]["prefill"],
            "PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf"
        );
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("id")));
    }

    #[test]
    fn builds_the_original_endpoint_and_encodes_the_playlist_id() {
        let base_url = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let request = build_playlist_details_request(
            &serde_json::json!({"id": " playlist id/with spaces "}),
            &base_url,
        )
        .unwrap();

        assert_eq!(request.id, "playlist id/with spaces");
        assert_eq!(request.url.host_str(), Some("ytapi.scrappa.co"));
        assert_eq!(request.url.path(), "/playlists");
        let id = request
            .url
            .query_pairs()
            .find(|(key, _)| key == "id")
            .unwrap()
            .1
            .into_owned();
        assert_eq!(id, "playlist id/with spaces");
        assert!(request
            .url
            .as_str()
            .contains("id=playlist+id%2Fwith+spaces"));
    }

    #[test]
    fn rejects_missing_non_string_and_blank_playlist_ids() {
        for input in [
            serde_json::json!({}),
            serde_json::json!({"id": "   "}),
            serde_json::json!({"id": 123}),
            Value::Null,
        ] {
            assert!(build_playlist_details_request(
                &input,
                &Url::parse(SCRAPPA_API_BASE_URL).unwrap()
            )
            .unwrap_err()
            .to_string()
            .contains("YouTube playlist ID \"id\" is required."));
        }
    }

    #[test]
    fn preserves_single_and_batch_dataset_row_counts() {
        assert_eq!(dataset_item_count(&serde_json::json!({"id": "one"})), 1);
        assert_eq!(
            dataset_item_count(&serde_json::json!([{"id": "one"}, {"id": "two"}])),
            2
        );
        assert_eq!(dataset_item_count(&serde_json::json!([])), 0);
    }

    #[tokio::test]
    async fn writes_a_single_playlist_object_as_one_dataset_row() {
        let server = MockServer::start(vec![
            pricing_response(1.0, 0),
            response(201, r#"{"ok":true}"#),
        ]);
        let config = config(&server.base_url);
        let data = serde_json::json!({"id": "one"});

        assert_eq!(
            push_dataset_data(&client(), &config, &data).await.unwrap(),
            1
        );

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_parts(&requests[0]).1, "/v2/actor-runs/test-run");
        let (_, _, body) = request_parts(&requests[1]);
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::json!([{"id":"one"}])
        );
    }

    #[tokio::test]
    async fn does_not_write_an_empty_result_array() {
        let server = MockServer::start(Vec::new());
        let config = config(&server.base_url);

        assert_eq!(
            push_dataset_data(&client(), &config, &serde_json::json!([]))
                .await
                .unwrap(),
            0
        );
        assert!(server.requests().is_empty());
    }

    #[tokio::test]
    async fn fetches_and_writes_each_playlist_row_in_order_through_apify_api() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"playlist id/with spaces"}"#),
            response(200, r#"[{"id":"first"},{"id":"second"}]"#),
            pricing_response(1.0, 0),
            response(201, r#"{"ok":true}"#),
        ]);
        let config = config(&server.base_url);

        run_actor(&client(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        let (method, path, _) = request_parts(&requests[0]);
        assert_eq!(
            (method, path),
            ("GET", "/v2/key-value-stores/test-store/records/INPUT")
        );
        assert!(has_test_bearer_token(&requests[0]));
        let (method, path, _) = request_parts(&requests[1]);
        assert_eq!(
            (method, path),
            ("GET", "/playlists?id=playlist+id%2Fwith+spaces")
        );
        assert!(!has_test_bearer_token(&requests[1]));
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
        assert!(has_test_bearer_token(&requests[2]));
        let (method, path, body) = request_parts(&requests[3]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert!(has_test_bearer_token(&requests[3]));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::json!([{"id":"first"},{"id":"second"}])
        );
    }

    #[tokio::test]
    async fn reports_scrappa_http_errors_without_writing_dataset_rows() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"playlist-id"}"#),
            response(429, r#"{"error":"rate limited"}"#),
        ]);
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert!(
            format!("{error:#}").contains("Scrappa API request failed with 429 Too Many Requests")
        );
        assert_eq!(server.requests().len(), 2);
    }

    #[tokio::test]
    async fn spending_budget_trims_two_playlist_rows_to_one() {
        let server = MockServer::start(vec![
            run_input(),
            playlist_rows(),
            pricing_response(0.00065, 1),
            response(201, "{}"),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        let (_, _, body) = request_parts(&requests[3]);
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::json!([{"id":"first"}])
        );
    }

    #[tokio::test]
    async fn zero_spending_budget_skips_the_dataset_post() {
        let server = MockServer::start(vec![
            run_input(),
            playlist_rows(),
            pricing_response(0.00005, 0),
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

    #[tokio::test]
    async fn sufficient_spending_budget_writes_all_playlist_rows() {
        let server = MockServer::start(vec![
            run_input(),
            playlist_rows(),
            pricing_response(4.506432, 0),
            response(201, "{}"),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        let (_, _, body) = request_parts(&requests[3]);
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::json!([{"id":"first"},{"id":"second"}])
        );
    }

    #[tokio::test]
    async fn pricing_lookup_failure_does_not_post_dataset_rows() {
        let server = MockServer::start(vec![run_input(), playlist_rows(), response(500, "{}")]);
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("Apify run pricing request failed"));
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));
    }

    #[tokio::test]
    async fn missing_pricing_fails_closed_without_posting_dataset_rows() {
        let server = MockServer::start(vec![run_input(), playlist_rows(), response(200, "{}")]);
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("Apify run pricing is missing"));
        assert!(server
            .requests()
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));
    }

    #[tokio::test]
    async fn missing_numeric_run_limit_fails_closed_without_posting_rows() {
        let metadata = serde_json::json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                "apify-actor-start": {"eventPriceUsd": 0.00005}
            }}},
            "options": {},
            "chargedEventCounts": {"apify-actor-start": 1}
        }}).to_string();
        let server =
            MockServer::start(vec![run_input(), playlist_rows(), response(200, &metadata)]);
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("Apify run did not provide the spending limit"));
        assert!(server
            .requests()
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));
    }

    #[tokio::test]
    async fn dataset_storage_failure_is_reported() {
        let server = MockServer::start(vec![
            run_input(),
            playlist_rows(),
            pricing_response(1.0, 0),
            response(500, "{}"),
        ]);
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("Apify dataset write failed with 500"));
    }
}
