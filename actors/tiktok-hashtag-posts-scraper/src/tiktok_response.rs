use crate::input::{js_trim, TikTokHashtagPostsParams, MAX_SAFE_INTEGER};
use serde_json::{Map, Value};

fn optional_js_number_string(value: &Value) -> Option<String> {
    let number = value.as_f64()?;
    if !number.is_finite() || number.fract() != 0.0 || number.abs() > MAX_SAFE_INTEGER {
        return None;
    }
    Some((number as i64).to_string())
}

pub(crate) fn get_challenge_id(challenge: &Value) -> Option<String> {
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

pub(crate) fn get_challenge_name(challenge: &Value) -> String {
    challenge
        .get("challenge_name")
        .filter(|value| !value.is_null())
        .or_else(|| challenge.get("cha_name"))
        .and_then(Value::as_str)
        .map(js_trim)
        .unwrap_or_default()
        .to_owned()
}

pub(crate) fn extract_challenges(data: Option<&Value>) -> Vec<Value> {
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

pub(crate) fn select_challenge_for_hashtag<'a>(
    challenges: &'a [Value],
    hashtag: &str,
) -> Option<&'a Value> {
    let normalized_target = normalize_challenge_name(hashtag);
    challenges.iter().find(|challenge| {
        normalize_challenge_name(&get_challenge_name(challenge)) == normalized_target
    })
}

pub(crate) fn extract_posts(data: Option<&Value>) -> Vec<&Value> {
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

pub(crate) fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_none_or(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

pub(crate) fn extract_pagination(data: Option<&Value>) -> (bool, Value) {
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

pub(crate) fn enrich_post(
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
