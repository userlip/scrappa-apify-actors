use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{env, time::Duration};
use url::Url;

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

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().map_or(true, |number| number != 0.0),
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
            .map(|value| match value {
                Value::Null => String::new(),
                _ => js_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn encode_component(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

fn build_channel_videos_url(input: &Value, api_base_url: &Url) -> Result<Url> {
    let id = input
        .get("id")
        .filter(|value| js_truthy(value))
        .ok_or_else(|| {
            anyhow!(
                "Search query \"id\" not provided. Please provide a value for \"id\" in the input."
            )
        })?;
    let mut url = endpoint_url(api_base_url, &["channels", "videos"])?;
    let mut query = format!("id={}", encode_component(&js_string(id)));

    let sort = input.get("sort").and_then(|sort| match sort {
        Value::Array(values) => values.first(),
        _ => Some(sort),
    });
    if let Some(sort) = sort
        .and_then(Value::as_str)
        .filter(|sort| !sort.trim().is_empty())
    {
        query.push_str("&sort=");
        query.push_str(&encode_component(sort));
    }

    if let Some(continuation) = input
        .get("continuation")
        .and_then(Value::as_str)
        .filter(|continuation| !continuation.trim().is_empty())
    {
        query.push_str("&continuation=");
        query.push_str(&encode_component(continuation));
    }

    url.set_query(Some(&query));
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

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    let message = error.to_string();
    if error.is_timeout() || message.contains("aborted") {
        anyhow!(
            "Scrappa API request timed out after {}s",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        anyhow!(message)
    }
}

async fn fetch_channel_videos(client: &Client, url: &Url) -> Result<Value> {
    let response = client
        .get(url.clone())
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
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

async fn run_dataset_capacity(
    client: &Client,
    config: &ActorConfig,
    requested: usize,
) -> Result<usize> {
    if requested == 0 {
        return Ok(0);
    }
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

fn dataset_item_count(videos: &Value) -> usize {
    match videos {
        Value::Array(items) => items.len(),
        Value::Null => 0,
        _ => 1,
    }
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    videos: &Value,
) -> Result<usize> {
    let requested = dataset_item_count(videos);
    if requested == 0 {
        return Ok(0);
    }
    let item_count = requested.min(run_dataset_capacity(client, config, requested).await?);
    if item_count == 0 {
        return Ok(0);
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let request = client.post(url).bearer_auth(&config.apify_token);
    let response = if let Some(items) = videos.as_array() {
        request.json(&items[..item_count]).send().await
    } else {
        request.json(videos).send().await
    }
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
    Ok(item_count)
}

fn js_length(value: &Value) -> Option<String> {
    match value {
        Value::Array(values) => Some(values.len().to_string()),
        Value::String(value) => Some(value.encode_utf16().count().to_string()),
        Value::Object(value) => value.get("length").map(js_string),
        _ => None,
    }
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let url = build_channel_videos_url(&input, &config.scrappa_api_base_url)?;
    println!("Fetching from: {url}");

    let response_data = fetch_channel_videos(client, &url).await?;
    let videos = response_data
        .get("videos")
        .filter(|videos| !videos.is_null())
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let saved_count = push_dataset_items(client, config, &videos).await?;

    let channel_id = input
        .get("id")
        .map(js_string)
        .unwrap_or_else(|| "undefined".to_owned());
    let video_count = js_length(&videos).unwrap_or_else(|| "undefined".to_owned());
    println!("Successfully fetched {video_count} videos for channel id: {channel_id}");
    println!("Saved {saved_count} videos to the default dataset");

    if let Some(continuation) = response_data
        .get("continuation")
        .filter(|continuation| js_truthy(continuation))
    {
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
        eprintln!("Failed to fetch YouTube channel videos: {error:#}");
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

    fn test_config(base_url: Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: base_url,
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
        }
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
    fn builds_channel_video_url_and_preserves_javascript_query_encoding() {
        let base_url = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let url = build_channel_videos_url(
            &serde_json::json!({
                "id": "UC example",
                "sort": ["popular"],
                "continuation": "next page"
            }),
            &base_url,
        )
        .unwrap();

        assert_eq!(
            url.as_str(),
            "https://ytapi.scrappa.co/channels/videos?id=UC%20example&sort=popular&continuation=next%20page"
        );
    }

    #[test]
    fn missing_or_empty_channel_id_keeps_the_existing_error() {
        let base_url = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let error = build_channel_videos_url(&serde_json::json!({"sort": "newest"}), &base_url)
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Search query \"id\" not provided. Please provide a value for \"id\" in the input."
        );
        assert!(build_channel_videos_url(&serde_json::json!({"id": ""}), &base_url).is_err());
    }

    #[test]
    fn ignores_blank_or_non_string_optional_query_values() {
        let base_url = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let url = build_channel_videos_url(
            &serde_json::json!({"id": "UC123", "sort": ["  "], "continuation": " "}),
            &base_url,
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://ytapi.scrappa.co/channels/videos?id=UC123"
        );
    }

    #[tokio::test]
    async fn writes_ordered_video_array_and_keeps_scrappa_unauthenticated() {
        let server = MockServer::start(vec![
            response(
                200,
                r#"{"id":"UC123","sort":["popular"],"continuation":"next page"}"#,
            ),
            response(
                200,
                r#"{"videos":[{"id":"first"},{"id":"second"}],"continuation":"next page"}"#,
            ),
            pricing_response(1.0, serde_json::json!({})),
            response(201, "{}"),
        ]);
        let config = test_config(server.base_url.clone());
        let client = Client::new();

        run_actor(&client, &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(
            requests.len(),
            4,
            "the continuation token is logged, not fetched automatically"
        );
        assert!(requests[0].starts_with("GET /v2/key-value-stores/test-store/records/INPUT "));
        assert_eq!(
            header_value(&requests[0], "authorization"),
            Some("Bearer test-token-not-a-real-credential")
        );
        assert!(requests[1]
            .starts_with("GET /channels/videos?id=UC123&sort=popular&continuation=next%20page "));
        assert!(header_value(&requests[1], "authorization").is_none());
        assert!(header_value(&requests[1], "x-api-key").is_none());
        assert!(requests[2].starts_with("GET /v2/actor-runs/test-run "));
        assert_eq!(
            header_value(&requests[2], "authorization"),
            Some("Bearer test-token-not-a-real-credential")
        );
        assert!(requests[3].starts_with("POST /v2/datasets/test-dataset/items "));
        assert_eq!(
            header_value(&requests[3], "authorization"),
            Some("Bearer test-token-not-a-real-credential")
        );
        assert_eq!(
            request_body(&requests[3]),
            serde_json::json!([{"id":"first"},{"id":"second"}])
        );
    }

    #[tokio::test]
    async fn pushes_object_videos_as_one_dataset_item() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC123"}"#),
            response(200, r#"{"videos":{"id":"single"}}"#),
            pricing_response(0.0003, serde_json::json!({})),
            response(201, "{}"),
        ]);
        let config = test_config(server.base_url.clone());

        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(
            request_body(&requests[3]),
            serde_json::json!({"id":"single"})
        );
    }

    #[tokio::test]
    async fn capped_run_posts_only_the_first_affordable_video() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC123"}"#),
            response(200, r#"{"videos":[{"id":"first"},{"id":"second"}]}"#),
            pricing_response(0.0003, serde_json::json!({})),
            response(201, "{}"),
        ]);
        let config = test_config(server.base_url.clone());

        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests[2].starts_with("GET /v2/actor-runs/test-run "));
        assert_eq!(
            request_body(&requests[3]),
            serde_json::json!([{"id":"first"}])
        );
    }

    #[tokio::test]
    async fn zero_budget_skips_the_dataset_post() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC123"}"#),
            response(200, r#"{"videos":[{"id":"first"},{"id":"second"}]}"#),
            pricing_response(0.0, serde_json::json!({})),
        ]);
        let config = test_config(server.base_url.clone());

        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));
    }

    #[test]
    fn all_charged_events_reduce_capacity_and_missing_pricing_fails_closed() {
        let run = serde_json::json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                "apify-actor-start": {"eventPriceUsd": 0.0001}
            }}},
            "options": {"maxTotalChargeUsd": 0.00065},
            "chargedEventCounts": {"apify-default-dataset-item": 1, "apify-actor-start": 1}
        }});
        assert_eq!(affordable_dataset_items(&run, 2).unwrap(), 0);

        let missing_counts = serde_json::json!({"data": {
            "pricingInfo": run["data"]["pricingInfo"],
            "options": {"maxTotalChargeUsd": 0.0004}
        }});
        assert!(affordable_dataset_items(&missing_counts, 2).is_err());

        let null_limit = serde_json::json!({"data": {
            "pricingInfo": run["data"]["pricingInfo"],
            "options": {"maxTotalChargeUsd": null},
            "chargedEventCounts": {}
        }});
        assert!(affordable_dataset_items(&null_limit, 2).is_err());
    }

    #[tokio::test]
    async fn missing_run_pricing_fails_without_posting_videos() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC123"}"#),
            response(200, r#"{"videos":[{"id":"first"},{"id":"second"}]}"#),
            response(
                200,
                r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"}}}"#,
            ),
        ]);
        let config = test_config(server.base_url.clone());

        let error = run_actor(&Client::new(), &config).await.unwrap_err();

        assert!(error.to_string().contains("event prices"));
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));
    }

    #[tokio::test]
    async fn dataset_storage_error_propagates_after_the_capped_write() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC123"}"#),
            response(200, r#"{"videos":[{"id":"first"},{"id":"second"}]}"#),
            pricing_response(0.0003, serde_json::json!({})),
            response(500, "dataset unavailable"),
        ]);
        let config = test_config(server.base_url.clone());

        let error = run_actor(&Client::new(), &config).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("Apify dataset write failed with 500"));
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(
            request_body(&requests[3]),
            serde_json::json!([{"id":"first"}])
        );
    }

    #[tokio::test]
    async fn missing_videos_maps_to_empty_array_without_a_dataset_write() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC123"}"#),
            response(200, r#"{"videos":null}"#),
        ]);
        let config = test_config(server.base_url.clone());

        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
    }

    #[tokio::test]
    async fn scrappa_http_errors_keep_status_and_reason() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC123"}"#),
            response(401, "not authorized"),
        ]);
        let config = test_config(server.base_url.clone());

        let error = run_actor(&Client::new(), &config).await.unwrap_err();

        assert_eq!(
            error.to_string(),
            "Scrappa API request failed with 401 Unauthorized"
        );
        assert_eq!(server.requests().len(), 2);
    }
}
