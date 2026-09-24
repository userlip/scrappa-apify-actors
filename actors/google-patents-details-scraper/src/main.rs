mod patents;

use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use patents::{
    build_error_dataset_item, build_success_dataset_item, collect_requests, describe_requests,
    error_from_response, PatentRequest,
};
use rand::random;
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_REQUEST_ATTEMPTS: usize = 3;
const SCRAPPA_RETRY_MAX_DELAY_MS: u64 = 10_000;
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const USER_AGENT: &str = "thescrappa-google-patents-details-scraper/1.0";

struct Config {
    apify_api_base: Url,
    scrappa_api_base: Url,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        Ok(Self {
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key,
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
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Value::Null);
        }
        response_json(response, "Apify INPUT request").await
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = self
            .http
            .get(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        response_json(response, "Apify run pricing request").await
    }

    async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(&[item])
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
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
            .timeout(APIFY_REQUEST_TIMEOUT)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }
}

#[derive(Debug)]
struct ScrappaFailure {
    status: Option<u16>,
    timed_out: bool,
    message: String,
}

impl std::fmt::Display for ScrappaFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(status) = self.status {
            write!(formatter, "Scrappa API error ({status}): {}", self.message)
        } else {
            formatter.write_str(&self.message)
        }
    }
}

impl std::error::Error for ScrappaFailure {}

impl ScrappaFailure {
    fn http(status: u16, message: String) -> Self {
        Self {
            status: Some(status),
            timed_out: false,
            message,
        }
    }

    fn timeout(timeout: Duration) -> Self {
        Self {
            status: None,
            timed_out: true,
            message: format!(
                "Scrappa API request timed out after {}ms",
                timeout.as_millis()
            ),
        }
    }

    fn other(message: impl Into<String>) -> Self {
        Self {
            status: None,
            timed_out: false,
            message: message.into(),
        }
    }

    fn is_retryable(&self) -> bool {
        self.timed_out
            || self
                .status
                .is_some_and(|status| matches!(status, 408 | 429 | 500 | 502 | 503 | 504))
    }
}

struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
    timeout: Duration,
    retry_max_delay_ms: u64,
}

impl ScrappaClient {
    fn new(api_key: String, base_url: Url, timeout: Duration) -> Self {
        Self {
            http: Client::new(),
            base_url,
            api_key,
            timeout,
            retry_max_delay_ms: SCRAPPA_RETRY_MAX_DELAY_MS,
        }
    }

    async fn get_details(
        &self,
        request: &PatentRequest,
    ) -> std::result::Result<Value, ScrappaFailure> {
        for attempt in 1..=SCRAPPA_REQUEST_ATTEMPTS {
            match self.send_details(request).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt >= SCRAPPA_REQUEST_ATTEMPTS || !error.is_retryable() {
                        return Err(error);
                    }
                    let jitter_ms = (random::<u32>() % 1000) as u64;
                    let delay_ms = retry_delay_ms(attempt, jitter_ms, self.retry_max_delay_ms);
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{SCRAPPA_REQUEST_ATTEMPTS} in {delay_ms}ms.",
                        error, attempt + 1
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
        Err(ScrappaFailure::other("Scrappa API request failed"))
    }

    async fn send_details(
        &self,
        request: &PatentRequest,
    ) -> std::result::Result<Value, ScrappaFailure> {
        let mut url = endpoint_url(&self.base_url, &["google-patents", "details"])
            .map_err(|error| ScrappaFailure::other(error.to_string()))?;
        url.query_pairs_mut()
            .append_pair("patent_id", &request.normalized_patent_id);

        let response = self
            .http
            .get(url)
            .timeout(self.timeout)
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .send()
            .await
            .map_err(|error| map_reqwest_error(error, self.timeout))?;
        let status = response.status();
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = response
            .text()
            .await
            .map_err(|error| map_reqwest_error(error, self.timeout))?;
        if !status.is_success() {
            return Err(parse_scrappa_error(status.as_u16(), reason, &body));
        }
        serde_json::from_str(&body).map_err(|error| {
            ScrappaFailure::other(format!("Scrappa API response was not valid JSON: {error}"))
        })
    }
}

fn retry_delay_ms(failed_attempt: usize, jitter_ms: u64, max_delay_ms: u64) -> u64 {
    let exponential_ms = 1_000u64.saturating_mul(2u64.saturating_pow(failed_attempt as u32));
    exponential_ms.saturating_add(jitter_ms).min(max_delay_ms)
}

fn map_reqwest_error(error: reqwest::Error, timeout: Duration) -> ScrappaFailure {
    if error.is_timeout() {
        ScrappaFailure::timeout(timeout)
    } else {
        ScrappaFailure::other(error.to_string())
    }
}

fn parse_scrappa_error(status: u16, reason: &str, body: &str) -> ScrappaFailure {
    let fallback = if reason.is_empty() {
        format!("HTTP {status}")
    } else {
        reason.to_owned()
    };
    let Ok(data) = serde_json::from_str::<Value>(body) else {
        return ScrappaFailure::http(status, truncate_error_body(body, &fallback));
    };
    let Some(object) = data.as_object() else {
        return ScrappaFailure::http(status, truncate_error_body(body, &fallback));
    };

    let mut message = object
        .get("message")
        .filter(|value| !value.is_null())
        .map(json_value_string)
        .unwrap_or_else(|| fallback.clone());
    if let Some(errors) = object.get("errors").filter(|value| json_truthy(value)) {
        let Some(errors) = errors.as_object() else {
            return ScrappaFailure::http(status, truncate_error_body(body, &fallback));
        };
        let details = errors
            .iter()
            .map(|(field, messages)| {
                let Some(messages) = messages.as_array() else {
                    return Err(());
                };
                Ok(format!(
                    "{field}: {}",
                    messages
                        .iter()
                        .map(json_value_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
            .collect::<std::result::Result<Vec<_>, _>>();
        let Ok(details) = details else {
            return ScrappaFailure::http(status, truncate_error_body(body, &fallback));
        };
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details.join("; "));
        }
    }
    ScrappaFailure::http(status, message)
}

fn truncate_error_body(body: &str, fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }
    let trimmed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut value = trimmed.chars().take(500).collect::<String>();
    if value.is_empty() {
        value = fallback.to_owned();
    }
    value
}

fn json_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn json_value_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(number) => number
            .as_f64()
            .filter(|value| value.fract() == 0.0 && value.abs() < 1e21)
            .map(|value| format!("{value:.0}"))
            .unwrap_or_else(|| number.to_string()),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(json_value_string)
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[derive(Debug)]
struct DatasetBudget {
    initial_spend: f64,
    saved_dataset_items: u64,
    item_price: f64,
    max_charge: Option<f64>,
}

impl DatasetBudget {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_model = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            .filter(|pricing_model| !pricing_model.is_empty())
            .ok_or_else(|| anyhow!("Apify run did not provide a valid pricing model"))?;
        if pricing_model != "PAY_PER_EVENT" {
            return Ok(Self::uncapped());
        }
        let event_prices = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let item_price = event_prices
            .get(DATASET_ITEM_EVENT)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
        let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
            None | Some(Value::Null) => None,
            Some(Value::Number(number)) => Some(
                number
                    .as_f64()
                    .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?,
            ),
            Some(_) => bail!("Apify run returned an invalid spending limit"),
        };
        if !item_price.is_finite()
            || item_price < 0.0
            || max_charge.is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            bail!("Apify run returned invalid charging values");
        }

        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut initial_spend = 0.0;
        for (event_name, count) in counts {
            let count = count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            if count == 0 {
                continue;
            }
            let price = event_prices
                .get(event_name)
                .and_then(|event| event.get("eventPriceUsd"))
                .and_then(Value::as_f64)
                .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
            if !price.is_finite() || price < 0.0 {
                bail!("Invalid price for charged event {event_name}");
            }
            initial_spend += price * count as f64;
        }
        if !initial_spend.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        Ok(Self {
            initial_spend,
            saved_dataset_items: 0,
            item_price,
            max_charge,
        })
    }

    fn uncapped() -> Self {
        Self {
            initial_spend: 0.0,
            saved_dataset_items: 0,
            item_price: 0.0,
            max_charge: None,
        }
    }

    fn affordable_items(&self, requested: usize) -> usize {
        let Some(max_charge) = self.max_charge else {
            return requested;
        };
        if self.item_price == 0.0 {
            return requested;
        }
        let tolerance = f64::EPSILON * max_charge.max(1.0);
        (1..=requested)
            .take_while(|new_items| {
                let count = self.saved_dataset_items.saturating_add(*new_items as u64);
                self.initial_spend + self.item_price * count as f64 <= max_charge + tolerance
            })
            .count()
    }

    fn item_saved(&mut self) -> Result<()> {
        self.saved_dataset_items = self
            .saved_dataset_items
            .checked_add(1)
            .ok_or_else(|| anyhow!("Dataset row count overflowed"))?;
        Ok(())
    }
}

fn format_google_patents_details_error(error: &ScrappaFailure) -> String {
    if error.timed_out {
        format!(
            "{}. The Google Patents details request exceeded the {}s Scrappa API timeout. Run the request again or try a smaller batch.",
            error,
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        error.to_string()
    }
}

async fn run_actor(http: &Client, config: &Config) -> Result<()> {
    let apify = ApifyClient { http, config };
    let input = apify.get_input().await?;
    if input.is_null() {
        bail!("Input is required");
    }
    let requests = collect_requests(&input)?;
    println!(
        "Fetching Google Patents details for {}",
        describe_requests(&requests)
    );

    let run = apify.get_run().await?;
    let mut budget = DatasetBudget::from_run(&run)?;
    let scrappa = ScrappaClient::new(
        config.scrappa_api_key.clone(),
        config.scrappa_api_base.clone(),
        SCRAPPA_REQUEST_TIMEOUT,
    );
    let mut first_item: Option<Value> = None;
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut saved = 0usize;
    let mut charge_limit_reached = false;

    for request in &requests {
        if budget.affordable_items(1) == 0 {
            charge_limit_reached = true;
            println!(
                "Apify pay-per-event charge limit reached; stopping before the next patent lookup"
            );
            break;
        }

        println!("Fetching patent details: {}", request.normalized_patent_id);
        let item = match scrappa.get_details(request).await {
            Ok(response) if response.get("success").is_some_and(json_truthy) => {
                succeeded += 1;
                build_success_dataset_item(&response, request)
            }
            Ok(response) => {
                failed += 1;
                build_error_dataset_item(&error_from_response(&response), request)
            }
            Err(error) => {
                failed += 1;
                let message = format_google_patents_details_error(&error);
                eprintln!(
                    "Patent details failed for {}: {message}",
                    request.normalized_patent_id
                );
                build_error_dataset_item(&message, request)
            }
        };

        apify.push_dataset_item(&item).await?;
        budget.item_saved()?;
        saved += 1;
        first_item.get_or_insert(item);
    }

    if requests.len() == 1 {
        if let Some(first_item) = &first_item {
            apify.put_output(first_item).await?;
        }
    }

    println!("Google Patents details scraping completed successfully");
    println!(
        "Details summary: {}",
        serde_json::to_string(&json!({
            "requested": requests.len(),
            "succeeded": succeeded,
            "failed": failed,
            "dataset_items_saved": saved,
            "charge_limit_reached": charge_limit_reached,
            "unprocessed": requests.len().saturating_sub(saved),
        }))?
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    let result = async {
        let config = Config::from_env()?;
        let http = Client::new();
        run_actor(&http, &config).await
    }
    .await;
    if let Err(error) = result {
        eprintln!("Actor failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        io::{BufRead, BufReader, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
        thread::{self, JoinHandle},
    };

    struct MockRequest {
        target: String,
        headers: HashMap<String, String>,
    }

    struct MockResponse {
        status: u16,
        body: String,
        delay: Duration,
    }

    struct MockServer {
        base_url: String,
        requests: Receiver<MockRequest>,
        thread: JoinHandle<()>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                for response in responses {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_mock_request(&stream);
                    sender.send(request).unwrap();
                    if !response.delay.is_zero() {
                        thread::sleep(response.delay);
                    }
                    let reason = match response.status {
                        200 => "OK",
                        404 => "Not Found",
                        429 => "Too Many Requests",
                        503 => "Service Unavailable",
                        _ => "Mock Status",
                    };
                    let response_text = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    let _ = stream.write_all(response_text.as_bytes());
                }
            });
            Self {
                base_url: format!("http://{address}/api"),
                requests,
                thread,
            }
        }

        fn next_request(&self) -> MockRequest {
            self.requests
                .recv_timeout(Duration::from_secs(2))
                .expect("mock request was not received")
        }

        fn finish(self) {
            self.thread.join().unwrap();
        }
    }

    fn read_mock_request(stream: &TcpStream) -> MockRequest {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let target = line.split_whitespace().nth(1).unwrap().to_owned();
        let mut headers = HashMap::new();
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
            }
        }
        MockRequest { target, headers }
    }

    fn mock_response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
            delay: Duration::ZERO,
        }
    }

    fn pricing_run(max_charge: Option<f64>, dataset_items: u64, start_events: u64) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0002},
                        "apify-actor-start": {"eventPriceUsd": 0.0001}
                    }}
                },
                "chargedEventCounts": {
                    "apify-default-dataset-item": dataset_items,
                    "apify-actor-start": start_events
                },
                "options": {"maxTotalChargeUsd": max_charge}
            }
        })
    }

    #[test]
    fn retry_policy_matches_three_attempt_exponential_backoff() {
        assert!(ScrappaFailure::timeout(SCRAPPA_REQUEST_TIMEOUT).is_retryable());
        assert!(ScrappaFailure::http(429, "busy".to_owned()).is_retryable());
        assert!(ScrappaFailure::http(503, "busy".to_owned()).is_retryable());
        assert!(!ScrappaFailure::http(404, "missing".to_owned()).is_retryable());
        assert!(!ScrappaFailure::http(401, "unauthorized".to_owned()).is_retryable());
        assert_eq!(retry_delay_ms(1, 500, SCRAPPA_RETRY_MAX_DELAY_MS), 2_500);
        assert_eq!(retry_delay_ms(2, 500, SCRAPPA_RETRY_MAX_DELAY_MS), 4_500);
        assert_eq!(retry_delay_ms(5, 999, SCRAPPA_RETRY_MAX_DELAY_MS), 10_000);
    }

    #[test]
    fn scrappa_http_errors_keep_status_and_field_details() {
        let error = parse_scrappa_error(
            422,
            "Unprocessable Entity",
            r#"{"message":"Invalid request","errors":{"patent_id":["is required","must be valid"]}}"#,
        );
        assert_eq!(
            error.to_string(),
            "Scrappa API error (422): Invalid request - patent_id: is required, must be valid"
        );
        assert!(!error.is_retryable());
        assert_eq!(
            parse_scrappa_error(404, "Not Found", "").to_string(),
            "Scrappa API error (404): Not Found"
        );
    }

    #[test]
    fn timeout_errors_keep_the_original_request_deadline_guidance() {
        let message =
            format_google_patents_details_error(&ScrappaFailure::timeout(SCRAPPA_REQUEST_TIMEOUT));
        assert_eq!(
            message,
            "Scrappa API request timed out after 60000ms. The Google Patents details request exceeded the 60s Scrappa API timeout. Run the request again or try a smaller batch."
        );
    }

    #[tokio::test]
    async fn scrappa_client_retries_transient_failures_and_sends_auth_and_normalized_id() {
        let server = MockServer::start(vec![
            mock_response(503, r#"{"message":"busy"}"#),
            mock_response(429, r#"{"message":"slow down"}"#),
            mock_response(200, r#"{"success":true,"data":{"title":"Patent"}}"#),
        ]);
        let mut client = ScrappaClient::new(
            "test-scrappa-key".to_owned(),
            Url::parse(&server.base_url).unwrap(),
            Duration::from_secs(2),
        );
        client.retry_max_delay_ms = 0;
        let request = PatentRequest {
            input_patent_id: "US9789384B1".to_owned(),
            normalized_patent_id: "patent/US9789384B1/en".to_owned(),
        };

        let response = client.get_details(&request).await.unwrap();
        assert_eq!(response["success"], true);
        for _ in 0..3 {
            let request = server.next_request();
            assert_eq!(
                request.target,
                "/api/google-patents/details?patent_id=patent%2FUS9789384B1%2Fen"
            );
            assert_eq!(request.headers["x-api-key"], "test-scrappa-key");
            assert_eq!(request.headers["accept"], "application/json");
            assert_eq!(request.headers["user-agent"], USER_AGENT);
        }
        server.finish();
    }

    #[tokio::test]
    async fn scrappa_request_timeout_is_mapped_to_the_configured_deadline() {
        let mut response = mock_response(200, r#"{"success":true}"#);
        response.delay = Duration::from_millis(100);
        let server = MockServer::start(vec![response]);
        let client = ScrappaClient::new(
            "test-scrappa-key".to_owned(),
            Url::parse(&server.base_url).unwrap(),
            Duration::from_millis(10),
        );
        let request = PatentRequest {
            input_patent_id: "US9789384B1".to_owned(),
            normalized_patent_id: "patent/US9789384B1/en".to_owned(),
        };

        let error = client.send_details(&request).await.unwrap_err();
        assert!(error.timed_out);
        assert_eq!(error.message, "Scrappa API request timed out after 10ms");
        let _ = server.next_request();
        server.finish();
    }

    #[test]
    fn dataset_budget_preserves_positive_cap_and_counts_existing_events() {
        let mut budget = DatasetBudget::from_run(&pricing_run(Some(0.0005), 0, 1)).unwrap();
        assert_eq!(budget.affordable_items(3), 2);
        budget.item_saved().unwrap();
        assert_eq!(budget.affordable_items(3), 1);
        budget.item_saved().unwrap();
        assert_eq!(budget.affordable_items(3), 0);
    }

    #[test]
    fn free_dataset_items_fit_under_a_zero_spending_limit() {
        let mut run = pricing_run(Some(0.0), 0, 0);
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"][DATASET_ITEM_EVENT]
            ["eventPriceUsd"] = json!(0.0);
        let budget = DatasetBudget::from_run(&run).unwrap();
        assert_eq!(budget.affordable_items(10), 10);
    }

    #[test]
    fn dataset_budget_treats_null_and_missing_caps_as_unlimited() {
        let null_cap = pricing_run(None, 0, 0);
        let mut absent_cap = pricing_run(Some(0.0005), 0, 0);
        absent_cap["data"]["options"]
            .as_object_mut()
            .unwrap()
            .remove("maxTotalChargeUsd");

        for run in [&null_cap, &absent_cap] {
            let budget = DatasetBudget::from_run(run).unwrap();
            assert_eq!(budget.max_charge, None);
            assert_eq!(budget.affordable_items(10), 10);
        }
    }

    #[test]
    fn dataset_budget_respects_an_explicit_zero_charge_cap() {
        let budget = DatasetBudget::from_run(&pricing_run(Some(0.0), 0, 0)).unwrap();
        assert_eq!(budget.max_charge, Some(0.0));
        assert_eq!(budget.affordable_items(10), 0);
    }

    #[test]
    fn dataset_budget_allows_non_ppe_runs() {
        let run = json!({"data": {"pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM"}}});
        let budget = DatasetBudget::from_run(&run).unwrap();
        assert_eq!(budget.max_charge, None);
        assert_eq!(budget.affordable_items(10), 10);
    }

    #[test]
    fn missing_or_invalid_pricing_model_fails_closed() {
        assert!(DatasetBudget::from_run(&json!({"data": {}})).is_err());
        assert!(
            DatasetBudget::from_run(&json!({"data": {"pricingInfo": {"pricingModel": 123}}}))
                .is_err()
        );
        let mut run = pricing_run(Some(1.0), 0, 0);
        run["data"]["chargedEventCounts"]["unknown-event"] = json!(1);
        assert!(DatasetBudget::from_run(&run)
            .unwrap_err()
            .to_string()
            .contains("Missing price for charged event"));
        let mut run = pricing_run(Some(1.0), 0, 0);
        run["data"]["options"]["maxTotalChargeUsd"] = json!("1.0");
        assert!(DatasetBudget::from_run(&run)
            .unwrap_err()
            .to_string()
            .contains("invalid spending limit"));
    }
}
