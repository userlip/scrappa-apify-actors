use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{env, fmt::Write as _, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://ytapi.scrappa.co";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
}

struct DatasetBudget {
    affordable_rows: usize,
    spent_rows: usize,
}

impl DatasetBudget {
    fn remaining_rows(&self) -> usize {
        self.affordable_rows.saturating_sub(self.spent_rows)
    }
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

fn encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
        {
            encoded.push(byte as char);
        } else {
            write!(encoded, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    encoded
}

fn build_related_videos_url(input: &Value, api_base_url: &Url) -> Result<Url> {
    let id = input
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| {
            anyhow!("YouTube video ID \"id\" not provided. Please provide a value for \"id\" in the input.")
        })?;
    let continuation = input
        .get("continuation")
        .and_then(Value::as_str)
        .filter(|continuation| !continuation.trim().is_empty());

    let mut url = endpoint_url(api_base_url, &["videos", "related"])?;
    let mut query = format!("id={}", encode_component(id));
    if let Some(continuation) = continuation {
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

async fn fetch_related_videos(client: &Client, url: &Url) -> Result<Value> {
    let response = client.get(url.clone()).send().await.map_err(|error| {
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
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        bail!(
            "Scrappa API request failed with {} {reason}",
            status.as_u16()
        );
    }
    response.json().await.map_err(|error| {
        if error.is_timeout() {
            anyhow!(
                "Scrappa API request timed out after {}s",
                REQUEST_TIMEOUT.as_secs()
            )
        } else {
            anyhow!("Scrappa API response was not valid JSON: {error}")
        }
    })
}

fn related_videos(response_data: &Value) -> Value {
    response_data
        .get("videos")
        .filter(|videos| !videos.is_null())
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()))
}

fn dataset_row_count(videos: &Value) -> usize {
    videos
        .as_array()
        .map_or_else(|| if videos.is_null() { 0 } else { 1 }, Vec::len)
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
    let run = response_json(response, "Apify run pricing request").await?;
    Ok(DatasetBudget {
        affordable_rows: affordable_dataset_items(&run, usize::MAX)?,
        spent_rows: 0,
    })
}

// maxItems limits pay-per-result actors; PPE dataset rows use the run's event budget.
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
    let limit = max_charge + tolerance;
    let mut low = 0;
    let mut high = requested;
    while low < high {
        let distance = high - low;
        let middle = low + distance / 2 + distance % 2;
        if spent + middle as f64 * item_price <= limit {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    Ok(low)
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    videos: &Value,
    budget: &mut Option<DatasetBudget>,
) -> Result<usize> {
    let row_count = dataset_row_count(videos);
    if row_count == 0 {
        return Ok(0);
    }
    if budget.is_none() {
        *budget = Some(run_dataset_budget(client, config).await?);
    }
    let rows_to_save = row_count.min(budget.as_ref().unwrap().remaining_rows());
    if rows_to_save == 0 {
        return Ok(0);
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let request = client.post(url).bearer_auth(&config.apify_token);
    let response = if let Some(items) = videos.as_array() {
        request.json(&items[..rows_to_save]).send().await
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
    budget.as_mut().unwrap().spent_rows += rows_to_save;
    Ok(rows_to_save)
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let url = build_related_videos_url(&input, &config.scrappa_api_base_url)?;
    println!("Fetching from: {url}");

    let response_data = fetch_related_videos(client, &url).await?;
    let videos = related_videos(&response_data);
    let mut budget = None;
    push_dataset_items(client, config, &videos, &mut budget).await?;
    let video_id = input.get("id").and_then(Value::as_str).unwrap_or_default();
    let count = videos
        .as_array()
        .map(Vec::len)
        .or_else(|| videos.is_object().then_some(1))
        .unwrap_or(0);
    println!("Successfully fetched {count} related videos for video id: {video_id}");

    if let Some(continuation) = response_data
        .get("continuation")
        .and_then(Value::as_str)
        .filter(|continuation| !continuation.is_empty())
    {
        println!("Continuation token available for next page: {continuation}");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube related videos: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("Could not create HTTP client")?;
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
            let (request_sender, requests) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                for response in responses {
                    let deadline = Instant::now() + Duration::from_secs(5);
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
    fn pricing_response(max_charge: f64, charged_rows: u64) -> MockResponse {
        response(
            200,
            &json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_charge},
                    "chargedEventCounts": {"apify-default-dataset-item": charged_rows}
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
    fn url_builder_encodes_components_and_preserves_continuation_rules() {
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let url = build_related_videos_url(
            &json!({"id":"video id!*'()/é", "continuation":"next page/token"}),
            &base,
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://ytapi.scrappa.co/videos/related?id=video%20id!*%27()%2F%C3%A9&continuation=next%20page%2Ftoken"
        );

        let url =
            build_related_videos_url(&json!({"id":"dQw4w9WgXcQ", "continuation":"  "}), &base)
                .unwrap();
        assert_eq!(
            url.as_str(),
            "https://ytapi.scrappa.co/videos/related?id=dQw4w9WgXcQ"
        );
    }

    #[test]
    fn url_builder_requires_a_nonempty_video_id() {
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let error = build_related_videos_url(&json!({}), &base).unwrap_err();
        assert_eq!(
            error.to_string(),
            "YouTube video ID \"id\" not provided. Please provide a value for \"id\" in the input."
        );
        assert!(build_related_videos_url(&json!({"id":""}), &base).is_err());
    }

    #[test]
    fn response_selection_preserves_arrays_and_objects_and_defaults_nullish_values() {
        assert_eq!(related_videos(&json!({})), json!([]));
        assert_eq!(related_videos(&json!({"videos":null})), json!([]));
        assert_eq!(
            related_videos(&json!({"videos":[{"videoId":"one"},{"videoId":"two"}]})),
            json!([{"videoId":"one"},{"videoId":"two"}])
        );
        assert_eq!(
            related_videos(&json!({"videos":{"videoId":"one"}})),
            json!({"videoId":"one"})
        );
    }

    #[tokio::test]
    async fn actor_fetches_related_videos_and_posts_each_as_dataset_rows() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"video id","continuation":"next page/token"}"#),
            response(
                200,
                r#"{"videos":[{"videoId":"one"},{"videoId":"two"}],"continuation":"later"}"#,
            ),
            pricing_response(1.0, 0),
            response(201, ""),
        ]);
        let config = config(&server.base_url);
        run_actor(&client(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        let (method, path, _) = request_parts(&requests[0]);
        assert_eq!(method, "GET");
        assert_eq!(path, "/v2/key-value-stores/test-store/records/INPUT");
        assert!(has_test_bearer_token(&requests[0]));

        let (method, path, _) = request_parts(&requests[1]);
        assert_eq!(method, "GET");
        assert_eq!(
            path,
            "/videos/related?id=video%20id&continuation=next%20page%2Ftoken"
        );
        assert!(!has_test_bearer_token(&requests[1]));

        let (method, path, _) = request_parts(&requests[2]);
        assert_eq!(method, "GET");
        assert_eq!(path, "/v2/actor-runs/test-run");
        assert!(has_test_bearer_token(&requests[2]));

        let (method, path, body) = request_parts(&requests[3]);
        assert_eq!(method, "POST");
        assert_eq!(path, "/v2/datasets/test-dataset/items");
        assert!(has_test_bearer_token(&requests[3]));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{"videoId":"one"},{"videoId":"two"}])
        );
    }

    #[tokio::test]
    async fn empty_related_videos_do_not_write_a_dataset_item() {
        let server = MockServer::start(Vec::new());
        let config = config(&server.base_url);
        let mut budget = None;
        push_dataset_items(&client(), &config, &json!([]), &mut budget)
            .await
            .unwrap();
        assert!(server.requests().is_empty());
    }

    #[tokio::test]
    async fn object_related_videos_are_posted_as_one_dataset_item() {
        let server = MockServer::start(vec![pricing_response(1.0, 0), response(201, "")]);
        let config = config(&server.base_url);
        let video = json!({"videoId":"single"});
        let mut budget = None;
        push_dataset_items(&client(), &config, &video, &mut budget)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        let (method, path, _) = request_parts(&requests[0]);
        assert_eq!(method, "GET");
        assert_eq!(path, "/v2/actor-runs/test-run");
        let (method, path, body) = request_parts(&requests[1]);
        assert_eq!(method, "POST");
        assert_eq!(path, "/v2/datasets/test-dataset/items");
        assert_eq!(serde_json::from_str::<Value>(body).unwrap(), video);
    }

    #[tokio::test]
    async fn one_row_budget_posts_only_the_first_video() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"video"}"#),
            response(200, r#"{"videos":[{"videoId":"one"},{"videoId":"two"}]}"#),
            pricing_response(0.0003, 0),
            response(201, ""),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        let (method, path, body) = request_parts(&requests[3]);
        assert_eq!(method, "POST");
        assert_eq!(path, "/v2/datasets/test-dataset/items");
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{"videoId":"one"}])
        );
    }

    #[tokio::test]
    async fn zero_row_budget_skips_the_dataset_post() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"video"}"#),
            response(200, r#"{"videos":[{"videoId":"one"},{"videoId":"two"}]}"#),
            pricing_response(0.0, 0),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| request_parts(request).0 != "POST"));
    }

    #[test]
    fn charged_counts_from_every_priced_event_reduce_dataset_capacity() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                        "other-event": {"eventPriceUsd": 0.0005}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.0013},
                "chargedEventCounts": {
                    "apify-default-dataset-item": 0,
                    "other-event": 2
                }
            }
        });
        assert_eq!(affordable_dataset_items(&run, 10).unwrap(), 1);
    }

    #[test]
    fn missing_charged_event_counts_do_not_authorize_related_rows() {
        let mut run = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
            }}},
            "options": {"maxTotalChargeUsd": 1.0}
        }});
        assert!(affordable_dataset_items(&run, 1).is_err());
        run["data"]["chargedEventCounts"] = json!({});
        assert_eq!(affordable_dataset_items(&run, 1).unwrap(), 1);
    }

    #[tokio::test]
    async fn pricing_lookup_error_fails_without_posting_rows() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"video"}"#),
            response(200, r#"{"videos":[{"videoId":"one"}]}"#),
            response(500, "pricing unavailable"),
        ]);
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Apify run pricing request failed"));

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_parts(&requests[2]).0, "GET");
    }

    #[tokio::test]
    async fn missing_numeric_spending_limit_fails_without_posting_rows() {
        let missing_limit = response(
            200,
            r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0003}}}},"options":{"maxTotalChargeUsd":null},"chargedEventCounts":{}}}"#,
        );
        let server = MockServer::start(vec![
            response(200, r#"{"id":"video"}"#),
            response(200, r#"{"videos":[{"videoId":"one"}]}"#),
            missing_limit,
        ]);
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("spending limit"));
        assert_eq!(server.requests().len(), 3);
    }

    #[tokio::test]
    async fn storage_error_is_returned_after_the_single_post_attempt() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"video"}"#),
            response(200, r#"{"videos":[{"videoId":"one"},{"videoId":"two"}]}"#),
            pricing_response(1.0, 0),
            response(500, "dataset unavailable"),
        ]);
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("dataset write failed"));
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(request_parts(&requests[3]).0, "POST");
    }

    #[tokio::test]
    async fn successful_rows_are_tracked_across_multiple_dataset_posts() {
        let server = MockServer::start(vec![
            pricing_response(0.0006, 0),
            response(201, ""),
            response(201, ""),
        ]);
        let config = config(&server.base_url);
        let mut budget = None;
        assert_eq!(
            push_dataset_items(&client(), &config, &json!([{"videoId":"one"}]), &mut budget)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            push_dataset_items(
                &client(),
                &config,
                &json!([{"videoId":"two"},{"videoId":"three"}]),
                &mut budget
            )
            .await
            .unwrap(),
            1
        );

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_parts(&requests[0]).1, "/v2/actor-runs/test-run");
        assert!(has_test_bearer_token(&requests[0]));
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[1]).2).unwrap(),
            json!([{"videoId":"one"}])
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[2]).2).unwrap(),
            json!([{"videoId":"two"}])
        );
    }

    #[tokio::test]
    async fn scrappa_http_errors_fail_without_retrying() {
        let server = MockServer::start(vec![response(429, r#"{"error":"limited"}"#)]);
        let error = fetch_related_videos(&client(), &server.base_url)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API request failed with 429 Too Many Requests"
        );
        assert_eq!(server.requests().len(), 1);
    }
}
