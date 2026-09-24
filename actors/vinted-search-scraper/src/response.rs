use serde_json::{Map, Value};

use crate::input::VintedSearchPlan;

pub fn get_items(response: &Value) -> &[Value] {
    response
        .get("items")
        .and_then(Value::as_array)
        .or_else(|| {
            response
                .get("data")
                .and_then(|data| data.get("items"))
                .and_then(Value::as_array)
        })
        .map(Vec::as_slice)
        .unwrap_or_default()
}

pub fn get_pagination(response: &Value) -> Option<&Value> {
    response
        .get("pagination")
        .filter(|pagination| !pagination.is_null())
        .or_else(|| {
            response
                .get("data")
                .and_then(|data| data.get("pagination"))
                .filter(|pagination| !pagination.is_null())
        })
}

fn nullish_field(value: &Value, key: &str) -> Value {
    value
        .get(key)
        .filter(|field| !field.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn first_nullish_field(value: &Value, keys: &[&str]) -> Value {
    keys.iter()
        .find_map(|key| value.get(*key).filter(|field| !field.is_null()).cloned())
        .unwrap_or(Value::Null)
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn label(value: Option<&Value>) -> Option<Value> {
    let value = value?;
    match value {
        Value::String(value) if !value.trim().is_empty() => Some(Value::String(value.clone())),
        Value::Object(record) => ["title", "name"]
            .iter()
            .find_map(|key| match record.get(*key) {
                Some(Value::String(value)) if !value.is_empty() => {
                    Some(Value::String(value.clone()))
                }
                _ => None,
            }),
        _ => None,
    }
}

fn money_amount(value: Option<&Value>) -> Value {
    let Some(value) = value else {
        return Value::Null;
    };
    match value {
        Value::Number(_) | Value::String(_) => value.clone(),
        _ => value
            .get("amount")
            .filter(|amount| !amount.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    }
}

fn money_currency(value: Option<&Value>) -> Value {
    let Some(value) = value else {
        return Value::Null;
    };
    for key in ["currency", "currency_code"] {
        if let Some(Value::String(currency)) = value.get(key) {
            return Value::String(currency.clone());
        }
    }
    Value::Null
}

fn first_photo_url(item: &Value) -> Value {
    for field in ["image_url", "photo_url"] {
        if let Some(value) = item.get(field).filter(|value| truthy(value)) {
            return value.clone();
        }
    }
    if let Some(value) = item
        .get("photo")
        .and_then(|photo| photo.get("url"))
        .filter(|value| truthy(value))
    {
        return value.clone();
    }
    item.get("photos")
        .and_then(Value::as_array)
        .and_then(|photos| {
            photos
                .iter()
                .find_map(|photo| photo.get("url").filter(|url| truthy(url)))
        })
        .cloned()
        .unwrap_or(Value::Null)
}

fn selected_seller(item: &Value) -> Option<&Value> {
    ["seller", "user"].iter().find_map(|key| {
        item.get(*key)
            .filter(|seller| seller.is_object() || seller.is_array())
    })
}

fn request_field(params: &Map<String, Value>, key: &str) -> Value {
    params
        .get(key)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn build_dataset_item(item: &Value, params: &Map<String, Value>, response: &Value) -> Value {
    let mut row = item.as_object().cloned().unwrap_or_default();
    let seller = selected_seller(item);
    let pagination = get_pagination(response);
    let total_pages = pagination
        .map(|pagination| nullish_field(pagination, "total_pages"))
        .unwrap_or(Value::Null);
    let total_entries = pagination
        .map(|pagination| first_nullish_field(pagination, &["total_entries", "total_items"]))
        .unwrap_or(Value::Null);

    row.insert("id".to_owned(), nullish_field(item, "id"));
    row.insert("title".to_owned(), nullish_field(item, "title"));
    row.insert("description".to_owned(), nullish_field(item, "description"));
    row.insert("url".to_owned(), nullish_field(item, "url"));
    row.insert("path".to_owned(), nullish_field(item, "path"));
    row.insert("image_url".to_owned(), first_photo_url(item));
    row.insert("price_amount".to_owned(), money_amount(item.get("price")));
    row.insert(
        "price_currency".to_owned(),
        money_currency(item.get("price")),
    );
    row.insert(
        "total_item_price".to_owned(),
        money_amount(item.get("total_item_price")),
    );
    row.insert(
        "total_item_price_currency".to_owned(),
        money_currency(item.get("total_item_price")),
    );
    row.insert(
        "shipping_price".to_owned(),
        money_amount(item.get("shipping_price")),
    );
    row.insert(
        "shipping_price_currency".to_owned(),
        money_currency(item.get("shipping_price")),
    );
    row.insert(
        "service_fee".to_owned(),
        money_amount(item.get("service_fee")),
    );
    row.insert(
        "service_fee_currency".to_owned(),
        money_currency(item.get("service_fee")),
    );
    row.insert(
        "brand_name".to_owned(),
        label(item.get("brand")).unwrap_or_else(|| nullish_field(item, "brand_title")),
    );
    row.insert(
        "category_name".to_owned(),
        label(item.get("category")).unwrap_or(Value::Null),
    );
    row.insert(
        "size_name".to_owned(),
        label(item.get("size")).unwrap_or_else(|| nullish_field(item, "size_title")),
    );
    row.insert(
        "color_name".to_owned(),
        label(item.get("color")).unwrap_or_else(|| nullish_field(item, "color_title")),
    );
    row.insert(
        "condition".to_owned(),
        first_nullish_field(
            item,
            &["condition", "condition_title", "status", "status_title"],
        ),
    );
    row.insert(
        "availability".to_owned(),
        nullish_field(item, "availability"),
    );
    row.insert(
        "favourite_count".to_owned(),
        first_nullish_field(item, &["favourite_count", "favorites_count"]),
    );
    row.insert("view_count".to_owned(), nullish_field(item, "view_count"));
    row.insert(
        "seller_id".to_owned(),
        seller
            .map(|seller| nullish_field(seller, "id"))
            .unwrap_or(Value::Null),
    );
    row.insert(
        "seller_login".to_owned(),
        seller
            .map(|seller| first_nullish_field(seller, &["login", "username"]))
            .unwrap_or(Value::Null),
    );
    row.insert(
        "seller_feedback_count".to_owned(),
        seller
            .map(|seller| nullish_field(seller, "feedback_count"))
            .unwrap_or(Value::Null),
    );
    row.insert(
        "seller_feedback_reputation".to_owned(),
        seller
            .map(|seller| nullish_field(seller, "feedback_reputation"))
            .unwrap_or(Value::Null),
    );
    for (source, destination) in [
        ("query", "request_query"),
        ("country", "request_country"),
        ("page", "request_page"),
        ("per_page", "request_per_page"),
        ("order", "request_order"),
        ("brand_ids", "request_brand_ids"),
        ("catalog_ids", "request_catalog_ids"),
        ("size_ids", "request_size_ids"),
        ("price_from", "request_price_from"),
        ("price_to", "request_price_to"),
    ] {
        row.insert(destination.to_owned(), request_field(params, source));
    }
    row.insert("total_pages".to_owned(), total_pages);
    row.insert("total_entries".to_owned(), total_entries);

    Value::Object(row)
}

pub fn total_pages(pagination: Option<&Value>) -> Value {
    pagination
        .map(|pagination| nullish_field(pagination, "total_pages"))
        .unwrap_or(Value::Null)
}

pub fn total_entries(pagination: Option<&Value>) -> Value {
    pagination
        .map(|pagination| first_nullish_field(pagination, &["total_entries", "total_items"]))
        .unwrap_or(Value::Null)
}

pub fn has_no_next_page(pagination: Option<&Value>, page: u64) -> bool {
    let Some(pagination) = pagination else {
        return false;
    };
    if pagination.get("has_next_page").and_then(Value::as_bool) == Some(false) {
        return true;
    }
    pagination
        .get("total_pages")
        .and_then(Value::as_f64)
        .is_some_and(|total_pages| page as f64 >= total_pages)
}

pub fn request_summary(plan: &VintedSearchPlan) -> Map<String, Value> {
    let mut request = plan.base_params.clone();
    request.insert("start_page".to_owned(), Value::from(plan.start_page));
    request.insert("max_pages".to_owned(), Value::from(plan.max_pages));
    request
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_items_and_pagination_from_both_response_shapes() {
        assert_eq!(
            get_items(&json!({"items": [{"title": "primary"}]})),
            &[json!({"title": "primary"})]
        );
        assert_eq!(
            get_items(&json!({"data": {"items": [{"title": "wrapped"}]}})),
            &[json!({"title": "wrapped"})]
        );
        assert!(get_items(&json!({})).is_empty());
        assert_eq!(
            get_pagination(&json!({"pagination": {"total_pages": 3}})),
            Some(&json!({"total_pages": 3}))
        );
        assert_eq!(
            get_pagination(&json!({
                "pagination": null,
                "data": {"pagination": {"total_entries": 42}}
            })),
            Some(&json!({"total_entries": 42}))
        );
    }

    #[test]
    fn normalizes_listing_fields_and_retains_raw_fields() {
        let item = json!({
            "id": "1234567890",
            "title": "Nike Air Max 90",
            "price": {"amount": "45.00", "currency_code": "EUR"},
            "total_item_price": {"amount": "50.49", "currency_code": "EUR"},
            "shipping_price": {"amount": "3.49", "currency_code": "EUR"},
            "service_fee": {"amount": "2.00", "currency_code": "EUR"},
            "brand_title": "Nike",
            "category": {"name": "Shoes"},
            "size_title": "EU 42",
            "status": "Very good",
            "url": "https://www.vinted.de/items/1234567890-nike-air-max-90",
            "photo": {"url": "https://images1.vinted.net/example.jpg"},
            "user": {
                "id": 98765432,
                "login": "seller123",
                "feedback_count": 50,
                "feedback_reputation": 4.8
            },
            "favourite_count": 15,
            "view_count": 234,
            "source_field": "retained"
        });
        let params = serde_json::from_value(json!({
            "query": "nike shoes",
            "country": "DE",
            "page": 2,
            "per_page": 50,
            "order": "newest_first",
            "price_to": 80
        }))
        .unwrap();
        let response = json!({
            "data": {"pagination": {"total_pages": 20, "total_entries": 980}}
        });
        let row = build_dataset_item(&item, &params, &response);

        assert_eq!(row["id"], json!("1234567890"));
        assert_eq!(row["price_amount"], json!("45.00"));
        assert_eq!(row["price_currency"], json!("EUR"));
        assert_eq!(row["total_item_price"], json!("50.49"));
        assert_eq!(row["shipping_price"], json!("3.49"));
        assert_eq!(row["service_fee"], json!("2.00"));
        assert_eq!(row["brand_name"], json!("Nike"));
        assert_eq!(row["category_name"], json!("Shoes"));
        assert_eq!(row["size_name"], json!("EU 42"));
        assert_eq!(row["condition"], json!("Very good"));
        assert_eq!(
            row["image_url"],
            json!("https://images1.vinted.net/example.jpg")
        );
        assert_eq!(row["seller_login"], json!("seller123"));
        assert_eq!(row["request_query"], json!("nike shoes"));
        assert_eq!(row["request_country"], json!("DE"));
        assert_eq!(row["request_page"], json!(2));
        assert_eq!(row["total_pages"], json!(20));
        assert_eq!(row["total_entries"], json!(980));
        assert_eq!(row["source_field"], json!("retained"));
    }

    #[test]
    fn applies_nullish_fallbacks_and_stops_on_reported_pagination() {
        let row = build_dataset_item(
            &json!({
                "brand": {"title": ""},
                "brand_title": "Fallback brand",
                "price": "12.50",
                "photos": [{"url": ""}, {"url": "https://image.example/first.jpg"}],
                "seller": {},
                "user": {"id": 7, "login": "ignored"}
            }),
            &Map::new(),
            &json!({"pagination": {"total_entries": null, "total_items": 45}}),
        );
        assert_eq!(row["brand_name"], json!("Fallback brand"));
        assert_eq!(row["price_amount"], json!("12.50"));
        assert_eq!(row["image_url"], json!("https://image.example/first.jpg"));
        assert_eq!(row["seller_id"], Value::Null);
        assert_eq!(row["total_entries"], json!(45));
        assert!(has_no_next_page(Some(&json!({"has_next_page": false})), 2));
        assert!(has_no_next_page(Some(&json!({"total_pages": 2})), 2));
        assert!(!has_no_next_page(Some(&json!({"has_next_page": true})), 2));
    }
}
