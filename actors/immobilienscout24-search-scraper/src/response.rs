use crate::input::SearchParams;
use serde_json::{Number, Value};

pub(crate) fn get_listings(response: &Value) -> Vec<Value> {
    let top_level = response.get("results").and_then(Value::as_array);
    let wrapped = response.pointer("/data/results").and_then(Value::as_array);
    if let Some(results) = top_level {
        if !results.is_empty() || wrapped.is_none() {
            return results.clone();
        }
    }
    if let Some(results) = wrapped {
        return results.clone();
    }
    eprintln!(
        "Unexpected ImmobilienScout24 response shape: expected \"results\" or \"data.results\" array."
    );
    Vec::new()
}

fn get_number(value: Option<&Value>) -> Option<Number> {
    let value = value?;
    if let Some(number) = value.as_number() {
        return number
            .as_f64()
            .filter(|number| number.is_finite())
            .map(|_| number.clone());
    }
    let text = value.as_str()?.trim();
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u64>()
        .ok()
        .map(Number::from)
        .or_else(|| text.parse::<f64>().ok().and_then(Number::from_f64))
}

pub(crate) fn get_total_results(response: &Value) -> Option<Number> {
    get_number(response.get("total_results"))
        .or_else(|| get_number(response.pointer("/data/total_results")))
}

pub(crate) fn get_response_page(response: &Value) -> Option<Number> {
    get_number(response.get("page")).or_else(|| get_number(response.pointer("/data/page")))
}

pub(crate) fn get_total_pages(response: &Value) -> Option<Number> {
    get_number(response.get("total_pages"))
        .or_else(|| get_number(response.pointer("/data/total_pages")))
}

pub(crate) fn dataset_item(listing: &Value, params: &SearchParams) -> Value {
    let mut item = listing.as_object().cloned().unwrap_or_default();
    for (field, source) in [
        ("id", "id"),
        ("online_id", "online_id"),
        ("title", "title"),
        ("price", "price"),
        ("price_formatted", "price_formatted"),
        ("rooms", "rooms"),
        ("rooms_max", "rooms_max"),
        ("size_m2", "size_m2"),
        ("size_m2_max", "size_m2_max"),
        ("address", "address"),
        ("image_url", "image_url"),
        ("url", "url"),
        ("is_private", "is_private"),
        ("published", "published"),
    ] {
        item.insert(
            field.to_owned(),
            listing.get(source).cloned().unwrap_or(Value::Null),
        );
    }
    item.insert(
        "latitude".to_owned(),
        listing.get("lat").cloned().unwrap_or(Value::Null),
    );
    item.insert(
        "longitude".to_owned(),
        listing.get("lon").cloned().unwrap_or(Value::Null),
    );
    let params = params.as_value();
    for field in [
        "location",
        "type",
        "price_min",
        "price_max",
        "rooms_min",
        "rooms_max",
        "size_min",
        "size_max",
        "page",
        "per_page",
    ] {
        item.insert(
            format!("request_{field}"),
            params.get(field).cloned().unwrap_or(Value::Null),
        );
    }
    Value::Object(item)
}

pub(crate) fn limited_response(response: &Value, limit: usize) -> Value {
    let mut limited = response.clone();
    if let Some(results) = response.get("results").and_then(Value::as_array) {
        if let Some(object) = limited.as_object_mut() {
            object.insert(
                "results".to_owned(),
                Value::Array(results.iter().take(limit).cloned().collect()),
            );
        }
    }
    if let Some(results) = response.pointer("/data/results").and_then(Value::as_array) {
        if let Some(data) = limited.get_mut("data").and_then(Value::as_object_mut) {
            data.insert(
                "results".to_owned(),
                Value::Array(results.iter().take(limit).cloned().collect()),
            );
        }
    }
    limited
}
