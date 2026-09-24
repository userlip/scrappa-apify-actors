use serde_json::{Map, Number, Value};

fn javascript_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64().filter(|number| number.is_finite()),
        Value::String(value) if !value.trim().is_empty() => {
            let trimmed = value.trim();
            let radix_value = if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                u128::from_str_radix(&trimmed[2..], 16)
                    .ok()
                    .map(|value| value as f64)
            } else if trimmed.starts_with("0b") || trimmed.starts_with("0B") {
                u128::from_str_radix(&trimmed[2..], 2)
                    .ok()
                    .map(|value| value as f64)
            } else if trimmed.starts_with("0o") || trimmed.starts_with("0O") {
                u128::from_str_radix(&trimmed[2..], 8)
                    .ok()
                    .map(|value| value as f64)
            } else {
                trimmed.parse::<f64>().ok()
            };
            radix_value.filter(|number| number.is_finite())
        }
        _ => None,
    }
}

fn number_value(number: f64) -> Value {
    if number.fract() == 0.0 && number >= 0.0 && number <= u64::MAX as f64 {
        let integer = number as u64;
        if integer as f64 == number {
            return Value::Number(Number::from(integer));
        }
    }
    if number.fract() == 0.0 && number >= i64::MIN as f64 && number < 0.0 {
        let integer = number as i64;
        if integer as f64 == number {
            return Value::Number(Number::from(integer));
        }
    }
    Number::from_f64(number)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn first_number(values: &[Option<&Value>]) -> Value {
    values
        .iter()
        .flatten()
        .find_map(|value| javascript_number(value).map(number_value))
        .unwrap_or(Value::Null)
}

fn number_to_string(value: &Number) -> String {
    if let Some(value) = value.as_i64() {
        return value.to_string();
    }
    if let Some(value) = value.as_u64() {
        return value.to_string();
    }
    value
        .as_f64()
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn first_string(values: &[Option<&Value>]) -> Value {
    values
        .iter()
        .flatten()
        .find_map(|value| match value {
            Value::String(value) if !value.trim().is_empty() => Some(Value::String(value.clone())),
            Value::Number(value) => Some(Value::String(number_to_string(value))),
            _ => None,
        })
        .unwrap_or(Value::Null)
}

fn nested_value<'a>(data: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    let value = data.get(key)?;
    match value.as_object().and_then(|value| value.get("value")) {
        Some(nested) => Some(nested),
        None => Some(value),
    }
}

pub fn get_redfin_valuation_data(response: &Value) -> Result<Map<String, Value>, String> {
    if response.is_null() {
        return Err("Cannot read properties of null (reading 'data')".to_owned());
    }
    if let Some(data) = response.get("data").and_then(Value::as_object) {
        return Ok(data.clone());
    }
    if let Some(object) = response.as_object() {
        return Ok(object.clone());
    }
    if let Some(values) = response.as_array() {
        return Ok(values
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect());
    }
    Ok(Map::new())
}

pub fn has_meaningful_valuation_data(data: &Map<String, Value>) -> bool {
    first_number(&[
        data.get("predictedValue"),
        data.get("predicted_value"),
        data.get("predictedValueLow"),
        data.get("predicted_value_low"),
        data.get("predictedValueHigh"),
        data.get("predicted_value_high"),
        data.get("lastSoldPrice"),
        data.get("last_sold_price"),
    ]) != Value::Null
}

pub fn build_redfin_valuation_dataset_item(
    response: &Value,
    request: &crate::request_params::RedfinValuationRequest,
) -> Result<Value, String> {
    let data = get_redfin_valuation_data(response)?;
    let comparables = data
        .get("comparables")
        .filter(|value| value.is_array())
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let comparables_count = comparables.as_array().map(Vec::len).unwrap_or_default();
    let mut item = data;
    item.insert("success".to_owned(), Value::Bool(true));
    item.insert(
        "property_id".to_owned(),
        Value::Number(request.property_id.clone()),
    );
    item.insert(
        "listing_id".to_owned(),
        request
            .listing_id
            .clone()
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    item.insert(
        "predicted_value".to_owned(),
        first_number(&[item.get("predictedValue"), item.get("predicted_value")]),
    );
    item.insert(
        "predicted_value_low".to_owned(),
        first_number(&[
            item.get("predictedValueLow"),
            item.get("predicted_value_low"),
        ]),
    );
    item.insert(
        "predicted_value_high".to_owned(),
        first_number(&[
            item.get("predictedValueHigh"),
            item.get("predicted_value_high"),
        ]),
    );
    item.insert(
        "last_sold_price".to_owned(),
        first_number(&[item.get("lastSoldPrice"), item.get("last_sold_price")]),
    );
    item.insert(
        "last_sold_date".to_owned(),
        first_string(&[item.get("lastSoldDate"), item.get("last_sold_date")]),
    );
    item.insert(
        "beds".to_owned(),
        first_number(&[item.get("numBeds"), item.get("beds")]),
    );
    item.insert(
        "baths".to_owned(),
        first_number(&[item.get("numBaths"), item.get("baths")]),
    );
    item.insert(
        "sqft".to_owned(),
        first_number(&[nested_value(&item, "sqFt"), item.get("sqft")]),
    );
    item.insert(
        "lot_size".to_owned(),
        first_number(&[nested_value(&item, "lotSize"), item.get("lot_size")]),
    );
    item.insert(
        "year_built".to_owned(),
        first_number(&[item.get("yearBuilt"), item.get("year_built")]),
    );
    item.insert(
        "comparables_count".to_owned(),
        Value::Number(Number::from(comparables_count)),
    );
    item.insert("comparables".to_owned(), comparables);
    item.insert(
        "request_index".to_owned(),
        Value::Number(Number::from(request.index)),
    );
    item.insert(
        "request_property_id".to_owned(),
        Value::Number(request.property_id.clone()),
    );
    item.insert(
        "request_listing_id".to_owned(),
        request
            .listing_id
            .clone()
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    item.insert(
        "request_url".to_owned(),
        request
            .url
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    Ok(Value::Object(item))
}

pub fn build_redfin_valuation_failure_item(
    request: &crate::request_params::RedfinValuationRequest,
    status: Value,
    message: String,
) -> Value {
    let mut item = Map::new();
    item.insert("success".to_owned(), Value::Bool(false));
    item.insert("status".to_owned(), status);
    item.insert("message".to_owned(), Value::String(message));
    item.insert(
        "property_id".to_owned(),
        Value::Number(request.property_id.clone()),
    );
    item.insert(
        "listing_id".to_owned(),
        request
            .listing_id
            .clone()
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    for field in [
        "predicted_value",
        "predicted_value_low",
        "predicted_value_high",
        "last_sold_price",
        "last_sold_date",
        "beds",
        "baths",
        "sqft",
        "lot_size",
        "year_built",
    ] {
        item.insert(field.to_owned(), Value::Null);
    }
    item.insert(
        "comparables_count".to_owned(),
        Value::Number(Number::from(0)),
    );
    item.insert("comparables".to_owned(), Value::Array(Vec::new()));
    item.insert(
        "request_index".to_owned(),
        Value::Number(Number::from(request.index)),
    );
    item.insert(
        "request_property_id".to_owned(),
        Value::Number(request.property_id.clone()),
    );
    item.insert(
        "request_listing_id".to_owned(),
        request
            .listing_id
            .clone()
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    item.insert(
        "request_url".to_owned(),
        request
            .url
            .clone()
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    Value::Object(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_params::build_redfin_valuation_requests;

    #[test]
    fn extracts_wrapped_data_and_normalizes_all_output_fields() {
        let response = serde_json::json!({"data": {
            "predictedValue": "850000", "predictedValueLow": 800000,
            "predictedValueHigh": "900000", "lastSoldPrice": "750000",
            "lastSoldDate": "2020-05-15", "numBeds": "3", "numBaths": "2.5",
            "sqFt": {"value":"1800"}, "lotSize": {"value":"4500"},
            "yearBuilt":"1920", "comparables":[{"address":"456 Oak Ave","price":875000}]
        }});
        let request = build_redfin_valuation_requests(&serde_json::json!({
            "property_id": 194191988, "listing_id": 207388793, "url":"https://redfin.test/home/194191988"
        })).unwrap().remove(0);
        let item = build_redfin_valuation_dataset_item(&response, &request).unwrap();
        assert_eq!(item["predicted_value"], 850000);
        assert_eq!(item["predicted_value_low"], 800000);
        assert_eq!(item["predicted_value_high"], 900000);
        assert_eq!(item["last_sold_price"], 750000);
        assert_eq!(item["last_sold_date"], "2020-05-15");
        assert_eq!(item["beds"], 3);
        assert_eq!(item["baths"], 2.5);
        assert_eq!(item["sqft"], 1800);
        assert_eq!(item["lot_size"], 4500);
        assert_eq!(item["year_built"], 1920);
        assert_eq!(item["comparables_count"], 1);
        assert_eq!(item["request_index"], 0);
        assert_eq!(item["request_url"], "https://redfin.test/home/194191988");
    }

    #[test]
    fn falls_back_to_top_level_fields_and_rejects_non_finite_values() {
        let response = serde_json::json!({"predicted_value":"0xff", "numBeds":"NaN"});
        let data = get_redfin_valuation_data(&response).unwrap();
        assert!(has_meaningful_valuation_data(&data));
        let request = build_redfin_valuation_requests(&serde_json::json!({"property_id":1}))
            .unwrap()
            .remove(0);
        let item = build_redfin_valuation_dataset_item(&response, &request).unwrap();
        assert_eq!(item["predicted_value"], 255);
        assert_eq!(item["beds"], Value::Null);
    }

    #[test]
    fn builds_failure_items_with_null_normalized_fields() {
        let request = build_redfin_valuation_requests(&serde_json::json!({
            "url":"https://www.redfin.com/home/194191988"
        }))
        .unwrap()
        .remove(0);
        let item = build_redfin_valuation_failure_item(
            &request,
            Value::Number(404.into()),
            "Valuation unavailable".to_owned(),
        );
        assert_eq!(item["success"], false);
        assert_eq!(item["status"], 404);
        assert_eq!(item["message"], "Valuation unavailable");
        assert_eq!(item["predicted_value"], Value::Null);
        assert_eq!(item["comparables_count"], 0);
        assert_eq!(item["comparables"], serde_json::json!([]));
        assert_eq!(item["request_property_id"], 194191988);
    }

    #[test]
    fn rejects_a_null_upstream_response_like_the_typescript_actor() {
        assert_eq!(
            get_redfin_valuation_data(&Value::Null).unwrap_err(),
            "Cannot read properties of null (reading 'data')"
        );
    }
}
