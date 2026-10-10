use crate::apify_retry::ApifyRetryExt;
mod apify_retry;
use crate::scrappa_retry::ScrappaRetryExt;
mod scrappa_retry;
use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use std::{env, time::Duration};
use tokio::{task::JoinSet, time::timeout};
use url::{form_urlencoded, Url};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/youtube";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const ACTOR_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_CONCURRENT_VIDEO_REQUESTS: usize = 5;

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
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|api_key| !api_key.is_empty())
            .ok_or_else(|| anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."))?;
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

fn batch_video_ids(input: &Value) -> Result<Vec<String>> {
    let ids = input
        .as_object()
        .and_then(|object| object.get("ids"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|ids| !ids.is_empty())
        .ok_or_else(|| anyhow!("Video IDs \"ids\" are required."))?;

    let normalized_ids: Vec<String> = ids
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
        .collect();
    if normalized_ids.is_empty() {
        bail!("Video IDs \"ids\" must include at least one non-empty ID.");
    }
    if normalized_ids.len() > 50 {
        bail!("Video IDs \"ids\" must contain 50 or fewer comma-separated IDs.");
    }
    Ok(normalized_ids)
}

fn build_video_url(api_base_url: &Url, id: &str) -> Result<Url> {
    let mut url = endpoint_url(api_base_url, &["video"])?;
    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("video_id", id)
        .finish();
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
        .send_apify_with_retry()
        .await
        .context("Apify INPUT request failed")?;
    response_json(response, "Apify INPUT request").await
}

/// Fetches one video. A 404 means YouTube has no such video; the batch skips it
/// like the old batch endpoint did, so the other videos still reach the dataset.
async fn fetch_video(client: Client, url: Url, api_key: String) -> Result<Option<Value>> {
    let response = client
        .get(url)
        .header("X-API-Key", api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .send_scrappa_with_retry("Scrappa API request")
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "Scrappa API request timed out after {}s",
                    REQUEST_TIMEOUT.as_secs()
                )
            } else {
                anyhow!("Scrappa API request failed: {error}")
            }
        })?;

    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        bail!(
            "Scrappa API request failed with {} {reason}",
            status.as_u16()
        );
    }

    let data: Value = response
        .json()
        .await
        .context("Scrappa API response was not valid JSON")?;
    if !data.is_object() {
        bail!("Scrappa API response is not a video object");
    }
    Ok(Some(data))
}

/// Fetches every video with at most MAX_CONCURRENT_VIDEO_REQUESTS requests in flight
/// and returns them in input order. The first failed request fails the batch and
/// aborts the requests that are still running.
async fn fetch_batch_videos(
    client: &Client,
    config: &ActorConfig,
    ids: &[String],
) -> Result<Vec<Value>> {
    let mut videos: Vec<Option<Value>> = vec![None; ids.len()];
    for (chunk_index, chunk) in ids.chunks(MAX_CONCURRENT_VIDEO_REQUESTS).enumerate() {
        let mut requests = JoinSet::new();
        for (offset, id) in chunk.iter().enumerate() {
            let url = build_video_url(&config.scrappa_api_base_url, id)?;
            let index = chunk_index * MAX_CONCURRENT_VIDEO_REQUESTS + offset;
            let request = fetch_video(client.clone(), url, config.scrappa_api_key.clone());
            requests.spawn(async move { (index, request.await) });
        }
        while let Some(joined) = requests.join_next().await {
            let (index, result) = joined.context("Scrappa API request task failed")?;
            match result? {
                Some(video) => videos[index] = Some(video),
                None => eprintln!("Video {} was not found; skipping it", ids[index]),
            }
        }
    }
    Ok(videos.into_iter().flatten().collect())
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
        .send_apify_with_retry()
        .await
        .context("Apify run pricing request failed")?;
    let run = response_json(response, "Apify run pricing request").await?;
    affordable_dataset_items(&run, requested)
}

// maxItems is for pay-per-result; PPE dataset writes consume the run's event budget.
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

async fn push_dataset_items(client: &Client, config: &ActorConfig, videos: &[Value]) -> Result<()> {
    if videos.is_empty() {
        return Ok(());
    }
    let limit = run_dataset_capacity(client, config, videos.len()).await?;
    let videos = &videos[..videos.len().min(limit)];
    if videos.is_empty() {
        return Ok(());
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(videos)
        .send_apify_with_retry()
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
    Ok(())
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let ids = batch_video_ids(&input)?;
    println!("Fetching data from Scrappa API");

    let videos = fetch_batch_videos(client, config, &ids).await?;
    push_dataset_items(client, config, &videos).await?;

    let ids = input.get("ids").and_then(Value::as_str).unwrap_or_default();
    println!(
        "Successfully fetched {} batch video(s) for ids: {ids}",
        videos.len()
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube batch videos: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("Could not create HTTP client")?;

    timeout(ACTOR_TIMEOUT, run_actor(&client, &config))
        .await
        .map_err(|_| anyhow!("Actor timed out after {}s", ACTOR_TIMEOUT.as_secs()))??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
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
        requests: Arc<Mutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let recorded_requests = Arc::clone(&requests);
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
                    recorded_requests.lock().unwrap().push(request);
                    let Some(response) = responses.next() else {
                        break;
                    };
                    if response.status == 0 {
                        continue;
                    }
                    let body = response.body.replace(
                        "{video_id}",
                        &requested_video_id(
                            &recorded_requests
                                .lock()
                                .unwrap()
                                .last()
                                .cloned()
                                .unwrap_or_default(),
                        ),
                    );
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        404 => "Not Found",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
                        504 => "Gateway Timeout",
                        _ => "Mock Response",
                    };
                    let message = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status, reason, body.len(), body
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
            self.requests.lock().unwrap().clone()
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

    /// Reads video_id from a recorded request so mock bodies can echo it back.
    fn requested_video_id(request: &str) -> String {
        let path = request_parts(request).1;
        Url::parse(&format!("http://mock{path}"))
            .ok()
            .and_then(|url| {
                url.query_pairs()
                    .find(|(key, _)| key == "video_id")
                    .map(|(_, value)| value.into_owned())
            })
            .unwrap_or_default()
    }

    fn header_value<'a>(request: &'a str, header: &str) -> Option<&'a str> {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case(header).then_some(value.trim())
            })
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    fn pricing_response(max_charge: f64, charged: u64) -> MockResponse {
        response(
            200,
            &serde_json::json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_charge},
                    "chargedEventCounts": {"apify-default-dataset-item": charged}
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
            scrappa_api_key: "test-scrappa-key-not-a-real-credential".to_owned(),
        }
    }

    fn client() -> Client {
        Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap()
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
    fn ids_are_normalized_and_mapped_to_scrappa_video_urls() {
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let ids = batch_video_ids(&serde_json::json!({"ids":" video-one, , video-two "})).unwrap();
        assert_eq!(ids, ["video-one", "video-two"]);
        assert_eq!(
            build_video_url(&base, &ids[0]).unwrap().as_str(),
            "https://scrappa.co/api/youtube/video?video_id=video-one"
        );
        assert!(batch_video_ids(&serde_json::json!({"ids":" , , "}))
            .unwrap_err()
            .to_string()
            .contains("at least one non-empty ID"));
        let too_many = (0..51)
            .map(|index| format!("id-{index}"))
            .collect::<Vec<_>>()
            .join(",");
        assert!(batch_video_ids(&serde_json::json!({"ids":too_many}))
            .unwrap_err()
            .to_string()
            .contains("50 or fewer"));
    }

    #[test]
    fn qa_schema_prefill_survives_the_rust_url_builder() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let prefill = schema["properties"]["ids"]["prefill"].as_str().unwrap();
        let ids = batch_video_ids(&serde_json::json!({"ids":prefill})).unwrap();
        assert_eq!(ids, [prefill]);
        let url = build_video_url(&Url::parse(SCRAPPA_API_BASE_URL).unwrap(), &ids[0]).unwrap();
        assert_eq!(
            url.query_pairs()
                .find(|(key, _)| key == "video_id")
                .unwrap()
                .1,
            prefill
        );
    }

    #[tokio::test]
    async fn local_apify_and_scrappa_mocks_preserve_rows_and_retry_504() {
        let videos = serde_json::json!([{"id":"7eul_Vt6SZY"}]);
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let ids = schema["properties"]["ids"]["prefill"].as_str().unwrap();
        let input = serde_json::json!({"ids":ids});
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(504, "{}"),
            response(200, r#"{"id":"{video_id}"}"#),
            pricing_response(1.0, 0),
            response(201, "{}"),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        let (method, path, _) = request_parts(&requests[0]);
        assert_eq!(
            (method, path),
            ("GET", "/v2/key-value-stores/test-store/records/INPUT")
        );
        let (method, path, _) = request_parts(&requests[1]);
        assert_eq!(method, "GET");
        assert_eq!(path, "/video?video_id=7eul_Vt6SZY");
        assert_eq!(request_parts(&requests[1]).1, request_parts(&requests[2]).1);
        assert_eq!(request_parts(&requests[3]).1, "/v2/actor-runs/test-run");
        let (method, path, body) = request_parts(&requests[4]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert_eq!(serde_json::from_str::<Value>(body).unwrap(), videos);
        assert!(requests
            .iter()
            .filter(|request| request.contains("/v2/"))
            .all(|request| has_test_bearer_token(request)));
        assert!(requests
            .iter()
            .filter(|request| request.starts_with("GET /video?"))
            .all(|request| !has_test_bearer_token(request)
                && header_value(request, "x-api-key")
                    == Some("test-scrappa-key-not-a-real-credential")));
        assert!(requests
            .iter()
            .filter(|request| request.contains("/v2/"))
            .all(|request| header_value(request, "x-api-key").is_none()));
    }

    #[tokio::test]
    async fn fails_fast_on_a_408_response() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"video"}"#),
            response(408, "{}"),
        ]);
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("408 Request Timeout"));
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
    }

    #[tokio::test]
    async fn capped_run_stores_only_chargeable_items() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"video-one,video-two"}"#),
            response(200, r#"{"id":"{video_id}"}"#),
            response(200, r#"{"id":"{video_id}"}"#),
            pricing_response(0.0003, 0),
            response(201, "{}"),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert_eq!(request_parts(&requests[3]).1, "/v2/actor-runs/test-run");
        assert!(has_test_bearer_token(&requests[3]));
        let (_, _, body) = request_parts(&requests[4]);
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::json!([{"id":"video-one"}])
        );
    }

    #[tokio::test]
    async fn exhausted_run_skips_dataset_write() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"video"}"#),
            response(200, r#"{"id":"video"}"#),
            pricing_response(0.0, 0),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        assert_eq!(server.requests().len(), 3);
    }

    #[test]
    fn previously_charged_events_reduce_available_capacity() {
        let run = serde_json::json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0002},
                "apify-actor-start": {"eventPriceUsd": 0.00005}
            }}},
            "options": {"maxTotalChargeUsd": 0.0005},
            "chargedEventCounts": {"apify-default-dataset-item": 1, "apify-actor-start": 1}
        }});
        assert_eq!(affordable_dataset_items(&run, 2).unwrap(), 1);
        assert!(affordable_dataset_items(&serde_json::json!({"data": {}}), 1).is_err());
        let mut missing_counts = run.clone();
        missing_counts["data"]
            .as_object_mut()
            .unwrap()
            .remove("chargedEventCounts");
        assert!(affordable_dataset_items(&missing_counts, 2).is_err());
    }

    #[tokio::test]
    async fn persistent_504_fails_after_three_upstream_attempts_without_dataset_rows() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"video"}"#),
            response(504, "{}"),
            response(504, "{}"),
            response(504, "{}"),
        ]);
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("504 Gateway Timeout"));
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));
    }

    #[tokio::test]
    async fn non_retryable_4xx_and_malformed_success_do_not_write_dataset_rows() {
        let bad_request = MockServer::start(vec![
            response(200, r#"{"ids":"video"}"#),
            response(400, "{}"),
        ]);
        let error = run_actor(&client(), &config(&bad_request.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("400 Bad Request"));
        assert_eq!(bad_request.requests().len(), 2);
        let malformed = MockServer::start(vec![
            response(200, r#"{"ids":"video"}"#),
            response(200, r#"[]"#),
        ]);
        let error = run_actor(&client(), &config(&malformed.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("not a video object"));
        assert_eq!(malformed.requests().len(), 2);
    }

    #[tokio::test]
    async fn retryable_scrappa_response_retries_and_apify_errors_propagate() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"video"}"#),
            response(503, r#"{"message":"Unavailable"}"#),
            response(200, r#"{"id":"video"}"#),
            pricing_response(1.0, 0),
            response(201, "{}"),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        assert_eq!(server.requests().len(), 5);

        let apify_error = MockServer::start(vec![response(401, "unauthorized")]);
        let error = run_actor(&client(), &config(&apify_error.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("401 Unauthorized"));
        assert_eq!(apify_error.requests().len(), 1);
    }

    #[tokio::test]
    async fn missing_videos_are_skipped_like_the_old_batch_endpoint() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"video-gone"}"#),
            response(404, r#"{"error":"Video not found"}"#),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_parts(&requests[1]).1, "/video?video_id=video-gone");
    }

    /// Answers each wave of parallel video requests after the client stops opening
    /// new connections, records the wave size and echoes the requested video ID.
    fn start_wave_server() -> (Url, Arc<Mutex<Vec<usize>>>, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base_url = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let waves = Arc::new(Mutex::new(Vec::new()));
        let recorded_waves = Arc::clone(&waves);
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut answered = 0;
            while answered < 12 && Instant::now() < deadline {
                let mut wave = Vec::new();
                let mut idle_since = Instant::now();
                while idle_since.elapsed() < Duration::from_millis(200) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            stream.set_nonblocking(false).unwrap();
                            wave.push(stream);
                            idle_since = Instant::now();
                        }
                        Err(_) => thread::sleep(Duration::from_millis(5)),
                    }
                }
                if wave.is_empty() {
                    continue;
                }
                recorded_waves.lock().unwrap().push(wave.len());
                // Answer in reverse order so completion order differs from input order.
                for mut stream in wave.into_iter().rev() {
                    let request = read_request(&mut stream).unwrap_or_default();
                    let body = format!(r#"{{"id":"{}"}}"#, requested_video_id(&request));
                    let message = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(message.as_bytes());
                    answered += 1;
                }
            }
        });
        (base_url, waves, thread)
    }

    #[tokio::test]
    async fn fetches_at_most_five_videos_at_once_and_keeps_input_order() {
        let (scrappa_url, waves, thread) = start_wave_server();
        let mut config = config(&scrappa_url);
        config.scrappa_api_base_url = scrappa_url;
        let ids: Vec<String> = (0..12).map(|index| format!("video-{index:02}")).collect();

        let videos = fetch_batch_videos(&client(), &config, &ids).await.unwrap();
        thread.join().unwrap();

        let fetched: Vec<&str> = videos
            .iter()
            .map(|video| video["id"].as_str().unwrap())
            .collect();
        assert_eq!(fetched, ids);
        assert_eq!(*waves.lock().unwrap(), [5, 5, 2]);
    }
}
