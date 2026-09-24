use anyhow::{anyhow, bail, Context, Result};
use reqwest::Response;
use serde_json::{json, Map, Value};
use std::{env, time::Duration};
use tokio::time::sleep;
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: u32 = 8;
const APIFY_RETRY_DELAY_MS: u64 = 500;
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

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

#[derive(Debug, PartialEq)]
struct TikTokHashtagPostsParams {
    challenge_name: Option<String>,
    challenge_id: Option<String>,
    region: Option<String>,
    count: Option<i64>,
    cursor: Option<String>,
    lookup_label: String,
}

impl TikTokHashtagPostsParams {
    fn append_to_url(&self, url: &mut Url) {
        let mut query = url.query_pairs_mut();
        if let Some(challenge_id) = &self.challenge_id {
            query.append_pair("challenge_id", challenge_id);
        }
        if let Some(challenge_name) = &self.challenge_name {
            query.append_pair("challenge_name", challenge_name);
        }
        if let Some(region) = &self.region {
            query.append_pair("region", region);
        }
        if let Some(count) = self.count {
            query.append_pair("count", &count.to_string());
        }
        if let Some(cursor) = &self.cursor {
            query.append_pair("cursor", cursor);
        }
    }

    fn metadata(&self) -> [(&'static str, Value); 3] {
        [
            (
                "lookup_challenge_name",
                optional_string(&self.challenge_name),
            ),
            ("lookup_challenge_id", optional_string(&self.challenge_id)),
            ("lookup_region", optional_string(&self.region)),
        ]
    }
}

fn optional_string(value: &Option<String>) -> Value {
    value.clone().map(Value::String).unwrap_or(Value::Null)
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

#[derive(Debug, PartialEq)]
struct ChallengeLookup {
    challenge_name: Option<String>,
    challenge_id: Option<String>,
    log_value: String,
}

fn normalize_hashtag_name(value: &str) -> Result<String> {
    let hashtag = value.strip_prefix('#').unwrap_or(value);
    let length = hashtag.chars().count();
    if !(1..=255).contains(&length)
        || hashtag.chars().any(|character| {
            is_js_whitespace(character) || matches!(character, '?' | '#' | '/' | '=' | ':')
        })
    {
        bail!("TikTok hashtag must be 1 to 255 characters and cannot contain whitespace or URL delimiter characters");
    }
    Ok(hashtag.to_owned())
}

fn looks_like_absolute_url(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once("://") else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
        })
}

fn normalize_tiktok_hashtag(value: &str) -> Result<String> {
    let trimmed = js_trim(value);
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if looks_like_absolute_url(trimmed) || trimmed.starts_with("//") {
        let url = Url::parse(trimmed)
            .map_err(|_| anyhow!("A valid TikTok hashtag URL or hashtag name is required"))?;
        let hostname = url.host_str().unwrap_or_default();
        if hostname != "tiktok.com" && !hostname.ends_with(".tiktok.com") {
            bail!("TikTok hashtag URL must be on tiktok.com");
        }
        if url.scheme() != "https" {
            bail!("TikTok hashtag URL must use HTTPS");
        }

        let path = url.path();
        let path_without_leading = path
            .strip_prefix("/tag/")
            .or_else(|| path.strip_prefix("/TAG/"))
            .or_else(|| {
                if path
                    .get(..5)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("/tag/"))
                {
                    path.get(5..)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                anyhow!("TikTok hashtag URL must use the format https://www.tiktok.com/tag/hashtag")
            })?;
        let hashtag = path_without_leading
            .strip_suffix('/')
            .unwrap_or(path_without_leading);
        if hashtag.is_empty() || hashtag.contains('/') {
            bail!("TikTok hashtag URL must use the format https://www.tiktok.com/tag/hashtag");
        }
        let hashtag = percent_encoding::percent_decode_str(hashtag)
            .decode_utf8()
            .map_err(|_| anyhow!("URI malformed"))?;
        return normalize_hashtag_name(&hashtag);
    }
    normalize_hashtag_name(trimmed)
}

fn normalize_challenge_id(value: &str) -> Result<String> {
    let challenge_id = js_trim(value);
    if challenge_id.is_empty() {
        return Ok(String::new());
    }
    if !challenge_id.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("TikTok challenge_id must contain digits only");
    }
    if challenge_id.len() > 100 {
        bail!("TikTok challenge_id must be 100 digits or fewer");
    }
    Ok(challenge_id.to_owned())
}

fn normalize_challenge_lookup(value: &str) -> Result<ChallengeLookup> {
    let trimmed = js_trim(value);
    if !trimmed.is_empty() && trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        let challenge_id = normalize_challenge_id(trimmed)?;
        return Ok(ChallengeLookup {
            log_value: format!("challenge_id:{challenge_id}"),
            challenge_name: None,
            challenge_id: Some(challenge_id),
        });
    }
    let challenge_name = normalize_tiktok_hashtag(trimmed)?;
    Ok(ChallengeLookup {
        log_value: format!("#{challenge_name}"),
        challenge_name: Some(challenge_name),
        challenge_id: None,
    })
}

fn resolve_challenge_lookup(input: &Value) -> Result<Option<ChallengeLookup>> {
    if let Some(value) = input.get("hashtag") {
        if let Some(hashtag) = value.as_str() {
            if !js_trim(hashtag).is_empty() {
                return normalize_challenge_lookup(hashtag).map(Some);
            }
        } else if !value.is_null() {
            eprintln!(
                "Warning: hashtag must be a string, got {}.",
                value_type(value)
            );
        }
    }

    if let Some(value) = input.get("challenge_name") {
        if let Some(challenge_name) = value.as_str() {
            let challenge_name = normalize_tiktok_hashtag(challenge_name)?;
            if !challenge_name.is_empty() {
                return Ok(Some(ChallengeLookup {
                    log_value: format!("#{challenge_name}"),
                    challenge_name: Some(challenge_name),
                    challenge_id: None,
                }));
            }
        } else if !value.is_null() {
            eprintln!(
                "Warning: challenge_name must be a string, got {}.",
                value_type(value)
            );
        }
    }

    if let Some(value) = input.get("challenge_id") {
        if let Some(challenge_id) = value.as_str() {
            let challenge_id = normalize_challenge_id(challenge_id)?;
            if !challenge_id.is_empty() {
                return Ok(Some(ChallengeLookup {
                    log_value: format!("challenge_id:{challenge_id}"),
                    challenge_name: None,
                    challenge_id: Some(challenge_id),
                }));
            }
        } else if !value.is_null() {
            eprintln!(
                "Warning: challenge_id must be a string, got {}.",
                value_type(value)
            );
        }
    }

    Ok(None)
}

fn normalize_region(value: &str) -> Option<String> {
    let region = js_trim(value).to_uppercase();
    if (2..=10).contains(&region.len()) && region.bytes().all(|byte| byte.is_ascii_uppercase()) {
        Some(region)
    } else {
        eprintln!(
            "Warning: region must be a 2 to 10 character country or region code. Omitting region."
        );
        None
    }
}

fn normalize_count(value: &Value) -> Option<i64> {
    let number = value.as_f64().filter(|number| {
        number.is_finite() && number.fract() == 0.0 && (1.0..=50.0).contains(number)
    });
    if let Some(number) = number {
        return Some(number as i64);
    }
    eprintln!(
        "Warning: count must be an integer between 1 and 50, got {}. Using Scrappa default.",
        js_string(value)
    );
    None
}

fn normalize_cursor(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => {
            let cursor = js_trim(value);
            (!cursor.is_empty()).then(|| cursor.to_owned())
        }
        Value::Number(number) => {
            let parsed = number.as_f64();
            if let Some(number) = parsed.filter(|number| {
                number.is_finite() && number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER
            }) {
                Some((number as i64).to_string())
            } else {
                eprintln!(
                    "Warning: cursor must be a string or safe integer, got {}. Starting from the first page.",
                    js_string(&Value::Number(number.clone()))
                );
                None
            }
        }
        Value::Null => None,
        value => {
            eprintln!(
                "Warning: cursor must be a string or number, got {}. Starting from the first page.",
                value_type(value)
            );
            None
        }
    }
}

fn build_hashtag_posts_params(input: &Value) -> Result<TikTokHashtagPostsParams> {
    let lookup = resolve_challenge_lookup(input)?;
    let Some(lookup) = lookup else {
        bail!("TikTok challenge_id or challenge_name is required");
    };

    let region = match input.get("region") {
        Some(Value::String(value)) if !js_trim(value).is_empty() => normalize_region(value),
        Some(Value::String(_)) | None | Some(Value::Null) => None,
        Some(value) => {
            eprintln!(
                "Warning: region must be a string, got {}. Omitting region.",
                value_type(value)
            );
            None
        }
    };
    let count = input.get("count").and_then(normalize_count);
    let cursor = input.get("cursor").and_then(normalize_cursor);

    Ok(TikTokHashtagPostsParams {
        challenge_name: lookup.challenge_name,
        challenge_id: lookup.challenge_id,
        region,
        count,
        cursor,
        lookup_label: lookup.log_value,
    })
}

fn optional_js_number_string(value: &Value) -> Option<String> {
    let number = value.as_f64()?;
    if !number.is_finite() || number.fract() != 0.0 || number.abs() > MAX_SAFE_INTEGER {
        return None;
    }
    Some((number as i64).to_string())
}

fn get_challenge_id(challenge: &Value) -> Option<String> {
    let id = challenge
        .get("challenge_id")
        .filter(|value| !value.is_null())
        .or_else(|| challenge.get("id"))?;
    match id {
        Value::String(id) => {
            let id = js_trim(id);
            (!id.is_empty()).then(|| id.to_owned())
        }
        Value::Number(_) => optional_js_number_string(id),
        _ => None,
    }
}

fn get_challenge_name(challenge: &Value) -> String {
    challenge
        .get("challenge_name")
        .filter(|value| !value.is_null())
        .or_else(|| challenge.get("cha_name"))
        .and_then(Value::as_str)
        .map(js_trim)
        .unwrap_or_default()
        .to_owned()
}

fn extract_challenges(data: Option<&Value>) -> Vec<Value> {
    let Some(data) = data.filter(|value| !value.is_null()) else {
        return Vec::new();
    };
    if let Some(challenges) = data.as_array() {
        return challenges.clone();
    }
    for key in ["challenges", "challenge_list"] {
        if let Some(challenges) = data.get(key).and_then(Value::as_array) {
            return challenges.clone();
        }
    }
    Vec::new()
}

fn normalize_challenge_name(value: &str) -> String {
    js_trim(value)
        .strip_prefix('#')
        .unwrap_or(js_trim(value))
        .to_lowercase()
}

fn select_challenge_for_hashtag<'a>(challenges: &'a [Value], hashtag: &str) -> Option<&'a Value> {
    let normalized_target = normalize_challenge_name(hashtag);
    challenges.iter().find(|challenge| {
        normalize_challenge_name(&get_challenge_name(challenge)) == normalized_target
    })
}

fn extract_posts(data: Option<&Value>) -> Vec<&Value> {
    let Some(data) = data.filter(|value| !value.is_null()) else {
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

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_none_or(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn extract_pagination(data: Option<&Value>) -> (bool, Value) {
    let Some(data) = data.filter(|value| !value.is_null() && !value.is_array()) else {
        return (false, Value::Null);
    };
    let has_next_page = data
        .get("hasMore")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("has_more").filter(|value| !value.is_null()))
        .is_some_and(js_truthy);
    let next_cursor = data
        .get("cursor")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("max_cursor").filter(|value| !value.is_null()))
        .or_else(|| data.get("min_cursor").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null);
    (has_next_page, next_cursor)
}

fn enrich_post(
    post: &Value,
    params: &TikTokHashtagPostsParams,
    resolved_challenge_name: Option<&str>,
    resolved_challenge_id: Option<&str>,
) -> Value {
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
    for (key, value) in params.metadata() {
        row.insert(key.to_owned(), value);
    }
    row.insert(
        "resolved_challenge_name".to_owned(),
        resolved_challenge_name
            .map(|name| Value::String(name.to_owned()))
            .unwrap_or(Value::Null),
    );
    row.insert(
        "resolved_challenge_id".to_owned(),
        resolved_challenge_id
            .map(|id| Value::String(id.to_owned()))
            .unwrap_or(Value::Null),
    );
    Value::Object(row)
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

async fn send_apify_request<F>(operation: &str, build_request: F) -> Result<Response>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let mut retries = 0;
    loop {
        match build_request().timeout(APIFY_REQUEST_TIMEOUT).send().await {
            Ok(response)
                if (response.status().as_u16() == 429 || response.status().is_server_error())
                    && retries < APIFY_MAX_RETRIES =>
            {
                drop(response);
            }
            Ok(response) => return Ok(response),
            Err(error) if retries < APIFY_MAX_RETRIES => {
                eprintln!("Apify {operation} failed: {error}");
            }
            Err(error) => return Err(error).with_context(|| format!("Apify {operation} failed")),
        }

        let delay = Duration::from_millis(APIFY_RETRY_DELAY_MS * (1_u64 << retries));
        retries += 1;
        eprintln!(
            "Retrying Apify {operation} in {}ms ({retries}/{APIFY_MAX_RETRIES})",
            delay.as_millis()
        );
        sleep(delay).await;
    }
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
    let response = send_apify_request("INPUT request", || {
        client.get(url.clone()).bearer_auth(&config.apify_token)
    })
    .await?;
    if response.status().as_u16() == 404 {
        return Ok(None);
    }
    response_json(response, "Apify INPUT request")
        .await
        .map(Some)
}

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() || error.to_string().contains("aborted") {
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
                            .map(js_string)
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
    endpoint: &[&str],
    query: &[(&str, String)],
) -> Result<Value> {
    let mut url = endpoint_url(&config.scrappa_api_base_url, endpoint)?;
    if !query.is_empty() {
        let mut query_pairs = url.query_pairs_mut();
        for (key, value) in query {
            query_pairs.append_pair(key, value);
        }
    }
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

fn validate_scrappa_code(response: &Value, operation: &str) -> Result<()> {
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
        "Scrappa {operation} API returned code {}: {message}",
        js_string(code)
    );
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
            let response = send_apify_request("run pricing request", || {
                client.get(url.clone()).bearer_auth(&config.apify_token)
            })
            .await?;
            self.run = Some(response_json(response, "Apify run pricing request").await?);
        }
        let run = self
            .run
            .as_ref()
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        affordable_dataset_items(run, requested, self.saved_rows)
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
            .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?,
    };
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    if max_charge == 0.0 {
        return Ok(requested);
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
    let response = send_apify_request("dataset write", || {
        client
            .post(url.clone())
            .bearer_auth(&config.apify_token)
            .json(rows)
    })
    .await?;
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
    let response = send_apify_request("OUTPUT write", || {
        client
            .put(url.clone())
            .bearer_auth(&config.apify_token)
            .json(output)
    })
    .await?;
    ensure_success(response, "Apify OUTPUT write").await
}

async fn resolve_challenge(
    client: &reqwest::Client,
    config: &ActorConfig,
    challenge_name: &str,
) -> Result<(String, Option<String>)> {
    let response = fetch_scrappa_response(
        client,
        config,
        &["tiktok", "challenges", "search"],
        &[
            ("keywords", challenge_name.to_owned()),
            ("count", "10".to_owned()),
        ],
    )
    .await?;
    validate_scrappa_code(&response, "TikTok Challenge Search")?;

    let data = response.get("data");
    let challenges = extract_challenges(data);
    let selection = select_challenge_for_hashtag(&challenges, challenge_name);
    let challenge_id = selection.and_then(get_challenge_id).ok_or_else(|| {
        anyhow!("Could not resolve TikTok hashtag \"{challenge_name}\" to a challenge_id")
    })?;
    let resolved_name = selection
        .map(get_challenge_name)
        .filter(|name| !name.is_empty())
        .or_else(|| Some(challenge_name.to_owned()));
    Ok((challenge_id, resolved_name))
}

async fn run_actor(client: &reqwest::Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let Some(input) = input.filter(js_truthy) else {
        bail!("TikTok challenge_id or challenge_name is required");
    };
    let params = build_hashtag_posts_params(&input)?;
    println!("Fetching TikTok hashtag posts for: {}", params.lookup_label);

    let mut posts_params = TikTokHashtagPostsParams {
        challenge_name: params.challenge_name.clone(),
        challenge_id: params.challenge_id.clone(),
        region: params.region.clone(),
        count: params.count,
        cursor: params.cursor.clone(),
        lookup_label: params.lookup_label.clone(),
    };
    let mut resolved_challenge_name = posts_params.challenge_name.clone();
    if let Some(challenge_name) = posts_params.challenge_name.clone() {
        if posts_params.challenge_id.is_none() {
            let (challenge_id, resolved_name) =
                resolve_challenge(client, config, &challenge_name).await?;
            posts_params.challenge_id = Some(challenge_id.clone());
            posts_params.challenge_name = None;
            resolved_challenge_name = resolved_name;
            println!(
                "Resolved hashtag to challenge_id:{challenge_id}{}",
                resolved_challenge_name
                    .as_ref()
                    .map(|name| format!(" ({name})"))
                    .unwrap_or_default()
            );
        }
    }

    let mut url = endpoint_url(
        &config.scrappa_api_base_url,
        &["tiktok", "challenges", "posts"],
    )?;
    posts_params.append_to_url(&mut url);
    let query = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    let query = query
        .iter()
        .map(|(key, value)| (key.as_str(), value.clone()))
        .collect::<Vec<_>>();
    let response =
        fetch_scrappa_response(client, config, &["tiktok", "challenges", "posts"], &query).await?;
    validate_scrappa_code(&response, "TikTok Hashtag Posts")?;

    let data = response.get("data");
    let posts = extract_posts(data);
    let (has_next_page, next_cursor) = extract_pagination(data);
    let saved_posts = if posts.is_empty() {
        println!("No posts found for the given TikTok hashtag lookup");
        0
    } else {
        let rows = posts
            .iter()
            .map(|post| {
                enrich_post(
                    post,
                    &params,
                    resolved_challenge_name.as_deref(),
                    posts_params.challenge_id.as_deref(),
                )
            })
            .collect::<Vec<_>>();
        let saved =
            push_dataset_items(client, config, &mut DatasetBudget::default(), &rows).await?;
        println!("Found {} posts; saved {saved}", posts.len());
        saved
    };

    put_output(client, config, &response).await?;
    let processed_time = response
        .get("processed_time")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);
    println!("TikTok hashtag posts extraction completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "posts_extracted": posts.len(),
            "posts_saved": saved_posts,
            "has_next_page": has_next_page,
            "next_cursor": next_cursor,
            "processed_time": processed_time,
        })
    );
    Ok(())
}

fn failure_message(error: &anyhow::Error) -> String {
    let message = format!("{error:#}");
    if message.contains("timed out") {
        format!(
            "{message}. The TikTok hashtag posts request exceeded the {}s Scrappa API timeout. Try a more specific hashtag or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
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
            apify_api_base_url: Url::parse(&format!("http://{address}/")).unwrap(),
            scrappa_api_base_url: Url::parse(&format!("http://{address}/api")).unwrap(),
            default_key_value_store_id: "store-test".to_owned(),
            default_dataset_id: "dataset-test".to_owned(),
            actor_run_id: "run-test".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-apify-token".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    #[test]
    fn keeps_actor_input_prefill_and_limits_in_schema() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let properties = schema["properties"].as_object().unwrap();
        assert_eq!(schema["required"][0], "hashtag");
        assert_eq!(properties["hashtag"]["prefill"], "cosplay");
        assert_eq!(properties["region"]["prefill"], "US");
        assert_eq!(properties["count"]["default"], 10);
        assert_eq!(properties["cursor"]["prefill"], "0");
        assert_eq!(properties["count"]["minimum"], 1);
        assert_eq!(properties["count"]["maximum"], 50);
    }

    #[test]
    fn builds_params_from_hashtag_alias_and_valid_optional_fields() {
        assert_eq!(
            build_hashtag_posts_params(&value(
                r##"{"hashtag":" #cosplay ","region":" us ","count":10,"cursor":" 0 "}"##
            ))
            .unwrap(),
            TikTokHashtagPostsParams {
                challenge_name: Some("cosplay".to_owned()),
                challenge_id: None,
                region: Some("US".to_owned()),
                count: Some(10),
                cursor: Some("0".to_owned()),
                lookup_label: "#cosplay".to_owned(),
            }
        );
        assert_eq!(
            build_hashtag_posts_params(&value(r#"{"hashtag":"33380"}"#))
                .unwrap()
                .challenge_id,
            Some("33380".to_owned())
        );
        assert_eq!(
            build_hashtag_posts_params(&value(
                r#"{"challenge_name":"BookTok","challenge_id":"abc"}"#
            ))
            .unwrap()
            .challenge_name,
            Some("BookTok".to_owned())
        );
    }

    #[test]
    fn validates_urls_names_and_challenge_ids() {
        assert_eq!(
            build_hashtag_posts_params(&value(
                r#"{"hashtag":"https://www.tiktok.com/tag/cosplay?lang=en"}"#
            ))
            .unwrap()
            .challenge_name,
            Some("cosplay".to_owned())
        );
        assert_eq!(
            build_hashtag_posts_params(&value(&format!(r#"{{"hashtag":"{}"}}"#, "1".repeat(100))))
                .unwrap()
                .challenge_id,
            Some("1".repeat(100))
        );
        assert!(
            build_hashtag_posts_params(&value(r#"{"hashtag":"https://example.com/tag/fyp"}"#))
                .unwrap_err()
                .to_string()
                .contains("must be on tiktok.com")
        );
        assert!(
            build_hashtag_posts_params(&value(r#"{"hashtag":"http://tiktok.com/tag/fyp"}"#))
                .unwrap_err()
                .to_string()
                .contains("must use HTTPS")
        );
        assert!(
            build_hashtag_posts_params(&value(r#"{"challenge_id":"abc"}"#))
                .unwrap_err()
                .to_string()
                .contains("digits only")
        );
        assert!(build_hashtag_posts_params(&value(r#"{"hashtag":"x/y"}"#))
            .unwrap_err()
            .to_string()
            .contains("cannot contain whitespace or URL delimiter characters"));
    }

    #[test]
    fn warns_and_omits_invalid_optional_values() {
        let params = build_hashtag_posts_params(&value(
            r#"{"hashtag":"fyp","region":12,"count":0,"cursor":true}"#,
        ))
        .unwrap();
        assert_eq!(params.region, None);
        assert_eq!(params.count, None);
        assert_eq!(params.cursor, None);
        assert!(build_hashtag_posts_params(&value(r#"{"hashtag":" "}"#))
            .unwrap_err()
            .to_string()
            .contains("TikTok challenge_id or challenge_name is required"));
    }

    #[test]
    fn selects_only_exact_case_insensitive_challenge_matches() {
        let challenges =
            value(r#"[{"id":"1","cha_name":"cosplaygirl"},{"id":"33380","cha_name":"Cosplay"}]"#);
        let selected = select_challenge_for_hashtag(challenges.as_array().unwrap(), "#cosplay");
        assert_eq!(
            get_challenge_id(selected.unwrap()),
            Some("33380".to_owned())
        );
        assert_eq!(get_challenge_name(selected.unwrap()), "Cosplay");
        assert!(select_challenge_for_hashtag(challenges.as_array().unwrap(), "cos").is_none());
        assert_eq!(
            extract_challenges(Some(&value(r#"{"challenge_list":[{"id":"2"}]}"#))).len(),
            1
        );
        assert_eq!(
            get_challenge_id(&value(r#"{"challenge_id":33380}"#)),
            Some("33380".to_owned())
        );
    }

    #[test]
    fn extracts_post_shapes_and_pagination_fields() {
        let data = value(
            r#"{"posts":[{"aweme_id":"1"}],"videos":[{"aweme_id":"ignored"}],"hasMore":true,"has_more":false,"cursor":"100","max_cursor":"200"}"#,
        );
        assert_eq!(extract_posts(Some(&data)).len(), 1);
        assert_eq!(
            extract_pagination(Some(&data)),
            (true, Value::String("100".to_owned()))
        );
        assert_eq!(
            extract_posts(Some(&value(r#"{"aweme_list":[{"aweme_id":"3"}]}"#))).len(),
            1
        );
        assert_eq!(
            extract_posts(Some(&value(r#"[{"aweme_id":"4"}]"#))).len(),
            1
        );
        assert_eq!(extract_pagination(Some(&Value::Null)), (false, Value::Null));
    }

    #[test]
    fn preserves_lookup_metadata_and_raw_post_fields() {
        let params =
            build_hashtag_posts_params(&value(r#"{"hashtag":"cosplay","region":"us"}"#)).unwrap();
        let post = enrich_post(
            &value(r#"{"aweme_id":"1","lookup_region":"old"}"#),
            &params,
            Some("Cosplay"),
            Some("33380"),
        );
        assert_eq!(post["aweme_id"], "1");
        assert_eq!(post["lookup_challenge_name"], "cosplay");
        assert!(post["lookup_challenge_id"].is_null());
        assert_eq!(post["lookup_region"], "US");
        assert_eq!(post["resolved_challenge_name"], "Cosplay");
        assert_eq!(post["resolved_challenge_id"], "33380");
    }

    #[test]
    fn validates_scrappa_api_codes() {
        assert!(validate_scrappa_code(&value(r#"{"code":0}"#), "test").is_ok());
        assert!(validate_scrappa_code(&value(r#"{"data":[]}"#), "test").is_ok());
        assert!(
            validate_scrappa_code(&value(r#"{"code":1,"msg":"bad"}"#), "test")
                .unwrap_err()
                .to_string()
                .contains("code 1: bad")
        );
    }

    #[test]
    fn allows_all_dataset_items_for_non_ppe_runs() {
        let run = value(r#"{"data":{"pricingInfo":{"pricingModel":"PRICE_PER_DATASET_ITEM"}}}"#);
        assert_eq!(affordable_dataset_items(&run, 3, 0).unwrap(), 3);
    }

    #[test]
    fn treats_zero_null_and_missing_ppe_limits_as_unlimited() {
        let runs = [
            r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"},"options":{"maxTotalChargeUsd":0}}}"#,
            r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"},"options":{"maxTotalChargeUsd":null}}}"#,
            r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"},"options":{}}}"#,
        ];
        for run in runs {
            assert_eq!(affordable_dataset_items(&value(run), 3, 0).unwrap(), 3);
        }
    }

    #[test]
    fn enforces_positive_ppe_limits_using_charges_and_saved_rows() {
        let run = value(
            r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001},"other-event":{"eventPriceUsd":0.0001}}}},"options":{"maxTotalChargeUsd":0.00025},"chargedEventCounts":{"other-event":1}}}"#,
        );
        assert_eq!(affordable_dataset_items(&run, 3, 0).unwrap(), 1);
        assert_eq!(affordable_dataset_items(&run, 3, 1).unwrap(), 0);
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

    #[tokio::test]
    async fn resolves_fetches_caps_and_persists_raw_output_with_expected_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let responses = [
                (
                    "200 OK",
                    r##"{"hashtag":"#Cosplay","region":"us","count":2,"cursor":"0"}"##,
                ),
                (
                    "200 OK",
                    r#"{"code":0,"data":{"challenge_list":[{"challenge_id":"33380","cha_name":"Cosplay"}]}}"#,
                ),
                (
                    "200 OK",
                    r#"{"code":0,"data":{"aweme_list":[{"aweme_id":"1","desc":"clip"},{"aweme_id":"2"}],"has_more":true,"max_cursor":"next"},"processed_time":99,"raw":"retained"}"#,
                ),
                (
                    "200 OK",
                    r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0003}}}},"options":{"maxTotalChargeUsd":0.0003},"chargedEventCounts":{"apify-default-dataset-item":0}}}"#,
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

        run_actor(&reqwest::Client::new(), &request_config(address))
            .await
            .unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 6);
        assert!(requests[0]
            .method_and_path
            .starts_with("GET /v2/key-value-stores/store-test/records/INPUT "));
        assert!(requests[1]
            .method_and_path
            .starts_with("GET /api/tiktok/challenges/search?"));
        assert!(requests[2]
            .method_and_path
            .starts_with("GET /api/tiktok/challenges/posts?"));
        assert!(requests[3]
            .method_and_path
            .starts_with("GET /v2/actor-runs/run-test "));
        assert!(requests[4]
            .method_and_path
            .starts_with("POST /v2/datasets/dataset-test/items "));
        assert!(requests[5]
            .method_and_path
            .starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT "));
        for index in [0, 3, 4, 5] {
            assert_eq!(
                requests[index]
                    .headers
                    .get("authorization")
                    .map(String::as_str),
                Some("Bearer test-apify-token")
            );
            assert!(!requests[index].headers.contains_key("x-api-key"));
        }
        for index in [1, 2] {
            assert_eq!(
                requests[index].headers.get("x-api-key").map(String::as_str),
                Some("test-scrappa-key")
            );
            assert_eq!(
                requests[index].headers.get("accept").map(String::as_str),
                Some("application/json")
            );
        }
        let search_url = Url::parse(&format!(
            "http://mock{}",
            requests[1]
                .method_and_path
                .split_whitespace()
                .nth(1)
                .unwrap()
        ))
        .unwrap();
        let search_query = search_url
            .query_pairs()
            .into_owned()
            .collect::<HashMap<_, _>>();
        assert_eq!(search_query["keywords"], "Cosplay");
        assert_eq!(search_query["count"], "10");
        let posts_url = Url::parse(&format!(
            "http://mock{}",
            requests[2]
                .method_and_path
                .split_whitespace()
                .nth(1)
                .unwrap()
        ))
        .unwrap();
        let posts_query = posts_url
            .query_pairs()
            .into_owned()
            .collect::<HashMap<_, _>>();
        assert_eq!(posts_query["challenge_id"], "33380");
        assert!(!posts_query.contains_key("challenge_name"));
        assert_eq!(posts_query["region"], "US");
        assert_eq!(posts_query["count"], "2");
        assert_eq!(posts_query["cursor"], "0");

        let rows: Value = serde_json::from_str(&requests[4].body).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 1);
        assert_eq!(rows[0]["aweme_id"], "1");
        assert_eq!(rows[0]["lookup_challenge_name"], "Cosplay");
        assert!(rows[0]["lookup_challenge_id"].is_null());
        assert_eq!(rows[0]["resolved_challenge_name"], "Cosplay");
        assert_eq!(rows[0]["resolved_challenge_id"], "33380");
        assert_eq!(rows[0]["lookup_region"], "US");
        let output: Value = serde_json::from_str(&requests[5].body).unwrap();
        assert_eq!(output["raw"], "retained");
        assert_eq!(output["data"]["aweme_list"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn zero_ppe_limit_is_uncapped_and_keeps_raw_output() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let responses = [
                ("200 OK", r#"{"challenge_id":"33380"}"#),
                (
                    "200 OK",
                    r#"{"code":0,"data":{"posts":[{"aweme_id":"1"}]}}"#,
                ),
                (
                    "200 OK",
                    r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0003}}}},"options":{"maxTotalChargeUsd":0.0},"chargedEventCounts":{}}}"#,
                ),
                ("201 Created", ""),
                ("201 Created", ""),
            ];
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                requests.push(read_request(&mut stream));
                mock_response(&mut stream, status, body);
            }
            requests
        });
        run_actor(&reqwest::Client::new(), &request_config(address))
            .await
            .unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 5);
        assert!(requests[3]
            .method_and_path
            .starts_with("POST /v2/datasets/dataset-test/items "));
        let rows: Value = serde_json::from_str(&requests[3].body).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 1);
        assert!(requests[4]
            .method_and_path
            .starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT "));
    }

    #[test]
    fn reports_scrappa_timeout_with_the_original_deadline() {
        assert!(
            failure_message(&anyhow!("Scrappa API request timed out after 60000ms"))
                .contains("60s Scrappa API timeout")
        );
        assert_eq!(SCRAPPA_REQUEST_TIMEOUT, Duration::from_secs(60));
        assert_eq!(APIFY_REQUEST_TIMEOUT, Duration::from_secs(360));
        assert_eq!(APIFY_MAX_RETRIES, 8);
    }

    #[tokio::test]
    async fn retries_transient_apify_errors_but_returns_client_errors_directly() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, body) in [("500 Internal Server Error", "{}"), ("200 OK", "{}")] {
                let (mut stream, _) = listener.accept().unwrap();
                requests.push(read_request(&mut stream));
                mock_response(&mut stream, status, body);
            }
            requests
        });
        let client = reqwest::Client::new();
        let url = format!("http://{address}/transient");
        let response = send_apify_request("test request", || client.get(&url))
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(server.join().unwrap().len(), 2);

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            mock_response(&mut stream, "400 Bad Request", "{} ");
            request
        });
        let url = format!("http://{address}/permanent");
        let response = send_apify_request("test request", || client.get(&url))
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
        server.join().unwrap();
    }

    #[tokio::test]
    async fn returns_scrappa_http_errors_without_retrying() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            mock_response(
                &mut stream,
                "503 Service Unavailable",
                "upstream unavailable",
            );
            request
        });
        let config = request_config(address);
        let error = fetch_scrappa_response(
            &reqwest::Client::new(),
            &config,
            &["tiktok", "challenges", "posts"],
            &[],
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("Scrappa API error (503)"));
        assert!(server
            .join()
            .unwrap()
            .method_and_path
            .starts_with("GET /api/tiktok/challenges/posts "));
    }
}
