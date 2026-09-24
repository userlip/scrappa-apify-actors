use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use std::{env, process, time::Duration};
use tokio::time::sleep;
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: u32 = 8;
const APIFY_RETRY_DELAY: Duration = Duration::from_millis(500);
const MAX_DATASET_PAYLOAD_BYTES: usize = 9_437_184 - 944;
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_COMMENTS_PER_PAGE: i64 = 50;
const MAX_REPLIES_PER_PAGE: usize = 50;
const DEFAULT_MAX_REPLIES_PER_COMMENT: usize = 50;

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

fn js_typeof(value: &Value) -> &'static str {
    match value {
        Value::Null | Value::Array(_) | Value::Object(_) => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
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

fn valid_integer(value: &Value, min: i64, max: i64) -> Option<i64> {
    let number = value.as_f64()?;
    if number.is_finite() && number.fract() == 0.0 && number >= min as f64 && number <= max as f64 {
        Some(number as i64)
    } else {
        None
    }
}

fn tiktok_video_id(path: &str) -> Option<&str> {
    let path = path.strip_prefix("/@")?;
    let (username, video_path) = path.split_once('/')?;
    if username.is_empty() {
        return None;
    }
    let video_id = video_path
        .get(..6)
        .filter(|prefix| prefix.eq_ignore_ascii_case("video/"))
        .map(|_| &video_path[6..])?;
    let video_id = video_id.strip_suffix('/').unwrap_or(video_id);
    if video_id.is_empty() || !video_id.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(video_id)
}

fn require_tiktok_video_url(raw_url: &str) -> Result<Url> {
    let parsed = Url::parse(js_trim(raw_url))
        .map_err(|_| anyhow!("A valid TikTok video URL is required"))?;
    let is_tiktok_host = parsed.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("tiktok.com")
            || host.to_ascii_lowercase().ends_with(".tiktok.com")
    });
    if !is_tiktok_host {
        bail!("A TikTok video URL is required");
    }
    if parsed.scheme() != "https" {
        bail!("A TikTok video URL must use HTTPS");
    }
    if tiktok_video_id(parsed.path()).is_none() {
        bail!("A TikTok video URL must use the format https://www.tiktok.com/@username/video/1234567890");
    }
    Ok(parsed)
}

fn format_tiktok_video_url_for_log(url: &Url) -> String {
    let mut url = url.clone();
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

#[derive(Debug, PartialEq)]
struct PaginationParams {
    count: Option<i64>,
    cursor: Option<String>,
}

fn normalize_cursor(value: Option<&Value>, label: &str) -> Option<String> {
    match value {
        Some(Value::String(value)) => {
            let cursor = js_trim(value);
            (!cursor.is_empty()).then(|| cursor.to_owned())
        }
        Some(Value::Null) | None => None,
        Some(value) => {
            eprintln!(
                "{label} must be a string, got {}. Starting from the first page.",
                js_typeof(value)
            );
            None
        }
    }
}

fn comments_pagination(input: &Value) -> PaginationParams {
    let count = input.get("count").and_then(|value| {
        if let Some(count) = valid_integer(value, 1, MAX_COMMENTS_PER_PAGE) {
            Some(count)
        } else {
            eprintln!(
                "count must be an integer between 1 and 50, got {}. Using Scrappa default.",
                js_string(value)
            );
            None
        }
    });
    let cursor = normalize_cursor(input.get("cursor"), "cursor");
    PaginationParams { count, cursor }
}

fn replies_cursor(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(value)) if value.is_empty() => None,
        _ => normalize_cursor(value, "reply cursor"),
    }
}

fn reply_count_as_number(value: &Value) -> f64 {
    match value {
        Value::Null => 0.0,
        Value::Bool(value) => f64::from(u8::from(*value)),
        Value::Number(value) => value.as_f64().unwrap_or(f64::NAN),
        Value::String(value) => {
            let value = js_trim(value);
            if value.is_empty() {
                0.0
            } else {
                value.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(_) => {
            let string_value = js_string(value);
            let value = js_trim(&string_value);
            if value.is_empty() {
                0.0
            } else {
                value.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Object(_) => f64::NAN,
    }
}

struct TikTokCommentsInput {
    video_url: String,
    count: Option<i64>,
    cursor: Option<String>,
    include_replies: bool,
    max_replies_per_comment: usize,
}

impl TikTokCommentsInput {
    fn parse(input: Option<Value>) -> Result<Self> {
        let input = input.unwrap_or(Value::Null);
        let Some(url) = input.get("url").filter(|value| js_truthy(value)) else {
            bail!("TikTok video URL is required");
        };
        let video_url = url
            .as_str()
            .ok_or_else(|| anyhow!("A valid TikTok video URL is required"))?
            .to_owned();
        require_tiktok_video_url(&video_url)?;

        let max_replies_per_comment = match input.get("maxRepliesPerComment") {
            None => DEFAULT_MAX_REPLIES_PER_COMMENT,
            Some(value) => valid_integer(value, 1, 500)
                .map(|value| value as usize)
                .unwrap_or_else(|| {
                    eprintln!(
                        "maxRepliesPerComment must be an integer between 1 and 500, got {}. Using 50.",
                        js_string(value)
                    );
                    DEFAULT_MAX_REPLIES_PER_COMMENT
                }),
        };

        let pagination = comments_pagination(&input);
        Ok(Self {
            video_url,
            count: pagination.count,
            cursor: pagination.cursor,
            include_replies: input.get("includeReplies") == Some(&Value::Bool(true)),
            max_replies_per_comment,
        })
    }
}

fn comments_url(base_url: &Url, input: &TikTokCommentsInput) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["tiktok", "comments", "list"])?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("url", &input.video_url);
        if let Some(count) = input.count {
            query.append_pair("count", &count.to_string());
        }
        if let Some(cursor) = &input.cursor {
            query.append_pair("cursor", cursor);
        }
    }
    Ok(url)
}

fn replies_url(
    base_url: &Url,
    comment_id: &Value,
    video_id: &str,
    count: usize,
    cursor: Option<&Value>,
) -> Result<Url> {
    let comment_id = comment_id
        .as_str()
        .ok_or_else(|| anyhow!("comment_id.trim is not a function"))?;
    let comment_id = js_trim(comment_id);
    if comment_id.is_empty() {
        bail!("comment_id is required to fetch TikTok comment replies");
    }

    let video_id = js_trim(video_id);
    let mut url = endpoint_url(base_url, &["tiktok", "comments", "replies"])?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("comment_id", comment_id);
        if !video_id.is_empty() {
            query.append_pair("video_id", video_id);
        }
        query.append_pair("count", &count.to_string());
        if let Some(cursor) = replies_cursor(cursor) {
            query.append_pair("cursor", &cursor);
        }
    }
    Ok(url)
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

async fn send_apify_request(
    build_request: impl Fn() -> reqwest::RequestBuilder,
    operation: &str,
) -> Result<Response> {
    for attempt in 0..=APIFY_MAX_RETRIES {
        match build_request().timeout(APIFY_REQUEST_TIMEOUT).send().await {
            Ok(response) => {
                let status = response.status();
                if (status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
                    && attempt < APIFY_MAX_RETRIES
                {
                    let delay = retry_delay(attempt);
                    eprintln!(
                        "Apify {operation} returned {}. Retrying after {}ms.",
                        status.as_u16(),
                        delay.as_millis()
                    );
                    sleep(delay).await;
                    continue;
                }
                return Ok(response);
            }
            Err(error) if attempt < APIFY_MAX_RETRIES => {
                let delay = retry_delay(attempt);
                eprintln!(
                    "Apify {operation} request failed: {error}. Retrying after {}ms.",
                    delay.as_millis()
                );
                sleep(delay).await;
            }
            Err(error) => {
                return Err(error).with_context(|| format!("Apify {operation} request failed"));
            }
        }
    }
    unreachable!("the retry loop always returns or fails")
}

fn retry_delay(attempt: u32) -> Duration {
    APIFY_RETRY_DELAY * (1_u32 << attempt)
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
        "INPUT",
    )
    .await?;
    if response.status() == StatusCode::NOT_FOUND {
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
                            .map(|message| {
                                if message.is_null() {
                                    String::new()
                                } else {
                                    js_string(message)
                                }
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

async fn fetch_scrappa_response(client: &Client, url: Url, api_key: &str) -> Result<Value> {
    println!("[Scrappa] GET {url}");
    let response = client
        .get(url)
        .header("X-API-Key", api_key)
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
    response
        .json()
        .await
        .map_err(scrappa_request_error)
        .context("Scrappa API response was not valid JSON")
}

fn assert_successful_response(response: &Value, label: &str) -> Result<()> {
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
        "Scrappa {label} API returned code {}: {message}",
        js_string(code)
    )
}

fn extract_comments(response: &Value) -> Result<Vec<Value>> {
    let Some(comments) = response
        .get("data")
        .filter(|data| !data.is_null())
        .and_then(|data| data.get("comments"))
        .filter(|comments| !comments.is_null())
    else {
        return Ok(Vec::new());
    };
    comments
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow!("comments.map is not a function"))
}

fn extract_replies(response: &Value) -> Result<Vec<Value>> {
    let data = response.get("data").filter(|data| !data.is_null());
    let replies = data
        .and_then(|data| data.get("replies"))
        .filter(|replies| !replies.is_null())
        .or_else(|| {
            data.and_then(|data| data.get("comments"))
                .filter(|comments| !comments.is_null())
        });
    let Some(replies) = replies else {
        return Ok(Vec::new());
    };
    replies
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow!("replies.map is not a function"))
}

fn comment_id(comment: &Value) -> Option<Value> {
    comment
        .get("comment_id")
        .filter(|value| !value.is_null())
        .or_else(|| comment.get("id").filter(|value| !value.is_null()))
        .cloned()
}

fn comment_reply_count(comment: &Value) -> Value {
    comment
        .get("reply_count")
        .filter(|value| !value.is_null())
        .or_else(|| comment.get("reply_total").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::from(0))
}

fn js_spread(value: &Value) -> Map<String, Value> {
    match value {
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
    }
}

fn to_comment_dataset_item(comment: &Value, input: &TikTokCommentsInput, video_id: &str) -> Value {
    let mut item = js_spread(comment);
    if let Some(comment_id) = comment_id(comment) {
        item.insert("comment_id".to_owned(), comment_id);
    } else {
        item.remove("comment_id");
    }
    item.insert(
        "comment_type".to_owned(),
        Value::String("comment".to_owned()),
    );
    item.insert(
        "video_url".to_owned(),
        Value::String(input.video_url.clone()),
    );
    item.insert("video_id".to_owned(), Value::String(video_id.to_owned()));
    item.insert("parent_comment_id".to_owned(), Value::Null);
    item.insert("parent_comment_text".to_owned(), Value::Null);
    Value::Object(item)
}

fn to_reply_dataset_item(
    reply: &Value,
    parent_comment: &Value,
    input: &TikTokCommentsInput,
    video_id: &str,
) -> Value {
    let mut item = js_spread(reply);
    if let Some(reply_id) = comment_id(reply) {
        item.insert("comment_id".to_owned(), reply_id);
    } else {
        item.remove("comment_id");
    }
    item.insert("comment_type".to_owned(), Value::String("reply".to_owned()));
    item.insert(
        "video_url".to_owned(),
        Value::String(input.video_url.clone()),
    );
    item.insert("video_id".to_owned(), Value::String(video_id.to_owned()));
    item.insert(
        "parent_comment_id".to_owned(),
        comment_id(parent_comment).unwrap_or(Value::Null),
    );
    item.insert(
        "parent_comment_text".to_owned(),
        parent_comment
            .get("text")
            .filter(|text| !text.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    );
    Value::Object(item)
}

fn reply_count_is_less_than_one(comment: &Value) -> bool {
    reply_count_as_number(&comment_reply_count(comment)) < 1.0
}

async fn fetch_replies_for_comment(
    client: &Client,
    config: &ActorConfig,
    comment: &Value,
    input: &TikTokCommentsInput,
    video_id: &str,
) -> Result<(Vec<Value>, Vec<Value>)> {
    let Some(raw_comment_id) = comment_id(comment) else {
        return Ok((Vec::new(), Vec::new()));
    };
    if !js_truthy(&raw_comment_id) || reply_count_is_less_than_one(comment) {
        return Ok((Vec::new(), Vec::new()));
    }

    let comment_id_for_request = raw_comment_id
        .as_str()
        .ok_or_else(|| anyhow!("comment_id.trim is not a function"))?;
    let mut rows = Vec::new();
    let mut responses = Vec::new();
    let mut cursor: Option<Value> = None;

    while rows.len() < input.max_replies_per_comment {
        let count = MAX_REPLIES_PER_PAGE.min(input.max_replies_per_comment - rows.len());
        let url = replies_url(
            &config.scrappa_api_base_url,
            &Value::String(comment_id_for_request.to_owned()),
            video_id,
            count,
            cursor.as_ref(),
        )?;
        let response = fetch_scrappa_response(client, url, &config.scrappa_api_key).await?;
        assert_successful_response(&response, "TikTok Comment Replies")?;
        responses.push(json!({
            "parent_comment_id": raw_comment_id,
            "response": response,
        }));

        let replies = extract_replies(&response)?;
        rows.extend(
            replies
                .iter()
                .map(|reply| to_reply_dataset_item(reply, comment, input, video_id)),
        );

        let data = response.get("data").filter(|data| !data.is_null());
        let has_more = data
            .and_then(|data| data.get("hasMore"))
            .is_some_and(js_truthy);
        let next_cursor = data
            .and_then(|data| data.get("cursor"))
            .filter(|cursor| !cursor.is_null());
        if !has_more || next_cursor.is_none_or(|cursor| !js_truthy(cursor)) || replies.is_empty() {
            break;
        }
        cursor = next_cursor.cloned();
    }

    Ok((rows, responses))
}

async fn get_dataset_capacity(
    apify_client: &Client,
    config: &ActorConfig,
    requested: usize,
) -> Result<usize> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = send_apify_request(
        || {
            apify_client
                .get(url.clone())
                .bearer_auth(&config.apify_token)
        },
        "run pricing",
    )
    .await?;
    let run = response_json(response, "Apify run pricing request").await?;
    affordable_dataset_items(&run, requested)
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
        spent += price * count as f64;
    }
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

fn dataset_chunk_ranges(
    rows: &[Value],
    payload_limit: usize,
) -> Result<Vec<std::ops::Range<usize>>> {
    let mut chunks = Vec::new();
    let mut chunk_start = 0;
    let mut chunk_bytes = 2;

    for (index, row) in rows.iter().enumerate() {
        if !row.is_object() {
            bail!("Data item at index {index} is not an object. You can push only objects into a dataset.");
        }
        let item_bytes = serde_json::to_vec(row)
            .context("Dataset item could not be serialized to JSON")?
            .len();
        if item_bytes > payload_limit {
            bail!(
                "Data item at index {index} is too large (size: {item_bytes} bytes, limit: {payload_limit} bytes)"
            );
        }

        let separator_bytes = usize::from(index > chunk_start);
        if chunk_bytes + separator_bytes + item_bytes > payload_limit && index > chunk_start {
            chunks.push(chunk_start..index);
            chunk_start = index;
            chunk_bytes = item_bytes + 2;
        } else {
            chunk_bytes += separator_bytes + item_bytes;
        }
    }

    if chunk_start < rows.len() {
        chunks.push(chunk_start..rows.len());
    }
    Ok(chunks)
}

async fn push_dataset_items(
    apify_client: &Client,
    config: &ActorConfig,
    rows: &[Value],
) -> Result<usize> {
    if rows.is_empty() {
        return Ok(0);
    }
    let limit = get_dataset_capacity(apify_client, config, rows.len()).await?;
    let rows = &rows[..rows.len().min(limit)];
    if rows.is_empty() {
        return Ok(0);
    }
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let chunks = dataset_chunk_ranges(rows, MAX_DATASET_PAYLOAD_BYTES)?;
    for chunk in chunks {
        let items = &rows[chunk];
        let response = send_apify_request(
            || {
                let request = apify_client
                    .post(url.clone())
                    .bearer_auth(&config.apify_token);
                if items.len() == 1 {
                    request.json(&items[0])
                } else {
                    request.json(items)
                }
            },
            "dataset write",
        )
        .await?;
        ensure_success(response, "Apify dataset write").await?;
    }
    Ok(rows.len())
}

async fn put_key_value_record(
    apify_client: &Client,
    config: &ActorConfig,
    record_key: &str,
    value: &Value,
) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            record_key,
        ],
    )?;
    let response = send_apify_request(
        || {
            apify_client
                .put(url.clone())
                .bearer_auth(&config.apify_token)
                .json(value)
        },
        &format!("{record_key} write"),
    )
    .await?;
    ensure_success(response, &format!("Apify {record_key} write")).await
}

async fn run_actor(
    apify_client: &Client,
    scrappa_client: &Client,
    config: &ActorConfig,
) -> Result<()> {
    let input = TikTokCommentsInput::parse(get_input(apify_client, config).await?)?;
    let validated_url = require_tiktok_video_url(&input.video_url)?;
    let video_id = tiktok_video_id(validated_url.path())
        .ok_or_else(|| anyhow!("A TikTok video URL must use the format https://www.tiktok.com/@username/video/1234567890"))?
        .to_owned();
    println!(
        "Fetching TikTok comments for: {}",
        format_tiktok_video_url_for_log(&validated_url)
    );

    let response = fetch_scrappa_response(
        scrappa_client,
        comments_url(&config.scrappa_api_base_url, &input)?,
        &config.scrappa_api_key,
    )
    .await?;
    assert_successful_response(&response, "TikTok Comments")?;

    let comments = extract_comments(&response)?;
    let mut rows = comments
        .iter()
        .map(|comment| to_comment_dataset_item(comment, &input, &video_id))
        .collect::<Vec<_>>();
    let mut reply_responses = Vec::new();

    if input.include_replies {
        println!(
            "Fetching up to {} replies for each top-level comment with replies",
            input.max_replies_per_comment
        );
        for comment in &comments {
            let (reply_rows, responses) =
                fetch_replies_for_comment(scrappa_client, config, comment, &input, &video_id)
                    .await?;
            rows.extend(reply_rows);
            reply_responses.extend(responses);
        }
    }

    let comments_extracted = comments.len();
    let replies_extracted = rows.len().saturating_sub(comments_extracted);
    let dataset_items = if rows.is_empty() {
        println!("No comments found for the given TikTok video URL");
        0
    } else {
        let dataset_items = push_dataset_items(apify_client, config, &rows).await?;
        println!(
            "Found {comments_extracted} comments and {replies_extracted} replies; saved {dataset_items} dataset items"
        );
        dataset_items
    };

    put_key_value_record(apify_client, config, "OUTPUT", &response).await?;
    if !reply_responses.is_empty() {
        put_key_value_record(
            apify_client,
            config,
            "REPLIES_OUTPUT",
            &Value::Array(reply_responses),
        )
        .await?;
    }

    let data = response.get("data").filter(|data| !data.is_null());
    let summary = json!({
        "comments_extracted": comments_extracted,
        "replies_extracted": replies_extracted,
        "dataset_items": dataset_items,
        "has_next_page": data.and_then(|data| data.get("hasMore")).filter(|value| !value.is_null()).cloned().unwrap_or(Value::Bool(false)),
        "next_cursor": data.and_then(|data| data.get("cursor")).filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "processed_time": response.get("processed_time").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
    });
    println!("TikTok comments extraction completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string(&summary).unwrap_or_else(|_| "{}".to_owned())
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify_client = Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .context("Could not create Apify HTTP client")?;
    let scrappa_client = Client::builder()
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .build()
        .context("Could not create Scrappa HTTP client")?;
    run_actor(&apify_client, &scrappa_client, &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{channel, Receiver},
            Arc,
        },
        thread::{self, JoinHandle},
        time::Instant,
    };

    const VIDEO_URL: &str = "https://www.tiktok.com/@tiktok/video/7568510388342443294";

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
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (recorded_requests, requests) = channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(15);
                let mut responses = responses.into_iter();
                while !stopped.load(Ordering::Relaxed) && Instant::now() < deadline {
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
                    let _ = recorded_requests.send(request);
                    let Some(response) = responses.next() else {
                        break;
                    };
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        422 => "Unprocessable Entity",
                        500 => "Internal Server Error",
                        _ => "OK",
                    };
                    let response = format!(
                        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    if stream.write_all(response.as_bytes()).is_err() {
                        break;
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

        fn take_requests(&self, expected: usize) -> Vec<String> {
            (0..expected)
                .map(|_| {
                    self.requests
                        .recv_timeout(Duration::from_secs(5))
                        .expect("mock server did not receive the expected request")
                })
                .collect()
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
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut bytes = Vec::new();
        let mut byte = [0_u8; 1];
        while !bytes.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte)?;
            bytes.push(byte[0]);
            if bytes.len() > 64 * 1024 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "request headers were too large",
                ));
            }
        }

        let headers = String::from_utf8_lossy(&bytes);
        let content_length = headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        let body_start = bytes.len();
        bytes.resize(body_start + content_length, 0);
        stream.read_exact(&mut bytes[body_start..])?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn config_for(server: &MockServer) -> ActorConfig {
        let mut scrappa_api_base_url = server.base_url.clone();
        scrappa_api_base_url.set_path("/api");
        ActorConfig {
            apify_api_base_url: server.base_url.clone(),
            scrappa_api_base_url,
            default_key_value_store_id: "store".to_owned(),
            default_dataset_id: "dataset".to_owned(),
            actor_run_id: "run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "apify-test-token".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
        }
    }

    fn pricing_run(max_total_charge: f64, charged_events: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-actor-start": { "eventPriceUsd": 0.02 },
                            "apify-default-dataset-item": { "eventPriceUsd": 0.01 }
                        }
                    }
                },
                "options": { "maxTotalChargeUsd": max_total_charge },
                "chargedEventCounts": charged_events
            }
        })
    }

    #[test]
    fn validates_tiktok_video_urls_and_extracts_the_video_id() {
        let parsed = require_tiktok_video_url(&format!("{VIDEO_URL}?lang=en#comments")).unwrap();
        assert_eq!(tiktok_video_id(parsed.path()), Some("7568510388342443294"));
        assert_eq!(format_tiktok_video_url_for_log(&parsed), VIDEO_URL);
        assert!(
            require_tiktok_video_url("https://m.tiktok.com/@tiktok/video/7568510388342443294")
                .is_ok()
        );
        assert!(
            require_tiktok_video_url("https://tiktok.com.attacker.example/@tiktok/video/1")
                .unwrap_err()
                .to_string()
                .contains("TikTok video URL is required")
        );
        assert!(
            require_tiktok_video_url("http://www.tiktok.com/@tiktok/video/1")
                .unwrap_err()
                .to_string()
                .contains("must use HTTPS")
        );
        assert!(require_tiktok_video_url("https://www.tiktok.com/@tiktok")
            .unwrap_err()
            .to_string()
            .contains("must use the format"));
    }

    #[test]
    fn normalizes_pagination_and_reply_options_like_the_actor_input_helpers() {
        let valid = TikTokCommentsInput::parse(Some(json!({
            "url": VIDEO_URL,
            "count": 50.0,
            "cursor": " 1700000000000 ",
            "includeReplies": true,
            "maxRepliesPerComment": 125
        })))
        .unwrap();
        assert_eq!(valid.count, Some(50));
        assert_eq!(valid.cursor.as_deref(), Some("1700000000000"));
        assert!(valid.include_replies);
        assert_eq!(valid.max_replies_per_comment, 125);

        let invalid = TikTokCommentsInput::parse(Some(json!({
            "url": VIDEO_URL,
            "count": "10",
            "cursor": 12345,
            "includeReplies": "true",
            "maxRepliesPerComment": 501
        })))
        .unwrap();
        assert_eq!(invalid.count, None);
        assert_eq!(invalid.cursor, None);
        assert!(!invalid.include_replies);
        assert_eq!(
            invalid.max_replies_per_comment,
            DEFAULT_MAX_REPLIES_PER_COMMENT
        );
    }

    #[test]
    fn builds_comments_and_reply_urls_with_pagination_values() {
        let input = TikTokCommentsInput::parse(Some(json!({
            "url": format!("{VIDEO_URL}?lang=en"),
            "count": 10,
            "cursor": " next page "
        })))
        .unwrap();
        let base = Url::parse("https://scrappa.co/api").unwrap();
        let comments = comments_url(&base, &input).unwrap();
        assert_eq!(
            comments.as_str(),
            "https://scrappa.co/api/tiktok/comments/list?url=https%3A%2F%2Fwww.tiktok.com%2F%40tiktok%2Fvideo%2F7568510388342443294%3Flang%3Den&count=10&cursor=next+page"
        );

        let replies = replies_url(
            &base,
            &json!(" comment-1 "),
            "7568510388342443294",
            25,
            Some(&json!(" reply-cursor ")),
        )
        .unwrap();
        assert_eq!(
            replies.as_str(),
            "https://scrappa.co/api/tiktok/comments/replies?comment_id=comment-1&video_id=7568510388342443294&count=25&cursor=reply-cursor"
        );
    }

    #[test]
    fn caps_default_dataset_items_against_all_prior_ppe_charges() {
        let run = pricing_run(
            0.05,
            json!({
                "apify-actor-start": 1,
                "apify-default-dataset-item": 0
            }),
        );
        assert_eq!(affordable_dataset_items(&run, 8).unwrap(), 3);
        assert_eq!(affordable_dataset_items(&run, 8).unwrap(), 3);
        assert_eq!(
            affordable_dataset_items(&pricing_run(0.02, json!({"apify-actor-start": 1})), 8)
                .unwrap(),
            0
        );
        assert_eq!(
            affordable_dataset_items(&pricing_run(1.0, json!({"apify-actor-start": 1})), 8)
                .unwrap(),
            8
        );
        assert!(affordable_dataset_items(&json!({"data": {}}), 1).is_err());
    }

    #[test]
    fn chunks_dataset_rows_under_the_apify_payload_limit_in_order() {
        let rows = vec![
            json!({"id": 1, "text": "first"}),
            json!({"id": 2, "text": "second"}),
            json!({"id": 3, "text": "third"}),
        ];
        let row_bytes = rows
            .iter()
            .map(|row| serde_json::to_vec(row).unwrap().len())
            .collect::<Vec<_>>();
        let two_row_limit = row_bytes[0] + row_bytes[1] + 3;

        assert_eq!(
            dataset_chunk_ranges(&rows, two_row_limit).unwrap(),
            vec![0..2, 2..3]
        );
        assert!(dataset_chunk_ranges(&[json!({"text": "too large"})], 4)
            .unwrap_err()
            .to_string()
            .contains("is too large"));
    }

    #[tokio::test]
    async fn collects_comments_replies_and_persists_default_dataset_and_key_value_outputs() {
        let server = MockServer::start(vec![
            MockResponse::json(
                200,
                json!({
                    "url": VIDEO_URL,
                    "count": 2,
                    "cursor": "first page",
                    "includeReplies": true,
                    "maxRepliesPerComment": 2
                }),
            ),
            MockResponse::json(
                200,
                json!({
                    "code": 0,
                    "processed_time": 0.4,
                    "data": {
                        "comments": [
                            {"comment_id": "parent-1", "text": "Parent", "reply_count": 3},
                            {"id": "parent-2", "text": "Second", "reply_total": 0}
                        ],
                        "hasMore": true,
                        "cursor": "comments-next"
                    }
                }),
            ),
            MockResponse::json(
                200,
                json!({
                    "code": 0,
                    "data": {
                        "replies": [{"id": "reply-1", "text": "First reply"}],
                        "hasMore": true,
                        "cursor": "reply-next"
                    }
                }),
            ),
            MockResponse::json(
                200,
                json!({
                    "code": 0,
                    "data": {
                        "comments": [{"comment_id": "reply-2", "text": "Second reply"}],
                        "hasMore": true,
                        "cursor": "reply-later"
                    }
                }),
            ),
            MockResponse::json(
                200,
                pricing_run(
                    0.05,
                    json!({
                        "apify-actor-start": 1,
                        "apify-default-dataset-item": 0
                    }),
                ),
            ),
            MockResponse::json(201, json!({})),
            MockResponse::json(201, json!({})),
            MockResponse::json(201, json!({})),
        ]);
        let config = config_for(&server);
        let apify_client = Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .unwrap();
        let scrappa_client = Client::builder()
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .build()
            .unwrap();

        run_actor(&apify_client, &scrappa_client, &config)
            .await
            .unwrap();

        let requests = server.take_requests(8);
        assert!(requests[0].starts_with("GET /v2/key-value-stores/store/records/INPUT "));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer apify-test-token"));
        assert!(requests[1].contains("/api/tiktok/comments/list?url="));
        assert!(requests[1].contains("&count=2&cursor=first+page"));
        assert!(requests[1]
            .to_ascii_lowercase()
            .contains("x-api-key: scrappa-test-key"));
        assert!(requests[2].contains(
            "/api/tiktok/comments/replies?comment_id=parent-1&video_id=7568510388342443294&count=2"
        ));
        assert!(requests[3].contains(
            "/api/tiktok/comments/replies?comment_id=parent-1&video_id=7568510388342443294&count=1&cursor=reply-next"
        ));
        assert!(requests[4].starts_with("GET /v2/actor-runs/run "));
        assert!(requests[4]
            .to_ascii_lowercase()
            .contains("authorization: bearer apify-test-token"));

        let dataset_request = &requests[5];
        assert!(dataset_request.starts_with("POST /v2/datasets/dataset/items "));
        let dataset: Value = serde_json::from_str(request_body(dataset_request)).unwrap();
        let rows = dataset.as_array().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["comment_type"], "comment");
        assert_eq!(rows[0]["parent_comment_id"], Value::Null);
        assert_eq!(rows[1]["comment_id"], "parent-2");
        assert_eq!(rows[2]["comment_type"], "reply");
        assert_eq!(rows[2]["comment_id"], "reply-1");
        assert_eq!(rows[2]["parent_comment_id"], "parent-1");
        assert_eq!(rows[2]["parent_comment_text"], "Parent");
        assert_eq!(
            rows[0]["video_url"],
            "https://www.tiktok.com/@tiktok/video/7568510388342443294"
        );

        assert!(requests[6].starts_with("PUT /v2/key-value-stores/store/records/OUTPUT "));
        let output: Value = serde_json::from_str(request_body(&requests[6])).unwrap();
        assert_eq!(output["data"]["hasMore"], true);
        assert_eq!(output["data"]["cursor"], "comments-next");

        assert!(requests[7].starts_with("PUT /v2/key-value-stores/store/records/REPLIES_OUTPUT "));
        let replies_output: Value = serde_json::from_str(request_body(&requests[7])).unwrap();
        assert_eq!(replies_output.as_array().unwrap().len(), 2);
        assert_eq!(replies_output[0]["parent_comment_id"], "parent-1");
    }

    #[tokio::test]
    async fn scrappa_http_errors_include_validation_details_without_retrying() {
        let server = MockServer::start(vec![
            MockResponse::json(200, json!({"url": VIDEO_URL})),
            MockResponse::json(
                422,
                json!({
                    "message": "Validation failed",
                    "errors": {"url": ["The URL field is required."]}
                }),
            ),
        ]);
        let config = config_for(&server);
        let apify_client = Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .unwrap();
        let scrappa_client = Client::builder()
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .build()
            .unwrap();

        let error = run_actor(&apify_client, &scrappa_client, &config)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains(
            "Scrappa API error (422): Validation failed - url: The URL field is required."
        ));
        assert_eq!(server.take_requests(2).len(), 2);
    }

    #[tokio::test]
    async fn keeps_raw_outputs_when_the_ppe_budget_cannot_cover_a_dataset_item() {
        let server = MockServer::start(vec![
            MockResponse::json(200, json!({"url": VIDEO_URL})),
            MockResponse::json(
                200,
                json!({
                    "code": 0,
                    "data": {
                        "comments": [{"comment_id": "comment-1", "text": "Still in OUTPUT"}],
                        "hasMore": false,
                        "cursor": null
                    }
                }),
            ),
            MockResponse::json(
                200,
                pricing_run(
                    0.02,
                    json!({
                        "apify-actor-start": 1,
                        "apify-default-dataset-item": 0
                    }),
                ),
            ),
            MockResponse::json(201, json!({})),
        ]);
        let config = config_for(&server);
        let apify_client = Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .unwrap();
        let scrappa_client = Client::builder()
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .build()
            .unwrap();

        run_actor(&apify_client, &scrappa_client, &config)
            .await
            .unwrap();

        let requests = server.take_requests(4);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));
        assert!(requests[3].starts_with("PUT /v2/key-value-stores/store/records/OUTPUT "));
        let output: Value = serde_json::from_str(request_body(&requests[3])).unwrap();
        assert_eq!(output["data"]["comments"][0]["comment_id"], "comment-1");
    }

    #[tokio::test]
    async fn retries_apify_rate_limit_and_server_errors() {
        let server = MockServer::start(vec![
            MockResponse::json(429, json!({"message": "slow down"})),
            MockResponse::json(503, json!({"message": "try again"})),
            MockResponse::json(200, json!({"ok": true})),
        ]);
        let client = Client::builder()
            .timeout(APIFY_REQUEST_TIMEOUT)
            .build()
            .unwrap();

        let response = send_apify_request(|| client.get(server.base_url.clone()), "test")
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(server.take_requests(3).len(), 3);
    }

    fn request_body(request: &str) -> &str {
        request
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .unwrap_or("")
    }
}
