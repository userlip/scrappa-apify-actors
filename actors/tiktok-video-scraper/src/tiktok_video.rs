use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use std::time::Duration;
use url::Url;

const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub(crate) const VIDEO_LOOKUP_ERROR: &str =
    "A valid TikTok video URL, short URL, photo URL, or video ID is required";

fn video_endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VideoRequest {
    pub(crate) url: String,
    pub(crate) validation_error: Option<String>,
}

pub(crate) fn resolve_video_requests(input: &Value) -> Result<Vec<VideoRequest>> {
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

pub(crate) fn require_video_lookup(value: &str) -> std::result::Result<(), String> {
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

pub(crate) fn build_video_url(base_url: &Url, video_url: &str, hd: bool) -> Result<Url> {
    let mut url = video_endpoint_url(base_url, &["tiktok", "video"])?;
    url.query_pairs_mut().append_pair("url", video_url);
    if hd {
        url.query_pairs_mut().append_pair("hd", "1");
    }
    Ok(url)
}

pub(crate) fn format_video_lookup_for_log(value: &str) -> Result<String> {
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

pub(crate) fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

pub(crate) fn js_trim(value: &str) -> &str {
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

pub(crate) struct ScrappaClient<'a> {
    pub(crate) http: &'a Client,
    pub(crate) base_url: &'a Url,
    pub(crate) api_key: &'a str,
}

impl ScrappaClient<'_> {
    pub(crate) async fn get_video(&self, video_url: &str, hd: bool) -> Result<Value> {
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

pub(crate) fn assert_successful_response(response: &Value, url: &str) -> Result<()> {
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

pub(crate) fn extract_video(data: Option<&Value>, url: &str) -> Option<Value> {
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

pub(crate) fn dataset_item(
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
