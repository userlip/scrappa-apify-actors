use std::{env, process::ExitCode, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Value};
use tokio::time::{timeout_at, Instant};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const RETRY_DELAYS: [Duration; 2] = [Duration::from_secs(5), Duration::from_secs(15)];
const APIFY_MAX_RETRIES: usize = 2;
const INPUT_KEY_DEFAULT: &str = "INPUT";
const OUTPUT_KEY: &str = "OUTPUT";

#[tokio::main]
async fn main() -> ExitCode {
    match run_actor().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {}", actor_failure_message(&error));
            ExitCode::FAILURE
        }
    }
}

async fn run_actor() -> Result<()> {
    let config = Config::from_env()?;
    run_actor_with_config(config).await
}

struct Config {
    apify_api_base: Url,
    apify_token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_base: Url,
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
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|key| !key.is_empty())
                .unwrap_or_else(|| INPUT_KEY_DEFAULT.to_owned()),
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is not set"))?;
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
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow!("API base URL cannot contain query or fragment"))?;
        path.pop_if_empty().extend(segments.iter().copied());
    }
    Ok(url)
}

#[derive(Debug, PartialEq, Eq)]
struct PostRequest {
    identifier: String,
    url: Option<String>,
    shortcode: Option<String>,
}

fn input_string(input: &Value, key: &str) -> Option<String> {
    input
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn resolve_input(input: &Value) -> Result<PostRequest> {
    let url = input_string(input, "url");
    let shortcode = input_string(input, "shortcode").or_else(|| input_string(input, "media_id"));
    let identifier = url.clone().or(shortcode.clone()).ok_or_else(|| {
        anyhow!("Instagram post URL or shortcode is required. Provide url, shortcode, or media_id in the input.")
    })?;

    if url.as_deref().is_some_and(looks_like_url) {
        Ok(PostRequest {
            identifier,
            url,
            shortcode: None,
        })
    } else {
        Ok(PostRequest {
            identifier: identifier.clone(),
            url: None,
            shortcode: Some(identifier),
        })
    }
}

fn looks_like_url(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    if value.starts_with("https://") || value.starts_with("http://") || value.starts_with("www.") {
        return true;
    }

    let Some((host, _)) = value.split_once('/') else {
        return false;
    };
    let Some((_, top_level_domain)) = host.rsplit_once('.') else {
        return false;
    };
    host.as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
        && top_level_domain.len() >= 2
        && top_level_domain
            .bytes()
            .all(|byte| byte.is_ascii_alphabetic())
}

#[derive(Debug, PartialEq, Eq)]
struct PostIdentity {
    username: String,
    shortcode: String,
}

fn get_post_identity(value: Option<&str>) -> Option<PostIdentity> {
    let value = value?;
    let normalized = if value.starts_with("http://") || value.starts_with("https://") {
        value.to_owned()
    } else {
        format!("https://{value}")
    };
    let parsed = Url::parse(&normalized).ok()?;
    if !matches!(parsed.host_str()?, "instagram.com" | "www.instagram.com") {
        return None;
    }

    let segments = parsed.path_segments()?.collect::<Vec<_>>();
    let segments = if segments.last() == Some(&"") {
        &segments[..segments.len().saturating_sub(1)]
    } else {
        &segments[..]
    };
    if segments.len() != 3 || !matches!(segments[1], "p" | "reel" | "reels" | "tv") {
        return None;
    }

    let valid_username = |username: &str| {
        !username.is_empty()
            && username
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.')
    };
    let valid_shortcode = |shortcode: &str| {
        !shortcode.is_empty()
            && shortcode
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    };
    if !valid_username(segments[0]) || !valid_shortcode(segments[2]) {
        return None;
    }

    Some(PostIdentity {
        username: segments[0].to_owned(),
        shortcode: segments[2].to_owned(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrappaErrorKind {
    Http,
    ApiBody,
    Transport,
}

#[derive(Debug)]
struct ScrappaError {
    kind: ScrappaErrorKind,
    http_status: Option<u16>,
    retry_status: Option<u16>,
    message: String,
    data: Option<Value>,
    timed_out: bool,
}

impl ScrappaError {
    fn http(status: u16, message: String, data: Option<Value>) -> Self {
        Self {
            kind: ScrappaErrorKind::Http,
            http_status: Some(status),
            retry_status: Some(status),
            message,
            data,
            timed_out: false,
        }
    }

    fn api_body(status: Option<u16>, message: String, data: Option<Value>) -> Self {
        Self {
            kind: ScrappaErrorKind::ApiBody,
            http_status: None,
            retry_status: status,
            message,
            data,
            timed_out: false,
        }
    }

    fn transport(message: String, timed_out: bool) -> Self {
        Self {
            kind: ScrappaErrorKind::Transport,
            http_status: None,
            retry_status: None,
            message,
            data: None,
            timed_out,
        }
    }

    fn timed_out() -> Self {
        Self::transport(
            format!(
                "Scrappa API request timed out after {}ms",
                REQUEST_TIMEOUT.as_millis()
            ),
            true,
        )
    }

    fn actor_failure_message(&self) -> String {
        match (self.kind, self.http_status) {
            (ScrappaErrorKind::Http, Some(status)) => {
                format!(
                    "Scrappa Instagram Post API request failed ({status}): {}",
                    self.message
                )
            }
            (ScrappaErrorKind::ApiBody, _) => {
                format!("Scrappa Instagram API: {}", self.message)
            }
            _ => format!(
                "Scrappa Instagram Post API request failed: {}",
                self.message
            ),
        }
    }

    fn response_status(&self) -> Option<u16> {
        self.retry_status
    }

    fn has_explicit_non_retryable_response(&self) -> bool {
        self.data.as_ref().and_then(|data| data.get("retryable")) == Some(&Value::Bool(false))
    }

    fn is_transient(&self) -> bool {
        if self.has_explicit_non_retryable_response() {
            return false;
        }

        let status = self.response_status();
        if matches!(status, Some(401 | 403)) {
            return false;
        }
        if matches!(status, Some(408 | 425 | 429 | 500 | 502 | 503 | 504)) {
            return true;
        }

        self.timed_out || has_transient_message(&self.retry_message())
    }

    fn is_rate_limit(&self) -> bool {
        if self.has_explicit_non_retryable_response() {
            return false;
        }
        if self.response_status() == Some(429) {
            return true;
        }
        let message = normalize_message(&self.retry_message());
        contains_retry_phrase(&message, true)
    }

    fn is_cooldown_auth(&self) -> bool {
        if self.has_explicit_non_retryable_response()
            || !matches!(self.response_status(), Some(401 | 403))
        {
            return false;
        }
        let message = normalize_message(&self.retry_message());
        message.contains("authentication required") || message.contains("unauthorized")
    }

    fn retry_message(&self) -> String {
        self.data
            .as_ref()
            .map(response_message)
            .unwrap_or_else(|| self.message.clone())
    }
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ScrappaError {}

fn actor_failure_message(error: &anyhow::Error) -> String {
    error
        .downcast_ref::<ScrappaError>()
        .map(ScrappaError::actor_failure_message)
        .unwrap_or_else(|| error.to_string())
}

fn response_message(data: &Value) -> String {
    data.get("message")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("error").filter(|value| !value.is_null()))
        .map(javascript_string)
        .unwrap_or_else(|| "Unknown Scrappa API error".to_owned())
}

fn javascript_string(value: &Value) -> String {
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
                    javascript_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn normalize_message(message: &str) -> String {
    message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn has_transient_message(message: &str) -> bool {
    contains_retry_phrase(&normalize_message(message), false)
}

fn contains_retry_phrase(message: &str, rate_limit_only: bool) -> bool {
    let rate_limited = message.contains("rate limited") || message.contains("ratelimited");
    let too_many_requests = message.contains("too many requests");
    let status_429 = message
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|word| word == "429");
    rate_limited
        || too_many_requests
        || status_429
        || (!rate_limit_only
            && (message.contains("temporarily unavailable") || message.contains("timeout")))
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

    async fn get_input(&self) -> Result<Option<Value>> {
        let url = endpoint_url(
            &self.base_url,
            &[
                "v2",
                "key-value-stores",
                &self.key_value_store_id,
                "records",
                &self.input_key,
            ],
        )?;
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

    async fn run_dataset_capacity(&self, requested: usize) -> Result<usize> {
        let url = endpoint_url(&self.base_url, &["v2", "actor-runs", &self.actor_run_id])?;
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

    async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = endpoint_url(
            &self.base_url,
            &["v2", "datasets", &self.dataset_id, "items"],
        )?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(item)
            .send()
            .await
            .context("Failed to publish dataset item to Apify API")?;
        require_apify_success(response, "dataset item publication").await?;
        Ok(())
    }

    async fn put_output(&self, output: &Value) -> Result<()> {
        let url = endpoint_url(
            &self.base_url,
            &[
                "v2",
                "key-value-stores",
                &self.key_value_store_id,
                "records",
                OUTPUT_KEY,
            ],
        )?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .put(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .json(output)
                .send()
                .await
                .context("Failed to write OUTPUT record to Apify API")?;
            if let Some(delay) = apify_retry_delay("PUT", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            require_apify_success(response, "OUTPUT record publication").await?;
            return Ok(());
        }
    }
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

    async fn fetch_post(
        &self,
        request: &PostRequest,
        deadline: Instant,
    ) -> std::result::Result<Value, ScrappaError> {
        match self.fetch_single_post(request, deadline).await {
            Ok(data) => Ok(data),
            Err(original_error) => {
                let identity = get_post_identity(request.url.as_deref());
                let login_required = original_error.response_status() == Some(403)
                    && original_error
                        .data
                        .as_ref()
                        .and_then(|data| data.get("error_code"))
                        .and_then(Value::as_str)
                        == Some("instagram_login_required");
                if identity.is_none()
                    || original_error.timed_out
                    || Instant::now() >= deadline
                    || (!login_required && !original_error.is_transient())
                {
                    return Err(original_error);
                }

                let identity = identity.expect("identity was checked above");
                eprintln!("Single-post lookup unavailable; checking the account feed for the exact shortcode.");
                let feed = self.fetch_user_posts(&identity.username, deadline).await?;
                let post = feed
                    .get("posts")
                    .and_then(Value::as_array)
                    .and_then(|posts| {
                        posts.iter().find(|post| {
                            post.get("shortcode").and_then(Value::as_str)
                                == Some(identity.shortcode.as_str())
                        })
                    });
                let Some(post) = post else {
                    return Err(original_error);
                };
                Ok(json!({"success": true, "found": true, "data": post}))
            }
        }
    }

    async fn fetch_single_post(
        &self,
        request: &PostRequest,
        deadline: Instant,
    ) -> std::result::Result<Value, ScrappaError> {
        let mut url = endpoint_url(&self.base_url, &["instagram", "post"])
            .map_err(|error| ScrappaError::transport(error.to_string(), false))?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(url_value) = &request.url {
                query.append_pair("url", url_value);
            } else if let Some(shortcode) = &request.shortcode {
                query.append_pair("shortcode", shortcode);
            }
        }
        self.fetch_scrappa_json(url, deadline).await
    }

    async fn fetch_user_posts(
        &self,
        username: &str,
        deadline: Instant,
    ) -> std::result::Result<Value, ScrappaError> {
        let mut url = endpoint_url(&self.base_url, &["instagram", "user", "posts"])
            .map_err(|error| ScrappaError::transport(error.to_string(), false))?;
        url.query_pairs_mut().append_pair("username", username);
        self.fetch_scrappa_json(url, deadline).await
    }

    async fn fetch_scrappa_json(
        &self,
        url: Url,
        deadline: Instant,
    ) -> std::result::Result<Value, ScrappaError> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(ScrappaError::timed_out());
        }

        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .timeout(remaining)
            .send()
            .await
            .map_err(|error| {
                let timed_out = error.is_timeout();
                let message = if timed_out {
                    format!(
                        "Scrappa API request timed out after {}ms",
                        REQUEST_TIMEOUT.as_millis()
                    )
                } else {
                    format!("Scrappa API request failed: {error}")
                };
                ScrappaError::transport(message, timed_out)
            })?;
        let status = response.status();
        let body = response.bytes().await.map_err(|error| {
            let timed_out = error.is_timeout();
            let message = if timed_out {
                format!(
                    "Scrappa API request timed out after {}ms",
                    REQUEST_TIMEOUT.as_millis()
                )
            } else {
                format!("Scrappa API request failed: {error}")
            };
            ScrappaError::transport(message, timed_out)
        })?;
        let data = serde_json::from_slice::<Value>(&body).ok();

        if !status.is_success() {
            let message = data
                .as_ref()
                .filter(|data| data.is_object())
                .map(response_message)
                .unwrap_or_else(|| {
                    status
                        .canonical_reason()
                        .unwrap_or("Unknown status")
                        .to_owned()
                });
            return Err(ScrappaError::http(status.as_u16(), message, data));
        }

        let Some(data) = data else {
            return Err(ScrappaError::api_body(
                None,
                "Unknown Scrappa API error".to_owned(),
                None,
            ));
        };
        if data.get("success") != Some(&Value::Bool(true)) {
            let status = data
                .get("status_code")
                .and_then(Value::as_u64)
                .and_then(|status| u16::try_from(status).ok());
            return Err(ScrappaError::api_body(
                status,
                response_message(&data),
                Some(data),
            ));
        }
        Ok(data)
    }
}

async fn request_with_retries(
    client: &ScrappaClient,
    request: &PostRequest,
) -> std::result::Result<Value, ScrappaError> {
    request_with_retry_policy(client, request, &RETRY_DELAYS, REQUEST_TIMEOUT).await
}

async fn request_with_retry_policy(
    client: &ScrappaClient,
    request: &PostRequest,
    delays: &[Duration],
    timeout: Duration,
) -> std::result::Result<Value, ScrappaError> {
    let mut saw_rate_limit = false;

    for attempt in 0..=delays.len() {
        let deadline = Instant::now() + timeout;
        let result = timeout_at(deadline, client.fetch_post(request, deadline))
            .await
            .unwrap_or_else(|_| Err(ScrappaError::timed_out()));
        match result {
            Ok(data) => return Ok(data),
            Err(error) => {
                let last_attempt = attempt == delays.len();
                let retry_reason = if error.is_transient() {
                    if error.is_rate_limit() {
                        saw_rate_limit = true;
                    }
                    Some("transient failure")
                } else if saw_rate_limit && error.is_cooldown_auth() {
                    Some("cooldown auth response")
                } else {
                    None
                };
                let Some(retry_reason) = retry_reason.filter(|_| !last_attempt) else {
                    return Err(error);
                };

                let delay = delays[attempt];
                let status = error
                    .response_status()
                    .map(|status| format!(" ({status})"))
                    .unwrap_or_default();
                eprintln!(
                    "Scrappa Instagram Post API {retry_reason}{status}: {}. Retry {} in {:.1}s.",
                    error.retry_message(),
                    attempt + 1,
                    delay.as_secs_f64(),
                );
                tokio::time::sleep(delay).await;
            }
        }
    }

    unreachable!("retry loop always returns or succeeds")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        task::JoinHandle,
    };

    #[derive(Debug)]
    struct MockResponse {
        status: u16,
        body: String,
        delay: Duration,
    }

    impl MockResponse {
        fn json(status: u16, body: Value) -> Self {
            Self {
                status,
                body: body.to_string(),
                delay: Duration::ZERO,
            }
        }

        fn delayed_json(status: u16, body: Value, delay: Duration) -> Self {
            Self {
                status,
                body: body.to_string(),
                delay,
            }
        }
    }

    #[derive(Debug)]
    struct CapturedRequest {
        method: String,
        path: String,
        headers: String,
        body: String,
    }

    async fn start_mock_server(
        responses: Vec<MockResponse>,
    ) -> (Url, JoinHandle<Vec<CapturedRequest>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for response in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                requests.push(read_request(&mut stream).await);
                if !response.delay.is_zero() {
                    tokio::time::sleep(response.delay).await;
                }
                let reason = StatusCode::from_u16(response.status)
                    .ok()
                    .and_then(|status| status.canonical_reason())
                    .unwrap_or("Unknown status");
                let response_bytes = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.status,
                    reason,
                    response.body.len(),
                    response.body,
                );
                let _ = stream.write_all(response_bytes.as_bytes()).await;
            }
            requests
        });
        (Url::parse(&format!("http://{address}/")).unwrap(), server)
    }

    async fn read_request(stream: &mut TcpStream) -> CapturedRequest {
        let mut bytes = Vec::new();
        let mut chunk = [0; 2048];
        loop {
            let read = stream.read(&mut chunk).await.unwrap();
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..read]);
            let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }

        let header_end = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let headers = String::from_utf8_lossy(&bytes[..header_end]).into_owned();
        let first_line = headers.lines().next().unwrap();
        let mut request_line = first_line.split_whitespace();
        let method = request_line.next().unwrap().to_owned();
        let path = request_line.next().unwrap().to_owned();
        let body = String::from_utf8_lossy(&bytes[header_end + 4..]).into_owned();
        CapturedRequest {
            method,
            path,
            headers,
            body,
        }
    }

    fn scrappa_client(base_url: Url) -> ScrappaClient {
        ScrappaClient::new(Client::new(), base_url, "test-api-key".to_owned())
    }

    fn config(base_url: Url) -> Config {
        Config {
            apify_api_base: base_url.clone(),
            apify_token: "test-apify-token".to_owned(),
            actor_run_id: "run-1".to_owned(),
            key_value_store_id: "store-1".to_owned(),
            dataset_id: "dataset-1".to_owned(),
            input_key: INPUT_KEY_DEFAULT.to_owned(),
            scrappa_api_base: base_url,
            scrappa_api_key: "test-api-key".to_owned(),
        }
    }

    fn run_pricing(max_charge: f64, charged_item_count: u64) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0002},
                        "other-result": {"eventPriceUsd": 0.0001}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": {
                    "apify-default-dataset-item": charged_item_count,
                    "other-result": 1
                }
            }
        })
    }

    #[test]
    fn input_keeps_url_priority_and_legacy_shortcode_alias() {
        assert_eq!(
            resolve_input(&json!({
                "url": " https://www.instagram.com/natgeo/p/DXHKcyvEWfr/ ",
                "shortcode": "SHOULD_NOT_BE_USED"
            }))
            .unwrap(),
            PostRequest {
                identifier: "https://www.instagram.com/natgeo/p/DXHKcyvEWfr/".to_owned(),
                url: Some("https://www.instagram.com/natgeo/p/DXHKcyvEWfr/".to_owned()),
                shortcode: None,
            }
        );
        assert_eq!(
            resolve_input(&json!({"url": "", "shortcode": " ", "media_id": "DXHKcyvEWfr"}))
                .unwrap(),
            PostRequest {
                identifier: "DXHKcyvEWfr".to_owned(),
                url: None,
                shortcode: Some("DXHKcyvEWfr".to_owned()),
            }
        );
        assert!(resolve_input(&Value::Null)
            .unwrap_err()
            .to_string()
            .contains("Instagram post URL or shortcode is required"));
    }

    #[test]
    fn input_prefill_stays_a_url_without_becoming_an_api_default() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let prefill = schema["properties"]["url"]["prefill"].as_str().unwrap();
        assert!(get_post_identity(Some(prefill)).is_some());
        assert!(schema["properties"]["url"].get("default").is_none());
        assert_eq!(
            resolve_input(&json!({"shortcode": "CUSTOM_POST"}))
                .unwrap()
                .shortcode
                .as_deref(),
            Some("CUSTOM_POST")
        );
    }

    #[test]
    fn url_detection_and_identity_match_supported_instagram_post_urls() {
        assert!(looks_like_url("https://instagram.com/user/p/POST"));
        assert!(looks_like_url("instagram.com/user/p/POST"));
        assert!(!looks_like_url("POST"));
        assert_eq!(
            get_post_identity(Some("instagram.com/name.1/reels/ABC-123/")),
            Some(PostIdentity {
                username: "name.1".to_owned(),
                shortcode: "ABC-123".to_owned(),
            })
        );
        assert!(get_post_identity(Some("https://example.com/user/p/ABC")).is_none());
        assert!(get_post_identity(Some("https://instagram.com/user/story/ABC")).is_none());
    }

    #[test]
    fn transient_retry_rules_keep_non_retryable_and_auth_responses_terminal() {
        let transient = ScrappaError::http(
            503,
            "temporarily unavailable".to_owned(),
            Some(json!({"message": "temporarily unavailable", "retryable": true})),
        );
        assert!(transient.is_transient());
        assert!(ScrappaError::http(
            429,
            "rate limited".to_owned(),
            Some(json!({"error": "Rate limited (HTTP 429)"})),
        )
        .is_rate_limit());
        assert!(!ScrappaError::http(
            429,
            "rate limited".to_owned(),
            Some(json!({"error": "Rate limited", "retryable": false})),
        )
        .is_transient());
        assert!(!ScrappaError::http(
            401,
            "Authentication timeout".to_owned(),
            Some(json!({"message": "Authentication timeout"})),
        )
        .is_transient());
        assert!(ScrappaError::http(
            401,
            "Authentication required".to_owned(),
            Some(json!({"message": "Authentication required"})),
        )
        .is_cooldown_auth());
        assert!(!ScrappaError::http(
            401,
            "Authentication required".to_owned(),
            Some(json!({"message": "Authentication required", "retryable": false})),
        )
        .is_cooldown_auth());
    }

    #[test]
    fn affordable_dataset_items_account_for_all_event_charges() {
        assert_eq!(
            affordable_dataset_items(&run_pricing(0.0006, 1), 1).unwrap(),
            1
        );
        assert_eq!(
            affordable_dataset_items(&run_pricing(0.0003, 1), 1).unwrap(),
            0
        );
        assert!(affordable_dataset_items(&json!({"data": {}}), 1).is_err());
    }

    #[tokio::test]
    async fn feed_fallback_returns_only_the_requested_post_with_api_auth() {
        let post = json!({"shortcode": "Dc30nJeRKKz", "caption": "Actual caption"});
        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(
                503,
                json!({"message": "Temporarily unavailable", "retryable": true}),
            ),
            MockResponse::json(
                200,
                json!({"success": true, "posts": [
                    {"shortcode": "OTHER"}, post.clone()
                ]}),
            ),
        ])
        .await;
        let client = scrappa_client(base_url);
        let request =
            resolve_input(&json!({"url": "https://www.instagram.com/instagram/p/Dc30nJeRKKz/"}))
                .unwrap();
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        let result = client.fetch_post(&request, deadline).await.unwrap();
        assert_eq!(
            result,
            json!({"success": true, "found": true, "data": post})
        );

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].path.starts_with(
            "/instagram/post?url=https%3A%2F%2Fwww.instagram.com%2Finstagram%2Fp%2FDc30nJeRKKz%2F"
        ));
        assert!(requests[1]
            .path
            .starts_with("/instagram/user/posts?username=instagram"));
        for request in requests {
            assert!(request
                .headers
                .to_ascii_lowercase()
                .contains("x-api-key: test-api-key"));
            assert!(request
                .headers
                .to_ascii_lowercase()
                .contains("accept: application/json"));
        }
    }

    #[tokio::test]
    async fn missing_feed_match_returns_the_original_single_post_error() {
        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(
                503,
                json!({"message": "Temporarily unavailable", "retryable": true}),
            ),
            MockResponse::json(
                200,
                json!({"success": true, "posts": [{"shortcode": "OTHER"}]}),
            ),
        ])
        .await;
        let client = scrappa_client(base_url);
        let request =
            resolve_input(&json!({"url": "https://www.instagram.com/instagram/p/REQUESTED/"}))
                .unwrap();
        let error = client
            .fetch_post(&request, Instant::now() + REQUEST_TIMEOUT)
            .await
            .unwrap_err();
        assert_eq!(error.http_status, Some(503));
        assert_eq!(error.message, "Temporarily unavailable");
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn fallback_is_limited_to_transient_or_login_required_url_failures() {
        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(
                403,
                json!({"error_code": "instagram_login_required", "message": "Login required"}),
            ),
            MockResponse::json(
                200,
                json!({"success": true, "posts": [{"shortcode": "CODE"}]}),
            ),
        ])
        .await;
        let client = scrappa_client(base_url);
        let request =
            resolve_input(&json!({"url": "https://www.instagram.com/name/p/CODE/"})).unwrap();
        let result = client
            .fetch_post(&request, Instant::now() + REQUEST_TIMEOUT)
            .await
            .unwrap();
        assert_eq!(result["data"]["shortcode"], "CODE");
        assert_eq!(server.await.unwrap().len(), 2);

        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(
                200,
                json!({
                    "success": false,
                    "status_code": 403,
                    "error_code": "instagram_login_required",
                    "message": "Login required"
                }),
            ),
            MockResponse::json(
                200,
                json!({"success": true, "posts": [{"shortcode": "CODE"}]}),
            ),
        ])
        .await;
        let client = scrappa_client(base_url);
        let request =
            resolve_input(&json!({"url": "https://www.instagram.com/name/p/CODE/"})).unwrap();
        let result = client
            .fetch_post(&request, Instant::now() + REQUEST_TIMEOUT)
            .await
            .unwrap();
        assert_eq!(result["data"]["shortcode"], "CODE");
        assert_eq!(server.await.unwrap().len(), 2);

        for (input, failure) in [
            (
                json!({"url": "https://www.instagram.com/name/p/CODE/"}),
                json!({"error_code": "invalid_api_key", "message": "Invalid API key"}),
            ),
            (
                json!({"url": "https://www.instagram.com/name/p/CODE/"}),
                json!({"message": "Rate limited", "retryable": false}),
            ),
            (
                json!({"shortcode": "CODE"}),
                json!({"message": "Temporarily unavailable", "retryable": true}),
            ),
        ] {
            let (base_url, server) =
                start_mock_server(vec![MockResponse::json(503, failure)]).await;
            let client = scrappa_client(base_url);
            let request = resolve_input(&input).unwrap();
            assert!(client
                .fetch_post(&request, Instant::now() + REQUEST_TIMEOUT)
                .await
                .is_err());
            assert_eq!(server.await.unwrap().len(), 1);
        }
    }

    #[tokio::test]
    async fn authentication_error_is_not_retried_without_a_rate_limit() {
        let (base_url, server) = start_mock_server(vec![MockResponse::json(
            401,
            json!({"message": "Authentication required"}),
        )])
        .await;
        let client = scrappa_client(base_url);
        let request = resolve_input(&json!({"shortcode": "CODE"})).unwrap();
        let error = request_with_retry_policy(
            &client,
            &request,
            &[Duration::ZERO, Duration::ZERO],
            REQUEST_TIMEOUT,
        )
        .await
        .unwrap_err();
        assert_eq!(error.http_status, Some(401));
        assert_eq!(server.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn rate_limit_can_be_followed_by_a_cooldown_auth_retry() {
        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(500, json!({"error": "Rate limited (HTTP 429)"})),
            MockResponse::json(401, json!({"error": "Authentication required (HTTP 401)"})),
            MockResponse::json(200, json!({"success": true, "data": {"shortcode": "CODE"}})),
        ])
        .await;
        let client = scrappa_client(base_url);
        let request = resolve_input(&json!({"shortcode": "CODE"})).unwrap();
        let result = request_with_retry_policy(
            &client,
            &request,
            &[Duration::ZERO, Duration::ZERO],
            REQUEST_TIMEOUT,
        )
        .await
        .unwrap();
        assert_eq!(result["data"]["shortcode"], "CODE");
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| request.path.starts_with("/instagram/post?shortcode=CODE")));
    }

    #[tokio::test]
    async fn fallback_shares_the_single_attempt_deadline() {
        let (base_url, server) = start_mock_server(vec![
            MockResponse::delayed_json(
                503,
                json!({"message": "Temporarily unavailable", "retryable": true}),
                Duration::from_millis(35),
            ),
            MockResponse::delayed_json(
                200,
                json!({"success": true, "posts": []}),
                Duration::from_millis(100),
            ),
        ])
        .await;
        let client = scrappa_client(base_url);
        let request =
            resolve_input(&json!({"url": "https://www.instagram.com/instagram/p/CODE/"})).unwrap();
        let error = request_with_retry_policy(&client, &request, &[], Duration::from_millis(70))
            .await
            .unwrap_err();
        assert!(error.timed_out);
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn actor_publishes_raw_response_to_dataset_and_output_store() {
        let output = json!({"success": true, "data": {"shortcode": "CODE", "caption": "hello"}});
        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(200, json!({"url": "CODE"})),
            MockResponse::json(200, output.clone()),
            MockResponse::json(200, run_pricing(1.0, 0)),
            MockResponse::json(201, json!({})),
            MockResponse::json(201, json!({})),
        ])
        .await;
        run_actor_with_config(config(base_url)).await.unwrap();
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 5);
        assert_eq!(requests[0].method, "GET");
        assert!(requests[0].path.ends_with("/records/INPUT"));
        assert!(requests[1]
            .path
            .starts_with("/instagram/post?shortcode=CODE"));
        assert_eq!(requests[2].path, "/v2/actor-runs/run-1");
        assert_eq!(requests[3].method, "POST");
        assert_eq!(requests[3].path, "/v2/datasets/dataset-1/items");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[3].body).unwrap(),
            output
        );
        assert_eq!(requests[4].method, "PUT");
        assert!(requests[4].path.ends_with("/records/OUTPUT"));
        assert_eq!(
            serde_json::from_str::<Value>(&requests[4].body).unwrap(),
            output
        );
        assert!(requests
            .iter()
            .filter(|request| request.path.starts_with("/v2/"))
            .all(|request| request
                .headers
                .to_ascii_lowercase()
                .contains("authorization: bearer test-apify-token")));
    }

    #[tokio::test]
    async fn exhausted_budget_skips_dataset_charge_but_keeps_output_record() {
        let output = json!({"success": true, "data": {"shortcode": "CODE"}});
        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(200, json!({"shortcode": "CODE"})),
            MockResponse::json(200, output.clone()),
            MockResponse::json(200, run_pricing(0.0001, 0)),
            MockResponse::json(201, json!({})),
        ])
        .await;
        run_actor_with_config(config(base_url)).await.unwrap();
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests
            .iter()
            .all(|request| !request.path.starts_with("/v2/datasets/")));
        assert!(requests.last().unwrap().path.ends_with("/records/OUTPUT"));
    }

    #[tokio::test]
    async fn failed_lookup_does_not_publish_a_dataset_item_or_output_record() {
        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(200, json!({"shortcode": "CODE"})),
            MockResponse::json(
                503,
                json!({"message": "Upstream rejected the request", "retryable": false}),
            ),
        ])
        .await;
        let error = run_actor_with_config(config(base_url)).await.unwrap_err();
        assert_eq!(
            actor_failure_message(&error),
            "Scrappa Instagram Post API request failed (503): Upstream rejected the request"
        );
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests.iter().all(|request| {
            !request.path.starts_with("/v2/datasets/") && !request.path.ends_with("/records/OUTPUT")
        }));
    }
}

async fn run_actor_with_config(config: Config) -> Result<()> {
    let apify_http = Client::builder()
        .build()
        .context("Could not create Apify HTTP client")?;
    let scrappa_http = Client::builder()
        .build()
        .context("Could not create Scrappa HTTP client")?;
    let apify = ApifyClient::new(apify_http, &config);
    let scrappa = ScrappaClient::new(
        scrappa_http,
        config.scrappa_api_base.clone(),
        config.scrappa_api_key.clone(),
    );

    let input = apify.get_input().await?.unwrap_or(Value::Null);
    let request = resolve_input(&input)?;
    let data = request_with_retries(&scrappa, &request).await?;

    let dataset_capacity = apify.run_dataset_capacity(1).await?;
    if dataset_capacity > 0 {
        apify.push_dataset_item(&data).await?;
    } else {
        eprintln!("Pay-per-event spending limit cannot cover another dataset item; kept the response in OUTPUT only.");
    }

    apify.put_output(&data).await?;
    eprintln!(
        "Successfully fetched data for Instagram post: {}",
        request.identifier
    );
    Ok(())
}
