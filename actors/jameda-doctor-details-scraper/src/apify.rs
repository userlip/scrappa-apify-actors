use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use url::Url;

pub(crate) const APIFY_API_DEFAULT: &str = "https://api.apify.com";
pub(crate) const APIFY_MAX_RETRIES: usize = 2;
pub(crate) const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub(crate) const SCRAPPA_CHARGE_EVENT: &str = "doctor-profile-result";
pub(crate) const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub(crate) struct ApifyConfig {
    pub(crate) api_base: String,
    pub(crate) token: String,
    pub(crate) run_id: String,
    pub(crate) key_value_store_id: String,
    pub(crate) dataset_id: String,
    pub(crate) input_key: String,
}

impl ApifyConfig {
    pub(crate) fn from_env() -> Result<Self> {
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

pub(crate) struct ApifyClient {
    pub(crate) http: Client,
    pub(crate) config: ApifyConfig,
}

impl ApifyClient {
    pub(crate) fn new(config: ApifyConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .context("Failed to configure Apify API client")?;
        Ok(Self { http, config })
    }

    pub(crate) fn endpoint(&self, parts: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.api_base, parts)
    }

    pub(crate) async fn get_input(&self) -> Result<Option<Value>> {
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

    pub(crate) async fn get_record(&self, key: &str) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            key,
        ])?;
        let response = self.request_with_retry("GET", url).await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        require_apify_success(response, &format!("{key} record retrieval"))
            .await?
            .json::<Value>()
            .await
            .context("Apify key-value record was not valid JSON")
            .map(Some)
    }

    pub(crate) async fn get_run(&self) -> Result<Value> {
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

    pub(crate) async fn push_dataset_item(&self, item: &Value) -> Result<()> {
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

    pub(crate) async fn dataset_contains_doctor_url(&self, doctor_url: &str) -> Result<bool> {
        let mut url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        url.query_pairs_mut()
            .append_pair("format", "json")
            .append_pair("clean", "true")
            .append_pair("limit", "1000");
        let items = require_apify_success(
            self.request_with_retry("GET", url).await?,
            "dataset recovery lookup",
        )
        .await?
        .json::<Vec<Value>>()
        .await
        .context("Apify dataset items response was not a JSON array")?;
        Ok(items.iter().any(|item| {
            item.get("requested_doctor_url").and_then(Value::as_str) == Some(doctor_url)
        }))
    }

    pub(crate) async fn push_dataset_item_with_recovery(
        &self,
        item: &Value,
        doctor_url: &str,
    ) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let mut retry_count = 0;
        loop {
            if self.dataset_contains_doctor_url(doctor_url).await? {
                return Ok(());
            }

            let response = self
                .http
                .post(url.clone())
                .bearer_auth(&self.config.token)
                .header(header::ACCEPT, "application/json")
                .json(item)
                .send()
                .await;
            let response = match response {
                Ok(response) => response,
                Err(error) if retry_count < APIFY_MAX_RETRIES => {
                    let delay = Duration::from_secs((retry_count + 1) as u64);
                    eprintln!(
                        "Apify dataset publication outcome is uncertain ({error}); checking for the result before retrying."
                    );
                    tokio::time::sleep(delay).await;
                    retry_count += 1;
                    continue;
                }
                Err(error) => {
                    if self.dataset_contains_doctor_url(doctor_url).await? {
                        return Ok(());
                    }
                    return Err(error)
                        .context("Failed to publish charged dataset item to Apify API");
                }
            };

            if response.status().is_success() {
                return Ok(());
            }
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            if let Some(delay) = apify_retry_delay("POST_DATASET", status, retry_count) {
                eprintln!(
                    "Apify dataset publication failed ({}): {body}; checking for the result before retrying.",
                    status.as_u16()
                );
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }
            if self.dataset_contains_doctor_url(doctor_url).await? {
                return Ok(());
            }
            bail!(
                "Apify dataset item publication failed ({}): {body}",
                status.as_u16()
            );
        }
    }

    pub(crate) async fn charge_event(&self, event_name: &str, idempotency_key: &str) -> Result<()> {
        // The REST endpoint returns an empty 201 and does not enforce maxTotalChargeUsd;
        // PricingState preflights the combined custom-event and dataset-item cost first.
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
                .await;
            let response = match response {
                Ok(response) => response,
                Err(error) if retry_count < APIFY_MAX_RETRIES => {
                    let delay = Duration::from_secs((retry_count + 1) as u64);
                    eprintln!(
                        "Apify event charge outcome is uncertain ({error}); retrying with the same idempotency key."
                    );
                    tokio::time::sleep(delay).await;
                    retry_count += 1;
                    continue;
                }
                Err(error) => return Err(error).context("Apify event charge request failed"),
            };

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

    pub(crate) async fn put_record(&self, key: &str, value: &Value) -> Result<()> {
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

    pub(crate) async fn set_terminal_status_message(&self, message: &str) -> Result<()> {
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

    pub(crate) async fn request_with_retry(&self, method: &str, url: Url) -> Result<Response> {
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

#[derive(Debug)]
pub(crate) struct PricingState {
    is_pay_per_event: bool,
    event_prices: Map<String, Value>,
    charged_event_counts: Map<String, Value>,
    max_total_charge_usd: Option<f64>,
}

impl PricingState {
    pub(crate) fn from_run(run: &Value) -> Result<Self> {
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

    pub(crate) fn can_save_one_result(&self) -> Result<bool> {
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

    pub(crate) fn can_save_dataset_item(&self) -> Result<bool> {
        if !self.is_pay_per_event {
            return Ok(true);
        }
        let Some(max_total_charge_usd) = self.max_total_charge_usd else {
            return Ok(true);
        };
        let item_price = self.event_price(DEFAULT_DATASET_ITEM_EVENT)?;
        let spent = self.spent_so_far()?;
        let tolerance = f64::EPSILON * max_total_charge_usd.max(1.0);
        Ok(spent + item_price <= max_total_charge_usd + tolerance)
    }

    pub(crate) fn event_price(&self, event_name: &str) -> Result<f64> {
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

    pub(crate) fn spent_so_far(&self) -> Result<f64> {
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

    pub(crate) fn record_dataset_item(&mut self) {
        if self.event_prices.contains_key(DEFAULT_DATASET_ITEM_EVENT) {
            increment_event_count(&mut self.charged_event_counts, DEFAULT_DATASET_ITEM_EVENT);
        }
    }

    pub(crate) fn record_custom_charge(&mut self) {
        increment_event_count(&mut self.charged_event_counts, SCRAPPA_CHARGE_EVENT);
    }

    pub(crate) fn event_count(&self, event_name: &str) -> u64 {
        self.charged_event_counts
            .get(event_name)
            .and_then(Value::as_u64)
            .unwrap_or_default()
    }

    pub(crate) fn ensure_event_count_at_least(&mut self, event_name: &str, minimum: u64) {
        if self.event_prices.contains_key(event_name) && self.event_count(event_name) < minimum {
            self.charged_event_counts
                .insert(event_name.to_owned(), json!(minimum));
        }
    }
}

pub(crate) fn increment_event_count(counts: &mut Map<String, Value>, event_name: &str) {
    let count = counts
        .get(event_name)
        .and_then(Value::as_u64)
        .unwrap_or_default();
    counts.insert(event_name.to_owned(), json!(count + 1));
}

#[derive(Debug)]
pub(crate) struct PushChargedItemResult {
    pub(crate) saved_count: usize,
    pub(crate) status_message: Option<String>,
}

pub(crate) async fn resume_charged_item(
    apify: &ApifyClient,
    pricing: &mut Option<PricingState>,
    result_index: usize,
    doctor_url: &str,
) -> Result<Option<PushChargedItemResult>> {
    let journal_key = format!("PPE_RESULT_{result_index:04}");
    let Some(record) = apify.get_record(&journal_key).await? else {
        return Ok(None);
    };
    if pricing.is_none() {
        *pricing = Some(PricingState::from_run(&apify.get_run().await?)?);
    }
    let state = pricing.as_mut().expect("Pricing state initialized");
    if !state.is_pay_per_event {
        bail!("PPE recovery record {journal_key} exists for a non-PPE run");
    }
    recover_charged_item(apify, state, record, result_index, doctor_url)
        .await
        .map(Some)
}

async fn recover_charged_item(
    apify: &ApifyClient,
    state: &mut PricingState,
    mut record: Value,
    result_index: usize,
    doctor_url: &str,
) -> Result<PushChargedItemResult> {
    let journal_key = format!("PPE_RESULT_{result_index:04}");
    if record.get("doctor_url").and_then(Value::as_str) != Some(doctor_url) {
        bail!("PPE recovery record {journal_key} does not match the current doctor URL");
    }
    let status = record
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("PPE recovery record {journal_key} has no status"))?;
    let recovery_idempotency_key = record
        .get("idempotency_key")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("PPE recovery record {journal_key} has no idempotency key"))?
        .to_owned();
    let baseline_custom = record
        .pointer("/baseline_event_counts/doctor-profile-result")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            anyhow!("PPE recovery record {journal_key} has no custom-charge baseline")
        })?;
    let baseline_dataset = record
        .pointer("/baseline_event_counts/apify-default-dataset-item")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let recovery_item = record
        .get("item")
        .cloned()
        .ok_or_else(|| anyhow!("PPE recovery record {journal_key} has no dataset item"))?;
    state.ensure_event_count_at_least(SCRAPPA_CHARGE_EVENT, baseline_custom);
    state.ensure_event_count_at_least(DEFAULT_DATASET_ITEM_EVENT, baseline_dataset);

    if status == "saved" {
        state.ensure_event_count_at_least(SCRAPPA_CHARGE_EVENT, baseline_custom + 1);
        state.ensure_event_count_at_least(DEFAULT_DATASET_ITEM_EVENT, baseline_dataset + 1);
        return Ok(PushChargedItemResult {
            saved_count: 1,
            status_message: None,
        });
    }
    if status != "pending" && status != "charged" {
        bail!("PPE recovery record {journal_key} has unsupported status {status}");
    }

    if apify.dataset_contains_doctor_url(doctor_url).await? {
        state.ensure_event_count_at_least(SCRAPPA_CHARGE_EVENT, baseline_custom + 1);
        state.ensure_event_count_at_least(DEFAULT_DATASET_ITEM_EVENT, baseline_dataset + 1);
        record["status"] = json!("saved");
        if let Err(error) = apify.put_record(&journal_key, &record).await {
            eprintln!("Could not finalize PPE recovery record {journal_key}: {error}");
        }
        return Ok(PushChargedItemResult {
            saved_count: 1,
            status_message: None,
        });
    }

    let charge_already_recorded =
        status == "charged" || state.event_count(SCRAPPA_CHARGE_EVENT) > baseline_custom;
    if charge_already_recorded {
        state.ensure_event_count_at_least(SCRAPPA_CHARGE_EVENT, baseline_custom + 1);
        if !state.can_save_dataset_item()? {
            bail!("Charge limit prevents recovery of the charged Jameda doctor result for {doctor_url}");
        }
        record["status"] = json!("charged");
        if let Err(error) = apify.put_record(&journal_key, &record).await {
            eprintln!("Could not update PPE recovery record {journal_key}: {error}");
        }
    } else if !state.can_save_one_result()? {
        return Ok(charge_limit_reached());
    } else {
        apify
            .charge_event(SCRAPPA_CHARGE_EVENT, &recovery_idempotency_key)
            .await?;
        state.ensure_event_count_at_least(SCRAPPA_CHARGE_EVENT, baseline_custom + 1);
        record["status"] = json!("charged");
        if let Err(error) = apify.put_record(&journal_key, &record).await {
            eprintln!("Could not update PPE recovery record {journal_key}: {error}");
        }
    }

    apify
        .push_dataset_item_with_recovery(&recovery_item, doctor_url)
        .await?;
    state.ensure_event_count_at_least(DEFAULT_DATASET_ITEM_EVENT, baseline_dataset + 1);
    record["status"] = json!("saved");
    if let Err(error) = apify.put_record(&journal_key, &record).await {
        eprintln!("Could not finalize PPE recovery record {journal_key}: {error}");
    }
    Ok(PushChargedItemResult {
        saved_count: 1,
        status_message: None,
    })
}

pub(crate) async fn push_charged_item(
    apify: &ApifyClient,
    pricing: &mut Option<PricingState>,
    item: &Value,
    result_index: usize,
    doctor_url: &str,
    recovery_checked: bool,
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

    let journal_key = format!("PPE_RESULT_{result_index:04}");
    let idempotency_key = format!(
        "{}-{SCRAPPA_CHARGE_EVENT}-{result_index}",
        apify.config.run_id
    );
    if !recovery_checked {
        if let Some(record) = apify.get_record(&journal_key).await? {
            return recover_charged_item(apify, state, record, result_index, doctor_url).await;
        }
    }

    if !state.can_save_one_result()? {
        return Ok(charge_limit_reached());
    }

    let baseline_custom = state.event_count(SCRAPPA_CHARGE_EVENT);
    let baseline_dataset = state.event_count(DEFAULT_DATASET_ITEM_EVENT);
    let mut record = json!({
        "status": "pending",
        "doctor_url": doctor_url,
        "item": item,
        "idempotency_key": idempotency_key,
        "baseline_event_counts": {
            "doctor-profile-result": baseline_custom,
            "apify-default-dataset-item": baseline_dataset
        }
    });
    apify.put_record(&journal_key, &record).await?;
    apify
        .charge_event(SCRAPPA_CHARGE_EVENT, &idempotency_key)
        .await?;
    state.ensure_event_count_at_least(SCRAPPA_CHARGE_EVENT, baseline_custom + 1);

    record["status"] = json!("charged");
    if let Err(error) = apify.put_record(&journal_key, &record).await {
        eprintln!("Could not update PPE recovery record {journal_key}: {error}");
    }

    apify
        .push_dataset_item_with_recovery(item, doctor_url)
        .await?;
    state.ensure_event_count_at_least(DEFAULT_DATASET_ITEM_EVENT, baseline_dataset + 1);

    record["status"] = json!("saved");
    if let Err(error) = apify.put_record(&journal_key, &record).await {
        eprintln!("Could not finalize PPE recovery record {journal_key}: {error}");
    }

    Ok(PushChargedItemResult {
        saved_count: 1,
        status_message: None,
    })
}

fn charge_limit_reached() -> PushChargedItemResult {
    let status_message =
        "Charge limit reached after saving 0 of 1 Jameda doctor profile results.".to_owned();
    eprintln!(
        "{status_message} {{\"event\":\"{SCRAPPA_CHARGE_EVENT}\",\"charged_count\":0,\"requested_count\":1,\"saved_count\":0}}"
    );
    PushChargedItemResult {
        saved_count: 0,
        status_message: Some(status_message),
    }
}

pub(crate) fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

pub(crate) fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

pub(crate) fn endpoint_url(base: &str, path: &[&str]) -> Result<Url> {
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

pub(crate) async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

pub(crate) fn apify_retry_delay(
    method: &str,
    status: StatusCode,
    retry_count: usize,
) -> Option<Duration> {
    if !matches!(method, "GET" | "PUT" | "POST_CHARGE" | "POST_DATASET")
        || retry_count >= APIFY_MAX_RETRIES
        || (status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error())
    {
        return None;
    }
    Some(Duration::from_secs((retry_count + 1) as u64))
}
