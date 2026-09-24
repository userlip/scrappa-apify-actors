use serde_json::{Map, Value, json};

#[derive(Clone, Copy)]
enum ResultKind {
    Query,
    Topic,
}

impl ResultKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Topic => "topic",
        }
    }
}

fn object_entries(value: Option<&Value>) -> Vec<&Map<String, Value>> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .collect()
}

fn related_groups(value: Option<&Value>) -> Vec<(Option<&'static str>, Vec<&Map<String, Value>>)> {
    match value {
        Some(Value::Array(_)) => vec![(None, object_entries(value))],
        Some(Value::Object(_)) => [
            (
                Some("top"),
                object_entries(value.and_then(|value| value.get("top"))),
            ),
            (
                Some("rising"),
                object_entries(value.and_then(|value| value.get("rising"))),
            ),
        ]
        .into_iter()
        .filter(|(_, entries)| !entries.is_empty())
        .collect(),
        _ => Vec::new(),
    }
}

fn first_nonempty_string<'a>(
    values: impl IntoIterator<Item = Option<&'a Value>>,
) -> Option<&'a str> {
    values
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .find(|value| !value.trim().is_empty())
}

fn dataset_item(
    entry: &Map<String, Value>,
    params: &Map<String, Value>,
    response: &Value,
    kind: ResultKind,
    group_type: Option<&str>,
    position: usize,
) -> Value {
    let query = if matches!(kind, ResultKind::Query) {
        first_nonempty_string([entry.get("query"), entry.get("title")])
    } else {
        None
    };
    let topic = if matches!(kind, ResultKind::Topic) {
        first_nonempty_string([entry.get("topic"), entry.get("title"), entry.get("query")])
    } else {
        None
    };
    let topic_type = if matches!(kind, ResultKind::Topic) {
        first_nonempty_string([entry.get("type")])
    } else {
        None
    };

    let mut item = entry.clone();
    item.insert("position".to_owned(), json!(position));
    item.insert("result_kind".to_owned(), json!(kind.as_str()));
    item.insert(
        "type".to_owned(),
        group_type.map_or(Value::Null, |value| json!(value)),
    );
    item.insert(
        "query".to_owned(),
        query.map_or(Value::Null, |value| json!(value)),
    );
    item.insert(
        "topic".to_owned(),
        topic.map_or(Value::Null, |value| json!(value)),
    );
    item.insert(
        "topic_type".to_owned(),
        topic_type.map_or(Value::Null, |value| json!(value)),
    );
    for (key, value) in [
        ("value", entry.get("value")),
        ("formatted_value", entry.get("formatted_value")),
        ("link", entry.get("link")),
        ("source_keyword", params.get("q")),
        ("request_geo", params.get("geo")),
        ("request_time_range", params.get("time_range")),
        ("request_hl", params.get("hl")),
        ("request_search_type", params.get("search_type")),
        ("response_time_ms", response.get("response_time_ms")),
        ("search_parameters", response.get("search_parameters")),
    ] {
        item.insert(
            key.to_owned(),
            value
                .filter(|value| !value.is_null())
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    Value::Object(item)
}

pub fn build_dataset_items(response: &Value, params: &Map<String, Value>) -> Vec<Value> {
    let mut items = Vec::new();
    for (kind, field) in [
        (ResultKind::Query, "related_queries"),
        (ResultKind::Topic, "related_topics"),
    ] {
        for (group_type, entries) in related_groups(response.get(field)) {
            for (index, entry) in entries.into_iter().enumerate() {
                items.push(dataset_item(
                    entry,
                    params,
                    response,
                    kind,
                    group_type,
                    index + 1,
                ));
            }
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn flattens_top_and_rising_queries_and_preserves_entry_fields() {
        let params = serde_json::from_value(
            json!({"q":"coffee", "geo":"US", "time_range":"90d", "hl":"en", "search_type":"web"}),
        )
        .unwrap();
        let response = json!({
            "search_parameters":{"keyword":"coffee"}, "response_time_ms":623,
            "related_queries":{"top":[{"query":"coffee near me", "value":100, "extra":"kept"}], "rising":[{"title":"mushroom coffee", "formatted_value":"+850%"}]}
        });
        let items = build_dataset_items(&response, &params);

        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["result_kind"], "query");
        assert_eq!(items[0]["type"], "top");
        assert_eq!(items[0]["query"], "coffee near me");
        assert_eq!(items[0]["extra"], "kept");
        assert_eq!(items[0]["request_time_range"], "90d");
        assert_eq!(items[1]["type"], "rising");
        assert_eq!(items[1]["query"], "mushroom coffee");
        assert_eq!(items[1]["position"], 1);
    }

    #[test]
    fn handles_flat_lists_topics_empty_values_and_position_scopes() {
        let params = serde_json::from_value(json!({"q":"coffee"})).unwrap();
        let flat = build_dataset_items(
            &json!({"related_queries":[{"query":"coffee shops"}, 4, null]}),
            &params,
        );
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0]["type"], Value::Null);
        assert_eq!(flat[0]["query"], "coffee shops");

        let topics = build_dataset_items(
            &json!({
                "related_queries":{"top":[{"query":"near me"},{"query":"shops"}],"rising":[{"query":"mushroom"}]},
                "related_topics":{"top":[{"topic":"Coffee", "type":"Drink"}],"rising":[{"title":"Cold brew", "type":"Topic"}]}
            }),
            &params,
        );
        assert_eq!(
            topics
                .iter()
                .map(|item| (
                    item["result_kind"].as_str().unwrap(),
                    item["type"].as_str(),
                    item["position"].as_u64().unwrap()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("query", Some("top"), 1),
                ("query", Some("top"), 2),
                ("query", Some("rising"), 1),
                ("topic", Some("top"), 1),
                ("topic", Some("rising"), 1),
            ]
        );
        assert_eq!(topics[3]["topic"], "Coffee");
        assert_eq!(topics[3]["topic_type"], "Drink");
        assert_eq!(topics[4]["topic"], "Cold brew");
        assert!(
            build_dataset_items(&json!({"related_queries":{"top":[null, 4]}}), &params).is_empty()
        );
    }
}
