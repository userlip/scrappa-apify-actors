use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub videos: Vec<Map<String, Value>>,
    pub cursor: Option<String>,
    pub has_more: bool,
}

pub fn parse_page(data: Option<&Value>) -> Page {
    let body = data.and_then(Value::as_object);
    let values = data
        .and_then(Value::as_array)
        .or_else(|| body.and_then(|body| body.get("posts").and_then(Value::as_array)))
        .or_else(|| body.and_then(|body| body.get("videos").and_then(Value::as_array)))
        .or_else(|| body.and_then(|body| body.get("aweme_list").and_then(Value::as_array)));

    let cursor_value = nullish_field(body, &["cursor", "max_cursor", "min_cursor"]);
    let cursor = match cursor_value {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Number(value)) => Some(js_number_string(value)),
        _ => None,
    };
    let has_more_value = nullish_field(body, &["hasMore", "has_more"]);
    let has_more = matches!(has_more_value, Some(Value::Bool(true)))
        || matches!(has_more_value, Some(Value::Number(value)) if value.as_f64() == Some(1.0))
        || matches!(has_more_value, Some(Value::String(value)) if value == "1");

    let videos = values
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .cloned()
        .collect();

    Page {
        videos,
        cursor,
        has_more,
    }
}

pub fn get_video_id(video: &Map<String, Value>) -> Option<String> {
    ["video_id", "aweme_id", "id"]
        .into_iter()
        .find_map(|field| {
            let value = video.get(field)?;
            match value {
                Value::String(value) if !value.is_empty() => Some(value.clone()),
                Value::Number(value) => {
                    let number = value.as_f64()?;
                    (number.is_finite()
                        && number.fract() == 0.0
                        && number.abs() <= 9_007_199_254_740_991.0)
                        .then(|| format!("{number:.0}"))
                }
                _ => None,
            }
        })
}

pub fn js_string(value: &Value) -> String {
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

fn js_number_string(value: &serde_json::Number) -> String {
    let Some(number) = value.as_f64() else {
        return value.to_string();
    };
    if number.is_finite() && number.fract() == 0.0 && number.abs() <= 9_007_199_254_740_991.0 {
        format!("{number:.0}")
    } else {
        value.to_string()
    }
}

fn nullish_field<'a>(body: Option<&'a Map<String, Value>>, names: &[&str]) -> Option<&'a Value> {
    names
        .iter()
        .filter_map(|name| body.and_then(|body| body.get(*name)))
        .find(|value| !value.is_null())
}
