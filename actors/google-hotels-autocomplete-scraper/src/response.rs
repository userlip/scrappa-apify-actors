use serde_json::{Map, Value};
use std::collections::HashSet;

const STRING_FIELDS: [&str; 4] = ["type", "value", "autocomplete_suggestion", "property_token"];

fn is_suggestion(value: &Value) -> bool {
    let Some(suggestion) = value.as_object() else {
        return false;
    };
    STRING_FIELDS.iter().all(|field| {
        suggestion
            .get(*field)
            .is_none_or(|value| value.as_str().is_some())
    })
}

fn identity(suggestion: &Map<String, Value>) -> (String, String) {
    let suggestion_type = suggestion
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let value = ["property_token", "value", "autocomplete_suggestion"]
        .iter()
        .find_map(|field| suggestion.get(*field).filter(|value| !value.is_null()))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    (suggestion_type, value)
}

fn nullable(suggestion: &Map<String, Value>, field: &str) -> Value {
    suggestion
        .get(field)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn build_dataset_items(
    response: &Value,
    source_query: &str,
    common_params: &[(String, String)],
) -> Vec<Value> {
    let Some(suggestions) = response.get("suggestions").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    let mut items = Vec::new();

    for suggestion in suggestions {
        if !is_suggestion(suggestion) {
            continue;
        }
        let Some(suggestion_object) = suggestion.as_object() else {
            continue;
        };
        let identity = identity(suggestion_object);
        if identity.1.is_empty() || !seen.insert(identity) {
            continue;
        }

        let mut item = suggestion_object.clone();
        item.insert(
            "position".to_owned(),
            nullable(suggestion_object, "position"),
        );
        let value = suggestion_object
            .get("value")
            .filter(|value| !value.is_null())
            .or_else(|| {
                suggestion_object
                    .get("autocomplete_suggestion")
                    .filter(|value| !value.is_null())
            })
            .cloned()
            .unwrap_or(Value::Null);
        item.insert("value".to_owned(), value);
        item.insert("type".to_owned(), nullable(suggestion_object, "type"));
        item.insert(
            "autocomplete_suggestion".to_owned(),
            nullable(suggestion_object, "autocomplete_suggestion"),
        );
        item.insert(
            "highlighted_words".to_owned(),
            suggestion_object
                .get("highlighted_words")
                .filter(|words| words.is_array())
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new())),
        );
        item.insert(
            "property_token".to_owned(),
            nullable(suggestion_object, "property_token"),
        );
        item.insert(
            "thumbnail".to_owned(),
            nullable(suggestion_object, "thumbnail"),
        );
        item.insert(
            "scrappa_google_hotels_link".to_owned(),
            nullable(suggestion_object, "scrappa_google_hotels_link"),
        );
        item.insert(
            "source_query".to_owned(),
            Value::String(source_query.to_owned()),
        );
        for (field, parameter) in [
            ("request_gl", "gl"),
            ("request_hl", "hl"),
            ("request_currency", "currency"),
            ("request_type", "type"),
        ] {
            let value = common_params
                .iter()
                .find_map(|(key, value)| (key == parameter).then_some(value))
                .cloned()
                .map(Value::String)
                .unwrap_or(Value::Null);
            item.insert(field.to_owned(), value);
        }
        item.insert(
            "response_time_ms".to_owned(),
            nullable(
                &response.as_object().cloned().unwrap_or_default(),
                "response_time_ms",
            ),
        );
        items.push(Value::Object(item));
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_upstream_fields_and_request_context() {
        let params = vec![
            ("gl".to_owned(), "de".to_owned()),
            ("hl".to_owned(), "en".to_owned()),
            ("currency".to_owned(), "EUR".to_owned()),
            ("type".to_owned(), "all".to_owned()),
        ];
        let items = build_dataset_items(
            &json!({"suggestions": [{"position": 1, "value": "Berlin hotels", "type": "location"}], "response_time_ms": 935}),
            "Berlin",
            &params,
        );

        assert_eq!(
            items,
            [json!({
                "position": 1,
                "value": "Berlin hotels",
                "type": "location",
                "autocomplete_suggestion": null,
                "highlighted_words": [],
                "property_token": null,
                "thumbnail": null,
                "scrappa_google_hotels_link": null,
                "source_query": "Berlin",
                "request_gl": "de",
                "request_hl": "en",
                "request_currency": "EUR",
                "request_type": "all",
                "response_time_ms": 935
            })]
        );
    }

    #[test]
    fn deduplicates_by_property_token_and_drops_empty_or_malformed_suggestions() {
        let items = build_dataset_items(
            &json!({"suggestions": [
                {"value": "Park Inn", "type": "accommodation", "property_token": "token-1"},
                {"value": "Renamed Park Inn", "type": "accommodation", "property_token": "TOKEN-1"},
                {"type": "location"},
                null,
                42,
                {"value": "Wrong type", "type": 1},
                {"value": "Valid location", "type": "location"}
            ]}),
            "park",
            &[],
        );

        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["value"], "Park Inn");
        assert_eq!(items[1]["value"], "Valid location");
        assert_eq!(items[0]["source_query"], "park");
    }

    #[test]
    fn uses_autocomplete_text_as_the_value_and_keeps_extra_upstream_fields() {
        let items = build_dataset_items(
            &json!({"suggestions": [{
                "autocomplete_suggestion": "Berlin",
                "type": "location",
                "google_id": "abc",
                "highlighted_words": ["Berlin"]
            }]}),
            "ber",
            &[],
        );

        assert_eq!(items[0]["value"], "Berlin");
        assert_eq!(items[0]["google_id"], "abc");
        assert_eq!(items[0]["highlighted_words"], json!(["Berlin"]));
    }

    #[test]
    fn unexpected_response_shapes_produce_no_dataset_rows() {
        assert!(build_dataset_items(&json!({}), "Berlin", &[]).is_empty());
        assert!(build_dataset_items(&json!({"suggestions": {}}), "Berlin", &[]).is_empty());
    }
}
