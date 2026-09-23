use std::{env, process::ExitCode, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const INPUT_KEY: &str = "INPUT";
const OUTPUT_KEY: &str = "OUTPUT";

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    key_value_store_id: String,
    input_key: String,
    dataset_id: String,
    actor_run_id: String,
    apify_token: String,
    scrappa_api_key: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|key| !key.is_empty())
                .unwrap_or_else(|| INPUT_KEY.to_owned()),
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
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
    let raw_url = env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned());
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

fn build_scrappa_url(base_url: &Url, input_url: &str, input: &Value) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["linkedin", "post"])?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("url", input_url);
        if input.get("use_cache") == Some(&Value::Bool(true)) {
            query.append_pair("use_cache", "1");
        }
        if let Some(age) = input.get("maximum_cache_age") {
            if let Some(age) = age.as_i64().filter(|age| *age >= 1) {
                query.append_pair("maximum_cache_age", &age.to_string());
            } else {
                eprintln!(
                    "maximum_cache_age must be at least 1, got {age}. Using default cache behavior."
                );
            }
        }
    }
    Ok(url)
}

#[derive(Debug)]
struct ScrappaApiError {
    status: u16,
    message: String,
}

impl std::fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.message
        )
    }
}

impl std::error::Error for ScrappaApiError {}

fn is_scrappa_not_found(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| error.status == StatusCode::NOT_FOUND.as_u16())
}

fn scrappa_error_message(status: u16, body: &str) -> String {
    let Ok(error_data) = serde_json::from_str::<Value>(body) else {
        return if body.is_empty() {
            format!("HTTP {status}")
        } else {
            body.to_owned()
        };
    };

    let mut message = error_data
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {status}"));
    if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .map(|(field, messages)| {
                let messages = messages
                    .as_array()
                    .map(|messages| {
                        messages
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                format!("{field}: {messages}")
            })
            .collect::<Vec<_>>()
            .join("; ");
        message.push_str(" - ");
        message.push_str(&details);
    }
    message
}

async fn get_input(client: &Client, config: &ActorConfig) -> Result<Option<Value>> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .context("Apify INPUT request failed")?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    require_apify_success(response, "INPUT request")
        .await?
        .json::<Value>()
        .await
        .context("Apify INPUT record was not valid JSON")
        .map(Some)
}

async fn fetch_scrappa_post(client: &Client, config: &ActorConfig, url: Url) -> Result<Value> {
    let response = client
        .get(url)
        .header("X-API-Key", &config.scrappa_api_key)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .context("Scrappa API request failed")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::Error::new(ScrappaApiError {
            status: status.as_u16(),
            message: scrappa_error_message(status.as_u16(), &body),
        }));
    }
    response
        .json::<Value>()
        .await
        .context("Scrappa API returned invalid JSON")
}

async fn push_dataset_item(client: &Client, config: &ActorConfig, item: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(item)
        .send()
        .await
        .context("Apify dataset write failed")?;
    require_apify_success(response, "dataset write").await?;
    Ok(())
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
    let run = require_apify_success(response, "run pricing request")
        .await?
        .json::<Value>()
        .await
        .context("Apify run pricing request returned invalid JSON")?;
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

async fn push_dataset_items(client: &Client, config: &ActorConfig, items: &[Value]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let capacity = run_dataset_capacity(client, config, items.len()).await?;
    let mut saved_rows = 0;
    for item in items {
        if saved_rows >= capacity {
            break;
        }
        push_dataset_item(client, config, item).await?;
        saved_rows += 1;
    }
    Ok(())
}

async fn put_output(client: &Client, config: &ActorConfig, output: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            OUTPUT_KEY,
        ],
    )?;
    let response = client
        .put(url)
        .bearer_auth(&config.apify_token)
        .json(output)
        .send()
        .await
        .context("Apify OUTPUT write failed")?;
    require_apify_success(response, "OUTPUT write").await?;
    Ok(())
}

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    bail!(
        "Apify {operation} failed with {} {reason}{detail}",
        status.as_u16()
    );
}

fn flattened_dataset_item(response: &Value) -> Value {
    let mut item = Map::with_capacity(17);
    for field in [
        "title",
        "url",
        "date_published",
        "date_modified",
        "image",
        "body",
    ] {
        if let Some(value) = response.get(field) {
            item.insert(field.to_owned(), value.clone());
        }
    }
    for (source, destination) in [
        ("name", "author_name"),
        ("url", "author_url"),
        ("image", "author_image"),
        ("headline", "author_headline"),
    ] {
        if let Some(value) = response
            .get("author")
            .and_then(Value::as_object)
            .and_then(|author| author.get(source))
        {
            item.insert(destination.to_owned(), value.clone());
        }
    }
    for (source, destination) in [
        ("total", "reactions_total"),
        ("likes", "reactions_likes"),
        ("comments", "comments_count"),
    ] {
        if let Some(value) = response
            .get("reactions")
            .and_then(Value::as_object)
            .and_then(|reactions| reactions.get(source))
        {
            item.insert(destination.to_owned(), value.clone());
        }
    }
    for field in ["topics", "more_articles", "comments", "success"] {
        if let Some(value) = response.get(field) {
            item.insert(field.to_owned(), value.clone());
        }
    }
    Value::Object(item)
}

fn flattened_dataset_items(response: &Value) -> Vec<Value> {
    match response {
        Value::Array(items) => items.iter().map(flattened_dataset_item).collect(),
        item => vec![flattened_dataset_item(item)],
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

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config)
        .await?
        .ok_or_else(|| anyhow!("LinkedIn post URL is required"))?;
    let input_url = input
        .get("url")
        .and_then(Value::as_str)
        .filter(|url| !url.is_empty())
        .ok_or_else(|| anyhow!("LinkedIn post URL is required"))?;

    println!("Scraping LinkedIn post: \"{input_url}\"");
    let request_url = build_scrappa_url(&config.scrappa_api_base_url, input_url, &input)?;
    let response = match fetch_scrappa_post(client, config, request_url).await {
        Ok(response) => response,
        Err(error) if is_scrappa_not_found(&error) => {
            eprintln!("404 Not Found for URL: {input_url}");
            let failure = json!({ "success": false, "error": error.to_string(), "url": input_url });
            push_dataset_items(client, config, &[failure]).await?;
            return Ok(());
        }
        Err(error) => return Err(error),
    };

    if response.is_null() {
        bail!("Scrappa API returned a null response");
    }
    if !response.is_array() && !response.get("success").is_some_and(js_truthy) {
        eprintln!("API returned success: false for URL: {input_url}");
    }
    let items = flattened_dataset_items(&response);
    push_dataset_items(client, config, &items).await?;
    put_output(client, config, &response).await?;
    println!(
        "Successfully scraped LinkedIn post: \"{}\"",
        response
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Unknown")
    );
    println!("LinkedIn Post scraping completed successfully");
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let result = async {
        let config = ActorConfig::from_env()?;
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("Could not create Actor HTTP client")?;
        run_actor(&client, &config).await
    }
    .await;
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            ExitCode::FAILURE
        }
    }
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
        time::{Duration, Instant},
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
                        404 => "Not Found",
                        422 => "Unprocessable Entity",
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
                base_url: Url::parse(&format!("http://{address}/")).unwrap(),
                requests,
                stop,
                thread: Some(thread),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.try_iter().collect()
        }

        fn scrappa_url(&self) -> Url {
            self.base_url.join("api").unwrap()
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
            key_value_store_id: "test-store".to_owned(),
            input_key: INPUT_KEY.to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            apify_token: "test-apify-token".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    fn pricing_response(max_charge: f64, charged_events: Value) -> MockResponse {
        response(
            200,
            &json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            "apify-actor-start": {"eventPriceUsd": 0.00005}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_charge},
                    "chargedEventCounts": charged_events
                }
            })
            .to_string(),
        )
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

    #[tokio::test]
    async fn reads_the_configured_input_record_key() {
        let server = MockServer::start(vec![response(200, r#"{"url":"post"}"#)]);
        let mut config = test_config(&server);
        config.input_key = "CUSTOM_INPUT".to_owned();
        assert!(get_input(&Client::new(), &config).await.unwrap().is_some());
        let requests = server.requests();
        assert_eq!(
            request_target(&requests[0]),
            "/v2/key-value-stores/test-store/records/CUSTOM_INPUT"
        );
    }

    #[test]
    fn input_schema_preserves_required_url_and_cache_defaults() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["required"], json!(["url"]));
        assert_eq!(schema["properties"]["use_cache"]["default"], true);
        assert_eq!(schema["properties"]["maximum_cache_age"]["minimum"], 1);
        assert_eq!(schema["properties"]["maximum_cache_age"]["default"], 3600);
    }

    #[test]
    fn query_keeps_original_cache_parameter_rules() {
        let base = Url::parse("https://scrappa.co/api").unwrap();
        let url = build_scrappa_url(
            &base,
            "https://www.linkedin.com/posts/example-activity-123",
            &json!({"use_cache": false, "maximum_cache_age": 1}),
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://scrappa.co/api/linkedin/post?url=https%3A%2F%2Fwww.linkedin.com%2Fposts%2Fexample-activity-123&maximum_cache_age=1"
        );
    }

    #[test]
    fn charged_events_reduce_available_dataset_capacity() {
        let run = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                "apify-actor-start": {"eventPriceUsd": 0.00005}
            }}},
            "options": {"maxTotalChargeUsd": 0.0006},
            "chargedEventCounts": {"apify-default-dataset-item": 1, "apify-actor-start": 1}
        }});
        assert_eq!(affordable_dataset_items(&run, 1).unwrap(), 0);
        let mut missing_limit = run.clone();
        missing_limit["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
        assert!(affordable_dataset_items(&missing_limit, 1).is_err());
        assert!(affordable_dataset_items(
            &json!({"data": {"pricingInfo": {"pricingModel": "PAY_PER_EVENT"}}}),
            1
        )
        .is_err());
    }

    #[tokio::test]
    async fn success_flattens_one_dataset_row_and_stores_the_raw_response() {
        let input = r#"{"url":"https://www.linkedin.com/posts/example-activity-123","use_cache":true,"maximum_cache_age":3600}"#;
        let raw = r#"{"success":true,"title":"Article","url":"https://www.linkedin.com/posts/example-activity-123","date_published":"2025-01-02","date_modified":"2025-01-03","image":"https://image.test/post.png","body":"Full post body","author":{"name":"Ada","url":"https://www.linkedin.com/in/ada","image":"https://image.test/ada.png","headline":"Engineer"},"reactions":{"total":12,"likes":9,"comments":3},"topics":["Rust"],"more_articles":[{"title":"Related"}],"comments":[{"text":"Nice"}],"extra":"kept only in OUTPUT"}"#;
        let server = MockServer::start(vec![
            response(200, input),
            response(200, raw),
            pricing_response(1.0, json!({})),
            response(201, ""),
            response(201, ""),
        ]);
        run_actor(&Client::new(), &test_config(&server))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert!(request_target(&requests[0])
            .starts_with("/v2/key-value-stores/test-store/records/INPUT"));
        assert_eq!(
            header_value(&requests[0], "authorization"),
            Some("Bearer test-apify-token")
        );
        assert_eq!(
            header_value(&requests[1], "x-api-key"),
            Some("test-scrappa-key")
        );
        assert!(request_target(&requests[1]).contains("/api/linkedin/post?url=https%3A%2F%2Fwww.linkedin.com%2Fposts%2Fexample-activity-123&use_cache=1&maximum_cache_age=3600"));
        assert_eq!(request_target(&requests[2]), "/v2/actor-runs/test-run");
        assert_eq!(
            header_value(&requests[2], "authorization"),
            Some("Bearer test-apify-token")
        );
        assert!(request_target(&requests[3]).starts_with("/v2/datasets/test-dataset/items"));
        assert_eq!(
            header_value(&requests[3], "authorization"),
            Some("Bearer test-apify-token")
        );
        assert_eq!(
            request_body(&requests[3]),
            json!({
                "title": "Article",
                "url": "https://www.linkedin.com/posts/example-activity-123",
                "date_published": "2025-01-02",
                "date_modified": "2025-01-03",
                "image": "https://image.test/post.png",
                "body": "Full post body",
                "author_name": "Ada",
                "author_url": "https://www.linkedin.com/in/ada",
                "author_image": "https://image.test/ada.png",
                "author_headline": "Engineer",
                "reactions_total": 12,
                "reactions_likes": 9,
                "comments_count": 3,
                "topics": ["Rust"],
                "more_articles": [{"title": "Related"}],
                "comments": [{"text": "Nice"}],
                "success": true
            })
        );
        assert!(request_target(&requests[4])
            .starts_with("/v2/key-value-stores/test-store/records/OUTPUT"));
        assert_eq!(
            header_value(&requests[4], "authorization"),
            Some("Bearer test-apify-token")
        );
        assert_eq!(
            request_body(&requests[4]),
            serde_json::from_str::<Value>(raw).unwrap()
        );
    }

    #[tokio::test]
    async fn scrappa_404_publishes_one_failure_row_without_failing_or_writing_output() {
        let input = r#"{"url":"https://www.linkedin.com/posts/missing-activity-123"}"#;
        let server = MockServer::start(vec![
            response(200, input),
            response(404, r#"{"message":"Post not found"}"#),
            pricing_response(0.0003, json!({})),
            response(201, ""),
        ]);
        run_actor(&Client::new(), &test_config(&server))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(request_target(&requests[3]).starts_with("/v2/datasets/test-dataset/items"));
        assert_eq!(
            request_body(&requests[3]),
            json!({
                "success": false,
                "error": "Scrappa API error (404): Post not found",
                "url": "https://www.linkedin.com/posts/missing-activity-123"
            })
        );
        assert!(requests
            .iter()
            .all(|request| !request_target(request).contains("/records/OUTPUT")));
    }

    #[tokio::test]
    async fn one_result_budget_posts_only_the_first_row_and_preserves_output() {
        let input = r#"{"url":"https://www.linkedin.com/posts/example-activity-123"}"#;
        let raw = r#"[{"title":"First","url":"post-1","success":true},{"title":"Second","url":"post-2","success":true}]"#;
        let server = MockServer::start(vec![
            response(200, input),
            response(200, raw),
            pricing_response(0.0003, json!({})),
            response(201, ""),
            response(201, ""),
        ]);
        run_actor(&Client::new(), &test_config(&server))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert_eq!(request_target(&requests[2]), "/v2/actor-runs/test-run");
        assert!(request_target(&requests[3]).starts_with("/v2/datasets/test-dataset/items"));
        assert_eq!(
            request_body(&requests[3]),
            json!({"title":"First","url":"post-1","success":true})
        );
        assert!(request_target(&requests[4]).ends_with("/records/OUTPUT"));
        assert_eq!(
            request_body(&requests[4]),
            serde_json::from_str::<Value>(raw).unwrap()
        );
    }

    #[tokio::test]
    async fn zero_budget_skips_dataset_rows_but_keeps_output() {
        let input = r#"{"url":"https://www.linkedin.com/posts/example-activity-123"}"#;
        let raw = r#"[{"title":"First"},{"title":"Second"}]"#;
        let server = MockServer::start(vec![
            response(200, input),
            response(200, raw),
            pricing_response(0.0, json!({})),
            response(201, ""),
        ]);
        run_actor(&Client::new(), &test_config(&server))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests
            .iter()
            .all(|request| !request_target(request).contains("/datasets/")));
        assert!(request_target(&requests[3]).ends_with("/records/OUTPUT"));
        assert_eq!(
            request_body(&requests[3]),
            serde_json::from_str::<Value>(raw).unwrap()
        );
    }

    #[tokio::test]
    async fn normal_budget_posts_all_rows_in_order() {
        let input = r#"{"url":"https://www.linkedin.com/posts/example-activity-123"}"#;
        let raw = r#"[{"title":"First"},{"title":"Second"}]"#;
        let server = MockServer::start(vec![
            response(200, input),
            response(200, raw),
            pricing_response(1.0, json!({})),
            response(201, ""),
            response(201, ""),
            response(201, ""),
        ]);
        run_actor(&Client::new(), &test_config(&server))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 6);
        assert_eq!(request_body(&requests[3]), json!({"title":"First"}));
        assert_eq!(request_body(&requests[4]), json!({"title":"Second"}));
        assert!(request_target(&requests[5]).ends_with("/records/OUTPUT"));
        assert_eq!(
            request_body(&requests[5]),
            serde_json::from_str::<Value>(raw).unwrap()
        );
    }

    #[tokio::test]
    async fn missing_run_pricing_fails_before_any_dataset_or_output_write() {
        let input = r#"{"url":"https://www.linkedin.com/posts/example-activity-123"}"#;
        let raw = r#"{"title":"Article","success":true}"#;
        let server = MockServer::start(vec![
            response(200, input),
            response(200, raw),
            response(
                200,
                r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{}}},"options":{"maxTotalChargeUsd":1},"chargedEventCounts":{}}}"#,
            ),
        ]);
        let error = run_actor(&Client::new(), &test_config(&server))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("dataset item price"));
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request_target(request).contains("/datasets/")
                && !request_target(request).contains("/records/OUTPUT")));
    }

    #[tokio::test]
    async fn dataset_write_error_fails_without_writing_output() {
        let input = r#"{"url":"https://www.linkedin.com/posts/example-activity-123"}"#;
        let server = MockServer::start(vec![
            response(200, input),
            response(200, r#"{"title":"Article","success":true}"#),
            pricing_response(1.0, json!({})),
            response(500, "dataset unavailable"),
        ]);
        let error = run_actor(&Client::new(), &test_config(&server))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("dataset write failed"));
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests
            .iter()
            .all(|request| !request_target(request).contains("/records/OUTPUT")));
    }

    #[tokio::test]
    async fn non_404_scrappa_errors_fail_without_dataset_or_output_writes() {
        let input = r#"{"url":"https://www.linkedin.com/posts/example-activity-123"}"#;
        let server = MockServer::start(vec![response(200, input), response(401, "Not authorized")]);
        let error = run_actor(&Client::new(), &test_config(&server))
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), "Scrappa API error (401): Not authorized");
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert!(requests
            .iter()
            .all(|request| !request_target(request).contains("/datasets/")));
        assert!(requests
            .iter()
            .all(|request| !request_target(request).contains("/records/OUTPUT")));
    }
}
