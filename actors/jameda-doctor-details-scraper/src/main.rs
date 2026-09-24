use std::{collections::HashSet, env, process::ExitCode, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use rand::Rng;
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use url::Url;

const JAMEDA_BASE_URL: &str = "https://www.jameda.de";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const SCRAPPA_MAX_URLS: usize = 100;
const SCRAPPA_CHARGE_EVENT: &str = "doctor-profile-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const APIFY_MAX_RETRIES: usize = 2;
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

struct ApifyConfig {
    api_base: String,
    token: String,
    run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            token: required_env("APIFY_TOKEN")?,
            run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
        })
    }
}

struct ApifyClient {
    http: Client,
    config: ApifyConfig,
}

impl ApifyClient {
    fn new(config: ApifyConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .context("Failed to configure Apify API client")?;
        Ok(Self { http, config })
    }

    fn endpoint(&self, parts: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.api_base, parts)
    }

    async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self.request_with_retry("GET", url).await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        require_apify_success(response, "input retrieval")
            .await?
            .json::<Value>()
            .await
            .context("Apify input record was not valid JSON")
            .map(Some)
    }

    async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.run_id])?;
        require_apify_success(
            self.request_with_retry("GET", url).await?,
            "run pricing request",
        )
        .await?
        .json::<Value>()
        .await
        .context("Apify run pricing response was not valid JSON")
    }

    async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.token)
            .header(header::ACCEPT, "application/json")
            .json(item)
            .send()
            .await
            .context("Failed to publish dataset item to Apify API")?;
        require_apify_success(response, "dataset item publication").await?;
        Ok(())
    }

    async fn charge_event(&self, event_name: &str, idempotency_key: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.run_id, "charge"])?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .post(url.clone())
                .bearer_auth(&self.config.token)
                .header(header::ACCEPT, "application/json")
                .header("idempotency-key", idempotency_key)
                .json(&json!({ "eventName": event_name, "count": 1 }))
                .send()
                .await
                .context("Apify event charge request failed")?;

            if let Some(delay) = apify_retry_delay("POST_CHARGE", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            require_apify_success(response, "event charge").await?;
            return Ok(());
        }
    }

    async fn put_record(&self, key: &str, value: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            key,
        ])?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .put(url.clone())
                .bearer_auth(&self.config.token)
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

    async fn set_terminal_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.config.run_id])?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.config.token)
            .header(header::ACCEPT, "application/json")
            .json(&json!({
                "runId": self.config.run_id,
                "statusMessage": message,
                "isStatusMessageTerminal": true
            }))
            .send()
            .await
            .context("Failed to set terminal Actor status message")?;
        require_apify_success(response, "status message update").await?;
        Ok(())
    }

    async fn request_with_retry(&self, method: &str, url: Url) -> Result<Response> {
        let mut retry_count = 0;
        loop {
            let request = match method {
                "GET" => self.http.get(url.clone()),
                _ => bail!("Unsupported Apify retry method {method}"),
            };
            let response = request
                .bearer_auth(&self.config.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
                .with_context(|| format!("Apify {method} request failed"))?;

            if let Some(delay) = apify_retry_delay(method, response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }
            return Ok(response);
        }
    }
}

struct ScrappaClient {
    http: Client,
    base_url: String,
    api_key: String,
}

#[derive(Debug)]
struct ScrappaError {
    message: String,
    retryable: bool,
}

impl std::fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ScrappaError {}

impl ScrappaClient {
    fn new(api_key: String, base_url: String) -> Result<Self> {
        let http = Client::builder()
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .build()
            .context("Failed to configure Scrappa API client")?;
        Ok(Self {
            http,
            base_url,
            api_key,
        })
    }

    async fn get(&self, doctor_url: &str) -> std::result::Result<Value, ScrappaError> {
        self.get_with_delay(doctor_url, |attempt| {
            let jitter_ms = rand::thread_rng().gen_range(0..1000);
            Duration::from_millis(get_retry_delay_ms(attempt, jitter_ms))
        })
        .await
    }

    async fn get_with_delay<F>(
        &self,
        doctor_url: &str,
        mut retry_delay: F,
    ) -> std::result::Result<Value, ScrappaError>
    where
        F: FnMut(usize) -> Duration,
    {
        let mut url =
            endpoint_url(&self.base_url, &["jameda", "doctor-details"]).map_err(|error| {
                ScrappaError {
                    message: error.to_string(),
                    retryable: false,
                }
            })?;
        url.query_pairs_mut().append_pair("doctor_url", doctor_url);

        let mut last_error = None;
        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            match self.send(url.clone()).await {
                Ok(response) => return Ok(response),
                Err(error) => {
                    let retryable = error.retryable;
                    last_error = Some(error);
                    if attempt == SCRAPPA_MAX_ATTEMPTS || !retryable {
                        break;
                    }

                    let delay = retry_delay(attempt);
                    let error = last_error.as_ref().expect("Scrappa request error recorded");
                    eprintln!(
                        "Scrappa API request failed ({}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {}ms.",
                        error.message,
                        attempt + 1,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(last_error.expect("Scrappa request made at least one attempt"))
    }

    async fn send(&self, url: Url) -> std::result::Result<Value, ScrappaError> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .header(
                header::USER_AGENT,
                "thescrappa-jameda-doctor-details-scraper/1.0",
            )
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ScrappaError {
                        message: format!(
                            "Scrappa API request timed out after {}ms",
                            SCRAPPA_REQUEST_TIMEOUT.as_millis()
                        ),
                        retryable: true,
                    }
                } else {
                    ScrappaError {
                        message: "fetch failed".to_owned(),
                        retryable: true,
                    }
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
                    ScrappaError {
                        message: format!(
                            "Scrappa API request timed out after {}ms",
                            SCRAPPA_REQUEST_TIMEOUT.as_millis()
                        ),
                        retryable: true,
                    }
                } else {
                    ScrappaError {
                        message: format!("Scrappa API error ({}): {fallback}", status.as_u16()),
                        retryable: is_retryable_scrappa_status(status),
                    }
                }
            })?;
            return Err(ScrappaError {
                message: format!(
                    "Scrappa API error ({}): {}",
                    status.as_u16(),
                    scrappa_error_message(status.as_u16(), &body, &fallback)
                ),
                retryable: is_retryable_scrappa_status(status),
            });
        }

        response.json::<Value>().await.map_err(|error| {
            if error.is_timeout() {
                ScrappaError {
                    message: format!(
                        "Scrappa API request timed out after {}ms",
                        SCRAPPA_REQUEST_TIMEOUT.as_millis()
                    ),
                    retryable: true,
                }
            } else {
                ScrappaError {
                    message: error.to_string(),
                    retryable: false,
                }
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InputFailure {
    doctor_url: String,
    error: String,
}

#[derive(Debug)]
struct DoctorDetailsPlan {
    doctor_urls: Vec<String>,
    input_failures: Vec<InputFailure>,
}

fn build_doctor_details_plan(input: &Value) -> Result<DoctorDetailsPlan> {
    let mut values: Vec<(&str, Value)> = Vec::new();
    let object = input.as_object();

    if let Some(value) = object.and_then(|object| object.get("doctorUrl")) {
        if !value.is_null() && value.as_str() != Some("") {
            values.push(("doctorUrl", value.clone()));
        }
    }

    if let Some(value) = object.and_then(|object| object.get("doctorUrls")) {
        if !value.is_null() && value.as_str() != Some("") {
            if let Some(array) = value.as_array() {
                values.extend(array.iter().map(|value| ("doctorUrls", value.clone())));
            } else if let Some(text) = value.as_str() {
                values.extend(
                    text.split(|character| character == ',' || character == '\n')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| ("doctorUrls", Value::String(value.to_owned()))),
                );
            } else {
                bail!("doctorUrls must be an array of strings or a comma/newline-separated string");
            }
        }
    }

    if values.is_empty() {
        bail!("Provide doctorUrls or doctorUrl");
    }

    let mut doctor_urls = Vec::new();
    let mut seen = HashSet::new();
    let mut input_failures = Vec::new();
    for (field, value) in values {
        match clean_jameda_doctor_url(&value, field) {
            Ok(url) => {
                if seen.insert(url.clone()) {
                    doctor_urls.push(url);
                }
            }
            Err(error) => input_failures.push(InputFailure {
                doctor_url: js_string(&value),
                error,
            }),
        }
    }

    if doctor_urls.is_empty() {
        bail!("No valid Jameda doctor URLs were provided");
    }
    if doctor_urls.len() > SCRAPPA_MAX_URLS {
        bail!("doctorUrls can include at most {SCRAPPA_MAX_URLS} doctor URLs per run");
    }

    Ok(DoctorDetailsPlan {
        doctor_urls,
        input_failures,
    })
}

fn clean_jameda_doctor_url(value: &Value, field: &str) -> std::result::Result<String, String> {
    let Some(raw_value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };
    let raw_value = raw_value.trim();
    if raw_value.is_empty() {
        return Err(format!("{field} cannot be empty"));
    }

    let lower_value = raw_value.to_ascii_lowercase();
    let parsed_url = if lower_value.starts_with("http://") || lower_value.starts_with("https://") {
        Url::parse(raw_value)
    } else if raw_value
        .split('/')
        .next()
        .unwrap_or_default()
        .contains('.')
    {
        Url::parse(&format!("https://{raw_value}"))
    } else {
        let path = if raw_value.starts_with('/') {
            raw_value.to_owned()
        } else {
            format!("/{raw_value}")
        };
        Url::parse(JAMEDA_BASE_URL).and_then(|base| base.join(&path))
    }
    .map_err(|_| format!("{field} must be a valid Jameda doctor URL or path"))?;

    let hostname = parsed_url.host_str().unwrap_or_default();
    let domain = hostname.strip_prefix("www.").unwrap_or(hostname);
    if !domain.eq_ignore_ascii_case("jameda.de") {
        return Err(format!("{field} must use the jameda.de domain"));
    }

    let normalized_path = normalize_path(parsed_url.path());
    if normalized_path == "/"
        || normalized_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .count()
            < 3
    {
        return Err(format!(
            "{field} must point to a Jameda doctor profile path, for example /markus-lietzau-msc/zahnarzt/berlin"
        ));
    }

    Ok(format!("{JAMEDA_BASE_URL}{normalized_path}"))
}

fn normalize_path(path: &str) -> String {
    let mut normalized = String::with_capacity(path.len());
    let mut previous_was_slash = false;
    for character in path.chars() {
        if character == '/' {
            if !previous_was_slash {
                normalized.push(character);
            }
            previous_was_slash = true;
        } else {
            normalized.push(character);
            previous_was_slash = false;
        }
    }
    while normalized.ends_with('/') {
        normalized.pop();
    }
    if normalized.starts_with('/') {
        normalized
    } else {
        format!("/{normalized}")
    }
}

fn build_doctor_details_params(doctor_url: &str) -> Vec<(String, String)> {
    vec![("doctor_url".to_owned(), doctor_url.to_owned())]
}

fn describe_request(doctor_urls: &[String]) -> String {
    if doctor_urls.len() == 1 {
        doctor_urls[0].clone()
    } else {
        format!("{} doctor URLs", doctor_urls.len())
    }
}

#[derive(Debug)]
struct PricingState {
    is_pay_per_event: bool,
    event_prices: Map<String, Value>,
    charged_event_counts: Map<String, Value>,
    max_total_charge_usd: Option<f64>,
}

impl PricingState {
    fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_info = data
            .get("pricingInfo")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        if pricing_info.get("pricingModel").and_then(Value::as_str) != Some("PAY_PER_EVENT") {
            return Ok(Self {
                is_pay_per_event: false,
                event_prices: Map::new(),
                charged_event_counts: Map::new(),
                max_total_charge_usd: None,
            });
        }

        let events = pricing_info
            .pointer("/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?
            .clone();
        let event_prices = events
            .into_iter()
            .map(|(event_name, event)| {
                let price = event.get("eventPriceUsd").cloned().unwrap_or(Value::Null);
                if let Some(price) = price.as_f64() {
                    if !price.is_finite() || price < 0.0 {
                        bail!("Invalid price for charged event {event_name}");
                    }
                }
                Ok((event_name, price))
            })
            .collect::<Result<Map<String, Value>>>()?;

        if !event_prices.contains_key(SCRAPPA_CHARGE_EVENT) {
            bail!("Apify run did not provide the {SCRAPPA_CHARGE_EVENT} charge event");
        }
        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?
            .clone();
        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64);
        if max_total_charge_usd.is_some_and(|value| !value.is_finite() || value < 0.0) {
            bail!("Apify run returned invalid spending limit");
        }

        let state = Self {
            is_pay_per_event: true,
            event_prices,
            charged_event_counts,
            max_total_charge_usd,
        };
        state.spent_so_far()?;
        state.event_price(SCRAPPA_CHARGE_EVENT)?;
        if state.event_prices.contains_key(DEFAULT_DATASET_ITEM_EVENT) {
            state.event_price(DEFAULT_DATASET_ITEM_EVENT)?;
        }
        Ok(state)
    }

    fn can_save_one_result(&self) -> Result<bool> {
        if !self.is_pay_per_event {
            return Ok(true);
        }
        let Some(max_total_charge_usd) = self.max_total_charge_usd else {
            return Ok(true);
        };
        let item_price = self.event_price(SCRAPPA_CHARGE_EVENT)?
            + self.event_price(DEFAULT_DATASET_ITEM_EVENT)?;
        let spent = self.spent_so_far()?;
        let tolerance = f64::EPSILON * max_total_charge_usd.max(1.0);
        Ok(spent + item_price <= max_total_charge_usd + tolerance)
    }

    fn event_price(&self, event_name: &str) -> Result<f64> {
        let Some(value) = self.event_prices.get(event_name) else {
            return Ok(0.0);
        };
        let price = value.as_f64().ok_or_else(|| {
            anyhow!("Apify run did not provide a flat price for charged event {event_name}")
        })?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        Ok(price)
    }

    fn spent_so_far(&self) -> Result<f64> {
        let mut spent = 0.0;
        for (event_name, count_value) in &self.charged_event_counts {
            let count = count_value
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            if count == 0 {
                continue;
            }
            let price = self.event_price(event_name)?;
            spent += price * count as f64;
        }
        if !spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }
        Ok(spent)
    }

    fn record_dataset_item(&mut self) {
        if self.event_prices.contains_key(DEFAULT_DATASET_ITEM_EVENT) {
            increment_event_count(&mut self.charged_event_counts, DEFAULT_DATASET_ITEM_EVENT);
        }
    }

    fn record_custom_charge(&mut self) {
        increment_event_count(&mut self.charged_event_counts, SCRAPPA_CHARGE_EVENT);
    }
}

fn increment_event_count(counts: &mut Map<String, Value>, event_name: &str) {
    let count = counts
        .get(event_name)
        .and_then(Value::as_u64)
        .unwrap_or_default();
    counts.insert(event_name.to_owned(), json!(count + 1));
}

struct PushChargedItemResult {
    saved_count: usize,
    status_message: Option<String>,
}

async fn push_charged_item(
    apify: &ApifyClient,
    pricing: &mut Option<PricingState>,
    item: &Value,
    result_index: usize,
) -> Result<PushChargedItemResult> {
    if pricing.is_none() {
        *pricing = Some(PricingState::from_run(&apify.get_run().await?)?);
    }
    let state = pricing.as_mut().expect("Pricing state initialized");
    if !state.is_pay_per_event {
        apify.push_dataset_item(item).await?;
        return Ok(PushChargedItemResult {
            saved_count: 1,
            status_message: None,
        });
    }

    if !state.can_save_one_result()? {
        let status_message =
            "Charge limit reached after saving 0 of 1 Jameda doctor profile results.".to_owned();
        eprintln!(
            "{status_message} {{\"event\":\"{SCRAPPA_CHARGE_EVENT}\",\"charged_count\":0,\"requested_count\":1,\"saved_count\":0}}"
        );
        return Ok(PushChargedItemResult {
            saved_count: 0,
            status_message: Some(status_message),
        });
    }

    apify.push_dataset_item(item).await?;
    state.record_dataset_item();
    let idempotency_key = format!(
        "{}-{SCRAPPA_CHARGE_EVENT}-{result_index}",
        apify.config.run_id
    );
    apify
        .charge_event(SCRAPPA_CHARGE_EVENT, &idempotency_key)
        .await?;
    state.record_custom_charge();

    Ok(PushChargedItemResult {
        saved_count: 1,
        status_message: None,
    })
}

fn build_dataset_item(response: &Value, doctor_url: &str, params: &[(String, String)]) -> Value {
    let profile = response
        .get("data")
        .filter(|value| !value.is_null())
        .unwrap_or(response);
    let basic_info = profile.get("basic_info").unwrap_or(&Value::Null);
    let rating = profile.get("rating").unwrap_or(&Value::Null);
    let clinic = profile.get("clinic").unwrap_or(&Value::Null);
    let contact = profile.get("contact").unwrap_or(&Value::Null);
    let coordinates = profile.get("coordinates").unwrap_or(&Value::Null);
    let metadata = profile.get("metadata").unwrap_or(&Value::Null);
    let scrape_metadata = response.get("meta").unwrap_or(&Value::Null);
    let address = profile.get("address").unwrap_or(&Value::Null);
    let rating_value = first_present(&[
        rating.get("rating"),
        rating.get("score"),
        rating.get("overall_score"),
    ]);
    let review_count = first_present(&[rating.get("count"), rating.get("review_count")]);
    let requested_url = params
        .iter()
        .find(|(name, _)| name == "doctor_url")
        .map(|(_, value)| json!(value))
        .unwrap_or(Value::Null);

    let mut item = response.as_object().cloned().unwrap_or_default();
    let fields = [
        ("requested_doctor_url", json!(doctor_url)),
        (
            "doctor_url",
            first_non_empty_string(&[basic_info.get("profile_url"), basic_info.get("url")])
                .map(|value| json!(value))
                .unwrap_or_else(|| json!(doctor_url)),
        ),
        (
            "doctor_name",
            first_non_empty_string(&[basic_info.get("name"), profile.get("name")])
                .map(|value| json!(value))
                .unwrap_or(Value::Null),
        ),
        ("title", value_or_null(basic_info.get("title"))),
        (
            "specialty",
            first_non_empty_string(&[
                basic_info.get("specialty"),
                basic_info.get("specializations"),
                profile.get("specialty"),
            ])
            .map(|value| json!(value))
            .unwrap_or(Value::Null),
        ),
        ("description", value_or_null(profile.get("description"))),
        ("rating", rating_value.cloned().unwrap_or(Value::Null)),
        (
            "rating_number",
            number_or_null(to_decimal_number(rating_value)),
        ),
        ("review_count", review_count.cloned().unwrap_or(Value::Null)),
        (
            "review_count_number",
            number_or_null(to_count_number(review_count)),
        ),
        (
            "clinic_name",
            first_non_empty_string(&[clinic.get("name")])
                .map(|value| json!(value))
                .unwrap_or(Value::Null),
        ),
        (
            "phone",
            first_non_empty_string(&[contact.get("phone")])
                .map(|value| json!(value))
                .unwrap_or(Value::Null),
        ),
        (
            "website_url",
            with_protocol(contact.get("website"))
                .map(|value| json!(value))
                .unwrap_or(Value::Null),
        ),
        ("address", json!(build_full_address(address))),
        (
            "city",
            address
                .as_object()
                .and_then(|value| value.get("city"))
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "postal_code",
            address
                .as_object()
                .and_then(|value| first_present(&[value.get("postal_code"), value.get("zip")]))
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "latitude",
            number_or_null(to_decimal_number(first_present(&[
                coordinates.get("latitude"),
                coordinates.get("lat"),
            ]))),
        ),
        (
            "longitude",
            number_or_null(to_decimal_number(first_present(&[
                coordinates.get("longitude"),
                coordinates.get("lng"),
            ]))),
        ),
        (
            "image_url",
            with_protocol(basic_info.get("image_url"))
                .map(|value| json!(value))
                .unwrap_or(Value::Null),
        ),
        ("services_count", count_items(profile.get("services"))),
        (
            "focus_areas_count",
            count_items(first_present(&[
                profile.get("focus_areas"),
                profile.get("specialization_focus"),
            ])),
        ),
        (
            "conditions_count",
            count_items(first_present(&[
                profile.get("conditions"),
                profile.get("conditions_treated"),
            ])),
        ),
        ("languages_count", count_items(profile.get("languages"))),
        ("opening_hours", value_or_null(profile.get("opening_hours"))),
        ("services", value_or_null(profile.get("services"))),
        (
            "accepted_patients",
            value_or_null(profile.get("accepted_patients")),
        ),
        (
            "focus_areas",
            value_or_null(first_present(&[
                profile.get("focus_areas"),
                profile.get("specialization_focus"),
            ])),
        ),
        (
            "conditions",
            value_or_null(first_present(&[
                profile.get("conditions"),
                profile.get("conditions_treated"),
            ])),
        ),
        ("languages", value_or_null(profile.get("languages"))),
        ("booking_ids", value_or_null(profile.get("booking_ids"))),
        ("request_doctor_url", requested_url),
        (
            "response_source",
            value_or_null(first_present(&[
                metadata.get("source"),
                scrape_metadata.get("source"),
            ])),
        ),
        (
            "scraped_at",
            value_or_null(first_present(&[
                metadata.get("scraped_at"),
                scrape_metadata.get("scraped_at"),
            ])),
        ),
    ];
    item.extend(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value)),
    );
    Value::Object(item)
}

fn build_output_summary(
    doctor_urls: &[String],
    saved_profiles: usize,
    failures: &[InputFailure],
    status_message: Option<&str>,
) -> Value {
    json!({
        "request": {
            "endpoint": "/jameda/doctor-details",
            "doctor_urls": doctor_urls,
        },
        "doctors_requested": doctor_urls.len(),
        "doctors_saved": saved_profiles,
        "doctors_failed": failures.len(),
        "responses_saved": saved_profiles,
        "status_message": status_message,
        "failures": failures.iter().map(|failure| json!({
            "doctor_url": failure.doctor_url,
            "error": failure.error,
        })).collect::<Vec<_>>(),
    })
}

struct RunOutcome {
    status_message: Option<String>,
    succeeded: bool,
}

async fn run_actor(apify: &ApifyClient, api_key: &str) -> Result<RunOutcome> {
    let input = apify
        .get_input()
        .await?
        .ok_or_else(|| anyhow!("Input is required"))?;
    let plan = build_doctor_details_plan(&input)?;
    println!(
        "Fetching Jameda doctor details for {}",
        describe_request(&plan.doctor_urls)
    );

    let scrappa = ScrappaClient::new(
        api_key.to_owned(),
        env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
    )?;
    let mut failures = plan.input_failures;
    let mut saved_profiles = 0;
    let mut status_message = None;
    let mut pricing = None;

    for (index, doctor_url) in plan.doctor_urls.iter().enumerate() {
        let params = build_doctor_details_params(doctor_url);
        println!("Fetching Jameda doctor details for {doctor_url}");

        let result = async {
            let response = scrappa.get(doctor_url).await.map_err(|error| {
                let message = if error.message.starts_with("Scrappa API request timed out after ") {
                    format!(
                        "{}. The Jameda doctor details request exceeded the 90s Scrappa API timeout. Try fewer doctor URLs or run the request again.",
                        error.message
                    )
                } else {
                    error.message
                };
                anyhow::Error::msg(message)
            })?;
            let item = build_dataset_item(&response, doctor_url, &params);
            push_charged_item(apify, &mut pricing, &item, index + 1).await
        }
        .await;

        match result {
            Ok(result) => {
                saved_profiles += result.saved_count;
                println!(
                    "Saved {} Jameda doctor profile result(s) for {doctor_url}",
                    result.saved_count
                );
                if result.status_message.is_some() {
                    status_message = result.status_message;
                    break;
                }
            }
            Err(error) => {
                let message = error.to_string();
                failures.push(InputFailure {
                    doctor_url: doctor_url.clone(),
                    error: message.clone(),
                });
                eprintln!("Failed to fetch Jameda doctor details for {doctor_url}: {message}");
            }
        }
    }

    if status_message.is_none() && !failures.is_empty() {
        status_message = Some(format!(
            "{} Jameda doctor detail request(s) failed; {} profile(s) saved.",
            failures.len(),
            saved_profiles
        ));
    }

    let summary = build_output_summary(
        &plan.doctor_urls,
        saved_profiles,
        &failures,
        status_message.as_deref(),
    );
    apify.put_record("OUTPUT", &summary).await?;

    println!("Jameda doctor details extraction completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "doctors_requested": plan.doctor_urls.len(),
            "doctors_saved": saved_profiles,
            "doctors_failed": failures.len(),
        })
    );

    if saved_profiles == 0 && !failures.is_empty() {
        return Ok(RunOutcome {
            status_message: Some(
                status_message
                    .unwrap_or_else(|| "No Jameda doctor profiles were saved.".to_owned()),
            ),
            succeeded: false,
        });
    }

    Ok(RunOutcome {
        status_message,
        succeeded: true,
    })
}

async fn run() -> ExitCode {
    let apify_config = match ApifyConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let apify = match ApifyClient::new(apify_config) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            return ExitCode::FAILURE;
        }
    };

    let api_key = match env::var("SCRAPPA_API_KEY") {
        Ok(value) if !value.is_empty() => value,
        _ => {
            let error = "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.";
            eprintln!("Actor failed: {error}");
            if let Err(status_error) = apify.set_terminal_status_message(error).await {
                eprintln!("Failed to set Actor status message: {status_error}");
            }
            return ExitCode::FAILURE;
        }
    };

    match run_actor(&apify, &api_key).await {
        Ok(outcome) => {
            if let Some(status_message) = outcome.status_message {
                if let Err(error) = apify.set_terminal_status_message(&status_message).await {
                    eprintln!("Failed to set Actor status message: {error}");
                    return ExitCode::FAILURE;
                }
            }
            if outcome.succeeded {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            if let Err(status_error) = apify.set_terminal_status_message(&message).await {
                eprintln!("Failed to set Actor status message: {status_error}");
            }
            ExitCode::FAILURE
        }
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

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

fn apify_retry_delay(method: &str, status: StatusCode, retry_count: usize) -> Option<Duration> {
    if !matches!(method, "GET" | "PUT" | "POST_CHARGE")
        || retry_count >= APIFY_MAX_RETRIES
        || (status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error())
    {
        return None;
    }
    Some(Duration::from_secs((retry_count + 1) as u64))
}

fn is_retryable_scrappa_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
}

fn get_retry_delay_ms(failed_attempt: usize, jitter_ms: u64) -> u64 {
    let exponential =
        1000u64.saturating_mul(1u64.checked_shl(failed_attempt as u32).unwrap_or(u64::MAX));
    exponential.saturating_add(jitter_ms).min(10_000)
}

fn scrappa_error_message(status: u16, body: &str, fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }
    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let Some(error_data) = error_data.as_object() else {
            return fallback.to_owned();
        };
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
    let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        fallback.to_owned()
    } else {
        let message: String = collapsed.chars().take(500).collect();
        if message.is_empty() {
            format!("HTTP {status}")
        } else {
            message
        }
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

fn value_or_null(value: Option<&Value>) -> Value {
    value.cloned().unwrap_or(Value::Null)
}

fn first_present<'a>(values: &[Option<&'a Value>]) -> Option<&'a Value> {
    values
        .iter()
        .copied()
        .find(|value| value.is_some_and(|value| !value.is_null()))
        .flatten()
}

fn first_non_empty_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().copied().find_map(|value| {
        value
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn with_protocol(value: Option<&Value>) -> Option<String> {
    let value = value?.as_str()?.trim();
    if value.is_empty() {
        return None;
    }
    if value.starts_with("//") {
        Some(format!("https:{value}"))
    } else if value.starts_with("http://") || value.starts_with("https://") {
        Some(value.to_owned())
    } else {
        Some(format!("https://{value}"))
    }
}

fn build_full_address(address: &Value) -> Option<String> {
    if let Some(address) = address.as_str() {
        let address = address.trim();
        return (!address.is_empty()).then(|| address.to_owned());
    }
    let address = address.as_object()?;
    first_non_empty_string(&[address.get("full_address")]).or_else(|| {
        let postal_code = first_present(&[address.get("postal_code"), address.get("zip")]);
        let components = [address.get("street"), postal_code, address.get("city")]
            .into_iter()
            .filter_map(|value| {
                value
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
            })
            .collect::<Vec<_>>();
        (!components.is_empty()).then(|| components.join(", "))
    })
}

fn extract_numeric_string(value: Option<&Value>) -> Option<String> {
    if let Some(number) = value.and_then(Value::as_f64) {
        return number.is_finite().then(|| number.to_string());
    }
    let text = value?.as_str()?;
    let bytes = text.as_bytes();
    let mut start = None;
    let mut end = None;
    for index in 0..bytes.len() {
        let is_start = bytes[index].is_ascii_digit()
            || (bytes[index] == b'-' && bytes.get(index + 1).is_some_and(u8::is_ascii_digit));
        if start.is_none() && is_start {
            start = Some(index);
            continue;
        }
        if let Some(start_index) = start {
            if bytes[index].is_ascii_digit() || bytes[index] == b'.' || bytes[index] == b',' {
                end = Some(index + 1);
            } else if index > start_index {
                break;
            }
        }
    }
    let range = start?..end?;
    text.get(range).map(str::to_owned)
}

fn to_decimal_number(value: Option<&Value>) -> Option<f64> {
    if let Some(number) = value.and_then(Value::as_f64) {
        return number.is_finite().then_some(number);
    }
    let numeric = extract_numeric_string(value)?;
    let has_comma = numeric.contains(',');
    let has_dot = numeric.contains('.');
    let normalized = if has_comma && has_dot {
        if numeric.rfind(',')? > numeric.rfind('.')? {
            numeric.replace('.', "").replacen(',', ".", 1)
        } else {
            numeric.replace(',', "")
        }
    } else if has_comma {
        if is_grouped_digits(&numeric, ',') {
            numeric.replace(',', "")
        } else {
            numeric.replacen(',', ".", 1)
        }
    } else if is_multi_grouped_digits(&numeric, '.') {
        numeric.replace('.', "")
    } else {
        numeric
    };
    normalized
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

fn to_count_number(value: Option<&Value>) -> Option<f64> {
    if let Some(number) = value.and_then(Value::as_f64) {
        return number.is_finite().then_some(number);
    }
    let numeric = extract_numeric_string(value)?;
    let normalized = if is_grouped_count(&numeric) {
        numeric.replace([',', '.'], "")
    } else {
        numeric.replacen(',', ".", 1)
    };
    normalized
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

fn is_grouped_digits(value: &str, separator: char) -> bool {
    let groups = value.split(separator).collect::<Vec<_>>();
    groups.len() > 1
        && (1..=3).contains(&groups[0].len())
        && groups[0]
            .chars()
            .all(|character| character.is_ascii_digit())
        && groups[1..].iter().all(|group| {
            group.len() == 3 && group.chars().all(|character| character.is_ascii_digit())
        })
}

fn is_multi_grouped_digits(value: &str, separator: char) -> bool {
    let groups = value.split(separator).collect::<Vec<_>>();
    groups.len() >= 3
        && (1..=3).contains(&groups[0].len())
        && groups[0]
            .chars()
            .all(|character| character.is_ascii_digit())
        && groups[1..].iter().all(|group| {
            group.len() == 3 && group.chars().all(|character| character.is_ascii_digit())
        })
}

fn is_grouped_count(value: &str) -> bool {
    [',', '.']
        .into_iter()
        .any(|separator| is_grouped_digits(value, separator))
}

fn count_items(value: Option<&Value>) -> Value {
    value
        .and_then(Value::as_array)
        .map(|items| json!(items.len()))
        .unwrap_or(Value::Null)
}

fn number_or_null(number: Option<f64>) -> Value {
    number
        .and_then(serde_json::Number::from_f64)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

#[tokio::main]
async fn main() -> ExitCode {
    run().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        task::JoinHandle,
    };

    const MARKUS_URL: &str = "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin";

    struct MockRequest {
        head: String,
        body: String,
    }

    async fn start_mock_server(
        responses: Vec<(u16, String)>,
    ) -> (String, JoinHandle<Vec<MockRequest>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0u8; 2048];
                let body_start = loop {
                    let read = stream.read(&mut buffer).await.unwrap();
                    if read == 0 {
                        panic!("mock client closed before request completed");
                    }
                    request.extend_from_slice(&buffer[..read]);
                    let Some(position) =
                        request.windows(4).position(|window| window == b"\r\n\r\n")
                    else {
                        continue;
                    };
                    let body_start = position + 4;
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
                        break body_start;
                    }
                };
                let headers = String::from_utf8_lossy(&request[..body_start]).to_string();
                let request_body = String::from_utf8_lossy(&request[body_start..]).to_string();
                requests.push(MockRequest {
                    head: headers,
                    body: request_body,
                });
                let reason = match status {
                    200 => "OK",
                    201 => "Created",
                    400 => "Bad Request",
                    429 => "Too Many Requests",
                    500 => "Internal Server Error",
                    503 => "Service Unavailable",
                    _ => "Mock Response",
                };
                let response_headers = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(response_headers.as_bytes()).await.unwrap();
                stream.write_all(body.as_bytes()).await.unwrap();
            }
            requests
        });
        (format!("http://{address}"), server)
    }

    fn mock_apify_client(base_url: String) -> ApifyClient {
        ApifyClient::new(ApifyConfig {
            api_base: base_url,
            token: "test-token".to_owned(),
            run_id: "test-run".to_owned(),
            key_value_store_id: "store".to_owned(),
            dataset_id: "dataset".to_owned(),
            input_key: "INPUT".to_owned(),
        })
        .unwrap()
    }

    fn mock_ppe_run(max_total_charge: Value, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "doctor-profile-result": {"eventPriceUsd": 0.001},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                        "other-event": {"eventPriceUsd": 0.0002}
                    }}
                },
                "chargedEventCounts": counts,
                "options": {"maxTotalChargeUsd": max_total_charge}
            }
        })
    }

    #[test]
    fn normalizes_full_urls_paths_queries_and_host_style_urls() {
        assert_eq!(
            clean_jameda_doctor_url(
                &json!(format!(" {MARKUS_URL}?utm_source=test ")),
                "doctorUrl"
            )
            .unwrap(),
            MARKUS_URL
        );
        assert_eq!(
            clean_jameda_doctor_url(&json!("/markus-lietzau-msc/zahnarzt/berlin/"), "doctorUrl")
                .unwrap(),
            MARKUS_URL
        );
        assert_eq!(
            clean_jameda_doctor_url(
                &json!("jameda.de/markus-lietzau-msc/zahnarzt/berlin"),
                "doctorUrl"
            )
            .unwrap(),
            MARKUS_URL
        );
        assert_eq!(
            clean_jameda_doctor_url(
                &json!("http://jameda.de/markus-lietzau-msc/zahnarzt/berlin"),
                "doctorUrl"
            )
            .unwrap(),
            MARKUS_URL
        );
        assert_eq!(
            clean_jameda_doctor_url(
                &json!("/markus-lietzau-msc/zahnarzt/berlin%20mitte"),
                "doctorUrl"
            )
            .unwrap(),
            "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin%20mitte"
        );
    }

    #[test]
    fn rejects_invalid_urls_with_typescript_validation_messages() {
        assert_eq!(
            clean_jameda_doctor_url(&json!(3), "doctorUrls").unwrap_err(),
            "doctorUrls must be a string"
        );
        assert_eq!(
            clean_jameda_doctor_url(&json!(""), "doctorUrl").unwrap_err(),
            "doctorUrl cannot be empty"
        );
        assert!(clean_jameda_doctor_url(
            &json!("https://example.com/doctor/zahnarzt/berlin"),
            "doctorUrl"
        )
        .unwrap_err()
        .contains("jameda.de domain"));
        assert!(clean_jameda_doctor_url(&json!("/search"), "doctorUrl")
            .unwrap_err()
            .contains("doctor profile path"));
    }

    #[test]
    fn combines_batch_and_legacy_input_deduplicates_and_keeps_bad_values() {
        let plan = build_doctor_details_plan(&json!({
            "doctorUrl": MARKUS_URL,
            "doctorUrls": [
                MARKUS_URL,
                "/markus-lietzau-msc/zahnarzt/berlin",
                "/anna-example/aerztin/hamburg",
                "https://example.com/doctor/zahnarzt/berlin",
                12
            ]
        }))
        .unwrap();
        assert_eq!(
            plan.doctor_urls,
            vec![
                MARKUS_URL,
                "https://www.jameda.de/anna-example/aerztin/hamburg"
            ]
        );
        assert_eq!(plan.input_failures.len(), 2);
        assert_eq!(
            plan.input_failures[0].doctor_url,
            "https://example.com/doctor/zahnarzt/berlin"
        );
        assert_eq!(plan.input_failures[1].doctor_url, "12");
        assert_eq!(describe_request(&plan.doctor_urls), "2 doctor URLs");
    }

    #[test]
    fn parses_comma_and_newline_lists_and_rejects_missing_or_too_many_urls() {
        let plan = build_doctor_details_plan(&json!({
            "doctorUrls": format!("{MARKUS_URL}, /anna-example/aerztin/hamburg\n/hans-example/orthopaede/muenchen")
        })).unwrap();
        assert_eq!(plan.doctor_urls.len(), 3);
        assert!(build_doctor_details_plan(&json!({}))
            .unwrap_err()
            .to_string()
            .contains("Provide doctorUrls or doctorUrl"));
        assert!(build_doctor_details_plan(&json!({"doctorUrls": 12}))
            .unwrap_err()
            .to_string()
            .contains("doctorUrls must be an array"));
        let too_many = (0..101)
            .map(|index| format!("/doctor-{index}/zahnarzt/berlin"))
            .collect::<Vec<_>>();
        assert!(build_doctor_details_plan(&json!({"doctorUrls": too_many}))
            .unwrap_err()
            .to_string()
            .contains("at most 100 doctor URLs"));
    }

    #[test]
    fn returns_params_and_describes_single_requests_as_before() {
        assert_eq!(
            build_doctor_details_params(MARKUS_URL),
            vec![("doctor_url".to_owned(), MARKUS_URL.to_owned())]
        );
        assert_eq!(describe_request(&[MARKUS_URL.to_owned()]), MARKUS_URL);
    }

    #[test]
    fn normalizes_profile_response_fields_and_preserves_upstream_payload() {
        let response = json!({
            "success": true,
            "meta": {"source": "scrappa", "scraped_at": "2026-06-20T00:00:00Z"},
            "data": {
                "basic_info": {
                    "name": " Markus Lietzau M.Sc. ", "title": "M.Sc.",
                    "specialty": "Zahnarzt", "profile_url": MARKUS_URL,
                    "image_url": "//images.example/doctor.jpg"
                },
                "description": "Zahnarzt in Berlin",
                "rating": {"rating": "1,0", "count": "1.234 Bewertungen"},
                "clinic": {"name": "Praxis Markus Lietzau M.Sc. Zahnarzt"},
                "contact": {"phone": "+49 30 123456", "website": "example.com"},
                "address": {"street": "Teststr. 1", "postal_code": "10115", "city": "Berlin"},
                "coordinates": {"latitude": "52,5200", "longitude": "13.4050"},
                "opening_hours": {"monday": "09:00-17:00"},
                "services": ["Implantologie", "Prophylaxe"],
                "accepted_patients": ["Privat"], "focus_areas": ["Zahnerhaltung"],
                "conditions": ["Karies"], "languages": ["Deutsch", "Englisch"],
                "booking_ids": {"doctor_id": "abc123"}
            }
        });
        let params = build_doctor_details_params(MARKUS_URL);
        let item = build_dataset_item(&response, MARKUS_URL, &params);
        assert_eq!(item["success"], true);
        assert_eq!(item["requested_doctor_url"], MARKUS_URL);
        assert_eq!(item["doctor_url"], MARKUS_URL);
        assert_eq!(item["doctor_name"], "Markus Lietzau M.Sc.");
        assert_eq!(item["title"], "M.Sc.");
        assert_eq!(item["specialty"], "Zahnarzt");
        assert_eq!(item["rating_number"], 1.0);
        assert_eq!(item["review_count_number"], 1234.0);
        assert_eq!(item["clinic_name"], "Praxis Markus Lietzau M.Sc. Zahnarzt");
        assert_eq!(item["website_url"], "https://example.com");
        assert_eq!(item["address"], "Teststr. 1, 10115, Berlin");
        assert_eq!(item["latitude"], 52.52);
        assert_eq!(item["longitude"], 13.405);
        assert_eq!(item["image_url"], "https://images.example/doctor.jpg");
        assert_eq!(item["services_count"], 2);
        assert_eq!(item["focus_areas_count"], 1);
        assert_eq!(item["conditions_count"], 1);
        assert_eq!(item["languages_count"], 2);
        assert_eq!(item["booking_ids"], json!({"doctor_id":"abc123"}));
        assert_eq!(item["request_doctor_url"], MARKUS_URL);
        assert_eq!(item["response_source"], "scrappa");
        assert_eq!(item["scraped_at"], "2026-06-20T00:00:00Z");
    }

    #[test]
    fn handles_sparse_response_aliases_and_numeric_separators() {
        let response = json!({
            "basic_info": {"name": "Example Doctor"},
            "address": "Berlin",
            "rating": {"score": 1.7, "review_count": 4},
            "coordinates": {"latitude": "52.520", "longitude": "13.405"}
        });
        let item = build_dataset_item(
            &response,
            MARKUS_URL,
            &build_doctor_details_params(MARKUS_URL),
        );
        assert_eq!(item["doctor_name"], "Example Doctor");
        assert_eq!(item["rating_number"], 1.7);
        assert_eq!(item["review_count_number"], 4.0);
        assert_eq!(item["address"], "Berlin");
        assert_eq!(item["services_count"], Value::Null);
        assert_eq!(item["languages"], Value::Null);
        assert_eq!(
            to_decimal_number(Some(&json!("1.234.567"))),
            Some(1234567.0)
        );
        assert_eq!(to_count_number(Some(&json!("1,234 reviews"))), Some(1234.0));
        assert_eq!(to_decimal_number(Some(&json!("-1,234"))), Some(-1.234));
        assert_eq!(to_count_number(Some(&json!("-1,234"))), Some(-1.234));
    }

    #[test]
    fn builds_compact_output_summary_and_keeps_failure_shapes() {
        let failures = vec![InputFailure {
            doctor_url: "bad".to_owned(),
            error: "invalid".to_owned(),
        }];
        let summary = build_output_summary(&[MARKUS_URL.to_owned()], 1, &failures, Some("partial"));
        assert_eq!(summary["request"]["endpoint"], "/jameda/doctor-details");
        assert_eq!(summary["doctors_requested"], 1);
        assert_eq!(summary["doctors_saved"], 1);
        assert_eq!(summary["doctors_failed"], 1);
        assert_eq!(summary["responses_saved"], 1);
        assert_eq!(summary["status_message"], "partial");
        assert_eq!(summary["failures"][0]["doctor_url"], "bad");
    }

    #[test]
    fn formats_scrappa_http_errors_and_retries_only_transient_statuses() {
        assert_eq!(
            scrappa_error_message(
                422,
                r#"{"message":"Invalid","errors":{"doctor_url":["bad","required"]}}"#,
                "Unprocessable Entity"
            ),
            "Invalid - doctor_url: bad, required"
        );
        assert_eq!(
            scrappa_error_message(503, "Unavailable\ntry later", "Service Unavailable"),
            "Unavailable try later"
        );
        assert_eq!(
            scrappa_error_message(503, "", "Service Unavailable"),
            "Service Unavailable"
        );
        assert_eq!(
            scrappa_error_message(503, "[]", "Service Unavailable"),
            "Service Unavailable"
        );
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_scrappa_status(
                StatusCode::from_u16(status).unwrap()
            ));
        }
        for status in [400, 401, 403, 404, 422] {
            assert!(!is_retryable_scrappa_status(
                StatusCode::from_u16(status).unwrap()
            ));
        }
    }

    #[test]
    fn keeps_exponential_backoff_jitter_and_ten_second_cap() {
        assert_eq!(get_retry_delay_ms(1, 0), 2000);
        assert_eq!(get_retry_delay_ms(2, 250), 4250);
        assert_eq!(get_retry_delay_ms(3, 999), 8999);
        assert_eq!(get_retry_delay_ms(4, 999), 10000);
    }

    #[test]
    fn accounts_for_custom_dataset_and_other_event_prices_under_budget() {
        let affordable =
            PricingState::from_run(&mock_ppe_run(json!(0.0013), json!({"other-event": 1})))
                .unwrap();
        assert!(affordable.can_save_one_result().unwrap());
        let too_expensive =
            PricingState::from_run(&mock_ppe_run(json!(0.00129), json!({"other-event": 1})))
                .unwrap();
        assert!(!too_expensive.can_save_one_result().unwrap());
        let mut state = affordable;
        state.record_dataset_item();
        state.record_custom_charge();
        assert!(!state.can_save_one_result().unwrap());
    }

    #[test]
    fn non_ppe_runs_skip_custom_event_budgeting() {
        let state =
            PricingState::from_run(&json!({"data":{"pricingInfo":{"pricingModel":"FREE"}}}))
                .unwrap();
        assert!(state.can_save_one_result().unwrap());
    }

    #[test]
    fn rejects_incomplete_or_unsafe_ppe_metadata() {
        assert!(PricingState::from_run(
            &json!({"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"}}})
        )
        .unwrap_err()
        .to_string()
        .contains("event prices"));
        let missing_custom = json!({
            "data": {
                "pricingInfo": {"pricingModel":"PAY_PER_EVENT", "pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001}}}},
                "chargedEventCounts": {}, "options":{"maxTotalChargeUsd":1.0}
            }
        });
        assert!(PricingState::from_run(&missing_custom)
            .unwrap_err()
            .to_string()
            .contains("doctor-profile-result"));
    }

    #[tokio::test]
    async fn calls_scrappa_with_query_auth_and_retries_transient_http_errors() {
        let response_body = r#"{"success":true,"data":{"basic_info":{"name":"Doctor"}}}"#;
        let (base_url, server) = start_mock_server(vec![
            (503, r#"{"message":"temporarily unavailable"}"#.to_owned()),
            (200, response_body.to_owned()),
        ])
        .await;
        let client = ScrappaClient::new("scrappa-test-key".to_owned(), base_url).unwrap();
        let result = client
            .get_with_delay(MARKUS_URL, |_| Duration::ZERO)
            .await
            .unwrap();
        assert_eq!(result["data"]["basic_info"]["name"], "Doctor");
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].head.starts_with("GET /jameda/doctor-details?doctor_url=https%3A%2F%2Fwww.jameda.de%2Fmarkus-lietzau-msc%2Fzahnarzt%2Fberlin HTTP/1.1"));
        assert!(requests[0]
            .head
            .to_ascii_lowercase()
            .contains("x-api-key: scrappa-test-key"));
        assert!(requests[0]
            .head
            .to_ascii_lowercase()
            .contains("user-agent: thescrappa-jameda-doctor-details-scraper/1.0"));
    }

    #[tokio::test]
    async fn does_not_retry_validation_errors() {
        let (base_url, server) =
            start_mock_server(vec![(400, r#"{"message":"bad input"}"#.to_owned())]).await;
        let client = ScrappaClient::new("scrappa-test-key".to_owned(), base_url).unwrap();
        let error = client
            .get_with_delay(MARKUS_URL, |_| panic!("validation error must not retry"))
            .await
            .unwrap_err();
        assert_eq!(error.message, "Scrappa API error (400): bad input");
        assert!(!error.retryable);
        assert_eq!(server.await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn apify_dataset_and_custom_event_charge_keep_auth_and_idempotency() {
        let (base_url, server) =
            start_mock_server(vec![(201, "{}".to_owned()), (201, "{}".to_owned())]).await;
        let apify = mock_apify_client(base_url);
        let item = json!({"doctor_name":"Doctor"});
        apify.push_dataset_item(&item).await.unwrap();
        apify
            .charge_event(SCRAPPA_CHARGE_EVENT, "test-run-doctor-profile-result-1")
            .await
            .unwrap();
        let requests = server.await.unwrap();
        assert!(requests[0]
            .head
            .starts_with("POST /v2/datasets/dataset/items HTTP/1.1"));
        assert!(requests[0].head.contains("Bearer test-token"));
        assert_eq!(
            serde_json::from_str::<Value>(&requests[0].body).unwrap(),
            item
        );
        assert!(requests[1]
            .head
            .starts_with("POST /v2/actor-runs/test-run/charge HTTP/1.1"));
        assert!(requests[1]
            .head
            .to_ascii_lowercase()
            .contains("idempotency-key: test-run-doctor-profile-result-1"));
        assert_eq!(
            serde_json::from_str::<Value>(&requests[1].body).unwrap(),
            json!({"eventName":"doctor-profile-result","count":1})
        );
    }

    #[tokio::test]
    async fn spending_limit_skips_dataset_and_custom_event_when_result_does_not_fit() {
        let (base_url, server) = start_mock_server(vec![(
            200,
            mock_ppe_run(json!(0.001), json!({})).to_string(),
        )])
        .await;
        let apify = mock_apify_client(base_url);
        let mut pricing = None;
        let result = push_charged_item(&apify, &mut pricing, &json!({"doctor_name":"Doctor"}), 1)
            .await
            .unwrap();

        assert_eq!(result.saved_count, 0);
        assert_eq!(
            result.status_message.as_deref(),
            Some("Charge limit reached after saving 0 of 1 Jameda doctor profile results.")
        );
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0]
            .head
            .starts_with("GET /v2/actor-runs/test-run HTTP/1.1"));
    }

    #[tokio::test]
    async fn writes_input_output_and_terminal_run_status_to_apify() {
        let (base_url, server) = start_mock_server(vec![
            (200, json!({"doctorUrl":MARKUS_URL}).to_string()),
            (201, "{}".to_owned()),
            (200, "{}".to_owned()),
        ])
        .await;
        let apify = mock_apify_client(base_url);
        assert_eq!(
            apify.get_input().await.unwrap(),
            Some(json!({"doctorUrl":MARKUS_URL}))
        );
        apify
            .put_record("OUTPUT", &json!({"doctors_saved":1}))
            .await
            .unwrap();
        apify
            .set_terminal_status_message("partial results")
            .await
            .unwrap();
        let requests = server.await.unwrap();
        assert!(requests[0]
            .head
            .starts_with("GET /v2/key-value-stores/store/records/INPUT HTTP/1.1"));
        assert!(requests[1]
            .head
            .starts_with("PUT /v2/key-value-stores/store/records/OUTPUT HTTP/1.1"));
        assert_eq!(
            serde_json::from_str::<Value>(&requests[1].body).unwrap(),
            json!({"doctors_saved":1})
        );
        assert!(requests[2]
            .head
            .starts_with("PUT /v2/actor-runs/test-run HTTP/1.1"));
        assert_eq!(
            serde_json::from_str::<Value>(&requests[2].body).unwrap()["isStatusMessageTerminal"],
            true
        );
    }

    #[test]
    fn constructs_api_paths_without_dropping_a_base_path() {
        assert_eq!(
            endpoint_url(
                "http://127.0.0.1:8080/mock/",
                &["v2", "datasets", "dataset", "items"]
            )
            .unwrap()
            .as_str(),
            "http://127.0.0.1:8080/mock/v2/datasets/dataset/items"
        );
    }
}
