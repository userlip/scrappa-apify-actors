mod business_id;

use anyhow::{anyhow, bail, Context, Result};
use business_id::{get_business_id_requests, BusinessIdRequest};
use reqwest::{header, Method, RequestBuilder, Response, StatusCode};
use serde_json::{json, Map, Value};
use std::{env, error::Error as StdError, fmt, time::Duration};
use tokio::{time::sleep, time::timeout};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_MAX_ATTEMPTS: u32 = 3;
const SCRAPPA_RETRY_DELAY: Duration = Duration::from_millis(500);
const ACTOR_TIMEOUT: Duration = Duration::from_secs(720);
const APIFY_MAX_RETRIES: u8 = 8;
const APIFY_INITIAL_RETRY_DELAY: Duration = Duration::from_millis(500);
const MAX_DATASET_REQUEST_BYTES: usize = 9 * 1024 * 1024;
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
    scrappa_api_key: String,
    max_total_charge_usd: Option<f64>,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key: required_env("SCRAPPA_API_KEY")?,
            max_total_charge_usd: optional_f64_env("ACTOR_MAX_TOTAL_CHARGE_USD")?,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value =
        env::var(name).with_context(|| format!("{name} environment variable is not set."))?;
    if value.trim().is_empty() {
        bail!("{name} environment variable is not set.");
    }
    Ok(value)
}

fn optional_f64_env(name: &str) -> Result<Option<f64>> {
    let Ok(value) = env::var(name) else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    let parsed = value
        .parse::<f64>()
        .with_context(|| format!("{name} must be a valid number"))?;
    if !parsed.is_finite() || parsed < 0.0 {
        bail!("{name} must be a non-negative finite number");
    }
    Ok(Some(parsed))
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
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

fn apify_retry_delay(retry_number: u8) -> Duration {
    APIFY_INITIAL_RETRY_DELAY * 2_u32.pow(u32::from(retry_number.saturating_sub(1)))
}

fn retryable_apify_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

async fn send_apify_request(client: &reqwest::Client, request: RequestBuilder) -> Result<Response> {
    let request = request
        .build()
        .context("Could not build Apify API request")?;
    let retryable_method = request.method() == Method::GET || request.method() == Method::PUT;
    let mut last_error = None;

    for attempt in 0..=APIFY_MAX_RETRIES {
        let request = request
            .try_clone()
            .ok_or_else(|| anyhow!("Apify API request body could not be retried"))?;
        match client.execute(request).await {
            Ok(response)
                if retryable_method
                    && retryable_apify_status(response.status())
                    && attempt < APIFY_MAX_RETRIES =>
            {
                drop(response);
                sleep(apify_retry_delay(attempt + 1)).await;
            }
            Ok(response) => return Ok(response),
            Err(error)
                if retryable_method
                    && (error.is_connect() || error.is_timeout())
                    && attempt < APIFY_MAX_RETRIES =>
            {
                last_error = Some(error);
                sleep(apify_retry_delay(attempt + 1)).await;
            }
            Err(error) => return Err(error).context("Apify API request failed"),
        }
    }

    let error = last_error
        .map(anyhow::Error::from)
        .unwrap_or_else(|| anyhow!("Apify API request failed after retries"));
    Err(error).context("Apify API request failed after retries")
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = response.text().await.unwrap_or_default();
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "{operation} failed with {} {reason}{detail}",
            status.as_u16()
        );
    }
    response
        .json()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

async fn response_status(response: Response, operation: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    bail!(
        "{operation} failed with {} {reason}{detail}",
        status.as_u16()
    );
}

async fn get_input(client: &reqwest::Client, config: &ActorConfig) -> Result<Option<Value>> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let response = send_apify_request(
        client,
        client
            .get(url)
            .bearer_auth(&config.apify_token)
            .header(header::ACCEPT, "application/json"),
    )
    .await
    .context("Apify INPUT request failed")?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response_json(response, "Apify INPUT request")
        .await
        .map(Some)
}

fn event_price(events: &Map<String, Value>, event_name: &str) -> Result<f64> {
    let price = events
        .get(event_name)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the price for event {event_name}"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned an invalid price for event {event_name}");
    }
    Ok(price)
}

fn affordable_dataset_items(
    run: &Value,
    requested: usize,
    environment_max_charge: Option<f64>,
    locally_saved_dataset_items: usize,
) -> Result<Option<usize>> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let Some(pricing_model) = data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
    else {
        bail!("Apify run pricing model is missing");
    };
    if pricing_model != "PAY_PER_EVENT" {
        return Ok(None);
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = event_price(events, DATASET_ITEM_EVENT)?;
    let max_charge = match data
        .pointer("/options/maxTotalChargeUsd")
        .and_then(Value::as_f64)
    {
        Some(0.0) => None,
        Some(max_charge) => Some(max_charge),
        None => environment_max_charge.filter(|max_charge| *max_charge != 0.0),
    };
    let Some(max_charge) = max_charge else {
        return Ok(Some(requested));
    };
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned an invalid spending limit");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let reported_dataset_items = match counts.get(DATASET_ITEM_EVENT) {
        Some(count) => count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {DATASET_ITEM_EVENT}"))?,
        None => 0,
    };
    let locally_saved_dataset_items = u64::try_from(locally_saved_dataset_items)
        .context("Too many dataset items were saved during this actor run")?;
    let charged_dataset_items = reported_dataset_items.max(locally_saved_dataset_items);
    let mut spent = item_price * charged_dataset_items as f64;
    for (event_name, count) in counts {
        if event_name == DATASET_ITEM_EVENT {
            continue;
        }
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if count > 0 {
            let price = events
                .get(event_name)
                .and_then(|event| event.get("eventPriceUsd"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if !price.is_finite() || price < 0.0 {
                bail!("Apify run returned an invalid price for event {event_name}");
            }
            spent += price * count as f64;
        }
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(Some(requested));
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let affordable = (1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .count();
    Ok(Some(affordable))
}

async fn run_dataset_capacity(
    client: &reqwest::Client,
    config: &ActorConfig,
    requested: usize,
    locally_saved_dataset_items: usize,
) -> Result<Option<usize>> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = send_apify_request(
        client,
        client
            .get(url)
            .bearer_auth(&config.apify_token)
            .header(header::ACCEPT, "application/json"),
    )
    .await
    .context("Apify run pricing request failed")?;
    let run = response_json(response, "Apify run pricing request").await?;
    affordable_dataset_items(
        &run,
        requested,
        config.max_total_charge_usd,
        locally_saved_dataset_items,
    )
}

fn dataset_chunks(items: &[Value]) -> Result<Vec<&[Value]>> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut byte_count = 2;

    for (index, item) in items.iter().enumerate() {
        let item_bytes = serde_json::to_vec(item)
            .context("Could not serialize Apify dataset item")?
            .len();
        if item_bytes + 2 > MAX_DATASET_REQUEST_BYTES {
            bail!("Apify dataset items cannot exceed 9 MB");
        }
        let separator_bytes = usize::from(index > start);
        if index > start && byte_count + separator_bytes + item_bytes > MAX_DATASET_REQUEST_BYTES {
            chunks.push(&items[start..index]);
            start = index;
            byte_count = 2;
        }
        byte_count += usize::from(index > start) + item_bytes;
    }
    if start < items.len() {
        chunks.push(&items[start..]);
    }
    Ok(chunks)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct DatasetSave {
    saved_count: usize,
    charge_limit_reached: bool,
}

#[derive(Debug, Default)]
struct DatasetBudget {
    saved_items: usize,
}

async fn push_dataset_items(
    client: &reqwest::Client,
    config: &ActorConfig,
    items: &[Value],
    dataset_budget: &mut DatasetBudget,
) -> Result<DatasetSave> {
    if items.is_empty() {
        return Ok(DatasetSave::default());
    }

    let capacity =
        run_dataset_capacity(client, config, items.len(), dataset_budget.saved_items).await?;
    let saved_count = capacity.unwrap_or(items.len()).min(items.len());
    if saved_count == 0 {
        return Ok(DatasetSave {
            saved_count: 0,
            charge_limit_reached: true,
        });
    }

    let items_to_save = &items[..saved_count];
    let chunks = dataset_chunks(items_to_save)?;
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;

    for chunk in chunks {
        let response = send_apify_request(
            client,
            client
                .post(url.clone())
                .bearer_auth(&config.apify_token)
                .header(header::ACCEPT, "application/json")
                .json(chunk),
        )
        .await
        .context("Apify dataset write failed")?;
        response_status(response, "Apify dataset write").await?;
        dataset_budget.saved_items = dataset_budget
            .saved_items
            .checked_add(chunk.len())
            .ok_or_else(|| anyhow!("Dataset item count exceeded the actor's capacity"))?;
    }

    Ok(DatasetSave {
        saved_count,
        charge_limit_reached: saved_count < items.len(),
    })
}

async fn set_output_value(
    client: &reqwest::Client,
    config: &ActorConfig,
    output: &Value,
) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            "OUTPUT",
        ],
    )?;
    let response = send_apify_request(
        client,
        client
            .put(url)
            .bearer_auth(&config.apify_token)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .json(output),
    )
    .await
    .context("Apify OUTPUT write failed")?;
    response_status(response, "Apify OUTPUT write").await
}

#[derive(Debug)]
struct ScrappaError {
    status_code: Option<u16>,
    message: String,
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for ScrappaError {}

struct ScrappaClient<'a> {
    http: &'a reqwest::Client,
    base_url: &'a Url,
    api_key: &'a str,
}

impl ScrappaClient<'_> {
    async fn get_photos(&self, business_id: &str, input: Option<&Value>) -> Result<Value> {
        let url = build_photos_url(self.base_url, business_id, input)?;
        for attempt in 0..SCRAPPA_MAX_ATTEMPTS {
            let response = self
                .http
                .get(url.clone())
                .header("X-API-Key", self.api_key)
                .header(header::ACCEPT, "application/json")
                .timeout(SCRAPPA_REQUEST_TIMEOUT)
                .send()
                .await;

            match response {
                Ok(response) if response.status().is_success() => {
                    return response
                        .json()
                        .await
                        .context("Scrappa API response was not valid JSON");
                }
                Ok(response) => {
                    let status = response.status();
                    if attempt + 1 < SCRAPPA_MAX_ATTEMPTS
                        && (status == StatusCode::REQUEST_TIMEOUT
                            || status == StatusCode::TOO_MANY_REQUESTS
                            || status.is_server_error())
                    {
                        sleep(SCRAPPA_RETRY_DELAY * (attempt + 1)).await;
                        continue;
                    }
                    let status_code = status.as_u16();
                    let message = scrappa_error_message(response).await;
                    return Err(ScrappaError {
                        status_code: Some(status_code),
                        message: format!("Scrappa API error ({status_code}): {message}"),
                    }
                    .into());
                }
                Err(error) => {
                    if attempt + 1 < SCRAPPA_MAX_ATTEMPTS
                        && (error.is_connect() || error.is_timeout())
                    {
                        sleep(SCRAPPA_RETRY_DELAY * (attempt + 1)).await;
                        continue;
                    }
                    let message = if error.is_timeout() {
                        format!(
                            "Scrappa API request timed out after {}ms",
                            SCRAPPA_REQUEST_TIMEOUT.as_millis()
                        )
                    } else {
                        format!("Scrappa API request failed: {error}")
                    };
                    return Err(ScrappaError {
                        status_code: None,
                        message,
                    }
                    .into());
                }
            }
        }
        unreachable!("bounded Scrappa GET attempts always return")
    }
}

fn build_photos_url(base_url: &Url, business_id: &str, input: Option<&Value>) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["maps", "photos"])?;
    url.query_pairs_mut()
        .append_pair("business_id", business_id);

    if input
        .and_then(|input| input.get("use_cache"))
        .and_then(Value::as_bool)
        != Some(false)
    {
        url.query_pairs_mut().append_pair("use_cache", "1");
    }

    if let Some(maximum_cache_age) = input
        .and_then(|input| input.get("maximum_cache_age"))
        .and_then(query_value)
    {
        url.query_pairs_mut()
            .append_pair("maximum_cache_age", &maximum_cache_age);
    }
    Ok(url)
}

fn query_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::Bool(false) => None,
        Value::Bool(true) => Some("1".to_owned()),
        Value::String(value) if value.is_empty() => None,
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Array(values) => Some(
            values
                .iter()
                .map(|value| match value {
                    Value::Null => String::new(),
                    Value::String(value) => value.clone(),
                    Value::Object(_) => "[object Object]".to_owned(),
                    value => value.to_string(),
                })
                .collect::<Vec<_>>()
                .join(","),
        ),
        Value::Object(_) => Some("[object Object]".to_owned()),
    }
}

async fn scrappa_error_message(response: Response) -> String {
    let fallback = response
        .status()
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", response.status().as_u16()));
    let Ok(body) = response.text().await else {
        return fallback;
    };
    if body.is_empty() {
        return fallback;
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| fallback.clone());
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, values)| {
                    let messages = values
                        .as_array()
                        .map(|values| {
                            values
                                .iter()
                                .map(|value| {
                                    value
                                        .as_str()
                                        .map(str::to_owned)
                                        .unwrap_or_else(|| value.to_string())
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .or_else(|| values.as_str().map(str::to_owned))
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

fn photo_results(response: &Value) -> Result<(Vec<Value>, Value)> {
    if let Some(photos) = response.as_array() {
        return Ok((photos.clone(), Value::Null));
    }
    let photos = response
        .get("items")
        .filter(|items| !items.is_null())
        .or_else(|| response.get("data"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    let photos = photos
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow!("Scrappa API response photos must be an array"))?;
    let next_page = response.get("nextPage").cloned().unwrap_or(Value::Null);
    Ok((photos, next_page))
}

fn dataset_photo(photo: &Value, input_business_id: &str, business_id: &str) -> Value {
    let mut fields = match photo {
        Value::Object(fields) => fields.clone(),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        Value::String(value) => value
            .chars()
            .enumerate()
            .map(|(index, character)| (index.to_string(), Value::String(character.to_string())))
            .collect(),
        _ => Map::new(),
    };
    fields.insert(
        "input_business_id".to_owned(),
        Value::String(input_business_id.to_owned()),
    );
    fields.insert(
        "business_id".to_owned(),
        Value::String(business_id.to_owned()),
    );
    Value::Object(fields)
}

fn input_error_output(input_business_id: &str, business_id: &str, error: &str) -> Value {
    json!({
        "success": false,
        "input_business_id": input_business_id,
        "business_id": business_id,
        "error": error
    })
}

fn api_input_error(status_code: u16) -> Option<&'static str> {
    match status_code {
        404 => Some("Business not found"),
        422 => Some("Invalid input"),
        _ => None,
    }
}

fn business_summary(
    input_business_id: &str,
    business_id: &str,
    total: usize,
    next_page: Value,
    error: Option<&str>,
) -> Value {
    let mut summary = json!({
        "input_business_id": input_business_id,
        "business_id": business_id,
        "total": total,
        "nextPage": next_page,
    });
    if let Some(error) = error {
        summary["error"] = Value::String(error.to_owned());
    }
    summary
}

async fn record_business_error(
    client: &reqwest::Client,
    config: &ActorConfig,
    dataset_budget: &mut DatasetBudget,
    request: &BusinessIdRequest,
    business_id: &str,
    error: &str,
    run_results: &mut Vec<Value>,
    first_output: &mut Option<Value>,
) -> Result<()> {
    let output = input_error_output(&request.input_business_id, business_id, error);
    push_dataset_items(client, config, &[output], dataset_budget).await?;
    run_results.push(business_summary(
        &request.input_business_id,
        business_id,
        0,
        Value::Null,
        Some(error),
    ));
    if first_output.is_none() {
        *first_output = Some(json!({ "photos": [], "total": 0, "nextPage": null, "error": error }));
    }
    Ok(())
}

async fn run_actor(client: &reqwest::Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let requests = get_business_id_requests(input.as_ref())?;
    if requests.is_empty() {
        bail!("At least one Business ID is required. Provide business_ids or legacy business_id.");
    }

    let scrappa = ScrappaClient {
        http: client,
        base_url: &config.scrappa_api_base_url,
        api_key: &config.scrappa_api_key,
    };
    let mut run_results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;
    let mut total_photos = 0;
    let mut first_output = None;
    let mut dataset_budget = DatasetBudget::default();

    println!(
        "Fetching Google Maps photos for {} business{}",
        requests.len(),
        if requests.len() == 1 { "" } else { "es" }
    );

    for request in &requests {
        let Some(business_id) = request.business_id.as_deref() else {
            let error = request
                .validation_error
                .as_deref()
                .unwrap_or("Invalid business input");
            println!("Invalid business input: {error}");
            record_business_error(
                client,
                config,
                &mut dataset_budget,
                request,
                &request.input_business_id,
                error,
                &mut run_results,
                &mut first_output,
            )
            .await?;
            failed += 1;
            continue;
        };

        if request.source == Some("url") {
            println!("Extracted Google Maps business identifier from URL: {business_id}");
        }
        println!("Fetching photos for business: {business_id}");

        let response = match scrappa.get_photos(business_id, input.as_ref()).await {
            Ok(response) => response,
            Err(error) => {
                let status_code = error
                    .downcast_ref::<ScrappaError>()
                    .and_then(|error| error.status_code);
                if let Some(error) = status_code.and_then(api_input_error) {
                    println!("Photos request returned {status_code:?}: {business_id}");
                    record_business_error(
                        client,
                        config,
                        &mut dataset_budget,
                        request,
                        business_id,
                        error,
                        &mut run_results,
                        &mut first_output,
                    )
                    .await?;
                    failed += 1;
                    continue;
                }
                return Err(error);
            }
        };

        let (photos, next_page) = photo_results(&response)?;
        let dataset_photos = photos
            .iter()
            .map(|photo| dataset_photo(photo, &request.input_business_id, business_id))
            .collect::<Vec<_>>();
        let mut saved_photo_count = dataset_photos.len();
        if !dataset_photos.is_empty() {
            let saved =
                push_dataset_items(client, config, &dataset_photos, &mut dataset_budget).await?;
            saved_photo_count = saved.saved_count;
            if saved.charge_limit_reached {
                println!(
                    "Apify PPE charge limit reached after saving {} photo item(s)",
                    saved.saved_count
                );
            }
            println!("Found {} photos for {business_id}", dataset_photos.len());
        } else {
            println!("No photos found for business: {business_id}");
        }

        run_results.push(business_summary(
            &request.input_business_id,
            business_id,
            dataset_photos.len(),
            next_page.clone(),
            None,
        ));
        if first_output.is_none() {
            first_output = Some(json!({
                "photos": &dataset_photos[..saved_photo_count],
                "total": photos.len(),
                "nextPage": next_page,
            }));
        }
        succeeded += 1;
        total_photos += dataset_photos.len();
    }

    let output = if requests.len() == 1 {
        first_output.unwrap_or_else(|| json!({ "photos": [], "total": 0, "nextPage": null }))
    } else {
        json!({
            "requested": requests.len(),
            "succeeded": succeeded,
            "failed": failed,
            "total_photos": total_photos,
            "results": run_results,
        })
    };
    set_output_value(client, config, &output).await?;
    println!("Photos extraction completed");
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = reqwest::Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .context("Could not create HTTP client")?;
    timeout(ACTOR_TIMEOUT, run_actor(&client, &config))
        .await
        .map_err(|_| anyhow!("Actor timed out after {}s", ACTOR_TIMEOUT.as_secs()))??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        thread::{self, JoinHandle},
        time::Instant,
    };

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: Url,
        requests: Arc<Mutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let captured_requests = Arc::clone(&requests);
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                for response in responses {
                    let (mut stream, _) = loop {
                        if stopped.load(Ordering::Relaxed) || Instant::now() >= deadline {
                            return;
                        }
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(_) => return,
                        }
                    };
                    let request = read_request(&mut stream).unwrap_or_default();
                    if let Ok(mut captured) = captured_requests.lock() {
                        captured.push(request);
                    }
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        404 => "Not Found",
                        422 => "Unprocessable Entity",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
                        _ => "Mock Response",
                    };
                    let message = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    if stream.write_all(message.as_bytes()).is_err() {
                        return;
                    }
                }
            });
            Self {
                base_url: Url::parse(&format!("http://{address}/")).unwrap(),
                requests,
                stop,
                thread: Some(thread),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests
                .lock()
                .map(|requests| requests.clone())
                .unwrap_or_default()
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
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
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn mock_response(status: u16, body: impl Into<String>) -> MockResponse {
        MockResponse {
            status,
            body: body.into(),
        }
    }

    fn config(base_url: &Url) -> ActorConfig {
        let mut scrappa_api_base_url = base_url.clone();
        scrappa_api_base_url
            .path_segments_mut()
            .unwrap()
            .extend(["api"]);
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url,
            default_key_value_store_id: "store-id".to_owned(),
            default_dataset_id: "dataset-id".to_owned(),
            actor_run_id: "run-id".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-apify-token".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
            max_total_charge_usd: None,
        }
    }

    fn pricing_run(max_charge: f64, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-default-dataset-item": { "eventPriceUsd": 0.05 },
                            "apify-actor-start": { "eventPriceUsd": 0.02 }
                        }
                    }
                },
                "options": { "maxTotalChargeUsd": max_charge },
                "chargedEventCounts": counts
            }
        })
    }

    fn request_body(request: &str) -> Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
        request.lines().find_map(|line| {
            let (header_name, value) = line.split_once(':')?;
            header_name.eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }

    #[test]
    fn ppe_budget_accounts_for_other_charges_and_trims_dataset_items() {
        let run = pricing_run(0.12, json!({ "apify-actor-start": 1 }));
        assert_eq!(affordable_dataset_items(&run, 5, None, 0).unwrap(), Some(2));

        let partially_caught_up_run = pricing_run(
            0.17,
            json!({
                "apify-actor-start": 1,
                "apify-default-dataset-item": 1
            }),
        );
        assert_eq!(
            affordable_dataset_items(&partially_caught_up_run, 5, None, 2).unwrap(),
            Some(1)
        );
    }

    #[test]
    fn non_ppe_pricing_keeps_all_default_dataset_items() {
        let run =
            json!({ "data": { "pricingInfo": { "pricingModel": "PRICE_PER_DATASET_ITEM" } } });
        assert_eq!(affordable_dataset_items(&run, 5, None, 0).unwrap(), None);
    }

    #[test]
    fn exhausted_ppe_budget_allows_no_chargeable_dataset_items() {
        let run = pricing_run(0.02, json!({ "apify-actor-start": 1 }));
        assert_eq!(affordable_dataset_items(&run, 5, None, 0).unwrap(), Some(0));
    }

    #[test]
    fn ppe_budget_ignores_unpriced_events_and_treats_zero_as_unlimited() {
        let run = pricing_run(
            0.12,
            json!({ "apify-actor-start": 1, "unpriced-event": 100 }),
        );
        assert_eq!(affordable_dataset_items(&run, 5, None, 0).unwrap(), Some(2));

        let unlimited = pricing_run(0.0, json!({ "apify-actor-start": 100 }));
        assert_eq!(
            affordable_dataset_items(&unlimited, 5, Some(0.01), 0).unwrap(),
            Some(5)
        );
    }

    #[tokio::test]
    async fn ppe_budget_tracks_dataset_rows_when_run_charges_are_stale_across_businesses() {
        let input = json!({ "business_ids": ["ChIJone", "ChIJtwo"] });
        let photos = json!({
            "items": [
                { "photo_id": "p1" },
                { "photo_id": "p2" },
                { "photo_id": "p3" }
            ]
        })
        .to_string();
        let stale_run = pricing_run(0.12, json!({ "apify-actor-start": 1 })).to_string();
        let server = MockServer::start(vec![
            mock_response(200, input.to_string()),
            mock_response(200, photos.clone()),
            mock_response(200, stale_run.clone()),
            mock_response(201, ""),
            mock_response(200, photos),
            mock_response(200, stale_run),
            mock_response(201, ""),
            mock_response(201, ""),
        ]);
        let config = config(&server.base_url);
        let client = reqwest::Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .unwrap();

        run_actor(&client, &config).await.unwrap();

        let requests = server.requests();
        let dataset_items_written = requests
            .iter()
            .filter(|request| request.starts_with("POST /v2/datasets/dataset-id/items "))
            .map(|request| request_body(request).as_array().unwrap().len())
            .sum::<usize>();
        assert_eq!(dataset_items_written, 2);
        assert!(requests
            .last()
            .unwrap()
            .starts_with("PUT /v2/key-value-stores/store-id/records/OUTPUT "));
    }

    #[test]
    fn apify_retries_match_the_client_default_backoff() {
        assert_eq!(APIFY_MAX_RETRIES, 8);
        assert_eq!(apify_retry_delay(1), Duration::from_millis(500));
        assert_eq!(apify_retry_delay(2), Duration::from_secs(1));
        assert_eq!(apify_retry_delay(3), Duration::from_secs(2));
        assert!(retryable_apify_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(retryable_apify_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(!retryable_apify_status(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn maps_request_keeps_cache_query_compatibility() {
        let base_url = Url::parse("https://scrappa.example/api").unwrap();
        let without_cache = build_photos_url(
            &base_url,
            "0x123:0x456",
            Some(&json!({ "use_cache": false, "maximum_cache_age": 0 })),
        )
        .unwrap();
        assert_eq!(
            without_cache.query(),
            Some("business_id=0x123%3A0x456&maximum_cache_age=0")
        );

        let defaults = build_photos_url(&base_url, "ChIJtest", None).unwrap();
        assert_eq!(defaults.query(), Some("business_id=ChIJtest&use_cache=1"));
    }

    #[test]
    fn wrapped_and_direct_photo_responses_keep_pagination_context() {
        let wrapped = json!({ "items": [{ "photo_id": "one" }], "data": [{ "photo_id": "fallback" }], "nextPage": "cursor" });
        let (photos, next_page) = photo_results(&wrapped).unwrap();
        assert_eq!(photos, vec![json!({ "photo_id": "one" })]);
        assert_eq!(next_page, json!("cursor"));

        let (photos, next_page) = photo_results(&json!([{ "photo_id": "direct" }])).unwrap();
        assert_eq!(photos, vec![json!({ "photo_id": "direct" })]);
        assert_eq!(next_page, Value::Null);
    }

    #[test]
    fn mapped_dataset_photo_keeps_upstream_fields_and_overrides_business_ids() {
        assert_eq!(
            dataset_photo(
                &json!({ "photo_id": "p1", "business_id": "upstream" }),
                "input-value",
                "normalized-value"
            ),
            json!({
                "photo_id": "p1",
                "business_id": "normalized-value",
                "input_business_id": "input-value"
            })
        );
    }

    #[tokio::test]
    async fn single_business_keeps_prefill_semantics_output_shape_auth_and_ppe_cap() {
        let input = json!({
            "business_id": "0x123:0x456",
            "use_cache": true,
            "maximum_cache_age": 0
        });
        let run = pricing_run(0.12, json!({ "apify-actor-start": 1 }));
        let server = MockServer::start(vec![
            mock_response(200, input.to_string()),
            mock_response(
                200,
                json!({
                    "items": [
                        { "photo_id": "p1", "photo_url": "https://example.test/1" },
                        { "photo_id": "p2", "photo_url": "https://example.test/2" },
                        { "photo_id": "p3", "photo_url": "https://example.test/3" }
                    ],
                    "nextPage": "next-cursor"
                })
                .to_string(),
            ),
            mock_response(200, run.to_string()),
            mock_response(201, ""),
            mock_response(201, ""),
        ]);
        let config = config(&server.base_url);
        let client = reqwest::Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .unwrap();

        run_actor(&client, &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert!(requests[0].starts_with("GET /v2/key-value-stores/store-id/records/INPUT "));
        assert_eq!(
            header_value(&requests[0], "authorization"),
            Some("Bearer test-apify-token")
        );
        assert!(requests[1].starts_with(
            "GET /api/maps/photos?business_id=0x123%3A0x456&use_cache=1&maximum_cache_age=0 "
        ));
        assert_eq!(
            header_value(&requests[1], "x-api-key"),
            Some("test-scrappa-key")
        );
        assert_eq!(
            header_value(&requests[1], "accept"),
            Some("application/json")
        );
        assert!(requests[2].starts_with("GET /v2/actor-runs/run-id "));
        assert_eq!(
            request_body(&requests[3]),
            json!([
                {
                    "photo_id": "p1",
                    "photo_url": "https://example.test/1",
                    "input_business_id": "0x123:0x456",
                    "business_id": "0x123:0x456"
                },
                {
                    "photo_id": "p2",
                    "photo_url": "https://example.test/2",
                    "input_business_id": "0x123:0x456",
                    "business_id": "0x123:0x456"
                }
            ])
        );
        let output = request_body(&requests[4]);
        assert!(requests[4].starts_with("PUT /v2/key-value-stores/store-id/records/OUTPUT "));
        assert_eq!(output["total"], 3);
        assert_eq!(output["nextPage"], "next-cursor");
        assert_eq!(output["photos"], request_body(&requests[3]));
        assert_eq!(output["photos"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn apify_safe_methods_retry_but_dataset_post_and_permanent_scrappa_errors_do_not() {
        let server = MockServer::start(vec![
            mock_response(500, "temporary"),
            mock_response(200, "{\"ok\":true}"),
        ]);
        let client = reqwest::Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .unwrap();
        let request = client.get(server.base_url.join("retry").unwrap());
        let response = send_apify_request(&client, request).await.unwrap();
        assert!(response.status().is_success());
        assert_eq!(server.requests().len(), 2);

        let post_server = MockServer::start(vec![
            mock_response(500, "temporary after possible append"),
            mock_response(201, ""),
        ]);
        let post_request = client
            .post(post_server.base_url.join("dataset/items").unwrap())
            .json(&json!([{ "photo_id": "one" }]));
        let response = send_apify_request(&client, post_request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(post_server.requests().len(), 1);

        let put_server = MockServer::start(vec![
            mock_response(500, "temporary"),
            mock_response(201, ""),
        ]);
        let put_request = client
            .put(put_server.base_url.join("records/OUTPUT").unwrap())
            .json(&json!({ "photos": [] }));
        let response = send_apify_request(&client, put_request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(put_server.requests().len(), 2);

        let error_server = MockServer::start(vec![mock_response(400, "invalid business")]);
        let mut scrappa_base_url = error_server.base_url.clone();
        scrappa_base_url
            .path_segments_mut()
            .unwrap()
            .extend(["api"]);
        let scrappa = ScrappaClient {
            http: &client,
            base_url: &scrappa_base_url,
            api_key: "test-scrappa-key",
        };
        let error = scrappa.get_photos("0x123:0x456", None).await.unwrap_err();
        assert!(error.to_string().contains("Scrappa API error (400)"));
        assert_eq!(error_server.requests().len(), 1);
    }

    #[tokio::test]
    async fn transient_scrappa_unavailability_retries_get_then_returns_photos() {
        let server = MockServer::start(vec![
            mock_response(503, "temporarily unavailable"),
            mock_response(
                200,
                json!({ "photos": [{ "photo_id": "one" }] }).to_string(),
            ),
        ]);
        let mut base_url = server.base_url.clone();
        base_url.path_segments_mut().unwrap().extend(["api"]);
        let client = reqwest::Client::new();
        let scrappa = ScrappaClient {
            http: &client,
            base_url: &base_url,
            api_key: "test-key",
        };

        let response = scrappa.get_photos("0x123:0x456", None).await.unwrap();
        assert_eq!(response["photos"][0]["photo_id"], "one");
        assert_eq!(server.requests().len(), 2);
    }

    #[tokio::test]
    async fn dataset_append_transient_failure_is_not_retried() {
        let run = pricing_run(1.0, json!({ "apify-actor-start": 1 }));
        let server = MockServer::start(vec![
            mock_response(200, run.to_string()),
            mock_response(500, "append may already have succeeded"),
            mock_response(201, ""),
        ]);
        let config = config(&server.base_url);
        let client = reqwest::Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .unwrap();

        let result = push_dataset_items(
            &client,
            &config,
            &[json!({ "photo_id": "one" })],
            &mut DatasetBudget::default(),
        )
        .await;

        assert!(result.is_err());
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert!(requests[1].starts_with("POST /v2/datasets/dataset-id/items "));
    }

    #[tokio::test]
    async fn missing_apify_input_uses_the_actor_input_validation_error() {
        let server = MockServer::start(vec![mock_response(404, "input not found")]);
        let config = config(&server.base_url);
        let client = reqwest::Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .unwrap();

        let error = run_actor(&client, &config).await.unwrap_err();
        assert!(error.to_string().contains(
            "At least one Business ID is required. Provide business_ids or legacy business_id."
        ));
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn input_api_errors_stay_structured_and_preserve_summary_counts() {
        let input = json!({ "business_ids": ["not-found", "invalid"] });
        let run = pricing_run(1.0, json!({ "apify-actor-start": 1 }));
        let server = MockServer::start(vec![
            mock_response(200, input.to_string()),
            mock_response(404, "not found"),
            mock_response(200, run.to_string()),
            mock_response(201, ""),
            mock_response(422, "invalid"),
            mock_response(200, run.to_string()),
            mock_response(201, ""),
            mock_response(201, ""),
        ]);
        let config = config(&server.base_url);
        let client = reqwest::Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .unwrap();

        run_actor(&client, &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 8);
        assert_eq!(request_body(&requests[3])[0]["error"], "Business not found");
        assert_eq!(request_body(&requests[6])[0]["error"], "Invalid input");
        let output = request_body(&requests[7]);
        assert_eq!(output["requested"], 2);
        assert_eq!(output["succeeded"], 0);
        assert_eq!(output["failed"], 2);
        assert_eq!(output["total_photos"], 0);
        assert_eq!(output["results"][0]["error"], "Business not found");
        assert_eq!(output["results"][1]["error"], "Invalid input");
    }

    #[tokio::test]
    async fn non_input_scrappa_errors_fail_without_writing_output() {
        let input = json!({ "business_id": "0x123:0x456" });
        let server = MockServer::start(vec![
            mock_response(200, input.to_string()),
            mock_response(503, "upstream unavailable"),
            mock_response(503, "upstream unavailable"),
            mock_response(503, "upstream unavailable"),
        ]);
        let config = config(&server.base_url);
        let client = reqwest::Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .unwrap();

        let error = run_actor(&client, &config).await.unwrap_err();
        assert!(error.to_string().contains("Scrappa API error (503)"));
        assert_eq!(server.requests().len(), 4);
    }
}
