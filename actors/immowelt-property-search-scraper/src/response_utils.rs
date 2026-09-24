use crate::request_params::{js_trim, SearchParams};
use serde_json::{Map, Number, Value};

pub fn property_listings(response: &Value) -> Vec<Value> {
    let top_level = response.get("results").and_then(Value::as_array);
    let wrapped = response
        .get("data")
        .and_then(|data| data.get("results"))
        .and_then(Value::as_array);

    if let Some(results) = top_level {
        if !results.is_empty() || wrapped.is_none() {
            return results.clone();
        }
    }
    if let Some(results) = wrapped {
        return results.clone();
    }

    eprintln!(
        "Unexpected Immowelt response shape: expected \"results\" or \"data.results\" array."
    );
    Vec::new()
}

pub fn dataset_item(listing: &Value, params: &SearchParams) -> Value {
    let mut item = listing.as_object().cloned().unwrap_or_default();
    for (key, source) in [
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
        insert_value(&mut item, key, non_null_value(listing, source));
    }
    insert_value(&mut item, "latitude", non_null_value(listing, "lat"));
    insert_value(&mut item, "longitude", non_null_value(listing, "lon"));
    for (key, value) in params.output_metadata() {
        insert_value(&mut item, key, value);
    }
    Value::Object(item)
}

pub fn limited_response(response: &Value, limit: usize) -> Value {
    let mut response = response.clone();
    let Some(object) = response.as_object_mut() else {
        return response;
    };
    if let Some(results) = object.get_mut("results").and_then(Value::as_array_mut) {
        results.truncate(limit);
    }
    if let Some(data) = object.get_mut("data").and_then(Value::as_object_mut) {
        if let Some(results) = data.get_mut("results").and_then(Value::as_array_mut) {
            results.truncate(limit);
        }
    }
    response
}

pub fn pagination_number(response: &Value, field: &str) -> Option<Value> {
    let wrapped = response.get("data").and_then(|data| data.get(field));
    parse_number(response.get(field)).or_else(|| parse_number(wrapped))
}

fn non_null_value(object: &Value, field: &str) -> Value {
    object
        .get(field)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn insert_value(object: &mut Map<String, Value>, key: &str, value: Value) {
    object.insert(key.to_owned(), value);
}

fn parse_number(value: Option<&Value>) -> Option<Value> {
    match value? {
        Value::Number(number) => number
            .as_f64()
            .filter(|number| number.is_finite())
            .and_then(json_number),
        Value::String(value) => {
            let value = js_trim(value);
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            if let Ok(number) = value.parse::<u64>() {
                return Some(Value::Number(Number::from(number)));
            }
            value
                .parse::<f64>()
                .ok()
                .filter(|number| number.is_finite())
                .and_then(json_number)
        }
        _ => None,
    }
}

fn json_number(value: f64) -> Option<Value> {
    if value.fract() == 0.0 && value >= 0.0 && value < u64::MAX as f64 {
        return Some(Value::Number(Number::from(value as u64)));
    }
    Number::from_f64(value).map(Value::Number)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_params::build_search_params;
    use serde_json::json;

    fn params() -> SearchParams {
        build_search_params(Some(&json!({
            "location": "Berlin",
            "type": "apartment-rent",
            "page": 1,
            "per_page": 20
        })))
        .unwrap()
    }

    #[test]
    fn reads_top_level_wrapped_and_empty_fallback_results() {
        let rows = vec![json!({"title": "Berlin flat"})];
        assert_eq!(property_listings(&json!({"results": rows.clone()})), rows);
        assert_eq!(
            property_listings(&json!({"data": {"results": rows.clone()}})),
            rows
        );
        assert_eq!(
            property_listings(&json!({"results": [], "data": {"results": rows.clone()}})),
            rows
        );
        assert!(property_listings(&json!({"results": []})).is_empty());
    }

    #[test]
    fn enriches_rows_without_mutating_source_listing() {
        let listing = json!({
            "id": "estate_123",
            "online_id": "2paau5t",
            "title": "Modern flat",
            "price": 2170.82,
            "price_formatted": "2.171 EUR (Kaltmiete)",
            "rooms": 3.5,
            "rooms_max": 3.5,
            "size_m2": 113.81,
            "size_m2_max": 113.81,
            "address": "Mitte, 10117, Berlin",
            "lat": 52.511009,
            "lon": 13.402116,
            "image_url": "https://example.test/image.jpg",
            "url": "https://www.immowelt.de/expose/2paau5t",
            "is_private": false,
            "published": "2026-05-11T19:02:59.797Z"
        });
        let original = listing.clone();
        let item = dataset_item(&listing, &params());
        assert_eq!(listing, original);
        assert_eq!(item["latitude"], 52.511009);
        assert_eq!(item["longitude"], 13.402116);
        assert_eq!(item["request_location"], "Berlin");
        assert_eq!(item["request_type"], "apartment-rent");
        assert_eq!(item["request_page"], 1);
        assert_eq!(item["request_per_page"], 20);
        assert_eq!(item["lat"], 52.511009);
        assert_eq!(item["lon"], 13.402116);
    }

    #[test]
    fn nulls_missing_and_null_fields_in_normalized_columns() {
        let item = dataset_item(&json!({"title": null, "lat": null}), &params());
        assert_eq!(item["title"], Value::Null);
        assert_eq!(item["latitude"], Value::Null);
        assert_eq!(item["id"], Value::Null);
        assert_eq!(item["longitude"], Value::Null);
    }

    #[test]
    fn limits_top_level_and_wrapped_output_results() {
        let response = json!({
            "results": [{"id": 1}, {"id": 2}],
            "data": {"results": [{"id": 3}, {"id": 4}]},
            "total_results": 4,
            "total_pages": 2
        });
        let original = response.clone();
        let limited = limited_response(&response, 1);
        assert_eq!(limited["results"], json!([{"id": 1}]));
        assert_eq!(limited["data"]["results"], json!([{"id": 3}]));
        assert_eq!(limited["total_results"], 4);
        assert_eq!(response, original);
    }

    #[test]
    fn reads_numeric_pagination_metadata_from_both_response_shapes() {
        assert_eq!(
            pagination_number(&json!({"total_results": 5473}), "total_results"),
            Some(json!(5473))
        );
        assert_eq!(
            pagination_number(&json!({"data": {"page": "3"}}), "page"),
            Some(json!(3))
        );
        assert_eq!(
            pagination_number(&json!({"page": 3.0}), "page"),
            Some(json!(3))
        );
        assert_eq!(
            pagination_number(&json!({"page": " \u{feff}3 "}), "page"),
            Some(json!(3))
        );
        assert_eq!(pagination_number(&json!({"page": "-3"}), "page"), None);
    }
}
