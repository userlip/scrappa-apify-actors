use anyhow::{anyhow, bail, Context, Result};
use reqwest::Response;
use serde_json::{json, Map, Value};
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/tiktok/feed/search";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

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
        .extend(segments);
    Ok(url)
}

#[derive(Debug, PartialEq)]
struct SearchParams {
    keywords: String,
    region: Option<String>,
    count: Option<i64>,
    cursor: Option<String>,
    publish_time: Option<i64>,
    sort_type: Option<i64>,
}

impl SearchParams {
    fn append_to_url(&self, url: &mut Url) {
        let mut query = url.query_pairs_mut();
        query.append_pair("keywords", &self.keywords);
        if let Some(region) = &self.region {
            query.append_pair("region", region);
        }
        if let Some(count) = self.count {
            query.append_pair("count", &count.to_string());
        }
        if let Some(cursor) = &self.cursor {
            query.append_pair("cursor", cursor);
        }
        if let Some(publish_time) = self.publish_time {
            query.append_pair("publish_time", &publish_time.to_string());
        }
        if let Some(sort_type) = self.sort_type {
            query.append_pair("sort_type", &sort_type.to_string());
        }
    }

    fn metadata(&self) -> [(&'static str, Value); 6] {
        [
            ("request_keywords", Value::String(self.keywords.clone())),
            ("request_region", optional_string(&self.region)),
            ("request_count", optional_integer(self.count)),
            ("request_cursor", optional_string(&self.cursor)),
            ("request_publish_time", optional_integer(self.publish_time)),
            ("request_sort_type", optional_integer(self.sort_type)),
        ]
    }
}

fn optional_string(value: &Option<String>) -> Value {
    value.clone().map(Value::String).unwrap_or(Value::Null)
}

fn optional_integer(value: Option<i64>) -> Value {
    value.map(Value::from).unwrap_or(Value::Null)
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

fn normalize_keywords(value: &str) -> Result<String> {
    let normalized = value
        .split(is_js_whitespace)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        return Ok(normalized);
    }
    if normalized.encode_utf16().count() > 255 {
        bail!("TikTok search keywords must be 255 characters or fewer");
    }
    if value
        .chars()
        .any(|character| matches!(character, '\r' | '\n' | '\t' | '\u{000c}' | '\u{000b}'))
    {
        bail!("TikTok search keywords cannot contain tabs, line breaks, or control whitespace");
    }
    Ok(normalized)
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

fn normalize_region(value: &str) -> Result<Option<String>> {
    let region = js_trim(value).to_uppercase();
    if region.is_empty() {
        return Ok(None);
    }
    if !(2..=10).contains(&region.len()) || !region.bytes().all(|byte| byte.is_ascii_uppercase()) {
        bail!("region must be a 2 to 10 character country or region code");
    }
    Ok(Some(region))
}

fn normalize_integer(value: Option<&Value>, field: &str, min: i64, max: i64) -> Option<i64> {
    let Some(value) = value else {
        return None;
    };
    if value.is_null() || matches!(value, Value::String(value) if value.is_empty()) {
        return None;
    }
    let parsed = value.as_f64().filter(|number| {
        number.is_finite()
            && number.fract() == 0.0
            && *number >= min as f64
            && *number <= max as f64
    });
    if let Some(number) = parsed {
        return Some(number as i64);
    }
    eprintln!(
        "Warning: {field} must be an integer between {min} and {max}, got {}. Omitting {field}.",
        js_string(value)
    );
    None
}

fn normalize_cursor(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(value)) => {
            let cursor = js_trim(value);
            (!cursor.is_empty()).then(|| cursor.to_owned())
        }
        Some(Value::Number(number)) => {
            let parsed = number.as_f64();
            if let Some(number) = parsed.filter(|number| {
                number.is_finite()
                    && number.fract() == 0.0
                    && number.abs() <= 9_007_199_254_740_991.0
            }) {
                Some((number as i64).to_string())
            } else {
                eprintln!("Warning: cursor must be a string or safe integer, got {}. Starting from the first page.", number);
                None
            }
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

fn build_search_params(input: &Value) -> Result<SearchParams> {
    let keywords = match input.get("keywords") {
        Some(Value::String(value)) => {
            let value = normalize_keywords(value)?;
            (!value.is_empty()).then_some(value)
        }
        Some(value) if !value.is_null() => {
            eprintln!(
                "Warning: keywords must be a string, got {}.",
                value_type(value)
            );
            None
        }
        _ => None,
    };
    let keywords = match keywords {
        Some(keywords) => Some(keywords),
        None => match input.get("query") {
            Some(Value::String(value)) => {
                let value = normalize_keywords(value)?;
                (!value.is_empty()).then_some(value)
            }
            Some(value) if !value.is_null() => {
                eprintln!(
                    "Warning: query must be a string, got {}.",
                    value_type(value)
                );
                None
            }
            _ => None,
        },
    };
    let Some(keywords) = keywords else {
        bail!("TikTok search keywords are required");
    };

    let region = match input.get("region") {
        Some(Value::String(value)) => normalize_region(value)?,
        Some(value) if !value.is_null() => {
            eprintln!(
                "Warning: region must be a string, got {}. Omitting region.",
                value_type(value)
            );
            None
        }
        _ => None,
    };
    let count = normalize_integer(input.get("count"), "count", 1, 50);
    let cursor = normalize_cursor(input.get("cursor"));
    let publish_time = normalize_integer(input.get("publish_time"), "publish_time", 0, 3650);
    let sort_type = normalize_integer(input.get("sort_type"), "sort_type", 0, 10);

    Ok(SearchParams {
        keywords,
        region,
        count,
        cursor,
        publish_time,
        sort_type,
    })
}

fn format_lookup_for_log(input: &Value) -> &str {
    input
        .get("keywords")
        .and_then(Value::as_str)
        .map(js_trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            input
                .get("query")
                .and_then(Value::as_str)
                .map(js_trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or("unknown TikTok search")
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

async fn ensure_success(response: Response, operation: &str) -> Result<()> {
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
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify INPUT request failed")?;
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    response_json(response, "Apify INPUT request")
        .await
        .map(Some)
}

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            SCRAPPA_REQUEST_TIMEOUT.as_millis()
        )
    } else {
        anyhow!(error.to_string())
    }
}

async fn read_scrappa_error(response: Response) -> Result<String> {
    let status = response.status();
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) if error.is_timeout() => return Err(scrappa_request_error(error)),
        Err(_) => return Ok(fallback),
    };
    if body.is_empty() {
        return Ok(fallback);
    }
    if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
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
                    Some(format!(
                        "{field}: {}",
                        messages
                            .iter()
                            .map(|message| if message.is_null() {
                                String::new()
                            } else {
                                js_string(message)
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details);
            }
        }
        return Ok(message);
    }
    Ok(body
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect())
}

async fn fetch_scrappa_response(
    client: &reqwest::Client,
    config: &ActorConfig,
    params: &SearchParams,
) -> Result<Value> {
    let mut url = config.scrappa_api_base_url.clone();
    params.append_to_url(&mut url);
    println!("[Scrappa] GET {url}");
    let response = client
        .get(url)
        .header("X-API-Key", &config.scrappa_api_key)
        .header("Accept", "application/json")
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(scrappa_request_error)?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let message = read_scrappa_error(response).await?;
        bail!("Scrappa API error ({status}): {message}");
    }
    response.json().await.map_err(scrappa_request_error)
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_none_or(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn extract_videos(data: Option<&Value>) -> Vec<&Value> {
    let Some(data) = data.filter(|data| js_truthy(data)) else {
        return Vec::new();
    };
    if let Some(videos) = data.as_array() {
        return videos.iter().collect();
    }
    for key in ["videos", "posts", "aweme_list", "item_list"] {
        if let Some(videos) = data.get(key).and_then(Value::as_array) {
            return videos.iter().collect();
        }
    }
    Vec::new()
}

fn extract_pagination(data: Option<&Value>) -> (bool, Value) {
    let Some(data) = data
        .filter(|data| js_truthy(data))
        .filter(|data| !data.is_array())
    else {
        return (false, Value::Null);
    };
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

fn enrich_video(video: &Value, params: &SearchParams) -> Value {
    let mut row = match video {
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
    for (key, value) in params.metadata() {
        row.insert(key.to_owned(), value);
    }
    Value::Object(row)
}

#[derive(Default)]
struct DatasetBudget {
    run: Option<Value>,
    saved_rows: usize,
}

impl DatasetBudget {
    async fn capacity(
        &mut self,
        client: &reqwest::Client,
        config: &ActorConfig,
        requested: usize,
    ) -> Result<usize> {
        if self.run.is_none() {
            let url = endpoint_url(
                &config.apify_api_base_url,
                &["v2", "actor-runs", &config.actor_run_id],
            )?;
            let response = client
                .get(url)
                .bearer_auth(&config.apify_token)
                .send()
                .await
                .context("Apify run pricing request failed")?;
            self.run = Some(response_json(response, "Apify run pricing request").await?);
        }
        let run = self
            .run
            .as_ref()
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        affordable_dataset_items(run, requested, self.saved_rows)
    }
}

// maxItems is for pay-per-result; PPE dataset writes consume the run's event budget.
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
        bail!("Apify run is not configured for pay-per-event pricing");
    }
    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = events
        .get("apify-default-dataset-item")
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
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
    if item_price == 0.0 {
        return Ok(requested);
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .count())
}

async fn push_dataset_items(
    client: &reqwest::Client,
    config: &ActorConfig,
    budget: &mut DatasetBudget,
    rows: &[Value],
) -> Result<usize> {
    if rows.is_empty() {
        return Ok(0);
    }
    let limit = budget.capacity(client, config, rows.len()).await?;
    let rows = &rows[..rows.len().min(limit)];
    if rows.is_empty() {
        return Ok(0);
    }
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(rows)
        .send()
        .await
        .context("Apify dataset write failed")?;
    ensure_success(response, "Apify dataset write").await?;
    budget.saved_rows += rows.len();
    Ok(rows.len())
}

async fn put_output(client: &reqwest::Client, config: &ActorConfig, output: &Value) -> Result<()> {
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
    let response = client
        .put(url)
        .bearer_auth(&config.apify_token)
        .json(output)
        .send()
        .await
        .context("Apify OUTPUT write failed")?;
    ensure_success(response, "Apify OUTPUT write").await
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
        "Scrappa TikTok Feed Search API returned code {}: {message}",
        js_string(code)
    );
}

async fn run_actor(client: &reqwest::Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let Some(input) = input.filter(js_truthy) else {
        bail!("TikTok search keywords are required");
    };
    let params = build_search_params(&input)?;
    println!(
        "Searching TikTok videos for: {}",
        format_lookup_for_log(&input)
    );

    let response = fetch_scrappa_response(client, config, &params).await?;
    validate_scrappa_code(&response)?;
    let data = response.get("data");
    let videos = extract_videos(data);
    let (has_next_page, next_cursor) = extract_pagination(data);
    let mut dataset_budget = DatasetBudget::default();
    let videos_saved = if videos.is_empty() {
        println!("No TikTok videos found for this search");
        0
    } else {
        let rows = videos
            .iter()
            .map(|video| enrich_video(video, &params))
            .collect::<Vec<_>>();
        let saved = push_dataset_items(client, config, &mut dataset_budget, &rows).await?;
        println!(
            "Found {} TikTok search results; saved {saved}",
            videos.len()
        );
        saved
    };

    put_output(client, config, &response).await?;
    let processed_time = response
        .get("processed_time")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);
    println!("TikTok search scraping completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "videos_extracted": videos.len(),
            "videos_saved": videos_saved,
            "has_next_page": has_next_page,
            "next_cursor": next_cursor,
            "processed_time": processed_time,
        })
    );
    Ok(())
}

fn failure_message(error: &anyhow::Error) -> String {
    let raw_message = format!("{error:#}");
    if raw_message.contains("timed out") {
        format!("{raw_message}. The TikTok search request exceeded the {}s Scrappa API timeout. Try a more specific keyword or run the request again.", SCRAPPA_REQUEST_TIMEOUT.as_secs())
    } else {
        raw_message
    }
}

#[tokio::main]
async fn main() {
    let result = async {
        let config = ActorConfig::from_env()?;
        run_actor(&reqwest::Client::new(), &config).await
    }
    .await;
    if let Err(error) = result {
        eprintln!("Actor failed: {}", failure_message(&error));
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    fn value(source: &str) -> Value {
        serde_json::from_str(source).unwrap()
    }

    fn request_config(address: std::net::SocketAddr) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: Url::parse(&format!("http://{address}")).unwrap(),
            scrappa_api_base_url: Url::parse(&format!("http://{address}/api/tiktok/feed/search"))
                .unwrap(),
            default_key_value_store_id: "store-test".to_owned(),
            default_dataset_id: "dataset-test".to_owned(),
            actor_run_id: "run-test".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-apify-token".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    #[test]
    fn builds_search_params_and_uses_query_alias() {
        let params = build_search_params(&value(
            r#"{"keywords":" basketball   highlights ","query":"ignored","region":" us ","count":10,"cursor":0,"publish_time":7,"sort_type":1}"#,
        ))
        .unwrap();
        assert_eq!(
            params,
            SearchParams {
                keywords: "basketball highlights".to_owned(),
                region: Some("US".to_owned()),
                count: Some(10),
                cursor: Some("0".to_owned()),
                publish_time: Some(7),
                sort_type: Some(1),
            }
        );
        assert_eq!(
            build_search_params(&value(r##"{"query":"#skincare"}"##))
                .unwrap()
                .keywords,
            "#skincare"
        );
    }

    #[test]
    fn rejects_missing_or_malformed_search_input() {
        assert!(build_search_params(&value("{}"))
            .unwrap_err()
            .to_string()
            .contains("keywords are required"));
        assert!(
            build_search_params(&value(r#"{"keywords":"basketball\nhighlights"}"#))
                .unwrap_err()
                .to_string()
                .contains("control whitespace")
        );
        assert!(
            build_search_params(&value(r#"{"keywords":"basketball","region":"USA-1"}"#))
                .unwrap_err()
                .to_string()
                .contains("region must be")
        );
    }

    #[test]
    fn omits_invalid_optional_params_and_keeps_metadata_nulls() {
        let params = build_search_params(&value(
            r#"{"keywords":"basketball","count":0,"publish_time":-1,"sort_type":11,"region":12}"#,
        ))
        .unwrap();
        assert_eq!(params.count, None);
        assert_eq!(params.publish_time, None);
        assert_eq!(params.sort_type, None);
        let row = enrich_video(&value(r#"{"aweme_id":"1"}"#), &params);
        assert_eq!(row["request_keywords"], "basketball");
        for key in [
            "request_region",
            "request_count",
            "request_cursor",
            "request_publish_time",
            "request_sort_type",
        ] {
            assert!(
                row[key].is_null(),
                "{key} must be present with null when omitted"
            );
        }
    }

    #[test]
    fn extracts_video_variants_and_cursor_precedence() {
        let response = value(
            r#"{"data":{"videos":[{"aweme_id":"1"}],"posts":[{"aweme_id":"ignored"}],"hasMore":true,"has_more":false,"cursor":"10","max_cursor":"20"}}"#,
        );
        let videos = extract_videos(response.get("data"));
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0]["aweme_id"], "1");
        assert_eq!(
            extract_pagination(response.get("data")),
            (true, Value::String("10".to_owned()))
        );
        let array_data = value(r#"[{"aweme_id":"2"}]"#);
        let alternate_data = value(r#"{"aweme_list":[{"aweme_id":"3"}]}"#);
        assert_eq!(extract_videos(Some(&array_data)).len(), 1);
        assert_eq!(extract_videos(Some(&alternate_data)).len(), 1);
        assert_eq!(extract_pagination(Some(&Value::Null)), (false, Value::Null));
    }

    #[test]
    fn validates_scrappa_api_code() {
        assert!(validate_scrappa_code(&value(r#"{"code":0}"#)).is_ok());
        assert!(validate_scrappa_code(&value(r#"{"data":[]}"#)).is_ok());
        let error =
            validate_scrappa_code(&value(r#"{"code":1001,"msg":"Bad request"}"#)).unwrap_err();
        assert!(error.to_string().contains("code 1001: Bad request"));
    }

    #[test]
    fn input_schema_retains_api_alias_and_optional_search_controls() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let properties = schema["properties"].as_object().unwrap();
        assert_eq!(properties["keywords"]["type"], "string");
        assert_eq!(properties["query"]["type"], "string");
        assert_eq!(properties["count"]["minimum"], 1);
        assert_eq!(properties["count"]["maximum"], 50);
        for field in ["region", "cursor", "publish_time", "sort_type"] {
            assert!(
                properties.contains_key(field),
                "missing input schema field {field}"
            );
        }
    }

    #[derive(Debug)]
    struct CapturedRequest {
        method_and_path: String,
        headers: HashMap<String, String>,
        body: String,
    }

    fn read_request(stream: &mut TcpStream) -> CapturedRequest {
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 4096];
        let (header_end, content_length) = loop {
            let read = stream.read(&mut chunk).unwrap();
            assert_ne!(read, 0, "client closed before sending a complete request");
            bytes.extend_from_slice(&chunk[..read]);
            if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..index]);
                let content_length = headers
                    .lines()
                    .filter_map(|line| line.split_once(':'))
                    .find(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                    .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                    .unwrap_or(0);
                if bytes.len() >= index + 4 + content_length {
                    break (index, content_length);
                }
            }
        };
        let header_text = String::from_utf8_lossy(&bytes[..header_end]).to_string();
        let mut lines = header_text.lines();
        let method_and_path = lines.next().unwrap().to_owned();
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(key, value)| (key.trim().to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        let body = String::from_utf8_lossy(&bytes[header_end + 4..header_end + 4 + content_length])
            .to_string();
        CapturedRequest {
            method_and_path,
            headers,
            body,
        }
    }

    fn mock_response(stream: &mut TcpStream, status: &str, body: &str) {
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    }

    fn start_mock_server(
        responses: Vec<(String, String)>,
    ) -> (
        std::net::SocketAddr,
        thread::JoinHandle<Vec<CapturedRequest>>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                requests.push(read_request(&mut stream));
                mock_response(&mut stream, &status, &body);
            }
            requests
        });
        (address, server)
    }

    fn pricing_body(max_charge: f64, counts: Value) -> String {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                        "other-event": {"eventPriceUsd": 0.0001}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": counts
            }
        })
        .to_string()
    }

    fn actor_responses(
        run_status: &str,
        run_body: &str,
        dataset_status: Option<&str>,
        include_output: bool,
    ) -> Vec<(String, String)> {
        let mut responses = vec![
            (
                "200 OK".to_owned(),
                r#"{"keywords":"basketball"}"#.to_owned(),
            ),
            (
                "200 OK".to_owned(),
                r#"{"code":0,"data":{"videos":[{"aweme_id":"1"},{"aweme_id":"2"}],"hasMore":true,"cursor":"next"},"processed_time":99}"#.to_owned(),
            ),
            (run_status.to_owned(), run_body.to_owned()),
        ];
        if let Some(status) = dataset_status {
            responses.push((status.to_owned(), "{}".to_owned()));
        }
        if include_output {
            responses.push(("201 Created".to_owned(), String::new()));
        }
        responses
    }

    #[tokio::test]
    async fn actor_uses_cloud_storage_and_preserves_output_and_dataset_rows() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let responses = [
                (
                    "200 OK",
                    r#"{"keywords":"basketball","region":"us","count":1,"cursor":"0","publish_time":7,"sort_type":1}"#,
                ),
                (
                    "200 OK",
                    r#"{"code":0,"data":{"videos":[{"aweme_id":"1","desc":"clip","request_keywords":"old"}],"hasMore":true,"cursor":"100"},"processed_time":99,"raw":"retained"}"#,
                ),
                (
                    "200 OK",
                    r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0003}}}},"options":{"maxTotalChargeUsd":1},"chargedEventCounts":{"apify-default-dataset-item":0}}}"#,
                ),
                ("201 Created", ""),
                ("201 Created", ""),
            ];
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                requests.push(read_request(&mut stream));
                mock_response(&mut stream, status, body);
            }
            requests
        });
        let config = request_config(address);
        run_actor(&reqwest::Client::new(), &config).await.unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 5);
        assert!(requests[0]
            .method_and_path
            .starts_with("GET /v2/key-value-stores/store-test/records/INPUT "));
        assert!(requests[2]
            .method_and_path
            .starts_with("GET /v2/actor-runs/run-test "));
        assert!(requests[3]
            .method_and_path
            .starts_with("POST /v2/datasets/dataset-test/items "));
        assert!(requests[4]
            .method_and_path
            .starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT "));
        for index in [0, 2, 3, 4] {
            assert_eq!(
                requests[index]
                    .headers
                    .get("authorization")
                    .map(String::as_str),
                Some("Bearer test-apify-token")
            );
            assert!(!requests[index].headers.contains_key("x-api-key"));
        }
        assert_eq!(
            requests[1].headers.get("x-api-key").map(String::as_str),
            Some("test-scrappa-key")
        );
        let query = Url::parse(&format!(
            "http://mock{}",
            requests[1]
                .method_and_path
                .split_whitespace()
                .nth(1)
                .unwrap()
        ))
        .unwrap();
        let query = query.query_pairs().into_owned().collect::<HashMap<_, _>>();
        assert_eq!(query["keywords"], "basketball");
        assert_eq!(query["region"], "US");
        assert_eq!(query["count"], "1");
        assert_eq!(query["cursor"], "0");
        assert_eq!(query["publish_time"], "7");
        assert_eq!(query["sort_type"], "1");

        let dataset: Value = serde_json::from_str(&requests[3].body).unwrap();
        assert_eq!(dataset.as_array().unwrap().len(), 1);
        assert_eq!(dataset[0]["aweme_id"], "1");
        assert_eq!(dataset[0]["request_keywords"], "basketball");
        assert_eq!(dataset[0]["request_region"], "US");
        assert_eq!(dataset[0]["request_count"], 1);
        assert_eq!(dataset[0]["request_cursor"], "0");
        assert_eq!(dataset[0]["request_publish_time"], 7);
        assert_eq!(dataset[0]["request_sort_type"], 1);
        let output: Value = serde_json::from_str(&requests[4].body).unwrap();
        assert_eq!(
            output,
            value(
                r#"{"code":0,"data":{"videos":[{"aweme_id":"1","desc":"clip","request_keywords":"old"}],"hasMore":true,"cursor":"100"},"processed_time":99,"raw":"retained"}"#
            )
        );
    }

    #[tokio::test]
    async fn one_result_budget_trims_two_videos_and_keeps_raw_output() {
        let run = pricing_body(
            0.0006,
            json!({"apify-default-dataset-item": 0, "other-event": 3}),
        );
        let (address, server) =
            start_mock_server(actor_responses("200 OK", &run, Some("201 Created"), true));
        run_actor(&reqwest::Client::new(), &request_config(address))
            .await
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 5);
        assert!(requests[2]
            .method_and_path
            .starts_with("GET /v2/actor-runs/run-test "));
        let rows: Value = serde_json::from_str(&requests[3].body).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 1);
        assert_eq!(rows[0]["aweme_id"], "1");
        let output: Value = serde_json::from_str(&requests[4].body).unwrap();
        assert_eq!(output["data"]["videos"].as_array().unwrap().len(), 2);
        assert_eq!(output["data"]["videos"][0]["aweme_id"], "1");
    }

    #[tokio::test]
    async fn zero_budget_skips_dataset_post_but_still_saves_output() {
        let run = pricing_body(0.0, json!({}));
        let (address, server) = start_mock_server(actor_responses("200 OK", &run, None, true));
        run_actor(&reqwest::Client::new(), &request_config(address))
            .await
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 4);
        assert!(requests
            .iter()
            .all(|request| !request.method_and_path.contains("/v2/datasets/")));
        assert!(requests[3]
            .method_and_path
            .starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT "));
    }

    #[tokio::test]
    async fn generous_numeric_budget_saves_all_video_rows_in_order() {
        let run = pricing_body(1.0, json!({}));
        let (address, server) =
            start_mock_server(actor_responses("200 OK", &run, Some("201 Created"), true));
        run_actor(&reqwest::Client::new(), &request_config(address))
            .await
            .unwrap();
        let requests = server.join().unwrap();

        let rows: Value = serde_json::from_str(&requests[3].body).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 2);
        assert_eq!(rows[0]["aweme_id"], "1");
        assert_eq!(rows[1]["aweme_id"], "2");
    }

    #[tokio::test]
    async fn invalid_spending_limit_fails_before_any_dataset_or_output_write() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
                    }}
                },
                "options": {"maxTotalChargeUsd": null},
                "chargedEventCounts": {}
            }
        })
        .to_string();
        let (address, server) = start_mock_server(actor_responses("200 OK", &run, None, false));
        let error = run_actor(&reqwest::Client::new(), &request_config(address))
            .await
            .unwrap_err();
        let requests = server.join().unwrap();

        assert!(error.to_string().contains("spending limit"));
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request.method_and_path.contains("/v2/datasets/")
                && !request.method_and_path.contains("/records/OUTPUT")));
    }

    #[tokio::test]
    async fn dataset_storage_error_fails_without_writing_output() {
        let run = pricing_body(1.0, json!({}));
        let (address, server) = start_mock_server(actor_responses(
            "200 OK",
            &run,
            Some("500 Internal Server Error"),
            false,
        ));
        let error = run_actor(&reqwest::Client::new(), &request_config(address))
            .await
            .unwrap_err();
        let requests = server.join().unwrap();

        assert!(error.to_string().contains("Apify dataset write failed"));
        assert_eq!(requests.len(), 4);
        assert!(requests
            .iter()
            .all(|request| !request.method_and_path.contains("/records/OUTPUT")));
    }

    #[tokio::test]
    async fn local_saved_rows_prevent_a_lagging_run_snapshot_from_overspending() {
        let run = pricing_body(0.0003, json!({}));
        let (address, server) = start_mock_server(vec![
            ("200 OK".to_owned(), run),
            ("201 Created".to_owned(), String::new()),
        ]);
        let config = request_config(address);
        let client = reqwest::Client::new();
        let rows = [json!({"aweme_id":"1"}), json!({"aweme_id":"2"})];
        let mut budget = DatasetBudget::default();

        assert_eq!(
            push_dataset_items(&client, &config, &mut budget, &rows)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            push_dataset_items(&client, &config, &mut budget, &rows[1..])
                .await
                .unwrap(),
            0
        );
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0]
            .method_and_path
            .starts_with("GET /v2/actor-runs/run-test "));
        let saved: Value = serde_json::from_str(&requests[1].body).unwrap();
        assert_eq!(saved.as_array().unwrap().len(), 1);
        assert_eq!(saved[0]["aweme_id"], "1");
    }
    #[tokio::test]
    async fn missing_apify_input_record_404_is_treated_as_no_input() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _request = read_request(&mut stream);
            mock_response(&mut stream, "404 Not Found", "{}");
        });
        let config = request_config(address);
        assert!(get_input(&reqwest::Client::new(), &config)
            .await
            .unwrap()
            .is_none());
        server.join().unwrap();
    }
}
