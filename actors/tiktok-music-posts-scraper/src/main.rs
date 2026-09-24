use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Map, Value};
use std::{collections::HashSet, env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: usize = 8;
const APIFY_RETRY_BASE_DELAY: Duration = Duration::from_millis(500);
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

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
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").unwrap_or_default();
        if scrappa_api_key.is_empty() {
            bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
        }

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
        .extend(segments.iter().copied());
    Ok(url)
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

fn safe_integer_string(value: &Value) -> Option<String> {
    let number = value.as_f64().filter(|number| {
        number.is_finite() && number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER
    })?;
    Some((number as i64).to_string())
}

fn normalize_music_id(value: &str) -> Result<String> {
    let music_id = js_trim(value);
    if music_id.is_empty() {
        return Ok(String::new());
    }
    if !music_id.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("TikTok music_id must contain digits only");
    }
    if music_id.len() > 100 {
        bail!("TikTok music_id must be 100 digits or fewer");
    }
    Ok(music_id.to_owned())
}

fn push_unique_music_id(music_ids: &mut Vec<String>, seen: &mut HashSet<String>, id: String) {
    if seen.insert(id.clone()) {
        music_ids.push(id);
    }
}

#[derive(Debug, PartialEq)]
struct MusicRequest {
    music_id: String,
    count: Option<i64>,
    cursor: Option<String>,
}

fn collect_music_ids(input: &Value) -> Result<Vec<String>> {
    let mut music_ids = Vec::new();
    let mut seen = HashSet::new();

    match input.get("musicIds") {
        Some(Value::Array(values)) => {
            for value in values {
                match value {
                    Value::String(value) => {
                        let id = normalize_music_id(value)?;
                        if !id.is_empty() {
                            push_unique_music_id(&mut music_ids, &mut seen, id);
                        }
                    }
                    Value::Number(_) => {
                        if let Some(id) = safe_integer_string(value) {
                            let id = normalize_music_id(&id)?;
                            if !id.is_empty() {
                                push_unique_music_id(&mut music_ids, &mut seen, id);
                            }
                        } else {
                            eprintln!(
                                "Warning: musicIds entries must be strings or safe integers, got number. Omitting entry."
                            );
                        }
                    }
                    value if !value.is_null() => eprintln!(
                        "Warning: musicIds entries must be strings or safe integers, got {}. Omitting entry.",
                        value_type(value)
                    ),
                    _ => {}
                }
            }
        }
        Some(value) if !value.is_null() => eprintln!(
            "Warning: musicIds must be an array, got {}. Falling back to music_id.",
            value_type(value)
        ),
        _ => {}
    }

    if !music_ids.is_empty() {
        return Ok(music_ids);
    }

    match input.get("music_id") {
        Some(Value::String(value)) => {
            let id = normalize_music_id(value)?;
            if !id.is_empty() {
                push_unique_music_id(&mut music_ids, &mut seen, id);
            }
        }
        Some(Value::Number(_)) => {
            if let Some(id) = input.get("music_id").and_then(safe_integer_string) {
                let id = normalize_music_id(&id)?;
                if !id.is_empty() {
                    push_unique_music_id(&mut music_ids, &mut seen, id);
                }
            } else {
                eprintln!("Warning: music_id must be a string or safe integer, got number.");
            }
        }
        Some(value) if !value.is_null() => eprintln!(
            "Warning: music_id must be a string or safe integer, got {}.",
            value_type(value)
        ),
        _ => {}
    }

    Ok(music_ids)
}

fn normalize_count(input: &Value) -> Option<i64> {
    let Some(value) = input.get("count") else {
        return None;
    };
    let count = value.as_f64().filter(|number| {
        number.is_finite() && number.fract() == 0.0 && (1.0..=50.0).contains(number)
    });
    if let Some(count) = count {
        return Some(count as i64);
    }
    eprintln!(
        "Warning: count must be an integer between 1 and 50, got {}. Using Scrappa default.",
        js_string(value)
    );
    None
}

fn normalize_cursor(input: &Value) -> Option<String> {
    match input.get("cursor") {
        Some(Value::String(value)) => {
            let cursor = js_trim(value);
            (!cursor.is_empty()).then(|| cursor.to_owned())
        }
        Some(Value::Number(value)) => {
            let cursor = value
                .as_f64()
                .filter(|number| {
                    number.is_finite() && number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER
                })
                .map(|number| (number as i64).to_string());
            if cursor.is_none() {
                eprintln!(
                    "Warning: cursor must be a string or safe integer, got {}. Starting from the first page.",
                    js_string(&Value::Number(value.clone()))
                );
            }
            cursor
        }
        Some(value) if !value.is_null() => {
            eprintln!(
                "Warning: cursor must be a string or number, got {}. Starting from the first page.",
                value_type(value)
            );
            None
        }
        _ => None,
    }
}

fn build_music_requests(input: &Value) -> Result<Vec<MusicRequest>> {
    let music_ids = collect_music_ids(input)?;
    if music_ids.is_empty() {
        bail!("At least one TikTok music_id is required");
    }
    let count = normalize_count(input);
    let cursor = normalize_cursor(input);
    Ok(music_ids
        .into_iter()
        .map(|music_id| MusicRequest {
            music_id,
            count,
            cursor: cursor.clone(),
        })
        .collect())
}

fn format_lookup_for_log(requests: &[MusicRequest]) -> String {
    match requests {
        [] => "unknown TikTok music".to_owned(),
        [request] => format!("music_id:{}", request.music_id),
        requests => format!("{} TikTok music IDs", requests.len()),
    }
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn extract_posts(data: Option<&Value>) -> Vec<&Value> {
    let Some(data) = data.filter(|data| js_truthy(data)) else {
        return Vec::new();
    };
    if let Some(posts) = data.as_array() {
        return posts.iter().collect();
    }
    for key in ["posts", "videos", "aweme_list"] {
        if let Some(posts) = data.get(key).and_then(Value::as_array) {
            return posts.iter().collect();
        }
    }
    Vec::new()
}

fn extract_pagination(data: Option<&Value>) -> (bool, Value) {
    let Some(data) = data.filter(|data| js_truthy(data)) else {
        return (false, Value::Null);
    };
    if data.is_array() {
        return (false, Value::Null);
    }
    let has_more = data
        .get("hasMore")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("has_more").filter(|value| !value.is_null()))
        .is_some_and(js_truthy);
    let cursor = data
        .get("cursor")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("max_cursor").filter(|value| !value.is_null()))
        .or_else(|| data.get("min_cursor").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null);
    (has_more, cursor)
}

fn enrich_post(post: &Value, music_id: &str) -> Value {
    let mut row = match post {
        Value::Object(fields) => fields.clone(),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        Value::String(value) => value
            .chars()
            .enumerate()
            .map(|(index, value)| (index.to_string(), Value::String(value.to_string())))
            .collect(),
        _ => Map::new(),
    };
    row.insert(
        "request_music_id".to_owned(),
        Value::String(music_id.to_owned()),
    );
    Value::Object(row)
}

fn build_scrappa_url(base_url: &Url, request: &MusicRequest) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["tiktok", "music", "posts"])?;
    url.set_query(None);
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("music_id", &request.music_id);
        if let Some(count) = request.count {
            query.append_pair("count", &count.to_string());
        }
        if let Some(cursor) = &request.cursor {
            query.append_pair("cursor", cursor);
        }
    }
    Ok(url)
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
            .filter(|message| !message.is_null())
            .map(js_string)
            .unwrap_or(fallback);
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
                    (!messages.is_empty()).then(|| format!("{field}: {messages}"))
                })
                .flatten()
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

async fn fetch_scrappa_response(
    client: &Client,
    config: &ActorConfig,
    request: &MusicRequest,
) -> Result<Value> {
    let url = build_scrappa_url(&config.scrappa_api_base_url, request)?;
    let response = client
        .get(url)
        .header("X-API-Key", &config.scrappa_api_key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(scrappa_transport_error)?;
    let status = response.status();
    let body = response.text().await.map_err(scrappa_transport_error)?;
    if !status.is_success() {
        let message = scrappa_error_message(status, &body);
        bail!("Scrappa API error ({}): {message}", status.as_u16());
    }
    serde_json::from_str(&body).context("Scrappa API response was not valid JSON")
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

fn apify_retry_delay(retry: usize) -> Duration {
    let multiplier = 1_u32 << retry.saturating_sub(1).min(7);
    APIFY_RETRY_BASE_DELAY * multiplier
}

async fn send_apify_request<F>(mut build_request: F, operation: &str) -> Result<Response>
where
    F: FnMut() -> RequestBuilder,
{
    let mut retries = 0;
    loop {
        match build_request().send().await {
            Ok(response)
                if retries < APIFY_MAX_RETRIES
                    && (response.status() == StatusCode::TOO_MANY_REQUESTS
                        || response.status().is_server_error()) =>
            {
                let status = response.status();
                retries += 1;
                let delay = apify_retry_delay(retries);
                eprintln!(
                    "{operation} returned HTTP {status}; retrying attempt {retries}/{APIFY_MAX_RETRIES} in {}ms.",
                    delay.as_millis()
                );
                drop(response);
                tokio::time::sleep(delay).await;
            }
            Ok(response) => return Ok(response),
            Err(error) if retries < APIFY_MAX_RETRIES => {
                retries += 1;
                let delay = apify_retry_delay(retries);
                eprintln!(
                    "{operation} request failed: {error}; retrying attempt {retries}/{APIFY_MAX_RETRIES} in {}ms.",
                    delay.as_millis()
                );
                tokio::time::sleep(delay).await;
            }
            Err(error) => {
                return Err(error).with_context(|| format!("{operation} request failed"));
            }
        }
    }
}

async fn ensure_success(response: Response, operation: &str) -> Result<()> {
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
    Ok(())
}

async fn get_input(client: &Client, config: &ActorConfig) -> Result<Option<Value>> {
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
        || client.get(url.clone()).bearer_auth(&config.apify_token),
        "Apify INPUT",
    )
    .await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let input = response_json(response, "Apify INPUT request").await?;
    Ok((!input.is_null()).then_some(input))
}

async fn put_output(client: &Client, config: &ActorConfig, output: &Value) -> Result<()> {
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
        || {
            client
                .put(url.clone())
                .bearer_auth(&config.apify_token)
                .json(output)
        },
        "Apify OUTPUT write",
    )
    .await?;
    ensure_success(response, "Apify OUTPUT write").await
}

#[derive(Default)]
struct DatasetBudget {
    run: Option<Value>,
    saved_rows: usize,
}

impl DatasetBudget {
    async fn affordable_count(
        &mut self,
        client: &Client,
        config: &ActorConfig,
        requested: usize,
    ) -> Result<usize> {
        if self.run.is_none() {
            let url = endpoint_url(
                &config.apify_api_base_url,
                &["v2", "actor-runs", &config.actor_run_id],
            )?;
            let response = send_apify_request(
                || client.get(url.clone()).bearer_auth(&config.apify_token),
                "Apify run pricing",
            )
            .await?;
            self.run = Some(response_json(response, "Apify run pricing request").await?);
        }
        let run = self
            .run
            .as_ref()
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        affordable_dataset_items(run, requested, self.saved_rows)
    }

    fn charge_limit_reached(&self, requested: usize, saved: usize) -> Result<bool> {
        if saved < requested {
            return Ok(true);
        }
        let run = self
            .run
            .as_ref()
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        Ok(affordable_dataset_items(run, 1, self.saved_rows)? == 0)
    }
}

fn affordable_dataset_items(
    run: &Value,
    requested: usize,
    locally_saved_rows: usize,
) -> Result<usize> {
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
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        Some(Value::Null) | None => return Ok(requested),
        Some(value) => value
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?,
    };
    if max_charge == 0.0 {
        return Ok(requested);
    }
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = events
        .get(DEFAULT_DATASET_ITEM_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    if !item_price.is_finite() || item_price < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    for (event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count"))?;
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
    spent += item_price * locally_saved_rows as f64;
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .count())
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    budget: &mut DatasetBudget,
    rows: &[Value],
) -> Result<(usize, bool)> {
    if rows.is_empty() {
        return Ok((0, false));
    }
    let affordable_count = budget.affordable_count(client, config, rows.len()).await?;
    let saved_count = rows.len().min(affordable_count);
    if saved_count > 0 {
        let url = endpoint_url(
            &config.apify_api_base_url,
            &["v2", "datasets", &config.default_dataset_id, "items"],
        )?;
        let response = send_apify_request(
            || {
                client
                    .post(url.clone())
                    .bearer_auth(&config.apify_token)
                    .json(&rows[..saved_count])
            },
            "Apify dataset write",
        )
        .await?;
        ensure_success(response, "Apify dataset write").await?;
        budget.saved_rows += saved_count;
    }
    let charge_limit_reached = budget.charge_limit_reached(rows.len(), saved_count)?;
    Ok((saved_count, charge_limit_reached))
}

fn validate_scrappa_code(response: &Value) -> Result<()> {
    let Some(code) = response.get("code") else {
        return Ok(());
    };
    if code.as_f64().is_some_and(|code| code == 0.0) {
        return Ok(());
    }
    let message = response
        .get("msg")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or_else(|| "Unknown error".to_owned());
    bail!(
        "Scrappa TikTok Music Posts API returned code {}: {message}",
        js_string(code)
    );
}

fn response_items(response: &Value, request: &MusicRequest) -> Vec<Value> {
    extract_posts(response.get("data"))
        .into_iter()
        .map(|post| enrich_post(post, &request.music_id))
        .collect()
}

async fn run_actor(
    scrappa_client: &Client,
    apify_client: &Client,
    config: &ActorConfig,
) -> Result<()> {
    let input = get_input(apify_client, config)
        .await?
        .filter(js_truthy)
        .ok_or_else(|| anyhow!("At least one TikTok music_id is required"))?;
    let requests = build_music_requests(&input)?;
    println!(
        "Fetching TikTok music posts for: {}",
        format_lookup_for_log(&requests)
    );

    let mut budget = DatasetBudget::default();
    let mut results = Vec::new();
    let mut total_posts = 0;
    let mut charge_limit_reached = false;

    for request in &requests {
        println!("Fetching TikTok posts for music_id:{}", request.music_id);
        let response = fetch_scrappa_response(scrappa_client, config, request).await?;
        validate_scrappa_code(&response)?;
        let posts = response_items(&response, request);
        let (has_next_page, next_cursor) = extract_pagination(response.get("data"));
        let (saved_posts, request_charge_limit_reached) = if posts.is_empty() {
            println!("No posts found for music_id:{}", request.music_id);
            (0, false)
        } else {
            let (saved_posts, charge_limit_reached) =
                push_dataset_items(apify_client, config, &mut budget, &posts).await?;
            println!(
                "Saved {saved_posts} of {} posts for music_id:{}",
                posts.len(),
                request.music_id
            );
            (saved_posts, charge_limit_reached)
        };
        let processed_time = response
            .get("processed_time")
            .filter(|processed_time| !processed_time.is_null())
            .cloned()
            .unwrap_or(Value::Null);
        total_posts += saved_posts;
        results.push(json!({
            "request_music_id": request.music_id,
            "posts_extracted": saved_posts,
            "posts_returned": posts.len(),
            "has_next_page": has_next_page,
            "next_cursor": next_cursor,
            "processed_time": processed_time,
            "charge_limit_reached": request_charge_limit_reached,
        }));

        if request_charge_limit_reached {
            charge_limit_reached = true;
            println!(
                "Apify event charge limit reached. Stopping before fetching additional music IDs."
            );
            break;
        }
    }

    let summary = json!({
        "music_ids_processed": results.len(),
        "posts_extracted": total_posts,
        "charge_limit_reached": charge_limit_reached,
        "results": results,
    });
    put_output(apify_client, config, &summary).await?;
    println!("TikTok music posts extraction completed successfully");
    println!("Results summary: {summary}");
    Ok(())
}

#[tokio::main]
async fn main() {
    let result = async {
        let config = ActorConfig::from_env()?;
        let scrappa_client = Client::builder()
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .build()
            .context("Failed to create Scrappa HTTP client")?;
        let apify_client = Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .context("Failed to create Apify HTTP client")?;
        run_actor(&scrappa_client, &apify_client, &config).await
    }
    .await;
    if let Err(error) = result {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{SocketAddr, TcpListener, TcpStream},
        thread,
    };

    #[derive(Debug)]
    struct RecordedRequest {
        method_and_path: String,
        headers: String,
        body: String,
    }

    fn value(source: &str) -> Value {
        serde_json::from_str(source).unwrap()
    }

    fn request_config(address: SocketAddr) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: Url::parse(&format!("http://{address}")).unwrap(),
            scrappa_api_base_url: Url::parse(&format!("http://{address}/api")).unwrap(),
            default_key_value_store_id: "store-test".to_owned(),
            default_dataset_id: "dataset-test".to_owned(),
            actor_run_id: "run-test".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "actor-token".to_owned(),
            scrappa_api_key: "scrappa-key".to_owned(),
        }
    }

    fn find_header_end(buffer: &[u8]) -> Option<usize> {
        buffer.windows(4).position(|window| window == b"\r\n\r\n")
    }

    fn read_request(stream: &mut TcpStream) -> RecordedRequest {
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut buffer = Vec::new();
        let mut chunk = [0; 4096];
        let (header_end, content_length) = loop {
            let bytes_read = stream.read(&mut chunk).unwrap();
            assert_ne!(bytes_read, 0, "request ended before its headers arrived");
            buffer.extend_from_slice(&chunk[..bytes_read]);
            if let Some(header_end) = find_header_end(&buffer) {
                let headers = String::from_utf8_lossy(&buffer[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if buffer.len() >= header_end + 4 + content_length {
                    break (header_end, content_length);
                }
            }
        };
        let header_text = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
        let mut lines = header_text.lines();
        let method_and_path = lines.next().unwrap_or_default().to_owned();
        let headers = lines.collect::<Vec<_>>().join("\n");
        let body =
            String::from_utf8_lossy(&buffer[header_end + 4..header_end + 4 + content_length])
                .into_owned();
        RecordedRequest {
            method_and_path,
            headers,
            body,
        }
    }

    fn mock_response(stream: &mut TcpStream, status: &str, body: &str) {
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
    }

    fn start_mock_server(
        responses: Vec<(String, String)>,
    ) -> (SocketAddr, thread::JoinHandle<Vec<RecordedRequest>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            responses
                .into_iter()
                .map(|(status, body)| {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_request(&mut stream);
                    mock_response(&mut stream, &status, &body);
                    request
                })
                .collect()
        });
        (address, server)
    }

    fn pricing_body(max_charge: Option<f64>, counts: Value) -> String {
        let max_charge = max_charge.map(Value::from).unwrap_or(Value::Null);
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-default-dataset-item": { "eventPriceUsd": 0.0003 },
                            "start-event": { "eventPriceUsd": 0.1 }
                        }
                    }
                },
                "options": { "maxTotalChargeUsd": max_charge },
                "chargedEventCounts": counts
            }
        })
        .to_string()
    }

    fn input_response(input: &str) -> (String, String) {
        ("200 OK".to_owned(), input.to_owned())
    }

    fn posts_response(posts: &str, pagination: &str) -> (String, String) {
        (
            "200 OK".to_owned(),
            format!(r#"{{"code":0,"data":{{"posts":{posts},{pagination}}},"processed_time":99}}"#),
        )
    }

    fn success_response() -> (String, String) {
        ("201 Created".to_owned(), String::new())
    }

    #[test]
    fn builds_requests_for_multiple_and_legacy_ids() {
        assert_eq!(
            build_music_requests(&value(
                r#"{"musicIds":[" 7001 ","7002","7001"],"music_id":"7003","count":10,"cursor":" 0 "}"#
            ))
            .unwrap(),
            vec![
                MusicRequest {
                    music_id: "7001".to_owned(),
                    count: Some(10),
                    cursor: Some("0".to_owned()),
                },
                MusicRequest {
                    music_id: "7002".to_owned(),
                    count: Some(10),
                    cursor: Some("0".to_owned()),
                },
            ]
        );
        assert_eq!(
            build_music_requests(&value(r#"{"musicIds":[],"music_id":7003}"#)).unwrap(),
            vec![MusicRequest {
                music_id: "7003".to_owned(),
                count: None,
                cursor: None,
            }]
        );
    }

    #[test]
    fn uses_music_id_fallback_when_music_ids_is_not_an_array() {
        assert_eq!(
            build_music_requests(&value(r#"{"musicIds":"7001","music_id":"7002"}"#)).unwrap(),
            vec![MusicRequest {
                music_id: "7002".to_owned(),
                count: None,
                cursor: None,
            }]
        );
    }

    #[test]
    fn rejects_missing_invalid_and_oversized_music_ids() {
        assert!(
            build_music_requests(&value(r#"{"musicIds":[],"music_id":""}"#))
                .unwrap_err()
                .to_string()
                .contains("At least one TikTok music_id")
        );
        assert!(normalize_music_id("track-name")
            .unwrap_err()
            .to_string()
            .contains("digits only"));
        assert!(normalize_music_id(&"1".repeat(101))
            .unwrap_err()
            .to_string()
            .contains("100 digits or fewer"));
    }

    #[test]
    fn invalid_count_and_cursor_values_fall_back_to_scrappa_defaults() {
        assert_eq!(
            build_music_requests(&value(
                r#"{"music_id":"7001","count":51,"cursor":9007199254740992}"#
            ))
            .unwrap(),
            vec![MusicRequest {
                music_id: "7001".to_owned(),
                count: None,
                cursor: None,
            }]
        );
        assert_eq!(
            build_music_requests(&value(r#"{"music_id":"7001","cursor":123}"#)).unwrap()[0].cursor,
            Some("123".to_owned())
        );
    }

    #[test]
    fn extracts_post_variants_and_pagination_fields() {
        let posts = value(r#"[{"aweme_id":"1"}]"#);
        assert_eq!(extract_posts(Some(&posts)), vec![&posts[0]]);
        let response = value(
            r#"{"videos":[{"aweme_id":"2"}],"hasMore":true,"has_more":false,"cursor":"10","max_cursor":"20"}"#,
        );
        assert_eq!(extract_posts(Some(&response)), vec![&response["videos"][0]]);
        assert_eq!(
            extract_pagination(Some(&response)),
            (true, Value::String("10".to_owned()))
        );
        let fallback =
            value(r#"{"aweme_list":[{"aweme_id":"3"}],"has_more":true,"max_cursor":"30"}"#);
        assert_eq!(
            extract_pagination(Some(&fallback)),
            (true, Value::String("30".to_owned()))
        );
        assert!(extract_posts(Some(&Value::Null)).is_empty());
    }

    #[test]
    fn numeric_zero_is_the_only_success_scrappa_code() {
        assert!(validate_scrappa_code(&value(r#"{"code":0}"#)).is_ok());
        assert!(validate_scrappa_code(&value(r#"{"code":"0"}"#))
            .unwrap_err()
            .to_string()
            .contains("code 0"));
    }

    #[test]
    fn positive_spending_limit_accounts_for_other_charged_events_and_saved_rows() {
        let run = value(&pricing_body(Some(0.101), json!({ "start-event": 1 })));
        assert_eq!(affordable_dataset_items(&run, 5, 0).unwrap(), 3);
        assert_eq!(affordable_dataset_items(&run, 1, 3).unwrap(), 0);
    }

    #[test]
    fn allows_non_ppe_runs_without_pricing_event_details() {
        let run = json!({ "data": { "pricingInfo": { "pricingModel": "FREE" } } });
        assert_eq!(affordable_dataset_items(&run, 50, 0).unwrap(), 50);
    }

    #[test]
    fn treats_missing_null_and_zero_spending_limits_as_uncapped() {
        let mut missing_limit = value(&pricing_body(Some(1.0), json!({})));
        missing_limit["data"]["options"]
            .as_object_mut()
            .unwrap()
            .remove("maxTotalChargeUsd");
        assert_eq!(affordable_dataset_items(&missing_limit, 50, 0).unwrap(), 50);

        let uncapped = value(&pricing_body(None, json!({})));
        assert_eq!(affordable_dataset_items(&uncapped, 50, 0).unwrap(), 50);

        let zero_limit = value(&pricing_body(Some(0.0), json!({})));
        assert_eq!(affordable_dataset_items(&zero_limit, 50, 0).unwrap(), 50);

        let mut free_event = value(&pricing_body(Some(0.0), json!({})));
        free_event["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            [DEFAULT_DATASET_ITEM_EVENT]["eventPriceUsd"] = json!(0);
        assert_eq!(affordable_dataset_items(&free_event, 2, 0).unwrap(), 2);
    }

    #[test]
    fn preserves_apify_prefill_and_schema_contract() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["musicIds"]["prefill"],
            json!(["7002634556977908485"])
        );
        assert!(schema["properties"]["music_id"].get("prefill").is_none());
        assert_eq!(schema["properties"]["cursor"]["type"], "string");
        assert_eq!(schema["properties"]["cursor"]["editor"], "textfield");
    }

    #[tokio::test]
    async fn fetches_posts_with_existing_auth_and_pagination_query_and_writes_output() {
        let input = r#"{"musicIds":["7001"],"count":2,"cursor":"0"}"#;
        let responses = vec![
            input_response(input),
            posts_response(
                r#"[{"aweme_id":"1"},{"aweme_id":"2"}]"#,
                r#""hasMore":true,"cursor":"20""#,
            ),
            ("200 OK".to_owned(), pricing_body(Some(1.0), json!({}))),
            success_response(),
            success_response(),
        ];
        let (address, server) = start_mock_server(responses);
        let config = request_config(address);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        run_actor(&client, &client, &config).await.unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 5);
        assert!(requests[0]
            .method_and_path
            .starts_with("GET /v2/key-value-stores/store-test/records/INPUT "));
        assert!(requests[1]
            .method_and_path
            .contains("/api/tiktok/music/posts?music_id=7001&count=2&cursor=0"));
        assert!(requests[1].headers.contains("x-api-key: scrappa-key"));
        assert!(requests[2]
            .method_and_path
            .starts_with("GET /v2/actor-runs/run-test "));
        assert!(requests[3]
            .method_and_path
            .starts_with("POST /v2/datasets/dataset-test/items "));
        assert!(requests[3]
            .headers
            .contains("authorization: Bearer actor-token"));
        let rows: Value = serde_json::from_str(&requests[3].body).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 2);
        assert_eq!(rows[0]["request_music_id"], "7001");
        assert!(requests[4]
            .method_and_path
            .starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT "));
        let summary: Value = serde_json::from_str(&requests[4].body).unwrap();
        assert_eq!(summary["music_ids_processed"], 1);
        assert_eq!(summary["posts_extracted"], 2);
        assert_eq!(summary["results"][0]["posts_returned"], 2);
        assert_eq!(summary["results"][0]["has_next_page"], true);
        assert_eq!(summary["results"][0]["next_cursor"], "20");
    }

    #[tokio::test]
    async fn caps_dataset_writes_at_the_ppe_budget_and_stops_before_the_next_music_id() {
        let responses = vec![
            input_response(r#"{"musicIds":["7001","7002"],"count":2}"#),
            posts_response(
                r#"[{"aweme_id":"1"},{"aweme_id":"2"}]"#,
                r#""hasMore":false"#,
            ),
            ("200 OK".to_owned(), pricing_body(Some(0.0003), json!({}))),
            success_response(),
            success_response(),
        ];
        let (address, server) = start_mock_server(responses);
        let config = request_config(address);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        run_actor(&client, &client, &config).await.unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 5);
        let rows: Value = serde_json::from_str(&requests[3].body).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 1);
        assert_eq!(rows[0]["aweme_id"], "1");
        let summary: Value = serde_json::from_str(&requests[4].body).unwrap();
        assert_eq!(summary["posts_extracted"], 1);
        assert_eq!(summary["charge_limit_reached"], true);
        assert_eq!(summary["results"][0]["posts_returned"], 2);
        assert_eq!(summary["results"][0]["charge_limit_reached"], true);
    }

    #[tokio::test]
    async fn exact_budget_exhaustion_stops_before_requesting_another_music_id() {
        let responses = vec![
            input_response(r#"{"musicIds":["7001","7002"],"count":2}"#),
            posts_response(
                r#"[{"aweme_id":"1"},{"aweme_id":"2"}]"#,
                r#""hasMore":false"#,
            ),
            ("200 OK".to_owned(), pricing_body(Some(0.0006), json!({}))),
            success_response(),
            success_response(),
        ];
        let (address, server) = start_mock_server(responses);
        let config = request_config(address);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        run_actor(&client, &client, &config).await.unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 5);
        let summary: Value = serde_json::from_str(&requests[4].body).unwrap();
        assert_eq!(summary["posts_extracted"], 2);
        assert_eq!(summary["charge_limit_reached"], true);
        assert_eq!(summary["music_ids_processed"], 1);
    }

    #[tokio::test]
    async fn scrappa_error_fails_without_dataset_or_output_writes_or_retries() {
        let responses = vec![
            input_response(r#"{"music_id":"7001"}"#),
            (
                "503 Service Unavailable".to_owned(),
                r#"{"message":"Try again later","errors":{"music_id":["busy"]}}"#.to_owned(),
            ),
        ];
        let (address, server) = start_mock_server(responses);
        let config = request_config(address);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let error = run_actor(&client, &client, &config).await.unwrap_err();
        let requests = server.join().unwrap();

        assert!(error
            .to_string()
            .contains("Scrappa API error (503): Try again later - music_id: busy"));
        assert_eq!(requests.len(), 2);
    }

    #[tokio::test]
    async fn dataset_write_error_fails_without_output_write() {
        let responses = vec![
            input_response(r#"{"music_id":"7001"}"#),
            posts_response(r#"[{"aweme_id":"1"}]"#, r#""hasMore":false"#),
            ("200 OK".to_owned(), pricing_body(Some(1.0), json!({}))),
            (
                "400 Bad Request".to_owned(),
                "dataset write failed".to_owned(),
            ),
        ];
        let (address, server) = start_mock_server(responses);
        let config = request_config(address);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let error = run_actor(&client, &client, &config).await.unwrap_err();
        let requests = server.join().unwrap();

        assert!(error.to_string().contains("Apify dataset write failed"));
        assert_eq!(requests.len(), 4);
        assert!(requests
            .iter()
            .all(|request| !request.method_and_path.contains("/records/OUTPUT")));
    }

    #[tokio::test]
    async fn apify_api_retries_server_errors_with_a_rebuilt_request() {
        let (address, server) = start_mock_server(vec![
            (
                "503 Service Unavailable".to_owned(),
                r#"{"message":"retry"}"#.to_owned(),
            ),
            ("200 OK".to_owned(), r#"{"ok":true}"#.to_owned()),
        ]);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let url = Url::parse(&format!("http://{address}/v2/test")).unwrap();
        let response = send_apify_request(|| client.get(url.clone()), "test Apify request")
            .await
            .unwrap();

        assert_eq!(
            response_json(response, "test Apify request").await.unwrap()["ok"],
            true
        );
        assert_eq!(server.join().unwrap().len(), 2);
    }
}
