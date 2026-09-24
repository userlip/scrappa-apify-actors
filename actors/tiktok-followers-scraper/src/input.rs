use anyhow::{anyhow, bail, Result};
use serde_json::{Number, Value};
use url::Url;

pub(super) fn js_number_string(number: &Number) -> String {
    if let Some(value) = number.as_f64() {
        if value.is_finite() && value.fract() == 0.0 && value >= 0.0 && value <= u64::MAX as f64 {
            return format!("{value:.0}");
        }
    }
    number.to_string()
}

pub(super) fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => js_number_string(value),
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

pub(super) fn js_typeof(value: &Value) -> &'static str {
    match value {
        Value::Null | Value::Array(_) | Value::Object(_) => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
    }
}

pub(super) fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum TikTokLookup {
    UniqueId(String),
    UserId(String),
}

impl TikTokLookup {
    pub(super) fn log_value(&self) -> String {
        match self {
            Self::UniqueId(value) => value.clone(),
            Self::UserId(value) => format!("user_id:{value}"),
        }
    }

    pub(super) fn unique_id(&self) -> Option<&str> {
        match self {
            Self::UniqueId(value) => Some(value),
            Self::UserId(_) => None,
        }
    }

    pub(super) fn user_id(&self) -> Option<&str> {
        match self {
            Self::UserId(value) => Some(value),
            Self::UniqueId(_) => None,
        }
    }
}

#[derive(Debug)]
pub(super) struct TikTokFollowersParams {
    pub(super) lookup: TikTokLookup,
    pub(super) count: Option<String>,
    pub(super) time: Option<String>,
}

pub(super) fn normalize_tiktok_user_id(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("TikTok user_id must contain digits only");
    }
    if value.len() > 30 {
        bail!("TikTok numeric user ID must be 30 digits or fewer");
    }
    Ok(value.to_owned())
}

pub(super) fn normalize_tiktok_username(value: &str) -> Result<String> {
    let username = value.strip_prefix('@').unwrap_or(value);
    let valid = (2..=255).contains(&username.len())
        && username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'_');
    if !valid {
        bail!(
            "TikTok username must be 2 to 255 characters and contain only letters, numbers, dots, or underscores"
        );
    }
    Ok(format!("@{username}"))
}

pub(super) fn looks_like_tiktok_url(value: &str) -> bool {
    let has_scheme = value.find("://").is_some_and(|scheme_end| {
        let scheme = &value[..scheme_end];
        let mut bytes = scheme.bytes();
        bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
            && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'))
    });

    let lowercased = value.to_ascii_lowercase();
    let contains_tiktok_host = lowercased.match_indices("tiktok.com").any(|(index, _)| {
        let before = index == 0 || lowercased.as_bytes().get(index - 1) == Some(&b'.');
        let after_index = index + "tiktok.com".len();
        let after = after_index == lowercased.len()
            || lowercased.as_bytes().get(after_index) == Some(&b'/');
        before && after
    });

    has_scheme || value.starts_with("//") || contains_tiktok_host
}

pub(super) fn normalize_tiktok_unique_id(value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    if looks_like_tiktok_url(trimmed) {
        let parsed = Url::parse(trimmed);
        let Ok(parsed) = parsed else {
            bail!("A valid TikTok profile URL or username is required");
        };

        let hostname = parsed.host_str().unwrap_or_default();
        if !(hostname.eq_ignore_ascii_case("tiktok.com")
            || hostname.to_ascii_lowercase().ends_with(".tiktok.com"))
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
            .filter(|path| path.starts_with('@'))
            .filter(|path| !path[1..].is_empty() && !path[1..].contains('/'))
            .ok_or_else(|| {
                anyhow!("TikTok profile URL must use the format https://www.tiktok.com/@username")
            })?;
        return normalize_tiktok_username(username);
    }

    normalize_tiktok_username(trimmed)
}

pub(super) fn nonempty_nonnull(value: Option<&Value>) -> bool {
    value.is_some_and(|value| !value.is_null() && value != &Value::String(String::new()))
}

pub(super) fn resolve_tiktok_lookup(
    input: &Value,
    warn: &mut impl FnMut(String),
) -> Result<Option<TikTokLookup>> {
    let profile = input.get("profile");
    if let Some(Value::String(profile)) = profile {
        let profile = profile.trim();
        if !profile.is_empty() {
            if profile.bytes().all(|byte| byte.is_ascii_digit()) {
                return normalize_tiktok_user_id(profile)
                    .map(TikTokLookup::UserId)
                    .map(Some);
            }
            return normalize_tiktok_unique_id(profile)
                .map(TikTokLookup::UniqueId)
                .map(Some);
        }
    } else if nonempty_nonnull(profile) {
        warn(format!(
            "profile must be a string, got {}.",
            js_typeof(profile.unwrap())
        ));
    }

    let unique_id = input.get("unique_id");
    if let Some(Value::String(unique_id)) = unique_id {
        let unique_id = normalize_tiktok_unique_id(unique_id)?;
        if !unique_id.is_empty() {
            return Ok(Some(TikTokLookup::UniqueId(unique_id)));
        }
    } else if nonempty_nonnull(unique_id) {
        warn(format!(
            "unique_id must be a string, got {}.",
            js_typeof(unique_id.unwrap())
        ));
    }

    let user_id = input.get("user_id");
    if let Some(Value::String(user_id)) = user_id {
        let user_id = normalize_tiktok_user_id(user_id)?;
        if !user_id.is_empty() {
            return Ok(Some(TikTokLookup::UserId(user_id)));
        }
    } else if nonempty_nonnull(user_id) {
        warn(format!(
            "user_id must be a string, got {}.",
            js_typeof(user_id.unwrap())
        ));
    }

    Ok(None)
}

pub(super) fn integer_query_value(
    value: &Value,
    minimum: f64,
    maximum: Option<f64>,
) -> Option<String> {
    let number = value.as_number()?.as_f64()?;
    if !number.is_finite() || number.fract() != 0.0 || number < minimum {
        return None;
    }
    if maximum.is_some_and(|maximum| number > maximum) {
        return None;
    }
    Some(format!("{number:.0}"))
}

pub(super) fn build_tiktok_followers_params(
    input: &Value,
    warn: &mut impl FnMut(String),
) -> Result<TikTokFollowersParams> {
    let lookup = resolve_tiktok_lookup(input, warn)?;

    let count = input.get("count").and_then(|count| {
        integer_query_value(count, 1.0, Some(50.0)).or_else(|| {
            warn(format!(
                "count must be an integer between 1 and 50, got {}. Using Scrappa default.",
                js_string(count)
            ));
            None
        })
    });

    let pagination_field = if input.get("time").is_some() {
        "time"
    } else {
        "cursor"
    };
    let pagination_value = input
        .get("time")
        .filter(|value| !value.is_null())
        .or_else(|| input.get("cursor"));
    let time = match pagination_value {
        Some(Value::Number(_)) => integer_query_value(
            pagination_value.unwrap(),
            0.0,
            None,
        )
        .or_else(|| {
            warn(format!(
                "{pagination_field} must be a non-negative integer, got {}. Starting from the first page.",
                js_string(pagination_value.unwrap())
            ));
            None
        }),
        Some(Value::String(value)) if !value.trim().is_empty() => {
            let value = value.trim();
            if value.bytes().all(|byte| byte.is_ascii_digit()) {
                Some(value.to_owned())
            } else {
                warn(format!(
                    "{pagination_field} must contain digits only. Starting from the first page."
                ));
                None
            }
        }
        Some(value) if !value.is_null() && value != &Value::String(String::new()) => {
            warn(format!(
                "{pagination_field} must be a non-negative integer or digit string, got {}. Starting from the first page.",
                js_typeof(value)
            ));
            None
        }
        _ => None,
    };

    let lookup = lookup.ok_or_else(|| anyhow!("TikTok unique_id or user_id is required"))?;
    Ok(TikTokFollowersParams {
        lookup,
        count,
        time,
    })
}
