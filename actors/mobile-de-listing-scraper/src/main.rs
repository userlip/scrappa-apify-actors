use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, bail};
use httpdate::parse_http_date;
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode, Url};
use serde_json::{Map, Number, Value, json};
use tokio::time::sleep;

const DEFAULT_SCRAPPA_URL: &str = "https://scrappa.co/api";
const DEFAULT_APIFY_URL: &str = "https://api.apify.com";
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(45);
const APIFY_RETRY_BUDGET: Duration = Duration::from_secs(150);
#[cfg(not(test))]
const DATASET_VERIFY_SETTLE: Duration = Duration::from_secs(2);
#[cfg(test)]
const DATASET_VERIFY_SETTLE: Duration = Duration::from_millis(2);
const DATASET_VERIFY_MAX_WAIT: Duration = Duration::from_secs(10);
const DATASET_VERIFY_TIMEOUT_MAX_WAIT: Duration = Duration::from_secs(30);
const ENTRY_TIME_BUDGET: Duration = Duration::from_secs(100);
const MAX_RETRY_BACKOFF: Duration = Duration::from_secs(15);
const MAX_SCRAPPA_RETRIES: usize = 7;
const MAX_BATCH_SIZE: usize = 100;
const MAX_DATASET_PUSH_ITEMS: usize = 500;
const MAX_DATASET_PUSH_BYTES: usize = 5 * 1024 * 1024;
const USER_AGENT: &str = "ScrappaApifyActor/1.0 (+https://scrappa.co)";

const SPEC_JSON: &str = include_str!("../spec.json");
#[cfg(test)]
const FIXTURE_JSON: &str = include_str!("../fixtures/response.json");

#[derive(Debug)]
struct ActorSpec {
    title: String,
    endpoint: String,
    endpoint_selectors: Vec<EndpointSelector>,
    mode: String,
    batch: BatchSpec,
    parameters: Vec<ParameterSpec>,
    result_pointer: Option<String>,
    fallback_result_pointers: Vec<String>,
    flatten_pointers: HashMap<String, String>,
    dedupe_pointer: Option<String>,
    relative_date_defaults: Vec<RelativeDateDefault>,
    pagination: Option<PaginationSpec>,
    enrichment: EnrichmentSpec,
    max_results: MaxResultsSpec,
    default_max_pages: usize,
}

#[derive(Debug)]
struct BatchSpec {
    field: String,
    value_field: String,
    api_param: String,
    path_param: Option<String>,
    item_required: Vec<String>,
    enrich_batch_item: bool,
}

#[derive(Debug)]
struct EndpointSelector {
    input: String,
    default: Option<String>,
    choices: Vec<EndpointChoice>,
}

#[derive(Debug)]
struct EndpointChoice {
    value: String,
    endpoint: String,
}

#[derive(Debug)]
struct RelativeDateDefault {
    input: String,
    base: String,
    offset_days: i64,
    endpoints: Vec<String>,
}

#[derive(Debug)]
struct ParameterSpec {
    input: String,
    api_param: String,
    location: String,
    required: bool,
    required_for_endpoints: Vec<String>,
    available_for_endpoints: Vec<String>,
}

#[derive(Debug)]
struct PaginationSpec {
    kind: String,
    param: String,
    start: Value,
    step: i64,
    next_pointer: Option<String>,
    next_pointers: Vec<String>,
    has_more_pointer: Option<String>,
    end_pointer: Option<String>,
    current_page_pointer: Option<String>,
    total_pages_pointer: Option<String>,
    max_pages: usize,
}

#[derive(Debug)]
struct EnrichmentSpec {
    field: String,
}

#[derive(Debug)]
struct MaxResultsSpec {
    input: String,
    default: usize,
    hard_limit: usize,
}

#[derive(Debug)]
struct RequestEntry {
    endpoint: String,
    value: Value,
    params: Map<String, Value>,
    path_params: Map<String, Value>,
}

#[derive(Debug, Default)]
struct PushResult {
    saved: usize,
    charge_limited: bool,
}

struct Storage {
    http: Client,
    local_root: Option<PathBuf>,
    apify_base: Url,
    token: Option<String>,
    key_value_store_id: Option<String>,
    dataset_id: Option<String>,
    run_id: Option<String>,
    input_key: String,
    remaining_items: usize,
    local_next_item: usize,
    dataset_written: Option<usize>,
}

impl Storage {
    fn from_env() -> Result<Self> {
        let local_mode = env::var("APIFY_LOCAL_MODE")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes"));
        let apify_base = Url::parse(
            &env::var("APIFY_API_PUBLIC_BASE_URL").unwrap_or_else(|_| DEFAULT_APIFY_URL.to_owned()),
        )
        .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?;
        let http = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .retry(reqwest::retry::never())
            .user_agent(USER_AGENT)
            .build()
            .context("Could not create the Apify HTTP client")?;

        if local_mode {
            let local_root = env::var_os("APIFY_LOCAL_STORAGE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("storage"));
            let dataset_dir = local_root.join("datasets/default");
            fs::create_dir_all(&dataset_dir)
                .context("Could not create the local Apify dataset directory")?;
            let local_next_item = fs::read_dir(&dataset_dir)
                .context("Could not read the local Apify dataset directory")?
                .filter_map(std::result::Result::ok)
                .filter_map(|entry| dataset_index_from_path(&entry.path()))
                .max()
                .unwrap_or(0);
            return Ok(Self {
                http,
                local_root: Some(local_root),
                apify_base,
                token: None,
                key_value_store_id: None,
                dataset_id: None,
                run_id: None,
                input_key: "INPUT".to_owned(),
                remaining_items: usize::MAX,
                local_next_item,
                dataset_written: Some(local_next_item),
            });
        }

        Ok(Self {
            http,
            local_root: None,
            apify_base,
            token: Some(required_env("APIFY_TOKEN")?),
            key_value_store_id: Some(required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?),
            dataset_id: Some(required_env("ACTOR_DEFAULT_DATASET_ID")?),
            run_id: Some(required_env("ACTOR_RUN_ID")?),
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "INPUT".to_owned()),
            remaining_items: usize::MAX,
            local_next_item: 0,
            dataset_written: None,
        })
    }

    async fn get_input(&self) -> Result<Value> {
        if let Some(root) = &self.local_root {
            let path = env::var_os("APIFY_LOCAL_INPUT_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| root.join("key_value_stores/default/INPUT.json"));
            let input = fs::read_to_string(&path)
                .with_context(|| format!("Could not read local input at {}", path.display()))?;
            return serde_json::from_str(&input).context("Local Actor input was not valid JSON");
        }

        let url = self.api_url(&[
            "v2",
            "key-value-stores",
            self.key_value_store_id.as_deref().unwrap_or_default(),
            "records",
            &self.input_key,
        ])?;
        let response = self
            .send_apify_with_retry("input retrieval", || {
                self.authorized(Method::GET, url.clone())
            })
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            bail!("Apify INPUT was not found in the default key-value store");
        }
        response_json(response, "read Apify INPUT").await
    }

    async fn initialize_budget(&mut self, requested: usize) -> Result<()> {
        if self.local_root.is_some() {
            self.remaining_items = requested;
            return Ok(());
        }

        let run_id = self.run_id.as_deref().unwrap_or_default();
        let url = self.api_url(&["v2", "actor-runs", run_id])?;
        let response = self
            .send_apify_with_retry("run pricing request", || {
                self.authorized(Method::GET, url.clone())
            })
            .await?;
        let run = response_json(response, "read Actor run pricing").await?;
        let dataset_written = self.current_dataset_item_count().await?;
        self.dataset_written = Some(dataset_written);
        let mut allowed = apify_dataset_budget(&run, requested)?;

        if let Some(max_paid_items) = configured_paid_item_limit()? {
            let pricing_model = run
                .pointer("/data/pricingInfo/pricingModel")
                .or_else(|| run.pointer("/pricingInfo/pricingModel"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let remaining_paid_items = if pricing_model == "PRICE_PER_DATASET_ITEM" {
                remaining_paid_items_for_model(
                    &run,
                    max_paid_items,
                    pricing_model,
                    Some(dataset_written),
                )?
            } else {
                remaining_paid_items_for_model(&run, max_paid_items, pricing_model, None)?
            };
            allowed = allowed.min(remaining_paid_items);
        }

        self.remaining_items = allowed;
        Ok(())
    }

    async fn push_items(&mut self, items: &[Value]) -> Result<PushResult> {
        let count = items.len().min(self.remaining_items);
        if count == 0 {
            return Ok(PushResult::default());
        }

        if let Some(root) = &self.local_root {
            let dataset_dir = root.join("datasets/default");
            fs::create_dir_all(&dataset_dir)
                .context("Could not create the local Apify dataset directory")?;
            for item in &items[..count] {
                self.local_next_item += 1;
                let path = dataset_dir.join(format!("{:09}.json", self.local_next_item));
                fs::write(&path, serde_json::to_vec(item)?).with_context(|| {
                    format!(
                        "Could not write local dataset item {}",
                        self.local_next_item
                    )
                })?;
            }
            self.remaining_items -= count;
            return Ok(PushResult {
                saved: count,
                charge_limited: false,
            });
        } else {
            let dataset_id = self.dataset_id.as_deref().unwrap_or_default();
            let url = self.api_url(&["v2", "datasets", dataset_id, "items"])?;
            let chunks = dataset_item_chunks(&items[..count])?;
            let mut saved = 0;
            for chunk in chunks {
                // After an unresolved write the count is unknown; re-read it instead of guessing 0.
                let written = match self.dataset_written {
                    Some(written) => written,
                    None => {
                        let written = self.current_dataset_item_count().await?;
                        self.dataset_written = Some(written);
                        written
                    }
                };
                let chunk_result = self
                    .push_dataset_chunk(&url, &chunk.items, &chunk.body, written)
                    .await;
                let chunk_result = match chunk_result {
                    Ok(result) => result,
                    Err(error) => {
                        self.dataset_written = None;
                        return Err(error);
                    }
                };
                match chunk_result {
                    DatasetChunkResult::Saved(chunk_saved) => {
                        saved += chunk_saved;
                        self.dataset_written = Some(written + chunk_saved);
                    }
                    DatasetChunkResult::ChargeLimited(chunk_saved) => {
                        saved += chunk_saved;
                        self.dataset_written = Some(written + chunk_saved);
                        eprintln!(
                            "Apify stopped accepting dataset items; ending the run without retrying the charge-limited push"
                        );
                        self.remaining_items = self.remaining_items.saturating_sub(saved);
                        return Ok(PushResult {
                            saved,
                            charge_limited: true,
                        });
                    }
                }
            }
            self.remaining_items -= saved;
            return Ok(PushResult {
                saved,
                charge_limited: false,
            });
        }
    }

    async fn current_dataset_item_count(&self) -> Result<usize> {
        let dataset_id = self.dataset_id.as_deref().unwrap_or_default();
        let mut url = self.api_url(&["v2", "datasets", dataset_id])?;
        url.query_pairs_mut().append_pair("fields", "itemCount");
        let response = self
            .send_apify_with_retry("dataset item count request", || {
                self.authorized(Method::GET, url.clone())
            })
            .await?;
        let dataset = response_json(response, "read dataset item count").await?;
        let mut count = dataset
            .pointer("/data/itemCount")
            .or_else(|| dataset.get("itemCount"))
            .and_then(Value::as_u64)
            .map(|count| count as usize)
            .ok_or_else(|| anyhow!("Apify dataset response did not include itemCount"))?;
        let mut checked_empty_at: Option<Instant> = None;
        loop {
            let mut items_url = self.api_url(&["v2", "datasets", dataset_id, "items"])?;
            items_url
                .query_pairs_mut()
                .append_pair("offset", &count.to_string())
                .append_pair("limit", "500");
            let response = self
                .send_apify_with_retry("dataset item count verification", || {
                    self.authorized(Method::GET, items_url.clone())
                })
                .await?;
            let items = response_json(response, "read dataset items at run start")
                .await?
                .as_array()
                .cloned()
                .ok_or_else(|| anyhow!("Apify dataset items response was not an array"))?;
            if items.is_empty() {
                if checked_empty_at
                    .is_none_or(|checked_at| checked_at.elapsed() < DATASET_VERIFY_SETTLE)
                {
                    checked_empty_at.get_or_insert_with(Instant::now);
                    dataset_settle_sleep(DATASET_VERIFY_SETTLE).await;
                    continue;
                }
                return Ok(count);
            }
            count += items.len();
            checked_empty_at = None;
        }
    }

    async fn push_dataset_chunk(
        &self,
        url: &Url,
        items: &[Value],
        initial_body: &[u8],
        offset: usize,
    ) -> Result<DatasetChunkResult> {
        let deadline = Instant::now() + APIFY_RETRY_BUDGET;
        let mut pending = items.to_vec();
        let mut next_body = Some(initial_body.to_vec());
        let mut saved_prefix = 0;
        let mut attempt = 0;

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                bail!("Apify dataset write retry budget exhausted");
            }
            let body = match next_body.take() {
                Some(body) if pending.len() == items.len() => body,
                _ => serde_json::to_vec(&pending).context("Could not encode dataset items")?,
            };
            let response = self
                .authorized(Method::POST, url.clone())
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .timeout(remaining.min(REQUEST_TIMEOUT))
                .body(body)
                .send()
                .await;

            match response {
                Ok(response) if response.status().is_success() => {
                    return Ok(DatasetChunkResult::Saved(saved_prefix + pending.len()));
                }
                Ok(response) => {
                    let status = response.status();
                    let response_body = response.text().await.unwrap_or_default();
                    if charge_limit_response(status, &response_body) {
                        return Ok(DatasetChunkResult::ChargeLimited(saved_prefix));
                    }
                    if status != StatusCode::TOO_MANY_REQUESTS && !should_retry_apify(status) {
                        bail!("Apify returned HTTP {status} while writing dataset items");
                    }
                    if status != StatusCode::TOO_MANY_REQUESTS {
                        eprintln!(
                            "Apify dataset write returned HTTP {status}; verifying before retry"
                        );
                    }

                    if status != StatusCode::TOO_MANY_REQUESTS {
                        match dataset_retry_resolution(
                            self.verify_dataset_chunk(
                                offset + saved_prefix,
                                &pending,
                                deadline,
                                DATASET_VERIFY_MAX_WAIT,
                            )
                            .await?,
                        ) {
                            DatasetRetryResolution::AlreadyWritten => {
                                return Ok(DatasetChunkResult::Saved(saved_prefix + pending.len()));
                            }
                            DatasetRetryResolution::RetryRemainder(prefix) => {
                                saved_prefix += prefix;
                                pending.drain(..prefix);
                            }
                            DatasetRetryResolution::RetryFull => {}
                            DatasetRetryResolution::Mismatch => {
                                bail!("Apify dataset contents did not match the ambiguous write");
                            }
                        }
                    }
                }
                Err(error) => {
                    let definitely_not_sent = error.is_connect() && !error.is_timeout();
                    if !definitely_not_sent {
                        let verify_wait = dataset_verification_window(error.is_timeout());
                        match dataset_retry_resolution(
                            self.verify_dataset_chunk(
                                offset + saved_prefix,
                                &pending,
                                deadline,
                                verify_wait,
                            )
                            .await?,
                        ) {
                            DatasetRetryResolution::AlreadyWritten => {
                                return Ok(DatasetChunkResult::Saved(saved_prefix + pending.len()));
                            }
                            DatasetRetryResolution::RetryRemainder(prefix) => {
                                saved_prefix += prefix;
                                pending.drain(..prefix);
                            }
                            DatasetRetryResolution::RetryFull => {}
                            DatasetRetryResolution::Mismatch => {
                                bail!("Apify dataset contents did not match the ambiguous write");
                            }
                        }
                    }
                    eprintln!("Apify dataset write failed ({})", error.without_url());
                }
            }

            if pending.is_empty() {
                return Ok(DatasetChunkResult::Saved(saved_prefix));
            }
            let delay = retry_delay(attempt);
            let remaining = deadline.saturating_duration_since(Instant::now());
            if delay >= remaining {
                bail!("Apify dataset write retry budget exhausted");
            }
            eprintln!(
                "Apify dataset write attempt failed; retrying after {}ms",
                delay.as_millis()
            );
            apify_retry_sleep(delay).await;
            attempt += 1;
        }
    }

    async fn verify_dataset_chunk(
        &self,
        offset: usize,
        expected: &[Value],
        deadline: Instant,
        wait_limit: Duration,
    ) -> Result<DatasetVerification> {
        let dataset_id = self.dataset_id.as_deref().unwrap_or_default();
        let mut url = self.api_url(&["v2", "datasets", dataset_id, "items"])?;
        url.query_pairs_mut()
            .append_pair("offset", &offset.to_string())
            .append_pair("limit", &expected.len().to_string());
        let stop_checking_at = (Instant::now() + wait_limit).min(deadline);
        let mut previous_prefix = None;
        let mut stable_prefix = None;
        // Once any rows of this chunk were seen, a full re-post could duplicate them.
        let mut saw_rows = false;

        loop {
            if Instant::now() >= stop_checking_at {
                return Ok(match stable_prefix {
                    Some(count) => DatasetVerification::Prefix(count),
                    None if saw_rows => DatasetVerification::Mismatch,
                    None => DatasetVerification::None,
                });
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                bail!("Apify dataset verification retry budget exhausted");
            }
            let response = self
                .send_apify_with_retry_until(
                    "dataset item verification",
                    || self.authorized(Method::GET, url.clone()),
                    stop_checking_at,
                )
                .await?;
            let rows = response
                .json::<Vec<Value>>()
                .await
                .context("Apify dataset verification response was invalid")?;
            let verification = compare_dataset_prefix(expected, &rows);
            match verification {
                DatasetVerification::Complete | DatasetVerification::Mismatch => {
                    return Ok(verification);
                }
                DatasetVerification::Prefix(count) => {
                    saw_rows = true;
                    stable_prefix = (previous_prefix == Some(count)).then_some(count);
                    previous_prefix = Some(count);
                    if cfg!(test) && stable_prefix == Some(count) {
                        return Ok(DatasetVerification::Prefix(count));
                    }
                }
                DatasetVerification::None => {
                    previous_prefix = None;
                    stable_prefix = None;
                }
            }
            if cfg!(test) && previous_prefix.is_none() {
                return Ok(if saw_rows {
                    DatasetVerification::Mismatch
                } else {
                    DatasetVerification::None
                });
            }

            if Instant::now() >= stop_checking_at {
                return Ok(match stable_prefix {
                    Some(count) => DatasetVerification::Prefix(count),
                    None if saw_rows => DatasetVerification::Mismatch,
                    None => DatasetVerification::None,
                });
            }
            let settle = DATASET_VERIFY_SETTLE
                .min(stop_checking_at.saturating_duration_since(Instant::now()));
            dataset_settle_sleep(settle).await;
        }
    }

    async fn set_status_message(&self, message: &str) -> Result<()> {
        if self.local_root.is_some() {
            println!("Status: {message}");
            return Ok(());
        }

        let run_id = self.run_id.as_deref().unwrap_or_default();
        let url = self.api_url(&["v2", "actor-runs", run_id])?;
        let response = self
            .send_apify_with_retry("status message update", || {
                self.authorized(Method::PUT, url.clone())
                    .json(&json!({"statusMessage": message}))
            })
            .await?;
        require_success(response, "update Actor status message").await?;
        Ok(())
    }

    fn authorized(&self, method: Method, url: Url) -> RequestBuilder {
        let request = self
            .http
            .request(method, url)
            .header(reqwest::header::ACCEPT, "application/json");
        match &self.token {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    fn api_url(&self, segments: &[&str]) -> Result<Url> {
        append_segments(&self.apify_base, segments)
    }

    async fn send_apify_with_retry<F>(&self, operation: &str, mut request: F) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        self.send_apify_with_retry_until(
            operation,
            &mut request,
            Instant::now() + APIFY_RETRY_BUDGET,
        )
        .await
    }

    async fn send_apify_with_retry_until<F>(
        &self,
        operation: &str,
        mut request: F,
        deadline: Instant,
    ) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        let mut attempt = 0;
        let mut attempts = 0;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() || (cfg!(test) && attempts >= 3) {
                bail!("Apify {operation} retry budget exhausted");
            }
            attempts += 1;
            match request()
                .timeout(remaining.min(REQUEST_TIMEOUT))
                .send()
                .await
            {
                Ok(response) if should_retry_apify(response.status()) => {
                    let delay = retry_delay(attempt);
                    if delay >= deadline.saturating_duration_since(Instant::now()) {
                        bail!("Apify {operation} retry budget exhausted");
                    }
                    eprintln!(
                        "Apify {operation} returned {}; retrying after {}ms",
                        response.status(),
                        delay.as_millis()
                    );
                    apify_retry_sleep(delay).await;
                    attempt += 1;
                }
                Ok(response) => return Ok(response),
                Err(error) => {
                    let delay = retry_delay(attempt);
                    if delay >= deadline.saturating_duration_since(Instant::now()) {
                        bail!("Apify {operation} retry budget exhausted");
                    }
                    eprintln!(
                        "Apify {operation} failed ({}); retrying after {}ms",
                        error.without_url(),
                        delay.as_millis()
                    );
                    apify_retry_sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum DatasetVerification {
    Complete,
    Prefix(usize),
    None,
    Mismatch,
}

#[derive(Debug, PartialEq, Eq)]
enum DatasetRetryResolution {
    AlreadyWritten,
    RetryFull,
    RetryRemainder(usize),
    Mismatch,
}

enum DatasetChunkResult {
    Saved(usize),
    ChargeLimited(usize),
}

fn compare_dataset_prefix(expected: &[Value], actual: &[Value]) -> DatasetVerification {
    if actual.len() > expected.len() {
        return DatasetVerification::Mismatch;
    }
    if actual
        .iter()
        .zip(expected)
        .any(|(actual, expected)| !json_values_equal(actual, expected))
    {
        return DatasetVerification::Mismatch;
    }
    if actual.len() == expected.len() {
        return DatasetVerification::Complete;
    }
    if actual.is_empty() {
        return DatasetVerification::None;
    }
    DatasetVerification::Prefix(actual.len())
}

fn json_values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Number(left), Value::Number(right)) => json_numbers_equal(left, right),
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| json_values_equal(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|other| json_values_equal(value, other))
                })
        }
        _ => false,
    }
}

fn json_numbers_equal(left: &Number, right: &Number) -> bool {
    match (integer_value(left), integer_value(right)) {
        (Some(left), Some(right)) => left == right,
        (Some(integer), None) => integer_as_exact_f64(integer) == right.as_f64(),
        (None, Some(integer)) => left.as_f64() == integer_as_exact_f64(integer),
        (None, None) => left.as_f64() == right.as_f64(),
    }
}

fn integer_value(number: &Number) -> Option<i128> {
    number
        .as_i64()
        .map(i128::from)
        .or_else(|| number.as_u64().map(i128::from))
}

fn integer_as_exact_f64(integer: i128) -> Option<f64> {
    let magnitude = integer.unsigned_abs();
    if magnitude == 0 {
        return Some(0.0);
    }

    let significant_bits = u128::BITS - magnitude.leading_zeros() - magnitude.trailing_zeros();
    (significant_bits <= f64::MANTISSA_DIGITS).then_some(integer as f64)
}

fn dataset_retry_resolution(verification: DatasetVerification) -> DatasetRetryResolution {
    match verification {
        DatasetVerification::Complete => DatasetRetryResolution::AlreadyWritten,
        DatasetVerification::Prefix(count) => DatasetRetryResolution::RetryRemainder(count),
        DatasetVerification::None => DatasetRetryResolution::RetryFull,
        DatasetVerification::Mismatch => DatasetRetryResolution::Mismatch,
    }
}

fn dataset_verification_window(is_timeout: bool) -> Duration {
    if is_timeout {
        DATASET_VERIFY_TIMEOUT_MAX_WAIT
    } else {
        DATASET_VERIFY_MAX_WAIT
    }
}

struct DatasetChunk {
    body: Vec<u8>,
    items: Vec<Value>,
}

fn dataset_item_chunks(items: &[Value]) -> Result<Vec<DatasetChunk>> {
    let mut chunks = Vec::new();
    let mut current = Vec::<(Vec<u8>, Value)>::new();
    let mut current_bytes = 2_usize;

    for item in items {
        let encoded = serde_json::to_vec(item).context("Could not serialize a dataset item")?;
        if encoded.len() + 2 > MAX_DATASET_PUSH_BYTES {
            bail!("A dataset item exceeds the 5 MB Apify push limit");
        }
        let separator_bytes = usize::from(!current.is_empty());
        if !current.is_empty()
            && (current.len() >= MAX_DATASET_PUSH_ITEMS
                || current_bytes + separator_bytes + encoded.len() > MAX_DATASET_PUSH_BYTES)
        {
            chunks.push(encode_dataset_chunk(&current));
            current.clear();
            current_bytes = 2;
        }
        current_bytes += usize::from(!current.is_empty()) + encoded.len();
        current.push((encoded, item.clone()));
    }

    if !current.is_empty() {
        chunks.push(encode_dataset_chunk(&current));
    }
    Ok(chunks)
}

fn encode_dataset_chunk(items: &[(Vec<u8>, Value)]) -> DatasetChunk {
    let byte_count = items
        .iter()
        .map(|(encoded, _)| encoded.len())
        .sum::<usize>()
        + items.len()
        + 1;
    let mut body = Vec::with_capacity(byte_count);
    body.push(b'[');
    for (index, (encoded, _)) in items.iter().enumerate() {
        if index > 0 {
            body.push(b',');
        }
        body.extend_from_slice(encoded);
    }
    body.push(b']');
    DatasetChunk {
        body,
        items: items.iter().map(|(_, value)| value.clone()).collect(),
    }
}

fn charge_limit_response(status: StatusCode, body: &str) -> bool {
    if status == StatusCode::PAYMENT_REQUIRED || status == StatusCode::FORBIDDEN {
        return true;
    }
    let body = body.to_ascii_lowercase();
    [
        "charge limit",
        "spending limit",
        "maximum charge",
        "max total charge",
        "max-total-charge-exceeded",
    ]
    .iter()
    .any(|phrase| body.contains(phrase))
}

fn dataset_index_from_path(path: &Path) -> Option<usize> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
        return None;
    }
    path.file_stem()?.to_str()?.parse::<usize>().ok()
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn parse_spec() -> Result<ActorSpec> {
    let value: Value =
        serde_json::from_str(SPEC_JSON).context("Compiled Actor spec is invalid JSON")?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("Compiled Actor spec must be an object"))?;
    let string = |name: &str| -> Result<String> {
        object
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("Compiled Actor spec is missing {name}"))
    };
    let number = |name: &str, fallback: usize| {
        object
            .get(name)
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(fallback)
    };
    let batch = object
        .get("batch")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Compiled Actor spec is missing batch configuration"))?;
    let batch_string = |name: &str| -> Result<String> {
        batch
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("Compiled Actor batch spec is missing {name}"))
    };
    let parameters = object
        .get("parameters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|parameter| {
            let parameter_object = parameter
                .as_object()
                .ok_or_else(|| anyhow!("Compiled Actor parameter must be an object"))?;
            Ok(ParameterSpec {
                input: parameter_object
                    .get("input")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("Compiled Actor parameter is missing input"))?
                    .to_owned(),
                api_param: parameter_object
                    .get("apiParam")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("Compiled Actor parameter is missing apiParam"))?
                    .to_owned(),
                location: parameter_object
                    .get("location")
                    .and_then(Value::as_str)
                    .unwrap_or("query")
                    .to_owned(),
                required: parameter_object
                    .get("required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                required_for_endpoints: parameter_object
                    .get("requiredForEndpoints")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
                available_for_endpoints: parameter_object
                    .get("availableForEndpoints")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut endpoint_selectors = Vec::new();
    if let Some(selectors) = object.get("endpointSelectors") {
        let selectors = selectors
            .as_array()
            .ok_or_else(|| anyhow!("Compiled Actor endpointSelectors must be an array"))?;
        endpoint_selectors = selectors
            .iter()
            .map(parse_endpoint_selector)
            .collect::<Result<Vec<_>>>()?;
    } else if let Some(selector) = object.get("endpointSelector") {
        endpoint_selectors.push(parse_endpoint_selector(selector)?);
    } else if let Some(legacy_selectors) = object.get("endpointByInput").and_then(Value::as_object)
    {
        for (input, choices) in legacy_selectors {
            let choices = choices
                .as_object()
                .ok_or_else(|| anyhow!("Endpoint choices for {input} must be an object"))?
                .iter()
                .map(|(value, endpoint)| {
                    let endpoint = endpoint
                        .as_str()
                        .ok_or_else(|| anyhow!("Endpoint choice must be a string"))?;
                    Ok(EndpointChoice {
                        value: value.clone(),
                        endpoint: endpoint.to_owned(),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            endpoint_selectors.push(EndpointSelector {
                input: input.clone(),
                default: object
                    .get("endpointByInputDefault")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                choices,
            });
        }
    }
    if endpoint_selectors.len() > 1 {
        bail!("Compiled Actor specs may define only one endpoint selector");
    }
    let pagination = object
        .get("pagination")
        .filter(|value| !value.is_null())
        .map(|value| {
            let pagination = value
                .as_object()
                .ok_or_else(|| anyhow!("Compiled Actor pagination must be an object"))?;
            let optional_string = |name: &str| {
                pagination
                    .get(name)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            };
            Ok::<PaginationSpec, anyhow::Error>(PaginationSpec {
                kind: pagination
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("page")
                    .to_owned(),
                param: pagination
                    .get("param")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("Compiled Actor pagination is missing param"))?
                    .to_owned(),
                start: pagination.get("start").cloned().unwrap_or(Value::from(1)),
                step: pagination.get("step").and_then(Value::as_i64).unwrap_or(1),
                next_pointer: optional_string("nextPointer"),
                next_pointers: pagination
                    .get("nextPointers")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
                has_more_pointer: optional_string("hasMorePointer"),
                end_pointer: optional_string("endPointer"),
                current_page_pointer: optional_string("currentPagePointer"),
                total_pages_pointer: optional_string("totalPagesPointer"),
                max_pages: pagination
                    .get("maxPages")
                    .and_then(Value::as_u64)
                    .unwrap_or(1) as usize,
            })
        })
        .transpose()?;
    let enrichment = object
        .get("enrichment")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Compiled Actor spec is missing enrichment"))?;
    let flatten_pointers = object
        .get("flattenPointers")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(name, pointer)| {
            pointer
                .as_str()
                .map(|value| (name.clone(), value.to_owned()))
        })
        .collect();
    let relative_date_defaults = object
        .get("relativeDateDefaults")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|rule| {
            Ok(RelativeDateDefault {
                input: rule
                    .get("input")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("Relative date default is missing input"))?
                    .to_owned(),
                base: rule
                    .get("base")
                    .and_then(Value::as_str)
                    .unwrap_or("today")
                    .to_owned(),
                offset_days: rule.get("offsetDays").and_then(Value::as_i64).unwrap_or(0),
                endpoints: rule
                    .get("endpoints")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let max_results = object
        .get("maxResults")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Compiled Actor spec is missing maxResults"))?;

    Ok(ActorSpec {
        title: string("title")?,
        endpoint: string("endpoint")?,
        endpoint_selectors,
        mode: string("mode")?,
        batch: BatchSpec {
            field: batch_string("field")?,
            value_field: batch_string("valueField")?,
            api_param: batch_string("apiParam")?,
            path_param: batch
                .get("pathParam")
                .and_then(Value::as_str)
                .map(str::to_owned),
            item_required: batch
                .get("itemRequired")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            enrich_batch_item: batch
                .get("enrichBatchItem")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        },
        parameters,
        result_pointer: object
            .get("resultPointer")
            .and_then(Value::as_str)
            .map(str::to_owned),
        fallback_result_pointers: object
            .get("fallbackResultPointers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        flatten_pointers,
        dedupe_pointer: object
            .get("dedupePointer")
            .and_then(Value::as_str)
            .map(str::to_owned),
        relative_date_defaults,
        pagination,
        enrichment: EnrichmentSpec {
            field: enrichment
                .get("field")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("Compiled Actor spec is missing enrichment field"))?
                .to_owned(),
        },
        max_results: MaxResultsSpec {
            input: max_results
                .get("input")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("Compiled Actor spec is missing maxResults input"))?
                .to_owned(),
            default: max_results
                .get("default")
                .and_then(Value::as_u64)
                .unwrap_or(100) as usize,
            hard_limit: max_results
                .get("hardLimit")
                .and_then(Value::as_u64)
                .unwrap_or(1000) as usize,
        },
        default_max_pages: number("defaultMaxPages", 1),
    })
}

fn env_or_default(name: &str, fallback: &str) -> String {
    env::var(name).unwrap_or_else(|_| fallback.to_owned())
}

fn configured_paid_item_limit() -> Result<Option<usize>> {
    match env::var("ACTOR_MAX_PAID_DATASET_ITEMS") {
        Ok(value) => {
            let limit = value
                .parse::<usize>()
                .context("ACTOR_MAX_PAID_DATASET_ITEMS must be a non-negative integer")?;
            Ok(Some(limit))
        }
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error).context("ACTOR_MAX_PAID_DATASET_ITEMS was not valid Unicode"),
    }
}

fn parse_endpoint_selector(value: &Value) -> Result<EndpointSelector> {
    let selector = value
        .as_object()
        .ok_or_else(|| anyhow!("Compiled Actor endpoint selector must be an object"))?;
    let input = selector
        .get("input")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Compiled Actor endpoint selector is missing input"))?
        .to_owned();
    let choices = selector
        .get("choices")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|choice| {
            Ok(EndpointChoice {
                value: choice
                    .get("value")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("Compiled endpoint choice is missing value"))?
                    .to_owned(),
                endpoint: choice
                    .get("endpoint")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("Compiled endpoint choice is missing endpoint"))?
                    .to_owned(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(EndpointSelector {
        input,
        default: selector
            .get("default")
            .and_then(Value::as_str)
            .map(str::to_owned),
        choices,
    })
}

fn remaining_paid_dataset_items(run: &Value, max_paid_items: usize) -> Result<usize> {
    let data = run.get("data").unwrap_or(run);
    let charged_items = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .and_then(|counts| counts.get(DATASET_ITEM_EVENT))
        .map(|count| {
            count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {DATASET_ITEM_EVENT}"))
        })
        .transpose()?
        .unwrap_or(0) as usize;

    Ok(max_paid_items.saturating_sub(charged_items))
}

fn remaining_paid_items_for_model(
    run: &Value,
    max_paid_items: usize,
    pricing_model: &str,
    dataset_item_count: Option<usize>,
) -> Result<usize> {
    if pricing_model == "PRICE_PER_DATASET_ITEM" {
        let item_count = dataset_item_count
            .ok_or_else(|| anyhow!("Apify dataset itemCount is required for this pricing model"))?;
        return Ok(max_paid_items.saturating_sub(item_count));
    }
    remaining_paid_dataset_items(run, max_paid_items)
}

fn apify_dataset_budget(run: &Value, requested: usize) -> Result<usize> {
    let environment_max_charge = match env::var("ACTOR_MAX_TOTAL_CHARGE_USD") {
        Ok(value) => Some(
            value
                .parse::<f64>()
                .context("ACTOR_MAX_TOTAL_CHARGE_USD must be a number")?,
        ),
        Err(env::VarError::NotPresent) => None,
        Err(error) => {
            return Err(error).context("ACTOR_MAX_TOTAL_CHARGE_USD was not valid Unicode");
        }
    };
    apify_dataset_budget_with_limit(run, requested, environment_max_charge)
}

fn apify_dataset_budget_with_limit(
    run: &Value,
    requested: usize,
    environment_max_charge: Option<f64>,
) -> Result<usize> {
    let data = run.get("data").unwrap_or(run);
    let pricing = data.get("pricingInfo").unwrap_or(&Value::Null);
    let run_max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .filter(|value| !value.is_null())
        .map(parse_charge_limit)
        .transpose()?;
    for limit in [environment_max_charge, run_max_charge]
        .into_iter()
        .flatten()
    {
        if !limit.is_finite() || limit < 0.0 {
            bail!("Apify run returned an invalid maximum charge");
        }
    }
    let max_charge = match (environment_max_charge, run_max_charge) {
        (Some(environment), Some(run)) => Some(environment.min(run)),
        (Some(environment), None) => Some(environment),
        (None, Some(run)) => Some(run),
        (None, None) => None,
    };
    let Some(max_charge) = max_charge else {
        return Ok(requested);
    };

    let pricing_model = pricing
        .get("pricingModel")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if pricing_model != "PAY_PER_EVENT" {
        bail!(
            "Cannot safely apply ACTOR_MAX_TOTAL_CHARGE_USD for Apify pricing model {pricing_model}"
        );
    }

    let events = pricing
        .pointer("/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run pricing did not include event prices"))?;
    let item_price = events
        .get(DATASET_ITEM_EVENT)
        .map(|event| event_price_for_budget(event, DATASET_ITEM_EVENT))
        .transpose()?
        .ok_or_else(|| {
            anyhow!("Apify run pricing did not include the default dataset item event")
        })?;
    if item_price == 0.0 {
        return Ok(requested);
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut spent = 0.0;
    for (event_name, count) in &counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Apify run provided an invalid event count"))?;
        if count == 0 {
            continue;
        }
        let event = events
            .get(event_name)
            .ok_or_else(|| anyhow!("Apify run did not provide a price for a charged event"))?;
        spent += event_price_for_budget(event, event_name)? * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((0..=requested)
        .rev()
        .find(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .unwrap_or(0))
}

fn event_price_for_budget(event: &Value, event_name: &str) -> Result<f64> {
    let tiered_prices = event
        .get("eventTieredPricingUsd")
        .and_then(Value::as_object)
        .filter(|prices| !prices.is_empty());
    if let Some(tiered_prices) = tiered_prices {
        return tiered_prices
            .iter()
            .map(|(tier, tiered_price)| {
                let price = tiered_price
                    .get("tieredEventPriceUsd")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| {
                        anyhow!("Apify run provided an invalid {tier} price for {event_name}")
                    })?;
                if !price.is_finite() || price < 0.0 {
                    bail!("Apify run provided an invalid {tier} price for {event_name}");
                }
                Ok(price)
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .reduce(f64::max)
            .ok_or_else(|| {
                anyhow!("Apify run did not provide a usable price for event {event_name}")
            });
    }

    if let Some(price) = event.get("eventPriceUsd").and_then(Value::as_f64) {
        if !price.is_finite() || price < 0.0 {
            bail!("Apify run returned an invalid price for event {event_name}");
        }
        return Ok(price);
    }
    bail!("Apify run did not provide a usable price for event {event_name}")
}

fn parse_charge_limit(value: &Value) -> Result<f64> {
    let limit = match value {
        Value::Number(number) => number.as_f64(),
        Value::String(value) => value.trim().parse::<f64>().ok(),
        _ => None,
    }
    .ok_or_else(|| anyhow!("Apify run returned an invalid maximum charge"))?;
    if !limit.is_finite() || limit < 0.0 {
        bail!("Apify run returned an invalid maximum charge");
    }
    Ok(limit)
}

fn parse_requests(input: &Value, spec: &ActorSpec) -> Result<Vec<RequestEntry>> {
    let object = input
        .as_object()
        .ok_or_else(|| anyhow!("Actor input must be a JSON object"))?;
    let entries = object
        .get(&spec.batch.field)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Input field {} must be an array", spec.batch.field))?;
    if entries.is_empty() {
        bail!("Provide at least one entry in {}", spec.batch.field);
    }
    if entries.len() > MAX_BATCH_SIZE {
        bail!(
            "{} accepts at most {MAX_BATCH_SIZE} batch entries per run",
            spec.title
        );
    }

    let endpoint = selected_endpoint(input, spec)?;
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let batch_value = batch_value(entry, &spec.batch.value_field).ok_or_else(|| {
                anyhow!(
                    "{} entry {} is missing {}",
                    spec.batch.field,
                    index + 1,
                    spec.batch.value_field
                )
            })?;
            if batch_value.is_null() || batch_value.as_str().is_some_and(str::is_empty) {
                bail!(
                    "{} entry {} has an empty {}",
                    spec.batch.field,
                    index + 1,
                    spec.batch.value_field
                );
            }

            let mut input_values = object.clone();
            if let Some(entry_object) = entry.as_object() {
                for (name, value) in entry_object {
                    input_values.insert(name.clone(), value.clone());
                }
            }
            apply_relative_date_defaults(
                &mut input_values,
                &spec.relative_date_defaults,
                &endpoint,
            )?;
            for field in &spec.batch.item_required {
                if input_values
                    .get(field)
                    .is_none_or(|value| value.is_null() || value.as_str() == Some(""))
                {
                    bail!(
                        "{} entry {} requires {}",
                        spec.batch.field,
                        index + 1,
                        field
                    );
                }
            }

            let mut params = Map::new();
            let mut path_params = Map::new();
            for parameter in &spec.parameters {
                if !parameter.available_for_endpoints.is_empty()
                    && !parameter
                        .available_for_endpoints
                        .iter()
                        .any(|fragment| endpoint.contains(fragment))
                {
                    continue;
                }
                let source = entry
                    .as_object()
                    .and_then(|entry_object| entry_object.get(&parameter.input))
                    .or_else(|| input_values.get(&parameter.input));
                let source = if parameter.input == spec.batch.value_field {
                    Some(&batch_value)
                } else {
                    source
                };
                let Some(source) = source.filter(|value| !value.is_null()) else {
                    continue;
                };
                if parameter.location == "path" {
                    path_params.insert(parameter.api_param.clone(), source.clone());
                } else {
                    params.insert(parameter.api_param.clone(), source.clone());
                }
            }
            if let Some(path_param) = &spec.batch.path_param {
                path_params.insert(path_param.clone(), batch_value.clone());
            } else if !spec.batch.api_param.is_empty() {
                params.insert(spec.batch.api_param.clone(), batch_value.clone());
            }

            validate_required_params(&params, &path_params, &endpoint, spec, index)?;
            Ok(RequestEntry {
                endpoint: endpoint.clone(),
                value: if spec.batch.enrich_batch_item {
                    entry.clone()
                } else {
                    batch_value
                },
                params,
                path_params,
            })
        })
        .collect()
}

fn apply_relative_date_defaults(
    values: &mut Map<String, Value>,
    defaults: &[RelativeDateDefault],
    endpoint: &str,
) -> Result<()> {
    let today = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86_400) as i64;

    for default in defaults {
        if !default.endpoints.is_empty()
            && !default
                .endpoints
                .iter()
                .any(|fragment| endpoint.contains(fragment))
        {
            continue;
        }
        if values
            .get(&default.input)
            .is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
        {
            continue;
        }
        let base_day = if default.base == "today" {
            today
        } else {
            let base_value = values
                .get(&default.base)
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    anyhow!(
                        "Relative date default for {} requires {}",
                        default.input,
                        default.base
                    )
                })?;
            parse_iso_date(base_value)
                .ok_or_else(|| anyhow!("{} must be a date in YYYY-MM-DD format", default.base))?
        };
        let date = format_iso_date(base_day.saturating_add(default.offset_days));
        values.insert(default.input.clone(), Value::String(date));
    }
    Ok(())
}

fn parse_iso_date(date: &str) -> Option<i64> {
    let mut parts = date.split('-');
    let year = parts.next()?.parse::<i64>().ok()?;
    let month = parts.next()?.parse::<i64>().ok()?;
    let day = parts.next()?.parse::<i64>().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    (format_iso_date(days) == date).then_some(days)
}

fn format_iso_date(days: i64) -> String {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn batch_value(entry: &Value, value_field: &str) -> Option<Value> {
    match entry {
        Value::Object(object) => object.get(value_field).cloned(),
        Value::String(_) | Value::Number(_) => Some(entry.clone()),
        _ => None,
    }
}

fn validate_required_params(
    params: &Map<String, Value>,
    path_params: &Map<String, Value>,
    endpoint: &str,
    spec: &ActorSpec,
    index: usize,
) -> Result<()> {
    for parameter in &spec.parameters {
        let available = parameter.available_for_endpoints.is_empty()
            || parameter
                .available_for_endpoints
                .iter()
                .any(|fragment| endpoint.contains(fragment));
        if !available {
            continue;
        }
        let required = if parameter.required_for_endpoints.is_empty() {
            parameter.required
        } else {
            parameter
                .required_for_endpoints
                .iter()
                .any(|fragment| endpoint.contains(fragment))
        };
        if !required {
            continue;
        }
        if parameter.location == "path" {
            if path_params
                .get(&parameter.api_param)
                .is_none_or(Value::is_null)
            {
                bail!(
                    "Batch entry {} requires path parameter {}",
                    index + 1,
                    parameter.api_param
                );
            }
        } else if params.get(&parameter.api_param).is_none_or(Value::is_null) {
            // Batch primary values are inserted after parameter mappings.
            if parameter.api_param == spec.batch.api_param && spec.batch.path_param.is_none() {
                continue;
            }
            bail!(
                "Input is missing required parameter {} for batch entry {}",
                parameter.input,
                index + 1
            );
        }
    }
    Ok(())
}

fn selected_endpoint(input: &Value, spec: &ActorSpec) -> Result<String> {
    if spec.endpoint_selectors.len() > 1 {
        bail!("Only one endpoint selector can be configured");
    }
    let Some(selector) = spec.endpoint_selectors.first() else {
        return Ok(spec.endpoint.clone());
    };
    let selected = input
        .get(&selector.input)
        .filter(|value| !value.is_null())
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| anyhow!("Input field {} must be a string", selector.input))
        })
        .transpose()?
        .or(selector.default.as_deref())
        .ok_or_else(|| anyhow!("Input field {} is required", selector.input))?;
    selector
        .choices
        .iter()
        .find(|choice| choice.value == selected)
        .map(|choice| choice.endpoint.clone())
        .ok_or_else(|| anyhow!("Unsupported {} value {selected}", selector.input))
}

fn max_results(input: &Value, spec: &ActorSpec) -> Result<usize> {
    let configured = match input.get(&spec.max_results.input) {
        Some(value) => value
            .as_u64()
            .map(|number| number as usize)
            .ok_or_else(|| anyhow!("{} must be a positive integer", spec.max_results.input))?,
        None => spec.max_results.default,
    };
    if configured == 0 {
        bail!("{} must be at least 1", spec.max_results.input);
    }
    Ok(configured.min(spec.max_results.hard_limit))
}

fn max_pages(input: &Value, spec: &ActorSpec) -> Result<usize> {
    let Some(pagination) = &spec.pagination else {
        return Ok(1);
    };
    let configured = match input.get("maxPages") {
        Some(value) => value
            .as_u64()
            .map(|number| number as usize)
            .ok_or_else(|| anyhow!("maxPages must be a positive integer"))?,
        None => spec.default_max_pages,
    };
    if configured == 0 {
        bail!("maxPages must be at least 1");
    }
    let hard_limit = if pagination.max_pages == 0 {
        spec.default_max_pages
    } else {
        pagination.max_pages
    };
    Ok(configured.min(hard_limit))
}

fn response_items(response: &Value, spec: &ActorSpec) -> Vec<Value> {
    if spec.mode == "single" {
        return (!response.is_null())
            .then(|| response.clone())
            .into_iter()
            .collect();
    }

    let pointers = spec
        .result_pointer
        .iter()
        .chain(spec.fallback_result_pointers.iter());
    for pointer in pointers {
        let value = if pointer.is_empty() {
            Some(response)
        } else {
            response.pointer(pointer)
        };
        if let Some(items) = value.and_then(Value::as_array) {
            return items.clone();
        }
    }
    Vec::new()
}

fn enrich_item(item: Value, input_value: &Value, field: &str, timestamp: &str) -> Value {
    let mut object = match item {
        Value::Object(object) => object,
        other => {
            let mut object = Map::new();
            object.insert("value".to_owned(), other);
            object
        }
    };
    object.insert(field.to_owned(), input_value.clone());
    object.insert("scraped_at".to_owned(), Value::String(timestamp.to_owned()));
    Value::Object(object)
}

fn timestamp_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_timestamp(seconds)
}

fn format_timestamp(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let day_seconds = seconds % 86_400;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let hour = day_seconds / 3_600;
    let minute = day_seconds % 3_600 / 60;
    let second = day_seconds % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn has_next_page(response: &Value, page_had_rows: bool, pagination: &PaginationSpec) -> bool {
    if let Some(end_pointer) = &pagination.end_pointer {
        if response
            .pointer(end_pointer)
            .and_then(truthy_flag)
            .unwrap_or(false)
        {
            return false;
        }
    }
    if let Some(pointer) = &pagination.has_more_pointer {
        if let Some(has_more) = response.pointer(pointer).and_then(truthy_flag) {
            return has_more;
        }
    }
    if let (Some(current), Some(total)) = (
        pagination
            .current_page_pointer
            .as_ref()
            .and_then(|pointer| response.pointer(pointer))
            .and_then(numeric_page),
        pagination
            .total_pages_pointer
            .as_ref()
            .and_then(|pointer| response.pointer(pointer))
            .and_then(numeric_page),
    ) {
        return current < total;
    }
    for pointer in pagination
        .next_pointer
        .iter()
        .chain(pagination.next_pointers.iter())
    {
        if response.pointer(pointer).is_some_and(|value| {
            !value.is_null() && value.as_str() != Some("") && value.as_str() != Some("0")
        }) {
            return true;
        }
    }
    page_had_rows
}

fn truthy_flag(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::Number(value) => value.as_f64().map(|value| value != 0.0),
        Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "" | "0" | "false" => Some(false),
            "1" | "true" => Some(true),
            _ => Some(true),
        },
        _ => None,
    }
}

fn numeric_page(value: &Value) -> Option<i64> {
    match value {
        Value::Number(value) => value.as_i64(),
        Value::String(value) => value.trim().parse::<i64>().ok(),
        _ => None,
    }
}

fn next_page_value(
    response: &Value,
    params: &Map<String, Value>,
    pagination: &PaginationSpec,
) -> Value {
    for pointer in pagination
        .next_pointer
        .iter()
        .chain(pagination.next_pointers.iter())
    {
        if let Some(value) = response
            .pointer(pointer)
            .filter(|value| !value.is_null() && value.as_str() != Some(""))
        {
            if pagination.kind == "page" {
                if let Some(page) = numeric_page(value) {
                    return json!(page);
                }
            }
            return value.clone();
        }
    }
    let current = params.get(&pagination.param).unwrap_or(&pagination.start);
    match current {
        Value::Number(number) => {
            let next = number
                .as_i64()
                .unwrap_or(0)
                .saturating_add(pagination.step.max(1));
            json!(next)
        }
        Value::String(value) if pagination.kind == "page" => {
            json!(
                value
                    .parse::<i64>()
                    .unwrap_or(0)
                    .saturating_add(pagination.step.max(1))
            )
        }
        Value::String(value) if pagination.kind == "cursor" => json!(value),
        _ => pagination.start.clone(),
    }
}

fn endpoint_url(base: &Url, endpoint: &str, path_params: &Map<String, Value>) -> Result<Url> {
    let mut url = base.clone();
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow!("Scrappa base URL cannot contain path segments"))?;
        segments.pop_if_empty();
        for segment in endpoint.trim_start_matches('/').split('/') {
            if let Some(name) = segment
                .strip_prefix('{')
                .and_then(|value| value.strip_suffix('}'))
            {
                let value = path_params
                    .get(name)
                    .ok_or_else(|| anyhow!("Missing path parameter {name}"))?;
                segments.push(&value_to_string(value));
            } else {
                segments.push(segment);
            }
        }
    }
    Ok(url)
}

fn append_segments(base: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base.clone();
    let mut path = url
        .path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?;
    path.pop_if_empty().extend(segments.iter().copied());
    drop(path);
    Ok(url)
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => String::new(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
        _ => value.to_string(),
    }
}

async fn request_scrappa(
    http: &Client,
    base: &Url,
    entry: &RequestEntry,
    api_key: &str,
    spec: &ActorSpec,
    row_limit: usize,
    page_limit: usize,
) -> Result<Value, EntryFailure> {
    let mut params = entry.params.clone();
    let mut seen_pages = HashSet::new();
    let mut seen_rows = HashSet::new();
    let mut pending_items = Vec::new();
    let mut previous_response_page = None;
    let deadline = Instant::now() + ENTRY_TIME_BUDGET;
    let mut page = 0;
    loop {
        let mut request_params = params.clone();
        if let Some(pagination) = &spec.pagination {
            if page == 0 && !request_params.contains_key(&pagination.param) {
                request_params.insert(pagination.param.clone(), pagination.start.clone());
            }
        }
        if let Some(pagination) = &spec.pagination {
            if let Some(value) = request_params.get(&pagination.param) {
                let key = value_to_string(value);
                if !seen_pages.insert(key) {
                    break;
                }
            }
        }

        let url = endpoint_url(base, &entry.endpoint, &entry.path_params)
            .map_err(|_| EntryFailure::Request)?;
        let mut url = url;
        {
            let mut query = url.query_pairs_mut();
            for (name, value) in &request_params {
                if value.is_null() || value.as_str() == Some("") {
                    continue;
                }
                query.append_pair(name, &value_to_string(value));
            }
        }

        let response = send_scrappa_with_retry(http, url, api_key, deadline).await?;
        let body = response.text().await.map_err(|_| EntryFailure::Request)?;
        let response: Value = serde_json::from_str(&body).map_err(|_| EntryFailure::InvalidJson)?;
        if let Some(message) = response_failure_message(&response) {
            return Err(EntryFailure::Upstream(message));
        }

        if let Some(pagination) = &spec.pagination {
            if let Some(response_page) = pagination
                .current_page_pointer
                .as_ref()
                .and_then(|pointer| response.pointer(pointer))
                .and_then(numeric_page)
            {
                if response_page_did_not_advance(previous_response_page, response_page) {
                    break;
                }
                previous_response_page = Some(response_page);
            }
        }

        let rows = response_items(&response, spec);
        let page_had_rows = !rows.is_empty();
        for row in rows {
            if is_duplicate_row(&row, spec.dedupe_pointer.as_deref(), &mut seen_rows) {
                continue;
            }
            pending_items.push(flatten_item(row, &spec.flatten_pointers));
            if pending_items.len() >= row_limit {
                break;
            }
        }

        let Some(pagination) = &spec.pagination else {
            break;
        };
        let page_limit = page_limit.min(pagination.max_pages.max(1));
        if pending_items.len() >= row_limit
            || page + 1 >= page_limit
            || !has_next_page(&response, page_had_rows, pagination)
        {
            break;
        }
        let next = next_page_value(&response, &request_params, pagination);
        if request_params
            .get(&pagination.param)
            .is_some_and(|current| value_to_string(current) == value_to_string(&next))
        {
            break;
        }
        params.insert(pagination.param.clone(), next);
        page += 1;
    }

    let timestamp = timestamp_now();
    let items = pending_items
        .into_iter()
        .map(|item| enrich_item(item, &entry.value, &spec.enrichment.field, &timestamp))
        .collect();
    Ok(Value::Array(items))
}

fn is_duplicate_row(row: &Value, pointer: Option<&str>, seen: &mut HashSet<String>) -> bool {
    let Some(key) = pointer
        .and_then(|pointer| row.pointer(pointer))
        .filter(|value| !value.is_null())
        .map(Value::to_string)
    else {
        return false;
    };
    !seen.insert(key)
}

fn response_page_did_not_advance(previous: Option<i64>, current: i64) -> bool {
    previous.is_some_and(|previous| current <= previous)
}

fn flatten_item(item: Value, fields: &HashMap<String, String>) -> Value {
    let mut object = match item {
        Value::Object(object) => object,
        value => {
            let mut object = Map::new();
            object.insert("value".to_owned(), value);
            object
        }
    };
    for (name, pointer) in fields {
        if let Some(value) = Value::Object(object.clone()).pointer(pointer).cloned() {
            object.insert(name.clone(), value);
        }
    }
    Value::Object(object)
}

fn response_failure_message(response: &Value) -> Option<String> {
    let success_failed = response.get("success").and_then(Value::as_bool) == Some(false);
    let nonzero_code = response.get("code").is_some_and(|code| match code {
        Value::Number(number) => number.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => value.parse::<f64>().is_ok_and(|value| value != 0.0),
        _ => false,
    });
    if !success_failed && !nonzero_code {
        return None;
    }
    response_error_message(response).or_else(|| {
        Some(if success_failed {
            "Scrappa returned success=false".to_owned()
        } else {
            "Scrappa returned a non-zero response code".to_owned()
        })
    })
}

fn response_error_message(response: &Value) -> Option<String> {
    response
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| response.pointer("/error/message").and_then(Value::as_str))
        .or_else(|| response.get("error").and_then(Value::as_str))
        .and_then(sanitize_upstream_message)
}

fn sanitize_upstream_message(message: &str) -> Option<String> {
    let mut safe = message
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();
    safe = safe.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = safe.to_ascii_lowercase();
    let redaction_markers = ["http://", "https://", "x-api-key", "authorization:"];
    if let Some(end) = redaction_markers
        .iter()
        .filter_map(|marker| lower.find(marker))
        .min()
    {
        safe.truncate(end);
        safe = safe.trim_end().to_owned();
    }
    let safe = safe.chars().take(300).collect::<String>();
    (!safe.is_empty()).then_some(safe)
}

#[derive(Debug)]
enum EntryFailure {
    Http(StatusCode, Option<String>),
    Request,
    InvalidJson,
    Upstream(String),
    TimedOut,
}

async fn send_scrappa_with_retry(
    http: &Client,
    url: Url,
    api_key: &str,
    deadline: Instant,
) -> Result<Response, EntryFailure> {
    let mut attempt = 0;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(EntryFailure::TimedOut);
        }

        let response = http
            .get(url.clone())
            .timeout(remaining.min(REQUEST_TIMEOUT))
            .header("X-API-KEY", api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await;
        match response {
            Ok(response)
                if should_retry_scrappa(response.status()) && attempt < MAX_SCRAPPA_RETRIES =>
            {
                let retry_after = response
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|value| value.to_str().ok())
                    .and_then(parse_retry_after);
                let delay = retry_delay(attempt)
                    .max(retry_after.unwrap_or_default())
                    .min(MAX_RETRY_BACKOFF);
                if delay >= deadline.saturating_duration_since(Instant::now()) {
                    return Err(EntryFailure::TimedOut);
                }
                eprintln!(
                    "Scrappa returned HTTP {}; retrying after {}ms",
                    response.status(),
                    delay.as_millis()
                );
                sleep(delay).await;
                attempt += 1;
            }
            Ok(response) if !response.status().is_success() => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                let message = serde_json::from_str::<Value>(&body)
                    .ok()
                    .and_then(|value| response_error_message(&value));
                return Err(EntryFailure::Http(status, message));
            }
            Ok(response) => return Ok(response),
            Err(error) if error.is_connect() && attempt < MAX_SCRAPPA_RETRIES => {
                let delay = retry_delay(attempt);
                if delay >= deadline.saturating_duration_since(Instant::now()) {
                    return Err(EntryFailure::TimedOut);
                }
                eprintln!(
                    "Scrappa connection failed; retrying after {}ms",
                    delay.as_millis()
                );
                sleep(delay).await;
                attempt += 1;
            }
            Err(error) if error.is_timeout() && deadline <= Instant::now() => {
                return Err(EntryFailure::TimedOut);
            }
            Err(_) => return Err(EntryFailure::Request),
        }
    }
}

fn should_retry_apify(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

fn should_retry_scrappa(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

fn status_message(
    saved_items: usize,
    successful_entries: usize,
    failed_entry_indexes: &[usize],
    budget_exhausted: bool,
    charge_limited: bool,
) -> String {
    let mut message = if charge_limited {
        format!(
            "Apify stopped accepting dataset writes; saved {saved_items} result(s) before the limit."
        )
    } else if saved_items > 0 {
        format!(
            "Completed {successful_entries} successful request(s); saved {saved_items} result(s)."
        )
    } else if successful_entries > 0 {
        format!("Completed {successful_entries} successful request(s); no results were found.")
    } else if failed_entry_indexes.is_empty() && budget_exhausted {
        "No requests were sent because the result budget is exhausted.".to_owned()
    } else {
        "No batch entries succeeded.".to_owned()
    };

    if failed_entry_indexes.is_empty() {
        message.push_str(" 0 failed entries.");
    } else {
        let indices = failed_entry_indexes
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        message.push_str(&format!(
            " {} failed entr{} (input {}).",
            failed_entry_indexes.len(),
            if failed_entry_indexes.len() == 1 {
                "y"
            } else {
                "ies"
            },
            indices
        ));
    }
    message
}

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(1_000_u64.saturating_mul(2_u64.saturating_pow(attempt as u32)))
        .min(MAX_RETRY_BACKOFF)
}

async fn apify_retry_sleep(duration: Duration) {
    if !cfg!(test) {
        sleep(duration).await;
    }
}

async fn dataset_settle_sleep(duration: Duration) {
    if !duration.is_zero() {
        sleep(duration).await;
    }
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    if let Ok(seconds) = value.trim().parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let retry_at = parse_http_date(value).ok()?;
    retry_at.duration_since(SystemTime::now()).ok()
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let response = require_success(response, operation).await?;
    response
        .json()
        .await
        .with_context(|| format!("{operation} did not contain valid JSON"))
}

async fn require_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    bail!(
        "Apify returned HTTP {} while trying to {operation}",
        response.status()
    );
}

async fn execute() -> Result<()> {
    let spec = parse_spec()?;
    let mut storage = Storage::from_env()?;
    let input = storage.get_input().await?;
    let requests = parse_requests(&input, &spec)?;
    let requested_max = max_results(&input, &spec)?;
    let page_limit = max_pages(&input, &spec)?;
    let api_key = required_env("SCRAPPA_API_KEY")?;
    let scrappa_base = Url::parse(&env_or_default("SCRAPPA_API_BASE_URL", DEFAULT_SCRAPPA_URL))
        .context("SCRAPPA_API_BASE_URL must be a valid URL")?;
    let scrappa_http = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .retry(reqwest::retry::never())
        .user_agent(USER_AGENT)
        .build()
        .context("Could not create the Scrappa HTTP client")?;
    storage.initialize_budget(requested_max).await?;

    println!(
        "{}: processing {} batch entr{}",
        spec.title,
        requests.len(),
        if requests.len() == 1 { "y" } else { "ies" }
    );
    let mut successful_entries = 0;
    let mut failed_entries = 0;
    let mut failed_entry_indexes = Vec::new();
    let mut saved_items = 0;
    let mut empty_entries = 0;
    let mut charge_limited = false;

    for (index, entry) in requests.iter().enumerate() {
        if saved_items >= requested_max || storage.remaining_items == 0 {
            break;
        }
        let remaining = requested_max
            .saturating_sub(saved_items)
            .min(storage.remaining_items);
        match request_scrappa(
            &scrappa_http,
            &scrappa_base,
            entry,
            &api_key,
            &spec,
            remaining,
            page_limit,
        )
        .await
        {
            Ok(value) => {
                let mut items = value.as_array().cloned().unwrap_or_default();
                items.truncate(remaining);
                successful_entries += 1;
                if items.is_empty() {
                    empty_entries += 1;
                    println!("Batch entry {} succeeded with no results", index + 1);
                    continue;
                }
                let pushed = storage.push_items(&items).await?;
                saved_items += pushed.saved;
                println!(
                    "Batch entry {} saved {} dataset item(s)",
                    index + 1,
                    pushed.saved
                );
                if pushed.charge_limited {
                    charge_limited = true;
                    break;
                }
            }
            Err(failure) => {
                failed_entries += 1;
                failed_entry_indexes.push(index + 1);
                match failure {
                    EntryFailure::Http(status, message) => eprintln!(
                        "Batch entry {} failed with Scrappa HTTP {}{}",
                        index + 1,
                        status,
                        message
                            .map(|message| format!(": {message}"))
                            .unwrap_or_default()
                    ),
                    EntryFailure::Request => eprintln!(
                        "Batch entry {} failed after Scrappa request retries",
                        index + 1
                    ),
                    EntryFailure::InvalidJson => {
                        eprintln!("Batch entry {} returned invalid Scrappa JSON", index + 1)
                    }
                    EntryFailure::Upstream(message) => {
                        eprintln!("Batch entry {} failed: {}", index + 1, message)
                    }
                    EntryFailure::TimedOut => {
                        eprintln!("Batch entry {} exceeded the Scrappa time budget", index + 1)
                    }
                }
            }
        }
    }

    let message = status_message(
        saved_items,
        successful_entries,
        &failed_entry_indexes,
        storage.remaining_items == 0,
        charge_limited,
    );
    // The status message is informational; never fail a run that already saved data because of it.
    if let Err(error) = storage.set_status_message(&message).await {
        eprintln!("Warning: could not update the Actor status message: {error:#}");
    }

    if successful_entries == 0 && failed_entries > 0 {
        bail!(
            "All {} batch entries failed. Review the per-entry Scrappa errors above and correct the input or retry later.",
            requests.len()
        );
    }
    if failed_entries > 0 {
        println!(
            "{} request entr{} failed; successful entries were retained",
            failed_entries,
            if failed_entries == 1 { "y" } else { "ies" }
        );
    }
    if empty_entries > 0 {
        println!(
            "{} successful entr{} returned no results",
            empty_entries,
            if empty_entries == 1 { "y" } else { "ies" }
        );
    }
    println!("{}", message);
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = execute().await {
        eprintln!("Actor failed: {error:#}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{Arc, Mutex},
        thread,
    };

    struct MockApifyServer {
        base: Url,
        requests: Arc<Mutex<Vec<String>>>,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl MockApifyServer {
        fn start(responses: Vec<(u16, String)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let captured = Arc::clone(&requests);
            let handle = thread::spawn(move || {
                for (status, body) in responses {
                    let deadline = std::time::Instant::now() + Duration::from_secs(5);
                    let (mut stream, _) = loop {
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error)
                                if error.kind() == std::io::ErrorKind::WouldBlock
                                    && std::time::Instant::now() < deadline =>
                            {
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(error) => panic!("mock server accept failed: {error}"),
                        }
                    };
                    let request = read_mock_request(&mut stream);
                    captured.lock().unwrap().push(request);
                    let reason = match status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        502 => "Bad Gateway",
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
                base: Url::parse(&format!("http://{address}")).unwrap(),
                requests,
                handle: Some(handle),
            }
        }

        fn finish(mut self) -> Vec<String> {
            self.handle.take().unwrap().join().unwrap();
            self.requests.lock().unwrap().clone()
        }
    }

    fn read_mock_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let count = stream.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            let text = String::from_utf8_lossy(&request);
            if let Some((headers, body)) = text.split_once("\r\n\r\n") {
                let body_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or_default();
                if body.len() >= body_length {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&request).into_owned()
    }

    fn test_storage(base: Url) -> Storage {
        Storage {
            http: Client::new(),
            local_root: None,
            apify_base: base,
            token: None,
            key_value_store_id: None,
            dataset_id: Some("dataset-1".to_owned()),
            run_id: Some("run-1".to_owned()),
            input_key: "INPUT".to_owned(),
            remaining_items: 10,
            local_next_item: 0,
            dataset_written: Some(0),
        }
    }

    fn request_method(request: &str) -> &str {
        request.split_whitespace().next().unwrap()
    }

    fn request_target(request: &str) -> &str {
        request.split_whitespace().nth(1).unwrap()
    }

    fn request_body(request: &str) -> Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    async fn push_test_chunk(storage: &Storage, items: &[Value]) -> Result<DatasetChunkResult> {
        let url = storage.api_url(&["v2", "datasets", "dataset-1", "items"])?;
        let body = serde_json::to_vec(items)?;
        storage
            .push_dataset_chunk(
                &url,
                items,
                &body,
                storage.dataset_written.unwrap_or_default(),
            )
            .await
    }

    #[test]
    fn compiled_fixture_has_rows_at_the_configured_result_location() {
        let spec = parse_spec().unwrap();
        let fixture: Value = serde_json::from_str(FIXTURE_JSON).unwrap();
        let rows = response_items(&fixture, &spec);
        assert!(
            !rows.is_empty(),
            "synthetic fixture must have at least one result"
        );
    }

    #[test]
    fn enriches_an_item_without_dropping_source_fields() {
        let item = enrich_item(
            json!({"title": "Synthetic result"}),
            &json!("example"),
            "input_query",
            "2026-01-01T00:00:00Z",
        );
        assert_eq!(item["title"], "Synthetic result");
        assert_eq!(item["input_query"], "example");
        assert_eq!(item["scraped_at"], "2026-01-01T00:00:00Z");
    }

    #[test]
    fn formats_unix_epoch_as_an_iso_timestamp() {
        assert_eq!(format_timestamp(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn inserts_a_path_parameter_as_one_encoded_segment() {
        let base = Url::parse("https://scrappa.co/api").unwrap();
        let params = Map::from_iter([("id".to_owned(), json!("123 456"))]);
        let url = endpoint_url(&base, "/immobilienscout24/property/{id}", &params).unwrap();
        assert_eq!(
            url.as_str(),
            "https://scrappa.co/api/immobilienscout24/property/123%20456"
        );
    }

    #[test]
    fn pagination_uses_observed_next_page_token() {
        let pagination = PaginationSpec {
            kind: "cursor".to_owned(),
            param: "cursor".to_owned(),
            start: json!("0"),
            step: 1,
            next_pointer: Some("/data/cursor".to_owned()),
            next_pointers: Vec::new(),
            has_more_pointer: Some("/data/hasMore".to_owned()),
            end_pointer: None,
            current_page_pointer: None,
            total_pages_pointer: None,
            max_pages: 3,
        };
        let response = json!({"data": {"cursor": "200", "hasMore": true}});
        assert!(has_next_page(&response, true, &pagination));
        let params = Map::from_iter([("cursor".to_owned(), json!("0"))]);
        assert_eq!(next_page_value(&response, &params, &pagination), "200");
    }

    #[test]
    fn stops_cursor_pagination_when_the_response_says_there_are_no_more_rows() {
        let pagination = PaginationSpec {
            kind: "cursor".to_owned(),
            param: "cursor".to_owned(),
            start: json!("0"),
            step: 1,
            next_pointer: Some("/data/cursor".to_owned()),
            next_pointers: Vec::new(),
            has_more_pointer: Some("/data/hasMore".to_owned()),
            end_pointer: None,
            current_page_pointer: None,
            total_pages_pointer: None,
            max_pages: 2,
        };
        let response = json!({"data": {"cursor": "200", "hasMore": false}});
        assert!(!has_next_page(&response, true, &pagination));
    }

    #[test]
    fn accepts_numeric_and_string_has_more_flags() {
        let pagination = PaginationSpec {
            kind: "cursor".to_owned(),
            param: "cursor".to_owned(),
            start: json!("0"),
            step: 1,
            next_pointer: Some("/data/cursor".to_owned()),
            next_pointers: Vec::new(),
            has_more_pointer: Some("/data/hasMore".to_owned()),
            end_pointer: None,
            current_page_pointer: None,
            total_pages_pointer: None,
            max_pages: 3,
        };

        for value in [json!(0), json!("0")] {
            assert!(!has_next_page(
                &json!({"data": {"cursor": "200", "hasMore": value}}),
                true,
                &pagination
            ));
        }
        for value in [json!(1), json!("1")] {
            assert!(has_next_page(
                &json!({"data": {"cursor": "200", "hasMore": value}}),
                true,
                &pagination
            ));
        }
    }

    #[test]
    fn explicit_false_has_more_wins_over_page_totals() {
        let pagination = PaginationSpec {
            kind: "page".to_owned(),
            param: "page".to_owned(),
            start: json!(1),
            step: 1,
            next_pointer: None,
            next_pointers: Vec::new(),
            has_more_pointer: Some("/hasMore".to_owned()),
            end_pointer: None,
            current_page_pointer: Some("/currentPage".to_owned()),
            total_pages_pointer: Some("/totalPages".to_owned()),
            max_pages: 5,
        };
        let response = json!({"hasMore": "0", "currentPage": "2", "totalPages": "5"});
        assert!(!has_next_page(&response, true, &pagination));
    }

    #[test]
    fn pagination_coerces_numeric_strings_and_stops_when_page_does_not_advance() {
        assert_eq!(numeric_page(&json!("12")), Some(12));
        assert_eq!(numeric_page(&json!(12)), Some(12));
        assert!(response_page_did_not_advance(Some(2), 2));
        assert!(response_page_did_not_advance(Some(2), 1));
        assert!(!response_page_did_not_advance(Some(2), 3));
    }

    #[test]
    fn flags_scrappa_success_and_code_errors_and_sanitizes_their_message() {
        assert_eq!(
            response_failure_message(&json!({
                "success": false,
                "error": {"message": "date range cannot exceed 14 days"}
            })),
            Some("date range cannot exceed 14 days".to_owned())
        );
        assert_eq!(
            response_failure_message(&json!({"code": 422, "message": "invalid request"})),
            Some("invalid request".to_owned())
        );
        assert_eq!(
            response_failure_message(&json!({"success": true, "code": 0})),
            None
        );

        let long = format!(
            "{}{}",
            "x".repeat(290),
            " https://example.invalid/?token=secret"
        );
        let safe = sanitize_upstream_message(&long).unwrap();
        assert_eq!(safe.chars().count(), 290);
        assert!(!safe.contains("https://"));
        assert!(!safe.contains("token=secret"));
    }

    #[test]
    fn flattens_nested_fields_and_deduplicates_only_configured_ids() {
        let flattened = flatten_item(
            json!({"user": {"user_id": "42"}, "stats": {"follower_count": 90}}),
            &HashMap::from([
                ("user_id".to_owned(), "/user/user_id".to_owned()),
                (
                    "follower_count".to_owned(),
                    "/stats/follower_count".to_owned(),
                ),
            ]),
        );
        assert_eq!(flattened["user_id"], "42");
        assert_eq!(flattened["follower_count"], 90);
        assert_eq!(flattened["user"]["user_id"], "42");

        let mut seen = HashSet::new();
        assert!(!is_duplicate_row(
            &json!({"id": "a"}),
            Some("/id"),
            &mut seen
        ));
        assert!(is_duplicate_row(
            &json!({"id": "a"}),
            Some("/id"),
            &mut seen
        ));
        assert!(!is_duplicate_row(&json!({"id": "b"}), None, &mut seen));
    }

    #[test]
    fn relative_date_defaults_resolve_from_today_or_another_input_date() {
        let mut values = Map::from_iter([("departure_date".to_owned(), json!("2027-01-01"))]);
        apply_relative_date_defaults(
            &mut values,
            &[RelativeDateDefault {
                input: "return_date".to_owned(),
                base: "departure_date".to_owned(),
                offset_days: 4,
                endpoints: Vec::new(),
            }],
            "/kayak/flights/round-trip",
        )
        .unwrap();
        assert_eq!(values["return_date"], "2027-01-05");
        assert_eq!(
            parse_iso_date("2024-02-29").map(format_iso_date),
            Some("2024-02-29".to_owned())
        );
        assert_eq!(parse_iso_date("2027-02-29"), None);
    }

    #[test]
    fn kayak_route_batch_applies_only_the_selected_trip_dates_before_requests() {
        let spec = parse_spec().unwrap();
        if spec.endpoint != "/kayak/flights/one-way" {
            return;
        }
        let one_way_input: Value =
            serde_json::from_str(include_str!("../input-prefill.json")).unwrap();
        let one_way = parse_requests(&one_way_input, &spec).unwrap().remove(0);
        assert_eq!(one_way.endpoint, "/kayak/flights/one-way");
        assert_eq!(one_way.params["origin"], "JFK");
        assert_eq!(one_way.params["destination"], "LAX");
        assert!(one_way.params.get("departure_date").is_some());
        assert!(one_way.params.get("return_date").is_none());

        let mut round_trip_input = one_way_input;
        round_trip_input["tripType"] = json!("round-trip");
        let round_trip = parse_requests(&round_trip_input, &spec).unwrap().remove(0);
        assert_eq!(round_trip.endpoint, "/kayak/flights/round-trip");
        assert!(round_trip.params.get("departure_date").is_some());
        assert!(round_trip.params.get("return_date").is_some());
        assert!(
            round_trip.params["return_date"].as_str().unwrap()
                > round_trip.params["departure_date"].as_str().unwrap()
        );
    }

    #[test]
    fn chunks_dataset_pushes_by_item_count_and_serialized_size() {
        let items = (0..501).map(|id| json!({"id": id})).collect::<Vec<_>>();
        let chunks = dataset_item_chunks(&items).unwrap();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].items.len(), 500);
        assert_eq!(chunks[1].items.len(), 1);
        assert!(
            chunks
                .iter()
                .all(|chunk| chunk.body.len() <= MAX_DATASET_PUSH_BYTES)
        );

        let large_item = json!({"value": "x".repeat(MAX_DATASET_PUSH_BYTES)});
        assert!(dataset_item_chunks(&[large_item]).is_err());
    }

    #[test]
    fn resolves_ambiguous_dataset_writes_without_reposting_written_rows() {
        let items = vec![json!({"id": "one"}), json!({"id": "two"})];

        assert_eq!(
            dataset_retry_resolution(compare_dataset_prefix(&items, &items)),
            DatasetRetryResolution::AlreadyWritten
        );
        assert_eq!(
            dataset_retry_resolution(compare_dataset_prefix(&items, &[])),
            DatasetRetryResolution::RetryFull
        );
        assert_eq!(
            dataset_retry_resolution(compare_dataset_prefix(&items, &items[..1])),
            DatasetRetryResolution::RetryRemainder(1)
        );
        assert_eq!(
            compare_dataset_prefix(&[json!({"value":4.0})], &[json!({"value":4})]),
            DatasetVerification::Complete
        );
        assert_eq!(
            compare_dataset_prefix(
                &[json!(9_007_199_254_740_993_u64)],
                &[json!(9_007_199_254_740_992_u64)]
            ),
            DatasetVerification::Mismatch
        );
        assert_eq!(
            compare_dataset_prefix(
                &[json!(9_007_199_254_740_993_u64)],
                &[json!(9_007_199_254_740_992.0)]
            ),
            DatasetVerification::Mismatch
        );
        assert_eq!(
            compare_dataset_prefix(
                &[json!(9_007_199_254_740_992_u64)],
                &[json!(9_007_199_254_740_992.0)]
            ),
            DatasetVerification::Complete
        );
        assert!(json_values_equal(
            &json!({"outer":[4.0, {"value": 2}]}),
            &json!({"outer":[4, {"value": 2.0}]})
        ));
        assert_eq!(dataset_verification_window(false), Duration::from_secs(10));
        assert_eq!(dataset_verification_window(true), Duration::from_secs(30));
        assert_eq!(
            dataset_retry_resolution(compare_dataset_prefix(
                &items,
                &[json!({"id": "different"})]
            )),
            DatasetRetryResolution::Mismatch
        );
    }

    #[tokio::test]
    async fn does_not_repost_a_dataset_chunk_that_a_502_already_wrote() {
        let items = vec![json!({"id":"one"}), json!({"id":"two"})];
        let server = MockApifyServer::start(vec![
            (502, "{}".to_owned()),
            (200, serde_json::to_string(&items).unwrap()),
        ]);
        let storage = test_storage(server.base.clone());

        let result = push_test_chunk(&storage, &items).await.unwrap();
        assert!(matches!(result, DatasetChunkResult::Saved(2)));

        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_method(&requests[0]), "POST");
        assert_eq!(request_method(&requests[1]), "GET");
        assert_eq!(
            request_target(&requests[1]),
            "/v2/datasets/dataset-1/items?offset=0&limit=2"
        );
    }

    #[tokio::test]
    async fn reposts_a_dataset_chunk_after_verifying_that_no_rows_were_written() {
        let items = vec![json!({"id":"one"})];
        let server = MockApifyServer::start(vec![
            (502, "{}".to_owned()),
            (200, "[]".to_owned()),
            (201, "{}".to_owned()),
        ]);
        let storage = test_storage(server.base.clone());

        let result = push_test_chunk(&storage, &items).await.unwrap();
        assert!(matches!(result, DatasetChunkResult::Saved(1)));

        let requests = server.finish();
        assert_eq!(requests.len(), 3);
        assert_eq!(request_method(&requests[0]), "POST");
        assert_eq!(request_method(&requests[1]), "GET");
        assert_eq!(request_method(&requests[2]), "POST");
        assert_eq!(request_body(&requests[0]), request_body(&requests[2]));
    }

    #[tokio::test]
    async fn pushes_only_the_missing_dataset_suffix_after_a_prefix_write() {
        let items = vec![
            json!({"id":"one"}),
            json!({"id":"two"}),
            json!({"id":"three"}),
        ];
        let server = MockApifyServer::start(vec![
            (502, "{}".to_owned()),
            (200, "[{\"id\":\"one\"}]".to_owned()),
            (200, "[{\"id\":\"one\"},{\"id\":\"two\"}]".to_owned()),
            (200, "[{\"id\":\"one\"},{\"id\":\"two\"}]".to_owned()),
            (201, "{}".to_owned()),
        ]);
        let storage = test_storage(server.base.clone());

        let result = push_test_chunk(&storage, &items).await.unwrap();
        assert!(matches!(result, DatasetChunkResult::Saved(3)));

        let requests = server.finish();
        assert_eq!(requests.len(), 5);
        assert_eq!(request_method(&requests[0]), "POST");
        assert_eq!(request_method(&requests[1]), "GET");
        assert_eq!(request_method(&requests[2]), "GET");
        assert_eq!(request_method(&requests[3]), "GET");
        assert_eq!(request_method(&requests[4]), "POST");
        assert_eq!(
            request_body(&requests[0]),
            json!([{"id":"one"},{"id":"two"},{"id":"three"}])
        );
        assert_eq!(request_body(&requests[4]), json!([{"id":"three"}]));
        assert_eq!(
            request_target(&requests[1]),
            "/v2/datasets/dataset-1/items?offset=0&limit=3"
        );
        assert_eq!(
            request_target(&requests[2]),
            "/v2/datasets/dataset-1/items?offset=0&limit=3"
        );
        assert_eq!(
            request_target(&requests[3]),
            "/v2/datasets/dataset-1/items?offset=0&limit=3"
        );
    }

    #[tokio::test]
    async fn initializes_the_restart_offset_from_lagging_dataset_items() {
        let items = vec![json!({"id":"new"})];
        let server = MockApifyServer::start(vec![
            (200, "{\"data\":{}}".to_owned()),
            (200, "{\"data\":{\"itemCount\":19}}".to_owned()),
            (200, "[]".to_owned()),
            (200, "[{\"id\":\"old-1\"},{\"id\":\"old-2\"}]".to_owned()),
            (200, "[]".to_owned()),
            (200, "[]".to_owned()),
            (502, "{}".to_owned()),
            (200, serde_json::to_string(&items).unwrap()),
        ]);
        let mut storage = test_storage(server.base.clone());
        storage.initialize_budget(10).await.unwrap();
        let count = storage.dataset_written.unwrap();
        assert_eq!(count, 21);

        let result = push_test_chunk(&storage, &items).await.unwrap();
        assert!(matches!(result, DatasetChunkResult::Saved(1)));

        let requests = server.finish();
        let item_reads = requests
            .iter()
            .filter(|request| request.starts_with("GET /v2/datasets/dataset-1/items?"))
            .map(|request| request_target(request))
            .collect::<Vec<_>>();
        assert_eq!(
            item_reads,
            vec![
                "/v2/datasets/dataset-1/items?offset=19&limit=500",
                "/v2/datasets/dataset-1/items?offset=19&limit=500",
                "/v2/datasets/dataset-1/items?offset=21&limit=500",
                "/v2/datasets/dataset-1/items?offset=21&limit=500",
                "/v2/datasets/dataset-1/items?offset=21&limit=1",
            ]
        );
        assert_eq!(request_method(&requests[6]), "POST");
        assert_eq!(request_method(&requests[7]), "GET");
    }

    #[tokio::test]
    async fn never_reposts_a_chunk_after_some_of_its_rows_were_seen() {
        let items = vec![json!({"id":"one"}), json!({"id":"two"})];
        let server = MockApifyServer::start(vec![
            (502, "{}".to_owned()),
            (200, "[{\"id\":\"one\"}]".to_owned()),
            (200, "[]".to_owned()),
            // Only consumed if the chunk were wrongly re-posted.
            (201, "{}".to_owned()),
        ]);
        let storage = test_storage(server.base.clone());

        let result = push_test_chunk(&storage, &items).await;
        assert!(result.is_err());

        // Read the captured requests directly: the spare response stays unused on success.
        let requests = server.requests.lock().unwrap().clone();
        let posts = requests
            .iter()
            .filter(|request| request_method(request) == "POST")
            .count();
        assert_eq!(posts, 1);
    }

    #[tokio::test]
    async fn charge_limit_response_stops_dataset_writes_without_retrying() {
        let items = vec![json!({"id":"one"})];
        let server = MockApifyServer::start(vec![(400, "max-total-charge-exceeded".to_owned())]);
        let storage = test_storage(server.base.clone());

        let result = push_test_chunk(&storage, &items).await.unwrap();
        assert!(matches!(result, DatasetChunkResult::ChargeLimited(0)));

        let requests = server.finish();
        assert_eq!(requests.len(), 1);
        assert_eq!(request_method(&requests[0]), "POST");
    }

    #[test]
    fn retries_only_the_configured_apify_transient_statuses() {
        for status in [
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::BAD_GATEWAY,
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::GATEWAY_TIMEOUT,
        ] {
            assert!(should_retry_apify(status));
        }
        assert!(!should_retry_apify(StatusCode::BAD_REQUEST));
        assert!(!should_retry_apify(StatusCode::NOT_FOUND));
        assert!(!should_retry_apify(StatusCode::NOT_IMPLEMENTED));
        assert_eq!(retry_delay(0), Duration::from_secs(1));
        assert_eq!(retry_delay(1), Duration::from_secs(2));
        assert_eq!(retry_delay(2), Duration::from_secs(4));
        assert_eq!(retry_delay(3), Duration::from_secs(8));
        assert_eq!(retry_delay(4), Duration::from_secs(15));
        assert_eq!(retry_delay(10), Duration::from_secs(15));
    }

    #[test]
    fn recognizes_charge_denials_and_local_numeric_dataset_indexes() {
        assert!(charge_limit_response(StatusCode::PAYMENT_REQUIRED, ""));
        assert!(charge_limit_response(StatusCode::FORBIDDEN, ""));
        assert!(charge_limit_response(
            StatusCode::BAD_REQUEST,
            "maximum charge reached"
        ));
        assert!(charge_limit_response(
            StatusCode::BAD_REQUEST,
            "max-total-charge-exceeded"
        ));
        assert!(!charge_limit_response(
            StatusCode::BAD_REQUEST,
            "invalid item"
        ));
        assert_eq!(
            dataset_index_from_path(Path::new("000000014.json")),
            Some(14)
        );
        assert_eq!(dataset_index_from_path(Path::new("other.json")), None);
        assert_eq!(dataset_index_from_path(Path::new("000000099.txt")), None);
    }

    #[test]
    fn retries_only_scrappa_transients_and_honors_retry_after() {
        assert!(should_retry_scrappa(StatusCode::TOO_MANY_REQUESTS));
        assert!(should_retry_scrappa(StatusCode::BAD_GATEWAY));
        assert!(should_retry_scrappa(StatusCode::SERVICE_UNAVAILABLE));
        assert!(should_retry_scrappa(StatusCode::GATEWAY_TIMEOUT));
        assert!(!should_retry_scrappa(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(!should_retry_scrappa(StatusCode::UNPROCESSABLE_ENTITY));
        assert_eq!(parse_retry_after("3"), Some(Duration::from_secs(3)));
        assert_eq!(
            retry_delay(0)
                .max(Duration::from_secs(30))
                .min(MAX_RETRY_BACKOFF),
            MAX_RETRY_BACKOFF
        );
        assert!(retry_delay(100) <= MAX_RETRY_BACKOFF);
    }

    #[test]
    fn status_message_reports_charge_stops_and_failed_entry_indexes() {
        assert_eq!(
            status_message(0, 1, &[], false, true),
            "Apify stopped accepting dataset writes; saved 0 result(s) before the limit. 0 failed entries."
        );
        assert_eq!(
            status_message(3, 1, &[2, 4], false, false),
            "Completed 1 successful request(s); saved 3 result(s). 2 failed entries (input 2, 4)."
        );
        assert_eq!(
            status_message(0, 0, &[1], false, false),
            "No batch entries succeeded. 1 failed entry (input 1)."
        );
        assert!(status_message(0, 1, &[], false, false).contains("no results were found"));
    }

    #[test]
    fn computes_pay_per_event_budget_from_existing_charges() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.10},
                        "other-event": {"eventPriceUsd": 0.05}
                    }}
                },
                "chargedEventCounts": {"apify-default-dataset-item": 1, "other-event": 1},
                "options": {"maxTotalChargeUsd": 0.35}
            }
        });
        assert_eq!(apify_dataset_budget(&run, 10).unwrap(), 2);
    }

    #[test]
    fn applies_paid_item_limit_to_run_charges_including_zero() {
        let run = json!({
            "data": {
                "chargedEventCounts": {
                    "apify-default-dataset-item": 3
                }
            }
        });

        assert_eq!(remaining_paid_dataset_items(&run, 10).unwrap(), 7);
        assert_eq!(remaining_paid_dataset_items(&run, 2).unwrap(), 0);
        assert_eq!(remaining_paid_dataset_items(&run, 0).unwrap(), 0);
        assert_eq!(
            remaining_paid_items_for_model(&run, 10, "PRICE_PER_DATASET_ITEM", Some(7)).unwrap(),
            3
        );
    }

    #[test]
    fn refuses_an_unresolved_cost_budget_instead_of_allowing_all_requested_items() {
        let dataset_item_pricing = json!({
            "data": {
                "pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM"},
                "options": {"maxTotalChargeUsd": 0.25}
            }
        });
        assert!(apify_dataset_budget_with_limit(&dataset_item_pricing, 10, None).is_err());

        let unresolved_event_price = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {}}
                },
                "options": {"maxTotalChargeUsd": 0.25}
            }
        });
        assert!(apify_dataset_budget_with_limit(&unresolved_event_price, 10, None).is_err());
    }

    #[test]
    fn uses_the_highest_tiered_dataset_item_price_for_budgeting() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.01, "eventTieredPricingUsd": {
                            "BRONZE": {"tieredEventPriceUsd": 0.05},
                            "GOLD": {"tieredEventPriceUsd": 0.12}
                        }}
                    }}
                },
                "chargedEventCounts": {},
                "options": {"maxTotalChargeUsd": 0.25}
            }
        });
        assert_eq!(apify_dataset_budget_with_limit(&run, 10, None).unwrap(), 2);
    }

    #[test]
    fn rejects_invalid_run_charge_limits_instead_of_ignoring_them() {
        let run = json!({
            "data": {
                "pricingInfo": {"pricingModel": "PAY_PER_EVENT"},
                "options": {"maxTotalChargeUsd": "not-a-price"}
            }
        });
        assert!(apify_dataset_budget_with_limit(&run, 10, None).is_err());
        assert_eq!(parse_charge_limit(&json!("0.25")).unwrap(), 0.25);
        assert!(parse_charge_limit(&json!(-0.1)).is_err());

        let null_limit = json!({
            "data": {
                "pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM"},
                "options": {"maxTotalChargeUsd": null}
            }
        });
        assert_eq!(
            apify_dataset_budget_with_limit(&null_limit, 10, None).unwrap(),
            10
        );
        assert!(apify_dataset_budget_with_limit(&null_limit, 10, Some(0.25)).is_err());
    }

    #[test]
    fn validates_the_prefilled_batch_input() {
        let spec = parse_spec().unwrap();
        let input: Value = serde_json::from_str(include_str!("../input-prefill.json")).unwrap();
        let requests = parse_requests(&input, &spec).unwrap();
        assert_eq!(requests.len(), 1);
    }
}
