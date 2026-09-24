use anyhow::{anyhow, bail, Result};
use serde_json::{Number, Value};
use url::Url;

use crate::value::js_string;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FollowingParams {
    pub(crate) unique_id: Option<String>,
    pub(crate) user_id: Option<String>,
    pub(crate) count: Option<f64>,
    pub(crate) time: Option<Value>,
}

impl FollowingParams {
    pub(crate) fn requested_count(&self) -> f64 {
        self.count.unwrap_or(10.0)
    }
}

fn warn_for_non_string(value: Option<&Value>, field: &str, warn: &mut impl FnMut(String)) {
    if let Some(value) = value {
        if !value.is_null() && value.as_str() != Some("") {
            warn(format!(
                "{field} must be a string, got {}.",
                js_typeof(value)
            ));
        }
    }
}

fn js_typeof(value: &Value) -> &'static str {
    match value {
        Value::Null => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) | Value::Object(_) => "object",
    }
}

fn resolve_lookup(
    input: &Value,
    warn: &mut impl FnMut(String),
) -> Result<(Option<String>, Option<String>)> {
    if let Some(value) = input.get("profile") {
        if let Some(profile) = value.as_str() {
            let profile = profile.trim();
            if !profile.is_empty() {
                if profile.bytes().all(|byte| byte.is_ascii_digit()) {
                    let user_id = normalize_user_id(profile)?;
                    if !user_id.is_empty() {
                        return Ok((None, Some(user_id)));
                    }
                } else {
                    let unique_id = normalize_unique_id(profile)?;
                    if !unique_id.is_empty() {
                        return Ok((Some(unique_id), None));
                    }
                }
            }
        } else {
            warn_for_non_string(Some(value), "profile", warn);
        }
    }

    if let Some(value) = input.get("unique_id") {
        if let Some(unique_id) = value.as_str() {
            let unique_id = normalize_unique_id(unique_id)?;
            if !unique_id.is_empty() {
                return Ok((Some(unique_id), None));
            }
        } else {
            warn_for_non_string(Some(value), "unique_id", warn);
        }
    }

    if let Some(value) = input.get("user_id") {
        if let Some(user_id) = value.as_str() {
            let user_id = normalize_user_id(user_id)?;
            if !user_id.is_empty() {
                return Ok((None, Some(user_id)));
            }
        } else {
            warn_for_non_string(Some(value), "user_id", warn);
        }
    }

    Ok((None, None))
}

fn is_tiktok_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("tiktok.com") || host.to_ascii_lowercase().ends_with(".tiktok.com")
}

fn is_url_like(value: &str) -> bool {
    let has_scheme = value.split_once("://").is_some_and(|(scheme, _)| {
        let mut chars = scheme.chars();
        chars
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
            && chars.all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
            })
    });
    if has_scheme || value.starts_with("//") {
        return true;
    }

    let lower = value.to_ascii_lowercase();
    lower.match_indices("tiktok.com").any(|(index, host)| {
        let before_is_boundary = index == 0 || lower.as_bytes()[index - 1] == b'.';
        let after_index = index + host.len();
        let after_is_boundary = after_index == lower.len() || lower.as_bytes()[after_index] == b'/';
        before_is_boundary && after_is_boundary
    })
}

fn normalize_unique_id(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    if is_url_like(trimmed) {
        let parsed = Url::parse(trimmed)
            .map_err(|_| anyhow!("A valid TikTok profile URL or username is required"))?;
        if !is_tiktok_host(parsed.host_str().unwrap_or_default()) {
            bail!("TikTok profile URL must be on tiktok.com");
        }
        if parsed.scheme() != "https" {
            bail!("TikTok profile URL must use HTTPS");
        }

        let path = parsed.path();
        let Some(username) = path
            .strip_prefix("/@")
            .map(|tail| tail.strip_suffix('/').unwrap_or(tail))
            .filter(|username| !username.is_empty() && !username.contains('/'))
        else {
            bail!("TikTok profile URL must use the format https://www.tiktok.com/@username");
        };
        return normalize_username(username);
    }

    normalize_username(trimmed)
}

fn normalize_username(value: &str) -> Result<String> {
    let username = value.strip_prefix('@').unwrap_or(value);
    if username.len() < 2
        || username.len() > 255
        || !username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_'))
    {
        bail!("TikTok username must be 2 to 255 characters and contain only letters, numbers, dots, or underscores");
    }

    Ok(format!("@{username}"))
}

fn normalize_user_id(value: &str) -> Result<String> {
    let user_id = value.trim();
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

pub(crate) fn build_params(input: &Value, mut warn: impl FnMut(String)) -> Result<FollowingParams> {
    let (unique_id, user_id) = resolve_lookup(input, &mut warn)?;

    let count = input
        .get("count")
        .and_then(Value::as_number)
        .and_then(|number| {
            let count = number.as_f64()?;
            (count.is_finite() && count.fract() == 0.0 && count >= 1.0).then_some(count)
        });
    if input.get("count").is_some()
        && input
            .get("count")
            .and_then(Value::as_number)
            .and_then(Number::as_f64)
            .is_none_or(|count| !count.is_finite() || count.fract() != 0.0 || count < 1.0)
    {
        warn(format!(
            "count must be a positive integer, got {}. Using Scrappa default.",
            input.get("count").map(js_string).unwrap_or_default()
        ));
    }

    let time_field = if input.get("time").is_some() {
        "time"
    } else {
        "cursor"
    };
    let pagination_value = input
        .get("time")
        .filter(|value| !value.is_null())
        .or_else(|| input.get("cursor").filter(|value| !value.is_null()));

    let time = match pagination_value {
        Some(Value::Number(number)) => {
            let value = number.as_f64().unwrap_or_default();
            if value.is_finite() && value.fract() == 0.0 && value >= 0.0 {
                Some(Value::Number(number.clone()))
            } else {
                warn(format!(
                    "{time_field} must be a non-negative integer, got {}. Starting from the first page.",
                    js_string(pagination_value.unwrap())
                ));
                None
            }
        }
        Some(Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                None
            } else if value.bytes().all(|byte| byte.is_ascii_digit()) {
                Some(Value::String(value.to_owned()))
            } else {
                warn(format!(
                    "{time_field} must contain digits only. Starting from the first page."
                ));
                None
            }
        }
        Some(Value::Null) | None => None,
        Some(value) if value.as_str() == Some("") => None,
        Some(value) => {
            warn(format!(
                "{time_field} must be a non-negative integer or digit string, got {}. Starting from the first page.",
                js_typeof(value)
            ));
            None
        }
    };

    if unique_id.is_none() && user_id.is_none() {
        bail!("TikTok unique_id or user_id is required");
    }

    Ok(FollowingParams {
        unique_id,
        user_id,
        count,
        time,
    })
}

pub(crate) fn format_lookup(input: &Value) -> String {
    let mut ignored_warning = |_| {};
    match resolve_lookup(input, &mut ignored_warning) {
        Ok((Some(unique_id), _)) => unique_id,
        Ok((_, Some(user_id))) => format!("user_id:{user_id}"),
        _ => "unknown TikTok profile".to_owned(),
    }
}
