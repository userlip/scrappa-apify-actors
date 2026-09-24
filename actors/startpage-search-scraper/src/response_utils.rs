use serde_json::{Map, Value};

pub fn extract_startpage_organic_results(response: &Value) -> Vec<Value> {
    if let Some(results) = response.as_array() {
        return results.clone();
    }
    if let Some(payload) = response.as_object() {
        for key in ["data", "organic_results", "results"] {
            if let Some(results) = payload.get(key).and_then(Value::as_array) {
                return results.clone();
            }
        }
    }
    eprintln!("Scrappa Startpage response did not include an organic result array");
    Vec::new()
}

fn clean_source(value: Option<&Value>) -> Option<String> {
    let value = value?.as_str()?.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn nullish(object: Option<&Map<String, Value>>, key: &str) -> Value {
    object
        .and_then(|object| object.get(key))
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn build_startpage_dataset_item(
    result: &Value,
    params: &Map<String, Value>,
    response: &Value,
) -> Value {
    let result = result.as_object();
    let response = response.as_object();
    let mut item = result.cloned().unwrap_or_default();

    let source = clean_source(result.and_then(|result| result.get("source")))
        .or_else(|| clean_source(response.and_then(|response| response.get("source"))))
        .unwrap_or_else(|| "startpage".to_owned());

    for (key, value) in [
        ("query", params.get("query").cloned().unwrap_or(Value::Null)),
        ("position", nullish(result, "position")),
        ("title", nullish(result, "title")),
        ("description", nullish(result, "description")),
        ("url", nullish(result, "url")),
        ("domain", nullish(result, "domain")),
        ("source", Value::String(source)),
        (
            "request_query",
            params.get("query").cloned().unwrap_or(Value::Null),
        ),
        (
            "request_language",
            params.get("language").cloned().unwrap_or(Value::Null),
        ),
        (
            "request_page",
            params.get("page").cloned().unwrap_or(Value::Null),
        ),
        (
            "request_safe_search",
            params.get("safe_search").cloned().unwrap_or(Value::Null),
        ),
        ("total_results", nullish(response, "total_results")),
        ("pagination", nullish(response, "pagination")),
        (
            "scrappa_pagination",
            nullish(response, "scrappa_pagination"),
        ),
    ] {
        item.insert(key.to_owned(), value);
    }
    Value::Object(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_supported_startpage_response_shapes() {
        let result = json!({"position": 1, "title": "Privacy Tools"});
        assert_eq!(
            extract_startpage_organic_results(&json!([result.clone()])),
            vec![result.clone()]
        );
        for key in ["data", "organic_results", "results"] {
            assert_eq!(
                extract_startpage_organic_results(&json!({key: [result.clone()]})),
                vec![result.clone()]
            );
        }
    }

    #[test]
    fn returns_an_empty_result_set_for_unexpected_response_shapes() {
        assert!(extract_startpage_organic_results(&json!({"items": [{"position": 1}]})).is_empty());
        assert!(extract_startpage_organic_results(&Value::Null).is_empty());
    }

    #[test]
    fn builds_dataset_items_with_request_and_pagination_metadata() {
        let params = json!({
            "query": "privacy tools", "language": "english", "page": 0, "safe_search": 1
        })
        .as_object()
        .unwrap()
        .clone();
        let item = build_startpage_dataset_item(
            &json!({
                "position": 1,
                "title": "Privacy Tools",
                "description": "Private search result",
                "url": "https://www.privacytools.io/",
                "domain": "www.privacytools.io"
            }),
            &params,
            &json!({
                "total_results": 20,
                "source": "startpage",
                "pagination": {"current": 0},
                "scrappa_pagination": {"page": 0}
            }),
        );
        assert_eq!(
            item,
            json!({
                "position": 1,
                "title": "Privacy Tools",
                "description": "Private search result",
                "url": "https://www.privacytools.io/",
                "domain": "www.privacytools.io",
                "query": "privacy tools",
                "source": "startpage",
                "request_query": "privacy tools",
                "request_language": "english",
                "request_page": 0,
                "request_safe_search": 1,
                "total_results": 20,
                "pagination": {"current": 0},
                "scrappa_pagination": {"page": 0}
            })
        );
    }

    #[test]
    fn falls_back_to_startpage_for_blank_sources_and_preserves_result_fields() {
        let params = json!({"query": "privacy tools"})
            .as_object()
            .unwrap()
            .clone();
        let item = build_startpage_dataset_item(
            &json!({"title": "Privacy Tools", "source": "  ", "extra": {"kept": true}}),
            &params,
            &json!({"source": "\t"}),
        );
        assert_eq!(item["source"], "startpage");
        assert_eq!(item["extra"], json!({"kept": true}));
        assert_eq!(item["position"], Value::Null);
    }
}
