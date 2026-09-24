use serde_json::{Map, Value};

pub fn trusted_shops(response: &Value) -> Vec<Value> {
    let shops = response.get("shops").and_then(Value::as_array);
    if let Some(shops) = shops.filter(|shops| !shops.is_empty()) {
        return shops.clone();
    }
    if let Some(shops) = response
        .get("data")
        .and_then(|data| data.get("shops"))
        .and_then(Value::as_array)
    {
        return shops.clone();
    }
    shops.cloned().unwrap_or_default()
}

pub fn build_dataset_item(shop: &Value, params: &Map<String, Value>, response: &Value) -> Value {
    let mut item = shop.as_object().cloned().unwrap_or_default();
    let categories = shop
        .get("shopCategories")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();

    insert_nullable_source_field(&mut item, shop, "tsID");
    insert_nullable_source_field(&mut item, shop, "accountName");
    insert_nullable_source_field(&mut item, shop, "shopName");
    insert_nullable_source_field(&mut item, shop, "shopUrl");
    item.insert("shop_url".to_owned(), with_protocol(shop.get("shopUrl")));
    insert_nullable_source_field(&mut item, shop, "profileUrl");
    item.insert(
        "profile_url".to_owned(),
        with_protocol(shop.get("profileUrl")),
    );
    insert_nullable_source_field(&mut item, shop, "profileType");
    insert_nullable_source_field(&mut item, shop, "shopDescription");
    insert_nullable_source_field(&mut item, shop, "shopLogoUrl");
    insert_nullable_source_field(&mut item, shop, "averageRating");
    insert_nullable_source_field(&mut item, shop, "reviewCount");
    insert_nullable_source_field(&mut item, shop, "certificationState");
    insert_nullable_source_field(&mut item, shop, "contractStartDate");

    item.insert(
        "category_names".to_owned(),
        Value::String(join_truthy_fields(categories, "name")),
    );
    item.insert(
        "category_ids".to_owned(),
        Value::String(join_present_fields(categories, "id")),
    );
    item.insert(
        "category_url_paths".to_owned(),
        Value::String(join_truthy_fields(categories, "urlPath")),
    );
    item.insert(
        "request_q".to_owned(),
        params.get("q").cloned().unwrap_or(Value::Null),
    );
    item.insert(
        "request_market".to_owned(),
        params.get("market").cloned().unwrap_or(Value::Null),
    );
    item.insert(
        "request_page".to_owned(),
        params.get("page").cloned().unwrap_or(Value::Null),
    );
    item.insert(
        "total_shop_count".to_owned(),
        nested_nullable_value(response, &["metaData", "totalShopCount"]),
    );
    item.insert(
        "total_page_count".to_owned(),
        nested_nullable_value(response, &["metaData", "totalPageCount"]),
    );
    Value::Object(item)
}

fn insert_nullable_source_field(item: &mut Map<String, Value>, source: &Value, field: &str) {
    item.insert(field.to_owned(), nullable_value(source.get(field)));
}

fn nested_nullable_value(source: &Value, fields: &[&str]) -> Value {
    fields
        .iter()
        .try_fold(source, |value, field| value.get(*field))
        .map(|value| nullable_value(Some(value)))
        .unwrap_or(Value::Null)
}

fn nullable_value(value: Option<&Value>) -> Value {
    value
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn join_truthy_fields(values: &[Value], field: &str) -> String {
    values
        .iter()
        .filter_map(|value| value.get(field))
        .filter(|value| js_truthy(value))
        .map(js_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn join_present_fields(values: &[Value], field: &str) -> String {
    values
        .iter()
        .filter_map(|value| value.as_object().and_then(|value| value.get(field)))
        .map(|value| {
            if value.is_null() {
                String::new()
            } else {
                js_string(value)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn with_protocol(value: Option<&Value>) -> Value {
    let Some(value) = value.filter(|value| js_truthy(value)) else {
        return Value::Null;
    };
    let value = js_string(value);
    if value
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || value
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
    {
        Value::String(value)
    } else {
        Value::String(format!("https://{value}"))
    }
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn reads_primary_and_wrapped_shop_arrays() {
        assert_eq!(
            trusted_shops(&json!({"shops": [{"shopName": "primary"}]})),
            vec![json!({"shopName": "primary"})]
        );
        assert_eq!(
            trusted_shops(&json!({"data": {"shops": [{"shopName": "wrapped"}]}})),
            vec![json!({"shopName": "wrapped"})]
        );
        assert_eq!(
            trusted_shops(&json!({"shops": [], "data": {"shops": [{"shopName": "fallback"}]}})),
            vec![json!({"shopName": "fallback"})]
        );
        assert!(trusted_shops(&json!({})).is_empty());
    }

    #[test]
    fn adds_urls_categories_request_fields_and_metadata() {
        let shop = json!({
            "profileType": "member",
            "accountName": "Example Shop GmbH",
            "tsID": "XFB15FFBDE1DEE7A55D292A7D48598A6A",
            "shopDescription": "Online shop.",
            "shopName": "example-shop.de",
            "shopUrl": "www.example-shop.de",
            "shopCategories": [
                {"name": "Fashion", "id": 23, "urlPath": "fashion"},
                {"name": "Shoes", "id": 24, "urlPath": "shoes"}
            ],
            "shopLogoUrl": "https://example.com/logo.png",
            "averageRating": 4.8,
            "reviewCount": 12000,
            "certificationState": true,
            "profileUrl": "www.trustedshops.de/shop.html",
            "contractStartDate": 1610582400000_i64,
            "upstreamExtra": {"preserved": true}
        });
        let params = json!({"q": "zalando", "market": "DEU", "page": 2})
            .as_object()
            .unwrap()
            .clone();
        let response = json!({"metaData": {"totalShopCount": 66, "totalPageCount": 4}});
        let item = build_dataset_item(&shop, &params, &response);

        assert_eq!(item["tsID"], "XFB15FFBDE1DEE7A55D292A7D48598A6A");
        assert_eq!(item["shop_url"], "https://www.example-shop.de");
        assert_eq!(item["profile_url"], "https://www.trustedshops.de/shop.html");
        assert_eq!(item["category_names"], "Fashion, Shoes");
        assert_eq!(item["category_ids"], "23, 24");
        assert_eq!(item["category_url_paths"], "fashion, shoes");
        assert_eq!(item["request_q"], "zalando");
        assert_eq!(item["request_market"], "DEU");
        assert_eq!(item["request_page"], 2);
        assert_eq!(item["total_shop_count"], 66);
        assert_eq!(item["total_page_count"], 4);
        assert_eq!(item["upstreamExtra"]["preserved"], true);
    }

    #[test]
    fn null_fallbacks_and_javascript_category_truthiness_match_dataset_contract() {
        let shop = json!({
            "shopUrl": "HTTP://example.test",
            "profileUrl": "",
            "shopCategories": [
                {"name": "Fashion", "id": 23, "urlPath": "fashion"},
                {"name": "", "id": null, "urlPath": null},
                {"name": 7, "id": 24, "urlPath": ""}
            ]
        });
        let params = Map::new();
        let item = build_dataset_item(
            &shop,
            &params,
            &json!({"metaData": {"totalPageCount": null}}),
        );

        assert_eq!(item["shopUrl"], "HTTP://example.test");
        assert_eq!(item["shop_url"], "HTTP://example.test");
        assert_eq!(item["profile_url"], Value::Null);
        assert_eq!(item["tsID"], Value::Null);
        assert_eq!(item["category_names"], "Fashion, 7");
        assert_eq!(item["category_ids"], "23, , 24");
        assert_eq!(item["category_url_paths"], "fashion");
        assert_eq!(item["total_page_count"], Value::Null);
        assert_eq!(item["request_q"], Value::Null);
    }
}
