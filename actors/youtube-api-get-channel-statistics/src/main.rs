use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, Url};
use serde_json::{json, Value};
use std::{collections::HashSet, env, time::Duration};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/youtube/channel";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
    scrappa_api_key: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").unwrap_or_default();
        if scrappa_api_key.is_empty() {
            bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
        }

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
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

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

fn build_channel_statistics_url(channel_id: &str, api_base_url: &Url) -> Url {
    let mut url = api_base_url.clone();
    url.query_pairs_mut().append_pair("channel_id", channel_id);
    url
}

fn collect_ids(value: &Value, ids: &mut Vec<String>, seen: &mut HashSet<String>) {
    match value {
        Value::String(value) => {
            for id in value.split(',').map(str::trim).filter(|id| !id.is_empty()) {
                if seen.insert(id.to_owned()) {
                    ids.push(id.to_owned());
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_ids(value, ids, seen);
            }
        }
        _ => {}
    }
}

fn get_channel_ids(input: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for field in ["ids", "id"] {
        if let Some(value) = input.get(field) {
            collect_ids(value, &mut ids, &mut seen);
        }
    }
    ids
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

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}s",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        anyhow!("Scrappa API request failed: {error}")
    }
}

async fn get_channel_statistics(client: &Client, url: &Url, api_key: &str) -> Result<Value> {
    let response = client
        .get(url.clone())
        .header("X-API-Key", api_key)
        .header("Accept", "application/json")
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(scrappa_request_error)?;

    if !response.status().is_success() {
        bail!(
            "Request failed with status code {}",
            response.status().as_u16()
        );
    }

    response.json().await.map_err(scrappa_request_error)
}

#[derive(Default)]
struct DatasetWriteBudget {
    capacity: Option<usize>,
    saved_items: usize,
}

async fn run_dataset_capacity(client: &Client, config: &ActorConfig) -> Result<usize> {
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
    affordable_dataset_items(&run, usize::MAX)
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
    if !item_price.is_finite() || item_price < 0.0 {
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

    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let affordable = ((max_charge - spent + tolerance) / item_price).floor();
    Ok(requested.min(affordable as usize))
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    rows: &[Value],
    budget: &mut DatasetWriteBudget,
) -> Result<usize> {
    if rows.is_empty() {
        return Ok(0);
    }

    let capacity = match budget.capacity {
        Some(capacity) => capacity,
        None => {
            let capacity = run_dataset_capacity(client, config).await?;
            budget.capacity = Some(capacity);
            capacity
        }
    };
    let saved_items = rows.len().min(capacity.saturating_sub(budget.saved_items));
    if saved_items == 0 {
        return Ok(0);
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(&rows[..saved_items])
        .send()
        .await
        .context("Apify dataset write failed")?;
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
    budget.saved_items += saved_items;
    Ok(saved_items)
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let ids = get_channel_ids(&input);
    if ids.is_empty() {
        bail!("At least one YouTube channel ID must be provided in \"ids\" or \"id\".");
    }

    let mut rows = Vec::with_capacity(ids.len());
    let mut success_count = 0;
    let mut failure_count = 0;
    for id in ids {
        let url = build_channel_statistics_url(&id, &config.scrappa_api_base_url);
        println!("Fetching from: {url}");
        match get_channel_statistics(client, &url, &config.scrappa_api_key).await {
            Ok(data) => {
                match data {
                    Value::Array(items) => rows.extend(items),
                    item => rows.push(item),
                }
                success_count += 1;
            }
            Err(error) => {
                let error = error.to_string();
                eprintln!("Failed to fetch YouTube channel statistics for id {id}: {error}");
                rows.push(json!({"id": id, "error": error, "success": false}));
                failure_count += 1;
            }
        }
    }

    let mut dataset_budget = DatasetWriteBudget::default();
    push_dataset_items(client, config, &rows, &mut dataset_budget).await?;
    if success_count == 0 {
        bail!("Failed to fetch statistics for all {failure_count} channel(s).");
    }

    println!(
        "Successfully fetched statistics for {success_count} channel(s); {failure_count} failed."
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube channel statistics: {error:#}");
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
        thread,
    };

    #[test]
    fn preserves_schema_prefill_values() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["ids"]["prefill"],
            json!("UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw")
        );
        assert_eq!(
            schema["properties"]["id"]["prefill"],
            json!("UCJZv4d5rbIKd4QHMPkcABCw")
        );
    }

    #[test]
    fn parses_recursive_batch_and_legacy_ids_in_first_seen_order() {
        let input = json!({
            "ids": [" UC1, UC2 ", ["UC3", 4], null],
            "id": "UC2, UC4"
        });
        assert_eq!(
            get_channel_ids(&input),
            vec![
                "UC1".to_owned(),
                "UC2".to_owned(),
                "UC3".to_owned(),
                "UC4".to_owned()
            ]
        );
    }

    #[test]
    fn builds_encoded_channel_query() {
        let base_url = Url::parse("https://scrappa.co/api/youtube/channel").unwrap();
        assert_eq!(
            build_channel_statistics_url("UC example&x", &base_url).as_str(),
            "https://scrappa.co/api/youtube/channel?channel_id=UC+example%26x"
        );
    }

    #[test]
    fn retains_sixty_second_scrappa_timeout() {
        assert_eq!(SCRAPPA_REQUEST_TIMEOUT.as_secs(), 60);
    }

    #[tokio::test]
    async fn preserves_auth_response_data_and_dataset_row_order() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"UC1, UC2"}"#),
            response(
                200,
                r#"[{"id":"UC1","statistics":{"views":12}},{"id":"UC1b","statistics":{"views":8}}]"#,
            ),
            response(500, ""),
            pricing_response(4.506432, 0, 1),
            response(201, ""),
        ]);
        let config = config(&server.base_url);

        run_actor(&client(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert!(has_header(
            &requests[0],
            "authorization",
            "Bearer test-token"
        ));
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert_eq!(
            request_parts(&requests[1]).1,
            "/api/youtube/channel?channel_id=UC1"
        );
        assert!(has_header(&requests[1], "x-api-key", "test-scrappa-key"));
        assert!(has_header(&requests[1], "accept", "application/json"));
        assert_eq!(
            request_parts(&requests[2]).1,
            "/api/youtube/channel?channel_id=UC2"
        );
        assert!(has_header(&requests[2], "x-api-key", "test-scrappa-key"));
        assert_eq!(request_parts(&requests[3]).1, "/v2/actor-runs/test-run");
        assert!(has_header(
            &requests[3],
            "authorization",
            "Bearer test-token"
        ));

        let (method, path, body) = request_parts(&requests[4]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert!(has_header(
            &requests[4],
            "authorization",
            "Bearer test-token"
        ));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([
                {"id":"UC1","statistics":{"views":12}},
                {"id":"UC1b","statistics":{"views":8}},
                {"id":"UC2","error":"Request failed with status code 500","success":false}
            ])
        );
    }
    #[tokio::test]
    async fn caps_two_rows_to_one_affordable_dataset_item() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"UC1"}"#),
            response(200, r#"[{"id":"first"},{"id":"second"}]"#),
            pricing_response(0.0005, 0, 1),
            response(201, ""),
        ]);
        let config = config(&server.base_url);

        run_actor(&client(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        let (_, path, body) = request_parts(&requests[3]);
        assert_eq!(path, "/v2/datasets/test-dataset/items");
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{"id":"first"}])
        );
    }

    #[tokio::test]
    async fn zero_capacity_skips_dataset_write() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"UC1"}"#),
            response(200, r#"[{"id":"first"},{"id":"second"}]"#),
            pricing_response(0.0, 0, 0),
        ]);
        let config = config(&server.base_url);

        run_actor(&client(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
    }

    #[tokio::test]
    async fn generous_normal_budget_posts_all_rows() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"UC1"}"#),
            response(200, r#"[{"id":"first"},{"id":"second"}]"#),
            pricing_response(4.506432, 0, 1),
            response(201, ""),
        ]);
        let config = config(&server.base_url);

        run_actor(&client(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        let (_, path, body) = request_parts(&requests[3]);
        assert_eq!(path, "/v2/datasets/test-dataset/items");
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{"id":"first"},{"id":"second"}])
        );
    }

    #[tokio::test]
    async fn reuses_initial_capacity_across_dataset_writes() {
        let server = MockServer::start(vec![pricing_response(0.0003, 0, 0), response(201, "")]);
        let config = config(&server.base_url);
        let mut budget = DatasetWriteBudget::default();
        let first = [json!({"id":"first"})];
        let second = [json!({"id":"second"})];

        assert_eq!(
            push_dataset_items(&client(), &config, &first, &mut budget)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            push_dataset_items(&client(), &config, &second, &mut budget)
                .await
                .unwrap(),
            0
        );

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_parts(&requests[0]).1, "/v2/actor-runs/test-run");
        assert_eq!(
            request_parts(&requests[1]).1,
            "/v2/datasets/test-dataset/items"
        );
        assert_eq!(budget.saved_items, 1);
    }

    #[tokio::test]
    async fn missing_spending_limit_fails_before_dataset_write() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"UC1"}"#),
            response(200, r#"[{"id":"row"}]"#),
            response(
                200,
                r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0003}}}},"options":{},"chargedEventCounts":{}}}"#,
            ),
        ]);
        let config = config(&server.base_url);

        let error = run_actor(&client(), &config).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("Apify run did not provide the spending limit"));
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
    }

    #[tokio::test]
    async fn invalid_pricing_fails_before_dataset_write() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"UC1"}"#),
            response(200, r#"[{"id":"row"}]"#),
            response(
                200,
                r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{}}},"options":{"maxTotalChargeUsd":4.506432},"chargedEventCounts":{}}}"#,
            ),
        ]);
        let config = config(&server.base_url);

        let error = run_actor(&client(), &config).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("Apify run did not provide the dataset item price"));
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
    }

    #[tokio::test]
    async fn dataset_storage_error_is_returned() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"UC1"}"#),
            response(200, r#"[{"id":"row"}]"#),
            pricing_response(4.506432, 0, 0),
            response(500, "dataset unavailable"),
        ]);
        let config = config(&server.base_url);

        let error = run_actor(&client(), &config).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("Apify dataset write failed with 500"));
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
    }

    #[tokio::test]
    async fn writes_failure_rows_before_failing_when_every_request_fails() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"UC1,UC2"}"#),
            response(500, ""),
            response(500, ""),
            pricing_response(1.0, 0, 0),
            response(201, ""),
        ]);
        let config = config(&server.base_url);

        let error = run_actor(&client(), &config).await.unwrap_err();
        assert!(error
            .to_string()
            .contains("Failed to fetch statistics for all 2 channel(s)."));
        let requests = server.requests();
        let (_, _, body) = request_parts(&requests[4]);
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([
                {"id":"UC1","error":"Request failed with status code 500","success":false},
                {"id":"UC2","error":"Request failed with status code 500","success":false}
            ])
        );
    }

    fn client() -> Client {
        Client::builder().build().unwrap()
    }

    fn config(base_url: &Url) -> ActorConfig {
        let mut scrappa_api_base_url = base_url.clone();
        scrappa_api_base_url.set_path("/api/youtube/channel");
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url,
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: Url,
        requests: mpsc::Receiver<String>,
        stop: Arc<AtomicBool>,
        thread: Option<thread::JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (request_sender, requests) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let thread_stop = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                for response in responses {
                    let (mut stream, _) = loop {
                        if thread_stop.load(Ordering::Relaxed) {
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

    fn pricing_response(
        max_charge: f64,
        charged_items: u64,
        charged_actor_starts: u64,
    ) -> MockResponse {
        response(
            200,
            &json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            "apify-actor-start": {"eventPriceUsd": 0.0002}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_charge},
                    "chargedEventCounts": {
                        "apify-default-dataset-item": charged_items,
                        "apify-actor-start": charged_actor_starts
                    }
                }
            })
            .to_string(),
        )
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

    fn has_header(request: &str, name: &str, value: &str) -> bool {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .skip(1)
            .any(|line| {
                let Some((header_name, header_value)) = line.split_once(':') else {
                    return false;
                };
                header_name.eq_ignore_ascii_case(name) && header_value.trim() == value
            })
    }
}
