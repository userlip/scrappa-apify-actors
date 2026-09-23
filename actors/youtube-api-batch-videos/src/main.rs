use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, StatusCode};
use serde_json::Value;
use std::{env, time::Duration};
use tokio::time::{sleep, timeout};
use url::{form_urlencoded, Url};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://ytapi.scrappa.co";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const ACTOR_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_ATTEMPTS: u8 = 3;

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

fn build_batch_videos_url(input: &Value, api_base_url: &Url) -> Result<Url> {
    let ids = input
        .as_object()
        .and_then(|object| object.get("ids"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|ids| !ids.is_empty())
        .ok_or_else(|| anyhow!("Video IDs \"ids\" are required."))?;

    let normalized_ids: Vec<&str> = ids
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .collect();
    if normalized_ids.is_empty() {
        bail!("Video IDs \"ids\" must include at least one non-empty ID.");
    }
    if normalized_ids.len() > 50 {
        bail!("Video IDs \"ids\" must contain 50 or fewer comma-separated IDs.");
    }

    let mut url = endpoint_url(api_base_url, &["videos", "batch"])?;
    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("ids", &normalized_ids.join(","))
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
        .send()
        .await
        .context("Apify INPUT request failed")?;
    response_json(response, "Apify INPUT request").await
}

async fn fetch_batch_videos(client: &Client, url: &Url) -> Result<Vec<Value>> {
    for attempt in 1..=MAX_ATTEMPTS {
        let response = client.get(url.clone()).send().await;
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                let message = if error.is_timeout() {
                    format!(
                        "Scrappa API request timed out after {}s",
                        REQUEST_TIMEOUT.as_secs()
                    )
                } else {
                    format!("Scrappa API request failed: {error}")
                };
                if attempt == MAX_ATTEMPTS {
                    bail!(message);
                }
                eprintln!("{message}; retrying ({attempt}/{MAX_ATTEMPTS})");
                sleep(Duration::from_secs(u64::from(attempt))).await;
                continue;
            }
        };

        let status = response.status();
        if !status.is_success() {
            let reason = status.canonical_reason().unwrap_or("Unknown status");
            let message = format!(
                "Scrappa API request failed with {} {reason}",
                status.as_u16()
            );
            let retryable = matches!(
                status,
                StatusCode::REQUEST_TIMEOUT | StatusCode::TOO_MANY_REQUESTS
            ) || status.is_server_error();
            if attempt == MAX_ATTEMPTS || !retryable {
                bail!(message);
            }
            eprintln!("{message}; retrying ({attempt}/{MAX_ATTEMPTS})");
            sleep(Duration::from_secs(u64::from(attempt))).await;
            continue;
        }

        let data: Value = response
            .json()
            .await
            .context("Scrappa API response was not valid JSON")?;
        return data
            .get("videos")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| anyhow!("Scrappa API response is missing the videos array"));
    }

    unreachable!("the retry loop always returns or fails")
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
    Ok(())
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let url = build_batch_videos_url(&input, &config.scrappa_api_base_url)?;
    println!("Fetching from: {url}");

    let videos = fetch_batch_videos(client, &url).await?;
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
                        response.status, reason, response.body.len(), response.body
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
    fn url_builder_normalizes_ids_and_rejects_invalid_counts() {
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let url = build_batch_videos_url(
            &serde_json::json!({"ids":" video-one, , video-two "}),
            &base,
        )
        .unwrap();
        assert_eq!(url.path(), "/videos/batch");
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "ids").unwrap().1,
            "video-one,video-two"
        );
        assert!(
            build_batch_videos_url(&serde_json::json!({"ids":" , , "}), &base)
                .unwrap_err()
                .to_string()
                .contains("at least one non-empty ID")
        );
        let too_many = (0..51)
            .map(|index| format!("id-{index}"))
            .collect::<Vec<_>>()
            .join(",");
        assert!(
            build_batch_videos_url(&serde_json::json!({"ids":too_many}), &base)
                .unwrap_err()
                .to_string()
                .contains("50 or fewer")
        );
    }

    #[test]
    fn qa_schema_prefill_survives_the_rust_url_builder() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let ids = schema["properties"]["ids"]["prefill"].as_str().unwrap();
        let url = build_batch_videos_url(
            &serde_json::json!({"ids":ids}),
            &Url::parse(SCRAPPA_API_BASE_URL).unwrap(),
        )
        .unwrap();
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "ids").unwrap().1,
            ids
        );
    }

    #[tokio::test]
    async fn local_apify_and_scrappa_mocks_preserve_rows_and_retry_504() {
        let videos = serde_json::json!([{"id":"7eul_Vt6SZY"},{"id":"6QQQKJJBJOY"}]);
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let ids = schema["properties"]["ids"]["prefill"].as_str().unwrap();
        let input = serde_json::json!({"ids":ids});
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(504, "{}"),
            response(200, &serde_json::json!({"videos":videos}).to_string()),
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
        assert!(path.starts_with("/videos/batch?ids="));
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
            .filter(|request| request.starts_with("GET /videos/"))
            .all(|request| !has_test_bearer_token(request)));
    }

    #[tokio::test]
    async fn retries_a_transient_408_before_saving_videos() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"video"}"#),
            response(408, "{}"),
            response(200, r#"{"videos":[{"id":"video"}]}"#),
            pricing_response(1.0, 0),
            response(201, "{}"),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert_eq!(request_parts(&requests[1]).1, request_parts(&requests[2]).1);
        assert_eq!(request_parts(&requests[4]).0, "POST");
    }

    #[tokio::test]
    async fn capped_run_stores_only_chargeable_items() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"video-one,video-two"}"#),
            response(200, r#"{"videos":[{"id":"video-one"},{"id":"video-two"}]}"#),
            pricing_response(0.0003, 0),
            response(201, "{}"),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
        assert!(has_test_bearer_token(&requests[2]));
        let (_, _, body) = request_parts(&requests[3]);
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::json!([{"id":"video-one"}])
        );
    }

    #[tokio::test]
    async fn exhausted_run_skips_dataset_write() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"video"}"#),
            response(200, r#"{"videos":[{"id":"video"}]}"#),
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
            response(200, r#"{"notVideos":[]}"#),
        ]);
        let error = run_actor(&client(), &config(&malformed.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("missing the videos array"));
        assert_eq!(malformed.requests().len(), 2);
    }

    #[tokio::test]
    async fn transport_failure_retries_and_apify_errors_propagate() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"video"}"#),
            response(0, ""),
            response(200, r#"{"videos":[{"id":"video"}]}"#),
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
}
