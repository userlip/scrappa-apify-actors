use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Method, Response, StatusCode};
use serde_json::{json, Map, Number, Value};
use std::{
    collections::{HashMap, HashSet},
    env,
    time::Duration,
};
use tokio::time::sleep;
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: usize = 8;
const APIFY_MIN_RETRY_DELAY: Duration = Duration::from_millis(500);
const CHALLENGE_RESULT_CHARGE_EVENT: &str = "challenge-result";
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

#[derive(Clone, Debug)]
pub struct ActorConfig {
    pub apify_api_base_url: Url,
    pub scrappa_api_base_url: Url,
    pub default_key_value_store_id: String,
    pub default_dataset_id: String,
    pub actor_run_id: String,
    pub input_key: String,
    pub apify_token: String,
    pub scrappa_api_key: String,
}

impl ActorConfig {
    pub fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|key| !key.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
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

pub struct ActorClient {
    config: ActorConfig,
    apify_http: reqwest::Client,
    scrappa_http: reqwest::Client,
}

impl ActorClient {
    pub fn new(config: ActorConfig) -> Result<Self> {
        let apify_http = reqwest::Client::builder()
            .build()
            .context("Could not create Apify HTTP client")?;
        let scrappa_http = reqwest::Client::builder()
            .build()
            .context("Could not create Scrappa HTTP client")?;
        Ok(Self {
            config,
            apify_http,
            scrappa_http,
        })
    }

    async fn send_apify_json(
        &self,
        method: Method,
        url: Url,
        body: Option<&Value>,
        operation: &str,
    ) -> Result<Response> {
        for attempt in 0..=APIFY_MAX_RETRIES {
            let mut request = self
                .apify_http
                .request(method.clone(), url.clone())
                .bearer_auth(&self.config.apify_token)
                .header(reqwest::header::ACCEPT, "application/json")
                .timeout(APIFY_REQUEST_TIMEOUT);
            if let Some(body) = body {
                request = request.json(body);
            }

            match request.send().await {
                Ok(response) if response.status().is_success() => {
                    return Ok(response);
                }
                Ok(response)
                    if is_retryable_status(response.status()) && attempt < APIFY_MAX_RETRIES =>
                {
                    eprintln!(
                        "Apify API request for {operation} failed with {}; retrying ({}/{APIFY_MAX_RETRIES})",
                        response.status(),
                        attempt + 1
                    );
                }
                Ok(response) => return Err(apify_response_error(response, operation).await),
                Err(error) if attempt < APIFY_MAX_RETRIES => {
                    eprintln!(
                        "Apify API request for {operation} failed: {error}; retrying ({}/{APIFY_MAX_RETRIES})",
                        attempt + 1
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "Apify API request failed while trying to {operation}: {error}"
                    ));
                }
            }

            sleep(apify_retry_delay(attempt)).await;
        }

        unreachable!("the retry loop returns after its final attempt")
    }

    async fn apify_json(
        &self,
        method: Method,
        url: Url,
        body: Option<&Value>,
        operation: &str,
    ) -> Result<Value> {
        let response = self.send_apify_json(method, url, body, operation).await?;
        response.json().await.with_context(|| {
            format!("Apify API response while trying to {operation} is not valid JSON")
        })
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &[
                "v2",
                "key-value-stores",
                &self.config.default_key_value_store_id,
                "records",
                &self.config.input_key,
            ],
        )?;
        let response = self
            .send_apify_json(Method::GET, url, None, "fetch Actor input")
            .await;
        let response = match response {
            Ok(response) => response,
            Err(error) if error.to_string().contains("(404)") => return Ok(None),
            Err(error) => return Err(error),
        };
        response
            .json()
            .await
            .map(Some)
            .context("Actor input record is not valid JSON")
    }

    async fn get_run_pricing(&self) -> Result<Value> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "actor-runs", &self.config.actor_run_id],
        )?;
        self.apify_json(Method::GET, url, None, "fetch Actor run pricing")
            .await
    }

    async fn store_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "datasets", &self.config.default_dataset_id, "items"],
        )?;
        self.send_apify_json(
            Method::POST,
            url,
            Some(&json!(items)),
            "store dataset items",
        )
        .await?;
        Ok(())
    }

    async fn charge_event(&self, event_name: &str, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "actor-runs", &self.config.actor_run_id, "charge"],
        )?;
        self.send_apify_json(
            Method::POST,
            url,
            Some(&json!({ "eventName": event_name, "count": count })),
            "charge Actor run events",
        )
        .await?;
        Ok(())
    }

    async fn set_output(&self, output: &Value) -> Result<()> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &[
                "v2",
                "key-value-stores",
                &self.config.default_key_value_store_id,
                "records",
                "OUTPUT",
            ],
        )?;
        self.send_apify_json(Method::PUT, url, Some(output), "write OUTPUT")
            .await?;
        Ok(())
    }

    async fn search_challenges(&self, request: &SearchRequest) -> Result<Value> {
        let mut url = endpoint_url(
            &self.config.scrappa_api_base_url,
            &["tiktok", "challenges", "search"],
        )?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("keywords", &request.keyword);
            if let Some(count) = request.count {
                query.append_pair("count", &count.to_string());
            }
        }

        let response = self
            .scrappa_http
            .get(url)
            .header("X-API-Key", &self.config.scrappa_api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    anyhow!(
                        "Scrappa API request timed out after {}ms",
                        SCRAPPA_REQUEST_TIMEOUT.as_millis()
                    )
                } else {
                    anyhow!("Scrappa API request failed: {error}")
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let fallback = status
                .canonical_reason()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            let body = response.text().await.map_err(|error| {
                if error.is_timeout() {
                    anyhow!(
                        "Scrappa API request timed out after {}ms",
                        SCRAPPA_REQUEST_TIMEOUT.as_millis()
                    )
                } else {
                    anyhow!("Scrappa API error response could not be read: {error}")
                }
            })?;
            let message = scrappa_error_message(&body, &fallback);
            bail!("Scrappa API error ({}): {message}", status.as_u16());
        }

        response.json().await.map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "Scrappa API request timed out after {}ms",
                    SCRAPPA_REQUEST_TIMEOUT.as_millis()
                )
            } else {
                anyhow!("Scrappa API response was not valid JSON: {error}")
            }
        })
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn apify_retry_delay(attempt: usize) -> Duration {
    APIFY_MIN_RETRY_DELAY.saturating_mul(2_u32.saturating_pow(attempt.min(16) as u32))
}

async fn apify_response_error(response: Response, operation: &str) -> anyhow::Error {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        format!("HTTP {}", status.as_u16())
    } else {
        body
    };
    anyhow!(
        "Apify API error ({}) while trying to {operation}: {detail}",
        status.as_u16()
    )
}

fn scrappa_error_message(body: &str, fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
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
                        .unwrap_or_else(|| js_string(messages));
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

#[derive(Clone, Debug, PartialEq)]
pub struct SearchRequest {
    pub keyword: String,
    pub count: Option<u64>,
}

fn is_js_whitespace(character: char) -> bool {
    character.is_whitespace() || character == '\u{feff}'
}

fn normalize_keyword(value: &str) -> Result<Option<String>> {
    let keyword = value
        .trim_matches(is_js_whitespace)
        .split(is_js_whitespace)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if keyword.is_empty() {
        return Ok(None);
    }
    if keyword.encode_utf16().count() > 255 {
        bail!("TikTok challenge search keywords must be 255 characters or fewer");
    }
    if value
        .chars()
        .any(|character| matches!(character, '\r' | '\n' | '\t' | '\u{000c}' | '\u{000b}'))
    {
        bail!("TikTok challenge search keywords cannot contain tabs, line breaks, or control whitespace");
    }
    Ok(Some(keyword))
}

fn js_type(value: &Value) -> &'static str {
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

pub fn build_search_requests(input: &Value) -> Result<(Vec<SearchRequest>, Vec<String>)> {
    let mut warnings = Vec::new();
    let mut keywords = Vec::new();
    let mut seen = HashSet::new();
    let input_object = input.as_object();
    let raw_keywords = input_object.and_then(|input| input.get("keywords"));

    match raw_keywords {
        Some(Value::Array(values)) => {
            for value in values {
                let Some(value) = value.as_str() else {
                    if !value.is_null() {
                        warnings.push(format!(
                            "keywords entries must be strings, got {}. Omitting entry.",
                            js_type(value)
                        ));
                    }
                    continue;
                };
                if let Some(keyword) = normalize_keyword(value)? {
                    if seen.insert(keyword.clone()) {
                        keywords.push(keyword);
                    }
                }
            }
        }
        Some(Value::String(value)) => {
            if let Some(keyword) = normalize_keyword(value)? {
                seen.insert(keyword.clone());
                keywords.push(keyword);
            }
        }
        Some(Value::Null) | None => {}
        Some(value) => warnings.push(format!(
            "keywords must be an array of strings, got {}. Falling back to keyword.",
            js_type(value)
        )),
    }

    if keywords.is_empty() {
        match input_object.and_then(|input| input.get("keyword")) {
            Some(Value::String(value)) => {
                if let Some(keyword) = normalize_keyword(value)? {
                    if seen.insert(keyword.clone()) {
                        keywords.push(keyword);
                    }
                }
            }
            Some(Value::Null) | None => {}
            Some(value) => {
                warnings.push(format!("keyword must be a string, got {}.", js_type(value)))
            }
        }
    }

    if keywords.is_empty() {
        bail!("At least one TikTok challenge search keyword is required");
    }

    let raw_count = input_object.and_then(|input| input.get("count"));
    let count = match raw_count {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) if value.is_empty() => None,
        Some(Value::Number(number)) => valid_count(number),
        Some(_) => None,
    };
    if let Some(raw_count) = raw_count {
        let is_missing = raw_count.is_null() || raw_count.as_str().is_some_and(str::is_empty);
        if !is_missing && count.is_none() {
            warnings.push(format!(
                "count must be an integer between 1 and 50, got {}. Using Scrappa default.",
                js_string(raw_count)
            ));
        }
    }

    Ok((
        keywords
            .into_iter()
            .map(|keyword| SearchRequest { keyword, count })
            .collect(),
        warnings,
    ))
}

fn valid_count(number: &Number) -> Option<u64> {
    let value = number.as_f64()?;
    if value.is_finite() && value.fract() == 0.0 && (1.0..=50.0).contains(&value) {
        Some(value as u64)
    } else {
        None
    }
}

pub fn format_lookup(requests: &[SearchRequest]) -> String {
    match requests {
        [] => "unknown TikTok challenge search".to_owned(),
        [request] => request.keyword.clone(),
        requests => format!("{} TikTok challenge searches", requests.len()),
    }
}

fn coalesce_non_null<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Option<&'a Value> {
    values.into_iter().flatten().find(|value| !value.is_null())
}

fn challenge_id(challenge: &Value) -> Value {
    let id = coalesce_non_null([
        challenge.get("challenge_id"),
        challenge.get("id"),
        challenge.get("cid"),
    ]);
    match id {
        Some(Value::String(value)) => {
            let value = value.trim_matches(is_js_whitespace);
            if value.is_empty() {
                Value::Null
            } else {
                Value::String(value.to_owned())
            }
        }
        Some(Value::Number(number)) => {
            let Some(number) = number.as_f64() else { return Value::Null };
            if number.is_finite() && number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER {
                Value::String(if number == 0.0 {
                    "0".to_owned()
                } else {
                    number.to_string().trim_end_matches(".0").to_owned()
                })
            } else {
                Value::Null
            }
        }
        _ => Value::Null,
    }
}

fn challenge_name(challenge: &Value) -> Value {
    coalesce_non_null([
        challenge.get("challenge_name"),
        challenge.get("cha_name"),
        challenge.get("name"),
        challenge.get("title"),
    ])
    .and_then(Value::as_str)
    .map(|value| value.trim_matches(is_js_whitespace))
    .filter(|value| !value.is_empty())
    .map(|value| Value::String(value.to_owned()))
    .unwrap_or(Value::Null)
}

fn normalized_challenge(challenge: &Value, request: &SearchRequest) -> Result<Value> {
    if challenge.is_null() {
        bail!("Cannot normalize a null TikTok challenge result");
    }

    let mut result = match challenge {
        Value::Object(fields) => fields.clone(),
        Value::String(value) => value
            .chars()
            .enumerate()
            .map(|(index, character)| (index.to_string(), Value::String(character.to_string())))
            .collect(),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        _ => Map::new(),
    };
    let stats = challenge.get("stats");
    let description = challenge
        .get("description")
        .and_then(Value::as_str)
        .map(|value| Value::String(value.to_owned()))
        .or_else(|| {
            challenge
                .get("desc")
                .filter(|value| !value.is_null())
                .cloned()
        })
        .unwrap_or(Value::Null);

    result.insert("challenge_id".to_owned(), challenge_id(challenge));
    result.insert("challenge_name".to_owned(), challenge_name(challenge));
    result.insert("description".to_owned(), description);
    result.insert(
        "view_count".to_owned(),
        coalesce_non_null([
            challenge.get("view_count"),
            stats.and_then(|value| value.get("view_count")),
        ])
        .cloned()
        .unwrap_or(Value::Null),
    );
    result.insert(
        "video_count".to_owned(),
        coalesce_non_null([
            challenge.get("video_count"),
            stats.and_then(|value| value.get("video_count")),
        ])
        .cloned()
        .unwrap_or(Value::Null),
    );
    result.insert(
        "user_count".to_owned(),
        coalesce_non_null([
            challenge.get("user_count"),
            stats.and_then(|value| value.get("user_count")),
        ])
        .cloned()
        .unwrap_or(Value::Null),
    );
    result.insert("request_keyword".to_owned(), json!(request.keyword));
    result.insert(
        "request_count".to_owned(),
        request.count.map_or(Value::Null, |count| json!(count)),
    );
    Ok(Value::Object(result))
}

pub fn extract_challenges(data: Option<&Value>) -> Vec<Value> {
    let Some(data) = data.filter(|value| !value.is_null()) else {
        return Vec::new();
    };
    if let Some(challenges) = data.as_array() {
        return challenges.clone();
    }
    for field in [
        "challenges",
        "challenge_list",
        "challengeList",
        "items",
        "results",
    ] {
        if let Some(challenges) = data.get(field).and_then(Value::as_array) {
            return challenges.clone();
        }
    }
    Vec::new()
}

fn validate_scrappa_response(response: &Value) -> Result<()> {
    let Some(code) = response.get("code") else {
        return Ok(());
    };
    if code.as_f64() == Some(0.0) {
        return Ok(());
    }
    let message = response
        .get("msg")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or_else(|| "Unknown error".to_owned());
    bail!(
        "Scrappa TikTok Challenge Search API returned code {}: {message}",
        js_string(code)
    );
}

#[derive(Clone, Debug)]
pub struct ChargeBudget {
    pub is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    charged_counts: HashMap<String, f64>,
}

impl ChargeBudget {
    pub fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let is_pay_per_event = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self {
                is_pay_per_event: false,
                max_total_charge_usd: f64::INFINITY,
                event_prices: HashMap::new(),
                charged_counts: HashMap::new(),
            });
        }

        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (event_name, event) in events {
            let price = event
                .get("eventPriceUsd")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    anyhow!("Apify run did not provide the price for event {event_name}")
                })?;
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned an invalid price for event {event_name}");
            }
            event_prices.insert(event_name.clone(), price);
        }

        let raw_limit = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64);
        let max_total_charge_usd = match raw_limit {
            Some(limit) if limit != 0.0 => limit,
            _ => f64::INFINITY,
        };
        if max_total_charge_usd.is_finite() && max_total_charge_usd < 0.0 {
            bail!("Apify run returned an invalid spending limit");
        }

        let mut charged_counts = HashMap::new();
        if let Some(counts) = data.get("chargedEventCounts").and_then(Value::as_object) {
            for (event_name, count) in counts {
                let count = count
                    .as_f64()
                    .filter(|count| count.is_finite() && *count >= 0.0)
                    .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
                charged_counts.insert(event_name.clone(), count);
            }
        }

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_counts,
        })
    }

    fn charged_total(&self) -> f64 {
        let total = self
            .charged_counts
            .iter()
            .map(|(event_name, count)| {
                self.event_prices.get(event_name).copied().unwrap_or(0.0) * count
            })
            .sum::<f64>();
        (total * 1_000_000.0).round() / 1_000_000.0
    }

    fn event_price_for_limit(&self, event_name: &str) -> f64 {
        self.event_prices.get(event_name).copied().unwrap_or(0.0)
    }

    fn max_count_for_price(&self, price: f64) -> usize {
        if price == 0.0 {
            return usize::MAX;
        }
        let remaining = (self.max_total_charge_usd - self.charged_total()) / price;
        if !remaining.is_finite() {
            return usize::MAX;
        }
        let rounded = format!("{remaining:.4}")
            .parse::<f64>()
            .unwrap_or(remaining);
        rounded.floor().max(0.0).min(usize::MAX as f64) as usize
    }

    pub fn chargeable_event_capacity(&self, event_name: &str) -> usize {
        if !self.is_pay_per_event {
            return usize::MAX;
        }
        self.max_count_for_price(self.event_price_for_limit(event_name))
    }

    fn push_capacity(&self, event_name: &str) -> usize {
        if !self.is_pay_per_event {
            return usize::MAX;
        }
        let item_price = self.event_price_for_limit(event_name)
            + self.event_price_for_limit("apify-default-dataset-item");
        self.max_count_for_price(item_price)
    }

    fn item_count_to_push(&self, requested: usize, event_name: &str) -> usize {
        if requested == 0 {
            return 0;
        }
        let max_count = self.push_capacity(event_name);
        if max_count >= requested {
            return requested;
        }
        if max_count == 0 && self.charged_total() <= self.max_total_charge_usd {
            return 1;
        }
        max_count
    }

    fn charged_count(&self, requested: usize, event_name: &str) -> usize {
        let max_count = self.chargeable_event_capacity(event_name);
        if requested <= max_count {
            return requested;
        }
        if self.charged_total() <= self.max_total_charge_usd {
            return max_count.saturating_add(1);
        }
        0
    }

    fn apply_charges(&mut self, event_name: &str, count: usize) {
        *self
            .charged_counts
            .entry(event_name.to_owned())
            .or_default() += count as f64;
        if self.event_prices.contains_key("apify-default-dataset-item") {
            *self
                .charged_counts
                .entry("apify-default-dataset-item".to_owned())
                .or_default() += count as f64;
        }
    }

    fn event_charge_limit_reached(&self, event_name: &str) -> bool {
        self.chargeable_event_capacity(event_name) == 0
            || (self.event_prices.contains_key("apify-default-dataset-item")
                && self.chargeable_event_capacity("apify-default-dataset-item") == 0)
    }

    fn is_configured_event(&self, event_name: &str) -> bool {
        self.event_prices.contains_key(event_name)
    }
}

fn output_rows(challenges: &[Value], request: &SearchRequest) -> Result<Vec<Value>> {
    challenges
        .iter()
        .map(|challenge| normalized_challenge(challenge, request))
        .collect()
}

pub async fn run_actor(client: &ActorClient) -> Result<Value> {
    let run = client.get_run_pricing().await?;
    let mut budget = ChargeBudget::from_run(&run)?;

    let input = client
        .get_input()
        .await?
        .ok_or_else(|| anyhow!("At least one TikTok challenge search keyword is required"))?;
    let (requests, warnings) = build_search_requests(&input)?;
    for warning in warnings {
        eprintln!("Warning: {warning}");
    }
    println!(
        "Searching TikTok challenges for: {}",
        format_lookup(&requests)
    );

    let mut results = Vec::new();
    let mut saved_challenges = 0_usize;
    let mut status_message = None;

    for request in &requests {
        if budget.chargeable_event_capacity(CHALLENGE_RESULT_CHARGE_EVENT) == 0 {
            let message = format!(
                "Charge limit reached before fetching TikTok challenge keyword {}.",
                request.keyword
            );
            println!("{message}");
            status_message = Some(message);
            break;
        }

        println!(
            "Searching TikTok challenges for keyword: {}",
            request.keyword
        );
        let response = client.search_challenges(request).await?;
        validate_scrappa_response(&response)?;
        let challenges = extract_challenges(response.get("data"));
        let rows = output_rows(&challenges, request)?;

        let (saved_count, charge_limit_reached) = if rows.is_empty() {
            (0, false)
        } else if !budget.is_pay_per_event {
            client.store_dataset_items(&rows).await?;
            (rows.len(), false)
        } else {
            let item_count = budget.item_count_to_push(rows.len(), CHALLENGE_RESULT_CHARGE_EVENT);
            if item_count == 0 {
                (0, true)
            } else {
                client.store_dataset_items(&rows[..item_count]).await?;
                let charged_count = budget.charged_count(item_count, CHALLENGE_RESULT_CHARGE_EVENT);
                if budget.is_configured_event(CHALLENGE_RESULT_CHARGE_EVENT) {
                    client
                        .charge_event(CHALLENGE_RESULT_CHARGE_EVENT, charged_count)
                        .await?;
                }
                budget.apply_charges(CHALLENGE_RESULT_CHARGE_EVENT, charged_count);
                (
                    charged_count.min(rows.len()),
                    budget.event_charge_limit_reached(CHALLENGE_RESULT_CHARGE_EVENT),
                )
            }
        };

        saved_challenges += saved_count;
        let processed_time = response
            .get("processed_time")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null);
        results.push(json!({
            "request_keyword": request.keyword,
            "challenges_returned": challenges.len(),
            "challenges_saved": saved_count,
            "processed_time": processed_time,
            "charge_limit_reached": charge_limit_reached,
        }));

        println!(
            "Found {} challenge(s); saved {} for keyword: {}",
            challenges.len(),
            saved_count,
            request.keyword
        );

        if charge_limit_reached {
            let message = format!(
                "Charge limit reached after saving {saved_count} of {} TikTok challenge result(s) for keyword {}.",
                challenges.len(),
                request.keyword
            );
            println!("{message}");
            status_message = Some(message);
            break;
        }
    }

    let output = json!({
        "keywords_requested": requests.len(),
        "keywords_completed": results.len(),
        "challenges_extracted": saved_challenges,
        "status_message": status_message,
        "results": results,
    });
    client.set_output(&output).await?;
    println!("TikTok challenge search completed successfully");
    println!("Results summary: {}", output);
    Ok(output)
}

pub fn actor_error_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if message.contains("timed out") {
        format!(
            "{message}. The TikTok challenge search request exceeded the {}s Scrappa API timeout. Try a more specific keyword or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            mpsc::{self, Receiver},
            Arc, Mutex,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    struct MockResponse {
        status: u16,
        body: String,
    }

    impl MockResponse {
        fn json(status: u16, body: Value) -> Self {
            Self {
                status,
                body: body.to_string(),
            }
        }
    }

    struct MockServer {
        base_url: Url,
        requests: Receiver<String>,
        stopped: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(handler: impl Fn(&str) -> MockResponse + Send + 'static) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (request_sender, requests) = mpsc::channel();
            let stopped = Arc::new(AtomicBool::new(false));
            let thread_stopped = Arc::clone(&stopped);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(15);
                while !thread_stopped.load(Ordering::Relaxed) && Instant::now() < deadline {
                    let (mut stream, _) = match listener.accept() {
                        Ok(connection) => connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                            continue;
                        }
                        Err(_) => break,
                    };
                    let request = match read_request(&mut stream) {
                        Ok(request) => request,
                        Err(_) => break,
                    };
                    if request_sender.send(request.clone()).is_err() {
                        break;
                    }
                    let response = handler(&request);
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        404 => "Not Found",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
                        _ => "Mock Response",
                    };
                    let reply = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    if stream.write_all(reply.as_bytes()).is_err() {
                        break;
                    }
                }
            });
            Self {
                base_url: Url::parse(&format!("http://{address}")).unwrap(),
                requests,
                stopped,
                thread: Some(thread),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.try_iter().collect()
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.stopped.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        let mut content_length = None;
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                if content_length.is_none() {
                    let headers = String::from_utf8_lossy(&bytes[..header_end]);
                    content_length = Some(
                        headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0),
                    );
                }
                if bytes.len() >= header_end + 4 + content_length.unwrap_or(0) {
                    break;
                }
            }
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn test_client(server: &MockServer) -> ActorClient {
        ActorClient::new(ActorConfig {
            apify_api_base_url: server.base_url.clone(),
            scrappa_api_base_url: Url::parse(&format!(
                "{}/api",
                server.base_url.as_str().trim_end_matches('/')
            ))
            .unwrap(),
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        })
        .unwrap()
    }

    fn request_parts(request: &str) -> (&str, &str, &str, &str) {
        let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
        let mut first_line = headers
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace();
        (
            first_line.next().unwrap_or_default(),
            first_line.next().unwrap_or_default(),
            headers,
            body,
        )
    }

    fn has_header(request: &str, name: &str, expected: &str) -> bool {
        let (_, _, headers, _) = request_parts(request);
        headers.lines().any(|line| {
            let Some((header, value)) = line.split_once(':') else {
                return false;
            };
            header.eq_ignore_ascii_case(name) && value.trim() == expected
        })
    }

    fn ppe_run(max_total: f64) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "challenge-result": { "eventPriceUsd": 0.00025 },
                        "apify-actor-start": { "eventPriceUsd": 0.0001 }
                    }}
                },
                "options": { "maxTotalChargeUsd": max_total },
                "chargedEventCounts": { "apify-actor-start": 1 }
            }
        })
    }

    #[test]
    fn normalizes_batch_legacy_deduplicated_and_prefilled_input() {
        let input = json!({
            "keywords": [" cosplay ", "fitness", "cosplay", 12, null],
            "keyword": "ignored",
            "count": 10
        });
        let (requests, warnings) = build_search_requests(&input).unwrap();
        assert_eq!(
            requests,
            vec![
                SearchRequest {
                    keyword: "cosplay".to_owned(),
                    count: Some(10)
                },
                SearchRequest {
                    keyword: "fitness".to_owned(),
                    count: Some(10)
                },
            ]
        );
        assert_eq!(
            warnings,
            ["keywords entries must be strings, got number. Omitting entry."]
        );

        let (legacy, _) =
            build_search_requests(&json!({ "keywords": [], "keyword": "  tea   trends " }))
                .unwrap();
        assert_eq!(legacy[0].keyword, "tea trends");
        let (string_value, _) =
            build_search_requests(&json!({ "keywords": " skincare " })).unwrap();
        assert_eq!(string_value[0].keyword, "skincare");
        let (fallback, _) = build_search_requests(&json!({
            "keywords": [" \n "],
            "keyword": "tea",
        }))
        .unwrap();
        assert_eq!(fallback[0].keyword, "tea");
        let (trimmed, _) = build_search_requests(&json!({
            "keywords": [format!("{}x", " ".repeat(300))],
        }))
        .unwrap();
        assert_eq!(trimmed[0].keyword, "x");

        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["keywords"]["prefill"],
            json!(["cosplay", "fitness"])
        );
        assert_eq!(schema["properties"]["count"]["default"], 10);
    }

    #[test]
    fn rejects_missing_invalid_and_oversized_keywords_and_defaults_invalid_count() {
        assert!(build_search_requests(&json!({}))
            .unwrap_err()
            .to_string()
            .contains("At least one"));
        assert!(
            build_search_requests(&json!({ "keywords": ["cosplay\nfitness"] }))
                .unwrap_err()
                .to_string()
                .contains("cannot contain tabs")
        );
        assert!(
            build_search_requests(&json!({ "keyword": "x".repeat(256) }))
                .unwrap_err()
                .to_string()
                .contains("255 characters")
        );
        assert!(
            build_search_requests(&json!({ "keyword": "😀".repeat(128) }))
                .unwrap_err()
                .to_string()
                .contains("255 characters")
        );
        let (requests, warnings) =
            build_search_requests(&json!({ "keyword": "trend", "count": 0 })).unwrap();
        assert_eq!(requests[0].count, None);
        assert!(warnings[0].contains("integer between 1 and 50"));
    }

    #[test]
    fn extracts_challenge_shapes_and_preserves_raw_fields_with_normalized_columns() {
        let challenge = json!({
            "id": 123,
            "cha_name": " cosplay ",
            "desc": "raw description",
            "stats": { "view_count": 100, "video_count": 20, "user_count": 5 },
            "extra": "kept"
        });
        for field in [
            "challenges",
            "challenge_list",
            "challengeList",
            "items",
            "results",
        ] {
            let data = json!({ (field): [challenge.clone()] });
            let extracted = extract_challenges(Some(&data));
            assert_eq!(extracted, [challenge.clone()]);
        }
        assert_eq!(
            extract_challenges(Some(&json!([challenge.clone()]))),
            [challenge.clone()]
        );
        assert!(extract_challenges(Some(&json!({}))).is_empty());
        assert!(extract_challenges(None).is_empty());

        let request = SearchRequest {
            keyword: "cosplay".to_owned(),
            count: Some(10),
        };
        let normalized = normalized_challenge(&challenge, &request).unwrap();
        assert_eq!(normalized["challenge_id"], "123");
        assert_eq!(normalized["challenge_name"], "cosplay");
        assert_eq!(normalized["description"], "raw description");
        assert_eq!(normalized["view_count"], 100);
        assert_eq!(normalized["request_count"], 10);
        assert_eq!(normalized["extra"], "kept");
    }

    #[test]
    fn preserves_safe_id_and_name_fallback_rules_and_api_error_code() {
        assert_eq!(
            challenge_id(&json!({ "challenge_id": "  ", "id": 5 })),
            Value::Null
        );
        assert_eq!(
            challenge_id(&json!({ "id": 9_007_199_254_740_992_u64 })),
            Value::Null
        );
        assert_eq!(
            challenge_name(&json!({ "challenge_name": " ", "title": "ignored" })),
            Value::Null
        );
        assert!(
            validate_scrappa_response(&json!({ "code": 7, "msg": "bad request" }))
                .unwrap_err()
                .to_string()
                .contains("code 7: bad request")
        );
        assert!(validate_scrappa_response(&json!({ "code": null }))
            .unwrap_err()
            .to_string()
            .contains("code null: Unknown error"));
        assert!(validate_scrappa_response(&json!({ "code": 0 })).is_ok());
    }

    #[test]
    fn respects_custom_event_budget_and_default_dataset_event_costs() {
        let mut run = ppe_run(0.0012);
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            ["apify-default-dataset-item"] = json!({ "eventPriceUsd": 0.0001 });
        let mut budget = ChargeBudget::from_run(&run).unwrap();
        assert_eq!(
            budget.chargeable_event_capacity(CHALLENGE_RESULT_CHARGE_EVENT),
            4
        );
        assert_eq!(
            budget.item_count_to_push(5, CHALLENGE_RESULT_CHARGE_EVENT),
            3
        );
        budget.apply_charges(CHALLENGE_RESULT_CHARGE_EVENT, 3);
        assert!(budget.event_charge_limit_reached(CHALLENGE_RESULT_CHARGE_EVENT));
        assert!(budget.is_pay_per_event);
    }

    #[tokio::test]
    async fn runs_batch_requests_stores_normalized_rows_charges_results_and_stops_at_limit() {
        let apify_run = ppe_run(0.00035);
        let captured_dataset = Arc::new(Mutex::new(Vec::<Value>::new()));
        let captured_output = Arc::new(Mutex::new(None::<Value>));
        let captured_charges = Arc::new(Mutex::new(Vec::<Value>::new()));
        let dataset = Arc::clone(&captured_dataset);
        let output = Arc::clone(&captured_output);
        let charges = Arc::clone(&captured_charges);
        let server = MockServer::start(move |request| {
            let (method, path, _, body) = request_parts(request);
            match (method, path.split('?').next().unwrap_or_default()) {
                ("GET", "/v2/actor-runs/test-run") => MockResponse::json(200, apify_run.clone()),
                ("GET", "/v2/key-value-stores/test-store/records/INPUT") => MockResponse::json(
                    200,
                    json!({ "keywords": ["cosplay", "fitness"], "count": 2 }),
                ),
                ("GET", "/api/tiktok/challenges/search") => MockResponse::json(
                    200,
                    json!({
                        "code": 0,
                        "processed_time": 12,
                        "data": { "challenges": [
                            { "id": "1", "cha_name": "cosplay", "desc": "First", "stats": { "view_count": 10 } },
                            { "id": "2", "cha_name": "fitness", "desc": "Second" }
                        ] }
                    }),
                ),
                ("POST", "/v2/datasets/test-dataset/items") => {
                    *dataset.lock().unwrap() = serde_json::from_str(body).unwrap();
                    MockResponse::json(201, json!({}))
                }
                ("POST", "/v2/actor-runs/test-run/charge") => {
                    charges
                        .lock()
                        .unwrap()
                        .push(serde_json::from_str(body).unwrap());
                    MockResponse::json(201, json!({}))
                }
                ("PUT", "/v2/key-value-stores/test-store/records/OUTPUT") => {
                    *output.lock().unwrap() = Some(serde_json::from_str(body).unwrap());
                    MockResponse::json(201, json!({}))
                }
                _ => {
                    eprintln!("Unmatched mock request: {method} {path}");
                    MockResponse::json(404, json!({ "error": "unexpected request" }))
                }
            }
        });
        let client = test_client(&server);

        let result = run_actor(&client).await.unwrap();
        let requests = server.requests();

        assert_eq!(result["keywords_requested"], 2);
        assert_eq!(result["keywords_completed"], 1);
        assert_eq!(result["challenges_extracted"], 1);
        assert_eq!(result["status_message"], "Charge limit reached after saving 1 of 2 TikTok challenge result(s) for keyword cosplay.");
        assert_eq!(result["results"][0]["challenges_returned"], 2);
        assert_eq!(result["results"][0]["challenges_saved"], 1);
        assert_eq!(result["results"][0]["charge_limit_reached"], true);
        assert_eq!(captured_dataset.lock().unwrap().len(), 1);
        assert_eq!(captured_dataset.lock().unwrap()[0]["challenge_id"], "1");
        assert_eq!(
            captured_dataset.lock().unwrap()[0]["request_keyword"],
            "cosplay"
        );
        assert_eq!(
            captured_charges.lock().unwrap().as_slice(),
            [json!({ "eventName": CHALLENGE_RESULT_CHARGE_EVENT, "count": 1 })]
        );
        assert_eq!(captured_output.lock().unwrap().as_ref().unwrap(), &result);

        let upstream = requests
            .iter()
            .find(|request| {
                request_parts(request)
                    .1
                    .starts_with("/api/tiktok/challenges/search")
            })
            .unwrap();
        assert!(request_parts(upstream)
            .1
            .contains("keywords=cosplay&count=2"));
        assert!(has_header(upstream, "x-api-key", "test-scrappa-key"));
        assert!(has_header(upstream, "accept", "application/json"));
        assert!(has_header(
            &requests[0],
            "authorization",
            "Bearer test-token"
        ));
        assert!(!requests
            .iter()
            .any(|request| request_parts(request).1.contains("keywords=fitness")));
    }

    #[tokio::test]
    async fn non_ppe_run_saves_results_without_custom_charge_request() {
        let captured_dataset = Arc::new(Mutex::new(Vec::<Value>::new()));
        let dataset = Arc::clone(&captured_dataset);
        let server = MockServer::start(move |request| {
            let (method, path, _, body) = request_parts(request);
            match (method, path.split('?').next().unwrap_or_default()) {
                ("GET", "/v2/actor-runs/test-run") => MockResponse::json(
                    200,
                    json!({ "data": { "pricingInfo": { "pricingModel": "PRICE_PER_DATASET_ITEM" } } }),
                ),
                ("GET", "/v2/key-value-stores/test-store/records/INPUT") => {
                    MockResponse::json(200, json!({ "keyword": "tea" }))
                }
                ("GET", "/api/tiktok/challenges/search") => MockResponse::json(
                    200,
                    json!({ "data": [{ "challenge_id": "1" }, { "challenge_id": "2" }] }),
                ),
                ("POST", "/v2/datasets/test-dataset/items") => {
                    *dataset.lock().unwrap() = serde_json::from_str(body).unwrap();
                    MockResponse::json(201, json!({}))
                }
                ("PUT", "/v2/key-value-stores/test-store/records/OUTPUT") => {
                    MockResponse::json(201, json!({}))
                }
                _ => {
                    eprintln!("Unmatched mock request: {method} {path}");
                    MockResponse::json(404, json!({ "error": "unexpected request" }))
                }
            }
        });
        let client = test_client(&server);
        let result = run_actor(&client).await.unwrap();
        let requests = server.requests();

        assert_eq!(captured_dataset.lock().unwrap().len(), 2);
        assert_eq!(result["challenges_extracted"], 2);
        assert!(!requests
            .iter()
            .any(|request| request_parts(request).1.ends_with("/charge")));
        let search = requests
            .iter()
            .find(|request| request_parts(request).1.starts_with("/api/"))
            .unwrap();
        assert!(!request_parts(search).1.contains("count="));
    }

    #[tokio::test]
    async fn retries_apify_storage_server_errors_but_not_scrappa_failures() {
        let dataset_attempts = Arc::new(AtomicUsize::new(0));
        let attempts = Arc::clone(&dataset_attempts);
        let server = MockServer::start(move |request| {
            if request_parts(request)
                .1
                .starts_with("/v2/datasets/test-dataset/items")
                && attempts.fetch_add(1, Ordering::SeqCst) == 0
            {
                MockResponse::json(500, json!({ "error": "temporary storage error" }))
            } else {
                MockResponse::json(201, json!({}))
            }
        });
        test_client(&server)
            .store_dataset_items(&[json!({ "id": 1 })])
            .await
            .unwrap();
        assert_eq!(dataset_attempts.load(Ordering::SeqCst), 2);

        let server = MockServer::start(|_| {
            MockResponse::json(
                401,
                json!({ "message": "Invalid key", "errors": { "keywords": ["is invalid"] } }),
            )
        });
        let client = test_client(&server);
        let request = SearchRequest {
            keyword: "cosplay".to_owned(),
            count: Some(3),
        };
        let error = client
            .search_challenges(&request)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("Scrappa API error (401): Invalid key - keywords: is invalid"));
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn formats_scrappa_deadline_failure_like_the_node_actor() {
        let error = anyhow!("Scrappa API request timed out after 60000ms");
        assert_eq!(
            actor_error_message(&error),
            "Scrappa API request timed out after 60000ms. The TikTok challenge search request exceeded the 60s Scrappa API timeout. Try a more specific keyword or run the request again."
        );
    }
}
