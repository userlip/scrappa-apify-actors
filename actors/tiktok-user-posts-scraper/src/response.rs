use anyhow::{bail, Result};
use serde_json::{Map, Value};

use crate::params::TikTokUserPostsParams;

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

pub(crate) fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

pub(crate) fn extract_posts(data: Option<&Value>) -> Vec<&Value> {
    let Some(data) = data.filter(|data| js_truthy(data)) else {
        return Vec::new();
    };
    if let Some(posts) = data.as_array() {
        return posts.iter().collect();
    }
    ["posts", "videos", "aweme_list"]
        .iter()
        .find_map(|key| data.get(key).and_then(Value::as_array))
        .map(|posts| posts.iter().collect())
        .unwrap_or_default()
}

pub(crate) fn extract_pagination(data: Option<&Value>) -> (bool, Value) {
    let Some(data) = data
        .filter(|data| js_truthy(data))
        .filter(|data| !data.is_array())
    else {
        return (false, Value::Null);
    };
    let has_more = data
        .get("hasMore")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("has_more").filter(|value| !value.is_null()))
        .is_some_and(js_truthy);
    let cursor = data
        .get("cursor")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("max_cursor").filter(|value| !value.is_null()))
        .or_else(|| data.get("min_cursor").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null);
    (has_more, cursor)
}

pub(crate) fn enrich_post(post: &Value, params: &TikTokUserPostsParams) -> Value {
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
            .map(|(index, character)| (index.to_string(), Value::String(character.to_string())))
            .collect(),
        _ => Map::new(),
    };
    row.insert(
        "lookup_unique_id".to_owned(),
        params
            .unique_id
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    row.insert(
        "lookup_user_id".to_owned(),
        params
            .user_id
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    Value::Object(row)
}

pub(crate) fn validate_scrappa_code(response: &Value) -> Result<()> {
    let Some(code) = response.get("code") else {
        return Ok(());
    };
    if code.as_f64() == Some(0.0) {
        return Ok(());
    }
    let message = response
        .get("msg")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or_else(|| "Unknown error".to_owned());
    bail!(
        "Scrappa TikTok User Posts API returned code {}: {message}",
        js_string(code)
    );
}
