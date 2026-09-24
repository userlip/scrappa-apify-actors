use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use std::{collections::HashSet, env, fmt, process, time::Duration};
use tokio::task::JoinSet;
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_USERNAMES: usize = 100;
const REQUEST_CONCURRENCY: usize = 5;
const APIFY_MAX_RETRIES: usize = 2;

struct ActorConfig {
    apify_api_base: Url,
    scrappa_api_base: Url,
    apify_token: String,
    scrappa_api_key: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
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
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
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

fn normalize_username(username: &str) -> String {
    username.trim().trim_start_matches('@').to_owned()
}

fn valid_username(username: &str) -> bool {
    !username.is_empty()
        && username.len() <= 30
        && username.bytes().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, b'.' | b'_' | b'-')
        })
}

fn get_usernames(input: &Value) -> Result<Vec<String>> {
    let mut candidates = Vec::new();
    if let Some(usernames) = input.get("usernames").and_then(Value::as_array) {
        candidates.extend(usernames.iter());
    }
    if let Some(username) = input.get("username").filter(|value| value.is_string()) {
        candidates.push(username);
    }

    let mut usernames = Vec::new();
    let mut seen = HashSet::new();
    for candidate in candidates {
        let Some(candidate) = candidate.as_str() else {
            bail!("Each Instagram username must be a string.");
        };
        let username = normalize_username(candidate);
        if !valid_username(&username) {
            bail!("Invalid Instagram username: {}", json!(candidate));
        }
        if seen.insert(username.to_ascii_lowercase()) {
            usernames.push(username);
        }
    }

    if usernames.is_empty() {
        bail!("At least one Instagram username is required. Provide usernames (recommended) or username.");
    }
    if usernames.len() > MAX_USERNAMES {
        bail!("A maximum of {MAX_USERNAMES} unique Instagram usernames can be processed per run.");
    }
    Ok(usernames)
}

fn flatten_profile(response: &Value) -> Value {
    let user = response
        .get("user")
        .filter(|user| !user.is_null())
        .or_else(|| response.get("data").and_then(|data| data.get("user")))
        .filter(|user| !user.is_null())
        .or_else(|| response.get("data"))
        .filter(|data| !data.is_null())
        .unwrap_or(response);
    let Some(user) = user.as_object() else {
        return response.clone();
    };

    let mut flattened = response.as_object().cloned().unwrap_or_default();
    flattened.extend(user.clone());
    Value::Object(flattened)
}

fn object_spread(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(object) => object,
        Value::Array(values) => values
            .into_iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value))
            .collect(),
        Value::String(value) => value
            .chars()
            .enumerate()
            .map(|(index, character)| (index.to_string(), json!(character.to_string())))
            .collect(),
        _ => Map::new(),
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::Null => String::new(),
                value => js_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn response_message(data: &Value) -> Value {
    data.get("message")
        .filter(|message| !message.is_null())
        .or_else(|| data.get("error").filter(|error| !error.is_null()))
        .cloned()
        .unwrap_or_else(|| json!("Unknown Scrappa API error"))
}

fn is_authentication_failure(status: u16, data: &Value) -> bool {
    let code = data
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let message = js_string(&response_message(data)).to_ascii_lowercase();
    status == 401
        || status == 403
        || code.contains("unauthorized")
        || code.contains("forbidden")
        || message.contains("authentication required")
        || message.contains("unauthorized")
        || message.contains("invalid api key")
        || message.contains("forbidden")
}

fn parse_response_body(body: &str, status: u16) -> Value {
    if body.is_empty() {
        return json!({ "message": format!("HTTP {status}") });
    }
    serde_json::from_str(body).unwrap_or_else(|_| json!({ "message": body }))
}

#[derive(Debug)]
enum InstagramUserError {
    Authentication(String),
    Lookup(String),
}

impl fmt::Display for InstagramUserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authentication(message) | Self::Lookup(message) => formatter.write_str(message),
        }
    }
}

#[derive(Clone)]
struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
}

impl ScrappaClient {
    fn new(http: Client, base_url: Url, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    async fn fetch_user(&self, username: &str) -> std::result::Result<Value, InstagramUserError> {
        let mut url = endpoint_url(&self.base_url, &["instagram", "user"])
            .map_err(|error| InstagramUserError::Lookup(error.to_string()))?;
        url.query_pairs_mut().append_pair("username", username);

        let response = self
            .http
            .get(url)
            .header("X-API-KEY", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    InstagramUserError::Lookup(format!(
                        "Scrappa API request timed out after {}ms",
                        REQUEST_TIMEOUT.as_millis()
                    ))
                } else {
                    InstagramUserError::Lookup(format!("Scrappa API request failed: {error}"))
                }
            })?;

        let status = response.status().as_u16();
        let body = response.text().await.map_err(|error| {
            InstagramUserError::Lookup(format!("Scrappa API response could not be read: {error}"))
        })?;
        let data = parse_response_body(&body, status);
        let message = js_string(&response_message(&data));

        if is_authentication_failure(status, &data) {
            return Err(InstagramUserError::Authentication(format!(
                "Scrappa API authentication failed: {message}. Check the SCRAPPA_API_KEY Actor secret."
            )));
        }
        if status >= 400 || data.get("success") == Some(&Value::Bool(false)) {
            let status_prefix = if status >= 400 {
                format!("HTTP {status}")
            } else {
                "an error response".to_owned()
            };
            return Err(InstagramUserError::Lookup(format!(
                "Scrappa API returned {status_prefix}: {message}"
            )));
        }

        let mut item = object_spread(flatten_profile(&data));
        item.insert("input_username".to_owned(), json!(username));
        Ok(Value::Object(item))
    }
}

async fn fetch_wave(
    client: &ScrappaClient,
    usernames: &[String],
) -> Result<Vec<(String, std::result::Result<Value, InstagramUserError>)>> {
    let mut tasks = JoinSet::new();
    for (index, username) in usernames.iter().cloned().enumerate() {
        let client = client.clone();
        tasks.spawn(async move {
            let result = client.fetch_user(&username).await;
            (index, username, result)
        });
    }

    let mut results = (0..usernames.len()).map(|_| None).collect::<Vec<_>>();
    while let Some(task) = tasks.join_next().await {
        let (index, username, result) = task.context("Instagram user lookup task failed")?;
        results[index] = Some((username, result));
    }

    results
        .into_iter()
        .map(|result| {
            result.ok_or_else(|| anyhow!("Instagram user lookup task returned no result"))
        })
        .collect()
}

fn failure_item(username: &str, error: &InstagramUserError) -> Value {
    json!({
        "success": false,
        "input_username": username,
        "username": username,
        "error": error.to_string(),
    })
}

struct ApifyClient {
    http: Client,
    base_url: Url,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    fn new(http: Client, config: &ActorConfig) -> Self {
        Self {
            http,
            base_url: config.apify_api_base.clone(),
            token: config.apify_token.clone(),
            actor_run_id: config.actor_run_id.clone(),
            key_value_store_id: config.key_value_store_id.clone(),
            dataset_id: config.dataset_id.clone(),
            input_key: config.input_key.clone(),
        }
    }

    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.base_url, segments)
    }

    async fn get_input(&self) -> Result<Value> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
                .context("Apify INPUT request failed")?;
            if response.status() == StatusCode::NOT_FOUND {
                return Ok(Value::Null);
            }
            if let Some(delay) = apify_retry_delay("GET", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }
            return response_json(response, "Apify INPUT request").await;
        }
    }

    async fn run_dataset_budget(&self) -> Result<DatasetBudget> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
                .context("Apify run pricing request failed")?;
            if let Some(delay) = apify_retry_delay("GET", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }
            let run = response_json(response, "Apify run pricing request").await?;
            return DatasetBudget::from_run(&run);
        }
    }

    async fn push_dataset_items(
        &self,
        budget: &mut Option<DatasetBudget>,
        items: &[Value],
    ) -> Result<usize> {
        if items.is_empty() {
            return Ok(0);
        }
        if budget.is_none() {
            *budget = Some(self.run_dataset_budget().await?);
        }
        let dataset_budget = budget
            .as_mut()
            .ok_or_else(|| anyhow!("Apify dataset pricing budget was not initialized"))?;
        let allowed = dataset_budget.affordable_items(items.len());
        if allowed == 0 {
            return Ok(0);
        }

        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(&items[..allowed])
            .send()
            .await
            .context("Apify dataset write failed")?;
        require_apify_success(response, "dataset write").await?;
        dataset_budget.record_saved_items(allowed);
        Ok(allowed)
    }
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    if !response.status().is_success() {
        let status = response.status();
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

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

fn apify_retry_delay(method: &str, status: StatusCode, retry_count: usize) -> Option<Duration> {
    if !matches!(method, "GET" | "PUT")
        || retry_count >= APIFY_MAX_RETRIES
        || (status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error())
    {
        return None;
    }
    Some(Duration::from_secs((retry_count + 1) as u64))
}

struct DatasetBudget {
    dataset_item_price_usd: f64,
    charged_usd: f64,
    max_total_charge_usd: f64,
    saved_items: usize,
}

impl DatasetBudget {
    fn from_run(run: &Value) -> Result<Self> {
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
        let dataset_item_price_usd = events
            .get("apify-default-dataset-item")
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

        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_usd = 0.0;
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
            charged_usd += price * count as f64;
        }
        if !charged_usd.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        Ok(Self {
            dataset_item_price_usd,
            charged_usd,
            max_total_charge_usd,
            saved_items: 0,
        })
    }

    fn affordable_items(&self, requested: usize) -> usize {
        if self.dataset_item_price_usd == 0.0 {
            return requested;
        }
        let tolerance = f64::EPSILON * self.max_total_charge_usd.max(1.0);
        (1..=requested)
            .take_while(|count| {
                self.saved_items
                    .checked_add(*count)
                    .is_some_and(|total_items| {
                        self.charged_usd + total_items as f64 * self.dataset_item_price_usd
                            <= self.max_total_charge_usd + tolerance
                    })
            })
            .count()
    }

    fn record_saved_items(&mut self, count: usize) {
        self.saved_items = self.saved_items.saturating_add(count);
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct BatchSummary {
    requested: usize,
    succeeded: usize,
    failed: usize,
    saved: usize,
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<BatchSummary> {
    let apify = ApifyClient::new(client.clone(), config);
    let input = apify.get_input().await?;
    let usernames = get_usernames(&input)?;
    let scrappa = ScrappaClient::new(
        client.clone(),
        config.scrappa_api_base.clone(),
        config.scrappa_api_key.clone(),
    );
    let mut budget = None;
    let mut summary = BatchSummary {
        requested: usernames.len(),
        ..BatchSummary::default()
    };

    println!(
        "Fetching Instagram user info for {} username{}.",
        usernames.len(),
        if usernames.len() == 1 { "" } else { "s" }
    );

    for batch in usernames.chunks(REQUEST_CONCURRENCY) {
        let fetched = fetch_wave(&scrappa, batch).await?;
        let mut items = Vec::with_capacity(fetched.len());
        let mut authentication_error = None;

        for (username, result) in fetched {
            match result {
                Ok(item) => {
                    summary.succeeded += 1;
                    items.push(item);
                }
                Err(InstagramUserError::Authentication(message)) => {
                    if authentication_error.is_none() {
                        authentication_error = Some(message);
                    }
                }
                Err(error) => {
                    summary.failed += 1;
                    eprintln!("Instagram user lookup failed for {username}: {error}");
                    items.push(failure_item(&username, &error));
                }
            }
        }

        if let Some(message) = authentication_error {
            bail!(message);
        }

        let saved = apify
            .push_dataset_items(&mut budget, &items)
            .await
            .with_context(|| {
                format!(
                    "Could not save Instagram results for {} username(s)",
                    batch.len()
                )
            })?;
        summary.saved += saved;
    }

    println!(
        "Instagram user batch completed: {} succeeded, {} failed, {} dataset item(s) saved.",
        summary.succeeded, summary.failed, summary.saved
    );
    Ok(summary)
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::builder()
        .build()
        .context("Could not create HTTP client")?;
    run_actor(&client, &config).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        thread::{self, JoinHandle},
    };

    #[derive(Clone, Debug)]
    struct RecordedRequest {
        method: String,
        path: String,
        headers: HashMap<String, String>,
        body: String,
    }

    struct MockResponse {
        status: u16,
        body: String,
    }

    impl MockResponse {
        fn json(status: u16, body: Value) -> Self {
            Self {
                status,
                body: body.to_string(),
            }
        }
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
            let base_url = Url::parse(&format!("http://{address}/")).unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let requests_for_thread = Arc::clone(&requests);
            let stop = Arc::new(AtomicBool::new(false));
            let stop_for_thread = Arc::clone(&stop);
            let handler = Arc::new(handler);
            let thread = thread::spawn(move || {
                while !stop_for_thread.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                            let request = read_request(&mut stream).unwrap();
                            requests_for_thread.lock().unwrap().push(request.clone());
                            let response = handler(&request);
                            write_response(&mut stream, response).unwrap();
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => panic!("Mock server accept failed: {error}"),
                    }
                }
            });

            Self {
                base_url,
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
                thread.join().unwrap();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<RecordedRequest> {
        let mut bytes = Vec::new();
        let mut header_end = None;
        let mut content_length = 0;
        loop {
            let mut chunk = [0_u8; 4096];
            let read = stream.read(&mut chunk)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..read]);
            if header_end.is_none() {
                if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    header_end = Some(index);
                    let headers = String::from_utf8_lossy(&bytes[..index]);
                    content_length = headers
                        .lines()
                        .skip(1)
                        .filter_map(|line| line.split_once(':'))
                        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                }
            }
            if header_end.is_some_and(|index| bytes.len() >= index + 4 + content_length) {
                break;
            }
        }

        let header_end = header_end.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "HTTP request had no headers",
            )
        })?;
        let headers_text = String::from_utf8_lossy(&bytes[..header_end]);
        let mut lines = headers_text.lines();
        let request_line = lines.next().unwrap_or_default();
        let mut request_parts = request_line.split_whitespace();
        let method = request_parts.next().unwrap_or_default().to_owned();
        let path = request_parts.next().unwrap_or_default().to_owned();
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        let body_start = header_end + 4;
        let body_end = (body_start + content_length).min(bytes.len());
        let body = String::from_utf8_lossy(&bytes[body_start..body_end]).to_string();

        Ok(RecordedRequest {
            method,
            path,
            headers,
            body,
        })
    }

    fn write_response(stream: &mut TcpStream, response: MockResponse) -> std::io::Result<()> {
        let reason = match response.status {
            200 => "OK",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Response",
        };
        write!(
            stream,
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response.status,
            reason,
            response.body.len(),
            response.body
        )?;
        stream.flush()
    }

    fn config(apify_api_base: Url, scrappa_api_base: Url) -> ActorConfig {
        ActorConfig {
            apify_api_base,
            scrappa_api_base,
            apify_token: "apify-test-token".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
            actor_run_id: "test-run".to_owned(),
            key_value_store_id: "test-store".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            input_key: "INPUT".to_owned(),
        }
    }

    fn pricing_response(max_charge: f64, charged_events: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "apify-default-dataset-item": { "eventPriceUsd": 0.0002 },
                        "apify-actor-start": { "eventPriceUsd": 0.00005 }
                    }}
                },
                "options": { "maxTotalChargeUsd": max_charge },
                "chargedEventCounts": charged_events
            }
        })
    }

    fn mock_apify(input: Value, max_charge: f64, charged_events: Value) -> MockServer {
        MockServer::start(move |request| {
            if request.path.ends_with("/records/INPUT") {
                return MockResponse::json(200, input.clone());
            }
            if request.path.ends_with("/actor-runs/test-run") {
                return MockResponse::json(
                    200,
                    pricing_response(max_charge, charged_events.clone()),
                );
            }
            MockResponse::json(200, json!({ "ok": true }))
        })
    }

    fn api_base(base_url: &Url) -> Url {
        let mut url = base_url.clone();
        url.path_segments_mut().unwrap().push("api");
        url
    }

    #[test]
    fn get_usernames_normalizes_legacy_input_and_deduplicates_case_insensitively() {
        assert_eq!(
            get_usernames(&json!({
                "usernames": ["@NatGeo", "instagram", "natgeo"],
                "username": "@legacy"
            }))
            .unwrap(),
            ["NatGeo", "instagram", "legacy"]
        );
    }

    #[test]
    fn get_usernames_rejects_empty_invalid_non_string_and_oversized_batches() {
        assert!(get_usernames(&json!({}))
            .unwrap_err()
            .to_string()
            .contains("At least one"));
        assert!(get_usernames(&json!({ "usernames": ["invalid username"] }))
            .unwrap_err()
            .to_string()
            .contains("Invalid Instagram username"));
        assert!(get_usernames(&json!({ "usernames": [3] }))
            .unwrap_err()
            .to_string()
            .contains("must be a string"));
        let usernames = (0..=MAX_USERNAMES)
            .map(|index| format!("user{index}"))
            .collect::<Vec<_>>();
        assert!(get_usernames(&json!({ "usernames": usernames }))
            .unwrap_err()
            .to_string()
            .contains("maximum of 100"));
    }

    #[test]
    fn flatten_profile_merges_user_fields_and_falls_through_null_user_values() {
        assert_eq!(
            flatten_profile(&json!({
                "success": true,
                "user": null,
                "data": { "user": { "username": "natgeo", "follower_count": 5 } }
            })),
            json!({
                "success": true,
                "user": null,
                "data": { "user": { "username": "natgeo", "follower_count": 5 } },
                "username": "natgeo",
                "follower_count": 5
            })
        );
    }

    #[test]
    fn authentication_detection_matches_status_code_and_message_forms() {
        assert!(is_authentication_failure(
            401,
            &json!({ "message": "bad key" })
        ));
        assert!(is_authentication_failure(
            200,
            &json!({ "code": "FORBIDDEN_ACCESS" })
        ));
        assert!(is_authentication_failure(
            200,
            &json!({ "error": "Unauthorized" })
        ));
        assert!(!is_authentication_failure(
            500,
            &json!({ "message": "upstream down" })
        ));
    }

    #[test]
    fn apify_retry_policy_retries_only_get_and_put_transient_statuses() {
        assert_eq!(
            apify_retry_delay("GET", StatusCode::INTERNAL_SERVER_ERROR, 0),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            apify_retry_delay("PUT", StatusCode::TOO_MANY_REQUESTS, 1),
            Some(Duration::from_secs(2))
        );
        assert_eq!(apify_retry_delay("GET", StatusCode::BAD_REQUEST, 0), None);
        assert_eq!(
            apify_retry_delay("POST", StatusCode::INTERNAL_SERVER_ERROR, 0),
            None
        );
        assert_eq!(
            apify_retry_delay("GET", StatusCode::INTERNAL_SERVER_ERROR, APIFY_MAX_RETRIES),
            None
        );
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(90));
    }

    #[test]
    fn dataset_budget_includes_previously_charged_events() {
        let run = pricing_response(
            0.001,
            json!({ "apify-default-dataset-item": 1, "apify-actor-start": 1 }),
        );
        let budget = DatasetBudget::from_run(&run).unwrap();
        assert_eq!(budget.affordable_items(5), 3);
    }

    #[test]
    fn actor_metadata_keeps_prefill_and_minimal_memory_configuration() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let actor: Value = serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();
        assert_eq!(
            schema["properties"]["usernames"]["prefill"],
            json!(["natgeo", "instagram"])
        );
        assert_eq!(schema["properties"]["username"]["prefill"], "natgeo");
        assert_eq!(actor["defaultMemoryMbytes"], 128);
        assert_eq!(actor["defaultRunOptions"]["timeoutSecs"], 120);
    }

    #[tokio::test]
    async fn actor_preserves_auth_batch_output_and_failure_rows() {
        let input = json!({
            "usernames": ["@user0", "user1", "user2", "user3", "user4", "user5", "user6"]
        });
        let apify = mock_apify(input, 1.0, json!({}));
        let scrappa = MockServer::start(|request| {
            if request.path.contains("username=user1") {
                MockResponse::json(404, json!({ "message": "not found" }))
            } else {
                MockResponse::json(
                    200,
                    json!({ "success": true, "data": { "user": { "username": "profile", "follower_count": 5 } } }),
                )
            }
        });
        let config = config(apify.base_url.clone(), api_base(&scrappa.base_url));
        let summary = run_actor(&Client::new(), &config).await.unwrap();
        assert_eq!(
            summary,
            BatchSummary {
                requested: 7,
                succeeded: 6,
                failed: 1,
                saved: 7
            }
        );

        let apify_requests = apify.requests();
        let input_request = apify_requests
            .iter()
            .find(|request| request.path.ends_with("/records/INPUT"))
            .unwrap();
        assert_eq!(input_request.method, "GET");
        assert_eq!(
            input_request
                .headers
                .get("authorization")
                .map(String::as_str),
            Some("Bearer apify-test-token")
        );
        let dataset_writes = apify_requests
            .iter()
            .filter(|request| request.method == "POST" && request.path.ends_with("/items"))
            .collect::<Vec<_>>();
        assert_eq!(dataset_writes.len(), 2);
        let first_batch: Vec<Value> = serde_json::from_str(&dataset_writes[0].body).unwrap();
        let second_batch: Vec<Value> = serde_json::from_str(&dataset_writes[1].body).unwrap();
        assert_eq!((first_batch.len(), second_batch.len()), (5, 2));
        let saved_items = first_batch
            .into_iter()
            .chain(second_batch)
            .collect::<Vec<_>>();
        assert_eq!(saved_items.len(), 7);
        assert_eq!(saved_items[0]["input_username"], "user0");
        assert_eq!(saved_items[1]["success"], false);
        assert_eq!(saved_items[1]["username"], "user1");
        assert_eq!(
            saved_items[1]["error"],
            "Scrappa API returned HTTP 404: not found"
        );
        assert_eq!(saved_items[2]["username"], "profile");
        assert_eq!(saved_items[2]["follower_count"], 5);
        assert!(apify_requests
            .iter()
            .all(|request| !(request.method == "PUT" && request.path.contains("/records/OUTPUT"))));

        let scrappa_requests = scrappa.requests();
        assert_eq!(scrappa_requests.len(), 7);
        for request in scrappa_requests {
            assert_eq!(request.method, "GET");
            assert!(request.path.starts_with("/api/instagram/user?username="));
            assert_eq!(
                request.headers.get("x-api-key").map(String::as_str),
                Some("scrappa-test-key")
            );
            assert_eq!(
                request.headers.get("accept").map(String::as_str),
                Some("application/json")
            );
        }
    }

    #[tokio::test]
    async fn actor_writes_only_dataset_items_that_fit_the_remaining_ppe_budget() {
        let apify = mock_apify(
            json!({ "usernames": ["one", "two", "three", "four", "five"] }),
            0.001,
            json!({ "apify-default-dataset-item": 1, "apify-actor-start": 1 }),
        );
        let scrappa = MockServer::start(|_| {
            MockResponse::json(200, json!({ "user": { "username": "profile" } }))
        });
        let config = config(apify.base_url.clone(), api_base(&scrappa.base_url));

        let summary = run_actor(&Client::new(), &config).await.unwrap();
        assert_eq!(summary.saved, 3);
        let writes = apify
            .requests()
            .into_iter()
            .filter(|request| request.method == "POST" && request.path.ends_with("/items"))
            .collect::<Vec<_>>();
        assert_eq!(writes.len(), 1);
        let saved: Vec<Value> = serde_json::from_str(&writes[0].body).unwrap();
        assert_eq!(saved.len(), 3);
        assert_eq!(saved[0]["input_username"], "one");
        assert_eq!(saved[2]["input_username"], "three");
    }

    #[tokio::test]
    async fn authentication_failure_fails_without_writing_the_current_batch() {
        let apify = mock_apify(json!({ "usernames": ["good", "denied"] }), 1.0, json!({}));
        let scrappa = MockServer::start(|request| {
            if request.path.contains("username=denied") {
                MockResponse::json(401, json!({ "message": "Invalid API key" }))
            } else {
                MockResponse::json(200, json!({ "user": { "username": "good" } }))
            }
        });
        let config = config(apify.base_url.clone(), api_base(&scrappa.base_url));
        let error = run_actor(&Client::new(), &config).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("Scrappa API authentication failed: Invalid API key"));
        assert!(apify
            .requests()
            .iter()
            .all(|request| !(request.method == "POST" && request.path.ends_with("/items"))));
    }
}
