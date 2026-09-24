use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Value};
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: usize = 8;
const APIFY_RETRY_BASE_DELAY: Duration = Duration::from_millis(500);
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    scrappa_api_key: String,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("SCRAPPA_API_KEY environment variable is not set."))?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            scrappa_api_key,
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

fn autocomplete_url(input: &Value, api_base_url: &Url) -> Result<(String, Url)> {
    let query = input
        .get("query")
        .and_then(Value::as_str)
        .filter(|query| !query.is_empty())
        .ok_or_else(|| anyhow!("Search query is required"))?
        .to_owned();
    let mut url = endpoint_url(api_base_url, &["maps", "autocomplete"])?;
    url.query_pairs_mut().append_pair("query", &query);
    Ok((query, url))
}

fn status_reason(status: StatusCode) -> String {
    status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()))
}

fn scrappa_error_message(status: StatusCode, body: &str) -> String {
    let fallback = status_reason(status);
    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(&fallback)
            .to_owned();
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    let messages = messages.as_array()?;
                    let messages = messages
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>();
                    (!messages.is_empty()).then(|| format!("{field}: {}", messages.join(", ")))
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details);
            }
        }
        return message;
    }

    if body.is_empty() {
        return fallback;
    }

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
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
            status.as_u16(),
        );
    }
    response
        .json()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

async fn send_apify_request(
    build_request: impl FnMut() -> RequestBuilder,
) -> std::result::Result<Response, reqwest::Error> {
    send_request_with_retries(
        build_request,
        APIFY_REQUEST_TIMEOUT,
        APIFY_MAX_RETRIES,
        APIFY_RETRY_BASE_DELAY,
    )
    .await
}

async fn send_request_with_retries(
    mut build_request: impl FnMut() -> RequestBuilder,
    timeout: Duration,
    max_retries: usize,
    initial_retry_delay: Duration,
) -> std::result::Result<Response, reqwest::Error> {
    let mut retries = 0;
    loop {
        let response = match build_request().timeout(timeout).send().await {
            Ok(response) => response,
            Err(_) if retries < max_retries => {
                tokio::time::sleep(retry_delay(initial_retry_delay, retries)).await;
                retries += 1;
                continue;
            }
            Err(error) => return Err(error),
        };

        if retries < max_retries
            && (response.status() == StatusCode::TOO_MANY_REQUESTS
                || response.status().is_server_error())
        {
            drop(response);
            tokio::time::sleep(retry_delay(initial_retry_delay, retries)).await;
            retries += 1;
            continue;
        }

        return Ok(response);
    }
}

fn retry_delay(initial_delay: Duration, retries: usize) -> Duration {
    initial_delay.saturating_mul(2_u32.saturating_pow(retries as u32))
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
    let response = send_apify_request(|| client.get(url.clone()).bearer_auth(&config.apify_token))
        .await
        .context("Apify INPUT request failed")?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(Value::Null);
    }
    response_json(response, "Apify INPUT request").await
}

async fn fetch_suggestions(client: &Client, config: &ActorConfig, url: &Url) -> Result<Value> {
    fetch_suggestions_with_timeout(client, config, url, SCRAPPA_REQUEST_TIMEOUT).await
}

async fn fetch_suggestions_with_timeout(
    client: &Client,
    config: &ActorConfig,
    url: &Url,
    timeout: Duration,
) -> Result<Value> {
    let response = client
        .get(url.clone())
        .header(reqwest::header::ACCEPT, "application/json")
        .header("X-API-Key", &config.scrappa_api_key)
        .timeout(timeout)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() || error.to_string().contains("aborted") {
                anyhow!(
                    "Scrappa API request timed out after {}ms",
                    timeout.as_millis()
                )
            } else {
                anyhow!("Scrappa API request failed: {error}")
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let message = scrappa_error_message(status, &body);
        bail!("Scrappa API error ({}): {message}", status.as_u16());
    }
    response
        .json()
        .await
        .context("Scrappa API response was not valid JSON")
}

struct DatasetBudget {
    dataset_item_price_usd: f64,
    charged_usd: f64,
    max_total_charge_usd: f64,
}

impl DatasetBudget {
    fn from_run(run: &Value) -> Result<Option<Self>> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_model = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Apify run pricing model is missing"))?;
        if pricing_model != "PAY_PER_EVENT" {
            return Ok(None);
        }

        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let dataset_item_price_usd = events
            .get(DATASET_ITEM_EVENT)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
        if !dataset_item_price_usd.is_finite()
            || dataset_item_price_usd < 0.0
            || !max_total_charge_usd.is_finite()
            || max_total_charge_usd < 0.0
        {
            bail!("Apify run returned invalid charging values");
        }

        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_usd = 0.0;
        for (event_name, count) in charged_event_counts {
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

        Ok(Some(Self {
            dataset_item_price_usd,
            charged_usd,
            max_total_charge_usd,
        }))
    }

    fn affordable_items(&self, requested: usize) -> usize {
        if self.dataset_item_price_usd == 0.0 {
            return requested;
        }
        let tolerance = f64::EPSILON * self.max_total_charge_usd.max(1.0);
        (1..=requested)
            .take_while(|count| {
                self.charged_usd + *count as f64 * self.dataset_item_price_usd
                    <= self.max_total_charge_usd + tolerance
            })
            .count()
    }
}

async fn get_dataset_budget(
    client: &Client,
    config: &ActorConfig,
) -> Result<Option<DatasetBudget>> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = send_apify_request(|| client.get(url.clone()).bearer_auth(&config.apify_token))
        .await
        .context("Apify run pricing request failed")?;
    let run = response_json(response, "Apify run pricing request").await?;
    DatasetBudget::from_run(&run)
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    items: &[Value],
) -> Result<usize> {
    if items.is_empty() {
        return Ok(0);
    }

    let budget = get_dataset_budget(client, config).await?;
    let saved_items = budget
        .as_ref()
        .map_or(items.len(), |budget| budget.affordable_items(items.len()));
    if saved_items == 0 {
        return Ok(0);
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = send_apify_request(|| {
        client
            .post(url.clone())
            .bearer_auth(&config.apify_token)
            .json(&items[..saved_items])
    })
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
    Ok(saved_items)
}

async fn set_output(client: &Client, config: &ActorConfig, output: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            "OUTPUT",
        ],
    )?;
    let response = send_apify_request(|| {
        client
            .put(url.clone())
            .bearer_auth(&config.apify_token)
            .json(output)
    })
    .await
    .context("Apify OUTPUT write failed")?;
    if response.status().is_success() {
        return Ok(());
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
        "Apify OUTPUT write failed with {} {reason}{detail}",
        status.as_u16()
    );
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let (query, url) = autocomplete_url(&input, &config.scrappa_api_base_url)?;
    println!("Getting autocomplete suggestions for: \"{query}\"");

    let response = fetch_suggestions(client, config, &url).await?;
    let suggestions = response
        .get("suggestions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let saved_count = if suggestions.is_empty() {
        println!("No autocomplete suggestions found for the given query");
        0
    } else {
        let saved_count = push_dataset_items(client, config, &suggestions).await?;
        println!("Found {} suggestions", suggestions.len());
        if saved_count < suggestions.len() {
            println!(
                "PAY_PER_EVENT spending limit allowed {saved_count} of {} suggestion dataset item(s)",
                suggestions.len()
            );
        }
        saved_count
    };

    set_output(client, config, &response).await?;

    let summary = json!({
        "query": query,
        "suggestions_found": suggestions.len(),
    });
    println!("Autocomplete completed: {}", summary);
    if saved_count == 0 && !suggestions.is_empty() {
        println!("No suggestions were saved because the PAY_PER_EVENT spending limit was reached");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let message = format!("{error:#}");
        eprintln!("Failed: {message}");
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
        collections::HashMap,
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc, Mutex,
        },
        thread::{self, JoinHandle},
        time::Instant,
    };

    #[derive(Clone, Debug)]
    struct RecordedRequest {
        method: String,
        target: String,
        headers: HashMap<String, String>,
        body: String,
    }

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: Url,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(
            handler: impl Fn(&RecordedRequest) -> MockResponse + Send + Sync + 'static,
        ) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let recorded_requests = Arc::clone(&requests);
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let handler = Arc::new(handler);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(15);
                while !stopped.load(Ordering::Relaxed) && Instant::now() < deadline {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                            if let Some(request) = read_request(&mut stream) {
                                recorded_requests.lock().unwrap().push(request.clone());
                                let response = handler(&request);
                                let _ = write_response(&mut stream, response);
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => break,
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

        fn requests(&self) -> Vec<RecordedRequest> {
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

    fn read_request(stream: &mut TcpStream) -> Option<RecordedRequest> {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];
        let mut header_end = None;
        let mut content_length = 0;

        loop {
            let read = stream.read(&mut buffer).ok()?;
            if read == 0 {
                return None;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if header_end.is_none() {
                if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    header_end = Some(position + 4);
                    let headers = String::from_utf8_lossy(&bytes[..position]);
                    content_length = headers
                        .lines()
                        .skip(1)
                        .filter_map(|line| line.split_once(':'))
                        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                }
            }
            if let Some(header_end) = header_end {
                if bytes.len() >= header_end + content_length {
                    break;
                }
            }
        }

        let header_end = header_end?;
        let headers_text = String::from_utf8_lossy(&bytes[..header_end - 4]);
        let mut lines = headers_text.lines();
        let mut request_line = lines.next()?.split_whitespace();
        let method = request_line.next()?;
        let target = request_line.next()?;
        let mut headers = HashMap::new();
        for line in lines {
            if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
            }
        }
        let body = String::from_utf8_lossy(&bytes[header_end..header_end + content_length]).into();
        Some(RecordedRequest {
            method: method.to_owned(),
            target: target.to_owned(),
            headers,
            body,
        })
    }

    fn write_response(stream: &mut TcpStream, response: MockResponse) -> std::io::Result<()> {
        let reason = match response.status {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            401 => "Unauthorized",
            503 => "Service Unavailable",
            _ => "Mock Response",
        };
        write!(
            stream,
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response.status,
            reason,
            response.body.len(),
            response.body
        )
    }

    fn config(apify_api_base_url: Url, scrappa_api_base_url: Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url,
            scrappa_api_base_url,
            scrappa_api_key: "scrappa-test-key".to_owned(),
            default_key_value_store_id: "store".to_owned(),
            default_dataset_id: "dataset".to_owned(),
            actor_run_id: "run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "apify-test-token".to_owned(),
        }
    }

    fn json_response(status: u16, body: Value) -> MockResponse {
        MockResponse {
            status,
            body: body.to_string(),
        }
    }

    #[test]
    fn autocomplete_url_keeps_the_query_and_encodes_it_for_scrappa() {
        let base_url = Url::parse("https://scrappa.co/api").unwrap();
        let (query, url) =
            autocomplete_url(&json!({"query": "time sq, New York"}), &base_url).unwrap();

        assert_eq!(query, "time sq, New York");
        assert_eq!(url.path(), "/api/maps/autocomplete");
        assert_eq!(
            url.query_pairs().collect::<Vec<_>>(),
            vec![("query".into(), "time sq, New York".into())]
        );
    }

    #[test]
    fn scrappa_error_message_matches_json_validation_details() {
        let message = scrappa_error_message(
            StatusCode::BAD_REQUEST,
            r#"{"message":"Invalid query","errors":{"query":["must be a string","is required"]}}"#,
        );
        assert_eq!(
            message,
            "Invalid query - query: must be a string, is required"
        );
    }

    #[test]
    fn scrappa_error_message_uses_status_only_for_an_empty_body() {
        assert_eq!(
            scrappa_error_message(StatusCode::BAD_GATEWAY, ""),
            "Bad Gateway"
        );
        assert_eq!(scrappa_error_message(StatusCode::BAD_GATEWAY, "  \n "), "");
    }

    #[tokio::test]
    async fn missing_apify_input_record_uses_the_required_query_error() {
        let apify = MockServer::start(|_| json_response(404, json!({"message": "Not found"})));
        let scrappa = MockServer::start(|_| {
            json_response(200, json!({"query": "unexpected", "suggestions": []}))
        });
        let config = config(apify.base_url.clone(), scrappa.base_url.clone());

        let error = run_actor(&Client::new(), &config)
            .await
            .unwrap_err()
            .to_string();

        assert!(error.contains("Search query is required"));
        assert_eq!(apify.requests().len(), 1);
        assert!(scrappa.requests().is_empty());
    }

    #[tokio::test]
    async fn apify_requests_retry_rate_limits_and_server_errors() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let request_attempts = Arc::clone(&attempts);
        let server =
            MockServer::start(
                move |_| match request_attempts.fetch_add(1, Ordering::Relaxed) {
                    0 => json_response(429, json!({"message": "Rate limited"})),
                    1 => json_response(503, json!({"message": "Temporarily unavailable"})),
                    _ => json_response(200, json!({"ok": true})),
                },
            );
        let client = Client::new();
        let url = server.base_url.clone();

        let response = send_request_with_retries(
            || client.get(url.clone()),
            Duration::from_secs(1),
            2,
            Duration::from_millis(1),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(attempts.load(Ordering::Relaxed), 3);
        assert_eq!(APIFY_REQUEST_TIMEOUT, Duration::from_secs(360));
        assert_eq!(APIFY_MAX_RETRIES, 8);
        assert_eq!(APIFY_RETRY_BASE_DELAY, Duration::from_millis(500));
        assert_eq!(
            retry_delay(Duration::from_millis(500), 3),
            Duration::from_secs(4)
        );
    }

    #[tokio::test]
    async fn apify_request_deadline_stops_a_slow_request() {
        let server = MockServer::start(|_| {
            thread::sleep(Duration::from_millis(100));
            json_response(200, json!({"ok": true}))
        });
        let client = Client::new();
        let url = server.base_url.clone();

        let error = send_request_with_retries(
            || client.get(url.clone()),
            Duration::from_millis(20),
            0,
            Duration::from_millis(1),
        )
        .await
        .unwrap_err();

        assert!(error.is_timeout());
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn scrappa_request_deadline_stops_a_slow_request_without_retry() {
        let apify = MockServer::start(|_| json_response(200, json!({"query": "coffee"})));
        let scrappa = MockServer::start(|_| {
            thread::sleep(Duration::from_millis(100));
            json_response(200, json!({"suggestions": []}))
        });
        let config = config(apify.base_url.clone(), scrappa.base_url.clone());
        let url = scrappa
            .base_url
            .join("maps/autocomplete?query=coffee")
            .unwrap();

        let error = fetch_suggestions_with_timeout(
            &Client::new(),
            &config,
            &url,
            Duration::from_millis(20),
        )
        .await
        .unwrap_err()
        .to_string();

        assert!(error.contains("Scrappa API request timed out after 20ms"));
        assert_eq!(scrappa.requests().len(), 1);
        assert_eq!(SCRAPPA_REQUEST_TIMEOUT, Duration::from_secs(60));
    }

    #[test]
    fn pay_per_event_budget_counts_existing_event_charges_and_caps_default_dataset_items() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                        "apify-actor-start": {"eventPriceUsd": 0.05}
                    }}
                },
                "options": {"maxTotalChargeUsd": 1.0},
                "chargedEventCounts": {
                    "apify-default-dataset-item": 7,
                    "apify-actor-start": 1
                }
            }
        });
        let budget = DatasetBudget::from_run(&run).unwrap().unwrap();

        assert_eq!(budget.affordable_items(10), 2);
    }

    #[tokio::test]
    async fn actor_preserves_auth_dataset_rows_output_and_pay_per_event_budget() {
        let apify =
            MockServer::start(
                |request| match (request.method.as_str(), request.target.as_str()) {
                    ("GET", "/v2/key-value-stores/store/records/INPUT") => {
                        json_response(200, json!({"query": "time sq, New York"}))
                    }
                    ("GET", "/v2/actor-runs/run") => json_response(
                        200,
                        json!({
                            "data": {
                                "pricingInfo": {
                                    "pricingModel": "PAY_PER_EVENT",
                                    "pricingPerEvent": {"actorChargeEvents": {
                                        "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                                        "apify-actor-start": {"eventPriceUsd": 0.05}
                                    }}
                                },
                                "options": {"maxTotalChargeUsd": 1.0},
                                "chargedEventCounts": {
                                    "apify-default-dataset-item": 7,
                                    "apify-actor-start": 1
                                }
                            }
                        }),
                    ),
                    ("POST", "/v2/datasets/dataset/items") => MockResponse {
                        status: 201,
                        body: String::new(),
                    },
                    ("PUT", "/v2/key-value-stores/store/records/OUTPUT") => MockResponse {
                        status: 201,
                        body: String::new(),
                    },
                    _ => json_response(400, json!({"message": "Unexpected Apify request"})),
                },
            );
        let expected_response = json!({
            "query": "time sq, New York",
            "suggestions": [
                {"main_text": "Times Square", "type": "geocode"},
                {"main_text": "New York, NY", "type": "locality"},
                {"main_text": "Times Square Station", "type": "transit_station"}
            ],
            "pagination": {"next_page_token": "unused"}
        });
        let response_text = expected_response.to_string();
        let scrappa = MockServer::start(move |request| {
            if request.method == "GET" && request.target.starts_with("/api/maps/autocomplete?") {
                MockResponse {
                    status: 200,
                    body: response_text.clone(),
                }
            } else {
                json_response(400, json!({"message": "Unexpected Scrappa request"}))
            }
        });
        let scrappa_api_base_url = scrappa.base_url.join("api").unwrap();
        let config = config(apify.base_url.clone(), scrappa_api_base_url);
        let client = Client::new();

        run_actor(&client, &config).await.unwrap();

        let apify_requests = apify.requests();
        assert_eq!(apify_requests.len(), 4);
        assert!(apify_requests.iter().all(|request| request
            .headers
            .get("authorization")
            .map(String::as_str)
            == Some("Bearer apify-test-token")));
        let dataset_request = apify_requests
            .iter()
            .find(|request| request.method == "POST")
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&dataset_request.body).unwrap(),
            json!([
                {"main_text": "Times Square", "type": "geocode"},
                {"main_text": "New York, NY", "type": "locality"}
            ])
        );
        let output_request = apify_requests
            .iter()
            .find(|request| request.method == "PUT")
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&output_request.body).unwrap(),
            expected_response
        );

        let scrappa_requests = scrappa.requests();
        assert_eq!(scrappa_requests.len(), 1);
        assert_eq!(
            scrappa_requests[0]
                .headers
                .get("x-api-key")
                .map(String::as_str),
            Some("scrappa-test-key")
        );
        assert_eq!(
            scrappa_requests[0]
                .headers
                .get("accept")
                .map(String::as_str),
            Some("application/json")
        );
        let request_url =
            Url::parse(&format!("http://mock{}", scrappa_requests[0].target)).unwrap();
        assert_eq!(
            request_url.query_pairs().collect::<Vec<_>>(),
            vec![("query".into(), "time sq, New York".into())]
        );
    }

    #[tokio::test]
    async fn scrappa_upstream_error_is_returned_without_retry() {
        let apify = MockServer::start(|request| {
            if request.method == "GET"
                && request.target == "/v2/key-value-stores/store/records/INPUT"
            {
                json_response(200, json!({"query": "coffee"}))
            } else {
                json_response(400, json!({"message": "Unexpected Apify request"}))
            }
        });
        let scrappa = MockServer::start(|_| {
            json_response(503, json!({"message": "Service temporarily unavailable"}))
        });
        let config = config(apify.base_url.clone(), scrappa.base_url.clone());
        let client = Client::new();

        let error = run_actor(&client, &config).await.unwrap_err().to_string();

        assert!(error.contains("Scrappa API error (503): Service temporarily unavailable"));
        assert_eq!(scrappa.requests().len(), 1);
        assert_eq!(apify.requests().len(), 1);
    }
}
