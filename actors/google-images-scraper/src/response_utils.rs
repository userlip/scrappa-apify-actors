use serde_json::Value;

pub fn extract_image_results(response: &Value) -> Vec<Value> {
    if let Some(results) = response.as_array() {
        return results.clone();
    }
    if let Some(results) = response.get("data").and_then(Value::as_array) {
        return results.clone();
    }
    eprintln!("Scrappa Google Images response did not include an image result array");
    Vec::new()
}

pub fn limit_image_results(response: &Value, limit: usize) -> Value {
    if let Some(results) = response.as_array() {
        return Value::Array(results.iter().take(limit).cloned().collect());
    }

    let Some(object) = response.as_object() else {
        return response.clone();
    };
    let Some(results) = object.get("data").and_then(Value::as_array) else {
        return response.clone();
    };
    let mut limited = object.clone();
    limited.insert(
        "data".into(),
        Value::Array(results.iter().take(limit).cloned().collect()),
    );
    Value::Object(limited)
}

pub fn enrich_result(result: &Value, params: &Value) -> Value {
    let mut enriched = result.as_object().cloned().unwrap_or_default();
    let result_value = |field: &str| non_null(result.get(field));
    let request_value = |field: &str| non_null(params.get(field));

    enriched.insert("position".into(), result_value("position"));
    enriched.insert("source".into(), result_value("source"));
    enriched.insert("image_url".into(), result_value("original"));
    enriched.insert("thumbnail_url".into(), result_value("thumbnail"));
    enriched.insert("source_url".into(), result_value("link"));
    enriched.insert("width".into(), result_value("original_width"));
    enriched.insert("height".into(), result_value("original_height"));
    enriched.insert(
        "is_product".into(),
        result
            .get("is_product")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Bool(false)),
    );

    for (output_field, request_field) in [
        ("request_q", "q"),
        ("request_page", "page"),
        ("request_hl", "hl"),
        ("request_gl", "gl"),
        ("request_imgsz", "imgsz"),
        ("request_imgtype", "imgtype"),
        ("request_imgcolor", "imgcolor"),
        ("request_imgar", "imgar"),
        ("request_tbs", "tbs"),
        ("request_safe", "safe"),
    ] {
        enriched.insert(output_field.into(), request_value(request_field));
    }

    Value::Object(enriched)
}

fn non_null(value: Option<&Value>) -> Value {
    value
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn extracts_results_from_arrays_and_data_wrappers() {
        let result = json!({"position": 1, "title": "Coffee"});
        assert_eq!(
            extract_image_results(&json!([result.clone()])),
            vec![result.clone()]
        );
        assert_eq!(
            extract_image_results(&json!({"data": [result.clone()]})),
            vec![result]
        );
    }

    #[test]
    fn unexpected_response_shapes_produce_no_results() {
        assert!(extract_image_results(&json!({"results": [{"position": 1}]})).is_empty());
        assert!(extract_image_results(&Value::Null).is_empty());
        assert!(extract_image_results(&json!("unexpected")).is_empty());
    }

    #[test]
    fn limits_array_and_wrapped_results_without_changing_other_response_fields() {
        assert_eq!(limit_image_results(&json!([1, 2, 3]), 2), json!([1, 2]));
        assert_eq!(
            limit_image_results(&json!({"meta":{"page":1},"data":[1,2,3]}), 1),
            json!({"meta":{"page":1},"data":[1]})
        );
    }

    #[test]
    fn enriches_image_results_with_aliases_request_metadata_and_null_defaults() {
        assert_eq!(
            enrich_result(
                &json!({
                    "title": "Coffee",
                    "original": "https://example.com/coffee.jpg",
                    "thumbnail": "https://example.com/thumb.jpg",
                    "link": "https://example.com/source"
                }),
                &json!({"q":"coffee", "page":1, "hl":"en", "gl":"us", "imgsz":"large", "safe":"active"})
            ),
            json!({
                "title": "Coffee",
                "original": "https://example.com/coffee.jpg",
                "thumbnail": "https://example.com/thumb.jpg",
                "link": "https://example.com/source",
                "position": null,
                "source": null,
                "image_url": "https://example.com/coffee.jpg",
                "thumbnail_url": "https://example.com/thumb.jpg",
                "source_url": "https://example.com/source",
                "width": null,
                "height": null,
                "is_product": false,
                "request_q": "coffee",
                "request_page": 1,
                "request_hl": "en",
                "request_gl": "us",
                "request_imgsz": "large",
                "request_imgtype": null,
                "request_imgcolor": null,
                "request_imgar": null,
                "request_tbs": null,
                "request_safe": "active"
            })
        );
    }
}
