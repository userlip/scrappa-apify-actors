use anyhow::{bail, Result};
use serde_json::Value;
use url::Url;

use crate::response::js_string;

#[derive(Debug, Default, PartialEq)]
pub(crate) struct TikTokUserPostsParams {
    pub(crate) unique_id: Option<String>,
    pub(crate) user_id: Option<String>,
    pub(crate) count: Option<i64>,
    pub(crate) cursor: Option<String>,
}

impl TikTokUserPostsParams {
    pub(crate) fn append_to_url(&self, url: &mut Url) {
        let mut query = url.query_pairs_mut();
        if let Some(unique_id) = &self.unique_id {
            query.append_pair("unique_id", unique_id);
        }
        if let Some(user_id) = &self.user_id {
            query.append_pair("user_id", user_id);
        }
        if let Some(count) = self.count {
            query.append_pair("count", &count.to_string());
        }
        if let Some(cursor) = &self.cursor {
            query.append_pair("cursor", cursor);
        }
    }
}

pub(crate) fn build_params(input: &Value) -> Result<TikTokUserPostsParams> {
    build_params_with_warning(input, |message| eprintln!("{message}"))
}

pub(crate) fn build_params_with_warning<F>(
    input: &Value,
    mut warn: F,
) -> Result<TikTokUserPostsParams>
where
    F: FnMut(String),
{
    let mut params = TikTokUserPostsParams::default();
    match input.get("profile") {
        Some(Value::String(profile)) if !js_trim(profile).is_empty() => {
            set_lookup(&mut params, normalize_profile_lookup(profile)?)
        }
        Some(value) if !value.is_null() && value.as_str() != Some("") => warn(format!(
            "profile must be a string, got {}.",
            value_type(value)
        )),
        _ => {}
    }

    if params.unique_id.is_none() && params.user_id.is_none() {
        match input.get("unique_id") {
            Some(Value::String(unique_id)) => {
                let unique_id = normalize_tiktok_unique_id(unique_id)?;
                if !unique_id.is_empty() {
                    params.unique_id = Some(unique_id);
                }
            }
            Some(value) if !value.is_null() && value.as_str() != Some("") => warn(format!(
                "unique_id must be a string, got {}.",
                value_type(value)
            )),
            _ => {}
        }
    }

    if params.unique_id.is_none() && params.user_id.is_none() {
        match input.get("user_id") {
            Some(Value::String(user_id)) => {
                let user_id = normalize_tiktok_user_id(user_id)?;
                if !user_id.is_empty() {
                    params.user_id = Some(user_id);
                }
            }
            Some(value) if !value.is_null() && value.as_str() != Some("") => warn(format!(
                "user_id must be a string, got {}.",
                value_type(value)
            )),
            _ => {}
        }
    }

    if let Some(count) = input.get("count") {
        let normalized = count.as_f64().filter(|number| {
            number.is_finite() && number.fract() == 0.0 && (1.0..=50.0).contains(number)
        });
        if let Some(count) = normalized {
            params.count = Some(count as i64);
        } else {
            warn(format!(
                "count must be an integer between 1 and 50, got {}. Using Scrappa default.",
                js_string(count)
            ));
        }
    }

    match input.get("cursor") {
        Some(Value::String(cursor)) => {
            let cursor = js_trim(cursor);
            if !cursor.is_empty() {
                params.cursor = Some(cursor.to_owned());
            }
        }
        Some(value) if !value.is_null() && value.as_str() != Some("") => warn(format!(
            "cursor must be a string, got {}. Starting from the first page.",
            value_type(value)
        )),
        _ => {}
    }

    if params.unique_id.is_none() && params.user_id.is_none() {
        bail!("TikTok unique_id or user_id is required");
    }
    Ok(params)
}

pub(crate) fn set_lookup(params: &mut TikTokUserPostsParams, (field, value): (&str, String)) {
    match field {
        "unique_id" => params.unique_id = Some(value),
        "user_id" => params.user_id = Some(value),
        _ => unreachable!("only TikTok lookup fields are produced"),
    }
}

pub(crate) fn format_lookup_for_log(input: &Value) -> Result<String> {
    if let Some(profile) = input
        .get("profile")
        .and_then(Value::as_str)
        .map(js_trim)
        .filter(|value| !value.is_empty())
    {
        let (field, value) = normalize_profile_lookup(profile)?;
        return Ok(match field {
            "unique_id" => value,
            "user_id" => format!("user_id:{value}"),
            _ => unreachable!(),
        });
    }
    if let Some(unique_id) = input
        .get("unique_id")
        .and_then(Value::as_str)
        .map(js_trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(normalize_tiktok_unique_id(unique_id)?);
    }
    if let Some(user_id) = input
        .get("user_id")
        .and_then(Value::as_str)
        .map(js_trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(format!("user_id:{}", normalize_tiktok_user_id(user_id)?));
    }
    Ok("unknown TikTok profile".to_owned())
}

pub(crate) fn normalize_profile_lookup(value: &str) -> Result<(&'static str, String)> {
    let trimmed = js_trim(value);
    if trimmed.bytes().all(|byte| byte.is_ascii_digit()) && !trimmed.is_empty() {
        return Ok(("user_id", normalize_tiktok_user_id(trimmed)?));
    }
    Ok(("unique_id", normalize_tiktok_unique_id(trimmed)?))
}

pub(crate) fn normalize_tiktok_unique_id(value: &str) -> Result<String> {
    let trimmed = js_trim(value);
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let url_like =
        has_url_scheme(trimmed) || trimmed.starts_with("//") || looks_like_tiktok_domain(trimmed);

    if let Ok(parsed) = Url::parse(trimmed) {
        let host = parsed.host_str().unwrap_or_default();
        if !(host.eq_ignore_ascii_case("tiktok.com")
            || host.to_ascii_lowercase().ends_with(".tiktok.com"))
        {
            bail!("TikTok profile URL must be on tiktok.com");
        }
        if parsed.scheme() != "https" {
            bail!("TikTok profile URL must use HTTPS");
        }
        let path = parsed.path();
        let username = path
            .strip_prefix('/')
            .and_then(|path| path.strip_suffix('/').or(Some(path)))
            .filter(|path| path.starts_with('@') && !path[1..].contains('/'));
        let Some(username) = username else {
            bail!("TikTok profile URL must use the format https://www.tiktok.com/@username");
        };
        return normalize_tiktok_username(username);
    }

    if url_like {
        bail!("A valid TikTok profile URL or username is required");
    }
    normalize_tiktok_username(trimmed)
}

fn has_url_scheme(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once("://") else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
        })
}

pub(crate) fn is_js_whitespace(character: char) -> bool {
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

fn looks_like_tiktok_domain(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.match_indices("tiktok.com").any(|(index, _)| {
        let before = index == 0 || value.as_bytes().get(index - 1) == Some(&b'.');
        let after = value.as_bytes().get(index + "tiktok.com".len());
        before && (after.is_none() || after == Some(&b'/'))
    })
}

pub(crate) fn normalize_tiktok_username(value: &str) -> Result<String> {
    let username = value.strip_prefix('@').unwrap_or(value);
    let valid = (2..=255).contains(&username.len())
        && username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'));
    if !valid {
        bail!("TikTok username must be 2 to 255 characters and contain only letters, numbers, dots, or underscores");
    }
    Ok(format!("@{username}"))
}

pub(crate) fn normalize_tiktok_user_id(value: &str) -> Result<String> {
    let user_id = js_trim(value);
    if user_id.is_empty() {
        return Ok(String::new());
    }
    if !user_id.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("TikTok user_id must contain digits only");
    }
    if user_id.len() > 30 {
        bail!("TikTok numeric user ID must be 30 digits or fewer");
    }
    Ok(user_id.to_owned())
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null | Value::Array(_) | Value::Object(_) => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
    }
}
