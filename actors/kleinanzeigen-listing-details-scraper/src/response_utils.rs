use serde_json::{Map, Value, json};

const DETAIL_FIELDS: &[&str] = &[
    "title",
    "price",
    "price_numeric",
    "description",
    "location",
    "images",
    "seller",
    "attributes",
    "shipping",
    "posted_at",
    "categories",
];

fn object(value: &Value) -> Option<&Map<String, Value>> {
    value.as_object()
}

fn has_listing_id(value: &Value) -> bool {
    let Some(id) = value.get("id") else {
        return false;
    };
    match id {
        Value::String(id) => !id.trim().is_empty(),
        Value::Number(number) => number.as_f64().is_some_and(f64::is_finite),
        _ => false,
    }
}

fn has_detail_fields(value: &Value) -> bool {
    DETAIL_FIELDS
        .iter()
        .any(|field| value.get(*field).is_some_and(|value| !value.is_null()))
}

pub fn select_listing_detail(response: &Value) -> Option<Value> {
    let data = response.get("data");
    let mut candidates = Vec::with_capacity(8);
    if let Some(data) = data {
        candidates.push(data.get("listing"));
        candidates.push(data.get("result"));
        candidates.push(data.get("item"));
        candidates.push(Some(data));
    }
    candidates.push(response.get("listing"));
    candidates.push(response.get("result"));
    candidates.push(response.get("item"));
    candidates.push(Some(response));

    candidates
        .iter()
        .flatten()
        .find(|candidate| has_listing_id(candidate))
        .or_else(|| {
            candidates
                .iter()
                .flatten()
                .find(|candidate| has_detail_fields(candidate))
        })
        .cloned()
        .cloned()
}

fn optional_field(detail: &Value, field: &str) -> Value {
    detail
        .get(field)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn optional_array(detail: &Value, field: &str) -> Value {
    detail
        .get(field)
        .filter(|value| value.is_array())
        .cloned()
        .unwrap_or(Value::Null)
}

fn optional_object(detail: &Value, field: &str) -> Value {
    detail
        .get(field)
        .filter(|value| object(value).is_some())
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn build_dataset_item(detail: &Value, request_ad_id: &str, request_index: usize) -> Value {
    let id = optional_field(detail, "id");
    let id = if id.is_null() {
        json!(request_ad_id)
    } else {
        id
    };
    json!({
        "id": id,
        "title": optional_field(detail, "title"),
        "price": optional_field(detail, "price"),
        "price_numeric": optional_field(detail, "price_numeric"),
        "description": optional_field(detail, "description"),
        "location": optional_field(detail, "location"),
        "images": optional_array(detail, "images"),
        "seller": optional_object(detail, "seller"),
        "attributes": optional_object(detail, "attributes"),
        "shipping": optional_object(detail, "shipping"),
        "posted_at": optional_field(detail, "posted_at"),
        "categories": optional_array(detail, "categories"),
        "request_ad_id": request_ad_id,
        "request_index": request_index,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_dataset_item, select_listing_detail};
    use serde_json::{Value, json};

    #[test]
    fn selects_direct_root_and_wrapped_details() {
        for (response, id) in [
            (json!({"data":{"id":"1"}}), "1"),
            (json!({"data":{"listing":{"id":"2"}}}), "2"),
            (json!({"data":{"result":{"id":"3"}}}), "3"),
            (json!({"data":{"item":{"id":"4"}}}), "4"),
            (json!({"listing":{"id":"5"}}), "5"),
            (json!({"result":{"id":"6"}}), "6"),
            (json!({"item":{"id":"7"}}), "7"),
            (json!({"id":"8"}), "8"),
        ] {
            assert_eq!(select_listing_detail(&response).unwrap()["id"], id);
        }
        assert_eq!(
            select_listing_detail(&json!({"data":{"title":"Listing without an ID"}})).unwrap()["title"],
            "Listing without an ID"
        );
        assert!(select_listing_detail(&json!({})).is_none());
        assert!(
            select_listing_detail(&json!({"data":{"success":false,"message":"removed"}})).is_none()
        );
        assert!(select_listing_detail(&json!({"data":{"id":"  "}})).is_none());
    }

    #[test]
    fn projects_only_dataset_fields_and_uses_requested_id_when_needed() {
        let detail = json!({
            "id": "3451021120",
            "title": "Bike",
            "price": "120 €",
            "seller": {"name": "A"},
            "images": ["image"],
            "categories": ["bikes"],
            "raw_payload": {"unexpected": true}
        });
        let item = build_dataset_item(&detail, "3451021120", 0);
        assert_eq!(item["id"], "3451021120");
        assert_eq!(item["title"], "Bike");
        assert_eq!(item["price_numeric"], Value::Null);
        assert_eq!(item["images"], json!(["image"]));
        assert_eq!(item["seller"], json!({"name":"A"}));
        assert!(item.get("raw_payload").is_none());
        assert_eq!(item["request_index"], 0);

        let requested = build_dataset_item(&json!({"title":"Bike"}), "3451021120", 2);
        assert_eq!(requested["id"], "3451021120");
        assert_eq!(requested["request_index"], 2);
    }
}
