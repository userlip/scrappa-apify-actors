use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response};
use serde_json::{json, Value};
use std::{env, time::Duration};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const SEARCH_PARAMETERS: [&str; 15] = [
    "query",
    "location",
    "gl",
    "hl",
    "google_domain",
    "start",
    "amount",
    "safe",
    "tbs",
    "tbm",
    "lr",
    "cr",
    "uule",
    "nfpr",
    "filter",
];

struct Config {
    apify_api_base: Url,
    scrappa_api_base: Url,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    scrappa_api_key: String,
    pricing_snapshot: Option<PricingSnapshot>,
    max_total_charge_usd: Option<f64>,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;
        let max_total_charge_usd = optional_charge_limit_from_env()?;
        Ok(Self {
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "INPUT".to_owned()),
            scrappa_api_key,
            pricing_snapshot: pricing_snapshot_from_env()?,
            max_total_charge_usd,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn optional_charge_limit_from_env() -> Result<Option<f64>> {
    let Ok(raw_limit) = env::var("ACTOR_MAX_TOTAL_CHARGE_USD") else {
        return Ok(None);
    };
    let limit = raw_limit
        .parse::<f64>()
        .with_context(|| "ACTOR_MAX_TOTAL_CHARGE_USD must be a number")?;
    if !limit.is_finite() || limit < 0.0 {
        bail!("ACTOR_MAX_TOTAL_CHARGE_USD must be a finite non-negative number");
    }
    Ok(Some(limit))
}

#[derive(Clone)]
struct PricingSnapshot {
    pricing_info: Value,
    charged_event_counts: Value,
    max_total_charge_usd: Option<f64>,
}

fn pricing_snapshot_from_env() -> Result<Option<PricingSnapshot>> {
    let (Ok(pricing_info), Ok(charged_event_counts)) = (
        env::var("APIFY_ACTOR_PRICING_INFO"),
        env::var("APIFY_CHARGED_ACTOR_EVENT_COUNTS"),
    ) else {
        return Ok(None);
    };

    parse_pricing_snapshot(
        &pricing_info,
        &charged_event_counts,
        optional_charge_limit_from_env()?,
    )
    .map(Some)
}

fn parse_pricing_snapshot(
    pricing_info: &str,
    charged_event_counts: &str,
    max_total_charge_usd: Option<f64>,
) -> Result<PricingSnapshot> {
    Ok(PricingSnapshot {
        pricing_info: serde_json::from_str(pricing_info)
            .context("APIFY_ACTOR_PRICING_INFO must contain valid JSON")?,
        charged_event_counts: serde_json::from_str(charged_event_counts)
            .context("APIFY_CHARGED_ACTOR_EVENT_COUNTS must contain valid JSON")?,
        max_total_charge_usd,
    })
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
    let body = response
        .text()
        .await
        .with_context(|| format!("{operation} response could not be read"))?;
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
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
    serde_json::from_str(&body).with_context(|| format!("{operation} returned invalid JSON"))
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

struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

impl ApifyClient<'_> {
    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
    }

    async fn get_input(&self) -> Result<Value> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(Value::Null);
        }
        response_json(response, "Apify INPUT request").await
    }

    async fn pricing_snapshot(&self) -> Result<PricingSnapshot> {
        if let Some(snapshot) = &self.config.pricing_snapshot {
            let mut snapshot = snapshot.clone();
            if self.config.max_total_charge_usd.is_some() {
                snapshot.max_total_charge_usd = self.config.max_total_charge_usd;
            }
            return Ok(snapshot);
        }

        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = response_json(response, "Apify run pricing request").await?;
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_info = data
            .get("pricingInfo")
            .filter(|value| !value.is_null())
            .cloned()
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let charged_event_counts = data
            .get("chargedEventCounts")
            .filter(|value| !value.is_null())
            .cloned()
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let max_total_charge_usd = self.config.max_total_charge_usd.or_else(|| {
            data.pointer("/options/maxTotalChargeUsd")
                .and_then(Value::as_f64)
        });
        Ok(PricingSnapshot {
            pricing_info,
            charged_event_counts,
            max_total_charge_usd,
        })
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<usize> {
        if items.is_empty() {
            return Ok(0);
        }

        let snapshot = self.pricing_snapshot().await?;
        let limit = affordable_dataset_items(&snapshot, items.len())?;
        let items = &items[..limit];
        if items.is_empty() {
            return Ok(0);
        }

        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await?;
        Ok(items.len())
    }

    async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }
}

fn affordable_dataset_items(snapshot: &PricingSnapshot, requested: usize) -> Result<usize> {
    if snapshot
        .pricing_info
        .get("pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        bail!("Apify run is not configured for pay-per-event pricing");
    }

    let events = snapshot
        .pricing_info
        .pointer("/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_event = events
        .get(DATASET_ITEM_EVENT)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;

    let Some(max_charge) = snapshot.max_total_charge_usd else {
        return Ok(requested);
    };
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned an invalid spending limit");
    }

    let item_price = event_price(item_event, DATASET_ITEM_EVENT)?;
    let counts = snapshot
        .charged_event_counts
        .as_object()
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    for (event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if count == 0 {
            continue;
        }
        let event = events
            .get(event_name)
            .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
        spent += event_price(event, event_name)? * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((0..=requested)
        .rev()
        .find(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .unwrap_or(0))
}

fn event_price(event: &Value, event_name: &str) -> Result<f64> {
    if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        return Ok(price);
    }

    let tiered_prices = event
        .get("eventTieredPricingUsd")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
    let mut highest_price: Option<f64> = None;
    // Use the highest tier when the run snapshot does not resolve a tier.
    for (tier, tiered_price) in tiered_prices {
        let price = tiered_price
            .get("tieredEventPriceUsd")
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Invalid {tier} price for charged event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid {tier} price for charged event {event_name}");
        }
        highest_price = Some(highest_price.map_or(price, |highest| highest.max(price)));
    }
    highest_price.ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(number) => number
            .as_f64()
            .filter(|value| value.fract() == 0.0 && value.abs() < 1e21)
            .map(|value| {
                if value == 0.0 {
                    "0".to_owned()
                } else {
                    format!("{value:.0}")
                }
            })
            .unwrap_or_else(|| number.to_string()),
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

fn build_search_url(base_url: &Url, input: &Value) -> Result<Url> {
    let query = input
        .as_object()
        .and_then(|input| input.get("query"))
        .filter(|value| js_truthy(value))
        .ok_or_else(|| anyhow!("Search query is required"))?;
    let input = input
        .as_object()
        .ok_or_else(|| anyhow!("Search query is required"))?;
    let mut url = endpoint_url(base_url, &["search"])?;
    for name in SEARCH_PARAMETERS {
        let Some(value) = input.get(name) else {
            continue;
        };
        if value.is_null() || value.as_str() == Some("") {
            continue;
        }
        url.query_pairs_mut().append_pair(name, &js_string(value));
    }
    if !input.contains_key("query") || !js_truthy(query) {
        bail!("Search query is required");
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

fn scrappa_request_error(error: reqwest::Error, timeout: Duration) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            timeout.as_millis()
        )
    } else {
        anyhow!(error.to_string())
    }
}

fn scrappa_error(status: u16, body: &str) -> ScrappaApiError {
    let Some(data) = serde_json::from_str::<Value>(body).ok() else {
        return ScrappaApiError {
            status,
            message: if body.is_empty() {
                format!("HTTP {status}")
            } else {
                body.to_owned()
            },
        };
    };
    let Some(object) = data.as_object() else {
        return ScrappaApiError {
            status,
            message: body.to_owned(),
        };
    };

    let mut message = object
        .get("message")
        .filter(|value| !value.is_null())
        .map(js_string)
        .unwrap_or_else(|| format!("HTTP {status}"));
    if let Some(errors) = object.get("errors").filter(|value| js_truthy(value)) {
        let Some(errors) = errors.as_object() else {
            return ScrappaApiError {
                status,
                message: body.to_owned(),
            };
        };
        let mut details = Vec::with_capacity(errors.len());
        for (field, messages) in errors {
            let Some(messages) = messages.as_array() else {
                return ScrappaApiError {
                    status,
                    message: body.to_owned(),
                };
            };
            details.push(format!(
                "{field}: {}",
                messages
                    .iter()
                    .map(js_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        message.push_str(" - ");
        message.push_str(&details.join("; "));
    }
    ScrappaApiError { status, message }
}

async fn fetch_google_search(
    http: &Client,
    config: &Config,
    input: &Value,
    timeout: Duration,
) -> Result<Value> {
    let url = build_search_url(&config.scrappa_api_base, input)?;
    let response = http
        .get(url)
        .timeout(timeout)
        .header("X-API-Key", &config.scrappa_api_key)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| scrappa_request_error(error, timeout))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| scrappa_request_error(error, timeout))?;
    if !status.is_success() {
        return Err(scrappa_error(status.as_u16(), &body).into());
    }
    serde_json::from_str(&body)
        .map_err(|error| anyhow!("Scrappa API response was not valid JSON: {error}"))
}

fn response_items(response: &Value, field: &str) -> Result<Vec<Value>> {
    match response.get(field) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => Ok(items.clone()),
        Some(_) => bail!("Scrappa API response field {field} must be an array"),
    }
}

fn response_length(response: &Value, field: &str) -> usize {
    match response.get(field) {
        Some(Value::Array(items)) => items.len(),
        Some(Value::String(value)) => value.encode_utf16().count(),
        Some(Value::Object(value)) => value
            .get("length")
            .and_then(Value::as_u64)
            .and_then(|length| usize::try_from(length).ok())
            .unwrap_or(0),
        _ => 0,
    }
}

fn actor_error_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if message.contains("timed out") {
        format!(
            "{message}. The Google Search request exceeded the {}s Scrappa API timeout. Try a smaller amount or run the query again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

async fn run_actor(http: &Client, config: &Config) -> Result<()> {
    let apify = ApifyClient { http, config };
    let input = apify.get_input().await?;
    let _ = build_search_url(&config.scrappa_api_base, &input)?;
    let query = input
        .get("query")
        .map(js_string)
        .unwrap_or_else(|| String::new());
    println!("Searching Google for: \"{query}\"");

    let response = fetch_google_search(http, config, &input, SCRAPPA_REQUEST_TIMEOUT).await?;
    let organic_results = response_items(&response, "organic_results")?;
    let saved_count = apify.push_dataset_items(&organic_results).await?;
    if !organic_results.is_empty() {
        println!("Found {} organic results", organic_results.len());
    }
    apify.put_output(&response).await?;

    println!("Google Search completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string(&json!({
            "organic_results": response_length(&response, "organic_results"),
            "related_searches": response_length(&response, "related_searches"),
            "related_questions": response_length(&response, "related_questions"),
            "inline_videos": response_length(&response, "inline_videos"),
            "inline_images": response_length(&response, "inline_images"),
            "has_knowledge_graph": response.get("knowledge_graph").is_some_and(js_truthy),
            "has_local_results": response.get("local_results").is_some_and(js_truthy),
            "dataset_items_saved": saved_count,
        }))?
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    let result = async {
        let config = Config::from_env()?;
        let http = Client::builder()
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .build()
            .context("Could not create HTTP client")?;
        run_actor(&http, &config).await
    }
    .await;

    if let Err(error) = result {
        eprintln!("Actor failed: {}", actor_error_message(&error));
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
        thread::{self, JoinHandle},
    };

    const INPUT_SCHEMA: &str = include_str!("../.actor/input_schema.json");

    struct MockRequest {
        method: String,
        target: String,
        headers: Map<String, Value>,
        body: String,
    }

    struct MockServer {
        base_url: String,
        requests: Receiver<MockRequest>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<(u16, String)>) -> Self {
            Self::start_delayed(responses, Duration::ZERO)
        }

        fn start_delayed(responses: Vec<(u16, String)>, delay: Duration) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                for (status, body) in responses {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_mock_request(&mut stream);
                    sender.send(request).unwrap();
                    if !delay.is_zero() {
                        thread::sleep(delay);
                    }
                    let reason = match status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        403 => "Forbidden",
                        404 => "Not Found",
                        422 => "Unprocessable Entity",
                        500 => "Internal Server Error",
                        _ => "Error",
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();
                }
            });
            Self {
                base_url: format!("http://{address}"),
                requests,
                thread: Some(thread),
            }
        }

        fn finish(mut self) -> Vec<MockRequest> {
            self.thread.take().unwrap().join().unwrap();
            self.requests.try_iter().collect()
        }
    }

    fn mock_response(status: u16, body: Value) -> (u16, String) {
        (status, body.to_string())
    }

    fn read_mock_request(stream: &mut TcpStream) -> MockRequest {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
            if let Some(header_end) = header_end {
                let header_text = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length = header_text
                    .lines()
                    .skip(1)
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0, "mock request ended before its body was read");
            bytes.extend_from_slice(&buffer[..count]);
        }
        let header_end = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let header_text = String::from_utf8_lossy(&bytes[..header_end]);
        let mut lines = header_text.lines();
        let mut request_line = lines.next().unwrap().split_whitespace();
        let method = request_line.next().unwrap().to_owned();
        let target = request_line.next().unwrap().to_owned();
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| {
                (
                    name.to_ascii_lowercase(),
                    Value::String(value.trim().to_owned()),
                )
            })
            .collect();
        let body = String::from_utf8(bytes[header_end + 4..].to_vec()).unwrap();
        MockRequest {
            method,
            target,
            headers,
            body,
        }
    }

    fn test_config(apify_base_url: &str, scrappa_base_url: &str) -> Config {
        Config {
            apify_api_base: Url::parse(apify_base_url).unwrap(),
            scrappa_api_base: Url::parse(scrappa_base_url).unwrap(),
            apify_token: "apify-test-token".to_owned(),
            key_value_store_id: "store-id".to_owned(),
            dataset_id: "dataset-id".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
            pricing_snapshot: None,
            max_total_charge_usd: None,
        }
    }

    fn ppe_run(max_charge: f64, charged_counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-actor-start": {"eventPriceUsd": 0.0001},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": charged_counts
            }
        })
    }

    fn sample_input() -> Value {
        json!({
            "query": "rust actor smoke",
            "location": "New York, NY",
            "gl": "us",
            "hl": "en",
            "google_domain": "google.com",
            "start": 20,
            "amount": 10,
            "safe": "off",
            "tbs": "qdr:w",
            "tbm": "nws",
            "lr": "lang_en",
            "cr": "countryUS",
            "uule": "w+CAIQICIFTmV3IFlvcms",
            "nfpr": 1,
            "filter": 0,
            "empty": ""
        })
    }

    fn sample_response() -> Value {
        json!({
            "search_information": {
                "query_displayed": "rust actor smoke",
                "total_results": 200
            },
            "organic_results": [
                {"position": 1, "title": "One", "link": "https://one.example", "source": "one.example"},
                {"position": 2, "title": "Two", "link": "https://two.example", "source": "two.example"},
                {"position": 3, "title": "Three", "link": "https://three.example", "source": "three.example"}
            ],
            "related_searches": [{"query": "related", "link": "https://related.example"}],
            "related_questions": [{"question": "A question?"}],
            "knowledge_graph": {"title": "Rust"},
            "local_results": {"places": []},
            "inline_videos": [{"title": "video"}],
            "inline_images": [{"title": "image"}],
            "unmodeled_response_field": {"value": true}
        })
    }

    fn actor_http_client() -> Client {
        Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap()
    }

    fn get_json_request(request: &MockRequest) -> Value {
        serde_json::from_str(&request.body).unwrap()
    }

    #[test]
    fn input_schema_keeps_the_public_actor_prefill_and_pagination_contract() {
        let schema: Value = serde_json::from_str(INPUT_SCHEMA).unwrap();
        assert_eq!(schema["title"], "Google Search Scraper");
        assert_eq!(
            schema["properties"]["query"]["prefill"],
            "best restaurants in new york"
        );
        assert_eq!(schema["properties"]["start"]["default"], 0);
        assert_eq!(schema["properties"]["amount"]["minimum"], 1);
        assert_eq!(schema["properties"]["amount"]["maximum"], 100);
    }

    #[test]
    fn search_url_forwards_all_supported_parameters_and_omits_empty_strings() {
        let base_url = Url::parse("https://scrappa.co/api").unwrap();
        let url = build_search_url(&base_url, &sample_input()).unwrap();
        let parameters = url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), Value::String(value.into_owned())))
            .collect::<Map<String, Value>>();
        for name in SEARCH_PARAMETERS {
            assert!(parameters.contains_key(name), "missing {name}");
        }
        assert!(!parameters.contains_key("empty"));
        assert_eq!(parameters["query"], "rust actor smoke");
        assert_eq!(parameters["start"], "20");
        assert_eq!(parameters["nfpr"], "1");
        assert_eq!(parameters["filter"], "0");
        assert_eq!(url.path(), "/api/search");
    }

    #[test]
    fn query_is_required_and_uses_javascript_truthiness() {
        let base_url = Url::parse("https://scrappa.co/api").unwrap();
        for input in [
            json!({}),
            json!({"query": ""}),
            json!({"query": 0}),
            Value::Null,
        ] {
            assert_eq!(
                build_search_url(&base_url, &input).unwrap_err().to_string(),
                "Search query is required"
            );
        }
        assert!(build_search_url(&base_url, &json!({"query": 1})).is_ok());
    }

    #[test]
    fn ppe_budget_accounts_for_prior_events_and_uses_the_highest_tier_price() {
        let snapshot = PricingSnapshot {
            pricing_info: json!({
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "apify-actor-start": {"eventPriceUsd": 0.0001},
                    "apify-default-dataset-item": {"eventTieredPricingUsd": {
                        "FREE": {"tieredEventPriceUsd": 0.0003},
                        "GOLD": {"tieredEventPriceUsd": 0.0002}
                    }}
                }}
            }),
            charged_event_counts: json!({"apify-actor-start": 1}),
            max_total_charge_usd: Some(0.0007),
        };
        assert_eq!(affordable_dataset_items(&snapshot, 3).unwrap(), 2);
    }

    #[test]
    fn runtime_pricing_snapshot_preserves_the_actor_budget_inputs() {
        let snapshot = parse_pricing_snapshot(
            r#"{
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {
                    "actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
                    }
                }
            }"#,
            r#"{"apify-default-dataset-item":2}"#,
            Some(0.001),
        )
        .unwrap();
        assert_eq!(snapshot.max_total_charge_usd, Some(0.001));
        assert_eq!(affordable_dataset_items(&snapshot, 2).unwrap(), 1);
    }

    #[test]
    fn ppe_without_a_run_limit_keeps_all_requested_results() {
        let snapshot = PricingSnapshot {
            pricing_info: json!({
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
                }}
            }),
            charged_event_counts: json!({}),
            max_total_charge_usd: None,
        };
        assert_eq!(affordable_dataset_items(&snapshot, 3).unwrap(), 3);
    }

    #[test]
    fn ppe_rejects_non_ppe_runs_and_invalid_charged_event_pricing() {
        let non_ppe = PricingSnapshot {
            pricing_info: json!({"pricingModel": "PRICE_PER_DATASET_ITEM"}),
            charged_event_counts: json!({}),
            max_total_charge_usd: Some(1.0),
        };
        assert!(affordable_dataset_items(&non_ppe, 1)
            .unwrap_err()
            .to_string()
            .contains("not configured for pay-per-event"));

        let missing_event_price = PricingSnapshot {
            pricing_info: json!({
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
                }}
            }),
            charged_event_counts: json!({"unpriced-event": 1}),
            max_total_charge_usd: Some(1.0),
        };
        assert!(affordable_dataset_items(&missing_event_price, 1)
            .unwrap_err()
            .to_string()
            .contains("Missing price for charged event unpriced-event"));
    }

    #[tokio::test]
    async fn search_passes_auth_pagination_and_legacy_fields_to_scrappa() {
        let scrappa = MockServer::start(vec![mock_response(200, sample_response())]);
        let config = test_config("http://127.0.0.1:1", &scrappa.base_url);
        let response = fetch_google_search(
            &actor_http_client(),
            &config,
            &sample_input(),
            Duration::from_secs(2),
        )
        .await
        .unwrap();
        assert_eq!(response["organic_results"].as_array().unwrap().len(), 3);

        let requests = scrappa.finish();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].headers["x-api-key"], "scrappa-test-key");
        assert_eq!(requests[0].headers["accept"], "application/json");
        let url = Url::parse(&format!("http://mock{}", requests[0].target)).unwrap();
        let parameters = url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), Value::String(value.into_owned())))
            .collect::<Map<String, Value>>();
        assert_eq!(parameters["start"], "20");
        assert_eq!(parameters["tbm"], "nws");
        assert_eq!(parameters["lr"], "lang_en");
        assert_eq!(parameters["nfpr"], "1");
    }

    #[tokio::test]
    async fn scrappa_errors_keep_status_message_and_field_details_without_retrying() {
        let scrappa = MockServer::start(vec![mock_response(
            422,
            json!({
                "message": "Invalid request",
                "errors": {"amount": ["must be positive", "must be below 100"]}
            }),
        )]);
        let config = test_config("http://127.0.0.1:1", &scrappa.base_url);
        let error = fetch_google_search(
            &actor_http_client(),
            &config,
            &sample_input(),
            Duration::from_secs(2),
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (422): Invalid request - amount: must be positive, must be below 100"
        );
        assert_eq!(scrappa.finish().len(), 1);
    }

    #[tokio::test]
    async fn scrappa_timeout_uses_the_actor_deadline_message() {
        let timeout = Duration::from_millis(20);
        let scrappa = MockServer::start_delayed(
            vec![mock_response(200, sample_response())],
            Duration::from_millis(100),
        );
        let config = test_config("http://127.0.0.1:1", &scrappa.base_url);
        let error = fetch_google_search(&actor_http_client(), &config, &sample_input(), timeout)
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Scrappa API request timed out after 20ms"));
        assert!(actor_error_message(&error).contains("Google Search request exceeded the 60s"));
        assert_eq!(scrappa.finish().len(), 1);
    }

    #[tokio::test]
    async fn actor_saves_budgeted_dataset_rows_and_the_unmodified_full_response() {
        let input = sample_input();
        let response = sample_response();
        let apify = MockServer::start(vec![
            mock_response(200, input),
            mock_response(200, ppe_run(0.0007, json!({"apify-actor-start": 1}))),
            mock_response(200, json!({})),
            mock_response(200, json!({})),
        ]);
        let scrappa = MockServer::start(vec![mock_response(200, response.clone())]);
        let config = test_config(&apify.base_url, &scrappa.base_url);

        run_actor(&actor_http_client(), &config).await.unwrap();

        let scrappa_requests = scrappa.finish();
        assert_eq!(scrappa_requests.len(), 1);
        assert_eq!(scrappa_requests[0].headers["x-api-key"], "scrappa-test-key");

        let apify_requests = apify.finish();
        assert_eq!(apify_requests.len(), 4);
        assert_eq!(
            apify_requests[0].target,
            "/v2/key-value-stores/store-id/records/INPUT"
        );
        assert_eq!(
            apify_requests[0].headers["authorization"],
            "Bearer apify-test-token"
        );
        assert_eq!(apify_requests[1].target, "/v2/actor-runs/test-run");
        assert_eq!(apify_requests[2].target, "/v2/datasets/dataset-id/items");
        assert_eq!(
            get_json_request(&apify_requests[2]),
            json!([
                {"position": 1, "title": "One", "link": "https://one.example", "source": "one.example"},
                {"position": 2, "title": "Two", "link": "https://two.example", "source": "two.example"}
            ])
        );
        assert_eq!(
            apify_requests[3].target,
            "/v2/key-value-stores/store-id/records/OUTPUT"
        );
        assert_eq!(get_json_request(&apify_requests[3]), response);
    }

    #[tokio::test]
    async fn actor_writes_output_without_dataset_or_pricing_when_no_organic_results_exist() {
        let input = json!({"query": "no results"});
        let response = json!({
            "organic_results": [],
            "related_searches": [],
            "search_information": {"total_results": 0}
        });
        let apify = MockServer::start(vec![
            mock_response(200, input),
            mock_response(200, json!({})),
        ]);
        let scrappa = MockServer::start(vec![mock_response(200, response.clone())]);
        let config = test_config(&apify.base_url, &scrappa.base_url);

        run_actor(&actor_http_client(), &config).await.unwrap();

        let apify_requests = apify.finish();
        assert_eq!(apify_requests.len(), 2);
        assert_eq!(
            apify_requests[1].target,
            "/v2/key-value-stores/store-id/records/OUTPUT"
        );
        assert_eq!(get_json_request(&apify_requests[1]), response);
        assert_eq!(scrappa.finish().len(), 1);
    }

    #[tokio::test]
    async fn apify_dataset_write_errors_are_reported_and_output_is_not_claimed() {
        let input = sample_input();
        let apify = MockServer::start(vec![
            mock_response(200, input),
            mock_response(200, ppe_run(1.0, json!({}))),
            mock_response(500, json!({"error": {"message": "dataset unavailable"}})),
        ]);
        let scrappa = MockServer::start(vec![mock_response(200, sample_response())]);
        let config = test_config(&apify.base_url, &scrappa.base_url);

        let error = run_actor(&actor_http_client(), &config).await.unwrap_err();
        assert!(error.to_string().contains("dataset unavailable"));
        assert_eq!(apify.finish().len(), 3);
        assert_eq!(scrappa.finish().len(), 1);
    }
}
