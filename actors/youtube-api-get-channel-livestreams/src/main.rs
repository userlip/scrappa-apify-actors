use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{collections::HashSet, env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/youtube/channel-videos";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_TARGET_RESULT_COUNT: usize = 10;
const MAX_FILTER_SCAN_PAGES: usize = 10;

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
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key: env::var("SCRAPPA_API_KEY").ok().filter(|key| !key.is_empty()).ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?,
        })
    }
}

struct DatasetBudget {
    item_price_usd: f64,
    charged_usd: f64,
    max_total_charge_usd: f64,
    locally_saved_rows: usize,
}

impl DatasetBudget {
    fn from_run(run: &Value) -> Result<Self> {
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
        let item_price_usd = events
            .get("apify-default-dataset-item")
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
        if !item_price_usd.is_finite()
            || item_price_usd < 0.0
            || !max_total_charge_usd.is_finite()
            || max_total_charge_usd < 0.0
        {
            bail!("Apify run returned invalid charging values");
        }

        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_usd = 0.0;
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
            charged_usd += price * count as f64;
        }
        if !charged_usd.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        Ok(Self {
            item_price_usd,
            charged_usd,
            max_total_charge_usd,
            locally_saved_rows: 0,
        })
    }

    fn affordable_items(&self, requested: usize) -> Result<usize> {
        if self.item_price_usd == 0.0 {
            return Ok(requested);
        }
        let spent = self.charged_usd + self.item_price_usd * self.locally_saved_rows as f64;
        if !spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }
        let tolerance = f64::EPSILON * self.max_total_charge_usd.max(1.0);
        Ok((1..=requested)
            .take_while(|count| {
                spent + *count as f64 * self.item_price_usd <= self.max_total_charge_usd + tolerance
            })
            .count())
    }
}

async fn run_dataset_budget(client: &Client, config: &ActorConfig) -> Result<DatasetBudget> {
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
    DatasetBudget::from_run(&response_json(response, "Apify run pricing request").await?)
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

fn parse_ids(value: &Value, ids: &mut Vec<String>) {
    match value {
        Value::String(value) => ids.extend(
            value
                .split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_owned),
        ),
        Value::Array(values) => values.iter().for_each(|value| parse_ids(value, ids)),
        _ => {}
    }
}

fn get_channel_ids(input: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(value) = input.get("ids") {
        parse_ids(value, &mut ids);
    }
    if let Some(value) = input.get("id") {
        parse_ids(value, &mut ids);
    }
    let mut seen = HashSet::with_capacity(ids.len());
    ids.into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

fn assert_continuation_matches_batch(input: &Value, ids: &[String]) -> Result<()> {
    if ids.len() > 1
        && input
            .get("continuation")
            .and_then(Value::as_str)
            .is_some_and(|continuation| !continuation.trim().is_empty())
    {
        bail!("The \"continuation\" token can only be used with a single YouTube channel ID.");
    }
    Ok(())
}

fn build_channel_livestreams_url(
    api_base_url: &Url,
    id: &str,
    sort: Option<&str>,
    continuation: &Value,
) -> Result<Url> {
    if id.is_empty() {
        bail!("Search query \"id\" not provided. Please provide a value for \"id\" in the input.");
    }
    let mut url = api_base_url.clone();
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("channel_id", id);
        if let Some(sort) = sort.filter(|sort| !sort.trim().is_empty()) {
            query.append_pair("sort", sort);
        }
        if let Some(continuation) = continuation
            .as_str()
            .filter(|continuation| !continuation.trim().is_empty())
        {
            query.append_pair("continuation", continuation);
        }
    }
    Ok(url)
}

fn is_livestream_video(video: &Value) -> bool {
    if video.get("isLive") == Some(&Value::Bool(true)) {
        return true;
    }
    let kind = video
        .get("type")
        .filter(|kind| !kind.is_null())
        .or_else(|| video.get("videoType"))
        .map(js_string)
        .unwrap_or_default();
    matches!(kind.to_lowercase().as_str(), "live" | "livestream")
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

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_none_or(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn response_continuation(response: &Value) -> Value {
    response
        .get("continuation")
        .filter(|value| !value.is_null())
        .or_else(|| {
            response
                .get("continuationToken")
                .filter(|value| !value.is_null())
        })
        .or_else(|| {
            response
                .get("pagination")
                .and_then(|pagination| pagination.get("continuation"))
                .filter(|value| !value.is_null())
        })
        .or_else(|| {
            response
                .get("pagination")
                .and_then(|pagination| pagination.get("continuationToken"))
                .filter(|value| !value.is_null())
        })
        .cloned()
        .unwrap_or_else(|| Value::String(String::new()))
}

async fn fetch_scrappa_page(client: &Client, config: &ActorConfig, url: Url) -> Result<Value> {
    let response = client
        .get(url)
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .header("X-API-Key", &config.scrappa_api_key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(scrappa_request_error)?;
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        bail!(
            "Scrappa API request failed with {} {reason}",
            status.as_u16()
        );
    }
    response.json().await.map_err(scrappa_request_error)
}

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}s",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        anyhow!(error.to_string())
    }
}

async fn collect_filtered_channel_videos(
    client: &Client,
    config: &ActorConfig,
    input: &Value,
    id: &str,
) -> Result<(Vec<Value>, Value)> {
    let sort = input.get("sort").and_then(Value::as_str);
    let mut videos = Vec::new();
    let mut continuation = input
        .get("continuation")
        .cloned()
        .unwrap_or_else(|| Value::String(String::new()));
    let mut next_continuation = Value::String(String::new());

    for _ in 0..MAX_FILTER_SCAN_PAGES {
        let url =
            build_channel_livestreams_url(&config.scrappa_api_base_url, id, sort, &continuation)?;
        println!("Fetching from: {url}");
        let response = fetch_scrappa_page(client, config, url).await?;
        if let Some(page_videos) = response.get("videos").and_then(Value::as_array) {
            videos.extend(
                page_videos
                    .iter()
                    .filter(|video| is_livestream_video(video))
                    .cloned(),
            );
        }
        next_continuation = response_continuation(&response);
        if !js_truthy(&next_continuation)
            || next_continuation == continuation
            || videos.len() >= DEFAULT_TARGET_RESULT_COUNT
        {
            break;
        }
        continuation = next_continuation.clone();
    }
    Ok((videos, next_continuation))
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    dataset_budget: &mut Option<DatasetBudget>,
    videos: &[Value],
) -> Result<usize> {
    if videos.is_empty() {
        return Ok(0);
    }
    if dataset_budget.is_none() {
        *dataset_budget = Some(run_dataset_budget(client, config).await?);
    }
    let budget = dataset_budget.as_mut().expect("dataset budget initialized");
    let saved = budget.affordable_items(videos.len())?;
    if saved == 0 {
        return Ok(0);
    }
    let next_saved_rows = budget
        .locally_saved_rows
        .checked_add(saved)
        .ok_or_else(|| anyhow!("Local dataset row count overflowed"))?;

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(&videos[..saved])
        .send()
        .await
        .context("Apify dataset write failed")?;
    ensure_success(response, "Apify dataset write").await?;
    budget.locally_saved_rows = next_saved_rows;
    Ok(saved)
}
async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let ids = get_channel_ids(&input);
    if ids.is_empty() {
        bail!("At least one YouTube channel ID must be provided in \"ids\" or \"id\".");
    }
    assert_continuation_matches_batch(&input, &ids)?;

    let mut dataset_budget = None;
    for id in ids {
        let (videos, continuation) =
            collect_filtered_channel_videos(client, config, &input, &id).await?;
        let saved = push_dataset_items(client, config, &mut dataset_budget, &videos).await?;
        println!(
            "Successfully fetched {} videos for query: {id}; saved {saved} to the default dataset",
            videos.len(),
        );
        if js_truthy(&continuation) {
            println!(
                "Continuation token available for next page: {}",
                js_string(&continuation)
            );
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::new();
    run_actor(&client, &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
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
            let (recorded_requests, requests) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                let mut responses = responses.into_iter();
                while !stopped.load(Ordering::Relaxed) && Instant::now() < deadline {
                    let (mut stream, _) = match listener.accept() {
                        Ok(connection) => connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                            continue;
                        }
                        Err(_) => break,
                    };
                    let request = read_request(&mut stream).unwrap_or_default();
                    if recorded_requests.send(request).is_err() {
                        break;
                    }
                    let Some(response) = responses.next() else {
                        break;
                    };
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
                        break;
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

        fn scrappa_url(&self) -> Url {
            self.base_url.join("api/youtube/channel-videos").unwrap()
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
        let mut header_end = None;
        let mut content_length = None;
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if header_end.is_none() {
                header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
                if let Some(header_end) = header_end {
                    let headers = String::from_utf8_lossy(&bytes[..header_end]);
                    content_length = Some(
                        headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0),
                    );
                }
            }
            if let (Some(header_end), Some(content_length)) = (header_end, content_length) {
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn test_config(server: &MockServer) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: server.base_url.clone(),
            scrappa_api_base_url: server.scrappa_url(),
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "CUSTOM_INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }
    fn pricing_response(max_charge: f64, charged_counts: Value) -> MockResponse {
        response(
            200,
            &json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            "synthetic-other-event": {"eventPriceUsd": 0.0001}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_charge},
                    "chargedEventCounts": charged_counts
                }
            })
            .to_string(),
        )
    }

    fn two_live_rows() -> &'static str {
        r#"{"videos":[{"id":"live-1","type":"live"},{"id":"live-2","type":"livestream"}]}"#
    }

    fn request_target(request: &str) -> &str {
        request
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
    }

    fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
        request.lines().find_map(|line| {
            let (header, value) = line.split_once(':')?;
            header.eq_ignore_ascii_case(name).then_some(value.trim())
        })
    }

    fn request_body(request: &str) -> Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    #[test]
    fn input_schema_keeps_single_and_batch_channel_prefills() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["id"]["prefill"],
            "UCJZv4d5rbIKd4QHMPkcABCw"
        );
        assert_eq!(
            schema["properties"]["ids"]["prefill"],
            "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw"
        );
    }

    #[test]
    fn parses_and_deduplicates_batch_ids_and_rejects_batch_continuations() {
        let input = json!({"ids": "UC1, UC2", "id": "UC2,UC3"});
        let ids = get_channel_ids(&input);
        assert_eq!(
            ids,
            vec!["UC1".to_owned(), "UC2".to_owned(), "UC3".to_owned()]
        );
        assert!(
            assert_continuation_matches_batch(&json!({"continuation": "next page"}), &ids).is_err()
        );
    }

    #[test]
    fn builds_encoded_scrappa_query_using_original_endpoint() {
        let url = build_channel_livestreams_url(
            &Url::parse(SCRAPPA_API_BASE_URL).unwrap(),
            "UC example",
            Some("popular"),
            &json!("next page"),
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://scrappa.co/api/youtube/channel-videos?channel_id=UC+example&sort=popular&continuation=next+page"
        );
    }

    #[tokio::test]
    async fn batches_live_rows_with_bounded_pagination_and_apify_auth() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"UC1, UC2","id":"UC2","sort":"popular"}"#),
            response(
                200,
                r#"{"videos":[{"id":"regular","type":"VIDEO"},{"id":"live-1","type":"LiVe"}],"pagination":{"continuationToken":"page 2"}}"#,
            ),
            response(
                200,
                r#"{"videos":[{"id":"live-2","isLive":true}],"pagination":{"continuationToken":"page 3"}}"#,
            ),
            response(200, r#"{"videos":[{"id":"not-live","isLive":"true"}]}"#),
            pricing_response(
                1.0,
                json!({"apify-default-dataset-item": 0, "synthetic-other-event": 0}),
            ),
            response(201, "{}"),
            response(
                200,
                r#"{"videos":[{"id":"live-3","videoType":"livestream"}]}"#,
            ),
            response(201, "{}"),
        ]);
        let config = test_config(&server);
        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 8);
        assert!(request_target(&requests[0])
            .starts_with("/v2/key-value-stores/test-store/records/CUSTOM_INPUT"));
        assert_eq!(
            header_value(&requests[0], "authorization"),
            Some("Bearer test-token-not-a-real-credential")
        );
        assert_eq!(
            header_value(&requests[1], "x-api-key"),
            Some("test-scrappa-key")
        );
        assert!(request_target(&requests[1]).contains("channel_id=UC1&sort=popular"));
        assert!(request_target(&requests[2])
            .contains("channel_id=UC1&sort=popular&continuation=page+2"));
        assert!(request_target(&requests[3]).contains("continuation=page+3"));
        assert_eq!(request_target(&requests[4]), "/v2/actor-runs/test-run");
        assert_eq!(
            header_value(&requests[4], "authorization"),
            Some("Bearer test-token-not-a-real-credential")
        );
        assert_eq!(
            request_body(&requests[5]),
            json!([
                {"id":"live-1","type":"LiVe"},
                {"id":"live-2","isLive":true}
            ])
        );
        assert!(request_target(&requests[6]).contains("channel_id=UC2&sort=popular"));
        assert_eq!(
            request_body(&requests[7]),
            json!([{"id":"live-3","videoType":"livestream"}])
        );
    }

    #[tokio::test]
    async fn cap_one_posts_only_the_first_row_and_counts_other_charges() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC1"}"#),
            response(200, two_live_rows()),
            pricing_response(
                0.0006,
                json!({"apify-default-dataset-item": 0, "synthetic-other-event": 1}),
            ),
            response(201, "{}"),
        ]);
        let config = test_config(&server);

        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(request_target(&requests[2]), "/v2/actor-runs/test-run");
        assert_eq!(
            request_body(&requests[3]),
            json!([{"id":"live-1","type":"live"}])
        );
    }

    #[tokio::test]
    async fn zero_budget_skips_dataset_post() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC1"}"#),
            response(200, two_live_rows()),
            pricing_response(
                0.0,
                json!({"apify-default-dataset-item": 0, "synthetic-other-event": 0}),
            ),
        ]);
        let config = test_config(&server);

        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request_target(request).starts_with("/v2/datasets/")));
    }

    #[tokio::test]
    async fn generous_numeric_budget_posts_all_rows() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC1"}"#),
            response(200, two_live_rows()),
            pricing_response(
                1.0,
                json!({"apify-default-dataset-item": 0, "synthetic-other-event": 0}),
            ),
            response(201, "{}"),
        ]);
        let config = test_config(&server);

        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(
            request_body(&requests[3]),
            json!([
                {"id":"live-1","type":"live"},
                {"id":"live-2","type":"livestream"}
            ])
        );
    }

    #[tokio::test]
    async fn local_saved_rows_prevent_overspend_across_channel_writes() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"UC1,UC2"}"#),
            response(200, r#"{"videos":[{"id":"live-1","type":"live"}]}"#),
            pricing_response(
                0.0003,
                json!({"apify-default-dataset-item": 0, "synthetic-other-event": 0}),
            ),
            response(201, "{}"),
            response(200, r#"{"videos":[{"id":"live-2","type":"live"}]}"#),
        ]);
        let config = test_config(&server);

        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert_eq!(
            request_body(&requests[3]),
            json!([{"id":"live-1","type":"live"}])
        );
        assert!(!request_target(&requests[4]).starts_with("/v2/datasets/"));
    }

    #[tokio::test]
    async fn missing_dataset_event_price_fails_before_any_dataset_post() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC1"}"#),
            response(200, two_live_rows()),
            response(
                200,
                r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{}}},"options":{"maxTotalChargeUsd":1.0},"chargedEventCounts":{}}}"#,
            ),
        ]);
        let config = test_config(&server);

        let error = run_actor(&Client::new(), &config).await.unwrap_err();

        assert!(error.to_string().contains("dataset item price"));
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request_target(request).starts_with("/v2/datasets/")));
    }

    #[tokio::test]
    async fn dataset_storage_errors_fail_the_actor() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC1"}"#),
            response(200, two_live_rows()),
            pricing_response(
                1.0,
                json!({"apify-default-dataset-item": 0, "synthetic-other-event": 0}),
            ),
            response(500, "storage failed"),
        ]);
        let config = test_config(&server);

        let error = run_actor(&Client::new(), &config).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("Apify dataset write failed with 500"));
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(request_target(&requests[3]).starts_with("/v2/datasets/test-dataset/items"));
    }

    #[tokio::test]
    async fn stops_after_reaching_ten_live_rows_and_returns_the_next_token() {
        let first_page = json!({
            "videos": (0..9)
                .map(|index| json!({"id": format!("first-{index}"), "type": "live"}))
                .collect::<Vec<_>>(),
            "continuation": "page-2"
        });
        let second_page = json!({
            "videos": (0..2)
                .map(|index| json!({"id": format!("second-{index}"), "type": "livestream"}))
                .collect::<Vec<_>>(),
            "continuation": "page-3"
        });
        let server = MockServer::start(vec![
            response(200, &first_page.to_string()),
            response(200, &second_page.to_string()),
        ]);
        let config = test_config(&server);

        let (videos, continuation) = collect_filtered_channel_videos(
            &Client::new(),
            &config,
            &json!({"sort": "newest"}),
            "UC1",
        )
        .await
        .unwrap();

        assert_eq!(videos.len(), 11);
        assert_eq!(videos[10]["id"], "second-1");
        assert_eq!(continuation, "page-3");
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert!(request_target(&requests[1]).contains("continuation=page-2"));
    }

    #[tokio::test]
    async fn stops_scanning_after_ten_pages() {
        let mut responses = vec![response(200, r#"{"id":"UC1"}"#)];
        for page in 1..=MAX_FILTER_SCAN_PAGES {
            responses.push(response(
                200,
                &format!(r#"{{"videos":[{{"id":"video-{page}","type":"video"}}],"continuation":"page-{}"}}"#, page + 1),
            ));
        }
        let server = MockServer::start(responses);
        let config = test_config(&server);
        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 1 + MAX_FILTER_SCAN_PAGES);
        assert!(request_target(&requests[10]).contains("continuation=page-10"));
    }

    #[tokio::test]
    async fn scrappa_errors_keep_status_and_reason() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC1"}"#),
            response(401, "not authorized"),
        ]);
        let config = test_config(&server);

        let error = run_actor(&Client::new(), &config).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API request failed with 401 Unauthorized"
        );
        assert_eq!(server.requests().len(), 2);
    }
}
