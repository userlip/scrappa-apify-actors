use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Value};
use std::{env, time::Duration};
use tokio::time::sleep;
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_MAX_RETRIES: u32 = 8;
const APIFY_MIN_RETRY_DELAY: Duration = Duration::from_millis(500);
const APIFY_MAX_RETRY_DELAY: Duration = Duration::from_secs(30);
const SCRAPPA_USER_AGENT: &str = "thescrappa-google-maps-search-scraper/1.0";
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const MAX_DATASET_REQUEST_BYTES: usize = 5_000_000;

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
    scrappa_api_key: String,
    max_total_charge_usd: Option<f64>,
    apify_request_timeout: Duration,
    scrappa_request_timeout: Duration,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env("SCRAPPA_API_KEY")?;
        let max_total_charge_usd = env::var("ACTOR_MAX_TOTAL_CHARGE_USD")
            .ok()
            .map(|value| {
                value
                    .parse::<f64>()
                    .context("ACTOR_MAX_TOTAL_CHARGE_USD must be a number")
            })
            .transpose()?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
            max_total_charge_usd,
            apify_request_timeout: APIFY_REQUEST_TIMEOUT,
            scrappa_request_timeout: SCRAPPA_REQUEST_TIMEOUT,
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
        .extend(segments.iter().copied());
    Ok(url)
}

fn required_query(input: &Value) -> Result<String> {
    input
        .get("query")
        .and_then(Value::as_str)
        .filter(|query| !query.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("Search query is required"))
}

fn js_string(value: &Value) -> String {
    match value {
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
        _ => value.to_string(),
    }
}

fn append_input_param(params: &mut Vec<(String, String)>, name: &str, value: Option<&Value>) {
    let Some(value) = value else {
        return;
    };
    if value.is_null() || value == &Value::Bool(false) {
        return;
    }
    if value == &Value::Bool(true) {
        params.push((name.to_owned(), "1".to_owned()));
        return;
    }

    let value = js_string(value);
    if !value.is_empty() {
        params.push((name.to_owned(), value));
    }
}

#[derive(Debug)]
struct SearchRequest {
    query: String,
    params: Vec<(String, String)>,
    fallback_zoom: String,
    debug: bool,
}

fn build_search_request(input: &Value) -> Result<SearchRequest> {
    let query = required_query(input)?;
    let mut params = vec![("query".to_owned(), query.clone())];

    let default_language = Value::String("en".to_owned());
    let language = input
        .get("hl")
        .filter(|value| !value.is_null())
        .unwrap_or(&default_language);
    append_input_param(&mut params, "hl", Some(language));
    append_input_param(&mut params, "gl", input.get("gl"));
    append_input_param(&mut params, "debug", input.get("debug"));

    if input.get("use_cache") != Some(&Value::Bool(false)) {
        params.push(("use_cache".to_owned(), "1".to_owned()));
        append_input_param(
            &mut params,
            "maximum_cache_age",
            input.get("maximum_cache_age"),
        );
    }

    let default_zoom = Value::Number(13.into());
    let fallback_zoom = input
        .get("fallback_zoom")
        .filter(|value| !value.is_null())
        .unwrap_or(&default_zoom);

    Ok(SearchRequest {
        query,
        params,
        fallback_zoom: js_string(fallback_zoom),
        debug: input.get("debug").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn request_url(base_url: &Url, endpoint: &[&str], params: &[(String, String)]) -> Result<Url> {
    let mut url = endpoint_url(base_url, endpoint)?;
    {
        let mut query = url.query_pairs_mut();
        for (name, value) in params {
            query.append_pair(name, value);
        }
    }
    Ok(url)
}

fn add_contact_aliases(item: &mut Value) {
    let Some(object) = item.as_object_mut() else {
        return;
    };

    if !object.contains_key("address") {
        if let Some(address) = object.get("full_address").and_then(Value::as_str) {
            object.insert("address".to_owned(), json!(address));
        }
    }

    if !object.contains_key("phone") {
        let phone_numbers = object
            .get("phone_numbers")
            .and_then(Value::as_array)
            .map(|phones| {
                phones
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|phone| !phone.trim().is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !phone_numbers.is_empty() {
            object.insert("phone".to_owned(), json!(phone_numbers.join(", ")));
        }
    }
}

fn add_search_response_aliases(mut response: Value) -> Value {
    if let Some(items) = response.get_mut("items").and_then(Value::as_array_mut) {
        for item in items {
            add_contact_aliases(item);
        }
    }
    response
}

fn is_transient_upstream_error(message: &str) -> bool {
    if let Some(status) = message
        .strip_prefix("Scrappa API error (")
        .and_then(|message| message.split_once(')'))
        .and_then(|(status, _)| status.parse::<u16>().ok())
    {
        if status == 408 || status == 429 || (500..=599).contains(&status) {
            return true;
        }
    }

    let message = message.to_ascii_lowercase();
    message.contains("timed out")
        || message.contains("timeout")
        || message.contains("temporarily unavailable")
        || message.contains("cloudflare")
}

fn status_description(status: StatusCode) -> &'static str {
    status.canonical_reason().unwrap_or("Unknown status")
}

async fn error_body(response: Response) -> String {
    response.text().await.unwrap_or_default()
}

fn append_error_detail(body: &str) -> String {
    if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    }
}

fn scrappa_error_message(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let message = match serde_json::from_str::<Value>(body) {
        Ok(value) => {
            let Some(errors) = value.get("errors").and_then(Value::as_object) else {
                return value
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or(&fallback)
                    .to_owned();
            };
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    let messages = messages.as_array()?;
                    Some(format!(
                        "{field}: {}",
                        messages
                            .iter()
                            .map(|message| match message {
                                Value::String(message) => message.clone(),
                                Value::Null => "null".to_owned(),
                                _ => js_string(message),
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                })
                .collect::<Vec<_>>()
                .join("; ");
            let mut message = value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or(&fallback)
                .to_owned();
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details);
            }
            message
        }
        Err(_) => body
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(500)
            .collect(),
    };

    if message.is_empty() {
        fallback
    } else {
        message.chars().take(500).collect()
    }
}

async fn fetch_scrappa_endpoint(
    client: &Client,
    config: &ActorConfig,
    endpoint: &[&str],
    params: &[(String, String)],
    debug: bool,
) -> Result<Value> {
    let url = request_url(&config.scrappa_api_base_url, endpoint, params)?;
    if debug {
        println!("[Scrappa] GET {url}");
    }

    let response = client
        .get(url)
        .header(header::ACCEPT, "application/json")
        .header(header::USER_AGENT, SCRAPPA_USER_AGENT)
        .header("X-API-Key", &config.scrappa_api_key)
        .timeout(config.scrappa_request_timeout)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "Scrappa API request timed out after {}ms",
                    config.scrappa_request_timeout.as_millis()
                )
            } else {
                anyhow!("Scrappa API request failed: {error}")
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "Scrappa API request timed out after {}ms",
                    config.scrappa_request_timeout.as_millis()
                )
            } else {
                anyhow!("Scrappa API request failed: {error}")
            }
        })?;
        bail!(
            "Scrappa API error ({}): {}",
            status.as_u16(),
            scrappa_error_message(status, &body)
        );
    }

    let body = response.bytes().await.map_err(|error| {
        if error.is_timeout() {
            anyhow!(
                "Scrappa API request timed out after {}ms",
                config.scrappa_request_timeout.as_millis()
            )
        } else {
            anyhow!("Scrappa API request failed: {error}")
        }
    })?;
    serde_json::from_slice(&body).context("Scrappa API response was not valid JSON")
}

async fn fetch_with_fallback(
    client: &Client,
    config: &ActorConfig,
    request: &SearchRequest,
) -> Result<Value> {
    match fetch_scrappa_endpoint(
        client,
        config,
        &["maps", "simple-search"],
        &request.params,
        request.debug,
    )
    .await
    {
        Ok(response) => return Ok(response),
        Err(error) if is_transient_upstream_error(&error.to_string()) => {
            eprintln!("Transient upstream issue on simple-search: {error}");
        }
        Err(error) => return Err(error),
    }

    let mut fallback_params = request.params.clone();
    fallback_params.push(("zoom".to_owned(), request.fallback_zoom.clone()));
    let mut response = fetch_scrappa_endpoint(
        client,
        config,
        &["maps", "advanced-search"],
        &fallback_params,
        request.debug,
    )
    .await?;
    if let Some(object) = response.as_object_mut() {
        object.insert("fallback_used".to_owned(), json!("advanced-search"));
    }
    Ok(response)
}

async fn send_apify_request<F>(build_request: F, operation: &str) -> Result<Response>
where
    F: Fn() -> RequestBuilder,
{
    for attempt in 0..=APIFY_MAX_RETRIES {
        match build_request().send().await {
            Ok(response)
                if is_retryable_apify_status(response.status()) && attempt < APIFY_MAX_RETRIES =>
            {
                sleep(apify_retry_delay(attempt)).await;
            }
            Ok(response) => return Ok(response),
            Err(error)
                if attempt < APIFY_MAX_RETRIES && is_retryable_apify_network_error(&error) =>
            {
                sleep(apify_retry_delay(attempt)).await;
            }
            Err(error) => return Err(error).with_context(|| format!("{operation} failed")),
        }
    }

    unreachable!("the final Apify attempt returns its response or error")
}

fn is_retryable_apify_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_retryable_apify_network_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request()
}

fn apify_retry_delay(retry_number: u32) -> Duration {
    let multiplier = 1_u32.checked_shl(retry_number).unwrap_or(u32::MAX);
    APIFY_MIN_RETRY_DELAY
        .saturating_mul(multiplier)
        .min(APIFY_MAX_RETRY_DELAY)
}

async fn apify_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
        let body = error_body(response).await;
        bail!(
            "{operation} failed with {} {}{}",
            status.as_u16(),
            status_description(status),
            append_error_detail(&body)
        );
    }

    response
        .json()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

async fn apify_write(response: Response, operation: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = error_body(response).await;
    bail!(
        "{operation} failed with {} {}{}",
        status.as_u16(),
        status_description(status),
        append_error_detail(&body)
    );
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
    let response = send_apify_request(
        || {
            client
                .get(url.clone())
                .bearer_auth(&config.apify_token)
                .timeout(config.apify_request_timeout)
        },
        "Apify INPUT request",
    )
    .await?;
    apify_json(response, "Apify INPUT request").await
}

async fn get_run(client: &Client, config: &ActorConfig) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = send_apify_request(
        || {
            client
                .get(url.clone())
                .bearer_auth(&config.apify_token)
                .timeout(config.apify_request_timeout)
        },
        "Apify run pricing request",
    )
    .await?;
    apify_json(response, "Apify run pricing request").await
}

struct DatasetBudget {
    dataset_item_price_usd: f64,
    charged_usd: f64,
    max_total_charge_usd: f64,
    saved_items: usize,
}

impl DatasetBudget {
    fn from_run(run: &Value, configured_limit: Option<f64>) -> Result<Option<Self>> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        if data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
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
            .or(configured_limit)
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

        Ok(Some(Self {
            dataset_item_price_usd,
            charged_usd,
            max_total_charge_usd,
            saved_items: 0,
        }))
    }

    fn affordable_items(&self, requested: usize) -> usize {
        if self.dataset_item_price_usd == 0.0 {
            return requested;
        }
        let tolerance = f64::EPSILON * self.max_total_charge_usd;
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

fn dataset_item_chunks(items: &[Value]) -> Result<Vec<&[Value]>> {
    let mut chunks = Vec::new();
    let mut chunk_start = 0;
    let mut chunk_bytes: usize = 2;

    for (index, item) in items.iter().enumerate() {
        let item_bytes = serde_json::to_vec(item)
            .context("Could not serialize dataset item")?
            .len();
        if item_bytes.saturating_add(2) >= MAX_DATASET_REQUEST_BYTES {
            bail!(
                "A dataset item must serialize to less than {} bytes",
                MAX_DATASET_REQUEST_BYTES - 2
            );
        }

        let separator_bytes = usize::from(index != chunk_start);
        let next_chunk_bytes = chunk_bytes
            .saturating_add(separator_bytes)
            .saturating_add(item_bytes);
        if next_chunk_bytes >= MAX_DATASET_REQUEST_BYTES {
            chunks.push(&items[chunk_start..index]);
            chunk_start = index;
            chunk_bytes = 2;
        }

        let separator_bytes = usize::from(index != chunk_start);
        chunk_bytes = chunk_bytes
            .saturating_add(separator_bytes)
            .saturating_add(item_bytes);
    }

    if chunk_start < items.len() {
        chunks.push(&items[chunk_start..]);
    }

    Ok(chunks)
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    items: &[Value],
) -> Result<usize> {
    if items.is_empty() {
        return Ok(0);
    }

    let run = get_run(client, config).await?;
    let mut budget = DatasetBudget::from_run(&run, config.max_total_charge_usd)?;
    let allowed = budget
        .as_ref()
        .map(|budget| budget.affordable_items(items.len()))
        .unwrap_or(items.len());
    if allowed == 0 {
        return Ok(0);
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    for chunk in dataset_item_chunks(&items[..allowed])? {
        let response = client
            .post(url.clone())
            .bearer_auth(&config.apify_token)
            .timeout(config.apify_request_timeout)
            .json(chunk)
            .send()
            .await
            .context("Apify dataset write failed")?;
        apify_write(response, "Apify dataset write").await?;
    }

    if let Some(budget) = budget.as_mut() {
        budget.record_saved_items(allowed);
    }
    Ok(allowed)
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
    let response = send_apify_request(
        || {
            client
                .put(url.clone())
                .bearer_auth(&config.apify_token)
                .timeout(config.apify_request_timeout)
                .json(output)
        },
        "Apify OUTPUT write",
    )
    .await?;
    apify_write(response, "Apify OUTPUT write").await
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let request = build_search_request(&input)?;
    println!("Searching Google Maps for: \"{}\"", request.query);

    let response = fetch_with_fallback(client, config, &request).await?;
    let response = add_search_response_aliases(response);
    let items = response
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let saved_items = push_dataset_items(client, config, &items).await?;
    if items.is_empty() {
        println!("No results found for the given search criteria");
    } else {
        println!("Found {} results", items.len());
        if saved_items < items.len() {
            println!(
                "Saved {saved_items} of {} results within the pay-per-event spending limit",
                items.len()
            );
        }
    }

    set_output(client, config, &response).await?;
    println!("Search completed successfully");
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
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
mod tests;
