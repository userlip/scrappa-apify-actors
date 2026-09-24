mod scrappa;
mod search;

use std::{env, process::ExitCode};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use url::Url;

use scrappa::{RetryPolicy, ScrappaClient, ScrappaError, REQUEST_TIMEOUT};
use search::{
    limit_result_count, normalize_input, organic_results, output_summary, validate_input,
};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const INPUT_KEY: &str = "INPUT";
const OUTPUT_KEY: &str = "OUTPUT";
const SEARCH_CHARGE_EVENT: &str = "linkedin-search-result";

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

fn apify_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut path = vec!["v2"];
    path.extend_from_slice(segments);
    endpoint_url(base_url, &path)
}

#[derive(Clone)]
struct PpeBudget {
    is_pay_per_event: bool,
    max_total_charge_usd: Option<f64>,
    event_prices: Map<String, Value>,
    charged_event_counts: Map<String, Value>,
}

impl PpeBudget {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let is_pay_per_event = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self {
                is_pay_per_event: false,
                max_total_charge_usd: None,
                event_prices: Map::new(),
                charged_event_counts: Map::new(),
            });
        }

        let event_prices = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .cloned()
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let raw_limit = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        if !raw_limit.is_finite() || raw_limit < 0.0 {
            bail!("Apify run returned invalid charging values");
        }
        let max_total_charge_usd = (raw_limit > 0.0).then_some(raw_limit);
        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        let budget = Self {
            is_pay_per_event: true,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        };
        budget.total_charged_usd()?;
        Ok(budget)
    }

    fn remaining_charges(&self, event_name: &str, requested: usize) -> Result<Option<usize>> {
        if !self.is_pay_per_event {
            return Ok(None);
        }
        let Some(limit) = self.max_total_charge_usd else {
            return Ok(Some(usize::MAX));
        };
        let event_price = self.event_price(event_name)?;
        if event_price == 0.0 {
            return Ok(Some(usize::MAX));
        }
        Ok(Some(self.affordable_count(
            limit,
            event_price,
            requested,
        )?))
    }

    fn can_save_result(&self, requested: usize) -> Result<bool> {
        if !self.is_pay_per_event {
            return Ok(true);
        }
        let custom_price = self.event_price(SEARCH_CHARGE_EVENT)?;
        let dataset_price = self.event_price("apify-default-dataset-item")?;
        let price_per_result = custom_price + dataset_price;
        if !price_per_result.is_finite() || price_per_result < 0.0 {
            bail!("Apify run returned invalid charging values");
        }
        if price_per_result == 0.0 || self.max_total_charge_usd.is_none() {
            return Ok(requested > 0);
        }
        let limit = self.max_total_charge_usd.expect("checked above");
        Ok(self.affordable_count(limit, price_per_result, requested)? > 0)
    }

    fn record_dataset_item(&mut self) {
        self.increment_event("apify-default-dataset-item");
    }

    fn record_custom_charge(&mut self) {
        self.increment_event(SEARCH_CHARGE_EVENT);
    }

    fn increment_event(&mut self, event_name: &str) {
        let current = self
            .charged_event_counts
            .get(event_name)
            .and_then(Value::as_u64)
            .unwrap_or(0);
        self.charged_event_counts.insert(
            event_name.to_owned(),
            Value::from(current.saturating_add(1)),
        );
    }

    fn has_custom_charge_event(&self) -> bool {
        self.event_prices.contains_key(SEARCH_CHARGE_EVENT)
    }

    fn event_price(&self, event_name: &str) -> Result<f64> {
        let Some(event) = self.event_prices.get(event_name) else {
            return Ok(0.0);
        };
        let price = event
            .get("eventPriceUsd")
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Apify run did not provide the price for {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Apify run returned invalid charging values");
        }
        Ok(price)
    }

    fn total_charged_usd(&self) -> Result<f64> {
        let mut total = 0.0;
        for (event_name, count) in &self.charged_event_counts {
            let count = count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            if count == 0 {
                continue;
            }
            total += self.event_price(event_name)? * count as f64;
        }
        if !total.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }
        Ok((total * 1_000_000.0).round() / 1_000_000.0)
    }

    fn affordable_count(&self, limit: f64, item_price: f64, requested: usize) -> Result<usize> {
        if item_price == 0.0 {
            return Ok(requested);
        }
        let remaining = limit - self.total_charged_usd()?;
        if remaining <= 0.0 {
            return Ok(0);
        }
        let unrounded_count = remaining / item_price;
        let rounded_count = (unrounded_count * 10_000.0).round() / 10_000.0;
        Ok((rounded_count.floor() as usize).min(requested))
    }
}

async fn get_input(client: &Client, config: &ActorConfig) -> Result<Option<Value>> {
    let url = apify_url(
        &config.apify_api_base_url,
        &[
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

async fn get_run_budget(client: &Client, config: &ActorConfig) -> Result<PpeBudget> {
    let url = apify_url(
        &config.apify_api_base_url,
        &["actor-runs", &config.actor_run_id],
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
    PpeBudget::from_run(&run)
}

async fn push_dataset_item(client: &Client, config: &ActorConfig, item: &Value) -> Result<()> {
    let url = apify_url(
        &config.apify_api_base_url,
        &["datasets", &config.dataset_id, "items"],
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

async fn charge_search_result(
    client: &Client,
    config: &ActorConfig,
    charge_index: usize,
    retry_policy: RetryPolicy,
) -> Result<()> {
    let url = apify_url(
        &config.apify_api_base_url,
        &["actor-runs", &config.actor_run_id, "charge"],
    )?;
    let idempotency_key = format!(
        "{}-{SEARCH_CHARGE_EVENT}-{charge_index}",
        config.actor_run_id
    );
    let attempts = retry_policy.attempts.max(1);

    for attempt in 1..=attempts {
        let response = client
            .post(url.clone())
            .bearer_auth(&config.apify_token)
            .header("idempotency-key", idempotency_key.as_str())
            .json(&json!({"eventName": SEARCH_CHARGE_EVENT, "count": 1}))
            .send()
            .await;

        match response {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(response) => {
                let status = response.status();
                if attempt == attempts || !is_retryable_charge_status(status) {
                    return require_apify_success(response, "result charge")
                        .await
                        .map(|_| ());
                }
                eprintln!(
                    "Apify result charge failed with HTTP {}; retrying attempt {}/{}.",
                    status.as_u16(),
                    attempt + 1,
                    attempts
                );
            }
            Err(error) => {
                let retryable = error.is_timeout() || error.is_connect();
                if attempt == attempts || !retryable {
                    return Err(error).context("Apify result charge request failed");
                }
                eprintln!(
                    "Apify result charge request failed ({error}); retrying attempt {}/{}.",
                    attempt + 1,
                    attempts
                );
            }
        }

        tokio::time::sleep(retry_policy.delay_after(attempt)).await;
    }

    unreachable!("at least one charge attempt is made")
}

fn is_retryable_charge_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
}

async fn put_output(client: &Client, config: &ActorConfig, output: &Value) -> Result<()> {
    put_key_value_record(client, config, OUTPUT_KEY, output, "OUTPUT write").await
}

async fn put_key_value_record(
    client: &Client,
    config: &ActorConfig,
    key: &str,
    value: &Value,
    operation: &str,
) -> Result<()> {
    let url = apify_url(
        &config.apify_api_base_url,
        &[
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            key,
        ],
    )?;
    let response = client
        .put(url)
        .bearer_auth(&config.apify_token)
        .json(value)
        .send()
        .await
        .with_context(|| format!("Apify {operation} request failed"))?;
    require_apify_success(response, operation).await?;
    Ok(())
}

async fn set_terminal_status(client: &Client, config: &ActorConfig, message: &str) -> Result<()> {
    let url = apify_url(
        &config.apify_api_base_url,
        &["actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .put(url)
        .bearer_auth(&config.apify_token)
        .json(&json!({
            "statusMessage": message,
            "isStatusMessageTerminal": true,
        }))
        .send()
        .await
        .context("Apify status message request failed")?;
    require_apify_success(response, "status message update").await?;
    Ok(())
}

async fn set_terminal_status_best_effort(client: &Client, config: &ActorConfig, message: &str) {
    if let Err(error) = set_terminal_status(client, config, message).await {
        eprintln!("Could not set terminal run status: {error:#}");
    }
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

async fn run_actor(
    client: &Client,
    scrappa_client: &ScrappaClient,
    config: &ActorConfig,
    retry_policy: RetryPolicy,
) -> Result<()> {
    let input = normalize_input(get_input(client, config).await?);
    validate_input(&input)?;
    let mut budget = get_run_budget(client, config).await?;
    let remaining_charges = budget.remaining_charges(SEARCH_CHARGE_EVENT, usize::MAX)?;
    if remaining_charges == Some(0) {
        let status_message = "Charge limit reached before calling Scrappa; no LinkedIn search results were requested.";
        println!("{status_message}");
        let mut output = output_summary(None, 0);
        output
            .as_object_mut()
            .expect("summary is an object")
            .insert(
                "status".to_owned(),
                Value::String(status_message.to_owned()),
            );
        put_output(client, config, &output).await?;
        set_terminal_status_best_effort(client, config, status_message).await;
        return Ok(());
    }

    let request_input = limit_result_count(&input, remaining_charges)?;
    let query = input.get("query").map(display_value).unwrap_or_default();
    println!("Searching LinkedIn for: \"{query}\"");

    let response = scrappa_client
        .search(
            &config.scrappa_api_base_url,
            &config.scrappa_api_key,
            &request_input,
            retry_policy,
        )
        .await
        .map_err(anyhow::Error::new)
        .map_err(|error| {
            if error
                .downcast_ref::<ScrappaError>()
                .is_some_and(ScrappaError::is_timeout)
            {
                anyhow!("{}", timeout_message(&error.to_string()))
            } else {
                error
            }
        })?;

    let results = organic_results(&response);
    let mut saved_results = 0;
    let mut status_message = None;

    if !results.is_empty() {
        for result in results {
            if budget.remaining_charges(SEARCH_CHARGE_EVENT, usize::MAX)? == Some(0)
                || !budget.can_save_result(1)?
            {
                let message = format!(
                    "Charge limit reached before saving the next LinkedIn search result; {saved_results} of {} result(s) were saved.",
                    results.len()
                );
                println!("{message}");
                status_message = Some(message);
                break;
            }

            push_dataset_item(client, config, result).await?;
            if budget.is_pay_per_event {
                budget.record_dataset_item();
            }
            saved_results += 1;
            if budget.is_pay_per_event && budget.has_custom_charge_event() {
                charge_search_result(client, config, saved_results, retry_policy).await?;
                budget.record_custom_charge();
            }
        }
        println!("Saved {saved_results} LinkedIn search result(s)");
    } else {
        println!("No LinkedIn search results found for the given search criteria");
    }

    let output = output_summary(Some(&response), saved_results);
    put_output(client, config, &output).await?;

    println!("LinkedIn search completed successfully");
    let summary = json!({
        "results": saved_results,
        "total_results": output.get("total_results").unwrap_or(&Value::Null),
        "current_page": output.get("current_page").unwrap_or(&Value::Null),
        "pages": output.get("pages").unwrap_or(&Value::Null),
    });
    println!("Results summary: {summary}");
    if let Some(status_message) = status_message {
        println!("[Status message]: {status_message}");
        set_terminal_status_best_effort(client, config, &status_message).await;
    }
    Ok(())
}

fn display_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

fn timeout_message(message: &str) -> String {
    format!(
        "{message}. The LinkedIn Search request exceeded the {}s Scrappa API timeout. Try again or refine the query.",
        REQUEST_TIMEOUT.as_secs()
    )
}

#[tokio::main]
async fn main() -> ExitCode {
    let config = match ActorConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            return ExitCode::FAILURE;
        }
    };

    let apify_client = match Client::builder().timeout(REQUEST_TIMEOUT).build() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Actor failed: Could not create Apify HTTP client: {error}");
            return ExitCode::FAILURE;
        }
    };
    let scrappa_client = match ScrappaClient::new(REQUEST_TIMEOUT) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Actor failed: Could not create Scrappa HTTP client: {error}");
            return ExitCode::FAILURE;
        }
    };

    match run_actor(
        &apify_client,
        &scrappa_client,
        &config,
        RetryPolicy::default(),
    )
    .await
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            if let Err(status_error) = set_terminal_status(&apify_client, &config, &message).await {
                eprintln!("Could not set terminal run status: {status_error:#}");
            }
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
        time::Duration,
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
            let (request_tx, requests) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = std::time::Instant::now() + Duration::from_secs(10);
                let mut responses = responses.into_iter();
                while !stopped.load(Ordering::Relaxed) && std::time::Instant::now() < deadline {
                    let (mut stream, _) = match listener.accept() {
                        Ok(connection) => connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                            continue;
                        }
                        Err(_) => break,
                    };
                    let request = read_request(&mut stream).unwrap_or_default();
                    if request_tx.send(request).is_err() {
                        break;
                    }
                    let Some(response) = responses.next() else {
                        break;
                    };
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        204 => "No Content",
                        400 => "Bad Request",
                        404 => "Not Found",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
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
        let mut request = Vec::new();
        let mut buffer = [0u8; 1024];
        let mut header_end = None;
        let mut expected_len = 0usize;
        loop {
            let bytes_read = stream.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..bytes_read]);
            if header_end.is_none() {
                if let Some(position) = request.windows(4).position(|window| window == b"\r\n\r\n")
                {
                    let end = position + 4;
                    let headers = String::from_utf8_lossy(&request[..end]);
                    expected_len = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                    header_end = Some(end);
                }
            }
            if header_end.is_some_and(|end| request.len() >= end + expected_len) {
                break;
            }
        }
        Ok(String::from_utf8_lossy(&request).into_owned())
    }

    fn response(status: u16, body: impl Into<String>) -> MockResponse {
        MockResponse {
            status,
            body: body.into(),
        }
    }

    fn idempotency_key(request: &str) -> &str {
        request
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("idempotency-key")
                    .then_some(value.trim())
            })
            .expect("charge request includes an idempotency key")
    }

    fn test_config(base_url: Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: base_url,
            key_value_store_id: "store-1".to_owned(),
            input_key: INPUT_KEY.to_owned(),
            dataset_id: "dataset-1".to_owned(),
            actor_run_id: "run-1".to_owned(),
            apify_token: "apify-test-token".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
        }
    }

    fn run_response(pricing_model: &str, max_charge: f64, charged_results: u64) -> String {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": pricing_model,
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "linkedin-search-result": {"eventPriceUsd": 0.0003},
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0}
                        }
                    }
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": {"linkedin-search-result": charged_results}
            }
        })
        .to_string()
    }

    #[tokio::test]
    async fn run_fetches_search_saves_each_result_charges_custom_event_and_writes_output() {
        let server = MockServer::start(vec![
            response(
                200,
                r#"{"query":" site:linkedin.com/in founder AI Berlin ","num":2}"#,
            ),
            response(200, run_response("PAY_PER_EVENT", 0.01, 0)),
            response(
                200,
                r#"{"organic_results":[{"position":1,"title":"Founder","link":"https://www.linkedin.com/in/founder"},{"position":2,"title":"CTO","link":"https://www.linkedin.com/in/cto"}],"total_results":40,"search_information":{"query_displayed":"founder","total_results":40},"pagination":{"current_page":1,"pages":[{"page":1},{"page":2}]}}"#,
            ),
            response(201, "{}"),
            response(200, "{}"),
            response(201, "{}"),
            response(200, "{}"),
            response(200, "{}"),
        ]);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let scrappa_client = ScrappaClient::new(Duration::from_secs(2)).unwrap();
        let config = test_config(server.base_url.clone());

        run_actor(
            &client,
            &scrappa_client,
            &config,
            RetryPolicy {
                attempts: 1,
                ..RetryPolicy::default()
            },
        )
        .await
        .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 8);
        assert!(requests[0].starts_with("GET /v2/key-value-stores/store-1/records/INPUT HTTP/1.1"));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer apify-test-token"));
        assert!(requests[1].starts_with("GET /v2/actor-runs/run-1 HTTP/1.1"));
        assert!(requests[2].starts_with("GET /linkedin/search?query=site%3Alinkedin.com%2Fin+founder+AI+Berlin&num=2&hl=en&gl=us&safe=off HTTP/1.1"));
        assert!(requests[2]
            .to_ascii_lowercase()
            .contains("x-api-key: scrappa-test-key"));
        assert!(requests[2].contains("thescrappa-linkedin-search-scraper/1.0"));
        assert!(requests[3].starts_with("POST /v2/datasets/dataset-1/items HTTP/1.1"));
        assert!(requests[3].contains("\"title\":\"Founder\""));
        assert!(requests[4].starts_with("POST /v2/actor-runs/run-1/charge HTTP/1.1"));
        assert!(requests[4].contains("\"eventName\":\"linkedin-search-result\""));
        assert!(requests[4].contains("\"count\":1"));
        assert_eq!(
            idempotency_key(&requests[4]),
            "run-1-linkedin-search-result-1"
        );
        assert!(requests[5].starts_with("POST /v2/datasets/dataset-1/items HTTP/1.1"));
        assert!(requests[5].contains("\"title\":\"CTO\""));
        assert!(requests[6].starts_with("POST /v2/actor-runs/run-1/charge HTTP/1.1"));
        assert_eq!(
            idempotency_key(&requests[6]),
            "run-1-linkedin-search-result-2"
        );
        assert!(requests[7].starts_with("PUT /v2/key-value-stores/store-1/records/OUTPUT HTTP/1.1"));
        assert!(requests[7].contains("\"results\":2"));
        assert!(requests[7].contains("\"current_page\":1"));
        assert!(requests[7].contains("\"pages\":2"));
    }

    #[tokio::test]
    async fn retries_result_charge_with_the_same_idempotency_key_after_writing_dataset_item() {
        let server = MockServer::start(vec![
            response(200, r#"{"query":"cto"}"#),
            response(200, run_response("PAY_PER_EVENT", 0.01, 0)),
            response(200, r#"{"organic_results":[{"position":1,"title":"CTO"}]}"#),
            response(201, "{}"),
            response(503, "temporarily unavailable"),
            response(200, "{}"),
            response(200, "{}"),
        ]);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let scrappa_client = ScrappaClient::new(Duration::from_secs(2)).unwrap();
        let config = test_config(server.base_url.clone());

        run_actor(
            &client,
            &scrappa_client,
            &config,
            RetryPolicy {
                attempts: 2,
                base_delay: Duration::ZERO,
                max_delay: Duration::ZERO,
                jitter_max_ms: 0,
            },
        )
        .await
        .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 7);
        assert!(requests[3].starts_with("POST /v2/datasets/dataset-1/items HTTP/1.1"));
        assert!(requests[3].contains("\"title\":\"CTO\""));
        assert!(requests[4].starts_with("POST /v2/actor-runs/run-1/charge HTTP/1.1"));
        assert!(requests[5].starts_with("POST /v2/actor-runs/run-1/charge HTTP/1.1"));
        assert_eq!(idempotency_key(&requests[4]), idempotency_key(&requests[5]));
        assert_eq!(
            idempotency_key(&requests[4]),
            "run-1-linkedin-search-result-1"
        );
        assert!(requests[6].starts_with("PUT /v2/key-value-stores/store-1/records/OUTPUT HTTP/1.1"));
    }

    #[tokio::test]
    async fn fails_actor_when_result_charge_fails_after_saving_dataset_item() {
        let server = MockServer::start(vec![
            response(200, r#"{"query":"cto"}"#),
            response(200, run_response("PAY_PER_EVENT", 0.01, 0)),
            response(200, r#"{"organic_results":[{"position":1,"title":"CTO"}]}"#),
            response(201, "{}"),
            response(500, "charge failed"),
        ]);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let scrappa_client = ScrappaClient::new(Duration::from_secs(2)).unwrap();
        let config = test_config(server.base_url.clone());

        let error = run_actor(
            &client,
            &scrappa_client,
            &config,
            RetryPolicy {
                attempts: 1,
                ..RetryPolicy::default()
            },
        )
        .await
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("Apify result charge failed with 500"));
        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert!(requests[3].starts_with("POST /v2/datasets/dataset-1/items HTTP/1.1"));
        assert!(requests[3].contains("\"title\":\"CTO\""));
        assert!(requests[4].starts_with("POST /v2/actor-runs/run-1/charge HTTP/1.1"));
        assert!(!requests.iter().any(|request| request
            .starts_with("PUT /v2/key-value-stores/store-1/records/OUTPUT HTTP/1.1")));
    }

    #[tokio::test]
    async fn does_not_charge_when_dataset_write_fails() {
        let server = MockServer::start(vec![
            response(200, r#"{"query":"cto"}"#),
            response(200, run_response("PAY_PER_EVENT", 0.01, 0)),
            response(200, r#"{"organic_results":[{"position":1,"title":"CTO"}]}"#),
            response(500, "dataset failed"),
        ]);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let scrappa_client = ScrappaClient::new(Duration::from_secs(2)).unwrap();
        let config = test_config(server.base_url.clone());

        let error = run_actor(
            &client,
            &scrappa_client,
            &config,
            RetryPolicy {
                attempts: 1,
                ..RetryPolicy::default()
            },
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("Apify dataset write failed"));
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests[3].starts_with("POST /v2/datasets/dataset-1/items HTTP/1.1"));
        assert!(!requests.iter().any(|request| {
            request.starts_with("POST /v2/actor-runs/run-1/charge HTTP/1.1")
                || request.starts_with("PUT /v2/key-value-stores/store-1/records/OUTPUT HTTP/1.1")
        }));
    }

    #[tokio::test]
    async fn stops_before_scrappa_and_writes_output_when_ppe_budget_is_empty() {
        let server = MockServer::start(vec![
            response(200, "{}"),
            response(200, run_response("PAY_PER_EVENT", 0.0003, 1)),
            response(200, "{}"),
            response(200, "{}"),
        ]);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let scrappa_client = ScrappaClient::new(Duration::from_secs(2)).unwrap();
        let config = test_config(server.base_url.clone());

        run_actor(
            &client,
            &scrappa_client,
            &config,
            RetryPolicy {
                attempts: 1,
                ..RetryPolicy::default()
            },
        )
        .await
        .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests[2].starts_with("PUT /v2/key-value-stores/store-1/records/OUTPUT HTTP/1.1"));
        assert!(requests[2].contains("Charge limit reached before calling Scrappa"));
        assert!(requests[3].starts_with("PUT /v2/actor-runs/run-1 HTTP/1.1"));
        assert!(requests[3].contains("\"isStatusMessageTerminal\":true"));
    }

    #[tokio::test]
    async fn retries_retryable_scrappa_status_then_saves_results() {
        let server = MockServer::start(vec![
            response(200, r#"{"query":"cto"}"#),
            response(200, run_response("NOT_PAY_PER_EVENT", 0.0, 0)),
            response(503, "temporarily unavailable"),
            response(200, r#"{"organic_results":[]}"#),
            response(200, "{}"),
        ]);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let scrappa_client = ScrappaClient::new(Duration::from_secs(2)).unwrap();
        let config = test_config(server.base_url.clone());

        run_actor(
            &client,
            &scrappa_client,
            &config,
            RetryPolicy {
                attempts: 2,
                base_delay: Duration::ZERO,
                max_delay: Duration::ZERO,
                jitter_max_ms: 0,
            },
        )
        .await
        .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert!(requests[2]
            .starts_with("GET /linkedin/search?query=cto&num=10&hl=en&gl=us&safe=off HTTP/1.1"));
        assert!(requests[3]
            .starts_with("GET /linkedin/search?query=cto&num=10&hl=en&gl=us&safe=off HTTP/1.1"));
        assert!(requests[4].starts_with("PUT /v2/key-value-stores/store-1/records/OUTPUT HTTP/1.1"));
    }

    #[test]
    fn ppe_budget_accounts_for_spend_from_other_events_and_dataset_items() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "linkedin-search-result": {"eventPriceUsd": 0.30},
                        "other-event": {"eventPriceUsd": 0.20},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.10}
                    }}
                },
                "options": {"maxTotalChargeUsd": 1.00},
                "chargedEventCounts": {"other-event": 2, "apify-default-dataset-item": 1}
            }
        });
        let mut budget = PpeBudget::from_run(&run).unwrap();
        assert_eq!(
            budget.remaining_charges(SEARCH_CHARGE_EVENT, 20).unwrap(),
            Some(1)
        );
        assert!(budget.can_save_result(1).unwrap());
        budget.record_dataset_item();
        budget.record_custom_charge();
        assert_eq!(
            budget.remaining_charges(SEARCH_CHARGE_EVENT, 20).unwrap(),
            Some(0)
        );
        assert!(!budget.can_save_result(1).unwrap());
    }

    #[test]
    fn ppe_budget_rounds_decimal_event_counts_before_capping() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "linkedin-search-result": {"eventPriceUsd": 0.10}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.30},
                "chargedEventCounts": {}
            }
        });
        let budget = PpeBudget::from_run(&run).unwrap();
        assert_eq!(
            budget.remaining_charges(SEARCH_CHARGE_EVENT, 20).unwrap(),
            Some(3)
        );
    }

    #[test]
    fn reports_timeout_with_original_actor_context() {
        assert_eq!(
            timeout_message("Scrappa API request timed out after 60000ms"),
            "Scrappa API request timed out after 60000ms. The LinkedIn Search request exceeded the 60s Scrappa API timeout. Try again or refine the query."
        );
    }
}
