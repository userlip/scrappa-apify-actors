use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    process::ExitCode,
    time::Duration,
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{SecondsFormat, Utc};
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Map, Value};
use tokio::time::sleep;
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const INPUT_KEY: &str = "INPUT";
const OUTPUT_KEY: &str = "OUTPUT";
const CHALLENGE_DETAIL_CHARGE_EVENT: &str = "challenge-detail-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const MAX_ENTITIES: usize = 100;
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: usize = 8;

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    key_value_store_id: String,
    input_key: String,
    dataset_id: String,
    actor_run_id: String,
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
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|key| !key.is_empty())
                .unwrap_or_else(|| INPUT_KEY.to_owned()),
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestType {
    ChallengeName,
    ChallengeId,
}

impl RequestType {
    fn as_str(self) -> &'static str {
        match self {
            Self::ChallengeName => "challenge_name",
            Self::ChallengeId => "challenge_id",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChallengeRequest {
    request_type: RequestType,
    value: String,
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

fn js_trim(value: &str) -> &str {
    value.trim_matches(is_js_whitespace)
}

fn normalize_challenge_name(value: &str) -> Result<String> {
    let value = js_trim(value);
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.is_empty() {
        return Ok(String::new());
    }
    if value.chars().count() > 255
        || value.chars().any(|character| {
            is_js_whitespace(character) || matches!(character, '#' | '?' | '/' | '=' | ':')
        })
    {
        bail!("TikTok challenge names must be 1 to 255 characters and cannot contain whitespace or URL delimiter characters");
    }
    Ok(value.to_owned())
}

fn safe_integer(value: &Value) -> Option<i64> {
    let number = value.as_f64()?;
    if !number.is_finite() || number.fract() != 0.0 || number.abs() > 9_007_199_254_740_991.0 {
        return None;
    }
    Some(number as i64)
}

fn normalize_challenge_id(value: &Value) -> Result<String> {
    let id = match value {
        Value::String(value) => js_trim(value).to_owned(),
        Value::Number(_) => safe_integer(value)
            .map(|value| value.to_string())
            .ok_or_else(|| {
                anyhow!("TikTok challenge IDs must be strings of digits or safe integers")
            })?,
        _ => bail!("TikTok challenge IDs must be strings of digits or safe integers"),
    };
    if id.is_empty() {
        return Ok(id);
    }
    if id.len() > 100 || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("TikTok challenge IDs must contain 1 to 100 digits");
    }
    Ok(id)
}

fn input_values<'a>(
    input: &'a Value,
    field: &str,
    plural: bool,
    warnings: &mut Vec<String>,
) -> Vec<&'a Value> {
    let Some(value) = input.get(field).filter(|value| !value.is_null()) else {
        return Vec::new();
    };
    if let Some(values) = value.as_array() {
        return values.iter().collect();
    }
    if plural {
        warnings.push(format!(
            "{field} must be an array. Treating the supplied value as one lookup for API compatibility."
        ));
    }
    vec![value]
}

fn build_requests(input: &Value, warnings: &mut Vec<String>) -> Result<Vec<ChallengeRequest>> {
    let mut names = Vec::new();
    for (field, plural) in [("challenge_names", true), ("challenge_name", false)] {
        for value in input_values(input, field, plural, warnings) {
            match value
                .as_str()
                .ok_or_else(|| anyhow!("TikTok challenge names must be strings"))
                .and_then(normalize_challenge_name)
            {
                Ok(name) if !name.is_empty() => names.push(name),
                Ok(_) => {}
                Err(error) => warnings.push(format!("{field} entry omitted: {error}")),
            }
        }
    }

    let mut ids = Vec::new();
    for (field, plural) in [("challenge_ids", true), ("challenge_id", false)] {
        for value in input_values(input, field, plural, warnings) {
            match normalize_challenge_id(value) {
                Ok(id) if !id.is_empty() => ids.push(id),
                Ok(_) => {}
                Err(error) => warnings.push(format!("{field} entry omitted: {error}")),
            }
        }
    }

    let mut seen_names = BTreeSet::new();
    names.retain(|name| seen_names.insert(name.to_lowercase()));
    let mut seen_ids = BTreeSet::new();
    ids.retain(|id| seen_ids.insert(id.clone()));

    if names.len() + ids.len() == 0 {
        bail!("At least one valid TikTok challenge name or challenge ID is required");
    }
    if names.len() + ids.len() > MAX_ENTITIES {
        bail!("A maximum of {MAX_ENTITIES} combined TikTok challenge names and IDs is allowed per run");
    }

    Ok(names
        .into_iter()
        .map(|value| ChallengeRequest {
            request_type: RequestType::ChallengeName,
            value,
        })
        .chain(ids.into_iter().map(|value| ChallengeRequest {
            request_type: RequestType::ChallengeId,
            value,
        }))
        .collect())
}

fn challenge_url(base_url: &Url, request: &ChallengeRequest) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["tiktok", "challenges", "details"])?;
    let key = match request.request_type {
        RequestType::ChallengeName => "challenge_name",
        RequestType::ChallengeId => "challenge_id",
    };
    url.query_pairs_mut().append_pair(key, &request.value);
    Ok(url)
}

fn first_non_null<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter()
        .find_map(|key| object.get(*key).filter(|value| !value.is_null()))
}

fn text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) => {
            let value = js_trim(value);
            (!value.is_empty()).then(|| value.to_owned())
        }
        value @ Value::Number(_) => safe_integer(value).map(|value| value.to_string()),
        _ => None,
    }
}

fn challenge_name(challenge: &Map<String, Value>) -> Option<String> {
    text(first_non_null(
        challenge,
        &["challenge_name", "cha_name", "name", "title"],
    ))
}

fn challenge_id(challenge: &Map<String, Value>) -> Option<String> {
    text(first_non_null(challenge, &["challenge_id", "id", "cid"]))
}

fn extract_challenge_detail(response: &Value) -> Option<&Map<String, Value>> {
    let data = response.get("data")?.as_object()?;
    for key in ["challenge", "item"] {
        if let Some(challenge) = data.get(key).and_then(Value::as_object) {
            return Some(challenge);
        }
    }
    Some(data)
}

fn normalize_challenge_detail(
    challenge: &Map<String, Value>,
    request: &ChallengeRequest,
    retrieved_at: String,
) -> Value {
    let mut item = challenge.clone();
    let name = challenge_name(challenge);
    let id = challenge_id(challenge);
    let description = text(first_non_null(challenge, &["description", "desc"]));
    let cover = text(first_non_null(challenge, &["cover", "cover_url"]));
    let stats = challenge.get("stats").and_then(Value::as_object);
    let metric = |field: &str| {
        challenge
            .get(field)
            .filter(|value| !value.is_null())
            .or_else(|| stats.and_then(|stats| stats.get(field)))
            .filter(|value| value.is_number())
            .cloned()
            .unwrap_or(Value::Null)
    };

    item.insert(
        "challenge_id".to_owned(),
        id.map(Value::String).unwrap_or(Value::Null),
    );
    item.insert(
        "challenge_name".to_owned(),
        name.map(Value::String).unwrap_or(Value::Null),
    );
    item.insert(
        "description".to_owned(),
        description.map(Value::String).unwrap_or(Value::Null),
    );
    item.insert("user_count".to_owned(), metric("user_count"));
    item.insert("view_count".to_owned(), metric("view_count"));
    item.insert("video_count".to_owned(), metric("video_count"));
    item.insert(
        "cover".to_owned(),
        cover.map(Value::String).unwrap_or(Value::Null),
    );
    item.insert(
        "request_challenge_name".to_owned(),
        if request.request_type == RequestType::ChallengeName {
            json!(request.value)
        } else {
            Value::Null
        },
    );
    item.insert(
        "request_challenge_id".to_owned(),
        if request.request_type == RequestType::ChallengeId {
            json!(request.value)
        } else {
            Value::Null
        },
    );
    item.insert("retrieved_at".to_owned(), Value::String(retrieved_at));
    Value::Object(item)
}

#[derive(Default)]
struct PpeBudget {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: BTreeMap<String, f64>,
    charged_event_counts: BTreeMap<String, u64>,
}

impl PpeBudget {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing = data.get("pricingInfo");
        let is_pay_per_event = pricing
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self::default());
        }

        let event_prices = pricing
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?
            .iter()
            .filter_map(|(name, event)| {
                event
                    .get("eventPriceUsd")
                    .and_then(Value::as_f64)
                    .map(|price| (name.clone(), price))
            })
            .collect::<BTreeMap<_, _>>();
        if event_prices
            .values()
            .any(|price| !price.is_finite() || *price < 0.0)
        {
            bail!("Apify run returned invalid charging values");
        }

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|charge| *charge != 0.0)
            .unwrap_or(f64::INFINITY);
        if !max_total_charge_usd.is_finite() && max_total_charge_usd != f64::INFINITY {
            bail!("Apify run returned invalid charging values");
        }
        if max_total_charge_usd < 0.0 {
            bail!("Apify run returned invalid charging values");
        }

        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .map(|events| {
                events
                    .iter()
                    .map(|(name, count)| {
                        count
                            .as_u64()
                            .map(|count| (name.clone(), count))
                            .ok_or_else(|| anyhow!("Invalid charged event count for {name}"))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()
            })
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        })
    }

    fn total_charged_amount(&self) -> f64 {
        let amount = self
            .charged_event_counts
            .iter()
            .map(|(name, count)| {
                self.event_prices.get(name).copied().unwrap_or(0.0) * *count as f64
            })
            .sum::<f64>();
        (amount * 1_000_000.0).round() / 1_000_000.0
    }

    fn max_charges_by_price(&self, price: f64) -> usize {
        if price == 0.0 || !price.is_finite() {
            return usize::MAX;
        }
        let remaining = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !remaining.is_finite() {
            return if remaining.is_sign_positive() {
                usize::MAX
            } else {
                0
            };
        }
        let rounded = (remaining * 10_000.0).round() / 10_000.0;
        rounded.floor().max(0.0).min(usize::MAX as f64) as usize
    }

    fn event_capacity(&self, event_name: &str) -> usize {
        if !self.is_pay_per_event {
            return usize::MAX;
        }
        self.event_prices
            .get(event_name)
            .copied()
            .map(|price| self.max_charges_by_price(price))
            .unwrap_or(usize::MAX)
    }

    fn can_push_dataset_item(&self) -> bool {
        if !self.is_pay_per_event {
            return true;
        }
        let item_price = self
            .event_prices
            .get(CHALLENGE_DETAIL_CHARGE_EVENT)
            .copied()
            .unwrap_or(0.0)
            + self
                .event_prices
                .get(DEFAULT_DATASET_ITEM_EVENT)
                .copied()
                .unwrap_or(0.0);
        item_price == 0.0
            || self.max_charges_by_price(item_price) > 0
            || self.total_charged_amount() <= self.max_total_charge_usd
    }

    fn prepare_event_charge(&mut self, event_name: &str) -> ChargePlan {
        if !self.is_pay_per_event {
            return ChargePlan::default();
        }
        let charge_count = if self.event_capacity(event_name) > 0
            || self.total_charged_amount() <= self.max_total_charge_usd
        {
            1
        } else {
            0
        };
        if charge_count > 0 {
            *self
                .charged_event_counts
                .entry(event_name.to_owned())
                .or_default() += charge_count;
        }
        ChargePlan {
            charged_count: charge_count,
            event_charge_limit_reached: self.event_capacity(event_name) == 0,
            requires_api_charge: charge_count > 0
                && !event_name.starts_with("apify-")
                && self.event_prices.contains_key(event_name),
        }
    }
}

#[derive(Default)]
struct ChargePlan {
    charged_count: u64,
    event_charge_limit_reached: bool,
    requires_api_charge: bool,
}

struct ApifyClient {
    http: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    fn new(config: &ActorConfig) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: config.apify_api_base_url.clone(),
            token: config.apify_token.clone(),
        })
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL must be a base URL"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(resource.iter().copied()));
        Ok(url)
    }

    fn request(&self, method: reqwest::Method, url: Url) -> RequestBuilder {
        self.http
            .request(method, url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
    }

    async fn send_with_retries<F>(&self, operation: &str, mut build_request: F) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        for attempt in 0..=APIFY_MAX_RETRIES {
            match build_request().send().await {
                Ok(response)
                    if attempt < APIFY_MAX_RETRIES
                        && (response.status() == StatusCode::TOO_MANY_REQUESTS
                            || response.status().is_server_error()) =>
                {
                    sleep(Duration::from_millis(
                        (500_u64 << attempt.min(6)).min(30_000),
                    ))
                    .await;
                }
                Ok(response) => return Ok(response),
                Err(error)
                    if attempt < APIFY_MAX_RETRIES
                        && (error.is_connect() || error.is_timeout() || error.is_request()) =>
                {
                    sleep(Duration::from_millis(
                        (500_u64 << attempt.min(6)).min(30_000),
                    ))
                    .await;
                }
                Err(error) => return Err(error).with_context(|| format!("{operation} failed")),
            }
        }
        unreachable!("the retry loop returns after its final attempt")
    }

    async fn successful_response(&self, response: Response, operation: &str) -> Result<Response> {
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let detail = if body.is_empty() {
            format!("HTTP {}", status.as_u16())
        } else {
            body
        };
        bail!(
            "Apify API error ({}) while trying to {operation}: {detail}",
            status.as_u16()
        );
    }

    async fn get_input(&self, config: &ActorConfig) -> Result<Option<Value>> {
        let url = self.resource_url(&[
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            &config.input_key,
        ])?;
        let response = self
            .send_with_retries("Apify INPUT request", || {
                self.request(reqwest::Method::GET, url.clone())
            })
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        self.successful_response(response, "INPUT request")
            .await?
            .json::<Value>()
            .await
            .context("Apify INPUT record was not valid JSON")
            .map(Some)
    }

    async fn run_budget(&self, config: &ActorConfig) -> Result<PpeBudget> {
        let url = self.resource_url(&["actor-runs", &config.actor_run_id])?;
        let response = self
            .send_with_retries("Apify run pricing request", || {
                self.request(reqwest::Method::GET, url.clone())
            })
            .await?;
        let run = self
            .successful_response(response, "run pricing request")
            .await?
            .json::<Value>()
            .await
            .context("Apify run pricing request returned invalid JSON")?;
        PpeBudget::from_run(&run)
    }

    async fn push_dataset_item(&self, config: &ActorConfig, item: &Value) -> Result<()> {
        let url = self.resource_url(&["datasets", &config.dataset_id, "items"])?;
        let response = self
            .send_with_retries("Apify dataset write", || {
                self.request(reqwest::Method::POST, url.clone()).json(item)
            })
            .await?;
        self.successful_response(response, "dataset write").await?;
        Ok(())
    }

    async fn charge_event(
        &self,
        config: &ActorConfig,
        count: u64,
        idempotency_key: &str,
    ) -> Result<()> {
        let url = self.resource_url(&["actor-runs", &config.actor_run_id, "charge"])?;
        let response = self
            .send_with_retries("Apify event charge", || {
                self.request(reqwest::Method::POST, url.clone())
                    .header("idempotency-key", idempotency_key)
                    .json(&json!({"eventName": CHALLENGE_DETAIL_CHARGE_EVENT, "count": count}))
            })
            .await?;
        self.successful_response(response, "event charge").await?;
        Ok(())
    }

    async fn set_output(&self, config: &ActorConfig, output: &Value) -> Result<()> {
        let url = self.resource_url(&[
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            OUTPUT_KEY,
        ])?;
        let response = self
            .send_with_retries("Apify OUTPUT write", || {
                self.request(reqwest::Method::PUT, url.clone()).json(output)
            })
            .await?;
        self.successful_response(response, "OUTPUT write").await?;
        Ok(())
    }
}

struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
}

impl ScrappaClient {
    fn new(config: &ActorConfig) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(SCRAPPA_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Scrappa HTTP client")?,
            base_url: config.scrappa_api_base_url.clone(),
            api_key: config.scrappa_api_key.clone(),
        })
    }

    async fn fetch(&self, request: &ChallengeRequest) -> Result<Value> {
        let url = challenge_url(&self.base_url, request)?;
        let response = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
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
            let fallback = status.canonical_reason().unwrap_or("Unknown status");
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Scrappa API error ({}): {}",
                status.as_u16(),
                scrappa_error_message(&body, fallback, status.as_u16())
            ));
        }
        response
            .json()
            .await
            .context("Scrappa API returned invalid JSON")
    }
}

fn scrappa_error_message(body: &str, fallback: &str, status: u16) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }
    if let Ok(error) = serde_json::from_str::<Value>(body) {
        let mut message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_owned();
        if let Some(errors) = error.get("errors").and_then(Value::as_object) {
            let detail = errors
                .iter()
                .map(|(field, messages)| {
                    let messages = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();
                    format!("{field}: {messages}")
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !detail.is_empty() {
                message.push_str(" - ");
                message.push_str(&detail);
            }
        }
        return message;
    }
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        format!("HTTP {status}")
    } else {
        compact.chars().take(500).collect()
    }
}

fn challenge_error(response: &Value) -> Option<String> {
    let code = response.get("code")?;
    if code.as_f64() == Some(0.0) {
        return None;
    }
    let code = serde_json::to_string(code).unwrap_or_else(|_| "null".to_owned());
    let message = response
        .get("msg")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or_else(|| "Unknown error".to_owned());
    Some(format!(
        "Scrappa TikTok Challenge Details API returned code {code}: {message}"
    ))
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn request_outcome(
    request: &ChallengeRequest,
    status: &str,
    error: Option<String>,
    canonical_id: Option<String>,
) -> Value {
    let mut outcome = json!({
        "request_type": request.request_type.as_str(),
        "request_value": request.value,
        "status": status,
    });
    if let Some(error) = error {
        outcome["error"] = Value::String(error);
    }
    if let Some(canonical_id) = canonical_id {
        outcome["canonical_challenge_id"] = Value::String(canonical_id);
    }
    outcome
}

fn add_not_attempted_outcomes(
    requests: &[ChallengeRequest],
    start_index: usize,
    outcomes: &mut Vec<Value>,
    error: &str,
) {
    outcomes.extend(
        requests[start_index..]
            .iter()
            .map(|request| request_outcome(request, "not_attempted", Some(error.to_owned()), None)),
    );
}

fn result_error(challenge: &Map<String, Value>, request: &ChallengeRequest) -> Option<String> {
    let canonical_id = challenge_id(challenge);
    if canonical_id.is_none() {
        return Some(
            "Scrappa returned a challenge detail without a canonical challenge ID".to_owned(),
        );
    }
    match request.request_type {
        RequestType::ChallengeName => {
            let returned_name = challenge_name(challenge);
            if returned_name
                .as_deref()
                .is_none_or(|name| !name.eq_ignore_ascii_case(&request.value))
            {
                let returned = returned_name.map(Value::String).unwrap_or(Value::Null);
                Some(format!(
                    "Scrappa returned challenge {} but {} was requested",
                    returned,
                    json!(request.value)
                ))
            } else {
                None
            }
        }
        RequestType::ChallengeId if canonical_id.as_deref() != Some(&request.value) => {
            Some(format!(
                "Scrappa returned challenge ID {} but {} was requested",
                json!(canonical_id.unwrap()),
                json!(request.value)
            ))
        }
        RequestType::ChallengeId => None,
    }
}

async fn save_challenge_detail(
    apify: &ApifyClient,
    config: &ActorConfig,
    budget: &mut PpeBudget,
    request: &ChallengeRequest,
    item: &Value,
    canonical_id: &str,
) -> Result<(u64, bool)> {
    if !budget.can_push_dataset_item() {
        return Ok((0, true));
    }
    apify.push_dataset_item(config, item).await?;
    if !budget.is_pay_per_event {
        return Ok((1, false));
    }

    let custom_charge = budget.prepare_event_charge(CHALLENGE_DETAIL_CHARGE_EVENT);
    let dataset_charge = budget.prepare_event_charge(DEFAULT_DATASET_ITEM_EVENT);
    if custom_charge.requires_api_charge {
        let idempotency_key = format!(
            "{}:{}:{}",
            config.actor_run_id,
            request.request_type.as_str(),
            canonical_id
        );
        apify
            .charge_event(config, custom_charge.charged_count, &idempotency_key)
            .await?;
    } else if custom_charge.charged_count > 0
        && !budget
            .event_prices
            .contains_key(CHALLENGE_DETAIL_CHARGE_EVENT)
    {
        eprintln!("Warning: attempt to charge unconfigured event '{CHALLENGE_DETAIL_CHARGE_EVENT}' was ignored");
    }

    Ok((
        custom_charge.charged_count + dataset_charge.charged_count,
        custom_charge.event_charge_limit_reached || dataset_charge.event_charge_limit_reached,
    ))
}

async fn run_batch(
    requests: &[ChallengeRequest],
    scrappa: &ScrappaClient,
    apify: &ApifyClient,
    config: &ActorConfig,
    budget: &mut PpeBudget,
) -> Result<Value> {
    let mut outcomes = Vec::new();
    let mut attempted = 0;
    let mut saved = 0;
    let mut charge_limit_reached = false;
    let mut status_message: Option<String> = None;
    let mut resolved_challenge_ids = BTreeSet::new();

    for (index, request) in requests.iter().enumerate() {
        if budget.event_capacity(CHALLENGE_DETAIL_CHARGE_EVENT) == 0 {
            charge_limit_reached = true;
            status_message = Some(
                "Charge limit reached before fetching another TikTok challenge detail.".to_owned(),
            );
            add_not_attempted_outcomes(requests, index, &mut outcomes, "Charge limit reached");
            break;
        }

        attempted += 1;
        let outcome = async {
            let response = scrappa.fetch(request).await?;
            if let Some(error) = challenge_error(&response) {
                bail!("{error}");
            }
            let challenge = extract_challenge_detail(&response)
                .ok_or_else(|| anyhow!("Scrappa returned no challenge detail record"))?;
            if let Some(error) = result_error(challenge, request) {
                bail!("{error}");
            }
            let canonical_id = challenge_id(challenge).expect("result validation requires a canonical ID");
            if resolved_challenge_ids.contains(&canonical_id) {
                return Ok((
                    request_outcome(
                        request,
                        "duplicate",
                        Some(format!(
                            "Duplicate canonical challenge ID {canonical_id}; result was not saved or charged"
                        )),
                        Some(canonical_id),
                    ),
                    false,
                    false,
                ));
            }

            let item = normalize_challenge_detail(
                challenge,
                request,
                Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            );
            let (charged_count, limit_reached) =
                save_challenge_detail(apify, config, budget, request, &item, &canonical_id).await?;
            if charged_count == 0 {
                return Ok((
                    request_outcome(request, "failed", Some("Apify did not save a chargeable challenge detail result".to_owned()), None),
                    false,
                    limit_reached,
                ));
            }
            resolved_challenge_ids.insert(canonical_id);
            Ok((request_outcome(request, "saved", None, None), true, limit_reached))
        }
        .await;

        match outcome {
            Ok((outcome, was_saved, limit_reached)) => {
                if was_saved {
                    saved += 1;
                }
                outcomes.push(outcome);
                if limit_reached {
                    charge_limit_reached = true;
                    status_message = Some(if was_saved {
                        "Charge limit reached after saving a TikTok challenge detail.".to_owned()
                    } else {
                        "Charge limit reached while saving a TikTok challenge detail.".to_owned()
                    });
                    add_not_attempted_outcomes(
                        requests,
                        index + 1,
                        &mut outcomes,
                        "Charge limit reached",
                    );
                    break;
                }
            }
            Err(error) => outcomes.push(request_outcome(
                request,
                "failed",
                Some(error.to_string()),
                None,
            )),
        }
    }

    let failed = outcomes
        .iter()
        .filter(|outcome| outcome["status"] == "failed")
        .count();
    Ok(json!({
        "requested": requests.len(),
        "attempted": attempted,
        "succeeded": saved,
        "failed": failed,
        "saved": saved,
        "charge_limit_reached": charge_limit_reached,
        "status_message": status_message,
        "outcomes": outcomes,
    }))
}

async fn run_actor() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify = ApifyClient::new(&config)?;
    let input = apify
        .get_input(&config)
        .await?
        .ok_or_else(|| anyhow!("At least one TikTok challenge name or challenge ID is required"))?;
    let mut warnings = Vec::new();
    let requests_result = build_requests(&input, &mut warnings);
    for warning in warnings {
        eprintln!("Warning: {warning}");
    }
    let requests = requests_result?;
    let mut budget = apify.run_budget(&config).await?;
    let scrappa = ScrappaClient::new(&config)?;
    let summary = run_batch(&requests, &scrappa, &apify, &config, &mut budget).await?;
    apify.set_output(&config, &summary).await?;
    println!("TikTok challenge details completed: {summary}");
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run_actor().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicUsize, Ordering},
            mpsc::{self, Receiver},
            Arc,
        },
        thread::{self, JoinHandle},
        time::Instant,
    };

    fn request(request_type: RequestType, value: &str) -> ChallengeRequest {
        ChallengeRequest {
            request_type,
            value: value.to_owned(),
        }
    }

    #[test]
    fn input_keeps_batch_prefill_and_legacy_fields() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["challenge_names"]["prefill"],
            json!(["booktok", "fitness"])
        );
        assert_eq!(
            schema["properties"]["challenge_ids"]["editor"],
            "stringList"
        );
        assert_eq!(
            schema["properties"]["challenge_name"]["editor"],
            "textfield"
        );
        assert_eq!(schema["properties"]["challenge_id"]["editor"], "textfield");
    }

    #[test]
    fn normalizes_and_deduplicates_batch_and_legacy_inputs() {
        let mut warnings = Vec::new();
        let requests = build_requests(
            &json!({
                "challenge_names": [" #BookTok ", "booktok", "fitness"],
                "challenge_ids": ["1622962893630470", "1622962893630470"],
                "challenge_name": "Fitness",
                "challenge_id": 42
            }),
            &mut warnings,
        )
        .unwrap();
        assert_eq!(warnings, Vec::<String>::new());
        assert_eq!(
            requests,
            vec![
                request(RequestType::ChallengeName, "BookTok"),
                request(RequestType::ChallengeName, "fitness"),
                request(RequestType::ChallengeId, "1622962893630470"),
                request(RequestType::ChallengeId, "42"),
            ]
        );
    }

    #[test]
    fn omits_invalid_entries_and_enforces_the_combined_limit() {
        let mut warnings = Vec::new();
        let requests = build_requests(
            &json!({"challenge_names": ["booktok", "bad/name", 42]}),
            &mut warnings,
        )
        .unwrap();
        assert_eq!(
            requests,
            vec![request(RequestType::ChallengeName, "booktok")]
        );
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].contains("omitted"));
        assert!(build_requests(&json!({"challenge_ids": ["not-an-id"]}), &mut Vec::new()).is_err());
        assert!(build_requests(
            &json!({"challenge_names": (0..101).map(|index| format!("tag{index}")).collect::<Vec<_>>() }),
            &mut Vec::new()
        )
        .unwrap_err()
        .to_string()
        .contains("maximum of 100"));
    }

    #[test]
    fn creates_the_exact_scrappa_lookup_route_and_query() {
        let base = Url::parse("https://scrappa.co/api").unwrap();
        let url = challenge_url(&base, &request(RequestType::ChallengeName, "book tok")).unwrap();
        assert_eq!(url.path(), "/api/tiktok/challenges/details");
        assert_eq!(
            url.query_pairs().collect::<Vec<_>>(),
            vec![("challenge_name".into(), "book tok".into())]
        );
        let id_url = challenge_url(&base, &request(RequestType::ChallengeId, "42")).unwrap();
        assert_eq!(
            id_url.query_pairs().collect::<Vec<_>>(),
            vec![("challenge_id".into(), "42".into())]
        );
    }

    #[test]
    fn validates_api_errors_and_canonical_identity_before_saving() {
        let error = challenge_error(&json!({"code": 404, "msg": "Not found"})).unwrap();
        assert!(error.contains("code 404: Not found"));

        let name_request = request(RequestType::ChallengeName, "booktok");
        let mismatch = json!({"id": "1", "challenge_name": "unrelated"});
        assert!(result_error(mismatch.as_object().unwrap(), &name_request)
            .unwrap()
            .contains("unrelated"));

        let id_request = request(RequestType::ChallengeId, "1");
        let mismatch = json!({"id": "2", "challenge_name": "booktok"});
        assert!(result_error(mismatch.as_object().unwrap(), &id_request)
            .unwrap()
            .contains("\"2\""));
        assert!(result_error(&Map::new(), &id_request)
            .unwrap()
            .contains("without a canonical challenge ID"));
    }

    #[test]
    fn normalizes_records_while_preserving_raw_fields_and_null_metrics() {
        let challenge: Map<String, Value> = serde_json::from_value(json!({
            "id": "1622962893630470", "cha_name": "BookTok", "desc": " Books ",
            "stats": {"user_count": 5, "view_count": 10}, "video_count": 3,
            "cover_url": "https://example.test/cover.jpg", "is_commerce": false
        }))
        .unwrap();
        let item = normalize_challenge_detail(
            &challenge,
            &request(RequestType::ChallengeName, "booktok"),
            "2026-07-11T00:00:00.000Z".to_owned(),
        );
        assert_eq!(item["challenge_id"], "1622962893630470");
        assert_eq!(item["challenge_name"], "BookTok");
        assert_eq!(item["description"], "Books");
        assert_eq!(item["user_count"], 5);
        assert_eq!(item["view_count"], 10);
        assert_eq!(item["video_count"], 3);
        assert_eq!(item["cover"], "https://example.test/cover.jpg");
        assert_eq!(item["request_challenge_name"], "booktok");
        assert_eq!(item["request_challenge_id"], Value::Null);
        assert_eq!(item["is_commerce"], false);
        assert_eq!(item["retrieved_at"], "2026-07-11T00:00:00.000Z");

        let sparse = normalize_challenge_detail(
            &json!({"challenge_id": "1", "challenge_name": "one"})
                .as_object()
                .unwrap()
                .clone(),
            &request(RequestType::ChallengeId, "1"),
            "now".to_owned(),
        );
        assert_eq!(sparse["user_count"], Value::Null);
        assert_eq!(sparse["view_count"], Value::Null);
        assert_eq!(sparse["video_count"], Value::Null);
        assert!(extract_challenge_detail(&json!({"data": {"challenge": {"id": "1"}}})).is_some());
        assert!(extract_challenge_detail(&json!({ "data": null })).is_none());
    }

    fn ppe_run(max_total: f64, item_price: f64, event_price: f64, charged: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": item_price},
                    "challenge-detail-result": {"eventPriceUsd": event_price}
                }}},
                "options": {"maxTotalChargeUsd": max_total},
                "chargedEventCounts": charged
            }
        })
    }

    #[test]
    fn ppe_budget_counts_prior_events_caps_lookups_and_charges_each_saved_row() {
        let mut budget = PpeBudget::from_run(&ppe_run(
            0.001,
            0.0001,
            0.00025,
            json!({"challenge-detail-result": 2}),
        ))
        .unwrap();
        assert_eq!(budget.event_capacity(CHALLENGE_DETAIL_CHARGE_EVENT), 2);
        assert!(budget.can_push_dataset_item());

        let custom = budget.prepare_event_charge(CHALLENGE_DETAIL_CHARGE_EVENT);
        let dataset = budget.prepare_event_charge(DEFAULT_DATASET_ITEM_EVENT);
        assert_eq!(custom.charged_count, 1);
        assert!(custom.requires_api_charge);
        assert_eq!(dataset.charged_count, 1);
        assert!(!dataset.requires_api_charge);
        assert_eq!(budget.event_capacity(CHALLENGE_DETAIL_CHARGE_EVENT), 0);
        assert_eq!(
            budget.charged_event_counts[CHALLENGE_DETAIL_CHARGE_EVENT],
            3
        );
        assert_eq!(budget.charged_event_counts[DEFAULT_DATASET_ITEM_EVENT], 1);
    }

    #[test]
    fn non_ppe_and_unconfigured_events_keep_dataset_output_unlimited() {
        let non_ppe = PpeBudget::from_run(
            &json!({"data": {"pricingInfo": {"pricingModel": "PAY_PER_RESULT"}}}),
        )
        .unwrap();
        assert!(!non_ppe.is_pay_per_event);
        assert_eq!(
            non_ppe.event_capacity(CHALLENGE_DETAIL_CHARGE_EVENT),
            usize::MAX
        );
        assert!(non_ppe.can_push_dataset_item());

        let mut unconfigured = PpeBudget::from_run(&json!({
            "data": {"pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {}}},
            "options": {"maxTotalChargeUsd": 0.01}, "chargedEventCounts": {}}
        }))
        .unwrap();
        assert_eq!(
            unconfigured.event_capacity(CHALLENGE_DETAIL_CHARGE_EVENT),
            usize::MAX
        );
        let charge = unconfigured.prepare_event_charge(CHALLENGE_DETAIL_CHARGE_EVENT);
        assert_eq!(charge.charged_count, 1);
        assert!(!charge.requires_api_charge);
    }

    #[test]
    fn scrappa_error_body_matches_message_and_validation_format() {
        assert_eq!(
            scrappa_error_message(
                r#"{"message":"Invalid input","errors":{"challenge_name":["required"]}}"#,
                "Bad Request",
                400
            ),
            "Invalid input - challenge_name: required"
        );
        assert_eq!(
            scrappa_error_message(" upstream  unavailable ", "Bad Gateway", 502),
            "upstream unavailable"
        );
        assert_eq!(scrappa_error_message("", "Bad Gateway", 502), "Bad Gateway");
    }

    struct MockRequest {
        method: String,
        path: String,
        headers: BTreeMap<String, String>,
        body: String,
    }

    struct MockServer {
        base_url: Url,
        requests: Receiver<MockRequest>,
        thread: JoinHandle<()>,
    }

    impl MockServer {
        fn start<F>(request_count: usize, handler: F) -> Self
        where
            F: Fn(&MockRequest) -> (u16, String) + Send + 'static,
        {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                for _ in 0..request_count {
                    let (mut stream, _) = loop {
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                assert!(
                                    Instant::now() < deadline,
                                    "mock server timed out waiting for requests"
                                );
                                thread::sleep(Duration::from_millis(10));
                            }
                            Err(error) => panic!("mock server accept failed: {error}"),
                        }
                    };
                    stream
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .unwrap();
                    let request = read_mock_request(&mut stream);
                    let (status, body) = handler(&request);
                    sender.send(request).unwrap();
                    let reason = match status {
                        200 => "OK",
                        201 => "Created",
                        204 => "No Content",
                        400 => "Bad Request",
                        404 => "Not Found",
                        500 => "Internal Server Error",
                        503 => "Service Unavailable",
                        _ => "Mock Response",
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();
                }
            });
            Self {
                base_url: Url::parse(&format!("http://{address}")).unwrap(),
                requests,
                thread,
            }
        }

        fn finish(self) -> Vec<MockRequest> {
            self.thread.join().unwrap();
            self.requests.into_iter().collect()
        }
    }

    fn read_mock_request(stream: &mut TcpStream) -> MockRequest {
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let read = stream.read(&mut chunk).unwrap();
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
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }

        let header_end = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let header_text = String::from_utf8_lossy(&bytes[..header_end]);
        let mut lines = header_text.lines();
        let mut request_line = lines.next().unwrap().split_whitespace();
        let method = request_line.next().unwrap().to_owned();
        let path = request_line.next().unwrap().to_owned();
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        MockRequest {
            method,
            path,
            headers,
            body: String::from_utf8_lossy(&bytes[header_end + 4..]).into_owned(),
        }
    }

    fn test_config(base_url: Url) -> ActorConfig {
        let mut scrappa_api_base_url = base_url.clone();
        scrappa_api_base_url.set_path("/api");
        ActorConfig {
            apify_api_base_url: base_url,
            scrappa_api_base_url,
            key_value_store_id: "test-store".to_owned(),
            input_key: INPUT_KEY.to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            apify_token: "test-apify-token".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    #[tokio::test]
    async fn batch_continues_after_lookup_errors_and_charges_only_a_saved_unique_result() {
        let server = MockServer::start(5, |request| {
            match request.path.as_str() {
            path if path.contains("challenge_name=booktok") || path.contains("challenge_id=1") => (
                200,
                r#"{"code":0,"data":{"challenge":{"id":"1","challenge_name":"BookTok","desc":"Books"}}}"#.to_owned(),
            ),
            path if path.contains("challenge_name=missing") => (
                200,
                r#"{"code":404,"msg":"Not found"}"#.to_owned(),
            ),
            "/v2/datasets/test-dataset/items" => (201, "{}".to_owned()),
            "/v2/actor-runs/test-run/charge" => (201, "{}".to_owned()),
            _ => (404, "{}".to_owned()),
        }
        });
        let config = test_config(server.base_url.clone());
        let apify = ApifyClient::new(&config).unwrap();
        let scrappa = ScrappaClient::new(&config).unwrap();
        let mut budget = PpeBudget::from_run(&ppe_run(0.001, 0.0, 0.00025, json!({}))).unwrap();
        let requests = vec![
            request(RequestType::ChallengeName, "booktok"),
            request(RequestType::ChallengeName, "missing"),
            request(RequestType::ChallengeId, "1"),
        ];

        let summary = run_batch(&requests, &scrappa, &apify, &config, &mut budget)
            .await
            .unwrap();
        assert_eq!(summary["requested"], 3);
        assert_eq!(summary["attempted"], 3);
        assert_eq!(summary["saved"], 1);
        assert_eq!(summary["failed"], 1);
        assert_eq!(summary["outcomes"][0]["status"], "saved");
        assert_eq!(summary["outcomes"][1]["status"], "failed");
        assert_eq!(summary["outcomes"][2]["status"], "duplicate");

        let captured = server.finish();
        let dataset_writes = captured
            .iter()
            .filter(|request| request.path == "/v2/datasets/test-dataset/items")
            .collect::<Vec<_>>();
        let charges = captured
            .iter()
            .filter(|request| request.path == "/v2/actor-runs/test-run/charge")
            .collect::<Vec<_>>();
        assert_eq!(dataset_writes.len(), 1);
        assert_eq!(charges.len(), 1);
        assert_eq!(
            serde_json::from_str::<Value>(&dataset_writes[0].body).unwrap()
                ["request_challenge_name"],
            "booktok"
        );
        assert_eq!(
            serde_json::from_str::<Value>(&charges[0].body).unwrap(),
            json!({"eventName": CHALLENGE_DETAIL_CHARGE_EVENT, "count": 1})
        );
        assert_eq!(
            charges[0].headers["idempotency-key"],
            "test-run:challenge_name:1"
        );
        for request in captured
            .iter()
            .filter(|request| request.path.starts_with("/api/"))
        {
            assert_eq!(request.headers["x-api-key"], "test-scrappa-key");
        }
        for request in captured
            .iter()
            .filter(|request| request.path.starts_with("/v2/"))
        {
            assert_eq!(request.headers["authorization"], "Bearer test-apify-token");
        }
    }

    #[tokio::test]
    async fn exhausted_result_budget_skips_every_remaining_lookup() {
        let server = MockServer::start(0, |_| panic!("an exhausted budget must not make requests"));
        let config = test_config(server.base_url.clone());
        let apify = ApifyClient::new(&config).unwrap();
        let scrappa = ScrappaClient::new(&config).unwrap();
        let mut budget = PpeBudget::from_run(&ppe_run(
            0.0005,
            0.0,
            0.00025,
            json!({"challenge-detail-result": 2}),
        ))
        .unwrap();
        let requests = vec![
            request(RequestType::ChallengeName, "booktok"),
            request(RequestType::ChallengeId, "1"),
        ];

        let summary = run_batch(&requests, &scrappa, &apify, &config, &mut budget)
            .await
            .unwrap();
        assert_eq!(summary["attempted"], 0);
        assert_eq!(summary["charge_limit_reached"], true);
        assert_eq!(
            summary["status_message"],
            "Charge limit reached before fetching another TikTok challenge detail."
        );
        assert_eq!(summary["outcomes"][0]["status"], "not_attempted");
        assert_eq!(summary["outcomes"][1]["status"], "not_attempted");
        assert!(server.finish().is_empty());
    }

    #[tokio::test]
    async fn apify_requests_retry_transient_server_errors() {
        let attempt_count = Arc::new(AtomicUsize::new(0));
        let attempts = Arc::clone(&attempt_count);
        let server = MockServer::start(2, move |request| {
            if request.method == "GET"
                && request.path == "/v2/key-value-stores/test-store/records/INPUT"
            {
                if request.headers.get("authorization").map(String::as_str)
                    != Some("Bearer test-apify-token")
                {
                    return (400, "{}".to_owned());
                }
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    return (503, r#"{"error":"temporary"}"#.to_owned());
                }
                return (200, r#"{"challenge_names":["booktok"]}"#.to_owned());
            }
            (404, "{}".to_owned())
        });
        let config = test_config(server.base_url.clone());
        let apify = ApifyClient::new(&config).unwrap();
        let input = apify.get_input(&config).await.unwrap().unwrap();
        assert_eq!(input["challenge_names"][0], "booktok");
        let captured = server.finish();
        assert_eq!(captured.len(), 2);
        assert!(captured.iter().all(|request| request.method == "GET"));
    }
}
