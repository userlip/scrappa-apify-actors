use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use url::Url;

pub(crate) const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

#[derive(Debug, PartialEq)]
pub(crate) struct TikTokHashtagPostsParams {
    pub(crate) challenge_name: Option<String>,
    pub(crate) challenge_id: Option<String>,
    pub(crate) region: Option<String>,
    pub(crate) count: Option<i64>,
    pub(crate) cursor: Option<String>,
    pub(crate) lookup_label: String,
}

impl TikTokHashtagPostsParams {
    pub(crate) fn append_to_url(&self, url: &mut Url) {
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

    pub(crate) fn metadata(&self) -> [(&'static str, Value); 3] {
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

pub(crate) fn js_trim(value: &str) -> &str {
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

pub(crate) fn js_string(value: &Value) -> String {
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

pub(crate) fn build_hashtag_posts_params(input: &Value) -> Result<TikTokHashtagPostsParams> {
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
