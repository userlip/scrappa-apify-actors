use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_MAX_RETRIES: usize = 2;
const DEFAULT_INPUT_KEY: &str = "INPUT";
const VIDEO_LOOKUP_ERROR: &str =
    "A valid TikTok video URL, short URL, photo URL, or video ID is required";

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
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
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| DEFAULT_INPUT_KEY.to_owned()),
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

struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a ActorConfig,
}

impl ApifyClient<'_> {
    async fn get_input(&self) -> Result<Option<Value>> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &[
                "v2",
                "key-value-stores",
                &self.config.key_value_store_id,
                "records",
                &self.config.input_key,
            ],
        )?;
        let response =
            apify_get(self.http, url, &self.config.apify_token, "input retrieval").await?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let response = require_apify_success(response, "input retrieval").await?;
        response
            .json::<Value>()
            .await
            .context("Apify input record was not valid JSON")
            .map(Some)
    }

    async fn run_pricing(&self) -> Result<Value> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "actor-runs", &self.config.actor_run_id],
        )?;
        let response = apify_get(
            self.http,
            url,
            &self.config.apify_token,
            "run pricing request",
        )
        .await?;
        require_apify_success(response, "run pricing request")
            .await?
            .json::<Value>()
            .await
            .context("Apify run pricing response was not valid JSON")
    }

    async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "datasets", &self.config.dataset_id, "items"],
        )?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .timeout(APIFY_REQUEST_TIMEOUT)
            .json(item)
            .send()
            .await
            .context("Apify dataset write failed")?;
        require_apify_success(response, "dataset write").await?;
        Ok(())
    }
}

async fn apify_get(client: &Client, url: Url, token: &str, operation: &str) -> Result<Response> {
    for retry_count in 0..=APIFY_MAX_RETRIES {
        let response = client
            .get(url.clone())
            .bearer_auth(token)
            .header(header::ACCEPT, "application/json")
            .timeout(APIFY_REQUEST_TIMEOUT)
            .send()
            .await;

        match response {
            Ok(response)
                if retry_count < APIFY_MAX_RETRIES && retryable_apify_status(response.status()) =>
            {
                eprintln!(
                    "Apify {operation} returned {}; retrying",
                    response.status().as_u16()
                );
                drop(response);
            }
            Ok(response) => return Ok(response),
            Err(error) if retry_count < APIFY_MAX_RETRIES => {
                eprintln!("Apify {operation} failed ({error}); retrying");
            }
            Err(error) => return Err(error).with_context(|| format!("Apify {operation} failed")),
        }

        tokio::time::sleep(Duration::from_secs((retry_count + 1) as u64)).await;
    }

    unreachable!("the bounded Apify retry loop always returns its final response")
}

fn retryable_apify_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

struct DatasetBudget {
    run: Option<Value>,
    saved_rows: usize,
}

impl DatasetBudget {
    fn new() -> Self {
        Self {
            run: None,
            saved_rows: 0,
        }
    }

    async fn can_save_one(&mut self, apify: &ApifyClient<'_>) -> Result<bool> {
        if self.run.is_none() {
            self.run = Some(apify.run_pricing().await?);
        }
        let run = self
            .run
            .as_ref()
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        Ok(affordable_dataset_items(run, 1, self.saved_rows)? > 0)
    }

    fn record_saved_row(&mut self) {
        self.saved_rows += 1;
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
        None | Some(Value::Null) => return Ok(requested),
        Some(value) => value
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run did not provide a valid spending limit"))?,
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
        .get("apify-default-dataset-item")
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    if !item_price.is_finite() || item_price < 0.0 {
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct VideoRequest {
    url: String,
    validation_error: Option<String>,
}

fn resolve_video_requests(input: &Value) -> Result<Vec<VideoRequest>> {
    let mut requests = Vec::new();

    if let Some(urls) = input.get("urls") {
        if let Some(urls) = urls.as_array() {
            for (index, value) in urls.iter().enumerate() {
                let Some(value) = value.as_str() else {
                    eprintln!(
                        "Warning: urls[{index}] must be a string, got {}. Skipping.",
                        js_typeof(value)
                    );
                    continue;
                };
                let url = js_trim(value);
                if url.is_empty() {
                    eprintln!("Warning: urls[{index}] is empty. Skipping.");
                    continue;
                }
                match require_video_lookup(url) {
                    Ok(()) => requests.push(VideoRequest {
                        url: url.to_owned(),
                        validation_error: None,
                    }),
                    Err(message) => {
                        eprintln!("Warning: urls[{index}] is invalid: {message}");
                        requests.push(VideoRequest {
                            url: url.to_owned(),
                            validation_error: Some(message),
                        });
                    }
                }
            }
        } else if !urls.is_null() {
            eprintln!(
                "Warning: urls must be an array of strings, got {}. Falling back to url.",
                js_typeof(urls)
            );
        }
    }

    if requests.is_empty() {
        match input.get("url") {
            Some(Value::String(url)) => {
                let url = js_trim(url);
                if !url.is_empty() {
                    require_video_lookup(url).map_err(anyhow::Error::msg)?;
                    requests.push(VideoRequest {
                        url: url.to_owned(),
                        validation_error: None,
                    });
                }
            }
            Some(value)
                if !value.is_null()
                    && !matches!(value, Value::String(value) if value.is_empty()) =>
            {
                eprintln!("Warning: url must be a string, got {}.", js_typeof(value));
            }
            _ => {}
        }
    }

    if requests.is_empty() {
        bail!("At least one TikTok video URL is required");
    }
    Ok(requests)
}

fn require_video_lookup(value: &str) -> std::result::Result<(), String> {
    let lookup = js_trim(value);
    if lookup.is_empty() {
        return Err("TikTok video URL is required".to_owned());
    }
    if is_video_id(lookup) {
        return Ok(());
    }

    let parsed = Url::parse(lookup).map_err(|_| VIDEO_LOOKUP_ERROR.to_owned())?;
    let Some(host) = parsed.host_str() else {
        return Err(VIDEO_LOOKUP_ERROR.to_owned());
    };
    if !(host.eq_ignore_ascii_case("tiktok.com")
        || host.to_ascii_lowercase().ends_with(".tiktok.com"))
    {
        return Err("A TikTok URL is required".to_owned());
    }
    if parsed.scheme() != "https" {
        return Err("A TikTok URL must use HTTPS".to_owned());
    }

    let path = parsed.path();
    if matches_user_content_path(path) || matches_tiktok_short_path(path) {
        return Ok(());
    }
    if (host.eq_ignore_ascii_case("vm.tiktok.com") || host.eq_ignore_ascii_case("vt.tiktok.com"))
        && matches_redirect_path(path)
    {
        return Ok(());
    }
    Err("A TikTok video URL, short URL, photo URL, or video ID is required".to_owned())
}

fn is_video_id(value: &str) -> bool {
    (5..=30).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn matches_user_content_path(path: &str) -> bool {
    let Some(after_at) = path.strip_prefix("/@") else {
        return false;
    };
    let Some((username, content_path)) = after_at.split_once('/') else {
        return false;
    };
    if username.is_empty() {
        return false;
    }
    let content_path = content_path.strip_suffix('/').unwrap_or(content_path);
    let mut parts = content_path.split('/');
    let Some(kind) = parts.next() else {
        return false;
    };
    let Some(id) = parts.next() else {
        return false;
    };
    (kind.eq_ignore_ascii_case("video") || kind.eq_ignore_ascii_case("photo"))
        && !id.is_empty()
        && id.bytes().all(|byte| byte.is_ascii_digit())
        && parts.next().is_none()
}

fn matches_tiktok_short_path(path: &str) -> bool {
    let Some(token_path) = path.strip_prefix("/t/") else {
        return false;
    };
    let token = token_path.strip_suffix('/').unwrap_or(token_path);
    token.len() >= 6 && token.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn matches_redirect_path(path: &str) -> bool {
    let Some(token_path) = path.strip_prefix('/') else {
        return false;
    };
    let token = token_path.strip_suffix('/').unwrap_or(token_path);
    token.len() >= 8 && token.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn build_video_url(base_url: &Url, video_url: &str, hd: bool) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["tiktok", "video"])?;
    url.query_pairs_mut().append_pair("url", video_url);
    if hd {
        url.query_pairs_mut().append_pair("hd", "1");
    }
    Ok(url)
}

fn format_video_lookup_for_log(value: &str) -> Result<String> {
    let lookup = js_trim(value);
    if is_video_id(lookup) {
        return Ok(format!("video_id:{lookup}"));
    }
    let mut parsed = Url::parse(lookup)?;
    parsed.set_query(None);
    parsed.set_fragment(None);
    Ok(parsed.to_string())
}

fn js_typeof(value: &Value) -> &'static str {
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

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_trim(value: &str) -> &str {
    value.trim_matches(|character: char| {
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
    })
}

struct ScrappaClient<'a> {
    http: &'a Client,
    base_url: &'a Url,
    api_key: &'a str,
}

impl ScrappaClient<'_> {
    async fn get_video(&self, video_url: &str, hd: bool) -> Result<Value> {
        let url = build_video_url(self.base_url, video_url, hd)?;
        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key)
            .header(header::ACCEPT, "application/json")
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
                    anyhow!(error)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let message = scrappa_error_message(response, status).await?;
            bail!("Scrappa API error ({}): {message}", status.as_u16());
        }

        response
            .json::<Value>()
            .await
            .context("Scrappa API response was not valid JSON")
    }
}

async fn scrappa_error_message(response: Response, status: StatusCode) -> Result<String> {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let body = response
        .text()
        .await
        .context("Failed to read Scrappa API error response")?;
    if body.is_empty() {
        return Ok(fallback);
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(&body) {
        if let Some(error_data) = error_data.as_object() {
            let mut message = error_data
                .get("message")
                .filter(|value| !value.is_null())
                .map(js_string)
                .unwrap_or(fallback);
            if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
                let details = errors
                    .iter()
                    .map(|(field, messages)| {
                        let values = messages
                            .as_array()
                            .map(|messages| {
                                messages
                                    .iter()
                                    .map(js_string)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default();
                        format!("{field}: {values}")
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
    }

    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    Ok(compact.chars().take(500).collect())
}

fn assert_successful_response(response: &Value, url: &str) -> Result<()> {
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
        "Scrappa TikTok Video API returned code {} for {url}: {message}",
        js_string(code)
    );
}

fn extract_video(data: Option<&Value>, url: &str) -> Option<Value> {
    let data = data.filter(|data| js_truthy(data))?;
    if let Some(videos) = data.as_array() {
        if videos.is_empty() {
            eprintln!("Warning: Scrappa returned an empty video record array for {url}. Saving a not-found dataset item.");
            return None;
        }
        if videos.len() > 1 {
            eprintln!("Warning: Scrappa returned {} video records for {url}. Saving the first record to keep one dataset item per requested URL.", videos.len());
        }
        return videos.first().filter(|video| js_truthy(video)).cloned();
    }
    Some(data.clone())
}

fn dataset_item(
    video: Option<Value>,
    url: &str,
    hd: bool,
    response: Option<&Value>,
    request_index: usize,
    error_message: Option<String>,
) -> Value {
    let result_found = video.is_some() && error_message.is_none();
    let mut item = Map::new();
    if let Some(video) = video {
        if let Some(fields) = video.as_object() {
            item.extend(fields.clone());
        }
    }

    item.insert("request_url".to_owned(), Value::String(url.to_owned()));
    item.insert("request_hd".to_owned(), Value::Bool(hd));
    item.insert("request_index".to_owned(), json!(request_index));
    item.insert("result_found".to_owned(), Value::Bool(result_found));
    item.insert(
        "processed_time".to_owned(),
        if error_message.is_some() {
            Value::Null
        } else {
            response
                .and_then(|response| response.get("processed_time"))
                .filter(|value| !value.is_null())
                .cloned()
                .unwrap_or(Value::Null)
        },
    );
    if let Some(error_message) = error_message {
        item.insert("error_message".to_owned(), Value::String(error_message));
    }
    Value::Object(item)
}

async fn run_actor(http: &Client, config: &ActorConfig) -> Result<()> {
    let apify = ApifyClient { http, config };
    let input = apify
        .get_input()
        .await?
        .filter(js_truthy)
        .ok_or_else(|| anyhow!("At least one TikTok video URL is required"))?;
    let requests = resolve_video_requests(&input)?;
    let hd = input.get("hd") == Some(&Value::Bool(true));
    let scrappa = ScrappaClient {
        http,
        base_url: &config.scrappa_api_base_url,
        api_key: &config.scrappa_api_key,
    };
    let mut dataset_budget = DatasetBudget::new();
    let mut dataset_items = 0;
    let mut videos_found = 0;
    let mut lookups_failed = 0;

    println!(
        "Fetching TikTok video details for {} URL{}",
        requests.len(),
        if requests.len() == 1 { "" } else { "s" }
    );

    for (index, request) in requests.iter().enumerate() {
        if !dataset_budget.can_save_one(&apify).await? {
            eprintln!(
                "Pay-per-event spending limit reached; stopping before lookup {}/{}",
                index + 1,
                requests.len()
            );
            break;
        }

        let request_index = index + 1;
        let formatted_url = format_video_lookup_for_log(&request.url)
            .unwrap_or_else(|_| js_trim(&request.url).to_owned());
        let lookup_result = match &request.validation_error {
            Some(message) => Err(anyhow!(message.clone())),
            None => {
                println!(
                    "Fetching TikTok video {request_index}/{}: {formatted_url}",
                    requests.len()
                );
                scrappa
                    .get_video(&request.url, hd)
                    .await
                    .and_then(|response| {
                        assert_successful_response(&response, &request.url)?;
                        Ok(response)
                    })
            }
        };

        let item = match lookup_result {
            Ok(response) => {
                let video = extract_video(response.get("data"), &request.url);
                if video.is_some() {
                    videos_found += 1;
                    println!("Found 1 TikTok video record");
                } else {
                    println!("No video details found for: {formatted_url}");
                }
                dataset_item(
                    video,
                    &request.url,
                    hd,
                    Some(&response),
                    request_index,
                    None,
                )
            }
            Err(error) => {
                let message = format!("{error:#}");
                eprintln!("TikTok video lookup failed for {formatted_url}: {message}");
                lookups_failed += 1;
                dataset_item(None, &request.url, hd, None, request_index, Some(message))
            }
        };

        apify.push_dataset_item(&item).await?;
        dataset_budget.record_saved_row();
        dataset_items += 1;
    }

    let summary = json!({
        "urls_requested": requests.len(),
        "dataset_items": dataset_items,
        "videos_found": videos_found,
        "lookups_failed": lookups_failed,
        "hd_requested": hd,
    });
    println!("TikTok video details extraction completed successfully");
    println!("Results summary: {summary}");
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
    let http = Client::builder()
        .build()
        .context("Failed to create HTTP client")?;
    run_actor(&http, &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        io::{Read, Write},
        net::{SocketAddr, TcpListener, TcpStream},
        thread,
    };

    const VIDEO_URL: &str = "https://www.tiktok.com/@tiktok/video/7568510388342443294";
    const SECOND_VIDEO_URL: &str = "https://www.tiktok.com/@tiktok/video/1234567890123456789";

    #[derive(Debug)]
    struct CapturedRequest {
        method_and_path: String,
        headers: HashMap<String, String>,
        body: String,
    }

    fn request_config(address: SocketAddr) -> ActorConfig {
        let base_url = Url::parse(&format!("http://{address}")).unwrap();
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: Url::parse(&format!("http://{address}/api")).unwrap(),
            key_value_store_id: "store-test".to_owned(),
            dataset_id: "dataset-test".to_owned(),
            actor_run_id: "run-test".to_owned(),
            input_key: DEFAULT_INPUT_KEY.to_owned(),
            apify_token: "test-apify-token".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    fn pricing_run(max_charge: f64, counts: Value) -> String {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0002},
                        "other-event": {"eventPriceUsd": 0.0001}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": counts
            }
        })
        .to_string()
    }

    fn start_mock_server(
        responses: Vec<(String, String)>,
    ) -> (SocketAddr, thread::JoinHandle<Vec<CapturedRequest>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let server = thread::spawn(move || {
            let mut captured = Vec::new();
            for (status, body) in responses {
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                let (mut stream, _) = loop {
                    match listener.accept() {
                        Ok(connection) => break connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(
                                std::time::Instant::now() < deadline,
                                "timed out waiting for actor request"
                            );
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(error) => panic!("mock server failed to accept connection: {error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let request = read_request(&mut stream);
                write_response(&mut stream, &status, &body);
                captured.push(request);
            }
            captured
        });
        (address, server)
    }

    fn read_request(stream: &mut TcpStream) -> CapturedRequest {
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];
        let header_end = loop {
            let read = stream.read(&mut buffer).unwrap();
            assert_ne!(read, 0, "client closed before sending a complete request");
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                break index;
            }
        };

        let headers_text = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
        let mut lines = headers_text.split("\r\n");
        let method_and_path = lines.next().unwrap_or_default().to_owned();
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
            .collect::<HashMap<_, _>>();
        let content_length = headers
            .get("content-length")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let body_start = header_end + 4;
        let body_end = body_start + content_length;
        while bytes.len() < body_end {
            let read = stream.read(&mut buffer).unwrap();
            assert_ne!(
                read, 0,
                "client closed before sending the full request body"
            );
            bytes.extend_from_slice(&buffer[..read]);
        }

        CapturedRequest {
            method_and_path,
            headers,
            body: String::from_utf8(bytes[body_start..body_end].to_vec()).unwrap(),
        }
    }

    fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        stream.flush().unwrap();
    }

    #[test]
    fn accepts_video_ids_and_supported_tiktok_urls() {
        for value in [
            VIDEO_URL,
            "https://www.tiktok.com/@tiktok/video/7568510388342443294?lang=en",
            "https://www.tiktok.com/@tiktok/photo/7568510388342443294",
            "https://vm.tiktok.com/ZGeqDY4yL/",
            "https://vt.tiktok.com/ZGeqDY4yL/",
            "https://tiktok.com/t/ZGeqDY4yL/",
            "7568510388342443294",
        ] {
            assert!(require_video_lookup(value).is_ok(), "should accept {value}");
        }
    }

    #[test]
    fn rejects_wrong_host_protocol_and_non_content_paths() {
        assert_eq!(
            require_video_lookup("https://example.com/@tiktok/video/7568510388342443294"),
            Err("A TikTok URL is required".to_owned())
        );
        assert_eq!(
            require_video_lookup("http://www.tiktok.com/@tiktok/video/7568510388342443294"),
            Err("A TikTok URL must use HTTPS".to_owned())
        );
        for value in [
            "https://www.tiktok.com/@tiktok",
            "https://www.tiktok.com/tag/example",
            "https://www.tiktok.com/privacy",
            "https://www.tiktok.com/ZGeqDY4yL",
            "https://www.tiktok.com/@tiktok/video/not-a-number",
        ] {
            assert!(
                require_video_lookup(value).is_err(),
                "should reject {value}"
            );
        }
    }

    #[test]
    fn batch_input_keeps_invalid_strings_as_error_requests_and_skips_other_values() {
        let input = json!({"urls": [VIDEO_URL, 123, " ", "not-a-url"]});
        let requests = resolve_video_requests(&input).unwrap();
        assert_eq!(
            requests,
            vec![
                VideoRequest {
                    url: VIDEO_URL.to_owned(),
                    validation_error: None,
                },
                VideoRequest {
                    url: "not-a-url".to_owned(),
                    validation_error: Some(VIDEO_LOOKUP_ERROR.to_owned()),
                },
            ]
        );
    }

    #[test]
    fn batch_urls_take_precedence_and_keep_duplicates() {
        let input = json!({"urls": [VIDEO_URL, VIDEO_URL], "url": SECOND_VIDEO_URL});
        let requests = resolve_video_requests(&input).unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests.iter().all(|request| request.url == VIDEO_URL));
    }

    #[test]
    fn legacy_url_is_used_when_batch_input_has_no_valid_strings() {
        let input = json!({"urls": [null, 42], "url": SECOND_VIDEO_URL});
        let requests = resolve_video_requests(&input).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, SECOND_VIDEO_URL);
    }

    #[test]
    fn invalid_legacy_url_fails_before_lookup() {
        assert!(resolve_video_requests(&json!({"url": "not-a-url"})).is_err());
        assert!(resolve_video_requests(&Value::Null).is_err());
    }

    #[test]
    fn hd_query_is_only_sent_when_true_and_url_values_are_encoded() {
        let base = Url::parse("https://scrappa.co/api").unwrap();
        let normal = build_video_url(&base, VIDEO_URL, false).unwrap();
        let hd = build_video_url(&base, VIDEO_URL, true).unwrap();
        assert_eq!(normal.path(), "/api/tiktok/video");
        assert_eq!(normal.query_pairs().count(), 1);
        assert_eq!(normal.query_pairs().find(|(key, _)| key == "hd"), None);
        assert_eq!(
            hd.query_pairs().find(|(key, _)| key == "hd").unwrap().1,
            "1"
        );
        assert_eq!(
            hd.query_pairs().find(|(key, _)| key == "url").unwrap().1,
            VIDEO_URL
        );
    }

    #[test]
    fn log_format_removes_query_and_fragment_and_labels_ids() {
        assert_eq!(
            format_video_lookup_for_log(&format!("{VIDEO_URL}?token=secret#comments")).unwrap(),
            VIDEO_URL
        );
        assert_eq!(
            format_video_lookup_for_log("7568510388342443294").unwrap(),
            "video_id:7568510388342443294"
        );
    }

    #[test]
    fn extracts_first_video_and_treats_empty_data_as_not_found() {
        assert_eq!(
            extract_video(
                Some(&json!([{"aweme_id":"first"}, {"aweme_id":"second"}])),
                VIDEO_URL
            ),
            Some(json!({"aweme_id":"first"}))
        );
        assert_eq!(extract_video(Some(&json!([])), VIDEO_URL), None);
        assert_eq!(extract_video(Some(&Value::Null), VIDEO_URL), None);
    }

    #[test]
    fn dataset_rows_preserve_upstream_fields_and_override_request_metadata() {
        let response = json!({"processed_time": 1.25});
        let item = dataset_item(
            Some(json!({"aweme_id":"123", "request_url":"upstream"})),
            VIDEO_URL,
            true,
            Some(&response),
            2,
            None,
        );
        assert_eq!(item["aweme_id"], "123");
        assert_eq!(item["request_url"], VIDEO_URL);
        assert_eq!(item["request_hd"], true);
        assert_eq!(item["request_index"], 2);
        assert_eq!(item["result_found"], true);
        assert_eq!(item["processed_time"], 1.25);
    }

    #[test]
    fn missing_data_and_failed_lookups_keep_one_error_free_or_error_row_shape() {
        let not_found = dataset_item(
            None,
            VIDEO_URL,
            false,
            Some(&json!({"processed_time": 2})),
            1,
            None,
        );
        assert_eq!(not_found["result_found"], false);
        assert_eq!(not_found["processed_time"], 2);
        assert!(not_found.get("error_message").is_none());

        let failed = dataset_item(
            None,
            VIDEO_URL,
            false,
            None,
            1,
            Some("upstream failed".to_owned()),
        );
        assert_eq!(failed["result_found"], false);
        assert_eq!(failed["processed_time"], Value::Null);
        assert_eq!(failed["error_message"], "upstream failed");
    }

    #[test]
    fn ppe_budget_accounts_for_other_events_and_existing_dataset_rows() {
        let run: Value = serde_json::from_str(&pricing_run(
            0.0005,
            json!({"other-event": 1, "apify-default-dataset-item": 0}),
        ))
        .unwrap();
        assert_eq!(affordable_dataset_items(&run, 3, 0).unwrap(), 2);
        assert_eq!(affordable_dataset_items(&run, 3, 1).unwrap(), 1);
    }

    #[test]
    fn zero_priced_dataset_events_do_not_consume_budget() {
        let mut run: Value = serde_json::from_str(&pricing_run(1.0, json!({}))).unwrap();
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            ["apify-default-dataset-item"]["eventPriceUsd"] = json!(0.0);
        assert_eq!(affordable_dataset_items(&run, 5, 0).unwrap(), 5);
    }

    #[test]
    fn non_ppe_pricing_keeps_unbounded_dataset_capacity() {
        for pricing_model in ["FREE", "PRICE_PER_DATASET_ITEM", "PAY_PER_RESULT"] {
            let run = json!({"data": {"pricingInfo": {"pricingModel": pricing_model}}});
            assert_eq!(affordable_dataset_items(&run, 5, 9).unwrap(), 5);
        }
    }

    #[test]
    fn ppe_without_a_positive_max_charge_keeps_legacy_unbounded_capacity() {
        let mut missing_cap: Value =
            serde_json::from_str(&pricing_run(1.0, json!({"other-event": 1}))).unwrap();
        missing_cap["data"]["options"]
            .as_object_mut()
            .unwrap()
            .remove("maxTotalChargeUsd");
        let mut null_cap: Value =
            serde_json::from_str(&pricing_run(1.0, json!({"other-event": 1}))).unwrap();
        null_cap["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
        let zero_cap: Value =
            serde_json::from_str(&pricing_run(0.0, json!({"other-event": 1}))).unwrap();

        for run in [&missing_cap, &null_cap, &zero_cap] {
            assert_eq!(affordable_dataset_items(run, 5, 0).unwrap(), 5);
        }
    }

    #[test]
    fn incomplete_ppe_pricing_fails_closed() {
        let run: Value = json!({
            "data": {
                "pricingInfo": {"pricingModel": "PAY_PER_EVENT"},
                "options": {"maxTotalChargeUsd": 1.0}
            }
        });
        assert!(affordable_dataset_items(&run, 1, 0).is_err());
    }

    #[tokio::test]
    async fn apify_get_retries_a_temporary_server_error() {
        let (address, server) = start_mock_server(vec![
            ("503 Service Unavailable".to_owned(), "temporary".to_owned()),
            ("200 OK".to_owned(), "{}".to_owned()),
        ]);
        let url = Url::parse(&format!("http://{address}/v2/pricing")).unwrap();
        let response = apify_get(&Client::new(), url, "test-token", "pricing request")
            .await
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(requests.len(), 2);
        assert!(requests
            .iter()
            .all(|request| request.method_and_path.starts_with("GET /v2/pricing ")));
    }

    #[tokio::test]
    async fn actor_reads_input_from_kv_and_writes_one_dataset_row_per_lookup() {
        let run = pricing_run(0.0004, json!({}));
        let (address, server) = start_mock_server(vec![
            (
                "200 OK".to_owned(),
                json!({"urls": [VIDEO_URL, SECOND_VIDEO_URL], "hd": true}).to_string(),
            ),
            ("200 OK".to_owned(), run),
            (
                "200 OK".to_owned(),
                json!({"code": 0, "data": [{"aweme_id":"first", "title":"clip"}, {"aweme_id":"ignored"}], "processed_time": 1.25}).to_string(),
            ),
            ("201 Created".to_owned(), String::new()),
            (
                "200 OK".to_owned(),
                json!({"code": 503, "msg":"upstream unavailable"}).to_string(),
            ),
            ("201 Created".to_owned(), String::new()),
        ]);

        run_actor(&Client::new(), &request_config(address))
            .await
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 6);
        assert!(requests[0]
            .method_and_path
            .starts_with("GET /v2/key-value-stores/store-test/records/INPUT "));
        assert!(requests[1]
            .method_and_path
            .starts_with("GET /v2/actor-runs/run-test "));
        assert!(
            requests[2]
                .method_and_path
                .contains("/api/tiktok/video?url="),
            "{}",
            requests[2].method_and_path
        );
        assert!(requests[3]
            .method_and_path
            .starts_with("POST /v2/datasets/dataset-test/items "));
        assert!(requests[4]
            .method_and_path
            .contains("/api/tiktok/video?url="));
        assert!(requests[5]
            .method_and_path
            .starts_with("POST /v2/datasets/dataset-test/items "));
        for index in [0, 1, 3, 5] {
            assert_eq!(
                requests[index]
                    .headers
                    .get("authorization")
                    .map(String::as_str),
                Some("Bearer test-apify-token")
            );
        }
        for index in [2, 4] {
            assert_eq!(
                requests[index].headers.get("x-api-key").map(String::as_str),
                Some("test-scrappa-key")
            );
            assert!(requests[index].method_and_path.contains("&hd=1"));
        }

        let first_row: Value = serde_json::from_str(&requests[3].body).unwrap();
        assert_eq!(first_row["aweme_id"], "first");
        assert_eq!(first_row["title"], "clip");
        assert_eq!(first_row["request_url"], VIDEO_URL);
        assert_eq!(first_row["request_index"], 1);
        assert_eq!(first_row["request_hd"], true);
        assert_eq!(first_row["result_found"], true);
        assert_eq!(first_row["processed_time"], 1.25);

        let second_row: Value = serde_json::from_str(&requests[5].body).unwrap();
        assert_eq!(second_row["request_url"], SECOND_VIDEO_URL);
        assert_eq!(second_row["request_index"], 2);
        assert_eq!(second_row["result_found"], false);
        assert_eq!(
            second_row["error_message"],
            "Scrappa TikTok Video API returned code 503 for https://www.tiktok.com/@tiktok/video/1234567890123456789: upstream unavailable"
        );
    }

    #[tokio::test]
    async fn actor_writes_dataset_rows_under_free_pricing() {
        let free_run = json!({"data": {"pricingInfo": {"pricingModel": "FREE"}}}).to_string();
        let (address, server) = start_mock_server(vec![
            ("200 OK".to_owned(), json!({"url": VIDEO_URL}).to_string()),
            ("200 OK".to_owned(), free_run),
            (
                "200 OK".to_owned(),
                json!({"data": {"aweme_id":"free-video"}}).to_string(),
            ),
            ("201 Created".to_owned(), String::new()),
        ]);

        run_actor(&Client::new(), &request_config(address))
            .await
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 4);
        assert!(requests[2].method_and_path.contains("/api/tiktok/video?"));
        assert!(requests[3]
            .method_and_path
            .starts_with("POST /v2/datasets/dataset-test/items "));
        let row: Value = serde_json::from_str(&requests[3].body).unwrap();
        assert_eq!(row["aweme_id"], "free-video");
        assert_eq!(row["request_url"], VIDEO_URL);
        assert_eq!(row["request_index"], 1);
        assert_eq!(row["result_found"], true);
    }

    #[tokio::test]
    async fn budget_stops_before_scrappa_lookup_and_keeps_an_affordable_prefix() {
        let run = pricing_run(0.0003, json!({"other-event": 1}));
        let (address, server) = start_mock_server(vec![
            (
                "200 OK".to_owned(),
                json!({"urls": [VIDEO_URL, SECOND_VIDEO_URL]}).to_string(),
            ),
            ("200 OK".to_owned(), run),
            (
                "200 OK".to_owned(),
                json!({"data": {"aweme_id":"first"}}).to_string(),
            ),
            ("201 Created".to_owned(), String::new()),
        ]);

        run_actor(&Client::new(), &request_config(address))
            .await
            .unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[2].method_and_path.contains("/api/tiktok/video?"));
        let row: Value = serde_json::from_str(&requests[3].body).unwrap();
        assert_eq!(row["request_index"], 1);
        assert_eq!(row["aweme_id"], "first");
    }

    #[tokio::test]
    async fn zero_total_charge_cap_keeps_legacy_unbounded_output() {
        let run = pricing_run(0.0, json!({}));
        let (address, server) = start_mock_server(vec![
            ("200 OK".to_owned(), json!({"url": VIDEO_URL}).to_string()),
            ("200 OK".to_owned(), run),
            (
                "200 OK".to_owned(),
                json!({"data": {"aweme_id":"uncapped-video"}}).to_string(),
            ),
            ("201 Created".to_owned(), String::new()),
        ]);

        run_actor(&Client::new(), &request_config(address))
            .await
            .unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[2].method_and_path.contains("/api/tiktok/video?"));
        assert!(requests[3]
            .method_and_path
            .starts_with("POST /v2/datasets/dataset-test/items "));
        let row: Value = serde_json::from_str(&requests[3].body).unwrap();
        assert_eq!(row["aweme_id"], "uncapped-video");
        assert_eq!(row["request_url"], VIDEO_URL);
        assert_eq!(row["result_found"], true);
    }

    #[tokio::test]
    async fn scrappa_http_errors_become_dataset_error_rows() {
        let run = pricing_run(0.0002, json!({}));
        let (address, server) = start_mock_server(vec![
            ("200 OK".to_owned(), json!({"url": VIDEO_URL}).to_string()),
            ("200 OK".to_owned(), run),
            (
                "500 Internal Server Error".to_owned(),
                json!({"message":"upstream failed", "errors":{"url":["unavailable"]}}).to_string(),
            ),
            ("201 Created".to_owned(), String::new()),
        ]);

        run_actor(&Client::new(), &request_config(address))
            .await
            .unwrap();
        let requests = server.join().unwrap();
        let row: Value = serde_json::from_str(&requests[3].body).unwrap();
        assert_eq!(row["result_found"], false);
        assert_eq!(
            row["error_message"],
            "Scrappa API error (500): upstream failed - url: unavailable"
        );
    }

    #[tokio::test]
    async fn dataset_storage_errors_fail_the_actor() {
        let run = pricing_run(0.0002, json!({}));
        let (address, server) = start_mock_server(vec![
            ("200 OK".to_owned(), json!({"url": VIDEO_URL}).to_string()),
            ("200 OK".to_owned(), run),
            (
                "200 OK".to_owned(),
                json!({"data": {"aweme_id":"1"}}).to_string(),
            ),
            (
                "500 Internal Server Error".to_owned(),
                "dataset unavailable".to_owned(),
            ),
        ]);

        let error = run_actor(&Client::new(), &request_config(address))
            .await
            .unwrap_err();
        let requests = server.join().unwrap();
        assert!(error.to_string().contains("Apify dataset write failed"));
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[3].method_and_path.matches("POST").count(), 1);
    }
}
