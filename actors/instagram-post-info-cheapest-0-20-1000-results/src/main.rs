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
    let pricing_model = data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Apify run pricing model is missing"))?;
    if pricing_model != "PAY_PER_EVENT" {
        return Ok(requested);
    }

    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => return Ok(requested),
        Some(value) => value
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run did not provide a valid spending limit"))?,
    };
    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = events
        .get("apify-default-dataset-item")
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
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
    if max_charge == 0.0 {
        return Ok(0);
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
mod tests;

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

    apify.put_output(&data).await?;

    let dataset_capacity = apify.run_dataset_capacity(1).await?;
    if dataset_capacity > 0 {
        apify.push_dataset_item(&data).await?;
    } else {
        eprintln!("Pay-per-event spending limit cannot cover another dataset item; kept the response in OUTPUT only.");
    }

    eprintln!(
        "Successfully fetched data for Instagram post: {}",
        request.identifier
    );
    Ok(())
}
