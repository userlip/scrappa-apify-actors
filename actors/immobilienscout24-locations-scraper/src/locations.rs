use std::collections::HashSet;

use serde_json::{Map, Value};

pub fn get_locations(response: &Value) -> &[Value] {
    response
        .get("locations")
        .and_then(Value::as_array)
        .or_else(|| {
            response
                .pointer("/data/locations")
                .and_then(Value::as_array)
        })
        .map(Vec::as_slice)
        .unwrap_or_default()
}

pub fn build_unique_location_items(
    response: &Value,
    source_query: &str,
    seen_geocodes: &mut HashSet<String>,
) -> Vec<Value> {
    get_locations(response)
        .iter()
        .filter_map(|location| {
            let object = location.as_object()?;
            let geocode = non_empty_string(object.get("geocode"))?;
            non_empty_string(object.get("name"))?;
            non_empty_string(object.get("type"))?;
            if !seen_geocodes.insert(geocode.to_owned()) {
                return None;
            }
            Some(with_source_query(object, source_query))
        })
        .collect()
}

fn non_empty_string(value: Option<&Value>) -> Option<&str> {
    value?.as_str().filter(|value| !value.is_empty())
}

fn with_source_query(location: &Map<String, Value>, source_query: &str) -> Value {
    let mut item = location.clone();
    item.insert(
        "source_query".to_owned(),
        Value::String(source_query.to_owned()),
    );
    Value::Object(item)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::json;

    use super::{build_unique_location_items, get_locations};

    #[test]
    fn reads_top_level_or_wrapped_location_arrays() {
        let locations = vec![json!({ "geocode": "1", "name": "Berlin", "type": "city" })];
        assert_eq!(
            get_locations(&json!({ "locations": locations })),
            &locations
        );
        assert_eq!(
            get_locations(&json!({ "data": { "locations": locations } })),
            &locations
        );
        assert!(get_locations(&json!({ "success": false })).is_empty());
    }

    #[test]
    fn preserves_extra_fields_and_deduplicates_in_first_query_order() {
        let mut seen = HashSet::new();
        let berlin = build_unique_location_items(
            &json!({ "locations": [
                { "geocode": "1", "name": "Berlin", "type": "city", "source_query": "old", "extra": true },
                { "geocode": "2", "name": "Mitte", "type": "district" }
            ] }),
            "Berlin",
            &mut seen,
        );
        let mitte = build_unique_location_items(
            &json!({ "locations": [
                { "geocode": "2", "name": "Mitte", "type": "district" },
                { "geocode": "3", "name": "Wedding", "type": "district" }
            ] }),
            "Mitte",
            &mut seen,
        );

        assert_eq!(
            berlin,
            vec![
                json!({ "geocode": "1", "name": "Berlin", "type": "city", "source_query": "Berlin", "extra": true }),
                json!({ "geocode": "2", "name": "Mitte", "type": "district", "source_query": "Berlin" }),
            ]
        );
        assert_eq!(
            mitte,
            vec![
                json!({ "geocode": "3", "name": "Wedding", "type": "district", "source_query": "Mitte" })
            ]
        );
    }

    #[test]
    fn drops_malformed_location_rows() {
        let mut seen = HashSet::new();
        let items = build_unique_location_items(
            &json!({ "locations": [
                null,
                { "geocode": "", "name": "Berlin", "type": "city" },
                { "geocode": "1", "name": "Berlin" },
                { "geocode": "2", "name": "Berlin", "type": "city" }
            ] }),
            "Berlin",
            &mut seen,
        );
        assert_eq!(
            items,
            vec![
                json!({ "geocode": "2", "name": "Berlin", "type": "city", "source_query": "Berlin" })
            ]
        );
    }
}
