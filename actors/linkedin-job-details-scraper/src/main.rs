use std::{collections::HashMap, env, process::ExitCode, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Number, Value};
use url::Url;

const JOB_URL_ERROR: &str =
    "Invalid LinkedIn job URL. Expected format: https://www.linkedin.com/jobs/view/job-id";
const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const STATUS_MESSAGE_TIMEOUT: Duration = Duration::from_secs(1);
const APIFY_MAX_RETRIES: usize = 2;
const JOB_RESULT_CHARGE_EVENT: &str = "job-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

#[derive(Debug, Clone, PartialEq, Eq)]
struct UrlRequest {
    input_url: String,
    normalized_url: Option<String>,
    validation_error: Option<String>,
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

fn is_recoverable_job_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| error.status == 404)
}

struct ApifyConfig {
    api_base: String,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
        })
    }
}

struct Config {
    apify: ApifyConfig,
    scrappa_api_base: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let apify = ApifyConfig::from_env()?;
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify,
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
        .filter(|value| !value.is_empty())
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
    fn new(http: Client, config: &ApifyConfig) -> Self {
        Self {
            http,
            base_url: config.api_base.clone(),
            token: config.token.clone(),
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
            let response = match self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let Some(delay) = apify_transport_retry_delay(retry_count) else {
                        return Err(error).context("Failed to retrieve actor input from Apify API");
                    };
                    tokio::time::sleep(delay).await;
                    retry_count += 1;
                    continue;
                }
            };

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

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let mut retry_count = 0;
        loop {
            let response = match self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let Some(delay) = apify_transport_retry_delay(retry_count) else {
                        return Err(error).context("Apify run pricing request failed");
                    };
                    tokio::time::sleep(delay).await;
                    retry_count += 1;
                    continue;
                }
            };

            if let Some(delay) = apify_retry_delay("GET", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            return require_apify_success(response, "run pricing request")
                .await?
                .json::<Value>()
                .await
                .context("Apify run pricing response was not valid JSON");
        }
    }

    async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
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

    async fn charge_event(
        &self,
        event_name: &str,
        count: usize,
        idempotency_key: &str,
    ) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id, "charge"])?;
        let body = json!({ "eventName": event_name, "count": count });
        let mut retry_count = 0;
        loop {
            let response = match self
                .http
                .post(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .header("idempotency-key", idempotency_key)
                .json(&body)
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let Some(delay) = apify_transport_retry_delay(retry_count) else {
                        return Err(error)
                            .with_context(|| format!("Apify {event_name} charge request failed"));
                    };
                    tokio::time::sleep(delay).await;
                    retry_count += 1;
                    continue;
                }
            };

            if let Some(delay) = apify_retry_delay("POST", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            require_apify_success(response, &format!("{event_name} charge")).await?;
            return Ok(());
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
            let response = match self
                .http
                .put(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .json(value)
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    let Some(delay) = apify_transport_retry_delay(retry_count) else {
                        return Err(error)
                            .with_context(|| format!("Failed to write {key} record to Apify API"));
                    };
                    tokio::time::sleep(delay).await;
                    retry_count += 1;
                    continue;
                }
            };

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

    async fn set_terminal_status_message(&self, message: &str) {
        let url = match self.endpoint(&["v2", "actor-runs", &self.actor_run_id]) {
            Ok(url) => url,
            Err(error) => {
                eprintln!("Failed to set terminal Actor status message: {error}");
                return;
            }
        };
        let response = self
            .http
            .put(url)
            .timeout(STATUS_MESSAGE_TIMEOUT)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(&json!({
                "runId": self.actor_run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true,
            }))
            .send()
            .await;

        match response {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                eprintln!(
                    "Failed to set terminal Actor status message ({}): {body}",
                    status.as_u16()
                );
            }
            Err(error) => eprintln!("Failed to set terminal Actor status message: {error}"),
        }
    }
}

fn apify_retry_delay(method: &str, status: StatusCode, retry_count: usize) -> Option<Duration> {
    let retryable_method = matches!(method, "GET" | "PUT" | "POST");
    let transient_status = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
    if !retryable_method || !transient_status || retry_count >= APIFY_MAX_RETRIES {
        return None;
    }

    Some(Duration::from_secs((retry_count + 1) as u64))
}

fn apify_transport_retry_delay(retry_count: usize) -> Option<Duration> {
    (retry_count < APIFY_MAX_RETRIES).then(|| Duration::from_secs((retry_count + 1) as u64))
}

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

#[derive(Debug)]
struct ChargeRecord {
    event_name: String,
    charged_count: usize,
    should_call_api: bool,
}

struct PricingState {
    is_pay_per_event: bool,
    max_total_charge_usd: f64,
    event_prices: HashMap<String, f64>,
    charged_event_counts: HashMap<String, usize>,
}

impl PricingState {
    fn from_run(run: &Value) -> Result<Self> {
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
                charged_event_counts: HashMap::new(),
            });
        }

        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let mut event_prices = HashMap::new();
        for (event_name, event) in events {
            let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) else {
                continue;
            };
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned an invalid price for event {event_name}");
            }
            event_prices.insert(event_name.clone(), price);
        }

        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_event_counts = HashMap::new();
        for (event_name, count) in counts {
            let count = count
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            charged_event_counts.insert(event_name.clone(), count);
        }

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .unwrap_or(f64::INFINITY);
        if max_total_charge_usd.is_nan() || max_total_charge_usd < 0.0 {
            bail!("Apify run returned an invalid spending limit");
        }

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        })
    }

    fn event_price(&self, event_name: &str) -> f64 {
        self.event_prices.get(event_name).copied().unwrap_or(0.0)
    }

    fn total_charged_amount(&self) -> f64 {
        let amount = self
            .charged_event_counts
            .iter()
            .map(|(event_name, count)| self.event_price(event_name) * *count as f64)
            .sum::<f64>();
        if amount.is_finite() {
            (amount * 1_000_000.0).round() / 1_000_000.0
        } else {
            amount
        }
    }

    fn max_charges_for_price(&self, price: f64) -> Option<usize> {
        if price <= 0.0 {
            return None;
        }
        let remaining = (self.max_total_charge_usd - self.total_charged_amount()) / price;
        if !remaining.is_finite() {
            return None;
        }
        let rounded = (remaining * 10_000.0).round() / 10_000.0;
        Some(rounded.floor().max(0.0).min(usize::MAX as f64) as usize)
    }

    fn max_event_charges_within_limit(&self, event_name: &str) -> Option<usize> {
        self.max_charges_for_price(self.event_price(event_name))
    }

    fn item_limit(&self, event_name: Option<&str>) -> usize {
        if !self.is_pay_per_event {
            return usize::MAX;
        }

        let explicit_event_price = event_name.map_or(0.0, |name| self.event_price(name));
        let default_item_price = self.event_price(DEFAULT_DATASET_ITEM_EVENT);
        self.max_charges_for_price(explicit_event_price + default_item_price)
            .unwrap_or(usize::MAX)
    }

    fn should_push_item(&self, event_name: Option<&str>) -> bool {
        let max_charges = self.item_limit(event_name);
        max_charges >= 1
            || (max_charges == 0 && self.total_charged_amount() <= self.max_total_charge_usd)
    }

    fn register_charge(&mut self, event_name: &str, count: usize) -> ChargeRecord {
        let max_charges = self.max_event_charges_within_limit(event_name);
        let total_before_charge = self.total_charged_amount();
        let charged_count = match max_charges {
            None => count,
            Some(max_charges) if count <= max_charges => count,
            Some(max_charges) if total_before_charge <= self.max_total_charge_usd => {
                max_charges.saturating_add(1)
            }
            Some(_) => 0,
        };

        if charged_count > 0 {
            let event_count = self
                .charged_event_counts
                .entry(event_name.to_owned())
                .or_default();
            *event_count = event_count.saturating_add(charged_count);
        }

        ChargeRecord {
            event_name: event_name.to_owned(),
            charged_count,
            should_call_api: charged_count > 0
                && !event_name.starts_with("apify-")
                && self.event_prices.contains_key(event_name),
        }
    }

    fn event_charge_limit_reached(&self, event_name: &str) -> bool {
        self.max_event_charges_within_limit(event_name)
            .is_some_and(|remaining| remaining == 0)
    }
}

struct ScrappaClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    fn new(http: Client, base_url: String, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    async fn get_job(&self, params: &[(String, String)]) -> Result<Value> {
        let url = job_endpoint_url(&self.base_url, params)?;
        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(scrappa_transport_error)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .context("Failed to read Scrappa API error response")?;
            return Err(ScrappaApiError {
                status,
                message: scrappa_error_message(status, &body),
            }
            .into());
        }

        response
            .json::<Value>()
            .await
            .map_err(scrappa_transport_error)
    }
}

fn scrappa_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            REQUEST_TIMEOUT.as_millis()
        )
    } else {
        error.into()
    }
}

fn endpoint_url(base: &str, path: &[&str]) -> Result<Url> {
    let mut url = Url::parse(&format!("{}/", base.trim_end_matches('/')))
        .with_context(|| format!("Invalid API base URL: {base}"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow!("API base URL cannot contain query or fragment: {base}"))?;
        segments.pop_if_empty();
        for segment in path {
            segments.push(segment);
        }
    }
    Ok(url)
}

fn job_endpoint_url(base: &str, params: &[(String, String)]) -> Result<Url> {
    let mut url = endpoint_url(base, &["linkedin", "job"])?;
    url.query_pairs_mut().extend_pairs(params.iter());
    Ok(url)
}

fn scrappa_error_message(status: u16, body: &str) -> String {
    let Ok(error_data) = serde_json::from_str::<Value>(body) else {
        return if body.is_empty() {
            format!("HTTP {status}")
        } else {
            body.to_owned()
        };
    };

    let mut message = error_data
        .get("message")
        .filter(|value| !value.is_null())
        .map(js_string)
        .unwrap_or_else(|| format!("HTTP {status}"));

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
        message.push_str(" - ");
        message.push_str(&details);
    }

    message
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

fn build_job_params(
    normalized_url: &str,
    use_cache: Option<&Value>,
    maximum_cache_age: Option<&Value>,
) -> Vec<(String, String)> {
    let mut params = vec![("url".to_owned(), normalized_url.to_owned())];
    if !use_cache.is_some_and(js_truthy) {
        return params;
    }

    params.push(("use_cache".to_owned(), "1".to_owned()));
    if let Some(age) = maximum_cache_age.and_then(cache_age_string) {
        params.push(("maximum_cache_age".to_owned(), age));
    }
    params
}

fn cache_age_string(value: &Value) -> Option<String> {
    let number = value.as_number()?;
    if let Some(age) = number.as_u64().filter(|age| *age >= 1) {
        return Some(age.to_string());
    }

    let age = number.as_f64()?;
    if !age.is_finite() || age < 1.0 || age.fract() != 0.0 {
        return None;
    }
    Some(format!("{age:.0}"))
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

fn get_input_urls(input: Option<&Value>) -> Result<Vec<UrlRequest>> {
    let mut raw_urls = Vec::new();
    if let Some(url) = input
        .and_then(|input| input.get("url"))
        .and_then(Value::as_str)
    {
        raw_urls.push(url);
    }
    if let Some(urls) = input
        .and_then(|input| input.get("urls"))
        .and_then(Value::as_array)
    {
        for url in urls {
            raw_urls.push(
                url.as_str()
                    .ok_or_else(|| anyhow!("LinkedIn job URLs must be strings"))?,
            );
        }
    }

    let mut seen = HashMap::new();
    let mut requests = Vec::new();
    for raw_url in raw_urls {
        let input_url = raw_url.trim().to_owned();
        if input_url.is_empty() {
            continue;
        }

        match normalize_linkedin_job_url(&input_url) {
            Ok(normalized_url) => {
                if seen.insert(normalized_url.clone(), ()).is_none() {
                    requests.push(UrlRequest {
                        input_url,
                        normalized_url: Some(normalized_url),
                        validation_error: None,
                    });
                }
            }
            Err(validation_error) => {
                let key = format!("invalid:{input_url}");
                if seen.insert(key, ()).is_none() {
                    requests.push(UrlRequest {
                        input_url,
                        normalized_url: None,
                        validation_error: Some(validation_error),
                    });
                }
            }
        }
    }

    Ok(requests)
}

fn normalize_linkedin_job_url(raw_url: &str) -> std::result::Result<String, String> {
    let candidate = raw_url.trim();
    if candidate.is_empty() {
        return Err(JOB_URL_ERROR.to_owned());
    }

    let has_protocol = candidate.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty() && scheme.bytes().all(|byte| byte.is_ascii_alphabetic())
    });
    let with_protocol = if has_protocol {
        candidate.to_owned()
    } else {
        format!("https://{candidate}")
    };
    let parsed = Url::parse(&with_protocol).map_err(|_| "Invalid URL".to_owned())?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(JOB_URL_ERROR.to_owned());
    }
    if !parsed.username().is_empty()
        || parsed
            .password()
            .is_some_and(|password| !password.is_empty())
        || parsed.port().is_some()
    {
        return Err(JOB_URL_ERROR.to_owned());
    }

    let hostname = parsed.host_str().unwrap_or_default();
    if !is_linkedin_hostname(hostname) {
        return Err(JOB_URL_ERROR.to_owned());
    }

    let path = parsed.path();
    let Some(rest) = strip_job_view_prefix(path) else {
        return Err(JOB_URL_ERROR.to_owned());
    };
    let job_id = rest.split('/').next().unwrap_or_default();
    if job_id.is_empty() {
        return Err(JOB_URL_ERROR.to_owned());
    }

    Ok(format!(
        "{}://www.linkedin.com/jobs/view/{job_id}",
        parsed.scheme()
    ))
}

fn strip_job_view_prefix(path: &str) -> Option<&str> {
    let prefix = "/jobs/view/";
    let path_prefix = path.get(..prefix.len())?;
    if !path_prefix.eq_ignore_ascii_case(prefix) {
        return None;
    }
    path.get(prefix.len()..)
}

fn is_linkedin_hostname(hostname: &str) -> bool {
    if hostname == "linkedin.com" {
        return true;
    }
    let Some(prefix) = hostname.strip_suffix(".linkedin.com") else {
        return false;
    };
    prefix == "www"
        || prefix == "m"
        || ((2..=3).contains(&prefix.len()) && prefix.bytes().all(|byte| byte.is_ascii_lowercase()))
}

fn first_present(response: &Map<String, Value>, keys: &[&str]) -> Option<Value> {
    keys.iter()
        .find_map(|key| {
            response.get(*key).filter(|value| {
                !value.is_null() && value.as_str().is_none_or(|value| !value.trim().is_empty())
            })
        })
        .cloned()
}

fn build_success_item(response: Value, input_url: &str, normalized_url: &str) -> Result<Value> {
    let mut fields = response
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("Scrappa job response was not a JSON object"))?;
    let response_url = fields
        .get("url")
        .and_then(Value::as_str)
        .filter(|url| !url.trim().is_empty())
        .unwrap_or(normalized_url)
        .to_owned();

    fields.insert(
        "success".to_owned(),
        fields
            .get("success")
            .filter(|success| !success.is_null())
            .cloned()
            .unwrap_or(Value::Bool(true)),
    );
    for (canonical, aliases) in [
        ("title", &["title", "job_title"][..]),
        ("company", &["company", "company_name"][..]),
        ("posted_date", &["posted_date", "date_posted"][..]),
        ("applicants", &["applicants", "applicant_count"][..]),
        ("apply_url", &["apply_url", "application_url"][..]),
    ] {
        if let Some(value) = first_present(&fields, aliases) {
            fields.insert(canonical.to_owned(), value);
        }
    }
    fields.insert("url".to_owned(), Value::String(response_url));
    fields.insert("input_url".to_owned(), Value::String(input_url.to_owned()));
    fields.insert(
        "normalized_url".to_owned(),
        Value::String(normalized_url.to_owned()),
    );
    Ok(Value::Object(fields))
}

fn build_failure_item(
    error_message: &str,
    api_status: Option<u16>,
    input_url: &str,
    normalized_url: Option<&str>,
) -> Value {
    let mut result = Map::new();
    result.insert("success".to_owned(), Value::Bool(false));
    result.insert("input_url".to_owned(), Value::String(input_url.to_owned()));
    if let Some(normalized_url) = normalized_url {
        result.insert(
            "normalized_url".to_owned(),
            Value::String(normalized_url.to_owned()),
        );
        result.insert("url".to_owned(), Value::String(normalized_url.to_owned()));
    }
    result.insert("error".to_owned(), Value::String(error_message.to_owned()));
    result.insert(
        "error_type".to_owned(),
        Value::String(
            if api_status.is_some() {
                "scrappa_api_error"
            } else {
                "error"
            }
            .to_owned(),
        ),
    );
    result.insert(
        "message".to_owned(),
        Value::String(if api_status == Some(404) {
            "Job not found".to_owned()
        } else {
            error_message.to_owned()
        }),
    );
    if let Some(status) = api_status {
        result.insert(
            "status_code".to_owned(),
            Value::Number(Number::from(status)),
        );
    }
    Value::Object(result)
}

fn is_success(result: &Value) -> bool {
    result.get("success").is_some_and(js_truthy)
}

fn should_charge_result(result: &Value) -> bool {
    result.get("success") == Some(&Value::Bool(true))
}

fn build_output(result: &Value) -> Value {
    let Some(fields) = result.as_object() else {
        return result.clone();
    };
    let mut output = fields.clone();
    for key in ["input_url", "normalized_url", "url", "error", "error_type"] {
        output.remove(key);
    }
    Value::Object(output)
}

struct PushChargedItemsResult {
    saved_count: usize,
    status_message: Option<String>,
}

async fn push_charged_item(
    apify: &ApifyClient,
    pricing: &mut PricingState,
    item: &Value,
    charge_event: bool,
    request_index: usize,
) -> Result<PushChargedItemsResult> {
    if !pricing.is_pay_per_event {
        apify.push_dataset_item(item).await?;
        return Ok(PushChargedItemsResult {
            saved_count: 1,
            status_message: None,
        });
    }

    let event_name = charge_event.then_some(JOB_RESULT_CHARGE_EVENT);
    if !pricing.should_push_item(event_name) {
        return Ok(PushChargedItemsResult {
            saved_count: usize::from(!charge_event),
            status_message: charge_event.then(|| charge_limit_message(0, 1)),
        });
    }

    apify.push_dataset_item(item).await?;

    let mut event_names = Vec::with_capacity(2);
    if let Some(event_name) = event_name {
        event_names.push(event_name.to_owned());
    }
    event_names.push(DEFAULT_DATASET_ITEM_EVENT.to_owned());

    let mut charges = Vec::with_capacity(event_names.len());
    for event_name in &event_names {
        charges.push(pricing.register_charge(event_name, 1));
    }
    for charge in &charges {
        if charge.should_call_api {
            let idempotency_key = format!(
                "{}-{}-{}",
                apify.actor_run_id, charge.event_name, request_index
            );
            apify
                .charge_event(&charge.event_name, charge.charged_count, &idempotency_key)
                .await?;
        }
    }

    if !charge_event {
        return Ok(PushChargedItemsResult {
            saved_count: 1,
            status_message: None,
        });
    }

    let charged_count = charges
        .iter()
        .map(|charge| charge.charged_count)
        .sum::<usize>();
    let event_charge_limit_reached = event_names
        .iter()
        .any(|event_name| pricing.event_charge_limit_reached(event_name));
    if !event_charge_limit_reached {
        return Ok(PushChargedItemsResult {
            saved_count: 1,
            status_message: None,
        });
    }

    let saved_count = charged_count.min(1);
    let status_message = charge_limit_message(saved_count, 1);
    println!(
        "{status_message} {}",
        json!({
            "event": JOB_RESULT_CHARGE_EVENT,
            "charged_count": charged_count,
            "requested_count": 1,
            "saved_count": saved_count,
        })
    );
    Ok(PushChargedItemsResult {
        saved_count,
        status_message: Some(status_message),
    })
}

fn charge_limit_message(saved_count: usize, requested_count: usize) -> String {
    format!(
        "Charge limit reached after saving {saved_count} of {requested_count} LinkedIn job detail results."
    )
}

async fn run(config: &Config, apify: &ApifyClient, http: Client) -> Result<Option<String>> {
    let run = apify.get_run().await?;
    let mut pricing = PricingState::from_run(&run)?;
    let input = apify.get_input().await?;
    let urls = get_input_urls(input.as_ref())?;
    if urls.is_empty() {
        bail!("At least one LinkedIn job URL is required. Provide either url (single URL) or urls (array of URLs).");
    }

    let scrappa = ScrappaClient::new(
        http,
        config.scrappa_api_base.clone(),
        config.scrappa_api_key.clone(),
    );
    let mut first_result: Option<Value> = None;
    let mut succeeded = 0;
    let mut failed = 0;
    let mut status_message: Option<String> = None;
    println!(
        "Scraping {} LinkedIn job URL{}",
        urls.len(),
        if urls.len() == 1 { "" } else { "s" }
    );

    let use_cache = input.as_ref().and_then(|input| input.get("use_cache"));
    let maximum_cache_age = input
        .as_ref()
        .and_then(|input| input.get("maximum_cache_age"));
    for (request_index, request) in urls.iter().enumerate() {
        let result = if let Some(normalized_url) = request.normalized_url.as_deref() {
            println!("Fetching LinkedIn job details: {normalized_url}");
            let params = build_job_params(normalized_url, use_cache, maximum_cache_age);
            match scrappa.get_job(&params).await {
                Ok(response) => build_success_item(response, &request.input_url, normalized_url)?,
                Err(error) if is_recoverable_job_error(&error) => {
                    let api_error = error
                        .downcast_ref::<ScrappaApiError>()
                        .expect("404 Scrappa error must retain its API error type");
                    eprintln!(
                        "Job detail scraping returned a per-item failure for {normalized_url}: {api_error}"
                    );
                    build_failure_item(
                        &api_error.to_string(),
                        Some(api_error.status),
                        &request.input_url,
                        Some(normalized_url),
                    )
                }
                Err(error) => return Err(error),
            }
        } else {
            eprintln!("Invalid LinkedIn job URL: \"{}\"", request.input_url);
            build_failure_item(
                request
                    .validation_error
                    .as_deref()
                    .unwrap_or("Invalid LinkedIn job URL"),
                None,
                &request.input_url,
                None,
            )
        };

        let push_result = push_charged_item(
            apify,
            &mut pricing,
            &result,
            should_charge_result(&result),
            request_index,
        )
        .await?;
        if let Some(message) = push_result.status_message {
            status_message = Some(message);
        }
        first_result.get_or_insert_with(|| result.clone());

        if is_success(&result) {
            succeeded += push_result.saved_count;
            let title = result
                .get("title")
                .filter(|title| !title.is_null())
                .map(js_string)
                .or_else(|| request.normalized_url.clone())
                .unwrap_or_else(|| request.input_url.clone());
            println!("Saved LinkedIn job detail: {title}");
        } else {
            failed += push_result.saved_count;
            let message = result
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if message.is_empty() {
                eprintln!("LinkedIn job detail failed");
            } else {
                eprintln!("LinkedIn job detail failed ({message})");
            }
        }

        if status_message.is_some() {
            break;
        }
    }

    let output = if urls.len() == 1 {
        build_output(first_result.as_ref().expect("a URL produces a result"))
    } else {
        json!({
            "requested": urls.len(),
            "succeeded": succeeded,
            "failed": failed,
        })
    };
    apify.put_record("OUTPUT", &output).await?;

    println!("LinkedIn job detail scraping completed");
    println!(
        "Job detail summary: {}",
        serde_json::to_string_pretty(&json!({
            "requested": urls.len(),
            "succeeded": succeeded,
            "failed": failed,
        }))?
    );

    Ok(status_message)
}

#[tokio::main]
async fn main() -> ExitCode {
    let apify_config = ApifyConfig::from_env().ok();
    let config = Config::from_env();
    let http = match Client::builder().timeout(REQUEST_TIMEOUT).build() {
        Ok(http) => http,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            return ExitCode::FAILURE;
        }
    };

    let config = match config {
        Ok(config) => config,
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            if let Some(apify_config) = apify_config {
                ApifyClient::new(http, &apify_config)
                    .set_terminal_status_message(&message)
                    .await;
            }
            return ExitCode::FAILURE;
        }
    };

    let apify = ApifyClient::new(http.clone(), &config.apify);
    match run(&config, &apify, http).await {
        Ok(status_message) => {
            if let Some(message) = status_message {
                println!("[Status message]: {message}");
                apify.set_terminal_status_message(&message).await;
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            println!("[Status message]: {message}");
            apify.set_terminal_status_message(&message).await;
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
        task::JoinHandle,
    };

    #[test]
    fn normalizes_linkedin_job_urls_and_removes_tracking_data() {
        assert_eq!(
            normalize_linkedin_job_url(
                "linkedin.com/jobs/view/1234567890/?trk=public_jobs_topcard-title"
            )
            .unwrap(),
            "https://www.linkedin.com/jobs/view/1234567890"
        );
        assert_eq!(
            normalize_linkedin_job_url(
                "https://de.linkedin.com/jobs/view/software-engineer-at-example-1234567890/extra"
            )
            .unwrap(),
            "https://www.linkedin.com/jobs/view/software-engineer-at-example-1234567890"
        );
        assert_eq!(
            normalize_linkedin_job_url("http://m.linkedin.com/jobs/view/1234567890?refId=abc")
                .unwrap(),
            "http://www.linkedin.com/jobs/view/1234567890"
        );
    }

    #[test]
    fn rejects_non_job_linkedin_urls_and_unsafe_hosts() {
        for url in [
            "https://www.linkedin.com/in/example",
            "https://example.com/jobs/view/123",
            "ftp://linkedin.com/jobs/view/123",
            "https://user:pass@linkedin.com/jobs/view/123",
            "https://linkedin.com:444/jobs/view/123",
            "   ",
        ] {
            assert_eq!(
                normalize_linkedin_job_url(url),
                Err(JOB_URL_ERROR.to_owned())
            );
        }
        assert_eq!(
            normalize_linkedin_job_url("https://%"),
            Err("Invalid URL".to_owned())
        );
    }

    #[test]
    fn combines_legacy_and_batch_inputs_and_deduplicates_normalized_urls() {
        let input = json!({
            "url": "https://linkedin.com/jobs/view/1234567890",
            "urls": [
                "https://de.linkedin.com/jobs/view/software-engineer-2345678901/?trk=foo",
                "https://www.linkedin.com/jobs/view/1234567890/?refId=abc",
                "https://example.com/jobs/view/123",
                "https://example.com/jobs/view/123"
            ]
        });
        let requests = get_input_urls(Some(&input)).unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(
            requests[0].normalized_url.as_deref(),
            Some("https://www.linkedin.com/jobs/view/1234567890")
        );
        assert_eq!(
            requests[1].input_url,
            "https://de.linkedin.com/jobs/view/software-engineer-2345678901/?trk=foo"
        );
        assert_eq!(requests[2].normalized_url, None);
        assert_eq!(requests[2].validation_error.as_deref(), Some(JOB_URL_ERROR));
    }

    #[test]
    fn rejects_non_string_batch_urls_and_ignores_non_string_legacy_url() {
        assert!(get_input_urls(Some(&json!({ "url": 4 })))
            .unwrap()
            .is_empty());
        assert_eq!(
            get_input_urls(Some(
                &json!({ "urls": ["https://linkedin.com/jobs/view/1", 2] })
            ))
            .unwrap_err()
            .to_string(),
            "LinkedIn job URLs must be strings"
        );
    }

    #[test]
    fn actor_input_schema_keeps_legacy_and_batch_prefills() {
        let schema_path = format!("{}/.actor/input_schema.json", env!("CARGO_MANIFEST_DIR"));
        let schema: Value =
            serde_json::from_str(&std::fs::read_to_string(schema_path).unwrap()).unwrap();
        assert!(schema["properties"]["urls"]["prefill"].as_array().is_some());
        assert!(schema["properties"]["url"]["prefill"].is_string());
        assert_eq!(schema["properties"]["use_cache"]["default"], true);
        assert_eq!(
            schema["properties"]["maximum_cache_age"]["default"],
            2_592_000
        );
    }

    #[test]
    fn builds_cache_params_only_for_valid_enabled_cache_settings() {
        let url = "https://www.linkedin.com/jobs/view/123";
        assert_eq!(
            build_job_params(url, Some(&json!(true)), Some(&json!(3600))),
            vec![
                ("url".to_owned(), url.to_owned()),
                ("use_cache".to_owned(), "1".to_owned()),
                ("maximum_cache_age".to_owned(), "3600".to_owned()),
            ]
        );
        for age in [json!(0), json!(-1), json!(1.5), json!("3600"), Value::Null] {
            assert_eq!(
                build_job_params(url, Some(&json!(true)), Some(&age)),
                vec![
                    ("url".to_owned(), url.to_owned()),
                    ("use_cache".to_owned(), "1".to_owned()),
                ]
            );
        }
        assert_eq!(
            build_job_params(url, Some(&json!(false)), Some(&json!(3600))),
            vec![("url".to_owned(), url.to_owned())]
        );
    }

    #[test]
    fn scrappa_error_messages_keep_api_fields_and_fallback_text() {
        assert_eq!(
            scrappa_error_message(
                422,
                r#"{"message":"Invalid","errors":{"url":["bad","required"]}}"#
            ),
            "Invalid - url: bad, required"
        );
        assert_eq!(scrappa_error_message(503, ""), "HTTP 503");
        assert_eq!(scrappa_error_message(503, "Unavailable"), "Unavailable");
    }

    #[test]
    fn success_items_preserve_response_and_fill_canonical_aliases() {
        let result = build_success_item(
            json!({
                "job_title": "Software Engineer",
                "company_name": "Example Corp",
                "date_posted": "2026-06-01",
                "applicant_count": "23 applicants",
                "application_url": "https://example.com/apply",
                "location": "New York, NY"
            }),
            "linkedin.com/jobs/view/123",
            "https://www.linkedin.com/jobs/view/123",
        )
        .unwrap();
        assert_eq!(result["success"], true);
        assert_eq!(result["title"], "Software Engineer");
        assert_eq!(result["company"], "Example Corp");
        assert_eq!(result["posted_date"], "2026-06-01");
        assert_eq!(result["applicants"], "23 applicants");
        assert_eq!(result["apply_url"], "https://example.com/apply");
        assert_eq!(result["location"], "New York, NY");
        assert_eq!(result["url"], "https://www.linkedin.com/jobs/view/123");
        assert_eq!(result["input_url"], "linkedin.com/jobs/view/123");
    }

    #[test]
    fn success_items_preserve_canonical_values_over_aliases_and_fill_blank_strings() {
        let result = build_success_item(
            json!({
                "title": "Senior Engineer",
                "job_title": "Engineer",
                "company": "  ",
                "company_name": "Example Corp",
                "posted_date": "",
                "date_posted": "2026-06-01",
                "apply_url": null,
                "application_url": "https://example.com/apply",
                "url": " "
            }),
            "input",
            "https://www.linkedin.com/jobs/view/123",
        )
        .unwrap();
        assert_eq!(result["title"], "Senior Engineer");
        assert_eq!(result["company"], "Example Corp");
        assert_eq!(result["posted_date"], "2026-06-01");
        assert_eq!(result["apply_url"], "https://example.com/apply");
        assert_eq!(result["url"], "https://www.linkedin.com/jobs/view/123");
    }

    #[test]
    fn failure_items_keep_scrappa_status_and_single_output_strips_wrapper_fields() {
        let missing = build_failure_item(
            "Scrappa API error (404): Not found",
            Some(404),
            "linkedin.com/jobs/view/missing",
            Some("https://www.linkedin.com/jobs/view/missing"),
        );
        assert_eq!(missing["success"], false);
        assert_eq!(missing["error_type"], "scrappa_api_error");
        assert_eq!(missing["message"], "Job not found");
        assert_eq!(missing["status_code"], 404);
        assert_eq!(
            build_output(&json!({
                "success": true,
                "title": "Software Engineer",
                "url": "https://www.linkedin.com/jobs/view/123",
                "input_url": "input",
                "normalized_url": "normalized",
                "error": "wrapper error",
                "error_type": "wrapper_error"
            })),
            json!({ "success": true, "title": "Software Engineer" })
        );
        assert_eq!(
            build_failure_item(JOB_URL_ERROR, None, "invalid", None),
            json!({
                "success": false,
                "input_url": "invalid",
                "error": JOB_URL_ERROR,
                "error_type": "error",
                "message": JOB_URL_ERROR
            })
        );
    }

    #[test]
    fn only_scrappa_not_found_errors_are_recoverable_and_charge_requires_true() {
        assert!(is_recoverable_job_error(&anyhow!(ScrappaApiError {
            status: 404,
            message: "Not found".to_owned(),
        })));
        assert!(!is_recoverable_job_error(&anyhow!(ScrappaApiError {
            status: 401,
            message: "Unauthorized".to_owned(),
        })));
        assert!(!is_recoverable_job_error(&anyhow!("timeout")));
        assert!(should_charge_result(&json!({ "success": true })));
        assert!(!should_charge_result(&json!({ "success": "true" })));
        assert!(is_success(&json!({ "success": "true" })));
    }

    fn ppe_run(max_total: f64, charged_event_counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "job-result": { "eventPriceUsd": 0.001 },
                            "apify-default-dataset-item": { "eventPriceUsd": 0.0002 }
                        }
                    }
                },
                "chargedEventCounts": charged_event_counts,
                "options": { "maxTotalChargeUsd": max_total }
            }
        })
    }

    #[test]
    fn ppe_budget_includes_custom_and_default_dataset_item_charges() {
        let run = ppe_run(0.0012, json!({}));
        let mut pricing = PricingState::from_run(&run).unwrap();
        assert!(pricing.should_push_item(Some(JOB_RESULT_CHARGE_EVENT)));
        let custom = pricing.register_charge(JOB_RESULT_CHARGE_EVENT, 1);
        let dataset = pricing.register_charge(DEFAULT_DATASET_ITEM_EVENT, 1);
        assert_eq!(custom.charged_count, 1);
        assert_eq!(dataset.charged_count, 1);
        assert!(pricing.event_charge_limit_reached(JOB_RESULT_CHARGE_EVENT));
        assert!(pricing.event_charge_limit_reached(DEFAULT_DATASET_ITEM_EVENT));
    }

    #[test]
    fn ppe_budget_skips_rows_only_after_the_run_is_already_over_the_limit() {
        let exact_limit = ppe_run(
            0.0012,
            json!({
                "job-result": 1,
                "apify-default-dataset-item": 1
            }),
        );
        let pricing = PricingState::from_run(&exact_limit).unwrap();
        assert!(pricing.should_push_item(Some(JOB_RESULT_CHARGE_EVENT)));

        let over_limit = ppe_run(
            0.0011,
            json!({
                "job-result": 1,
                "apify-default-dataset-item": 1
            }),
        );
        let pricing = PricingState::from_run(&over_limit).unwrap();
        assert!(!pricing.should_push_item(Some(JOB_RESULT_CHARGE_EVENT)));
    }

    #[test]
    fn free_runs_do_not_limit_rows_or_charge_named_events() {
        let run = json!({ "data": { "pricingInfo": { "pricingModel": "FREE" } } });
        let pricing = PricingState::from_run(&run).unwrap();
        assert!(!pricing.is_pay_per_event);
        assert!(pricing.should_push_item(Some(JOB_RESULT_CHARGE_EVENT)));
    }

    #[test]
    fn apify_retries_only_bounded_transient_responses() {
        assert_eq!(
            apify_retry_delay("GET", StatusCode::TOO_MANY_REQUESTS, 0),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            apify_retry_delay("PUT", StatusCode::INTERNAL_SERVER_ERROR, 1),
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            apify_retry_delay("GET", StatusCode::SERVICE_UNAVAILABLE, 2),
            None
        );
        assert_eq!(
            apify_retry_delay("DELETE", StatusCode::BAD_REQUEST, 0),
            None
        );
    }

    #[tokio::test]
    async fn charge_retries_reuse_idempotency_key_and_dataset_posts_are_not_retried() {
        let (base, charge_server) = start_response_sequence(vec![503, 200]).await;
        let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
        let config = mock_config(&base);
        let apify = ApifyClient::new(http, &config.apify);
        let idempotency_key = "test-run-job-result-4";

        apify
            .charge_event(JOB_RESULT_CHARGE_EVENT, 1, idempotency_key)
            .await
            .unwrap();

        let charge_requests = charge_server.await.unwrap();
        assert_eq!(charge_requests.len(), 2);
        assert!(charge_requests.iter().all(|request| {
            request_line(request).starts_with("POST /v2/actor-runs/test-run/charge ")
                && request_text(request).contains("idempotency-key: test-run-job-result-4")
        }));

        let (base, dataset_server) = start_response_sequence(vec![503]).await;
        let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
        let config = mock_config(&base);
        let apify = ApifyClient::new(http, &config.apify);
        let error = apify
            .push_dataset_item(&json!({ "success": true }))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Apify dataset item publication failed (503)"));
        assert_eq!(dataset_server.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn actor_run_preserves_auth_batch_results_dataset_output_and_success_charges() {
        let input = json!({
            "url": "linkedin.com/jobs/view/missing",
            "urls": [
                "https://de.linkedin.com/jobs/view/missing/?trk=foo",
                "https://www.linkedin.com/jobs/view/1234567890?refId=abc"
            ],
            "use_cache": true,
            "maximum_cache_age": 3600
        });
        let (base, server) = start_actor_mock(input, ppe_run(1.0, json!({})), 8).await;
        let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
        let config = mock_config(&base);
        let apify = ApifyClient::new(http.clone(), &config.apify);

        let status_message = run(&config, &apify, http).await.unwrap();
        assert_eq!(status_message, None);

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 8);
        assert!(requests
            .iter()
            .filter(|request| !request_line(request).contains("/api/linkedin/job?"))
            .all(|request| request_text(request).contains("authorization: bearer test-token")));
        let scrappa_requests: Vec<_> = requests
            .iter()
            .filter(|request| request_line(request).contains("GET /api/linkedin/job?"))
            .collect();
        assert_eq!(scrappa_requests.len(), 2);
        assert!(request_text(scrappa_requests[0]).contains("x-api-key: test-api-key"));
        assert!(request_line(scrappa_requests[1]).contains("use_cache=1"));
        assert!(request_line(scrappa_requests[1]).contains("maximum_cache_age=3600"));

        let dataset_requests: Vec<_> = requests
            .iter()
            .filter(|request| {
                request_line(request).starts_with("POST /v2/datasets/test-dataset/items")
            })
            .collect();
        assert_eq!(dataset_requests.len(), 2);
        assert_eq!(request_body(dataset_requests[0])["status_code"], 404);
        assert_eq!(
            request_body(dataset_requests[0])["message"],
            "Job not found"
        );
        assert_eq!(
            request_body(dataset_requests[1])["title"],
            "Senior Engineer"
        );

        let charge_requests: Vec<_> = requests
            .iter()
            .filter(|request| {
                request_line(request).starts_with("POST /v2/actor-runs/test-run/charge")
            })
            .collect();
        assert_eq!(charge_requests.len(), 1);
        assert_eq!(
            request_body(charge_requests[0]),
            json!({
                "eventName": "job-result",
                "count": 1
            })
        );
        assert!(request_text(charge_requests[0]).contains("idempotency-key: test-run-job-result-1"));

        let output = requests
            .iter()
            .find(|request| {
                request_line(request)
                    .starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT")
            })
            .unwrap();
        assert_eq!(
            request_body(output),
            json!({ "requested": 2, "succeeded": 1, "failed": 1 })
        );
        assert!(requests
            .iter()
            .all(|request| { !request_line(request).starts_with("PUT /v2/actor-runs/test-run ") }));
    }

    #[tokio::test]
    async fn actor_run_stops_at_the_ppe_limit_and_sets_terminal_status() {
        let input = json!({
            "urls": [
                "linkedin.com/jobs/view/first",
                "linkedin.com/jobs/view/second"
            ]
        });
        let (base, server) = start_actor_mock(input, ppe_run(0.0012, json!({})), 7).await;
        let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
        let config = mock_config(&base);
        let apify = ApifyClient::new(http.clone(), &config.apify);

        let status_message = run(&config, &apify, http).await.unwrap().unwrap();
        assert_eq!(
            status_message,
            "Charge limit reached after saving 1 of 1 LinkedIn job detail results."
        );
        apify.set_terminal_status_message(&status_message).await;

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 7);
        assert_eq!(
            requests
                .iter()
                .filter(|request| request_line(request).contains("GET /api/linkedin/job?"))
                .count(),
            1
        );
        let output = requests
            .iter()
            .find(|request| {
                request_line(request)
                    .starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT")
            })
            .unwrap();
        assert_eq!(
            request_body(output),
            json!({ "requested": 2, "succeeded": 1, "failed": 0 })
        );
        let status = requests
            .iter()
            .find(|request| request_line(request).starts_with("PUT /v2/actor-runs/test-run "))
            .unwrap();
        assert_eq!(request_body(status)["statusMessage"], status_message);
        assert_eq!(request_body(status)["isStatusMessageTerminal"], true);
    }

    fn mock_config(base: &str) -> Config {
        Config {
            apify: ApifyConfig {
                api_base: base.to_owned(),
                token: "test-token".to_owned(),
                actor_run_id: "test-run".to_owned(),
                key_value_store_id: "test-store".to_owned(),
                dataset_id: "test-dataset".to_owned(),
                input_key: "INPUT".to_owned(),
            },
            scrappa_api_base: format!("{base}/api"),
            scrappa_api_key: "test-api-key".to_owned(),
        }
    }

    async fn start_response_sequence(statuses: Vec<u16>) -> (String, JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for status in statuses {
                let accepted =
                    tokio::time::timeout(Duration::from_secs(5), listener.accept()).await;
                let Ok(Ok((mut stream, _))) = accepted else {
                    break;
                };
                let request = read_http_request(&mut stream).await;
                let reason = if status == 200 {
                    "OK"
                } else if status == 503 {
                    "Service Unavailable"
                } else {
                    "Bad Request"
                };
                let body = if status == 200 {
                    "{}"
                } else {
                    r#"{"message":"temporary"}"#
                };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
                requests.push(request);
            }
            requests
        });
        (format!("http://{address}"), server)
    }

    async fn start_actor_mock(
        input: Value,
        pricing: Value,
        expected_requests: usize,
    ) -> (String, JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for _ in 0..expected_requests {
                let accepted =
                    tokio::time::timeout(Duration::from_secs(5), listener.accept()).await;
                let Ok(Ok((mut stream, _))) = accepted else {
                    break;
                };
                let request = read_http_request(&mut stream).await;
                let line = request_line(&request).to_owned();
                let (status, body) = if line.starts_with("GET /v2/actor-runs/test-run ") {
                    (200, pricing.to_string())
                } else if line.starts_with("GET /v2/key-value-stores/test-store/records/INPUT ") {
                    (200, input.to_string())
                } else if line.starts_with("GET /api/linkedin/job?") {
                    if line.contains("missing") {
                        (404, json!({ "message": "Not found" }).to_string())
                    } else {
                        (
                            200,
                            json!({
                                "success": true,
                                "title": "Senior Engineer",
                                "job_title": "Engineer",
                                "company": "Example Corp"
                            })
                            .to_string(),
                        )
                    }
                } else if line.starts_with("POST /v2/datasets/test-dataset/items ")
                    || line.starts_with("POST /v2/actor-runs/test-run/charge ")
                    || line.starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT ")
                    || line.starts_with("PUT /v2/actor-runs/test-run ")
                {
                    (200, "{}".to_owned())
                } else {
                    (
                        404,
                        json!({ "message": "Unexpected mock request", "path": line }).to_string(),
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    if status == 200 { "OK" } else { "Not Found" },
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
                requests.push(request);
            }
            requests
        });
        (format!("http://{address}"), server)
    }

    async fn read_http_request(stream: &mut TcpStream) -> Vec<u8> {
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
            let headers = String::from_utf8_lossy(&request[..body_start]);
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

    fn request_line(request: &[u8]) -> &str {
        std::str::from_utf8(request)
            .unwrap()
            .lines()
            .next()
            .unwrap_or_default()
    }

    fn request_text(request: &[u8]) -> String {
        String::from_utf8_lossy(request).to_lowercase()
    }

    fn request_body(request: &[u8]) -> Value {
        let body_start = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
            .unwrap();
        serde_json::from_slice(&request[body_start..]).unwrap()
    }
}
