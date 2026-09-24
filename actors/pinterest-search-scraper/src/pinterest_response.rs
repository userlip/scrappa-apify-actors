use anyhow::{bail, Result};
use serde_json::{json, Map, Value};

use crate::pinterest_input::PinterestSearchParams;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PinterestPinsSource {
    Pins,
    DataPins,
    Results,
    DataResults,
}

#[derive(Debug)]
pub(crate) struct PinterestPinsSelection<'a> {
    pub(crate) pins: &'a [Value],
    pub(crate) source: Option<PinterestPinsSource>,
}

pub(crate) fn select_pinterest_pins(response: &Value) -> PinterestPinsSelection<'_> {
    if let Some(pins) = response.get("pins").and_then(Value::as_array) {
        return PinterestPinsSelection {
            pins,
            source: Some(PinterestPinsSource::Pins),
        };
    }
    if let Some(pins) = response.pointer("/data/pins").and_then(Value::as_array) {
        return PinterestPinsSelection {
            pins,
            source: Some(PinterestPinsSource::DataPins),
        };
    }
    if let Some(pins) = response.get("results").and_then(Value::as_array) {
        return PinterestPinsSelection {
            pins,
            source: Some(PinterestPinsSource::Results),
        };
    }
    if let Some(pins) = response.pointer("/data/results").and_then(Value::as_array) {
        return PinterestPinsSelection {
            pins,
            source: Some(PinterestPinsSource::DataResults),
        };
    }
    PinterestPinsSelection {
        pins: &[],
        source: None,
    }
}

pub(crate) fn pinterest_next_bookmark(response: &Value) -> Value {
    response
        .get("nextBookmark")
        .filter(|bookmark| !bookmark.is_null())
        .or_else(|| {
            response
                .get("bookmark")
                .filter(|bookmark| !bookmark.is_null())
        })
        .cloned()
        .unwrap_or(Value::Null)
}

pub(crate) fn limit_pinterest_search_response(
    response: &Value,
    limit: usize,
    selected_source: Option<PinterestPinsSource>,
) -> Value {
    let Some(response) = response.as_object() else {
        return json!({});
    };
    let source =
        selected_source.or_else(|| select_pinterest_pins(&Value::Object(response.clone())).source);
    let mut limited = response.clone();

    if source == Some(PinterestPinsSource::Pins) {
        if let Some(pins) = response.get("pins").and_then(Value::as_array) {
            limited.insert(
                "pins".to_owned(),
                json!(pins.iter().take(limit).cloned().collect::<Vec<_>>()),
            );
        }
    } else {
        limited.remove("pins");
    }
    if source == Some(PinterestPinsSource::Results) {
        if let Some(results) = response.get("results").and_then(Value::as_array) {
            limited.insert(
                "results".to_owned(),
                json!(results.iter().take(limit).cloned().collect::<Vec<_>>()),
            );
        }
    } else {
        limited.remove("results");
    }

    if let Some(data) = response.get("data").and_then(Value::as_object) {
        let mut limited_data = data.clone();
        if source == Some(PinterestPinsSource::DataPins) {
            if let Some(pins) = data.get("pins").and_then(Value::as_array) {
                limited_data.insert(
                    "pins".to_owned(),
                    json!(pins.iter().take(limit).cloned().collect::<Vec<_>>()),
                );
            }
        } else {
            limited_data.remove("pins");
        }
        if source == Some(PinterestPinsSource::DataResults) {
            if let Some(results) = data.get("results").and_then(Value::as_array) {
                limited_data.insert(
                    "results".to_owned(),
                    json!(results.iter().take(limit).cloned().collect::<Vec<_>>()),
                );
            }
        } else {
            limited_data.remove("results");
        }
        limited.insert("data".to_owned(), Value::Object(limited_data));
    }
    Value::Object(limited)
}

fn first_image_url(pin: &Map<String, Value>) -> Option<Value> {
    for field in ["image_url", "image"] {
        if let Some(Value::String(value)) = pin.get(field) {
            if !value.is_empty() {
                return Some(Value::String(value.clone()));
            }
        }
    }

    if let Some(images) = pin.get("images") {
        if let Some(images) = images.as_array() {
            for image in images {
                if let Some(image) = image.as_str() {
                    return Some(json!(image));
                }
                if let Some(url) = image.get("url").and_then(Value::as_str) {
                    return Some(json!(url));
                }
            }
        }
        if let Some(images) = images.as_object() {
            for key in ["orig", "original", "736x", "564x", "236x"] {
                let Some(value) = images.get(key) else {
                    continue;
                };
                if let Some(value) = value.as_str().filter(|value| !value.is_empty()) {
                    return Some(json!(value));
                }
                if let Some(url) = value.get("url").and_then(Value::as_str) {
                    return Some(json!(url));
                }
            }
        }
    }
    None
}

fn object_value<'a>(value: Option<&'a Value>, keys: &[&str]) -> Option<&'a Value> {
    let object = value?.as_object()?;
    keys.iter().find_map(|key| {
        object
            .get(*key)
            .filter(|value| !value.is_null() && value.as_str() != Some(""))
    })
}

fn javascript_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Number(value)) => value.as_f64().is_some_and(|value| value != 0.0),
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Array(_)) | Some(Value::Object(_)) | Some(Value::Bool(true)) => true,
    }
}

pub(crate) fn nullish(value: Option<&Value>) -> Value {
    value
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

pub(crate) fn pinterest_dataset_item(
    pin: &Value,
    params: &PinterestSearchParams,
    response: &Value,
) -> Result<Value> {
    let Some(pin) = pin.as_object() else {
        bail!("Pinterest pin result must be an object");
    };
    let mut item = pin.clone();
    let image_url = first_image_url(pin).unwrap_or(Value::Null);
    let pinner_id = object_value(pin.get("pinner"), &["id", "user_id"])
        .cloned()
        .unwrap_or(Value::Null);
    let pinner_username = object_value(pin.get("pinner"), &["username", "userName", "name"])
        .cloned()
        .unwrap_or(Value::Null);
    let board_id = object_value(pin.get("board"), &["id", "board_id"])
        .cloned()
        .unwrap_or(Value::Null);
    let board_name = object_value(pin.get("board"), &["name", "title"])
        .cloned()
        .unwrap_or(Value::Null);
    let response_pins_length = select_pinterest_pins(response).pins.len();
    let results_count = response
        .get("results_count")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or_else(|| json!(response_pins_length));

    let link = pin
        .get("link")
        .filter(|value| !value.is_null())
        .or_else(|| pin.get("url").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null);
    item.insert("id".to_owned(), nullish(pin.get("id")));
    item.insert("title".to_owned(), nullish(pin.get("title")));
    item.insert("description".to_owned(), nullish(pin.get("description")));
    item.insert("image_url".to_owned(), image_url);
    item.insert("link".to_owned(), link);
    item.insert("domain".to_owned(), nullish(pin.get("domain")));
    item.insert("pinner_id".to_owned(), pinner_id);
    item.insert("pinner_username".to_owned(), pinner_username);
    item.insert("board_id".to_owned(), board_id);
    item.insert("board_name".to_owned(), board_name);
    item.insert(
        "has_video".to_owned(),
        json!(javascript_truthy(pin.get("video"))),
    );
    item.insert("repin_count".to_owned(), nullish(pin.get("repin_count")));
    item.insert(
        "comment_count".to_owned(),
        nullish(pin.get("comment_count")),
    );
    item.insert("like_count".to_owned(), nullish(pin.get("like_count")));
    item.insert("save_count".to_owned(), nullish(pin.get("save_count")));
    item.insert("request_query".to_owned(), json!(params.query));
    item.insert("request_limit".to_owned(), json!(params.limit));
    item.insert(
        "request_bookmark".to_owned(),
        params
            .bookmark
            .as_ref()
            .map_or(Value::Null, |value| json!(value)),
    );
    item.insert("count".to_owned(), nullish(response.get("count")));
    item.insert("results_count".to_owned(), results_count);
    item.insert("nextBookmark".to_owned(), pinterest_next_bookmark(response));
    Ok(Value::Object(item))
}
