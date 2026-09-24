use serde_json::{Map, Value};

const LISTING_SOURCES: [&str; 7] = [
    "data",
    "listings",
    "results",
    "items",
    "data.listings",
    "data.results",
    "data.items",
];

fn listing_array<'a>(response: &'a Value, source: &str) -> Option<&'a Vec<Value>> {
    match source {
        "data" => response.get("data")?.as_array(),
        "listings" | "results" | "items" => response.get(source)?.as_array(),
        "data.listings" | "data.results" | "data.items" => response
            .get("data")?
            .get(source.strip_prefix("data.")?)?
            .as_array(),
        _ => None,
    }
}

pub(crate) fn select_listings(response: &Value) -> (Vec<Value>, Option<&'static str>) {
    let candidates = LISTING_SOURCES
        .iter()
        .filter_map(|source| listing_array(response, source).map(|listings| (*source, listings)))
        .collect::<Vec<_>>();

    if let Some((source, listings)) = candidates.iter().find(|(_, listings)| !listings.is_empty()) {
        return (listings.to_vec(), Some(*source));
    }
    if let Some((source, listings)) = candidates.first() {
        return (listings.to_vec(), Some(*source));
    }
    eprintln!("Unexpected Kleinanzeigen response shape: expected \"data\", \"listings\", \"results\", or \"items\" array.");
    (Vec::new(), None)
}

fn value_or_null(object: &Value, key: &str) -> Value {
    object
        .get(key)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

pub(crate) fn build_dataset_item(
    listing: &Value,
    params: &Map<String, Value>,
    response: &Value,
) -> Value {
    let mut item = listing.as_object().cloned().unwrap_or_default();
    for field in [
        "id",
        "title",
        "url",
        "price",
        "price_numeric",
        "location",
        "description",
        "has_shipping",
    ] {
        item.insert(field.to_owned(), value_or_null(listing, field));
    }
    item.insert(
        "image_url".to_owned(),
        listing
            .get("image_url")
            .filter(|value| !value.is_null())
            .or_else(|| listing.get("image").filter(|value| !value.is_null()))
            .cloned()
            .unwrap_or(Value::Null),
    );
    for (request_field, output_field) in [
        ("query", "request_query"),
        ("page", "request_page"),
        ("location", "request_location"),
        ("category", "request_category"),
        ("price_min", "request_price_min"),
        ("price_max", "request_price_max"),
    ] {
        item.insert(
            output_field.to_owned(),
            params.get(request_field).cloned().unwrap_or(Value::Null),
        );
    }
    item.insert(
        "results_count".to_owned(),
        response
            .pointer("/meta/results_count")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    );
    Value::Object(item)
}

pub(crate) fn limit_search_response(
    response: &Value,
    limit: usize,
    selected_source: Option<&str>,
) -> Value {
    let mut limited = response.as_object().cloned().unwrap_or_default();
    match response.get("data") {
        Some(Value::Array(listings)) => {
            if selected_source == Some("data") {
                limited.insert(
                    "data".to_owned(),
                    Value::Array(listings.iter().take(limit).cloned().collect()),
                );
            } else {
                limited.remove("data");
            }
        }
        Some(Value::Object(data)) => {
            let mut limited_data = data.clone();
            for source in ["listings", "results", "items"] {
                if let Some(Value::Array(listings)) = data.get(source) {
                    let nested_source = format!("data.{source}");
                    if selected_source == Some(nested_source.as_str()) {
                        limited_data.insert(
                            source.to_owned(),
                            Value::Array(listings.iter().take(limit).cloned().collect()),
                        );
                    } else {
                        limited_data.remove(source);
                    }
                }
            }
            limited.insert("data".to_owned(), Value::Object(limited_data));
        }
        _ => {}
    }
    for source in ["listings", "results", "items"] {
        if let Some(Value::Array(listings)) = response.get(source) {
            if selected_source == Some(source) {
                limited.insert(
                    source.to_owned(),
                    Value::Array(listings.iter().take(limit).cloned().collect()),
                );
            } else {
                limited.remove(source);
            }
        }
    }
    Value::Object(limited)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::build_search_plan;
    use serde_json::json;

    #[test]
    fn selects_and_limits_listing_response_shapes() {
        for (response, expected_source) in [
            (json!({"data": [{"id": "data"}]}), "data"),
            (json!({"listings": [{"id": "listings"}]}), "listings"),
            (json!({"results": [{"id": "results"}]}), "results"),
            (json!({"items": [{"id": "items"}]}), "items"),
            (
                json!({"data": {"listings": [{"id": "nested-listings"}]}}),
                "data.listings",
            ),
            (
                json!({"data": {"results": [{"id": "nested-results"}]}}),
                "data.results",
            ),
            (
                json!({"data": {"items": [{"id": "nested-items"}]}}),
                "data.items",
            ),
        ] {
            let (_, source) = select_listings(&response);
            assert_eq!(source, Some(expected_source));
        }
        assert_eq!(select_listings(&json!({})), (Vec::new(), None));

        let (listings, source) = select_listings(&json!({
            "data": [],
            "listings": [],
            "results": [{"id": "result"}],
            "items": [{"id": "item"}],
            "data_extra": [{"id": "ignored"}]
        }));
        assert_eq!(source, Some("results"));
        assert_eq!(listings, vec![json!({"id": "result"})]);

        let (nested, source) = select_listings(&json!({
            "listings": [],
            "data": {"listings": [{"id": "nested"}]}
        }));
        assert_eq!(source, Some("data.listings"));
        assert_eq!(nested, vec![json!({"id": "nested"})]);

        let params = build_search_plan(&json!({
            "query": "iphone", "page": 2, "location": "Berlin", "price_min": 500
        }))
        .unwrap()
        .remove(0)
        .params;
        let response = json!({
            "data": {
                "cursor": "next-page",
                "listings": [{"id": "a"}, {"id": "b"}],
                "results": [{"id": "ignored"}]
            },
            "listings": [{"id": "top"}],
            "meta": {"results_count": 26}
        });
        let item = build_dataset_item(
            &json!({"id": "listing-1", "image": "image.jpg", "extra": true}),
            &params,
            &response,
        );
        assert_eq!(item["id"], "listing-1");
        assert_eq!(item["image_url"], "image.jpg");
        assert_eq!(item["request_query"], "iphone");
        assert_eq!(item["request_page"], 2);
        assert_eq!(item["request_location"], "Berlin");
        assert_eq!(item["request_category"], Value::Null);
        assert_eq!(item["request_price_min"], 500);
        assert_eq!(item["results_count"], 26);
        assert_eq!(item["extra"], true);
        assert_eq!(
            limit_search_response(&response, 1, Some("data.listings")),
            json!({
                "data": {"cursor": "next-page", "listings": [{"id": "a"}]},
                "meta": {"results_count": 26}
            })
        );
    }
}
