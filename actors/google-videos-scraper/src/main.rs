use std::{collections::HashSet, env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response};
use serde_json::{json, Map, Number, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_RETRY_DELAYS: [Duration; 2] = [Duration::from_secs(1), Duration::from_secs(2)];
const TRANSIENT_SCRAPPA_STATUSES: [u16; 6] = [408, 429, 500, 502, 503, 504];
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const MAX_QUERIES_PER_RUN: usize = 10;
const REQUEST_ENRICHMENT_FIELDS: [&str; 13] = [
    "q",
    "page",
    "start",
    "hl",
    "gl",
    "google_domain",
    "location",
    "uule",
    "tbs",
    "safe",
    "filter",
    "nfpr",
    "lr",
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
        .context("Failed to read API response")?;
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

#[derive(Default)]
struct DatasetBudget {
    initial_dataset_items: Option<u64>,
    saved_dataset_items: u64,
}

fn affordable_dataset_items(
    run: &Value,
    requested: usize,
    budget: &mut DatasetBudget,
) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        return Ok(requested);
    }
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => return Ok(requested),
        Some(value) => value
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?,
    };
    if max_charge == 0.0 {
        return Ok(requested);
    }
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
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
    if !item_price.is_finite() || item_price < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let current_dataset_items = counts
        .get(DATASET_ITEM_EVENT)
        .map(|count| {
            count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {DATASET_ITEM_EVENT}"))
        })
        .transpose()?
        .unwrap_or(0);
    let initial_dataset_items = *budget
        .initial_dataset_items
        .get_or_insert(current_dataset_items);
    let local_dataset_items = initial_dataset_items
        .checked_add(budget.saved_dataset_items)
        .ok_or_else(|| anyhow!("Dataset row count overflowed"))?;

    let mut spent = 0.0;
    let mut saw_dataset_count = false;
    for (event_name, count) in counts {
        let mut count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if event_name == DATASET_ITEM_EVENT {
            saw_dataset_count = true;
            count = count.max(local_dataset_items);
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
    if !saw_dataset_count && local_dataset_items > 0 {
        spent += item_price * local_dataset_items as f64;
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

struct DatasetWrite {
    saved_count: usize,
    charge_limit_reached: bool,
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

    async fn push_dataset_items(
        &self,
        items: &[Value],
        budget: &mut DatasetBudget,
    ) -> Result<DatasetWrite> {
        if items.is_empty() {
            return Ok(DatasetWrite {
                saved_count: 0,
                charge_limit_reached: false,
            });
        }

        let run_url = self.endpoint(&["v2", "actor-runs", &self.config.actor_run_id])?;
        let run_response = self
            .http
            .get(run_url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = response_json(run_response, "Apify run pricing request").await?;
        let limit = affordable_dataset_items(&run, items.len(), budget)?;
        let affordable_items = &items[..limit];
        if affordable_items.is_empty() {
            return Ok(DatasetWrite {
                saved_count: 0,
                charge_limit_reached: true,
            });
        }

        let saved_count =
            u64::try_from(affordable_items.len()).context("Dataset row count is too large")?;
        let new_saved_count = budget
            .saved_dataset_items
            .checked_add(saved_count)
            .ok_or_else(|| anyhow!("Dataset row count overflowed"))?;
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(affordable_items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await?;
        budget.saved_dataset_items = new_saved_count;

        Ok(DatasetWrite {
            saved_count: affordable_items.len(),
            charge_limit_reached: limit < items.len(),
        })
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

#[derive(Debug)]
struct ScrappaTimeoutError {
    timeout_ms: u128,
}

impl std::fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            self.timeout_ms
        )
    }
}

impl std::error::Error for ScrappaTimeoutError {}

fn scrappa_request_error(error: reqwest::Error, timeout: Duration) -> anyhow::Error {
    if error.is_timeout() {
        anyhow::Error::new(ScrappaTimeoutError {
            timeout_ms: timeout.as_millis(),
        })
    } else {
        anyhow::Error::new(error)
    }
}

fn is_retryable_scrappa_error(error: &anyhow::Error) -> bool {
    if error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| TRANSIENT_SCRAPPA_STATUSES.contains(&error.status))
    {
        return true;
    }
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return true;
    }
    error
        .downcast_ref::<reqwest::Error>()
        .is_some_and(|error| error.is_connect() || error.is_body())
}

fn scrappa_error(status: u16, body: &str) -> ScrappaApiError {
    let Ok(data) = serde_json::from_str::<Value>(body) else {
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
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

struct ScrappaClient<'a> {
    http: &'a Client,
    config: &'a Config,
    request_timeout: Duration,
    retry_delays: &'a [Duration],
}

impl ScrappaClient<'_> {
    async fn get_google_videos(&self, params: &Map<String, Value>) -> Result<Value> {
        let mut url = endpoint_url(&self.config.scrappa_api_base, &["google", "videos"])?;
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, &js_string(value));
        }

        let max_attempts = self.retry_delays.len() + 1;
        for attempt in 1..=max_attempts {
            let result = self.request_once(&url).await;
            match result {
                Ok(response) => return Ok(response),
                Err(error) if attempt < max_attempts && is_retryable_scrappa_error(&error) => {
                    eprintln!(
                        "Transient Scrappa API failure; retrying attempt {}/{}",
                        attempt + 1,
                        max_attempts
                    );
                    tokio::time::sleep(self.retry_delays[attempt - 1]).await;
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("at least one Scrappa request attempt is configured")
    }

    async fn request_once(&self, url: &Url) -> Result<Value> {
        let response = self
            .http
            .get(url.clone())
            .timeout(self.request_timeout)
            .header("X-API-Key", &self.config.scrappa_api_key)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| scrappa_request_error(error, self.request_timeout))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|error| scrappa_request_error(error, self.request_timeout))?;
        if !status.is_success() {
            return Err(scrappa_error(status.as_u16(), &body).into());
        }
        serde_json::from_str(&body)
            .map_err(|error| anyhow!("Scrappa API response was not valid JSON: {error}"))
    }
}

fn clean_optional_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(value.to_owned()))
}

fn clean_two_letter_code(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    let Some(value) = clean_optional_string(value, field, 2)? else {
        return Ok(None);
    };
    if value.len() != 2 || !value.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        bail!("{field} must be a two-letter code");
    }
    Ok(Some(value.to_ascii_lowercase()))
}

fn clean_integer(value: Option<&Value>, field: &str, min: i64) -> Result<Option<Value>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(number) = value.as_number() else {
        bail!("{field} must be an integer");
    };
    let numeric = number
        .as_i64()
        .map(|value| value as f64)
        .or_else(|| number.as_u64().map(|value| value as f64))
        .or_else(|| number.as_f64().filter(|value| value.is_finite()))
        .ok_or_else(|| anyhow!("{field} must be an integer"))?;
    if numeric.fract() != 0.0 {
        bail!("{field} must be an integer");
    }
    if numeric < min as f64 {
        bail!("{field} must be greater than or equal to {min}");
    }

    let normalized = if numeric >= i64::MIN as f64 && numeric <= i64::MAX as f64 {
        Number::from(numeric as i64)
    } else if numeric >= 0.0 && numeric <= u64::MAX as f64 {
        Number::from(numeric as u64)
    } else {
        Number::from_f64(numeric).ok_or_else(|| anyhow!("{field} must be an integer"))?
    };
    Ok(Some(Value::Number(normalized)))
}

fn clean_zero_one(value: Option<&Value>, field: &str) -> Result<Option<Value>> {
    let Some(cleaned) = clean_integer(value, field, 0)? else {
        return Ok(None);
    };
    if cleaned.as_i64() != Some(0) && cleaned.as_i64() != Some(1) {
        bail!("{field} must be 0 or 1");
    }
    Ok(Some(cleaned))
}

fn clean_enum(value: Option<&Value>, field: &str, allowed: &[&str]) -> Result<Option<String>> {
    let Some(value) = clean_optional_string(value, field, 100)? else {
        return Ok(None);
    };
    let normalized = value.to_ascii_lowercase();
    if !allowed.contains(&normalized.as_str()) {
        bail!("{field} must be one of: {}", allowed.join(", "));
    }
    Ok(Some(normalized))
}

fn get_google_videos_queries(input: &Map<String, Value>) -> Result<Vec<String>> {
    let mut raw_queries = Vec::new();
    if let Some(query) = input
        .get("q")
        .filter(|value| !value.is_null() && value.as_str() != Some(""))
    {
        raw_queries.push(query);
    }
    if let Some(queries) = input
        .get("queries")
        .filter(|value| !value.is_null() && value.as_str() != Some(""))
    {
        let Some(queries) = queries.as_array() else {
            bail!("queries must be an array");
        };
        if queries.len() > MAX_QUERIES_PER_RUN {
            bail!("queries must contain {MAX_QUERIES_PER_RUN} items or fewer");
        }
        raw_queries.extend(queries.iter());
    }

    let mut seen = HashSet::new();
    let mut queries = Vec::new();
    for raw_query in raw_queries {
        let Some(query) = clean_optional_string(Some(raw_query), "q", 500)? else {
            continue;
        };
        if seen.insert(query.clone()) {
            queries.push(query);
        }
    }
    if queries.is_empty() {
        bail!("q is required");
    }
    if queries.len() > MAX_QUERIES_PER_RUN {
        bail!("queries must contain {MAX_QUERIES_PER_RUN} items or fewer");
    }
    Ok(queries)
}

fn build_google_videos_params(
    input: &Map<String, Value>,
    query: &str,
) -> Result<Map<String, Value>> {
    let page = clean_integer(input.get("page"), "page", 1)?;
    let start = clean_integer(input.get("start"), "start", 0)?;
    if page.is_some() && start.is_some() {
        bail!("Cannot use both page and start parameters");
    }

    let location = clean_optional_string(input.get("location"), "location", 500)?;
    let uule = clean_optional_string(input.get("uule"), "uule", 500)?;
    if location.is_some() && uule.is_some() {
        bail!("Cannot use both location and uule parameters");
    }

    let hl = clean_two_letter_code(input.get("hl"), "hl")?;
    let gl = clean_two_letter_code(input.get("gl"), "gl")?;
    let google_domain = clean_optional_string(input.get("google_domain"), "google_domain", 50)?;
    let tbs = clean_optional_string(input.get("tbs"), "tbs", 500)?;
    let safe = clean_enum(input.get("safe"), "safe", &["active", "off"])?;
    let filter = clean_zero_one(input.get("filter"), "filter")?;
    let nfpr = clean_zero_one(input.get("nfpr"), "nfpr")?;
    let lr = clean_optional_string(input.get("lr"), "lr", 200)?;

    let mut params = Map::new();
    params.insert("q".to_owned(), Value::String(query.to_owned()));
    if let Some(page) = page {
        params.insert("page".to_owned(), page);
    }
    if let Some(start) = start {
        params.insert("start".to_owned(), start);
    }
    if let Some(hl) = hl {
        params.insert("hl".to_owned(), Value::String(hl));
    }
    if let Some(gl) = gl {
        params.insert("gl".to_owned(), Value::String(gl));
    }
    if let Some(google_domain) = google_domain {
        params.insert("google_domain".to_owned(), Value::String(google_domain));
    }
    if let Some(location) = location {
        params.insert("location".to_owned(), Value::String(location));
    }
    if let Some(uule) = uule {
        params.insert("uule".to_owned(), Value::String(uule));
    }
    if let Some(tbs) = tbs {
        params.insert("tbs".to_owned(), Value::String(tbs));
    }
    if let Some(safe) = safe {
        params.insert("safe".to_owned(), Value::String(safe));
    }
    if let Some(filter) = filter {
        params.insert("filter".to_owned(), filter);
    }
    if let Some(nfpr) = nfpr {
        params.insert("nfpr".to_owned(), nfpr);
    }
    if let Some(lr) = lr {
        params.insert("lr".to_owned(), Value::String(lr));
    }
    Ok(params)
}

fn build_google_videos_param_list(input: &Value) -> Result<Vec<Map<String, Value>>> {
    let Some(input) = input.as_object() else {
        bail!("Input must be an object");
    };
    get_google_videos_queries(input)?
        .iter()
        .map(|query| build_google_videos_params(input, query))
        .collect()
}

fn describe_google_videos_request(params: &Map<String, Value>) -> String {
    let mut suffix_parts = Vec::new();
    if let Some(page) = params.get("page") {
        suffix_parts.push(format!("page {}", js_string(page)));
    }
    if let Some(start) = params.get("start") {
        suffix_parts.push(format!("start {}", js_string(start)));
    }
    for field in [
        "google_domain",
        "hl",
        "gl",
        "location",
        "uule",
        "tbs",
        "safe",
        "filter",
        "nfpr",
        "lr",
    ] {
        if let Some(value) = params.get(field) {
            suffix_parts.push(format!("{field}={}", js_string(value)));
        }
    }
    let suffix = if suffix_parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", suffix_parts.join(", "))
    };
    let query = params
        .get("q")
        .and_then(Value::as_str)
        .unwrap_or("unknown query");
    format!("query \"{query}\"{suffix}")
}

fn extract_video_results(response: &Value) -> Vec<Value> {
    if let Some(results) = response.as_array() {
        return results.clone();
    }
    if let Some(payload) = response.as_object() {
        if let Some(results) = payload.get("video_results").and_then(Value::as_array) {
            return results.clone();
        }
        if let Some(results) = payload.get("data").and_then(Value::as_array) {
            return results.clone();
        }
    }
    eprintln!("Scrappa Google Videos response did not include a video result array");
    Vec::new()
}

fn response_array_or_string_length(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(values)) => values.len(),
        Some(Value::String(value)) => value.encode_utf16().count(),
        _ => 0,
    }
}

fn enrich_result(result: &Value, params: &Map<String, Value>) -> Result<Value> {
    let Some(result_object) = result.as_object() else {
        bail!("Scrappa API video_results items must be objects");
    };
    let mut enriched = result_object.clone();
    let value_or_null = |field: &str| {
        result_object
            .get(field)
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null)
    };
    let video_url = result_object
        .get("link")
        .filter(|value| !value.is_null())
        .or_else(|| {
            result_object
                .get("video_link")
                .filter(|value| !value.is_null())
        })
        .cloned()
        .unwrap_or(Value::Null);
    enriched.insert("position".to_owned(), value_or_null("position"));
    enriched.insert("title".to_owned(), value_or_null("title"));
    enriched.insert("video_url".to_owned(), video_url);
    enriched.insert(
        "google_redirect_url".to_owned(),
        value_or_null("video_link"),
    );
    enriched.insert("source_url".to_owned(), value_or_null("link"));
    enriched.insert("displayed_link".to_owned(), value_or_null("displayed_link"));
    enriched.insert("thumbnail_url".to_owned(), value_or_null("thumbnail"));
    enriched.insert("snippet".to_owned(), value_or_null("snippet"));
    enriched.insert("duration".to_owned(), value_or_null("duration"));
    enriched.insert("date".to_owned(), value_or_null("date"));
    let key_moments_count = result_object
        .get("key_moments")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    enriched.insert("key_moments_count".to_owned(), json!(key_moments_count));
    for field in REQUEST_ENRICHMENT_FIELDS {
        enriched.insert(
            format!("request_{field}"),
            params.get(field).cloned().unwrap_or(Value::Null),
        );
    }
    Ok(Value::Object(enriched))
}

fn actor_error_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if message.contains("timed out") {
        format!(
            "{message}. The Google Videos request exceeded the {}s Scrappa API timeout. Try a more specific query or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

async fn run_actor(http: &Client, config: &Config) -> Result<Option<String>> {
    let apify = ApifyClient { http, config };
    let input = apify.get_input().await?;
    if input.is_null() {
        bail!("Input is required");
    }
    let param_list = build_google_videos_param_list(&input)?;
    println!(
        "Running {} Google Videos request{}",
        param_list.len(),
        if param_list.len() == 1 { "" } else { "s" }
    );

    let scrappa = ScrappaClient {
        http,
        config,
        request_timeout: SCRAPPA_REQUEST_TIMEOUT,
        retry_delays: &SCRAPPA_RETRY_DELAYS,
    };
    let keep_raw_response = param_list.len() == 1;
    let mut single_response = None;
    let mut request_summaries = Vec::with_capacity(param_list.len());
    let mut total_video_results = 0;
    let mut total_found_in_videos = 0;
    let mut total_short_videos = 0;
    let mut total_related_searches = 0;
    let mut has_pagination = false;
    let mut has_scrappa_pagination = false;
    let mut dataset_budget = DatasetBudget::default();
    let mut status_message = None;

    for params in &param_list {
        println!(
            "Fetching Google Videos for {}",
            describe_google_videos_request(params)
        );
        let response = scrappa.get_google_videos(params).await?;
        if keep_raw_response {
            single_response = Some(response.clone());
        }

        let video_results = extract_video_results(&response);
        let dataset_items = video_results
            .iter()
            .map(|result| enrich_result(result, params))
            .collect::<Result<Vec<_>>>()?;
        let mut saved_video_results = video_results.len();
        total_found_in_videos += response_array_or_string_length(response.get("found_in_videos"));
        total_short_videos += response_array_or_string_length(response.get("short_videos"));
        total_related_searches += response_array_or_string_length(response.get("related_searches"));
        has_pagination |= response.get("pagination").is_some_and(js_truthy);
        has_scrappa_pagination |= response.get("scrappa_pagination").is_some_and(js_truthy);

        if !dataset_items.is_empty() {
            let write = apify
                .push_dataset_items(&dataset_items, &mut dataset_budget)
                .await?;
            saved_video_results = write.saved_count;
            if write.charge_limit_reached {
                let message = format!(
                    "Charge limit reached after saving {saved_video_results} of {} Google Videos results; OUTPUT will be written before exit.",
                    dataset_items.len()
                );
                println!(
                    "{message} {}",
                    json!({
                        "charged_count": write.saved_count,
                        "requested_count": dataset_items.len(),
                    })
                );
                status_message = Some(message);
            }
            println!(
                "Found {} video results; saved {} dataset items",
                video_results.len(),
                saved_video_results
            );
        } else {
            println!("No Google Videos results found for this request");
        }

        total_video_results += saved_video_results;
        request_summaries.push(json!({
            "request": params,
            "video_results": saved_video_results,
        }));
        if status_message.is_some() {
            break;
        }
    }

    let output = if keep_raw_response {
        single_response.ok_or_else(|| anyhow!("Google Videos response was not available"))?
    } else {
        json!({
            "requests": request_summaries,
            "video_results": total_video_results,
        })
    };
    apify.put_output(&output).await?;

    println!("Google Videos scraping completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string(&json!({
            "requests": param_list.len(),
            "video_results": total_video_results,
            "found_in_videos": total_found_in_videos,
            "short_videos": total_short_videos,
            "related_searches": total_related_searches,
            "has_pagination": has_pagination,
            "has_scrappa_pagination": has_scrappa_pagination,
        }))?
    );
    Ok(status_message)
}

#[tokio::main]
async fn main() {
    let result = async {
        let config = Config::from_env()?;
        let http = Client::new();
        run_actor(&http, &config).await
    }
    .await;
    match result {
        Ok(Some(status_message)) => println!("{status_message}"),
        Ok(None) => {}
        Err(error) => {
            eprintln!("Actor failed: {}", actor_error_message(&error));
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
        thread::{self, JoinHandle},
    };

    const INPUT_SCHEMA: &str = include_str!("../.actor/input_schema.json");

    struct MockResponse {
        status: u16,
        body: String,
        delay: Duration,
    }

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
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                let workers = responses
                    .into_iter()
                    .map(|response| {
                        let (mut stream, _) = listener.accept().unwrap();
                        let sender = sender.clone();
                        thread::spawn(move || {
                            let request = read_mock_request(&mut stream);
                            sender.send(request).unwrap();
                            thread::sleep(response.delay);
                            let reason = match response.status {
                                200 => "OK",
                                201 => "Created",
                                400 => "Bad Request",
                                401 => "Unauthorized",
                                403 => "Forbidden",
                                404 => "Not Found",
                                408 => "Request Timeout",
                                429 => "Too Many Requests",
                                500 => "Internal Server Error",
                                502 => "Bad Gateway",
                                503 => "Service Unavailable",
                                504 => "Gateway Timeout",
                                _ => "Error",
                            };
                            let response_body = response.body;
                            let _ = write!(
                                stream,
                                "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                response.status,
                                reason,
                                response_body.len(),
                                response_body
                            );
                        })
                    })
                    .collect::<Vec<_>>();
                for worker in workers {
                    worker.join().unwrap();
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

    fn mock_response(status: u16, body: Value) -> MockResponse {
        MockResponse {
            status,
            body: body.to_string(),
            delay: Duration::ZERO,
        }
    }

    fn delayed_response(status: u16, body: Value, delay: Duration) -> MockResponse {
        MockResponse {
            status,
            body: body.to_string(),
            delay,
        }
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

    fn test_config(base_url: &str) -> Config {
        Config {
            apify_api_base: Url::parse(base_url).unwrap(),
            scrappa_api_base: Url::parse(base_url).unwrap(),
            apify_token: "apify-test-token".to_owned(),
            key_value_store_id: "store-id".to_owned(),
            dataset_id: "dataset-id".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
        }
    }

    fn pricing_response(max_charge: f64, dataset_items_charged: u64) -> MockResponse {
        mock_response(
            200,
            json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            "apify-actor-start": {"eventPriceUsd": 0.00005}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_charge},
                    "chargedEventCounts": {
                        "apify-actor-start": 1,
                        "apify-default-dataset-item": dataset_items_charged
                    }
                }
            }),
        )
    }

    fn input_request() -> String {
        "/v2/key-value-stores/store-id/records/INPUT".to_owned()
    }

    fn run_request() -> String {
        "/v2/actor-runs/test-run".to_owned()
    }

    fn output_request() -> String {
        "/v2/key-value-stores/store-id/records/OUTPUT".to_owned()
    }

    fn input_with_queries() -> Value {
        json!({
            "q": "coffee",
            "queries": [" coffee ", "espresso"],
            "page": 1,
            "hl": "EN",
            "gl": "US",
            "google_domain": "google.com",
            "safe": "off"
        })
    }

    fn video_response(title: &str) -> Value {
        json!({
            "video_results": [{
                "position": 1,
                "title": title,
                "link": "https://www.youtube.com/watch?v=example",
                "video_link": "https://www.google.com/url?q=example",
                "key_moments": [{"time": "00:00", "title": "Intro"}]
            }],
            "found_in_videos": [{"title": "found"}],
            "short_videos": [],
            "related_searches": [{"query": "more"}],
            "pagination": {"next": "next-page"},
            "scrappa_pagination": {"next_start": 10},
            "provider_field": {"kept": true}
        })
    }

    fn object_body(request: &MockRequest) -> Value {
        serde_json::from_str(&request.body).unwrap()
    }

    #[test]
    fn input_schema_prefill_and_defaults_still_produce_one_prefilled_request() {
        let schema: Value = serde_json::from_str(INPUT_SCHEMA).unwrap();
        let properties = schema["properties"].as_object().unwrap();
        let mut input = Map::new();
        for (name, property) in properties {
            if let Some(value) = property.get("prefill").or_else(|| property.get("default")) {
                input.insert(name.clone(), value.clone());
            }
        }

        assert_eq!(schema["title"], "Google Videos Scraper");
        assert_eq!(schema["required"], Value::Null);
        assert_eq!(
            schema["properties"]["queries"]["prefill"],
            json!(["coffee brewing tutorial"])
        );
        assert_eq!(schema["properties"]["q"]["prefill"], Value::Null);
        assert_eq!(
            build_google_videos_param_list(&Value::Object(input))
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            build_google_videos_param_list(&json!({"queries": ["coffee", "espresso"]}))
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn request_builder_trims_normalizes_and_deduplicates_queries() {
        let params = build_google_videos_param_list(&json!({
            "q": " coffee brewing tutorial ",
            "queries": ["coffee brewing tutorial", " espresso machine review "],
            "page": 2,
            "hl": "EN",
            "gl": "US",
            "google_domain": " google.com ",
            "location": " Austin, Texas ",
            "tbs": " qdr:w ",
            "safe": "OFF",
            "filter": 1,
            "nfpr": 0,
            "lr": " lang_en "
        }))
        .unwrap();

        assert_eq!(params.len(), 2);
        assert_eq!(params[0]["q"], "coffee brewing tutorial");
        assert_eq!(params[1]["q"], "espresso machine review");
        assert_eq!(params[0]["hl"], "en");
        assert_eq!(params[0]["gl"], "us");
        assert_eq!(params[0]["safe"], "off");
        assert_eq!(params[0]["filter"], 1);
        assert_eq!(params[0]["nfpr"], 0);
        assert_eq!(params[0]["location"], "Austin, Texas");
        assert_eq!(params[0]["tbs"], "qdr:w");
        assert_eq!(params[0]["lr"], "lang_en");
    }

    #[test]
    fn request_builder_keeps_start_pagination_and_location_encoding() {
        let params = build_google_videos_param_list(&json!({
            "q": "coffee",
            "start": 10,
            "uule": "encoded-location"
        }))
        .unwrap();
        assert_eq!(params[0]["start"], 10);
        assert_eq!(params[0]["uule"], "encoded-location");
        assert_eq!(
            describe_google_videos_request(&params[0]),
            "query \"coffee\" (start 10, uule=encoded-location)"
        );
    }

    #[test]
    fn request_builder_rejects_invalid_queries_pagination_and_locations() {
        assert!(
            build_google_videos_param_list(&json!({"queries": "coffee"}))
                .unwrap_err()
                .to_string()
                .contains("queries must be an array")
        );
        assert!(build_google_videos_param_list(&json!({"q": "  "}))
            .unwrap_err()
            .to_string()
            .contains("q is required"));
        assert!(build_google_videos_param_list(&json!({
            "q": "coffee",
            "queries": ["one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"]
        }))
        .unwrap_err()
        .to_string()
        .contains("queries must contain 10 items or fewer"));
        assert!(
            build_google_videos_param_list(&json!({"q": "coffee", "page": 1, "start": 0}))
                .unwrap_err()
                .to_string()
                .contains("Cannot use both page and start parameters")
        );
        assert!(build_google_videos_param_list(
            &json!({"q": "coffee", "location": "Austin", "uule": "encoded"})
        )
        .unwrap_err()
        .to_string()
        .contains("Cannot use both location and uule parameters"));
    }

    #[test]
    fn request_builder_rejects_invalid_types_codes_and_filter_values() {
        assert!(build_google_videos_param_list(&json!({"q": "coffee", "gl": 123})).is_err());
        assert!(
            build_google_videos_param_list(&json!({"q": "coffee", "hl": "eng"}))
                .unwrap_err()
                .to_string()
                .contains("hl must be 2 characters or fewer")
        );
        assert!(
            build_google_videos_param_list(&json!({"q": "coffee", "safe": "moderate"}))
                .unwrap_err()
                .to_string()
                .contains("safe must be one of: active, off")
        );
        assert!(
            build_google_videos_param_list(&json!({"q": "coffee", "filter": 2}))
                .unwrap_err()
                .to_string()
                .contains("filter must be 0 or 1")
        );
        assert!(
            build_google_videos_param_list(&json!({"q": "coffee", "nfpr": 1.5}))
                .unwrap_err()
                .to_string()
                .contains("nfpr must be an integer")
        );
        assert!(build_google_videos_param_list(
            &json!({"q": "coffee", "google_domain": "x".repeat(51)})
        )
        .unwrap_err()
        .to_string()
        .contains("google_domain must be 50 characters or fewer"));
    }

    #[test]
    fn response_extraction_supports_arrays_video_results_and_data() {
        let result = json!({"position": 1, "title": "Coffee Tutorial"});
        assert_eq!(
            extract_video_results(&json!([result.clone()])),
            vec![result.clone()]
        );
        assert_eq!(
            extract_video_results(&json!({"video_results": [result.clone()]})),
            vec![result.clone()]
        );
        assert_eq!(
            extract_video_results(&json!({"data": [result.clone()]})),
            vec![result]
        );
        assert!(extract_video_results(&json!({"results": []})).is_empty());
        assert!(extract_video_results(&Value::Null).is_empty());
    }

    #[test]
    fn dataset_enrichment_preserves_provider_fields_and_adds_compatibility_aliases() {
        let params = build_google_videos_param_list(&json!({
            "q": "coffee",
            "page": 1,
            "hl": "en",
            "gl": "us",
            "google_domain": "google.com",
            "safe": "off"
        }))
        .unwrap()
        .remove(0);
        let enriched = enrich_result(
            &json!({
                "position": 1,
                "title": "Coffee Tutorial",
                "link": "https://www.youtube.com/watch?v=example",
                "video_link": "https://www.google.com/url?q=example",
                "thumbnail": "https://example.com/thumb.jpg",
                "key_moments": [{"time": "00:00"}],
                "provider_field": "kept"
            }),
            &params,
        )
        .unwrap();

        assert_eq!(enriched["provider_field"], "kept");
        assert_eq!(
            enriched["video_url"],
            "https://www.youtube.com/watch?v=example"
        );
        assert_eq!(
            enriched["google_redirect_url"],
            "https://www.google.com/url?q=example"
        );
        assert_eq!(
            enriched["source_url"],
            "https://www.youtube.com/watch?v=example"
        );
        assert_eq!(enriched["thumbnail_url"], "https://example.com/thumb.jpg");
        assert_eq!(enriched["key_moments_count"], 1);
        assert_eq!(enriched["request_q"], "coffee");
        assert_eq!(enriched["request_page"], 1);
        assert_eq!(enriched["request_start"], Value::Null);
        assert_eq!(enriched["request_location"], Value::Null);
    }

    #[test]
    fn dataset_enrichment_uses_google_redirect_when_destination_link_is_missing() {
        let params = build_google_videos_param_list(&json!({"q": "coffee"}))
            .unwrap()
            .remove(0);
        let enriched = enrich_result(
            &json!({"video_link": "https://google.test/redirect"}),
            &params,
        )
        .unwrap();
        assert_eq!(enriched["video_url"], "https://google.test/redirect");
        assert_eq!(enriched["source_url"], Value::Null);
        assert_eq!(enriched["key_moments_count"], 0);
    }

    fn pricing_run(max_charge: Option<Value>, charged_event_counts: Value) -> Value {
        let mut run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                        "apify-actor-start": {"eventPriceUsd": 0.00005},
                        "custom-event": {"eventPriceUsd": 0.0001}
                    }}
                },
                "options": {},
                "chargedEventCounts": charged_event_counts
            }
        });
        if let Some(max_charge) = max_charge {
            run["data"]["options"]["maxTotalChargeUsd"] = max_charge;
        }
        run
    }

    #[test]
    fn zero_missing_and_null_limits_allow_unbounded_prefill_results() {
        let cases = [
            ("zero", Some(json!(0))),
            ("omitted", None),
            ("null", Some(Value::Null)),
        ];

        for (name, max_charge) in cases {
            let run = pricing_run(max_charge, json!({"apify-actor-start": 1}));
            assert_eq!(
                affordable_dataset_items(&run, 3, &mut DatasetBudget::default()).unwrap(),
                3,
                "{name} limit should be unbounded"
            );
        }
    }

    #[test]
    fn positive_limit_keeps_only_the_affordable_prefix_after_custom_charges() {
        let run = pricing_run(Some(json!(0.00035)), json!({"apify-actor-start": 1}));

        assert_eq!(
            affordable_dataset_items(&run, 3, &mut DatasetBudget::default()).unwrap(),
            1
        );
    }

    #[test]
    fn positive_limit_accounts_for_existing_dataset_and_custom_event_charges() {
        let run = pricing_run(
            Some(json!(0.00079)),
            json!({
                "custom-event": 2,
                "apify-default-dataset-item": 1
            }),
        );

        assert_eq!(
            affordable_dataset_items(&run, 3, &mut DatasetBudget::default()).unwrap(),
            0
        );
    }

    #[test]
    fn non_pay_per_event_runs_allow_dataset_rows_without_a_spending_limit() {
        let run = json!({
            "data": {
                "pricingInfo": {"pricingModel": "PAY_PER_ACTOR"}
            }
        });

        assert_eq!(
            affordable_dataset_items(&run, 3, &mut DatasetBudget::default()).unwrap(),
            3
        );
    }

    #[tokio::test]
    async fn scrappa_get_retries_transient_statuses_and_keeps_auth_headers() {
        let server = MockServer::start(vec![
            mock_response(503, json!({"message": "busy"})),
            mock_response(200, json!({"video_results": [{"title": "Espresso"}]})),
        ]);
        let config = test_config(&server.base_url);
        let client = ScrappaClient {
            http: &Client::new(),
            config: &config,
            request_timeout: Duration::from_secs(1),
            retry_delays: &[Duration::ZERO, Duration::ZERO],
        };
        let params = build_google_videos_param_list(&json!({"q": "espresso"}))
            .unwrap()
            .remove(0);

        let response = client.get_google_videos(&params).await.unwrap();
        assert_eq!(response["video_results"][0]["title"], "Espresso");
        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].target, "/google/videos?q=espresso");
        assert_eq!(requests[0].headers["x-api-key"], "scrappa-test-key");
        assert_eq!(requests[0].headers["accept"], "application/json");
    }

    #[tokio::test]
    async fn scrappa_authentication_errors_are_not_retried() {
        let server = MockServer::start(vec![mock_response(401, json!({"message": "denied"}))]);
        let config = test_config(&server.base_url);
        let client = ScrappaClient {
            http: &Client::new(),
            config: &config,
            request_timeout: Duration::from_secs(1),
            retry_delays: &[Duration::ZERO, Duration::ZERO],
        };
        let params = build_google_videos_param_list(&json!({"q": "coffee"}))
            .unwrap()
            .remove(0);

        let error = client.get_google_videos(&params).await.unwrap_err();
        assert!(error
            .to_string()
            .contains("Scrappa API error (401): denied"));
        let requests = server.finish();
        assert_eq!(requests.len(), 1);
    }

    #[tokio::test]
    async fn scrappa_timeout_covers_the_response_body_and_uses_three_total_attempts() {
        let server = MockServer::start(vec![
            delayed_response(
                200,
                json!({"video_results": []}),
                Duration::from_millis(120),
            ),
            delayed_response(
                200,
                json!({"video_results": []}),
                Duration::from_millis(120),
            ),
            delayed_response(
                200,
                json!({"video_results": []}),
                Duration::from_millis(120),
            ),
        ]);
        let config = test_config(&server.base_url);
        let client = ScrappaClient {
            http: &Client::new(),
            config: &config,
            request_timeout: Duration::from_millis(30),
            retry_delays: &[Duration::ZERO, Duration::ZERO],
        };
        let params = build_google_videos_param_list(&json!({"q": "coffee"}))
            .unwrap()
            .remove(0);

        let error = client.get_google_videos(&params).await.unwrap_err();
        assert!(error.to_string().contains("timed out after 30ms"));
        assert_eq!(server.finish().len(), 3);
    }

    #[tokio::test]
    async fn single_query_writes_one_dataset_batch_and_the_unmodified_raw_response() {
        let response = video_response("Coffee Tutorial");
        let server = MockServer::start(vec![
            mock_response(200, json!({"q": "coffee", "page": 2, "gl": "US"})),
            mock_response(200, response.clone()),
            pricing_response(1.0, 0),
            mock_response(200, json!({})),
            mock_response(201, json!({})),
        ]);
        let config = test_config(&server.base_url);

        assert_eq!(run_actor(&Client::new(), &config).await.unwrap(), None);
        let requests = server.finish();
        assert_eq!(requests.len(), 5);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].target, input_request());
        assert_eq!(
            requests[0].headers["authorization"],
            "Bearer apify-test-token"
        );
        assert_eq!(requests[1].method, "GET");
        assert_eq!(requests[1].target, "/google/videos?q=coffee&page=2&gl=us");
        assert_eq!(requests[1].headers["x-api-key"], "scrappa-test-key");
        assert_eq!(requests[2].target, run_request());
        assert_eq!(requests[3].method, "POST");
        assert_eq!(requests[3].target, "/v2/datasets/dataset-id/items");
        let dataset = object_body(&requests[3]);
        assert_eq!(dataset.as_array().unwrap().len(), 1);
        assert_eq!(dataset[0]["title"], "Coffee Tutorial");
        assert_eq!(
            dataset[0]["video_url"],
            "https://www.youtube.com/watch?v=example"
        );
        assert_eq!(dataset[0]["request_q"], "coffee");
        assert_eq!(requests[4].method, "PUT");
        assert_eq!(requests[4].target, output_request());
        assert_eq!(object_body(&requests[4]), response);
    }

    #[tokio::test]
    async fn batch_queries_keep_order_and_write_compact_output_summaries() {
        let server = MockServer::start(vec![
            mock_response(200, input_with_queries()),
            mock_response(200, video_response("Coffee Tutorial")),
            pricing_response(1.0, 0),
            mock_response(200, json!({})),
            mock_response(200, json!({"data": [{"title": "Espresso Review"}]})),
            pricing_response(1.0, 1),
            mock_response(200, json!({})),
            mock_response(201, json!({})),
        ]);
        let config = test_config(&server.base_url);

        assert_eq!(run_actor(&Client::new(), &config).await.unwrap(), None);
        let requests = server.finish();
        assert_eq!(requests.len(), 8);
        assert_eq!(
            requests[1].target,
            "/google/videos?q=coffee&page=1&hl=en&gl=us&google_domain=google.com&safe=off"
        );
        assert_eq!(
            requests[4].target,
            "/google/videos?q=espresso&page=1&hl=en&gl=us&google_domain=google.com&safe=off"
        );
        assert_eq!(
            object_body(&requests[7]),
            json!({
                "requests": [
                    {"request": {"q": "coffee", "page": 1, "hl": "en", "gl": "us", "google_domain": "google.com", "safe": "off"}, "video_results": 1},
                    {"request": {"q": "espresso", "page": 1, "hl": "en", "gl": "us", "google_domain": "google.com", "safe": "off"}, "video_results": 1}
                ],
                "video_results": 2
            })
        );
    }

    #[tokio::test]
    async fn charge_limit_saves_affordable_rows_writes_output_and_stops_batch_requests() {
        let server = MockServer::start(vec![
            mock_response(200, json!({"q": "coffee", "queries": ["espresso"]})),
            mock_response(
                200,
                json!({"video_results": [
                    {"title": "First"}, {"title": "Second"}
                ]}),
            ),
            pricing_response(0.00035, 0),
            mock_response(200, json!({})),
            mock_response(201, json!({})),
        ]);
        let config = test_config(&server.base_url);

        let status = run_actor(&Client::new(), &config).await.unwrap().unwrap();
        assert!(status.contains("Charge limit reached after saving 1 of 2"));
        let requests = server.finish();
        assert_eq!(requests.len(), 5);
        assert!(requests[3]
            .target
            .starts_with("/v2/datasets/dataset-id/items"));
        assert_eq!(object_body(&requests[3]).as_array().unwrap().len(), 1);
        assert_eq!(
            object_body(&requests[4]),
            json!({
                "requests": [{"request": {"q": "coffee"}, "video_results": 1}],
                "video_results": 1
            })
        );
    }

    #[tokio::test]
    async fn empty_results_skip_pricing_and_still_save_raw_output() {
        let response = json!({
            "video_results": [],
            "found_in_videos": [],
            "pagination": null,
            "provider_field": "preserved"
        });
        let server = MockServer::start(vec![
            mock_response(200, json!({"q": "coffee"})),
            mock_response(200, response.clone()),
            mock_response(201, json!({})),
        ]);
        let config = test_config(&server.base_url);

        run_actor(&Client::new(), &config).await.unwrap();
        let requests = server.finish();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[2].target, output_request());
        assert_eq!(object_body(&requests[2]), response);
    }

    #[tokio::test]
    async fn authentication_failure_fails_the_actor_before_writing_output() {
        let server = MockServer::start(vec![
            mock_response(200, json!({"queries": ["one", "two"]})),
            mock_response(403, json!({"message": "forbidden"})),
        ]);
        let config = test_config(&server.base_url);

        let error = run_actor(&Client::new(), &config).await.unwrap_err();
        assert!(error
            .to_string()
            .contains("Scrappa API error (403): forbidden"));
        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert!(requests
            .iter()
            .all(|request| request.target != output_request()));
    }
}
