use serde_json::{Map, Value};

use crate::input::VintedUserItemsPlan;

pub fn get_items(response: &Value) -> Vec<Value> {
    if let Some(items) = response.get("items").and_then(Value::as_array) {
        return items.clone();
    }
    response
        .pointer("/data/items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

pub fn get_pagination(response: &Value) -> Option<&Value> {
    response
        .get("pagination")
        .filter(|pagination| !pagination.is_null())
        .or_else(|| {
            response
                .pointer("/data/pagination")
                .filter(|pagination| !pagination.is_null())
        })
}

pub fn build_dataset_item(
    item: &Value,
    plan: &VintedUserItemsPlan,
    user_id: &str,
    page: u64,
    response: &Value,
) -> Value {
    let mut output = item.as_object().cloned().unwrap_or_else(Map::new);
    let pagination = get_pagination(response);
    let seller = item
        .get("seller")
        .filter(|seller| seller.is_object() || seller.is_array())
        .or_else(|| {
            item.get("user")
                .filter(|seller| seller.is_object() || seller.is_array())
        });
    let price = item.get("price");
    let total_item_price = item.get("total_item_price");
    let shipping_price = item.get("shipping_price");
    let service_fee = item.get("service_fee");

    insert_nullish(&mut output, "id", item.get("id"));
    insert_nullish(&mut output, "title", item.get("title"));
    insert_nullish(&mut output, "description", item.get("description"));
    insert_nullish(&mut output, "url", item.get("url"));
    insert_nullish(&mut output, "path", item.get("path"));
    output.insert("image_url".to_owned(), first_photo_url(item));
    output.insert("price_amount".to_owned(), money_amount(price));
    output.insert("price_currency".to_owned(), money_currency(price));
    output.insert(
        "total_item_price".to_owned(),
        money_amount(total_item_price),
    );
    output.insert(
        "total_item_price_currency".to_owned(),
        money_currency(total_item_price),
    );
    output.insert("shipping_price".to_owned(), money_amount(shipping_price));
    output.insert(
        "shipping_price_currency".to_owned(),
        money_currency(shipping_price),
    );
    output.insert("service_fee".to_owned(), money_amount(service_fee));
    output.insert(
        "service_fee_currency".to_owned(),
        money_currency(service_fee),
    );
    output.insert(
        "brand_name".to_owned(),
        label(item.get("brand"))
            .or_else(|| item.get("brand_title").cloned())
            .unwrap_or(Value::Null),
    );
    output.insert(
        "category_name".to_owned(),
        label(item.get("category")).unwrap_or(Value::Null),
    );
    output.insert(
        "size_name".to_owned(),
        label(item.get("size"))
            .or_else(|| item.get("size_title").cloned())
            .unwrap_or(Value::Null),
    );
    output.insert(
        "color_name".to_owned(),
        label(item.get("color"))
            .or_else(|| item.get("color_title").cloned())
            .unwrap_or(Value::Null),
    );
    output.insert(
        "condition".to_owned(),
        first_non_null(
            item,
            &["condition", "condition_title", "status", "status_title"],
        ),
    );
    insert_nullish(&mut output, "availability", item.get("availability"));
    output.insert(
        "favourite_count".to_owned(),
        first_non_null(item, &["favourite_count", "favorites_count"]),
    );
    insert_nullish(&mut output, "view_count", item.get("view_count"));
    output.insert(
        "seller_id".to_owned(),
        seller
            .and_then(|seller| nullish(seller.get("id")))
            .cloned()
            .unwrap_or(Value::Null),
    );
    output.insert(
        "seller_login".to_owned(),
        seller
            .and_then(|seller| {
                nullish(seller.get("login")).or_else(|| nullish(seller.get("username")))
            })
            .cloned()
            .unwrap_or(Value::Null),
    );
    output.insert(
        "seller_feedback_count".to_owned(),
        seller
            .and_then(|seller| nullish(seller.get("feedback_count")))
            .cloned()
            .unwrap_or(Value::Null),
    );
    output.insert(
        "seller_feedback_reputation".to_owned(),
        seller
            .and_then(|seller| nullish(seller.get("feedback_reputation")))
            .cloned()
            .unwrap_or(Value::Null),
    );
    output.insert(
        "input_user_id".to_owned(),
        Value::String(user_id.to_owned()),
    );
    output.insert(
        "request_country".to_owned(),
        Value::String(plan.country.clone()),
    );
    output.insert("request_page".to_owned(), Value::from(page));
    output.insert("request_per_page".to_owned(), Value::from(plan.per_page));
    output.insert(
        "request_order".to_owned(),
        Value::String(plan.order.clone()),
    );
    output.insert(
        "total_pages".to_owned(),
        pagination
            .and_then(|pagination| nullish(pagination.get("total_pages")))
            .cloned()
            .unwrap_or(Value::Null),
    );
    output.insert(
        "total_entries".to_owned(),
        pagination
            .and_then(|pagination| {
                nullish(pagination.get("total_entries"))
                    .or_else(|| nullish(pagination.get("total_items")))
            })
            .cloned()
            .unwrap_or(Value::Null),
    );
    Value::Object(output)
}

fn insert_nullish(output: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    output.insert(
        key.to_owned(),
        value
            .cloned()
            .filter(|value| !value.is_null())
            .unwrap_or(Value::Null),
    );
}

fn nullish(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| !value.is_null())
}

fn first_non_null(item: &Value, fields: &[&str]) -> Value {
    fields
        .iter()
        .find_map(|field| item.get(field).filter(|value| !value.is_null()).cloned())
        .unwrap_or(Value::Null)
}

fn label(value: Option<&Value>) -> Option<Value> {
    let value = value?;
    if let Some(text) = value.as_str().filter(|text| !text.trim().is_empty()) {
        return Some(Value::String(text.to_owned()));
    }
    ["title", "name"].iter().find_map(|key| {
        value
            .get(key)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(|text| Value::String(text.to_owned()))
    })
}

fn money_amount(value: Option<&Value>) -> Value {
    let Some(value) = value else {
        return Value::Null;
    };
    if value.is_number() || value.is_string() {
        return value.clone();
    }
    value.get("amount").cloned().unwrap_or(Value::Null)
}

fn money_currency(value: Option<&Value>) -> Value {
    let Some(value) = value else {
        return Value::Null;
    };
    ["currency", "currency_code"]
        .iter()
        .find_map(|key| value.get(key).and_then(Value::as_str))
        .map(|currency| Value::String(currency.to_owned()))
        .unwrap_or(Value::Null)
}

fn first_photo_url(item: &Value) -> Value {
    for key in ["image_url", "photo_url"] {
        if let Some(value) = item.get(key).filter(|value| js_truthy(value)) {
            return value.clone();
        }
    }
    if let Some(value) = item.pointer("/photo/url").filter(|value| js_truthy(value)) {
        return value.clone();
    }
    item.get("photos")
        .and_then(Value::as_array)
        .and_then(|photos| {
            photos
                .iter()
                .filter_map(|photo| photo.get("url"))
                .find(|url| js_truthy(url))
        })
        .cloned()
        .unwrap_or(Value::Null)
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{build_plan, ActorInput};
    use serde_json::json;

    #[test]
    fn reads_root_and_wrapped_item_lists_and_pagination() {
        assert_eq!(
            get_items(&json!({"items":[{"title":"Root"}]})),
            vec![json!({"title":"Root"})]
        );
        assert_eq!(
            get_items(&json!({"items":null,"data":{"items":[{"title":"Wrapped"}]}})),
            vec![json!({"title":"Wrapped"})]
        );
        assert!(get_items(&json!({})).is_empty());
        assert_eq!(
            get_pagination(&json!({"pagination":null,"data":{"pagination":{"total_pages":3}}})),
            Some(&json!({"total_pages":3}))
        );
    }

    #[test]
    fn preserves_raw_fields_and_normalizes_listing_output() {
        let input: ActorInput = serde_json::from_value(json!({
            "user_id": "98765432",
            "country": "DE",
            "page": 2,
            "per_page": 50
        }))
        .unwrap();
        let plan = build_plan(&input).unwrap();
        let response = json!({
            "data":{"pagination":{"total_pages":20,"total_entries":980}},
            "items":[]
        });
        let item = build_dataset_item(
            &json!({
                "id": "1234567890",
                "title": "Nike Air Max 90",
                "price": {"amount":"45.00","currency_code":"EUR"},
                "total_item_price":{"amount":"50.49","currency_code":"EUR"},
                "shipping_price":{"amount":"3.49","currency_code":"EUR"},
                "service_fee":{"amount":"2.00","currency_code":"EUR"},
                "brand_title":"Nike",
                "category":{"name":"Shoes"},
                "size_title":"EU 42",
                "status":"Very good",
                "url":"https://www.vinted.de/items/1234567890-nike-air-max-90",
                "photo":{"url":"https://images1.vinted.net/example.jpg"},
                "user":{"id":98765432,"login":"seller123","feedback_count":50,"feedback_reputation":4.8},
                "favourite_count":15,
                "view_count":234,
                "unmodeled_field":{"preserved":true}
            }),
            &plan,
            "98765432",
            2,
            &response,
        );

        assert_eq!(item["price_amount"], "45.00");
        assert_eq!(item["price_currency"], "EUR");
        assert_eq!(item["total_item_price"], "50.49");
        assert_eq!(item["service_fee"], "2.00");
        assert_eq!(item["brand_name"], "Nike");
        assert_eq!(item["category_name"], "Shoes");
        assert_eq!(item["size_name"], "EU 42");
        assert_eq!(item["condition"], "Very good");
        assert_eq!(item["image_url"], "https://images1.vinted.net/example.jpg");
        assert_eq!(item["seller_login"], "seller123");
        assert_eq!(item["input_user_id"], "98765432");
        assert_eq!(item["request_country"], "DE");
        assert_eq!(item["request_page"], 2);
        assert_eq!(item["request_per_page"], 50);
        assert_eq!(item["request_order"], "newest_first");
        assert_eq!(item["total_pages"], 20);
        assert_eq!(item["total_entries"], 980);
        assert_eq!(item["unmodeled_field"]["preserved"], true);
    }

    #[test]
    fn normalizes_fallback_fields_and_handles_missing_values() {
        let input: ActorInput = serde_json::from_value(json!({"user_id":"111"})).unwrap();
        let plan = build_plan(&input).unwrap();
        let item = build_dataset_item(
            &json!({
                "price":"12.50",
                "favorites_count":4,
                "condition":null,
                "condition_title":"Good",
                "image_url":"",
                "photos":[{"url":""},{"url":"photo.jpg"}],
                "seller":{"login":null,"username":"seller"},
                "brand":{"title":"  "}
            }),
            &plan,
            "111",
            1,
            &json!({"pagination":{"total_items":8}}),
        );

        assert_eq!(item["price_amount"], "12.50");
        assert_eq!(item["favourite_count"], 4);
        assert_eq!(item["condition"], "Good");
        assert_eq!(item["image_url"], "photo.jpg");
        assert_eq!(item["seller_login"], "seller");
        assert_eq!(item["brand_name"], "  ");
        assert_eq!(item["total_entries"], 8);
    }
}
