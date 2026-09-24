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
        let max_total_charge_usd = match data.pointer("/options/maxTotalChargeUsd") {
            None | Some(Value::Null) => f64::INFINITY,
            Some(value) => {
                let amount = value
                    .as_f64()
                    .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
                if amount == 0.0 {
                    f64::INFINITY
                } else if amount.is_finite() && amount > 0.0 {
                    amount
                } else {
                    bail!("Apify run returned invalid charging values");
                }
            }
        };
        if !dataset_item_price_usd.is_finite() || dataset_item_price_usd < 0.0 {
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
        if self.dataset_item_price_usd == 0.0 || self.max_total_charge_usd.is_infinite() {
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
mod tests;
