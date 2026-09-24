use std::{
    env,
    process::ExitCode,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_USER_AGENT: &str = "thescrappa-arbeitsagentur-jobs-scraper/1.0";
const APIFY_TIMEOUT: Duration = Duration::from_secs(30);
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const SCRAPPA_MAX_ATTEMPTS: usize = 4;
const SCRAPPA_MAX_RETRY_DELAY: Duration = Duration::from_secs(20);
const SCRAPPA_REQUEST_DEADLINE: Duration = Duration::from_secs(180);
const APIFY_MAX_RETRIES: usize = 2;
#[cfg(test)]
const ACTOR_TIMEOUT_MS: u64 = 240_000;
#[cfg(test)]
const ACTOR_COMPLETION_RESERVE_MS: u64 = 30_000;

const INPUT_KEYS: &[&str] = &[
    "was",
    "wo",
    "berufsfeld",
    "arbeitgeber",
    "angebotsart",
    "arbeitszeit",
    "befristung",
    "veroeffentlichtseit",
    "umkreis",
    "zeitarbeit",
    "pav",
    "page",
    "size",
];

#[derive(Debug)]
struct Config {
    apify_api_base: String,
    apify_token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_base: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify_api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_base: env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
            scrappa_api_key,
        })
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

struct ApifyClient {
    http: Client,
    base_url: String,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    fn new(http: Client, config: &Config) -> Self {
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

    fn endpoint(&self, path: &[&str]) -> Result<Url> {
        endpoint_url(&self.base_url, path)
    }

    async fn get_input(&self) -> Result<Option<Value>> {
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
                .context("Failed to retrieve actor input from Apify API")?;

            if response.status() == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if let Some(delay) = apify_retry_delay("GET", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            let response = require_apify_success(response, "input retrieval").await?;
            return response
                .json::<Value>()
                .await
                .context("Apify input record was not valid JSON")
                .map(Some);
        }
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<usize> {
        if items.is_empty() {
            return Ok(0);
        }

        let capacity = self.run_dataset_capacity(items.len()).await?;
        let affordable_items = &items[..capacity];
        if affordable_items.is_empty() {
            return Ok(0);
        }

        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(affordable_items)
            .send()
            .await
            .context("Failed to publish dataset item to Apify API")?;
        require_apify_success(response, "dataset item publication").await?;
        Ok(affordable_items.len())
    }

    async fn run_dataset_capacity(&self, requested: usize) -> Result<usize> {
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
            let response = require_apify_success(response, "run pricing request").await?;
            let run = response
                .json::<Value>()
                .await
                .context("Apify run pricing response was not valid JSON")?;
            return affordable_dataset_items(&run, requested);
        }
    }

    async fn put_record(&self, key: &str, value: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            key,
        ])?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .put(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .json(value)
                .send()
                .await
                .with_context(|| format!("Failed to write {key} record to Apify API"))?;

            if let Some(delay) = apify_retry_delay("PUT", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            require_apify_success(response, &format!("{key} record publication")).await?;
            return Ok(());
        }
    }
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
        .get("apify-default-dataset-item")
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
    if !item_price.is_finite() || item_price < 0.0 || !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
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

struct ScrappaClient {
    http: Client,
    base_url: String,
    api_key: String,
    policy: ScrappaRetryPolicy,
}

#[derive(Clone, Copy)]
struct ScrappaRetryPolicy {
    attempts: usize,
    request_timeout: Duration,
    max_retry_delay: Duration,
    request_deadline: Duration,
}

impl ScrappaRetryPolicy {
    fn production() -> Self {
        Self {
            attempts: SCRAPPA_MAX_ATTEMPTS,
            request_timeout: SCRAPPA_REQUEST_TIMEOUT,
            max_retry_delay: SCRAPPA_MAX_RETRY_DELAY,
            request_deadline: SCRAPPA_REQUEST_DEADLINE,
        }
    }
}

impl ScrappaClient {
    fn new(http: Client, base_url: String, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
            policy: ScrappaRetryPolicy::production(),
        }
    }

    async fn get_jobs(&self, params: &Map<String, Value>) -> Result<Value> {
        let url = build_jobs_url(&self.base_url, params)?;
        let deadline = Instant::now() + self.policy.request_deadline;

        for attempt in 1..=self.policy.attempts {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(ScrappaFailure::Timeout.into());
            }
            let timeout = self.policy.request_timeout.min(remaining);
            let result = self.send(&url, timeout).await;
            match result {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if attempt == self.policy.attempts || !error.is_retryable() {
                        return Err(error.into());
                    }

                    let retry_after = match &error {
                        ScrappaFailure::Api {
                            status,
                            retry_after_ms,
                            ..
                        } => retry_after_ms.or_else(|| (*status == 503).then_some(20_000)),
                        _ => None,
                    };
                    let delay_ms = get_retry_delay_ms(
                        attempt,
                        retry_jitter_ms(),
                        retry_after,
                        duration_millis(self.policy.max_retry_delay),
                    );
                    let delay = Duration::from_millis(delay_ms);
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if delay >= remaining {
                        return Err(ScrappaFailure::Timeout.into());
                    }
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{}, in {delay_ms}ms.",
                        attempt + 1,
                        self.policy.attempts
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }

        unreachable!("the retry loop returns after its configured attempts")
    }

    async fn send(
        &self,
        url: &Url,
        timeout: Duration,
    ) -> std::result::Result<Value, ScrappaFailure> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, SCRAPPA_USER_AGENT)
            .timeout(timeout)
            .send()
            .await
            .map_err(ScrappaFailure::from_reqwest)?;

        let status = response.status();
        let retry_after_ms = response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(parse_retry_after_ms);
        let body = response
            .bytes()
            .await
            .map_err(ScrappaFailure::from_reqwest)?;

        if !status.is_success() {
            return Err(ScrappaFailure::Api {
                status: status.as_u16(),
                message: scrappa_error_message(status, &body),
                retry_after_ms,
            });
        }

        serde_json::from_slice(&body)
            .map_err(|error| ScrappaFailure::InvalidJson(error.to_string()))
    }
}

#[derive(Debug)]
enum ScrappaFailure {
    Timeout,
    Api {
        status: u16,
        message: String,
        retry_after_ms: Option<u64>,
    },
    Transport(String),
    InvalidJson(String),
}

impl ScrappaFailure {
    fn from_reqwest(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::Timeout
        } else {
            Self::Transport(error.to_string())
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::Timeout => true,
            Self::Api { status, .. } => matches!(*status, 408 | 429 | 500 | 502 | 503 | 504),
            Self::Transport(_) | Self::InvalidJson(_) => false,
        }
    }
}

impl std::fmt::Display for ScrappaFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(
                formatter,
                "Scrappa API request timed out after {}ms",
                SCRAPPA_REQUEST_TIMEOUT.as_millis()
            ),
            Self::Api {
                status, message, ..
            } => write!(formatter, "Scrappa API error ({status}): {message}"),
            Self::Transport(message) | Self::InvalidJson(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ScrappaFailure {}

fn build_jobs_url(base_url: &str, params: &Map<String, Value>) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["arbeitsagentur", "jobs"])?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            if value.is_null() || value.as_str().is_some_and(str::is_empty) {
                continue;
            }
            query.append_pair(key, &query_value(value));
        }
    }
    Ok(url)
}

fn query_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
        Value::Null => String::new(),
    }
}

fn get_retry_delay_ms(
    failed_attempt: usize,
    jitter_ms: u64,
    retry_after_ms: Option<u64>,
    max_retry_delay_ms: u64,
) -> u64 {
    let exponential_delay_ms = 1_000_u64
        .saturating_mul(2_u64.saturating_pow(failed_attempt.min(63) as u32))
        .saturating_add(jitter_ms);
    exponential_delay_ms
        .max(retry_after_ms.unwrap_or_default())
        .min(max_retry_delay_ms)
}

fn retry_jitter_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| (duration.subsec_nanos() / 1_000_000) as u64)
        .unwrap_or_default()
}

fn parse_retry_after_ms(value: &str) -> Option<u64> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<f64>() {
        if seconds.is_finite() && seconds >= 0.0 {
            return Some((seconds * 1_000.0) as u64);
        }
    }

    let retry_at = httpdate::parse_http_date(value).ok()?;
    Some(
        retry_at
            .duration_since(SystemTime::now())
            .unwrap_or_default()
            .as_millis()
            .min(u64::MAX as u128) as u64,
    )
}

fn duration_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn scrappa_error_message(status: StatusCode, body: &[u8]) -> String {
    let fallback = status.canonical_reason().unwrap_or("HTTP error");
    let body = String::from_utf8_lossy(body);
    if body.is_empty() {
        return fallback.to_owned();
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
        let mut message = error_data
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or_else(|| fallback.to_owned());
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let messages = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .map(js_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {messages}")
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

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
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

fn endpoint_url(base_url: &str, path: &[&str]) -> Result<Url> {
    let mut url = Url::parse(&format!("{}/", base_url.trim_end_matches('/')))
        .with_context(|| format!("Invalid API base URL: {base_url}"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow!("API base URL cannot contain query or fragment: {base_url}"))?;
        segments.pop_if_empty();
        for segment in path {
            segments.push(segment);
        }
    }
    Ok(url)
}

fn normalize_input(input: Option<&Value>) -> Map<String, Value> {
    let mut normalized = Map::new();
    if let Some(fields) = input.and_then(Value::as_object) {
        for (key, value) in fields {
            if !INPUT_KEYS.contains(&key.as_str()) || value.is_null() {
                continue;
            }
            if let Some(value) = value.as_str() {
                let value = value.trim();
                if !value.is_empty() {
                    let value = if key == "arbeitszeit" {
                        value
                            .chars()
                            .filter(|character| !character.is_whitespace())
                            .flat_map(char::to_lowercase)
                            .collect()
                    } else {
                        value.to_owned()
                    };
                    normalized.insert(key.clone(), Value::String(value));
                }
            } else {
                normalized.insert(key.clone(), value.clone());
            }
        }
    }

    let has_known_input = normalized
        .iter()
        .any(|(key, value)| INPUT_KEYS.contains(&key.as_str()) && value.as_str() != Some(""));
    if !has_known_input {
        return default_input();
    }

    let mut input = default_input();
    input.extend(normalized);
    if input.get("was").is_none_or(Value::is_null) {
        input.insert(
            "was".to_owned(),
            Value::String("Software Entwickler".to_owned()),
        );
    }
    input
}

fn default_input() -> Map<String, Value> {
    let mut input = Map::new();
    input.insert("was".to_owned(), json!("Software Entwickler"));
    input.insert("wo".to_owned(), json!("Berlin"));
    input.insert("umkreis".to_owned(), json!(25));
    input.insert("page".to_owned(), json!(1));
    input.insert("size".to_owned(), json!(25));
    input
}

fn build_jobs_params(input: &Map<String, Value>) -> Map<String, Value> {
    let mut params = Map::new();
    for key in INPUT_KEYS {
        if let Some(value) = input.get(*key) {
            if !value.is_null() && value.as_str() != Some("") {
                params.insert((*key).to_owned(), value.clone());
            }
        }
    }
    params
}

fn js_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Number(value)) => value.as_f64().is_some_and(|value| value != 0.0),
        Some(Value::String(value)) => !value.is_empty(),
        Some(_) => true,
    }
}

fn get_jobs(response: &Value) -> &[Value] {
    if let Some(jobs) = response
        .pointer("/data/stellenangebote")
        .and_then(Value::as_array)
    {
        return jobs;
    }
    if let Some(jobs) = response.get("stellenangebote").and_then(Value::as_array) {
        return jobs;
    }
    eprintln!("Unexpected Arbeitsagentur Jobs response shape: expected \"data.stellenangebote\" or \"stellenangebote\" array.");
    &[]
}

fn get_metadata(response: &Value) -> &Value {
    response
        .get("data")
        .filter(|data| !data.is_null())
        .unwrap_or(response)
}

fn to_dataset_job(job: &Value) -> Value {
    let mut dataset_job = job.as_object().cloned().unwrap_or_default();
    let location = job.get("arbeitsort");
    let location_fields = location.and_then(Value::as_object);
    let coordinates = location_fields
        .and_then(|fields| fields.get("koordinaten"))
        .and_then(Value::as_object);

    dataset_job.insert("title".to_owned(), value_or_null(job.get("titel")));
    dataset_job.insert("occupation".to_owned(), value_or_null(job.get("beruf")));
    dataset_job.insert(
        "company_name".to_owned(),
        value_or_null(job.get("arbeitgeber")),
    );
    dataset_job.insert(
        "location_formatted".to_owned(),
        get_formatted_location(location)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    dataset_job.insert(
        "location_city".to_owned(),
        value_or_null(location_fields.and_then(|fields| fields.get("ort"))),
    );
    dataset_job.insert(
        "postal_code".to_owned(),
        value_or_null(location_fields.and_then(|fields| fields.get("plz"))),
    );
    dataset_job.insert(
        "region".to_owned(),
        value_or_null(location_fields.and_then(|fields| fields.get("region"))),
    );
    dataset_job.insert(
        "country".to_owned(),
        value_or_null(location_fields.and_then(|fields| fields.get("land"))),
    );
    dataset_job.insert(
        "published_date".to_owned(),
        value_or_null(job.get("aktuelleVeroeffentlichungsdatum")),
    );
    dataset_job.insert(
        "start_date".to_owned(),
        value_or_null(job.get("eintrittsdatum")),
    );
    dataset_job.insert("job_url".to_owned(), value_or_null(job.get("externeUrl")));
    dataset_job.insert(
        "reference_number".to_owned(),
        value_or_null(job.get("refnr")),
    );
    dataset_job.insert(
        "distance_km".to_owned(),
        value_or_null(location_fields.and_then(|fields| fields.get("entfernung"))),
    );
    dataset_job.insert(
        "latitude".to_owned(),
        coordinates
            .and_then(|coordinates| coordinates.get("lat"))
            .filter(|value| value.is_number())
            .cloned()
            .unwrap_or(Value::Null),
    );
    dataset_job.insert(
        "longitude".to_owned(),
        coordinates
            .and_then(|coordinates| coordinates.get("lon"))
            .filter(|value| value.is_number())
            .cloned()
            .unwrap_or(Value::Null),
    );
    Value::Object(dataset_job)
}

fn get_formatted_location(location: Option<&Value>) -> Option<String> {
    let location = location?;
    if let Some(location) = location.as_str() {
        return Some(location.to_owned());
    }
    let fields = location.as_object()?;
    let parts = ["plz", "ort", "region", "land"]
        .iter()
        .filter_map(|key| fields.get(*key).and_then(Value::as_str))
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn value_or_null(value: Option<&Value>) -> Value {
    value.cloned().unwrap_or(Value::Null)
}

async fn run_actor(config: Config) -> Result<()> {
    let apify_http = Client::builder().timeout(APIFY_TIMEOUT).build()?;
    let scrappa_http = Client::builder().build()?;
    let apify = ApifyClient::new(apify_http, &config);
    let scrappa = ScrappaClient::new(
        scrappa_http,
        config.scrappa_api_base.clone(),
        config.scrappa_api_key.clone(),
    );

    let input = normalize_input(apify.get_input().await?.as_ref());
    if !js_truthy(input.get("was")) {
        bail!("Arbeitsagentur jobs search keyword is required.");
    }
    let query = query_value(
        input
            .get("was")
            .expect("normalized input always contains was"),
    );
    println!("Searching Arbeitsagentur Jobs for: \"{query}\"");

    let response = scrappa.get_jobs(&build_jobs_params(&input)).await.map_err(|error| {
        if error.downcast_ref::<ScrappaFailure>().is_some_and(|error| matches!(error, ScrappaFailure::Timeout)) {
            anyhow!("{error}. The Arbeitsagentur Jobs request exceeded the {}s Scrappa API timeout. Try again or refine the query.", SCRAPPA_REQUEST_TIMEOUT.as_secs())
        } else {
            error
        }
    })?;
    let jobs = get_jobs(&response);
    let dataset_jobs = jobs.iter().map(to_dataset_job).collect::<Vec<_>>();

    let saved = if dataset_jobs.is_empty() {
        println!("No Arbeitsagentur job results found for the given search criteria");
        0
    } else {
        let saved = apify.push_dataset_items(&dataset_jobs).await?;
        println!(
            "Saved {saved} of {} Arbeitsagentur job result(s)",
            dataset_jobs.len()
        );
        saved
    };

    apify.put_record("OUTPUT", &response).await?;
    println!("Arbeitsagentur Jobs search completed successfully");

    let metadata = get_metadata(&response);
    let first_job = jobs.first().map(|job| {
        json!({
            "title": job.get("titel").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
            "company": job.get("arbeitgeber").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
            "location": get_formatted_location(job.get("arbeitsort")).map(Value::String).unwrap_or(Value::Null),
            "reference_number": job.get("refnr").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        })
    });
    let summary = json!({
        "jobs": jobs.len(),
        "saved": saved,
        "total_jobs": metadata.get("maxErgebnisse").filter(|value| !value.is_null()).cloned().unwrap_or(json!(jobs.len())),
        "page": metadata.get("page").filter(|value| !value.is_null()).cloned().unwrap_or_else(|| input.get("page").cloned().unwrap_or(Value::Null)),
        "size": metadata.get("size").filter(|value| !value.is_null()).cloned().unwrap_or_else(|| input.get("size").cloned().unwrap_or(Value::Null)),
        "query": input.get("was"),
        "location": input.get("wo").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "first_job": first_job,
    });
    println!("Results summary: {}", serde_json::to_string(&summary)?);
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match Config::from_env() {
        Ok(config) => match run_actor(config).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Actor failed: {error:#}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    struct MockResponse {
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    impl MockResponse {
        fn json(status: u16, body: Value) -> Self {
            Self {
                status,
                headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                body: serde_json::to_vec(&body).unwrap(),
            }
        }

        fn with_header(mut self, name: &str, value: &str) -> Self {
            self.headers.push((name.to_owned(), value.to_owned()));
            self
        }
    }

    async fn start_mock_server(
        responses: Vec<MockResponse>,
    ) -> (String, tokio::task::JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for response in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let request = read_request(&mut stream).await;
                write_response(&mut stream, response).await;
                requests.push(request);
            }
            requests
        });
        (format!("http://{address}"), server)
    }

    async fn read_request(stream: &mut TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut chunk = [0; 2048];
        loop {
            let read = stream.read(&mut chunk).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            let Some(body_start) = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|position| position + 4)
            else {
                continue;
            };
            let headers = std::str::from_utf8(&request[..body_start]).unwrap();
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap_or_default())
                })
                .unwrap_or_default();
            if request.len() >= body_start + content_length {
                break;
            }
        }
        request
    }

    async fn write_response(stream: &mut TcpStream, response: MockResponse) {
        let reason = match response.status {
            200 => "OK",
            400 => "Bad Request",
            401 => "Unauthorized",
            404 => "Not Found",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "Test Response",
        };
        let mut headers = response.headers;
        headers.push(("Content-Length".to_owned(), response.body.len().to_string()));
        headers.push(("Connection".to_owned(), "close".to_owned()));
        let headers = headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}\r\n"))
            .collect::<String>();
        let status_line = format!("HTTP/1.1 {} {reason}\r\n", response.status);
        stream.write_all(status_line.as_bytes()).await.unwrap();
        stream.write_all(headers.as_bytes()).await.unwrap();
        stream.write_all(b"\r\n").await.unwrap();
        stream.write_all(&response.body).await.unwrap();
    }

    fn request_line(request: &[u8]) -> String {
        String::from_utf8_lossy(request)
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned()
    }

    fn request_header<'a>(request: &'a [u8], name: &str) -> Option<&'a str> {
        let request = std::str::from_utf8(request).ok()?;
        request
            .lines()
            .skip(1)
            .take_while(|line| !line.is_empty())
            .find_map(|line| {
                let (header_name, value) = line.split_once(':')?;
                header_name.eq_ignore_ascii_case(name).then(|| value.trim())
            })
    }

    fn request_body(request: &[u8]) -> Value {
        let body_start = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
            .unwrap();
        serde_json::from_slice(&request[body_start..]).unwrap()
    }

    fn config(apify_api_base: String, scrappa_api_base: String) -> Config {
        Config {
            apify_api_base,
            apify_token: "test-token".to_owned(),
            actor_run_id: "test-run".to_owned(),
            key_value_store_id: "store".to_owned(),
            dataset_id: "dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_base,
            scrappa_api_key: "test-key".to_owned(),
        }
    }

    fn mock_priced_run(max_total_charge: f64, charged_event_counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            "other-event": {"eventPriceUsd": 0.0002}
                        }
                    }
                },
                "chargedEventCounts": charged_event_counts,
                "options": {"maxTotalChargeUsd": max_total_charge}
            }
        })
    }

    fn mock_job(title: &str) -> Value {
        json!({
            "refnr": "12265-399943_JB5100405-S",
            "titel": title,
            "beruf": "Softwareentwickler/-in",
            "arbeitgeber": "TechGmbH",
            "arbeitsort": {
                "ort": "Berlin",
                "plz": "10115",
                "region": "Berlin",
                "land": "Deutschland",
                "entfernung": "3",
                "koordinaten": {"lat": 52.531976, "lon": 13.386737}
            },
            "aktuelleVeroeffentlichungsdatum": "2026-03-20",
            "eintrittsdatum": "2026-04-01",
            "externeUrl": "https://www.arbeitsagentur.de/jobsuche/jobdetail/10000001"
        })
    }

    #[test]
    fn actor_schema_keeps_the_qa_prefill_and_single_page_pagination_defaults() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let properties = schema.get("properties").unwrap();
        let mut qa_input = Map::new();
        for (name, property) in properties.as_object().unwrap() {
            if let Some(value) = property.get("prefill").or_else(|| property.get("default")) {
                qa_input.insert(name.clone(), value.clone());
            }
        }

        assert_eq!(qa_input["was"], "Software Entwickler");
        assert_eq!(qa_input["wo"], "Berlin");
        assert_eq!(qa_input["arbeitszeit"], "vz;ho");
        assert_eq!(qa_input["page"], 1);
        assert_eq!(qa_input["size"], 25);
        assert_eq!(
            normalize_input(Some(&Value::Object(qa_input.clone())))["arbeitszeit"],
            "vz;ho"
        );
        assert_eq!(
            build_jobs_params(&normalize_input(Some(&Value::Object(qa_input))))["page"],
            1
        );

        let actor: Value = serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();
        assert_eq!(
            actor.pointer("/defaultRunOptions/timeoutSecs"),
            Some(&json!(240))
        );
        assert_eq!(actor.pointer("/resources/memoryMbytes"), Some(&json!(128)));
        assert!(actor.get("disable").is_none());
    }

    #[test]
    fn input_normalization_matches_defaults_and_preserves_false_filters() {
        assert_eq!(normalize_input(None), default_input());
        assert_eq!(
            normalize_input(Some(&json!({"helloWorld": 123}))),
            default_input()
        );

        let input = normalize_input(Some(&json!({
            "wo": " Hamburg ",
            "arbeitszeit": " VZ ; HO ",
            "zeitarbeit": false,
            "size": 10,
            "ignored": "field"
        })));
        assert_eq!(input["was"], "Software Entwickler");
        assert_eq!(input["wo"], "Hamburg");
        assert_eq!(input["arbeitszeit"], "vz;ho");
        assert_eq!(input["zeitarbeit"], false);
        assert_eq!(input["size"], 10);
        assert!(!input.contains_key("ignored"));
    }

    #[test]
    fn query_params_keep_false_and_omit_empty_or_null_values() {
        let input = normalize_input(Some(&json!({
            "was": "Software Entwickler",
            "zeitarbeit": false,
            "pav": true,
            "berufsfeld": " ",
            "arbeitgeber": null,
            "page": 3,
            "size": 50
        })));
        let params = build_jobs_params(&input);
        let url = build_jobs_url("https://example.test/api", &params).unwrap();
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query["was"], "Software Entwickler");
        assert_eq!(query["zeitarbeit"], "false");
        assert_eq!(query["pav"], "true");
        assert_eq!(query["page"], "3");
        assert_eq!(query["size"], "50");
        assert!(!query.contains_key("berufsfeld"));
        assert!(!query.contains_key("arbeitgeber"));
    }

    #[test]
    fn response_shapes_and_metadata_match_the_node_actor() {
        let nested = json!({"data": {"stellenangebote": [mock_job("Nested")], "page": 2}});
        assert_eq!(get_jobs(&nested).len(), 1);
        assert_eq!(get_metadata(&nested)["page"], 2);

        let top_level = json!({"stellenangebote": [mock_job("Top-level")], "size": 10});
        assert_eq!(get_jobs(&top_level)[0]["titel"], "Top-level");
        assert_eq!(get_metadata(&top_level)["size"], 10);
        assert!(get_jobs(
            &json!({"data": {"stellenangebote": []}, "stellenangebote": [mock_job("Fallback")]})
        )
        .is_empty());
    }

    #[test]
    fn dataset_job_preserves_raw_fields_and_adds_table_aliases() {
        let job = mock_job("Software Entwickler (m/w/d)");
        assert_eq!(
            to_dataset_job(&job),
            json!({
                "refnr": "12265-399943_JB5100405-S",
                "titel": "Software Entwickler (m/w/d)",
                "beruf": "Softwareentwickler/-in",
                "arbeitgeber": "TechGmbH",
                "arbeitsort": {
                    "ort": "Berlin", "plz": "10115", "region": "Berlin", "land": "Deutschland",
                    "entfernung": "3", "koordinaten": {"lat": 52.531976, "lon": 13.386737}
                },
                "aktuelleVeroeffentlichungsdatum": "2026-03-20",
                "eintrittsdatum": "2026-04-01",
                "externeUrl": "https://www.arbeitsagentur.de/jobsuche/jobdetail/10000001",
                "title": "Software Entwickler (m/w/d)",
                "occupation": "Softwareentwickler/-in",
                "company_name": "TechGmbH",
                "location_formatted": "10115, Berlin, Berlin, Deutschland",
                "location_city": "Berlin",
                "postal_code": "10115",
                "region": "Berlin",
                "country": "Deutschland",
                "published_date": "2026-03-20",
                "start_date": "2026-04-01",
                "job_url": "https://www.arbeitsagentur.de/jobsuche/jobdetail/10000001",
                "reference_number": "12265-399943_JB5100405-S",
                "distance_km": "3",
                "latitude": 52.531976,
                "longitude": 13.386737
            })
        );
        assert_eq!(
            get_formatted_location(Some(&json!("Berlin"))).as_deref(),
            Some("Berlin")
        );
        assert_eq!(get_formatted_location(Some(&json!({"ort": "  "}))), None);
        assert_eq!(
            to_dataset_job(&json!({"titel": "No location"}))["latitude"],
            Value::Null
        );
        assert_eq!(
            to_dataset_job(&json!({"arbeitsort": {"entfernung": false}}))["distance_km"],
            false
        );
    }

    #[test]
    fn scrappa_retry_rules_and_retry_after_match_the_node_client() {
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(ScrappaFailure::Api {
                status,
                message: String::new(),
                retry_after_ms: None,
            }
            .is_retryable());
        }
        assert!(ScrappaFailure::Timeout.is_retryable());
        assert!(!ScrappaFailure::Api {
            status: 401,
            message: String::new(),
            retry_after_ms: None,
        }
        .is_retryable());
        assert!(!ScrappaFailure::Transport("network error".to_owned()).is_retryable());
        assert!(!ScrappaFailure::InvalidJson("bad JSON".to_owned()).is_retryable());

        assert_eq!(get_retry_delay_ms(1, 0, None, 20_000), 2_000);
        assert_eq!(get_retry_delay_ms(2, 250, None, 20_000), 4_250);
        assert_eq!(get_retry_delay_ms(1, 0, Some(15_000), 20_000), 15_000);
        assert_eq!(get_retry_delay_ms(1, 0, Some(60_000), 20_000), 20_000);
        assert_eq!(parse_retry_after_ms("0.5"), Some(500));
        assert_eq!(parse_retry_after_ms("not a date"), None);
        assert_eq!(get_retry_delay_ms(3, 999, Some(20_000), 20_000), 20_000);
    }

    #[test]
    fn retry_budget_leaves_the_actor_completion_reserve() {
        let maximum_request_ms = duration_millis(SCRAPPA_REQUEST_DEADLINE);
        let configured_maximum_ms = maximum_request_ms + ACTOR_COMPLETION_RESERVE_MS;
        assert_eq!(maximum_request_ms, 180_000);
        assert_eq!(configured_maximum_ms, 210_000);
        assert!(configured_maximum_ms < ACTOR_TIMEOUT_MS);
    }

    #[test]
    fn scrappa_error_messages_keep_json_details_and_text_fallbacks() {
        assert_eq!(
            scrappa_error_message(StatusCode::NOT_FOUND, br#"{"message":"Not found"}"#),
            "Not found"
        );
        assert_eq!(
            scrappa_error_message(
                StatusCode::UNPROCESSABLE_ENTITY,
                br#"{"message":"Invalid","errors":{"was":["bad","required"]}}"#
            ),
            "Invalid - was: bad, required"
        );
        assert_eq!(
            scrappa_error_message(StatusCode::SERVICE_UNAVAILABLE, b""),
            "Service Unavailable"
        );
        assert_eq!(
            scrappa_error_message(StatusCode::SERVICE_UNAVAILABLE, b"  Busy\n now "),
            "Busy now"
        );
    }

    #[test]
    fn pay_per_event_budget_counts_all_events_and_only_allows_affordable_items() {
        let run = mock_priced_run(0.0006, json!({"other-event": 1}));
        assert_eq!(affordable_dataset_items(&run, 3).unwrap(), 1);
        assert_eq!(
            affordable_dataset_items(&mock_priced_run(0.0, json!({})), 3).unwrap(),
            0
        );
        let free_items = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0}
                    }}
                },
                "chargedEventCounts": {},
                "options": {"maxTotalChargeUsd": 0.0}
            }
        });
        assert_eq!(affordable_dataset_items(&free_items, 3).unwrap(), 3);
        assert!(affordable_dataset_items(&json!({"data": {}}), 1)
            .unwrap_err()
            .to_string()
            .contains("pay-per-event"));
    }

    #[tokio::test]
    async fn run_preserves_the_prefill_request_and_outputs_budgeted_rows_and_raw_response() {
        let input = json!({
            "was": "Software Entwickler",
            "wo": "Berlin",
            "umkreis": 25,
            "arbeitszeit": "vz;ho",
            "page": 1,
            "size": 25
        });
        let response = json!({
            "success": true,
            "data": {
                "stellenangebote": [mock_job("Job 1"), mock_job("Job 2")],
                "maxErgebnisse": 100,
                "page": 1,
                "size": 25,
                "facetten": {"beruf": []}
            }
        });
        let (apify_base, apify_server) = start_mock_server(vec![
            MockResponse::json(200, input),
            MockResponse::json(200, mock_priced_run(0.0006, json!({"other-event": 1}))),
            MockResponse::json(200, Value::Null),
            MockResponse::json(200, response.clone()),
        ])
        .await;
        let (scrappa_base, scrappa_server) =
            start_mock_server(vec![MockResponse::json(200, response.clone())]).await;

        run_actor(config(apify_base, format!("{scrappa_base}/api")))
            .await
            .unwrap();

        let apify_requests = apify_server.await.unwrap();
        assert_eq!(apify_requests.len(), 4);
        assert!(request_line(&apify_requests[0])
            .starts_with("GET /v2/key-value-stores/store/records/INPUT HTTP/1.1"));
        assert!(request_header(&apify_requests[0], "Authorization")
            .is_some_and(|value| value == "Bearer test-token"));
        assert_eq!(
            request_line(&apify_requests[1]),
            "GET /v2/actor-runs/test-run HTTP/1.1"
        );
        assert_eq!(
            request_line(&apify_requests[2]),
            "POST /v2/datasets/dataset/items HTTP/1.1"
        );
        assert_eq!(request_body(&apify_requests[2])[0]["title"], "Job 1");
        assert_eq!(
            request_line(&apify_requests[3]),
            "PUT /v2/key-value-stores/store/records/OUTPUT HTTP/1.1"
        );
        assert_eq!(request_body(&apify_requests[3]), response);

        let scrappa_requests = scrappa_server.await.unwrap();
        assert_eq!(scrappa_requests.len(), 1);
        assert!(request_line(&scrappa_requests[0]).starts_with("GET /api/arbeitsagentur/jobs?"));
        let scrappa_request = String::from_utf8_lossy(&scrappa_requests[0]);
        let request_url = Url::parse(&format!(
            "http://localhost{}",
            request_line(&scrappa_requests[0])
                .split_whitespace()
                .nth(1)
                .unwrap()
        ))
        .unwrap();
        let query: std::collections::HashMap<_, _> =
            request_url.query_pairs().into_owned().collect();
        assert_eq!(query["was"], "Software Entwickler");
        assert_eq!(query["wo"], "Berlin");
        assert_eq!(query["arbeitszeit"], "vz;ho");
        assert_eq!(query["page"], "1");
        assert_eq!(query["size"], "25");
        assert_eq!(
            request_header(&scrappa_requests[0], "X-API-Key"),
            Some("test-key")
        );
        assert!(scrappa_request.contains(SCRAPPA_USER_AGENT));
    }

    #[tokio::test]
    async fn dataset_publisher_checks_pricing_and_does_not_retry_append_posts() {
        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(200, mock_priced_run(0.0003, json!({}))),
            MockResponse::json(503, json!({"message": "Unavailable"})),
        ])
        .await;
        let config = config(base_url, SCRAPPA_API_DEFAULT.to_owned());
        let apify = ApifyClient::new(Client::new(), &config);
        let rows = vec![json!({"title": "first"}), json!({"title": "second"})];

        let error = apify.push_dataset_items(&rows).await.unwrap_err();
        assert!(error
            .to_string()
            .contains("Apify dataset item publication failed (503)"));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(request_line(&requests[1]).starts_with("POST /v2/datasets/dataset/items HTTP/1.1"));
        assert_eq!(request_body(&requests[1]), json!([{"title": "first"}]));
    }

    #[tokio::test]
    async fn dataset_publisher_sends_affordable_rows_in_one_array_request() {
        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(200, mock_priced_run(0.0006, json!({}))),
            MockResponse::json(201, Value::Null),
        ])
        .await;
        let config = config(base_url, SCRAPPA_API_DEFAULT.to_owned());
        let apify = ApifyClient::new(Client::new(), &config);
        let rows = vec![
            json!({"title": "first"}),
            json!({"title": "second"}),
            json!({"title": "third"}),
        ];

        assert_eq!(apify.push_dataset_items(&rows).await.unwrap(), 2);
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            request_line(&requests[1]),
            "POST /v2/datasets/dataset/items HTTP/1.1"
        );
        assert_eq!(
            request_body(&requests[1]),
            json!([{"title": "first"}, {"title": "second"}])
        );
    }

    #[tokio::test]
    async fn scrappa_retries_only_transient_http_responses_and_honors_retry_after() {
        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(503, json!({"message": "temporary"}))
                .with_header("Retry-After", "0"),
            MockResponse::json(
                200,
                json!({"success": true, "data": {"stellenangebote": []}}),
            ),
        ])
        .await;
        let params = build_jobs_params(&normalize_input(None));
        let client = ScrappaClient {
            http: Client::new(),
            base_url: base_url.clone(),
            api_key: "test-key".to_owned(),
            policy: ScrappaRetryPolicy {
                attempts: 2,
                request_timeout: Duration::from_secs(1),
                max_retry_delay: Duration::ZERO,
                request_deadline: Duration::from_secs(2),
            },
        };

        assert_eq!(client.get_jobs(&params).await.unwrap()["success"], true);
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests
            .iter()
            .all(|request| request_line(request).starts_with("GET /arbeitsagentur/jobs?")));
    }

    #[tokio::test]
    async fn scrappa_request_timeout_is_reported_as_retryable_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = read_request(&mut stream).await;
            tokio::time::sleep(Duration::from_millis(100)).await;
        });
        let client = ScrappaClient {
            http: Client::new(),
            base_url: format!("http://{address}"),
            api_key: "test-key".to_owned(),
            policy: ScrappaRetryPolicy {
                attempts: 1,
                request_timeout: Duration::from_millis(20),
                max_retry_delay: Duration::ZERO,
                request_deadline: Duration::from_millis(50),
            },
        };

        let error = client
            .get_jobs(&build_jobs_params(&normalize_input(None)))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("timed out after 30000ms"));
        server.await.unwrap();
    }
}
