use std::{env, process::ExitCode, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{Map, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_millis(90_000);
const APIFY_MAX_RETRIES: usize = 2;
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

struct Config {
    apify_api_base: Url,
    scrappa_api_base: Url,
    apify_token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_key: Option<String>,
}

impl Config {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key: env::var("SCRAPPA_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
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

#[derive(Debug, PartialEq, Eq)]
struct ActorInput {
    username: String,
    max_id: String,
}

fn parse_input(input: &Value) -> Result<ActorInput> {
    let username = input
        .get("username")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .trim_start_matches('@')
        .to_owned();
    if username.is_empty() {
        bail!("Instagram username is required.");
    }

    let max_id = input
        .get("max_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    Ok(ActorInput { username, max_id })
}

fn build_scrappa_url(base_url: &Url, input: &ActorInput) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["instagram", "user", "posts"])?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("username", &input.username);
        if !input.max_id.is_empty() {
            query.append_pair("max_id", &input.max_id);
        }
    }
    Ok(url)
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
                value => js_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn response_message(data: &Value) -> String {
    data.get("message")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("error").filter(|value| !value.is_null()))
        .map(js_string)
        .unwrap_or_else(|| "Unknown Scrappa API error".to_owned())
}

fn is_authentication_failure(status: u16, data: &Value) -> bool {
    if matches!(status, 401 | 403) {
        return true;
    }
    let code = data
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let message = response_message(data).to_ascii_lowercase();
    [
        code.contains("unauthorized"),
        code.contains("forbidden"),
        message.contains("authentication required"),
        message.contains("unauthorized"),
        message.contains("invalid api key"),
        message.contains("forbidden"),
    ]
    .into_iter()
    .any(|match_found| match_found)
}

fn parse_response_body(body: &str, status: u16) -> (Value, bool) {
    if body.is_empty() {
        return (
            serde_json::json!({"message": format!("HTTP {status} with an empty response body")}),
            false,
        );
    }
    match serde_json::from_str(body) {
        Ok(data) => (data, true),
        Err(_) => (serde_json::json!({"message": body}), false),
    }
}

fn validate_scrappa_response(status: u16, body: &str) -> Result<Value> {
    let (data, is_json) = parse_response_body(body, status);
    let message = response_message(&data);
    if is_authentication_failure(status, &data) {
        bail!(
            "Scrappa API authentication failed: {message}. Check the SCRAPPA_API_KEY Actor secret."
        );
    }
    if status >= 500 {
        bail!("Scrappa API returned HTTP {status}: {message}");
    }
    if !is_json {
        bail!("Scrappa API returned a non-JSON response: {message}");
    }
    if status >= 400 || data.get("success") == Some(&Value::Bool(false)) {
        let status_prefix = if status >= 400 {
            format!("HTTP {status}")
        } else {
            "an error response".to_owned()
        };
        bail!("Scrappa API returned {status_prefix}: {message}");
    }
    Ok(data)
}

fn posts_from_response(response: &Value) -> Vec<Value> {
    response
        .get("posts")
        .and_then(Value::as_array)
        .or_else(|| response.pointer("/data/posts").and_then(Value::as_array))
        .or_else(|| response.get("data").and_then(Value::as_array))
        .cloned()
        .unwrap_or_default()
}

fn enrich_post(post: &Value, requested_username: &str) -> Value {
    let mut item = Map::new();
    item.insert(
        "request_username".to_owned(),
        Value::String(requested_username.to_owned()),
    );
    match post {
        Value::Object(fields) => item.extend(fields.clone()),
        Value::Array(values) => {
            item.extend(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| (index.to_string(), value.clone())),
            );
        }
        Value::String(value) => {
            item.extend(value.chars().enumerate().map(|(index, character)| {
                (index.to_string(), Value::String(character.to_string()))
            }));
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Value::Object(item)
}

fn affordable_dataset_items(run: &Value, requested: usize, already_saved: u64) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let pricing_model = data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Apify run did not provide the pricing model"))?;
    if pricing_model != "PAY_PER_EVENT" {
        return Ok(requested);
    }
    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = events
        .get(DATASET_ITEM_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    // Apify's charging.js treats zero, null, and missing limits as unbounded.
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let amount = value
                .as_f64()
                .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
            if !amount.is_finite() || amount < 0.0 {
                bail!("Apify run returned invalid charging values");
            }
            (amount > 0.0).then_some(amount)
        }
    };
    if !item_price.is_finite() || item_price < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    let mut saw_dataset_count = false;
    for (event_name, count) in counts {
        let mut count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if event_name == DATASET_ITEM_EVENT {
            saw_dataset_count = true;
            count = count.max(already_saved);
        }
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
    if !saw_dataset_count && already_saved > 0 {
        spent += item_price * already_saved as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    let Some(max_charge) = max_charge else {
        return Ok(requested);
    };
    if item_price == 0.0 {
        return Ok(requested);
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .count())
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

async fn apify_response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("Apify {operation} failed ({}): {body}", status.as_u16());
    }
    response
        .json::<Value>()
        .await
        .with_context(|| format!("Apify {operation} returned invalid JSON"))
}

async fn ensure_apify_success(response: Response, operation: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
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
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.config.apify_token)
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
            return apify_response_json(response, "INPUT request").await;
        }
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<usize> {
        if items.is_empty() {
            return Ok(0);
        }
        let capacity = self.run_dataset_capacity(items.len()).await?;
        if capacity == 0 {
            return Ok(0);
        }
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(&items[..capacity])
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_apify_success(response, "dataset write").await?;
        Ok(capacity)
    }

    async fn run_dataset_capacity(&self, requested: usize) -> Result<usize> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.config.apify_token)
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
            let run = apify_response_json(response, "run pricing request").await?;
            return affordable_dataset_items(&run, requested, 0);
        }
    }

    async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .put(url.clone())
                .bearer_auth(&self.config.apify_token)
                .header(header::ACCEPT, "application/json")
                .json(output)
                .send()
                .await
                .context("Apify OUTPUT write failed")?;
            if let Some(delay) = apify_retry_delay("PUT", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }
            return ensure_apify_success(response, "OUTPUT write").await;
        }
    }
}

async fn fetch_scrappa_response(
    http: &Client,
    config: &Config,
    input: &ActorInput,
    api_key: &str,
) -> Result<Value> {
    let url = build_scrappa_url(&config.scrappa_api_base, input)?;
    let response = http
        .get(url)
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .header("X-API-KEY", api_key)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(scrappa_request_error)?;
    let status = response.status().as_u16();
    let body = response.text().await.map_err(scrappa_request_error)?;
    validate_scrappa_response(status, &body)
}

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            SCRAPPA_REQUEST_TIMEOUT.as_millis()
        )
    } else {
        anyhow!(error.to_string())
    }
}

async fn run_actor(http: &Client, config: &Config) -> Result<()> {
    let apify = ApifyClient { http, config };
    let input = parse_input(&apify.get_input().await?)?;
    let api_key = config.scrappa_api_key.as_deref().ok_or_else(|| {
        anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
    })?;

    println!(
        "Fetching Instagram user posts for: {}{}",
        input.username,
        if input.max_id.is_empty() {
            String::new()
        } else {
            format!(" with max_id {}", input.max_id)
        }
    );

    let response = fetch_scrappa_response(http, config, &input, api_key).await?;
    let posts = posts_from_response(&response);
    let dataset_items = posts
        .iter()
        .map(|post| enrich_post(post, &input.username))
        .collect::<Vec<_>>();
    let saved_count = apify.push_dataset_items(&dataset_items).await?;
    apify.put_output(&response).await?;

    if posts.is_empty() {
        println!("No Instagram posts returned for: {}", input.username);
    }
    println!(
        "Successfully fetched {} Instagram posts for: {}; saved {} dataset item(s)",
        posts.len(),
        input.username,
        saved_count
    );
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let http = Client::builder()
        .build()
        .context("Could not create HTTP client")?;
    run_actor(&http, &config).await
}

#[cfg(test)]
mod tests;
