use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
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
    fn from_env(scrappa_api_key: String) -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
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

fn scrappa_api_key(value: Option<&str>) -> Result<String> {
    value
        .filter(|key| !key.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments.iter().copied());
    Ok(url)
}

fn single_input_string(value: &Value) -> Option<&str> {
    let value = value
        .as_array()
        .and_then(|values| values.first())
        .unwrap_or(value);
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn build_video_comments_url(input: &Value, api_base_url: &Url) -> Result<Url> {
    let id = input
        .get("id")
        .and_then(single_input_string)
        .ok_or_else(|| anyhow!("YouTube video ID \"id\" is required."))?;
    let sort = input.get("sort").and_then(single_input_string);
    let continuation = input.get("continuation").and_then(single_input_string);

    let mut url = endpoint_url(api_base_url, &["youtube", "comments"])?;
    url.set_query(None);
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("video_id", id);
        if let Some(sort) = sort {
            query.append_pair("sort", sort);
        }
        if let Some(continuation) = continuation {
            query.append_pair("continuation", continuation);
        }
    }
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

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn continuation_log_message(data: &Value) -> Option<String> {
    data.get("continuation")
        .filter(|continuation| js_truthy(continuation))
        .map(|continuation| {
            format!(
                "Continuation token available for next page: {}",
                js_string(continuation)
            )
        })
}

fn comments_from_response(data: &Value) -> Option<&Value> {
    data.get("comments").filter(|comments| !comments.is_null())
}

fn comment_count_label(comments: &Value) -> String {
    match comments {
        Value::Array(comments) => comments.len().to_string(),
        Value::String(comments) => comments.encode_utf16().count().to_string(),
        Value::Object(comments) => comments
            .get("length")
            .map(js_string)
            .unwrap_or_else(|| "undefined".to_owned()),
        _ => "undefined".to_owned(),
    }
}

async fn fetch_video_comments(
    client: &Client,
    config: &ActorConfig,
    input: &Value,
) -> Result<Value> {
    let url = build_video_comments_url(input, &config.scrappa_api_base_url)?;
    println!("Fetching from: {url}");
    let response = client
        .get(url)
        .header("X-API-Key", &config.scrappa_api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "Scrappa API request timed out after {}s",
                    SCRAPPA_REQUEST_TIMEOUT.as_secs()
                )
            } else {
                anyhow!(error.to_string())
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

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    items: Option<&Value>,
) -> Result<()> {
    let Some(items) = items else {
        return Ok(());
    };
    let requested = items.as_array().map_or(1, Vec::len);
    if requested == 0 {
        return Ok(());
    }
    let limit = run_dataset_capacity(client, config, requested).await?;
    if limit == 0 {
        return Ok(());
    }
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let request = client.post(url).bearer_auth(&config.apify_token);
    let response = match items.as_array() {
        Some(rows) => request.json(&rows[..limit]).send().await,
        None => request.json(items).send().await,
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
    Ok(())
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let data = fetch_video_comments(client, config, &input).await?;
    let comments = comments_from_response(&data);
    push_dataset_items(client, config, comments).await?;

    let count = comments
        .map(comment_count_label)
        .unwrap_or_else(|| "0".to_owned());
    let video_id = input
        .get("id")
        .map(js_string)
        .unwrap_or_else(|| "undefined".to_owned());
    println!("Successfully fetched {count} comment(s) for video id: {video_id}");
    if let Some(message) = continuation_log_message(&data) {
        println!("{message}");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube video comments: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let api_key = scrappa_api_key(env::var("SCRAPPA_API_KEY").ok().as_deref())?;
    let config = ActorConfig::from_env(api_key)?;
    let client = Client::builder()
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
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
            mpsc::{channel, Receiver},
            Arc,
        },
        thread::{self, JoinHandle},
        time::Instant,
    };

    struct MockResponse {
        status: u16,
        body: String,
        delay: Duration,
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
            let (recorded_requests, requests) = channel();
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
                    let _ = recorded_requests.send(request);
                    let Some(response) = responses.next() else {
                        break;
                    };
                    if !response.delay.is_zero() {
                        thread::sleep(response.delay);
                    }
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        429 => "Too Many Requests",
                        503 => "Service Unavailable",
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
            delay: Duration::ZERO,
        }
    }

    fn delayed_response(status: u16, body: &str, delay: Duration) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
            delay,
        }
    }

    fn pricing_response(max_charge: f64, charged_items: u64) -> MockResponse {
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
                    "chargedEventCounts": {"apify-default-dataset-item": charged_items}
                }
            })
            .to_string(),
        )
    }

    fn config(server: &MockServer) -> ActorConfig {
        let mut scrappa_api_base_url = server.base_url.clone();
        scrappa_api_base_url.set_path("/api");
        ActorConfig {
            apify_api_base_url: server.base_url.clone(),
            scrappa_api_base_url,
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token".to_owned(),
            scrappa_api_key: "test-key".to_owned(),
        }
    }

    fn client(timeout: Duration) -> Client {
        Client::builder().timeout(timeout).build().unwrap()
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

    fn has_header(request: &str, header_name: &str, expected_value: &str) -> bool {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .skip(1)
            .any(|line| {
                let Some((name, value)) = line.split_once(':') else {
                    return false;
                };
                name.eq_ignore_ascii_case(header_name) && value.trim() == expected_value
            })
    }

    fn request_url(target: &str) -> Url {
        Url::parse(&format!("http://mock{target}")).unwrap()
    }

    #[test]
    fn api_key_is_required_but_whitespace_is_not_trimmed() {
        assert_eq!(scrappa_api_key(Some(" test-key ")).unwrap(), " test-key ");
        assert!(scrappa_api_key(Some(""))
            .unwrap_err()
            .to_string()
            .contains("SCRAPPA_API_KEY"));
        assert!(scrappa_api_key(None)
            .unwrap_err()
            .to_string()
            .contains("SCRAPPA_API_KEY"));
    }

    #[test]
    fn schema_prefill_and_input_boundaries_build_expected_query() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let id = schema["properties"]["id"]["prefill"].as_str().unwrap();
        let sort = schema["properties"]["sort"]["prefill"].as_array().unwrap();
        let url = build_video_comments_url(
            &json!({"id": id, "sort": sort}),
            &Url::parse(SCRAPPA_API_BASE_URL).unwrap(),
        )
        .unwrap();
        assert_eq!(url.path(), "/api/youtube/comments");
        assert_eq!(
            url.query_pairs()
                .find(|(key, _)| key == "video_id")
                .unwrap()
                .1,
            id
        );
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "sort").unwrap().1,
            "TOP_COMMENTS"
        );
        assert!(url.query_pairs().all(|(key, _)| key != "id"));

        let url = build_video_comments_url(
            &json!({
                "id": [" video id ", "ignored"],
                "sort": [" NEWEST_FIRST ", "TOP_COMMENTS"],
                "continuation": " next token/? "
            }),
            &Url::parse(SCRAPPA_API_BASE_URL).unwrap(),
        )
        .unwrap();
        let query = url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect::<Vec<_>>();
        assert_eq!(
            query,
            vec![
                ("video_id".to_owned(), "video id".to_owned()),
                ("sort".to_owned(), "NEWEST_FIRST".to_owned()),
                ("continuation".to_owned(), "next token/?".to_owned()),
            ]
        );

        for input in [
            json!({}),
            json!({"id":"  "}),
            json!({"id":[]}),
            json!({"id":42}),
        ] {
            assert!(
                build_video_comments_url(&input, &Url::parse(SCRAPPA_API_BASE_URL).unwrap())
                    .unwrap_err()
                    .to_string()
                    .contains("YouTube video ID \"id\" is required")
            );
        }
    }

    #[test]
    fn response_shape_defaults_only_missing_or_null_comments_and_logs_continuation() {
        assert_eq!(comments_from_response(&json!({})), None);
        assert_eq!(comments_from_response(&json!({"comments":null})), None);
        let object = json!({"comments":{"id":"one"}});
        assert_eq!(comments_from_response(&object), object.get("comments"));
        let array = json!({"comments":[{"id":"one"}]});
        assert_eq!(comments_from_response(&array), array.get("comments"));
        assert_eq!(
            continuation_log_message(&json!({"continuation":"next-page"})).as_deref(),
            Some("Continuation token available for next page: next-page")
        );
        assert_eq!(continuation_log_message(&json!({"continuation":""})), None);
    }

    #[tokio::test]
    async fn local_apify_and_scrappa_flow_preserves_comments_and_auth() {
        let input = json!({
            "id": " dQw4w9WgXcQ ",
            "sort": ["NEWEST_FIRST"],
            "continuation": "previous-page"
        });
        let comments = json!([
            {"id":"comment-1","text":"First","author":{"id":"author-1"},"replies":[{"id":"reply-1"}]},
            {"id":"comment-2","text":"Second","likeCount":3}
        ]);
        let data = json!({"comments":comments.clone(),"continuation":"next-page"});
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(200, &data.to_string()),
            pricing_response(0.0006, 0),
            response(201, "{}"),
        ]);
        run_actor(&client(SCRAPPA_REQUEST_TIMEOUT), &config(&server))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(request_parts(&requests[0]).0, "GET");
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert!(has_header(
            &requests[0],
            "Authorization",
            "Bearer test-token"
        ));

        let (method, target, _) = request_parts(&requests[1]);
        assert_eq!(method, "GET");
        let url = request_url(target);
        assert_eq!(url.path(), "/api/youtube/comments");
        let query = url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect::<Vec<_>>();
        assert_eq!(
            query,
            vec![
                ("video_id".to_owned(), "dQw4w9WgXcQ".to_owned()),
                ("sort".to_owned(), "NEWEST_FIRST".to_owned()),
                ("continuation".to_owned(), "previous-page".to_owned()),
            ]
        );
        assert!(has_header(&requests[1], "X-API-Key", "test-key"));
        assert!(has_header(&requests[1], "Accept", "application/json"));
        assert!(!has_header(
            &requests[1],
            "Authorization",
            "Bearer test-token"
        ));

        let (method, target, _) = request_parts(&requests[2]);
        assert_eq!(method, "GET");
        assert_eq!(target, "/v2/actor-runs/test-run");
        assert!(has_header(
            &requests[2],
            "Authorization",
            "Bearer test-token"
        ));

        let (method, target, body) = request_parts(&requests[3]);
        assert_eq!(method, "POST");
        assert_eq!(target, "/v2/datasets/test-dataset/items");
        assert!(has_header(
            &requests[3],
            "Authorization",
            "Bearer test-token"
        ));
        assert_eq!(serde_json::from_str::<Value>(body).unwrap(), comments);
    }

    #[tokio::test]
    async fn capped_comments_only_save_affordable_rows() {
        let input = json!({"id":"video"});
        let comments = json!([{"id":"first"},{"id":"second"}]);
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(200, &json!({"comments":comments}).to_string()),
            pricing_response(0.0003, 0),
            response(201, "{}"),
        ]);
        run_actor(&client(SCRAPPA_REQUEST_TIMEOUT), &config(&server))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
        assert!(has_header(
            &requests[2],
            "Authorization",
            "Bearer test-token"
        ));
        assert_eq!(request_parts(&requests[3]).0, "POST");
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[3]).2).unwrap(),
            json!([{"id":"first"}])
        );
    }

    #[tokio::test]
    async fn zero_budget_skips_dataset_post() {
        let input = json!({"id":"video"});
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(200, r#"{"comments":[{"id":"comment"}]}"#),
            pricing_response(0.0, 0),
        ]);
        run_actor(&client(SCRAPPA_REQUEST_TIMEOUT), &config(&server))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
    }

    #[tokio::test]
    async fn missing_run_pricing_fails_without_dataset_post() {
        let input = json!({"id":"video"});
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(200, r#"{"comments":[{"id":"comment"}]}"#),
            response(
                200,
                r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"}}}"#,
            ),
        ]);
        let error = run_actor(&client(SCRAPPA_REQUEST_TIMEOUT), &config(&server))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("did not provide event prices"));
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_parts(&requests[2]).0, "GET");
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
    }

    #[test]
    fn pricing_capacity_includes_all_charged_event_types() {
        let run = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                "apify-actor-start": {"eventPriceUsd": 0.00005}
            }}},
            "options": {"maxTotalChargeUsd": 0.00035},
            "chargedEventCounts": {"apify-default-dataset-item": 0, "apify-actor-start": 1}
        }});
        assert_eq!(affordable_dataset_items(&run, 2).unwrap(), 1);
    }

    #[tokio::test]
    async fn upstream_and_storage_errors_fail_without_retry_or_partial_output() {
        let input = json!({"id":"video"});
        let upstream_error = MockServer::start(vec![
            response(200, &input.to_string()),
            response(503, "unavailable"),
        ]);
        let error = run_actor(&client(SCRAPPA_REQUEST_TIMEOUT), &config(&upstream_error))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Scrappa API request failed with 503 Service Unavailable"));
        assert_eq!(upstream_error.requests().len(), 2);

        let storage_error = MockServer::start(vec![
            response(200, &input.to_string()),
            response(200, r#"{"comments":[{"id":"comment"}]}"#),
            pricing_response(0.0003, 0),
            response(401, "dataset denied"),
        ]);
        let error = run_actor(&client(SCRAPPA_REQUEST_TIMEOUT), &config(&storage_error))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Apify dataset write failed with 401 Unauthorized"));
        let requests = storage_error.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(request_parts(&requests[3]).0, "POST");
    }

    #[tokio::test]
    async fn scrappa_request_timeout_uses_original_timeout_message() {
        let server = MockServer::start(vec![delayed_response(
            200,
            r#"{"comments":[]}"#,
            Duration::from_millis(100),
        )]);
        let error = fetch_video_comments(
            &client(Duration::from_millis(20)),
            &config(&server),
            &json!({"id":"video"}),
        )
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "Scrappa API request timed out after 60s");
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn empty_comment_array_does_not_create_dataset_rows() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"video"}"#),
            response(200, r#"{"comments":[]}"#),
        ]);
        run_actor(&client(SCRAPPA_REQUEST_TIMEOUT), &config(&server))
            .await
            .unwrap();
        assert_eq!(server.requests().len(), 2);
    }
}
