use std::{env, process, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Method, Response, StatusCode};
use serde_json::{json, Map, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: usize = 8;
const APIFY_MIN_RETRY_DELAY: Duration = Duration::from_millis(500);
const APIFY_MAX_RETRY_DELAY: Duration = Duration::from_secs(30);
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

#[derive(Debug)]
struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
    scrappa_api_key: String,
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
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            apify_token: required_env("APIFY_TOKEN")?,
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
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env_or_default(name, default);
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn endpoint_url(base_url: &Url, path: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(path);
    Ok(url)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct TikTokProfileParams {
    unique_id: Option<String>,
    user_id: Option<String>,
}

impl TikTokProfileParams {
    fn append_to_url(&self, url: &mut Url) {
        let mut query = url.query_pairs_mut();
        if let Some(unique_id) = &self.unique_id {
            query.append_pair("unique_id", unique_id);
        }
        if let Some(user_id) = &self.user_id {
            query.append_pair("user_id", user_id);
        }
    }

    fn lookup_for_log(&self) -> String {
        if let Some(unique_id) = &self.unique_id {
            return unique_id.clone();
        }
        if let Some(user_id) = &self.user_id {
            return format!("user_id:{user_id}");
        }
        "unknown TikTok profile".to_owned()
    }
}

fn build_tiktok_profile_params(input: &Value) -> Result<TikTokProfileParams> {
    let mut params = TikTokProfileParams::default();

    if let Some(profile) = input.get("profile") {
        if let Some(profile) = profile.as_str() {
            let profile = js_trim(profile);
            if !profile.is_empty() {
                if profile.chars().all(|character| character.is_ascii_digit()) {
                    params.user_id = Some(normalize_tiktok_user_id(profile)?);
                } else {
                    params.unique_id = Some(normalize_tiktok_unique_id(profile)?);
                }
            }
        } else if !profile.is_null() && profile != "" {
            eprintln!(
                "Warning: profile must be a string, got {}.",
                value_type(profile)
            );
        }
    }

    if !params.has_lookup() {
        if let Some(unique_id) = input.get("unique_id") {
            if let Some(unique_id) = unique_id.as_str() {
                let normalized = normalize_tiktok_unique_id(unique_id)?;
                if !normalized.is_empty() {
                    params.unique_id = Some(normalized);
                }
            } else if !unique_id.is_null() && unique_id != "" {
                eprintln!(
                    "Warning: unique_id must be a string, got {}.",
                    value_type(unique_id)
                );
            }
        }
    }

    if !params.has_lookup() {
        if let Some(user_id) = input.get("user_id") {
            if let Some(user_id) = user_id.as_str() {
                let normalized = normalize_tiktok_user_id(user_id)?;
                if !normalized.is_empty() {
                    params.user_id = Some(normalized);
                }
            } else if !user_id.is_null() && user_id != "" {
                eprintln!(
                    "Warning: user_id must be a string, got {}.",
                    value_type(user_id)
                );
            }
        }
    }

    if !params.has_lookup() {
        bail!("TikTok unique_id or user_id is required");
    }
    Ok(params)
}

impl TikTokProfileParams {
    fn has_lookup(&self) -> bool {
        self.unique_id.is_some() || self.user_id.is_some()
    }
}

fn normalize_tiktok_unique_id(value: &str) -> Result<String> {
    let value = js_trim(value);
    if value.is_empty() {
        return Ok(String::new());
    }

    if let Ok(parsed) = Url::parse(value) {
        let host = parsed.host_str().unwrap_or_default();
        if !host.eq_ignore_ascii_case("tiktok.com")
            && !host.to_ascii_lowercase().ends_with(".tiktok.com")
        {
            bail!("TikTok profile URL must be on tiktok.com");
        }
        if parsed.scheme() != "https" {
            bail!("TikTok profile URL must use HTTPS");
        }

        let username = profile_url_username(parsed.path()).ok_or_else(|| {
            anyhow!("TikTok profile URL must use the format https://www.tiktok.com/@username")
        })?;
        return normalize_tiktok_username(username);
    }

    if is_url_like(value) {
        bail!("A valid TikTok profile URL or username is required");
    }
    normalize_tiktok_username(value)
}

fn profile_url_username(path: &str) -> Option<&str> {
    let path = path.strip_prefix('/')?;
    let path = path.strip_suffix('/').unwrap_or(path);
    if path.contains('/') || !path.starts_with('@') {
        return None;
    }
    Some(path)
}

fn is_url_like(value: &str) -> bool {
    if value.starts_with("//") {
        return true;
    }

    if let Some((scheme, _)) = value.split_once("://") {
        if !scheme.is_empty()
            && scheme.as_bytes()[0].is_ascii_alphabetic()
            && scheme
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'))
        {
            return true;
        }
    }

    let value = value.to_ascii_lowercase();
    value.match_indices("tiktok.com").any(|(start, _)| {
        let end = start + "tiktok.com".len();
        let bytes = value.as_bytes();
        let domain_start = start == 0 || bytes.get(start - 1) == Some(&b'.');
        let domain_end = end == bytes.len() || bytes.get(end) == Some(&b'/');
        domain_start && domain_end
    })
}

fn normalize_tiktok_username(value: &str) -> Result<String> {
    let username = value.strip_prefix('@').unwrap_or(value);
    if !(2..=255).contains(&username.len())
        || !username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'))
    {
        bail!("TikTok username must be 2 to 255 characters and contain only letters, numbers, dots, or underscores");
    }
    Ok(format!("@{username}"))
}

fn normalize_tiktok_user_id(value: &str) -> Result<String> {
    let value = js_trim(value);
    if value.is_empty() {
        return Ok(String::new());
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("TikTok user_id must contain digits only");
    }
    if value.len() > 30 {
        bail!("TikTok numeric user ID must be 30 digits or fewer");
    }
    Ok(value.to_owned())
}

fn js_trim(value: &str) -> &str {
    value.trim_matches(is_js_whitespace)
}

fn is_js_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'..='\u{000d}'
            | '\u{0020}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null | Value::Array(_) | Value::Object(_) => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
    }
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

fn normalize_tiktok_profile_record(profile: &Value) -> Result<Map<String, Value>> {
    let mut normalized = match profile {
        Value::Object(profile) => profile.clone(),
        Value::Array(profile) => profile
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        Value::String(profile) => profile
            .chars()
            .enumerate()
            .map(|(index, value)| (index.to_string(), Value::String(value.to_string())))
            .collect(),
        Value::Null | Value::Bool(_) | Value::Number(_) => Map::new(),
    };
    let user = profile.get("user");
    let stats = profile.get("stats");

    let user_id = match profile.get("user_id") {
        Some(value) if !value.is_null() => Some(value.clone()),
        _ => user
            .and_then(|user| user.get("id"))
            .filter(|value| !value.is_null())
            .map(|value| Value::String(js_string(value))),
    };
    set_optional(&mut normalized, "user_id", user_id);

    let unique_id = match normalize_optional_unique_id(profile.get("unique_id"))? {
        Some(value) => Some(value),
        None => normalize_optional_unique_id(user.and_then(|user| user.get("uniqueId")))?,
    };
    set_optional(&mut normalized, "unique_id", unique_id);

    for (output_key, top_level_key, nested, nested_key) in [
        ("nickname", "nickname", user, "nickname"),
        ("signature", "signature", user, "signature"),
        ("verified", "verified", user, "verified"),
        ("private_account", "private_account", user, "privateAccount"),
        ("region", "region", user, "region"),
        ("language", "language", user, "language"),
        ("follower_count", "follower_count", stats, "followerCount"),
        (
            "following_count",
            "following_count",
            stats,
            "followingCount",
        ),
        ("heart_count", "heart_count", stats, "heartCount"),
        ("video_count", "video_count", stats, "videoCount"),
        ("digg_count", "digg_count", stats, "diggCount"),
    ] {
        let value = nullish_fallback(
            profile.get(top_level_key),
            nested.and_then(|value| value.get(nested_key)),
        );
        set_optional(&mut normalized, output_key, value);
    }

    let avatar = first_non_null_or_last([
        profile.get("avatar"),
        user.and_then(|user| user.get("avatarLarger")),
        user.and_then(|user| user.get("avatarMedium")),
        user.and_then(|user| user.get("avatarThumb")),
    ]);
    set_optional(&mut normalized, "avatar", avatar);

    Ok(normalized)
}

fn normalize_optional_unique_id(value: Option<&Value>) -> Result<Option<Value>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .ok_or_else(|| anyhow!("TikTok profile unique_id must be a string"))?;
    if value.is_empty() {
        return Ok(None);
    }
    let normalized = if value.starts_with('@') {
        value.to_owned()
    } else {
        format!("@{value}")
    };
    Ok(Some(Value::String(normalized)))
}

fn nullish_fallback(primary: Option<&Value>, fallback: Option<&Value>) -> Option<Value> {
    primary
        .filter(|value| !value.is_null())
        .or(fallback)
        .cloned()
}

fn first_non_null_or_last(values: [Option<&Value>; 4]) -> Option<Value> {
    for value in values.iter().flatten() {
        if !value.is_null() {
            return Some((*value).clone());
        }
    }
    values[3].cloned()
}

fn set_optional(object: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        object.insert(key.to_owned(), value);
    } else {
        object.remove(key);
    }
}

fn extract_profile(data: Option<&Value>) -> Option<&Value> {
    match data? {
        Value::Null | Value::Bool(false) => None,
        Value::Number(number) if number.as_f64() == Some(0.0) => None,
        Value::String(value) if value.is_empty() => None,
        Value::Array(profiles) => {
            if profiles.len() > 1 {
                eprintln!("Scrappa returned {} profiles for a single lookup. Saving the first profile only.", profiles.len());
            }
            profiles.first().filter(|profile| !profile.is_null())
        }
        profile => Some(profile),
    }
}

struct ScrappaClient {
    http: reqwest::Client,
    base_url: Url,
    api_key: String,
}

impl ScrappaClient {
    fn new(http: reqwest::Client, base_url: Url, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    async fn get_profile(&self, params: &TikTokProfileParams) -> Result<Value> {
        let mut url = endpoint_url(&self.base_url, &["tiktok", "user", "profile"])?;
        params.append_to_url(&mut url);

        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(scrappa_transport_error)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.map_err(scrappa_transport_error)?;
            bail!(
                "Scrappa API error ({}): {}",
                status.as_u16(),
                scrappa_error_message(status, &body)
            );
        }

        response.json::<Value>().await.map_err(|error| {
            if error.is_timeout() {
                scrappa_transport_error(error)
            } else {
                anyhow!("Scrappa API response was not valid JSON")
            }
        })
    }
}

fn scrappa_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            SCRAPPA_REQUEST_TIMEOUT.as_millis()
        )
    } else {
        anyhow!("Scrappa API request failed: {error}")
    }
}

fn scrappa_error_message(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    if body.is_empty() {
        return fallback;
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let mut message = error_data
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or(fallback);
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    let messages = messages.as_array()?;
                    let messages = messages
                        .iter()
                        .map(js_string)
                        .collect::<Vec<_>>()
                        .join(", ");
                    Some(format!("{field}: {messages}"))
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

struct ApifyClient {
    http: reqwest::Client,
    base_url: Url,
    token: String,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
}

impl ApifyClient {
    fn new(http: reqwest::Client, config: &ActorConfig) -> Self {
        Self {
            http,
            base_url: config.apify_api_base_url.clone(),
            token: config.apify_token.clone(),
            default_key_value_store_id: config.default_key_value_store_id.clone(),
            default_dataset_id: config.default_dataset_id.clone(),
            actor_run_id: config.actor_run_id.clone(),
            input_key: config.input_key.clone(),
        }
    }

    async fn get_input(&self) -> Result<Option<Value>> {
        let url = endpoint_url(
            &self.base_url,
            &[
                "v2",
                "key-value-stores",
                &self.default_key_value_store_id,
                "records",
                &self.input_key,
            ],
        )?;
        let response = self
            .send_request(Method::GET, url, None, "input retrieval", true)
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = require_apify_success(response, "input retrieval").await?;
        response
            .json::<Value>()
            .await
            .context("Apify input record was not valid JSON")
            .map(Some)
    }

    async fn get_run(&self) -> Result<Value> {
        let url = endpoint_url(&self.base_url, &["v2", "actor-runs", &self.actor_run_id])?;
        let response = self
            .send_request(Method::GET, url, None, "run pricing request", true)
            .await?;
        let response = require_apify_success(response, "run pricing request").await?;
        response
            .json::<Value>()
            .await
            .context("Apify run pricing response was not valid JSON")
    }

    async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = endpoint_url(
            &self.base_url,
            &["v2", "datasets", &self.default_dataset_id, "items"],
        )?;
        let response = self
            .send_request(
                Method::POST,
                url,
                Some(item),
                "dataset item publication",
                false,
            )
            .await?;
        require_apify_success(response, "dataset item publication").await?;
        Ok(())
    }

    async fn put_record(&self, key: &str, value: &Value) -> Result<()> {
        let url = endpoint_url(
            &self.base_url,
            &[
                "v2",
                "key-value-stores",
                &self.default_key_value_store_id,
                "records",
                key,
            ],
        )?;
        let response = self
            .send_request(
                Method::PUT,
                url,
                Some(value),
                "key-value-store publication",
                true,
            )
            .await?;
        require_apify_success(response, "key-value-store publication").await?;
        Ok(())
    }

    async fn send_request(
        &self,
        method: Method,
        url: Url,
        body: Option<&Value>,
        operation: &str,
        retry_timeouts: bool,
    ) -> Result<Response> {
        for retry_count in 0..=APIFY_MAX_RETRIES {
            let mut request = self
                .http
                .request(method.clone(), url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .timeout(APIFY_REQUEST_TIMEOUT);
            if let Some(body) = body {
                request = request.json(body);
            }

            match request.send().await {
                Ok(response)
                    if should_retry_status(response.status())
                        && retry_count < APIFY_MAX_RETRIES =>
                {
                    let delay = apify_retry_delay(retry_count);
                    eprintln!(
                        "Apify {operation} returned {}; retrying in {}ms (attempt {}/{})",
                        response.status().as_u16(),
                        delay.as_millis(),
                        retry_count + 1,
                        APIFY_MAX_RETRIES
                    );
                    drop(response);
                    tokio::time::sleep(delay).await;
                }
                Ok(response) => return Ok(response),
                Err(error)
                    if should_retry_transport_error(&error, retry_timeouts)
                        && retry_count < APIFY_MAX_RETRIES =>
                {
                    let delay = apify_retry_delay(retry_count);
                    eprintln!("Apify {operation} request failed: {error}; retrying in {}ms (attempt {}/{})", delay.as_millis(), retry_count + 1, APIFY_MAX_RETRIES);
                    tokio::time::sleep(delay).await;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify {operation} request failed"));
                }
            }
        }
        unreachable!("retry loop returns after the last attempt")
    }
}

fn should_retry_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn should_retry_transport_error(error: &reqwest::Error, retry_timeouts: bool) -> bool {
    if error.is_timeout() {
        return retry_timeouts;
    }
    error.is_connect() || error.is_request() || error.is_body()
}

fn apify_retry_delay(retry_count: usize) -> Duration {
    let multiplier = 1_u32.checked_shl(retry_count as u32).unwrap_or(u32::MAX);
    APIFY_MIN_RETRY_DELAY
        .saturating_mul(multiplier)
        .min(APIFY_MAX_RETRY_DELAY)
}

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    bail!(
        "Apify {operation} failed ({}): {status}{detail}",
        status.as_u16()
    );
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
        return Ok(requested);
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let Some(item_event) = events.get(DATASET_ITEM_EVENT) else {
        return Ok(requested);
    };
    let item_price = item_event
        .get("eventPriceUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    if !item_price.is_finite() || item_price < 0.0 {
        bail!("Apify run returned invalid dataset item pricing");
    }
    if item_price == 0.0 || requested == 0 {
        return Ok(requested);
    }

    let max_total_charge_usd = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => return Ok(requested),
        Some(limit) => limit
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?,
    };
    if max_total_charge_usd == 0.0 {
        return Ok(requested);
    }
    if !max_total_charge_usd.is_finite() || max_total_charge_usd < 0.0 {
        bail!("Apify run returned an invalid spending limit");
    }

    let charged_counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut charged_usd = 0.0;
    for (event_name, count) in charged_counts {
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

    let tolerance = f64::EPSILON * max_total_charge_usd.max(1.0);
    Ok((1..=requested)
        .take_while(|count| {
            charged_usd + *count as f64 * item_price <= max_total_charge_usd + tolerance
        })
        .count())
}

async fn run_actor(config: &ActorConfig, http: reqwest::Client) -> Result<()> {
    let apify = ApifyClient::new(http.clone(), config);
    let input = apify
        .get_input()
        .await?
        .ok_or_else(|| anyhow!("TikTok unique_id or user_id is required"))?;
    let params = build_tiktok_profile_params(&input)?;
    println!("Fetching TikTok profile for: {}", params.lookup_for_log());

    let scrappa = ScrappaClient::new(
        http,
        config.scrappa_api_base_url.clone(),
        config.scrappa_api_key.clone(),
    );
    let response = scrappa.get_profile(&params).await?;
    if let Some(code) = response.get("code") {
        if code.as_f64() != Some(0.0) {
            let message = response
                .get("msg")
                .filter(|message| !message.is_null())
                .map(js_string)
                .unwrap_or_else(|| "Unknown error".to_owned());
            bail!(
                "Scrappa TikTok Profile API returned code {}: {message}",
                js_string(code)
            );
        }
    }

    let normalized_profile = extract_profile(response.get("data"))
        .map(normalize_tiktok_profile_record)
        .transpose()?;
    if let Some(normalized_profile) = &normalized_profile {
        let mut item = normalized_profile.clone();
        item.insert(
            "lookup_unique_id".to_owned(),
            params
                .unique_id
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
        );
        item.insert(
            "lookup_user_id".to_owned(),
            params
                .user_id
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
        );

        let run = apify.get_run().await?;
        if affordable_dataset_items(&run, 1)? == 1 {
            // Apify automatically charges its default dataset-item synthetic event for this write.
            apify
                .push_dataset_item(&Value::Object(item.clone()))
                .await?;
            println!(
                "Found TikTok profile: {}",
                item.get("unique_id")
                    .or_else(|| item.get("user_id"))
                    .map(js_string)
                    .unwrap_or_else(|| "unknown".to_owned())
            );
        } else {
            println!("Charge limit reached before saving the TikTok profile; OUTPUT will still be written.");
        }
    } else {
        println!("No profile found for the given TikTok lookup");
    }

    apify.put_record("OUTPUT", &response).await?;

    let summary = json!({
        "profile_found": normalized_profile.is_some(),
        "unique_id": normalized_profile.as_ref().and_then(|profile| profile.get("unique_id")).cloned().unwrap_or(Value::Null),
        "user_id": normalized_profile.as_ref().and_then(|profile| profile.get("user_id")).cloned().unwrap_or(Value::Null),
        "follower_count": normalized_profile.as_ref().and_then(|profile| profile.get("follower_count")).cloned().unwrap_or(Value::Null),
        "processed_time": response.get("processed_time").cloned().unwrap_or(Value::Null),
    });
    println!("TikTok profile extraction completed successfully");
    println!("Results summary: {}", summary);
    Ok(())
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let http = reqwest::Client::builder()
        .build()
        .context("Could not create HTTP client")?;
    run_actor(&config, http).await
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests;
