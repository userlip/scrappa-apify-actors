use std::{
    collections::BTreeMap,
    env,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Map, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_APIFY_REQUEST_ATTEMPTS: usize = 3;
const DATASET_BATCH_MAX_BYTES: usize = 4_500_000;
const SEARCH_EVENT: &str = "search";
const RESULT_EVENT: &str = "result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

struct Config {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
    scrappa_api_key: String,
    scrappa_request_timeout: Duration,
    pricing_info: Option<Value>,
    charged_event_counts: Option<Value>,
    max_total_charge_usd: Option<f64>,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").unwrap_or_default();
        if scrappa_api_key.is_empty() {
            bail!("SCRAPPA_API_KEY environment variable is not set.");
        }

        let max_total_charge_usd = env::var("ACTOR_MAX_TOTAL_CHARGE_USD")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|value| {
                value
                    .parse::<f64>()
                    .with_context(|| "ACTOR_MAX_TOTAL_CHARGE_USD must be a valid number")
            })
            .transpose()?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
            scrappa_request_timeout: SCRAPPA_REQUEST_TIMEOUT,
            pricing_info: optional_json_env("APIFY_ACTOR_PRICING_INFO")?,
            charged_event_counts: optional_json_env("APIFY_CHARGED_ACTOR_EVENT_COUNTS")?,
            max_total_charge_usd,
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

fn optional_json_env(name: &str) -> Result<Option<Value>> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| {
            serde_json::from_str(&value).with_context(|| format!("{name} must contain valid JSON"))
        })
        .transpose()
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

fn json_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::Null => String::new(),
                _ => json_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn is_javascript_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn append_query_value(url: &mut Url, name: &str, value: Option<&Value>) {
    let Some(value) = value else {
        return;
    };
    if value.is_null() || value.as_str() == Some("") {
        return;
    }
    if let Value::Bool(boolean) = value {
        if *boolean {
            url.query_pairs_mut().append_pair(name, "1");
        }
        return;
    }
    url.query_pairs_mut().append_pair(name, &json_string(value));
}

fn build_search_url(input: &Value, api_base_url: &Url) -> Result<Url> {
    let mut url = endpoint_url(api_base_url, &["maps", "advance-search"])?;
    url.set_query(None);

    let query = input
        .get("query")
        .ok_or_else(|| anyhow!("Search query and zoom level are required"))?;
    let zoom = input
        .get("zoom")
        .ok_or_else(|| anyhow!("Search query and zoom level are required"))?;
    if !is_javascript_truthy(query) {
        bail!("Search query and zoom level are required");
    }

    append_query_value(&mut url, "query", Some(query));
    append_query_value(&mut url, "zoom", Some(zoom));
    append_query_value(&mut url, "lat", input.get("latitude"));
    append_query_value(&mut url, "lon", input.get("longitude"));
    append_query_value(&mut url, "limit", input.get("limit"));

    let language = input
        .get("hl")
        .filter(|value| is_javascript_truthy(value))
        .map(json_string)
        .unwrap_or_else(|| "en".to_owned());
    url.query_pairs_mut().append_pair("hl", &language);
    append_query_value(&mut url, "gl", input.get("gl"));

    Ok(url)
}

fn transient_apify_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_EARLY
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn apify_retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(200 * 2_u64.pow((attempt.saturating_sub(1)) as u32))
}

async fn send_apify_with_retries<F>(make_request: F, operation: &str) -> Result<Response>
where
    F: Fn() -> RequestBuilder,
{
    for attempt in 1..=MAX_APIFY_REQUEST_ATTEMPTS {
        match make_request().timeout(APIFY_REQUEST_TIMEOUT).send().await {
            Ok(response)
                if transient_apify_status(response.status())
                    && attempt < MAX_APIFY_REQUEST_ATTEMPTS =>
            {
                tokio::time::sleep(apify_retry_delay(attempt)).await;
            }
            Ok(response) => return Ok(response),
            Err(_) if attempt < MAX_APIFY_REQUEST_ATTEMPTS => {
                tokio::time::sleep(apify_retry_delay(attempt)).await;
            }
            Err(error) => return Err(anyhow!("{operation} failed: {error}")),
        }
    }
    unreachable!("retry loop always returns a response or error")
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

fn next_idempotency_key(actor_run_id: &str, event_name: &str) -> String {
    static NEXT_CHARGE_ID: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = NEXT_CHARGE_ID.fetch_add(1, Ordering::Relaxed);
    format!("{actor_run_id}-{event_name}-{now}-{sequence}")
}

struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

impl ApifyClient<'_> {
    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base_url, segments)
    }

    async fn get_input(&self) -> Result<Value> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = send_apify_with_retries(
            || {
                self.http
                    .get(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
            },
            "Apify INPUT request",
        )
        .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(Value::Null);
        }
        response_json(response, "Apify INPUT request").await
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let response = send_apify_with_retries(
            || {
                self.http
                    .get(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
            },
            "Apify run pricing request",
        )
        .await?;
        response_json(response, "Apify run pricing request").await
    }

    async fn charge(&self, event_name: &str, count: usize) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id, "charge"])?;
        let count = u64::try_from(count).context("Charge event count is too large")?;
        let idempotency_key = next_idempotency_key(&self.config.actor_run_id, event_name);
        let body = json!({ "eventName": event_name, "count": count });
        let response = send_apify_with_retries(
            || {
                self.http
                    .post(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
                    .header("idempotency-key", &idempotency_key)
                    .json(&body)
            },
            "Apify event charge",
        )
        .await?;
        ensure_success(response, "Apify event charge").await
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        for batch in dataset_batches(items)? {
            let response = send_apify_with_retries(
                || {
                    self.http
                        .post(url.clone())
                        .bearer_auth(&self.config.apify_token)
                        .header(header::ACCEPT, "application/json")
                        .json(&batch)
                },
                "Apify dataset write",
            )
            .await?;
            ensure_success(response, "Apify dataset write").await?;
        }
        Ok(())
    }

    async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = send_apify_with_retries(
            || {
                self.http
                    .put(url.clone())
                    .bearer_auth(&self.config.apify_token)
                    .header(header::ACCEPT, "application/json")
                    .json(output)
            },
            "Apify OUTPUT write",
        )
        .await?;
        ensure_success(response, "Apify OUTPUT write").await
    }
}

fn dataset_batches(items: &[Value]) -> Result<Vec<Vec<Value>>> {
    let mut batches = Vec::new();
    let mut batch = Vec::new();
    let mut batch_bytes = 2;

    for item in items {
        let item_bytes = serde_json::to_vec(item)
            .context("Could not serialize Apify dataset item")?
            .len();
        if item_bytes + 2 > DATASET_BATCH_MAX_BYTES {
            bail!("Apify dataset item exceeds the 4.5 MB write limit");
        }
        let separator_bytes = usize::from(!batch.is_empty());
        if !batch.is_empty() && batch_bytes + separator_bytes + item_bytes > DATASET_BATCH_MAX_BYTES
        {
            batches.push(std::mem::take(&mut batch));
            batch_bytes = 2;
        }
        if !batch.is_empty() {
            batch_bytes += 1;
        }
        batch_bytes += item_bytes;
        batch.push(item.clone());
    }

    if !batch.is_empty() {
        batches.push(batch);
    }
    Ok(batches)
}

struct ChargeBudget {
    event_prices: BTreeMap<String, f64>,
    max_total_charge_usd: f64,
    charged_event_counts: BTreeMap<String, u64>,
}

impl ChargeBudget {
    fn from_metadata(
        pricing_info: &Value,
        charged_event_counts: &Value,
        max_total_charge_usd: f64,
    ) -> Result<Self> {
        if pricing_info.get("pricingModel").and_then(Value::as_str) != Some("PAY_PER_EVENT") {
            bail!("Apify run is not configured for pay-per-event pricing");
        }

        if max_total_charge_usd.is_nan() || max_total_charge_usd < 0.0 {
            bail!("Apify run returned invalid charging values");
        }

        let event_prices_json = pricing_info
            .pointer("/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = BTreeMap::new();
        for (event_name, event) in event_prices_json {
            let price = event
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    anyhow!("Apify run did not provide a price for event {event_name}")
                })?;
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned invalid price for event {event_name}");
            }
            event_prices.insert(event_name.clone(), price);
        }

        let counts_json = match charged_event_counts.get("chargedEventCounts") {
            Some(counts) => counts,
            None => charged_event_counts,
        };
        let counts_json = counts_json
            .as_object()
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_counts = BTreeMap::new();
        for (event_name, count) in counts_json {
            let count = count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            charged_counts.insert(event_name.clone(), count);
        }

        Ok(Self {
            event_prices,
            max_total_charge_usd,
            charged_event_counts: charged_counts,
        })
    }

    fn affordable_count(&self, event_name: &str, requested: usize) -> Result<usize> {
        let item_price =
            self.event_prices.get(event_name).copied().ok_or_else(|| {
                anyhow!("Apify run did not provide the price for event {event_name}")
            })?;
        self.affordable_count_at_price(item_price, requested)
    }

    fn affordable_dataset_item_count(&self, event_name: &str, requested: usize) -> Result<usize> {
        let event_price =
            self.event_prices.get(event_name).copied().ok_or_else(|| {
                anyhow!("Apify run did not provide the price for event {event_name}")
            })?;
        let dataset_item_price = self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0.0);
        let item_price = event_price + dataset_item_price;
        if !item_price.is_finite() {
            bail!("Apify run returned invalid combined dataset item price");
        }
        self.affordable_count_at_price(item_price, requested)
    }

    fn affordable_count_at_price(&self, item_price: f64, requested: usize) -> Result<usize> {
        if !item_price.is_finite() || item_price < 0.0 {
            bail!("Apify run returned invalid charging values");
        }

        let mut spent = 0.0;
        for (charged_event_name, count) in &self.charged_event_counts {
            if *count == 0 {
                continue;
            }
            let price = self
                .event_prices
                .get(charged_event_name)
                .copied()
                .unwrap_or(0.0);
            spent += price * *count as f64;
        }
        if !spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        if item_price == 0.0 || self.max_total_charge_usd == f64::INFINITY {
            return Ok(requested);
        }

        let tolerance = f64::EPSILON * self.max_total_charge_usd.max(1.0);
        Ok((1..=requested)
            .take_while(|count| {
                spent + *count as f64 * item_price <= self.max_total_charge_usd + tolerance
            })
            .count())
    }

    async fn charge_event(
        &mut self,
        apify: &ApifyClient<'_>,
        event_name: &str,
        requested: usize,
    ) -> Result<ChargeResult> {
        if requested == 0 {
            return Ok(ChargeResult {
                charged: 0,
                event_charge_limit_reached: false,
            });
        }

        let affordable = self.affordable_count(event_name, requested)?;
        if affordable == 0 {
            return Ok(ChargeResult {
                charged: 0,
                event_charge_limit_reached: true,
            });
        }

        apify.charge(event_name, affordable).await?;
        let affordable = u64::try_from(affordable).context("Charge event count is too large")?;
        let total = self
            .charged_event_counts
            .get(event_name)
            .copied()
            .unwrap_or(0)
            .checked_add(affordable)
            .ok_or_else(|| anyhow!("Charged event count overflowed"))?;
        self.charged_event_counts
            .insert(event_name.to_owned(), total);
        let event_charge_limit_reached = self.affordable_count(event_name, 1)? == 0;

        Ok(ChargeResult {
            charged: usize::try_from(affordable).context("Charge event count is too large")?,
            event_charge_limit_reached,
        })
    }

    fn record_default_dataset_items(&mut self, count: usize) -> Result<()> {
        if !self.event_prices.contains_key(DEFAULT_DATASET_ITEM_EVENT) || count == 0 {
            return Ok(());
        }
        let count = u64::try_from(count).context("Dataset item count is too large")?;
        let total = self
            .charged_event_counts
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0)
            .checked_add(count)
            .ok_or_else(|| anyhow!("Charged dataset item count overflowed"))?;
        self.charged_event_counts
            .insert(DEFAULT_DATASET_ITEM_EVENT.to_owned(), total);
        Ok(())
    }
}

struct ChargeResult {
    charged: usize,
    event_charge_limit_reached: bool,
}

async fn load_charge_budget(http: &Client, config: &Config) -> Result<ChargeBudget> {
    if let (Some(pricing_info), Some(charged_counts)) = (
        config.pricing_info.as_ref(),
        config.charged_event_counts.as_ref(),
    ) {
        return ChargeBudget::from_metadata(
            pricing_info,
            charged_counts,
            config.max_total_charge_usd.unwrap_or(f64::INFINITY),
        );
    }

    let apify = ApifyClient { http, config };
    let run = apify.get_run().await?;
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let pricing_info = data
        .get("pricingInfo")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let empty_counts = Value::Object(Map::new());
    let charged_counts = config
        .charged_event_counts
        .as_ref()
        .or_else(|| data.get("chargedEventCounts"))
        .unwrap_or(&empty_counts);
    let max_charge = config
        .max_total_charge_usd
        .or_else(|| {
            data.pointer("/options/maxTotalChargeUsd")
                .and_then(Value::as_f64)
        })
        .unwrap_or(f64::INFINITY);

    ChargeBudget::from_metadata(pricing_info, charged_counts, max_charge)
}

fn scrappa_error_message(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    if body.is_empty() {
        return fallback;
    }

    if let Ok(data) = serde_json::from_str::<Value>(body) {
        let mut message = data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(&fallback)
            .to_owned();
        if let Some(errors) = data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    messages.as_array().map(|messages| {
                        let joined = messages
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{field}: {joined}")
                    })
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

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

async fn fetch_search(http: &Client, config: &Config, input: &Value) -> Result<Value> {
    let url = build_search_url(input, &config.scrappa_api_base_url)?;
    let response = http
        .get(url)
        .timeout(config.scrappa_request_timeout)
        .header("X-API-Key", &config.scrappa_api_key)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "Scrappa API request timed out after {}ms",
                    config.scrappa_request_timeout.as_millis()
                )
            } else {
                anyhow!(error.to_string())
            }
        })?;

    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read Scrappa API response")?;
    if !status.is_success() {
        bail!(
            "Scrappa API error ({}): {}",
            status.as_u16(),
            scrappa_error_message(status, &body)
        );
    }
    serde_json::from_str(&body).context("Scrappa API returned invalid JSON")
}

async fn run_actor(http: &Client, config: &Config) -> Result<()> {
    if config.scrappa_api_key.is_empty() {
        bail!("SCRAPPA_API_KEY environment variable is not set.");
    }

    let apify = ApifyClient { http, config };
    let input = apify.get_input().await?;
    if !input.get("query").is_some_and(is_javascript_truthy) || input.get("zoom").is_none() {
        bail!("Search query and zoom level are required");
    }

    let mut budget = load_charge_budget(http, config).await?;
    let search_charge = budget.charge_event(&apify, SEARCH_EVENT, 1).await?;
    if search_charge.event_charge_limit_reached {
        println!("User budget limit reached, stopping.");
        return Ok(());
    }

    let query = input.get("query").map(json_string).unwrap_or_default();
    let zoom = input.get("zoom").map(json_string).unwrap_or_default();
    let location_info = if input.get("latitude").is_some_and(is_javascript_truthy)
        && input.get("longitude").is_some_and(is_javascript_truthy)
    {
        format!(
            "at lat {}, lon {}",
            input.get("latitude").map(json_string).unwrap_or_default(),
            input.get("longitude").map(json_string).unwrap_or_default()
        )
    } else {
        "(auto-resolved location)".to_owned()
    };
    println!("Advanced search: \"{query}\" at zoom {zoom} {location_info}");

    let response = fetch_search(http, config, &input).await?;
    let items = response
        .get("items")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if !items.is_empty() {
        let allowed_items = budget.affordable_dataset_item_count(RESULT_EVENT, items.len())?;
        let result_charge = budget
            .charge_event(&apify, RESULT_EVENT, allowed_items)
            .await?;
        if result_charge.charged > 0 {
            apify
                .push_dataset_items(&items[..result_charge.charged])
                .await?;
            budget.record_default_dataset_items(result_charge.charged)?;
            println!("Found {} results", result_charge.charged);
        }
    } else {
        println!("No results found for the given search criteria");
    }

    apify.put_output(&response).await?;

    let language = input
        .get("hl")
        .filter(|value| is_javascript_truthy(value))
        .map(json_string)
        .unwrap_or_else(|| "en".to_owned());
    let region = input
        .get("gl")
        .filter(|value| is_javascript_truthy(value))
        .map(json_string)
        .unwrap_or_else(|| "worldwide".to_owned());
    let results_found = response
        .get("items")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    println!(
        "Advanced search completed: {}",
        json!({
            "query": query,
            "results_found": results_found,
            "zoom_level": input.get("zoom"),
            "language": language,
            "region": region
        })
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Actor failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let config = Config::from_env()?;
    let http = Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .context("Could not create HTTP client")?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Could not start Tokio runtime")?;
    runtime.block_on(run_actor(&http, &config))
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
        thread,
        time::Instant,
    };

    struct MockResponse {
        status: u16,
        body: String,
        delay: Duration,
    }

    struct MockServer {
        base_url: Url,
        requests: Receiver<String>,
        stop: Arc<AtomicBool>,
        thread: Option<thread::JoinHandle<()>>,
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
                    let _ = recorded_requests.send(request);
                    let Some(response) = responses.next() else {
                        break;
                    };
                    if !response.delay.is_zero() {
                        thread::sleep(response.delay);
                    }
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        429 => "Too Many Requests",
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
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        let mut content_length = 0;
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                if content_length == 0 {
                    let headers = String::from_utf8_lossy(&bytes[..header_end]);
                    content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                }
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
            delay: Duration::ZERO,
        }
    }

    fn delayed_response(status: u16, body: &str, delay: Duration) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
            delay,
        }
    }

    fn pricing_info() -> Value {
        json!({
            "pricingModel": "PAY_PER_EVENT",
            "pricingPerEvent": {"actorChargeEvents": {
                "search": {"eventPriceUsd": 0.001},
                "result": {"eventPriceUsd": 0.0003},
                "apify-actor-start": {"eventPriceUsd": 0.0001}
            }}
        })
    }

    fn pricing_info_with_dataset_item_price() -> Value {
        let mut pricing = pricing_info();
        pricing["pricingPerEvent"]["actorChargeEvents"][DEFAULT_DATASET_ITEM_EVENT] =
            json!({"eventPriceUsd": 0.00075});
        pricing
    }

    fn config(server: &MockServer) -> Config {
        let mut scrappa_api_base_url = server.base_url.clone();
        scrappa_api_base_url.set_path("/api");
        Config {
            apify_api_base_url: server.base_url.clone(),
            scrappa_api_base_url,
            key_value_store_id: "test-store".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "apify-test-token".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
            scrappa_request_timeout: SCRAPPA_REQUEST_TIMEOUT,
            pricing_info: Some(pricing_info()),
            charged_event_counts: Some(json!({"apify-actor-start": 1})),
            max_total_charge_usd: Some(1.0),
        }
    }

    fn client(timeout: Duration) -> Client {
        Client::builder().timeout(timeout).build().unwrap()
    }

    fn request_parts(request: &str) -> (&str, &str, &str, &str) {
        let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
        let mut parts = headers
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace();
        (
            parts.next().unwrap_or_default(),
            parts.next().unwrap_or_default(),
            headers,
            body,
        )
    }

    fn header_value<'a>(headers: &'a str, header_name: &str) -> Option<&'a str> {
        headers.lines().skip(1).find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(header_name)
                .then_some(value.trim())
        })
    }

    fn test_input() -> Value {
        json!({
            "query": "coffee shops",
            "zoom": 15,
            "latitude": 40.758,
            "longitude": -73.9855,
            "limit": 50,
            "hl": "de",
            "gl": "de"
        })
    }

    #[test]
    fn input_schema_and_prefill_contract_are_kept() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["required"], json!(["query", "zoom"]));
        assert_eq!(schema["properties"]["query"]["prefill"], "coffee shops");
        assert_eq!(schema["properties"]["zoom"]["default"], 15);
        assert_eq!(schema["properties"]["zoom"]["minimum"], 3);
        assert_eq!(schema["properties"]["zoom"]["maximum"], 21);
        assert!(schema["properties"].get("page").is_none());
        assert_eq!(
            schema["properties"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                "query",
                "zoom",
                "latitude",
                "longitude",
                "limit",
                "hl",
                "gl"
            ]
        );
    }

    #[test]
    fn search_request_keeps_legacy_path_auth_parameters_and_default_page() {
        let base_url = Url::parse("https://scrappa.co/api").unwrap();
        let url = build_search_url(&test_input(), &base_url).unwrap();
        assert_eq!(url.path(), "/api/maps/advance-search");
        let query = url
            .query_pairs()
            .map(|(name, value)| (name.into_owned(), value.into_owned()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(query.get("query").map(String::as_str), Some("coffee shops"));
        assert_eq!(query.get("zoom").map(String::as_str), Some("15"));
        assert_eq!(query.get("lat").map(String::as_str), Some("40.758"));
        assert_eq!(query.get("lon").map(String::as_str), Some("-73.9855"));
        assert_eq!(query.get("limit").map(String::as_str), Some("50"));
        assert_eq!(query.get("hl").map(String::as_str), Some("de"));
        assert_eq!(query.get("gl").map(String::as_str), Some("de"));
        assert!(!query.contains_key("page"));

        let defaults = build_search_url(
            &json!({"query":"coffee shops","zoom":15,"latitude":0,"longitude":0,"hl":"","gl":""}),
            &base_url,
        )
        .unwrap();
        let query = defaults.query_pairs().collect::<BTreeMap<_, _>>();
        assert_eq!(query.get("lat").map(|value| value.as_ref()), Some("0"));
        assert_eq!(query.get("lon").map(|value| value.as_ref()), Some("0"));
        assert_eq!(query.get("hl").map(|value| value.as_ref()), Some("en"));
        assert!(!query.contains_key("gl"));
        assert!(!query.contains_key("page"));
    }

    #[tokio::test]
    async fn full_actor_flow_preserves_auth_charges_dataset_and_raw_output() {
        let input = test_input();
        let output = json!({
            "items": [
                {"name":"First Cafe","business_id":"first","rating":4.8},
                {"name":"Second Cafe","business_id":"second","phone_numbers":["+1 555 0100"]}
            ],
            "pagination": {"page": 0, "next": null},
            "provider_field": {"kept": true}
        });
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(201, "{}"),
            response(200, &output.to_string()),
            response(201, "{}"),
            response(201, "{}"),
            response(200, "{}"),
        ]);
        let config = config(&server);
        run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 6);
        assert_eq!(request_parts(&requests[0]).0, "GET");
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert_eq!(
            header_value(request_parts(&requests[0]).2, "authorization"),
            Some("Bearer apify-test-token")
        );

        assert_eq!(request_parts(&requests[1]).0, "POST");
        assert_eq!(
            request_parts(&requests[1]).1,
            "/v2/actor-runs/test-run/charge"
        );
        let search_charge: Value = serde_json::from_str(request_parts(&requests[1]).3).unwrap();
        assert_eq!(search_charge, json!({"eventName":"search","count":1}));
        assert!(header_value(request_parts(&requests[1]).2, "idempotency-key").is_some());

        assert_eq!(request_parts(&requests[2]).0, "GET");
        assert_eq!(
            request_parts(&requests[2]).1.split('?').next(),
            Some("/api/maps/advance-search")
        );
        let target = request_parts(&requests[2]).1;
        assert!(target.contains("query=coffee+shops"));
        assert!(target.contains("lat=40.758"));
        assert!(target.contains("page=") == false);
        assert_eq!(
            header_value(request_parts(&requests[2]).2, "x-api-key"),
            Some("scrappa-test-key")
        );

        assert_eq!(request_parts(&requests[3]).0, "POST");
        assert_eq!(
            request_parts(&requests[3]).1,
            "/v2/actor-runs/test-run/charge"
        );
        let result_charge: Value = serde_json::from_str(request_parts(&requests[3]).3).unwrap();
        assert_eq!(result_charge, json!({"eventName":"result","count":2}));

        assert_eq!(request_parts(&requests[4]).0, "POST");
        assert_eq!(
            request_parts(&requests[4]).1,
            "/v2/datasets/test-dataset/items"
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[4]).3).unwrap(),
            output["items"]
        );

        assert_eq!(request_parts(&requests[5]).0, "PUT");
        assert_eq!(
            request_parts(&requests[5]).1,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[5]).3).unwrap(),
            output
        );
    }

    #[tokio::test]
    async fn result_budget_caps_dataset_rows_after_search_charge_and_keeps_full_output() {
        let input = test_input();
        let output = json!({
            "items": [
                {"name":"First Cafe","business_id":"first"},
                {"name":"Second Cafe","business_id":"second"},
                {"name":"Third Cafe","business_id":"third"},
                {"name":"Fourth Cafe","business_id":"fourth"}
            ],
            "pagination": {"page": 0}
        });
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(201, "{}"),
            response(200, &output.to_string()),
            response(201, "{}"),
            response(201, "{}"),
            response(200, "{}"),
        ]);
        let mut config = config(&server);
        config.max_total_charge_usd = Some(0.0021);
        run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 6);
        let result_charge: Value = serde_json::from_str(request_parts(&requests[3]).3).unwrap();
        assert_eq!(result_charge, json!({"eventName":"result","count":3}));
        let dataset: Value = serde_json::from_str(request_parts(&requests[4]).3).unwrap();
        assert_eq!(dataset.as_array().unwrap().len(), 3);
        assert_eq!(dataset[0]["business_id"], "first");
        assert_eq!(dataset[2]["business_id"], "third");
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[5]).3).unwrap(),
            output
        );
    }

    #[tokio::test]
    async fn default_dataset_event_cost_is_included_in_result_budget() {
        let input = test_input();
        let output = json!({
            "items": [
                {"name":"First Cafe","business_id":"first"},
                {"name":"Second Cafe","business_id":"second"}
            ],
            "pagination": {"page": 0}
        });
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(201, "{}"),
            response(200, &output.to_string()),
            response(201, "{}"),
            response(201, "{}"),
            response(200, "{}"),
        ]);
        let mut config = config(&server);
        config.pricing_info = Some(pricing_info_with_dataset_item_price());
        config.max_total_charge_usd = Some(0.00215);
        run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 6);
        let result_charge: Value = serde_json::from_str(request_parts(&requests[3]).3).unwrap();
        assert_eq!(result_charge, json!({"eventName":"result","count":1}));
        let dataset: Value = serde_json::from_str(request_parts(&requests[4]).3).unwrap();
        assert_eq!(dataset.as_array().unwrap().len(), 1);
        assert_eq!(dataset[0]["business_id"], "first");
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[5]).3).unwrap(),
            output
        );
    }

    #[tokio::test]
    async fn exhausted_search_budget_stops_before_scrappa_or_output_writes() {
        let server = MockServer::start(vec![response(200, &test_input().to_string())]);
        let mut config = config(&server);
        config.max_total_charge_usd = Some(0.0001);
        run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
    }

    #[tokio::test]
    async fn exact_search_budget_boundary_stops_before_scrappa_or_output_writes() {
        let server = MockServer::start(vec![
            response(200, &test_input().to_string()),
            response(201, "{}"),
        ]);
        let mut config = config(&server);
        config.max_total_charge_usd = Some(0.0011);
        run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_parts(&requests[0]).0, "GET");
        assert_eq!(request_parts(&requests[1]).0, "POST");
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[1]).3).unwrap(),
            json!({"eventName":"search","count":1})
        );
    }

    #[tokio::test]
    async fn run_metadata_is_loaded_when_apify_charge_environment_is_missing() {
        let input = test_input();
        let run = json!({
            "data": {
                "pricingInfo": pricing_info(),
                "options": {},
                "chargedEventCounts": {"apify-actor-start": 1}
            }
        });
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(200, &run.to_string()),
            response(201, "{}"),
            response(200, "{\"items\":[]}"),
            response(200, "{}"),
        ]);
        let mut config = config(&server);
        config.pricing_info = None;
        config.charged_event_counts = None;
        config.max_total_charge_usd = None;
        run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert_eq!(request_parts(&requests[1]).0, "GET");
        assert_eq!(request_parts(&requests[1]).1, "/v2/actor-runs/test-run");
        assert_eq!(
            request_parts(&requests[2]).1,
            "/v2/actor-runs/test-run/charge"
        );
        assert_eq!(
            request_parts(&requests[4]).1,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
    }

    #[tokio::test]
    async fn missing_spending_limit_uses_unbounded_budget_without_api_lookup() {
        let server = MockServer::start(vec![]);
        let mut config = config(&server);
        config.max_total_charge_usd = None;
        let http = client(APIFY_REQUEST_TIMEOUT);
        let budget = load_charge_budget(&http, &config).await.unwrap();
        assert_eq!(budget.max_total_charge_usd, f64::INFINITY);
        assert_eq!(budget.affordable_count(SEARCH_EVENT, 1).unwrap(), 1);
        assert!(server.requests().is_empty());
    }

    #[tokio::test]
    async fn missing_input_record_keeps_actor_validation_error() {
        let server = MockServer::start(vec![response(404, "not found")]);
        let config = config(&server);
        let error = run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Search query and zoom level are required"
        );
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn affordable_result_count_accounts_for_every_priced_event() {
        let budget =
            ChargeBudget::from_metadata(&pricing_info(), &json!({"apify-actor-start":1}), 0.0014)
                .unwrap();
        assert_eq!(budget.affordable_count(SEARCH_EVENT, 1).unwrap(), 1);
        let mut budget = budget;
        budget.charged_event_counts.insert("search".to_owned(), 1);
        assert_eq!(budget.affordable_count(RESULT_EVENT, 5).unwrap(), 1);

        let no_room =
            ChargeBudget::from_metadata(&pricing_info(), &json!({"apify-actor-start": 2}), 0.0001)
                .unwrap();
        assert_eq!(no_room.affordable_count(SEARCH_EVENT, 1).unwrap(), 0);
    }

    #[tokio::test]
    async fn scrappa_http_errors_keep_structured_details_and_do_not_retry() {
        let server = MockServer::start(vec![response(
            503,
            r#"{"message":"upstream busy","errors":{"query":["try later"]}}"#,
        )]);
        let config = config(&server);
        let error = fetch_search(&client(APIFY_REQUEST_TIMEOUT), &config, &test_input())
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (503): upstream busy - query: try later"
        );
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn scrappa_request_keeps_the_sixty_second_deadline_message() {
        let server = MockServer::start(vec![delayed_response(
            200,
            r#"{"items":[]}"#,
            Duration::from_millis(100),
        )]);
        let mut config = config(&server);
        config.scrappa_request_timeout = Duration::from_millis(20);
        let error = fetch_search(&client(APIFY_REQUEST_TIMEOUT), &config, &test_input())
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API request timed out after 20ms"
        );
    }

    #[tokio::test]
    async fn apify_storage_reads_retry_transient_server_errors() {
        let server = MockServer::start(vec![
            response(503, "temporary"),
            response(200, r#"{"query":"coffee shops","zoom":15}"#),
        ]);
        let config = config(&server);
        let http = client(APIFY_REQUEST_TIMEOUT);
        let input = ApifyClient {
            http: &http,
            config: &config,
        }
        .get_input()
        .await
        .unwrap();
        assert_eq!(input["query"], "coffee shops");
        assert_eq!(server.requests().len(), 2);
    }

    #[test]
    fn dataset_writes_split_batches_below_apify_payload_limit() {
        let items = vec![
            json!({"payload":"a".repeat(2_400_000)}),
            json!({"payload":"b".repeat(2_400_000)}),
        ];
        let batches = dataset_batches(&items).unwrap();
        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 1);
        assert_eq!(batches[1].len(), 1);
    }

    #[test]
    fn missing_query_or_zoom_keeps_actor_validation_error() {
        let base_url = Url::parse(SCRAPPA_API_DEFAULT).unwrap();
        for input in [json!({}), json!({"query":""}), json!({"query":"coffee"})] {
            assert_eq!(
                build_search_url(&input, &base_url).unwrap_err().to_string(),
                "Search query and zoom level are required"
            );
        }
    }
}
