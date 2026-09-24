use std::collections::BTreeSet;

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Map, Value};
use url::Url;

use crate::config::{endpoint_url, MAX_ENTITIES};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RequestType {
    ChallengeName,
    ChallengeId,
}

impl RequestType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ChallengeName => "challenge_name",
            Self::ChallengeId => "challenge_id",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChallengeRequest {
    pub(crate) request_type: RequestType,
    pub(crate) value: String,
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

fn normalize_challenge_name(value: &str) -> Result<String> {
    let value = js_trim(value);
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.is_empty() {
        return Ok(String::new());
    }
    if value.chars().count() > 255
        || value.chars().any(|character| {
            is_js_whitespace(character) || matches!(character, '#' | '?' | '/' | '=' | ':')
        })
    {
        bail!("TikTok challenge names must be 1 to 255 characters and cannot contain whitespace or URL delimiter characters");
    }
    Ok(value.to_owned())
}

fn safe_integer(value: &Value) -> Option<i64> {
    let number = value.as_f64()?;
    if !number.is_finite() || number.fract() != 0.0 || number.abs() > 9_007_199_254_740_991.0 {
        return None;
    }
    Some(number as i64)
}

fn normalize_challenge_id(value: &Value) -> Result<String> {
    let id = match value {
        Value::String(value) => js_trim(value).to_owned(),
        Value::Number(_) => safe_integer(value)
            .map(|value| value.to_string())
            .ok_or_else(|| {
                anyhow!("TikTok challenge IDs must be strings of digits or safe integers")
            })?,
        _ => bail!("TikTok challenge IDs must be strings of digits or safe integers"),
    };
    if id.is_empty() {
        return Ok(id);
    }
    if id.len() > 100 || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("TikTok challenge IDs must contain 1 to 100 digits");
    }
    Ok(id)
}

fn input_values<'a>(
    input: &'a Value,
    field: &str,
    plural: bool,
    warnings: &mut Vec<String>,
) -> Vec<&'a Value> {
    let Some(value) = input.get(field).filter(|value| !value.is_null()) else {
        return Vec::new();
    };
    if let Some(values) = value.as_array() {
        return values.iter().collect();
    }
    if plural {
        warnings.push(format!(
            "{field} must be an array. Treating the supplied value as one lookup for API compatibility."
        ));
    }
    vec![value]
}

pub(crate) fn build_requests(
    input: &Value,
    warnings: &mut Vec<String>,
) -> Result<Vec<ChallengeRequest>> {
    let mut names = Vec::new();
    for (field, plural) in [("challenge_names", true), ("challenge_name", false)] {
        for value in input_values(input, field, plural, warnings) {
            match value
                .as_str()
                .ok_or_else(|| anyhow!("TikTok challenge names must be strings"))
                .and_then(normalize_challenge_name)
            {
                Ok(name) if !name.is_empty() => names.push(name),
                Ok(_) => {}
                Err(error) => warnings.push(format!("{field} entry omitted: {error}")),
            }
        }
    }

    let mut ids = Vec::new();
    for (field, plural) in [("challenge_ids", true), ("challenge_id", false)] {
        for value in input_values(input, field, plural, warnings) {
            match normalize_challenge_id(value) {
                Ok(id) if !id.is_empty() => ids.push(id),
                Ok(_) => {}
                Err(error) => warnings.push(format!("{field} entry omitted: {error}")),
            }
        }
    }

    let mut seen_names = BTreeSet::new();
    names.retain(|name| seen_names.insert(name.to_lowercase()));
    let mut seen_ids = BTreeSet::new();
    ids.retain(|id| seen_ids.insert(id.clone()));

    if names.len() + ids.len() == 0 {
        bail!("At least one valid TikTok challenge name or challenge ID is required");
    }
    if names.len() + ids.len() > MAX_ENTITIES {
        bail!("A maximum of {MAX_ENTITIES} combined TikTok challenge names and IDs is allowed per run");
    }

    Ok(names
        .into_iter()
        .map(|value| ChallengeRequest {
            request_type: RequestType::ChallengeName,
            value,
        })
        .chain(ids.into_iter().map(|value| ChallengeRequest {
            request_type: RequestType::ChallengeId,
            value,
        }))
        .collect())
}

pub(crate) fn challenge_url(base_url: &Url, request: &ChallengeRequest) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["tiktok", "challenges", "details"])?;
    let key = match request.request_type {
        RequestType::ChallengeName => "challenge_name",
        RequestType::ChallengeId => "challenge_id",
    };
    url.query_pairs_mut().append_pair(key, &request.value);
    Ok(url)
}

fn first_non_null<'a>(object: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter()
        .find_map(|key| object.get(*key).filter(|value| !value.is_null()))
}

fn text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) => {
            let value = js_trim(value);
            (!value.is_empty()).then(|| value.to_owned())
        }
        value @ Value::Number(_) => safe_integer(value).map(|value| value.to_string()),
        _ => None,
    }
}

pub(crate) fn challenge_name(challenge: &Map<String, Value>) -> Option<String> {
    text(first_non_null(
        challenge,
        &["challenge_name", "cha_name", "name", "title"],
    ))
}

pub(crate) fn challenge_id(challenge: &Map<String, Value>) -> Option<String> {
    text(first_non_null(challenge, &["challenge_id", "id", "cid"]))
}

pub(crate) fn extract_challenge_detail(response: &Value) -> Option<&Map<String, Value>> {
    let data = response.get("data")?.as_object()?;
    for key in ["challenge", "item"] {
        if let Some(challenge) = data.get(key).and_then(Value::as_object) {
            return Some(challenge);
        }
    }
    Some(data)
}

pub(crate) fn normalize_challenge_detail(
    challenge: &Map<String, Value>,
    request: &ChallengeRequest,
    retrieved_at: String,
) -> Value {
    let mut item = challenge.clone();
    let name = challenge_name(challenge);
    let id = challenge_id(challenge);
    let description = text(first_non_null(challenge, &["description", "desc"]));
    let cover = text(first_non_null(challenge, &["cover", "cover_url"]));
    let stats = challenge.get("stats").and_then(Value::as_object);
    let metric = |field: &str| {
        challenge
            .get(field)
            .filter(|value| !value.is_null())
            .or_else(|| stats.and_then(|stats| stats.get(field)))
            .filter(|value| value.is_number())
            .cloned()
            .unwrap_or(Value::Null)
    };

    item.insert(
        "challenge_id".to_owned(),
        id.map(Value::String).unwrap_or(Value::Null),
    );
    item.insert(
        "challenge_name".to_owned(),
        name.map(Value::String).unwrap_or(Value::Null),
    );
    item.insert(
        "description".to_owned(),
        description.map(Value::String).unwrap_or(Value::Null),
    );
    item.insert("user_count".to_owned(), metric("user_count"));
    item.insert("view_count".to_owned(), metric("view_count"));
    item.insert("video_count".to_owned(), metric("video_count"));
    item.insert(
        "cover".to_owned(),
        cover.map(Value::String).unwrap_or(Value::Null),
    );
    item.insert(
        "request_challenge_name".to_owned(),
        if request.request_type == RequestType::ChallengeName {
            json!(request.value)
        } else {
            Value::Null
        },
    );
    item.insert(
        "request_challenge_id".to_owned(),
        if request.request_type == RequestType::ChallengeId {
            json!(request.value)
        } else {
            Value::Null
        },
    );
    item.insert("retrieved_at".to_owned(), Value::String(retrieved_at));
    Value::Object(item)
}
