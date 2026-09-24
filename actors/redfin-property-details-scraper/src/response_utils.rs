use serde_json::{json, Map, Number, Value};

use crate::request_params::RedfinPropertyDetailsRequest;

pub(crate) fn get_redfin_property_details(response: &Value) -> Option<Map<String, Value>> {
    let data = response.get("data");
    if let Some(properties) = data.and_then(Value::as_array) {
        return properties.first().and_then(Value::as_object).cloned();
    }
    if let Some(property) = data.and_then(Value::as_object) {
        return Some(property.clone());
    }
    response.get("property").and_then(Value::as_object).cloned()
}

fn first_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().find_map(|value| {
        let value = (*value)?;
        match value {
            Value::String(value) if !value.trim().is_empty() => Some(value.clone()),
            Value::Number(value) if value.as_f64().is_some_and(f64::is_finite) => {
                Some(value.to_string())
            }
            _ => None,
        }
    })
}

fn first_number(values: &[Option<&Value>]) -> Option<Number> {
    values.iter().find_map(|value| {
        let value = (*value)?;
        let number = match value {
            Value::Number(value) => value.as_f64().filter(|number| number.is_finite()),
            Value::String(value) if !value.trim().is_empty() => value
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|number| number.is_finite()),
            _ => None,
        }?;
        if number.fract() == 0.0 && number >= 0.0 && number <= u64::MAX as f64 {
            Some(Number::from(number as u64))
        } else if number.fract() == 0.0 && number >= i64::MIN as f64 {
            Some(Number::from(number as i64))
        } else {
            Number::from_f64(number)
        }
    })
}

pub(crate) fn build_redfin_property_details_dataset_item(
    property: Map<String, Value>,
    request: &RedfinPropertyDetailsRequest,
) -> Value {
    let mut item = property;
    let requested_id = json!(request.params.property_id);
    item.insert(
        "property_id".to_owned(),
        first_number(&[item.get("property_id"), Some(&requested_id)])
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );

    for field in [
        "address",
        "city",
        "state",
        "country",
        "price_label",
        "status_label",
        "url",
        "description",
    ] {
        let value = first_string(&[item.get(field)])
            .map(Value::String)
            .unwrap_or(Value::Null);
        item.insert(field.to_owned(), value);
    }
    item.insert(
        "zip".to_owned(),
        first_string(&[item.get("zip")])
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    for field in [
        "price",
        "beds",
        "baths",
        "sqft",
        "lot_size",
        "year_built",
        "property_type",
        "status",
        "latitude",
        "longitude",
    ] {
        let value = first_number(&[item.get(field)])
            .map(Value::Number)
            .unwrap_or(Value::Null);
        item.insert(field.to_owned(), value);
    }
    if !item.get("photos").is_some_and(Value::is_array) {
        item.insert("photos".to_owned(), Value::Array(Vec::new()));
    }
    item.insert("request_property_index".to_owned(), json!(request.index));
    item.insert(
        "request_property_id".to_owned(),
        json!(request.params.property_id),
    );
    item.insert("request_input".to_owned(), request.input.clone());
    item.insert("request_source".to_owned(), json!(request.source));
    Value::Object(item)
}

pub(crate) fn build_redfin_property_error_dataset_item(
    request: &RedfinPropertyDetailsRequest,
    message: &str,
    status_code: Option<u16>,
) -> Value {
    json!({
        "success": false,
        "property_id": request.params.property_id,
        "request_property_index": request.index,
        "request_property_id": request.params.property_id,
        "request_input": request.input,
        "request_source": request.source,
        "error": message,
        "status_code": status_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_params::build_redfin_property_details_requests;
    use serde_json::json;

    #[test]
    fn extracts_data_and_property_fallbacks() {
        let property = json!({"property_id": 60791456, "address": "1549 Ely St"});
        assert_eq!(
            get_redfin_property_details(&json!({"data": property})),
            property.as_object().cloned()
        );
        assert_eq!(
            get_redfin_property_details(&json!({"data": [property.clone()]})),
            property.as_object().cloned()
        );
        assert_eq!(
            get_redfin_property_details(&json!({"property": property.clone()})),
            property.as_object().cloned()
        );
        assert_eq!(get_redfin_property_details(&json!({"data": []})), None);
    }

    #[test]
    fn normalizes_property_fields_and_preserves_unmapped_upstream_fields() {
        let request = build_redfin_property_details_requests(&json!({
            "url": "https://redfin.com/home/60791456"
        }))
        .unwrap()
        .remove(0);
        let property = json!({
            "property_id": "60791456",
            "address": "1549 Ely St",
            "zip": 38106,
            "price": "125000",
            "beds": "3",
            "photos": [{"url": "https://example.com/photo.jpg"}],
            "description": "Property description",
            "upstream_only": {"kept": true}
        });
        let item = build_redfin_property_details_dataset_item(
            property.as_object().unwrap().clone(),
            &request,
        );

        assert_eq!(item["property_id"], json!(60791456));
        assert_eq!(item["zip"], json!("38106"));
        assert_eq!(item["price"], json!(125000));
        assert_eq!(item["beds"], json!(3));
        assert_eq!(item["photos"], property["photos"]);
        assert_eq!(item["request_property_index"], json!(0));
        assert_eq!(
            item["request_input"],
            json!("https://redfin.com/home/60791456")
        );
        assert_eq!(item["request_source"], json!("url"));
        assert_eq!(item["upstream_only"], json!({"kept": true}));
        assert_eq!(item["city"], Value::Null);
    }

    #[test]
    fn builds_non_charged_per_property_error_rows() {
        let request = build_redfin_property_details_requests(&json!({"property_id": 60791456}))
            .unwrap()
            .remove(0);
        assert_eq!(
            build_redfin_property_error_dataset_item(&request, "Property not found", Some(404)),
            json!({
                "success": false,
                "property_id": 60791456,
                "request_property_index": 0,
                "request_property_id": 60791456,
                "request_input": 60791456,
                "request_source": "property_id",
                "error": "Property not found",
                "status_code": 404
            })
        );
    }
}
