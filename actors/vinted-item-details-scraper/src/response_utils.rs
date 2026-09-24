use crate::request_params::VintedItemDetailsRequest;
use serde_json::{json, Map, Value};

const ITEM_DETAIL_FIELDS: &[&str] = &[
    "id",
    "title",
    "description",
    "price",
    "total_item_price",
    "shipping_price",
    "service_fee",
    "brand",
    "brand_title",
    "category",
    "size",
    "size_title",
    "color",
    "color_title",
    "condition",
    "condition_title",
    "status",
    "status_title",
    "url",
    "path",
    "image_url",
    "photo_url",
    "photo",
    "photos",
    "seller",
    "user",
    "favourite_count",
    "favorites_count",
    "view_count",
    "availability",
    "created_at",
    "updated_at",
];

pub fn get_vinted_item_details(response: &Value) -> Result<Map<String, Value>, String> {
    if response.get("success") == Some(&Value::Bool(false)) {
        return Err(scrappa_failure_message(response));
    }

    if let Some(item) = response
        .get("item")
        .filter(|item| is_vinted_item_details(item))
    {
        return Ok(item.as_object().expect("details is an object").clone());
    }
    if let Some(data) = response.get("data").and_then(Value::as_object) {
        if let Some(item) = data.get("item").filter(|item| is_vinted_item_details(item)) {
            return Ok(item.as_object().expect("details is an object").clone());
        }
    }
    if let Some(data) = response
        .get("data")
        .filter(|data| is_vinted_item_details(data))
    {
        return Ok(data.as_object().expect("details is an object").clone());
    }
    if is_vinted_item_details(response) {
        return Ok(response.as_object().expect("details is an object").clone());
    }

    Err("Scrappa response did not include Vinted item details".to_owned())
}

pub fn build_vinted_item_details_dataset_item(
    details: &Map<String, Value>,
    request: &VintedItemDetailsRequest,
) -> Value {
    let seller = details
        .get("seller")
        .and_then(Value::as_object)
        .or_else(|| details.get("user").and_then(Value::as_object));
    let mut item = details.clone();

    item.insert(
        "id".to_owned(),
        details
            .get("id")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or_else(|| json!(request.item_id)),
    );
    for field in ["title", "description", "url", "path"] {
        item.insert(
            field.to_owned(),
            details.get(field).cloned().unwrap_or(Value::Null),
        );
    }

    item.insert("image_url".to_owned(), first_photo_url(details));

    let price = details.get("price");
    let total_item_price = details.get("total_item_price");
    let shipping_price = details.get("shipping_price");
    let service_fee = details.get("service_fee");
    item.insert("price_amount".to_owned(), money_amount(price));
    item.insert("price_currency".to_owned(), money_currency(price));
    item.insert(
        "total_item_price".to_owned(),
        money_amount(total_item_price),
    );
    item.insert(
        "total_item_price_currency".to_owned(),
        money_currency(total_item_price),
    );
    item.insert("shipping_price".to_owned(), money_amount(shipping_price));
    item.insert(
        "shipping_price_currency".to_owned(),
        money_currency(shipping_price),
    );
    item.insert("service_fee".to_owned(), money_amount(service_fee));
    item.insert(
        "service_fee_currency".to_owned(),
        money_currency(service_fee),
    );
    item.insert(
        "brand_name".to_owned(),
        label(details.get("brand"))
            .or_else(|| non_null(details.get("brand_title")))
            .unwrap_or(Value::Null),
    );
    item.insert(
        "category_name".to_owned(),
        label(details.get("category")).unwrap_or(Value::Null),
    );
    item.insert(
        "size_name".to_owned(),
        label(details.get("size"))
            .or_else(|| non_null(details.get("size_title")))
            .unwrap_or(Value::Null),
    );
    item.insert(
        "color_name".to_owned(),
        label(details.get("color")).unwrap_or(Value::Null),
    );
    item.insert(
        "condition".to_owned(),
        ["condition", "condition_title", "status", "status_title"]
            .iter()
            .find_map(|field| non_null(details.get(*field)))
            .unwrap_or(Value::Null),
    );
    item.insert(
        "availability".to_owned(),
        details.get("availability").cloned().unwrap_or(Value::Null),
    );
    item.insert(
        "favourite_count".to_owned(),
        non_null(details.get("favourite_count"))
            .or_else(|| non_null(details.get("favorites_count")))
            .unwrap_or(Value::Null),
    );
    item.insert(
        "view_count".to_owned(),
        details.get("view_count").cloned().unwrap_or(Value::Null),
    );
    for (output, source) in [
        ("seller_id", "id"),
        ("seller_feedback_count", "feedback_count"),
        ("seller_feedback_reputation", "feedback_reputation"),
    ] {
        item.insert(
            output.to_owned(),
            seller
                .and_then(|seller| seller.get(source))
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    item.insert(
        "seller_login".to_owned(),
        seller
            .and_then(|seller| {
                seller
                    .get("login")
                    .filter(|value| !value.is_null())
                    .or_else(|| seller.get("username").filter(|value| !value.is_null()))
            })
            .cloned()
            .unwrap_or(Value::Null),
    );
    item.insert("request_item_id".to_owned(), json!(request.item_id));
    item.insert("request_country".to_owned(), json!(request.country));
    item.insert("request_index".to_owned(), json!(request.index));
    item.insert("request_success".to_owned(), Value::Bool(true));

    Value::Object(item)
}

pub fn build_vinted_item_details_error_item(
    request: &VintedItemDetailsRequest,
    error: &str,
) -> Value {
    json!({
        "id": request.item_id,
        "request_item_id": request.item_id,
        "request_country": request.country,
        "request_index": request.index,
        "request_success": false,
        "error_message": error,
    })
}

fn is_vinted_item_details(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    ITEM_DETAIL_FIELDS
        .iter()
        .any(|field| object.get(*field).is_some_and(|value| !value.is_null()))
}

fn scrappa_failure_message(response: &Value) -> String {
    let message = non_empty_string(response.get("message")).or_else(|| {
        response
            .get("data")
            .and_then(Value::as_object)
            .and_then(|data| non_empty_string(data.get("message")))
    });
    let status_code = response
        .get("status_code")
        .filter(|value| value.is_number() || value.is_string());
    let suffix = status_code
        .map(|status| format!(" (status_code: {})", js_string(status)))
        .unwrap_or_default();

    message
        .map(|message| format!("{message}{suffix}"))
        .unwrap_or_else(|| format!("Scrappa response reported failure{suffix}"))
}

fn non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn js_string(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

fn label(value: Option<&Value>) -> Option<Value> {
    let value = value?;
    if value.as_str().is_some_and(|value| !value.trim().is_empty()) {
        return Some(value.clone());
    }
    let object = value.as_object()?;
    ["title", "name"]
        .iter()
        .filter_map(|field| object.get(*field))
        .find(|value| value.as_str().is_some_and(|value| !value.is_empty()))
        .cloned()
}

fn money_amount(value: Option<&Value>) -> Value {
    let Some(value) = value else {
        return Value::Null;
    };
    if value.is_number() || value.is_string() {
        return value.clone();
    }
    value
        .as_object()
        .and_then(|value| value.get("amount"))
        .filter(|amount| !amount.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn money_currency(value: Option<&Value>) -> Value {
    let Some(object) = value.and_then(Value::as_object) else {
        return Value::Null;
    };
    object
        .get("currency")
        .filter(|value| value.is_string())
        .or_else(|| {
            object
                .get("currency_code")
                .filter(|value| value.is_string())
        })
        .cloned()
        .unwrap_or(Value::Null)
}

fn first_photo_url(details: &Map<String, Value>) -> Value {
    for value in [details.get("image_url"), details.get("photo_url")]
        .into_iter()
        .flatten()
    {
        if js_truthy(value) {
            return value.clone();
        }
    }
    if let Some(url) = details
        .get("photo")
        .and_then(Value::as_object)
        .and_then(|photo| photo.get("url"))
        .filter(|url| js_truthy(url))
    {
        return url.clone();
    }
    details
        .get("photos")
        .and_then(Value::as_array)
        .and_then(|photos| {
            photos.iter().find_map(|photo| {
                photo
                    .as_object()
                    .and_then(|photo| photo.get("url"))
                    .filter(|url| js_truthy(url))
            })
        })
        .cloned()
        .unwrap_or(Value::Null)
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

fn non_null(value: Option<&Value>) -> Option<Value> {
    value.filter(|value| !value.is_null()).cloned()
}

#[cfg(test)]
mod tests {
    use super::{
        build_vinted_item_details_dataset_item, build_vinted_item_details_error_item,
        get_vinted_item_details,
    };
    use crate::request_params::VintedItemDetailsRequest;
    use serde_json::{json, Value};

    fn request() -> VintedItemDetailsRequest {
        VintedItemDetailsRequest {
            item_id: "1234567890".to_owned(),
            country: "DE".to_owned(),
            index: 0,
        }
    }

    #[test]
    fn extracts_all_supported_response_shapes() {
        for response in [
            json!({"data": {"title": "Item A"}}),
            json!({"data": {"item": {"title": "Item B"}}}),
            json!({"item": {"title": "Item C"}}),
            json!({"path": "/items/123"}),
            json!({"data": {"brand_title": "Nike"}}),
        ] {
            assert!(get_vinted_item_details(&response).is_ok(), "{response}");
        }
    }

    #[test]
    fn reports_failed_and_empty_scrappa_envelopes() {
        assert!(get_vinted_item_details(&json!({
            "success": false,
            "data": {"item": {"id": 123}},
            "message": "Item not found",
            "status_code": 404
        }))
        .unwrap_err()
        .contains("Item not found (status_code: 404)"));
        assert!(get_vinted_item_details(&json!({
            "success": false,
            "data": {"message": "Nested item failure"},
            "status_code": 422
        }))
        .unwrap_err()
        .contains("Nested item failure (status_code: 422)"));
        assert!(
            get_vinted_item_details(&json!({"success": true, "data": {}}))
                .unwrap_err()
                .contains("did not include Vinted item details")
        );
        assert!(get_vinted_item_details(&json!({
            "success": false,
            "data": {"item": {"id": 123}}
        }))
        .unwrap_err()
        .contains("Scrappa response reported failure"));
    }

    #[test]
    fn creates_normalized_result_with_original_detail_payload() {
        let details = get_vinted_item_details(&json!({"data": {"item": {
            "id": "1234567890",
            "title": "Nike Air Max 90",
            "description": "Very good condition",
            "price": {"amount": "45.00", "currency_code": "EUR"},
            "total_item_price": {"amount": "50.49", "currency_code": "EUR"},
            "shipping_price": {"amount": "3.49", "currency_code": "EUR"},
            "service_fee": {"amount": "2.00", "currency_code": "EUR"},
            "brand_title": "Nike",
            "category": {"name": "Shoes"},
            "size_title": "EU 42",
            "status": "Very good",
            "availability": "available",
            "url": "https://www.vinted.de/items/1234567890-nike-air-max-90",
            "photo": {"url": "https://images1.vinted.net/example.jpg"},
            "user": {"id": 98765432, "login": "seller123", "feedback_count": 50, "feedback_reputation": 4.8},
            "favourite_count": 15,
            "view_count": 234
        }}}))
        .unwrap();
        let item = build_vinted_item_details_dataset_item(&details, &request());

        assert_eq!(item["id"], "1234567890");
        assert_eq!(item["price_amount"], "45.00");
        assert_eq!(item["price_currency"], "EUR");
        assert_eq!(item["total_item_price"], "50.49");
        assert_eq!(item["service_fee"], "2.00");
        assert_eq!(item["brand_name"], "Nike");
        assert_eq!(item["category_name"], "Shoes");
        assert_eq!(item["size_name"], "EU 42");
        assert_eq!(item["condition"], "Very good");
        assert_eq!(item["availability"], "available");
        assert_eq!(item["image_url"], "https://images1.vinted.net/example.jpg");
        assert_eq!(item["seller_login"], "seller123");
        assert_eq!(item["request_country"], "DE");
        assert_eq!(item["request_index"], 0);
        assert_eq!(item["request_success"], true);
        assert_eq!(item["user"]["login"], "seller123");
    }

    #[test]
    fn falls_back_to_requested_id_and_builds_item_failure_rows() {
        let details = get_vinted_item_details(&json!({"data": {"title": "Missing ID"}})).unwrap();
        assert_eq!(
            build_vinted_item_details_dataset_item(&details, &request())["id"],
            "1234567890"
        );
        assert_eq!(
            build_vinted_item_details_error_item(&request(), "Scrappa API error (404): Not found"),
            json!({
                "id": "1234567890",
                "request_item_id": "1234567890",
                "request_country": "DE",
                "request_index": 0,
                "request_success": false,
                "error_message": "Scrappa API error (404): Not found"
            })
        );
    }

    #[test]
    fn missing_normalized_fields_are_explicit_nulls() {
        let details = get_vinted_item_details(&json!({"item": {"path": "/items/123"}})).unwrap();
        let item = build_vinted_item_details_dataset_item(&details, &request());
        for key in [
            "description",
            "price_amount",
            "price_currency",
            "seller_login",
            "image_url",
        ] {
            assert_eq!(item[key], Value::Null);
        }
    }
}
