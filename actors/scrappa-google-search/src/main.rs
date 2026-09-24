use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Value};
use tokio::time::sleep;
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_MAX_ATTEMPTS: u8 = 3;
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
    scrappa_request_timeout: Duration,
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
            scrappa_request_timeout: SCRAPPA_REQUEST_TIMEOUT,
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

fn build_search_params(input: &Value) -> Result<Vec<(String, String)>> {
    let object = input
        .as_object()
        .ok_or_else(|| anyhow!("Search query is required"))?;
    object
        .get("query")
        .filter(|value| js_truthy(value))
        .ok_or_else(|| anyhow!("Search query is required"))?;

    let mut params = Vec::with_capacity(SEARCH_PARAMETERS.len());
    for name in SEARCH_PARAMETERS {
        let Some(value) = object.get(name) else {
            continue;
        };
        if value.is_null() || value == "" || value == false {
            continue;
        }
        let value = match value {
            Value::Bool(true) => "1".to_owned(),
            _ => js_string(value),
        };
        params.push((name.to_owned(), value));
    }
    Ok(params)
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
        Value::Number(number) => number.to_string(),
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

fn build_search_url(base_url: &Url, params: &[(String, String)]) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["search"])?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            query.append_pair(key, value);
        }
    }
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

#[derive(Debug)]
enum ScrappaFailure {
    Http { status: u16, message: String },
    Request(reqwest::Error),
    InvalidJson(String),
}

impl ScrappaFailure {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Http { status, .. } => {
                matches!(*status, 408 | 429 | 500 | 502 | 503 | 504)
            }
            Self::Request(error) => error.is_timeout() || error.is_connect() || error.is_body(),
            Self::InvalidJson(_) => false,
        }
    }

    fn into_anyhow(self, timeout: Duration) -> anyhow::Error {
        match self {
            Self::Http { status, message } => anyhow!("Scrappa API error ({status}): {message}"),
            Self::Request(error) if error.is_timeout() => anyhow!(
                "Scrappa API request timed out after {}ms",
                timeout.as_millis()
            ),
            Self::Request(error) => anyhow!("{error}"),
            Self::InvalidJson(message) => {
                anyhow!("Scrappa API response was not valid JSON: {message}")
            }
        }
    }
}

fn error_message(response_status: StatusCode, body: &str) -> String {
    let fallback = response_status
        .canonical_reason()
        .unwrap_or("Unknown status")
        .to_owned();
    let Ok(data) = serde_json::from_str::<Value>(body) else {
        return if body.is_empty() {
            fallback
        } else {
            body.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(500)
                .collect()
        };
    };
    let Some(object) = data.as_object() else {
        return if body.is_empty() {
            fallback
        } else {
            body.to_owned()
        };
    };
    let mut message = object
        .get("message")
        .filter(|value| !value.is_null())
        .map(js_string)
        .unwrap_or(fallback);
    if let Some(errors) = object.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .filter_map(|(field, values)| {
                values.as_array().map(|values| {
                    format!(
                        "{field}: {}",
                        values.iter().map(js_string).collect::<Vec<_>>().join(", ")
                    )
                })
            })
            .collect::<Vec<_>>();
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details.join("; "));
        }
    }
    message
}

async fn fetch_google_search(
    http: &Client,
    config: &Config,
    params: &[(String, String)],
) -> Result<Value> {
    let url = build_search_url(&config.scrappa_api_base, params)?;
    for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
        let response = http
            .get(url.clone())
            .timeout(config.scrappa_request_timeout)
            .header("X-API-Key", &config.scrappa_api_key)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(ScrappaFailure::Request);

        let result = match response {
            Ok(response) => {
                let status = response.status();
                match response.text().await {
                    Ok(body) if !status.is_success() => Err(ScrappaFailure::Http {
                        status: status.as_u16(),
                        message: error_message(status, &body),
                    }),
                    Ok(body) => serde_json::from_str(&body)
                        .map_err(|error| ScrappaFailure::InvalidJson(error.to_string())),
                    Err(error) => Err(ScrappaFailure::Request(error)),
                }
            }
            Err(error) => Err(error),
        };

        match result {
            Ok(response) => return Ok(response),
            Err(error) if error.is_retryable() && attempt < SCRAPPA_MAX_ATTEMPTS => {
                eprintln!(
                    "Transient Scrappa API failure; retrying attempt {}/{}",
                    attempt + 1,
                    SCRAPPA_MAX_ATTEMPTS
                );
                sleep(Duration::from_secs(u64::from(attempt))).await;
            }
            Err(error) => return Err(error.into_anyhow(config.scrappa_request_timeout)),
        }
    }
    unreachable!("the retry loop always returns or fails")
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
        .get(DATASET_ITEM_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => return Ok(requested),
        Some(value) => value
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?,
    };
    if !item_price.is_finite() || item_price < 0.0 || !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    if max_charge == 0.0 {
        return Ok(requested);
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
        response_json(response, "Apify INPUT request").await
    }

    async fn dataset_capacity(&self, requested: usize) -> Result<usize> {
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
        affordable_dataset_items(&run, requested)
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<usize> {
        if items.is_empty() {
            return Ok(0);
        }
        let limit = self.dataset_capacity(items.len()).await?;
        let items = &items[..items.len().min(limit)];
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

fn array_length(response: &Value, field: &str) -> usize {
    response
        .get(field)
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
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
    let params = build_search_params(&input)?;
    let query = input
        .get("query")
        .map(js_string)
        .unwrap_or_else(|| "".to_owned());
    println!("Searching Google for: \"{query}\"");

    let response = fetch_google_search(http, config, &params).await?;
    let organic_results = response
        .get("organic_results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let saved_results = apify.push_dataset_items(&organic_results).await?;
    apify.put_output(&response).await?;

    println!("Google Search completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string(&json!({
            "organic_results": organic_results.len(),
            "dataset_items_saved": saved_results,
            "related_searches": array_length(&response, "related_searches"),
            "related_questions": array_length(&response, "related_questions"),
            "inline_videos": array_length(&response, "inline_videos"),
            "inline_images": array_length(&response, "inline_images"),
            "has_knowledge_graph": response.get("knowledge_graph").is_some_and(js_truthy),
            "has_local_results": response.get("local_results").is_some_and(js_truthy),
        }))?
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    let result = async {
        let config = Config::from_env()?;
        let http = Client::builder()
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
mod tests;
