use serde_json::{Map, Number, Value};

const PROPERTY_TYPE_LABELS: &[(i64, &str)] = &[
    (1, "House"),
    (2, "Townhouse"),
    (3, "Condo / Co-op"),
    (4, "Multi-family"),
    (5, "Land"),
    (6, "Other"),
    (7, "Manufactured"),
    (8, "Parking"),
];
const STATUS_LABELS: &[(i64, &str)] = &[
    (1, "Active"),
    (9, "All"),
    (130, "Pending"),
    (131, "Active and pending"),
];

pub(crate) fn property_listings(response: &Value) -> Vec<Value> {
    response
        .get("data")
        .and_then(|data| data.get("properties"))
        .and_then(Value::as_array)
        .or_else(|| response.get("properties").and_then(Value::as_array))
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn search_count(response: &Value) -> Option<f64> {
    first_number([response.pointer("/data/count"), response.get("count")])
}

pub(crate) fn dataset_item(
    property: &Value,
    params: &Map<String, Value>,
    search_index: usize,
) -> Value {
    let mut item = property.as_object().cloned().unwrap_or_default();
    let property_type = first_number([property.get("property_type")]);
    let request_status = first_number([params.get("status")]);

    insert(
        &mut item,
        "property_id",
        numeric_value(property.get("property_id")),
    );
    insert(
        &mut item,
        "listing_id",
        numeric_value(property.get("listing_id")),
    );
    insert(
        &mut item,
        "address",
        first_string([property.get("address")]),
    );
    insert(&mut item, "city", first_string([property.get("city")]));
    insert(&mut item, "state", first_string([property.get("state")]));
    insert(&mut item, "zip", first_string([property.get("zip")]));
    insert(&mut item, "price", numeric_value(property.get("price")));
    insert(&mut item, "beds", numeric_value(property.get("beds")));
    insert(&mut item, "baths", numeric_value(property.get("baths")));
    insert(&mut item, "sqft", numeric_value(property.get("sqft")));
    insert(
        &mut item,
        "lot_size",
        numeric_value(property.get("lot_size")),
    );
    insert(
        &mut item,
        "year_built",
        numeric_value(property.get("year_built")),
    );
    insert(&mut item, "property_type", number_value(property_type));
    insert(
        &mut item,
        "property_type_label",
        label(property_type, PROPERTY_TYPE_LABELS),
    );
    insert(&mut item, "status", first_string([property.get("status")]));
    insert(
        &mut item,
        "latitude",
        numeric_value(property.get("latitude")),
    );
    insert(
        &mut item,
        "longitude",
        numeric_value(property.get("longitude")),
    );
    insert(&mut item, "url", first_string([property.get("url")]));
    insert(
        &mut item,
        "mls_number",
        first_string([property.get("mls_number")]),
    );
    item.insert("request_search_index".to_owned(), Value::from(search_index));

    for field in [
        "region_id",
        "region_type",
        "market",
        "min_price",
        "max_price",
        "num_beds",
        "num_baths",
        "property_types",
        "status",
        "sold_within_days",
        "num_homes",
        "page",
    ] {
        let value = params.get(field).cloned().unwrap_or(Value::Null);
        item.insert(format!("request_{field}"), value);
    }
    item.insert(
        "request_status_label".to_owned(),
        label(request_status, STATUS_LABELS),
    );
    Value::Object(item)
}

fn insert(item: &mut Map<String, Value>, key: &str, value: Value) {
    item.insert(key.to_owned(), value);
}

fn first_string<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Value {
    for value in values.into_iter().flatten() {
        match value {
            Value::String(value) if !value.trim().is_empty() => {
                return Value::String(value.clone());
            }
            Value::Number(value) => {
                if value.as_f64().is_some_and(f64::is_finite) {
                    return Value::String(value.to_string());
                }
            }
            _ => {}
        }
    }
    Value::Null
}

fn first_number<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Option<f64> {
    values.into_iter().flatten().find_map(numeric_value_raw)
}

fn numeric_value(value: Option<&Value>) -> Value {
    number_value(value.and_then(numeric_value_raw))
}

fn numeric_value_raw(value: &Value) -> Option<f64> {
    let number = match value {
        Value::Number(value) => value.as_f64(),
        Value::String(value) if !value.trim().is_empty() => parse_js_number(value.trim()),
        _ => None,
    }?;
    number.is_finite().then_some(number)
}

fn parse_js_number(value: &str) -> Option<f64> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16)
            .ok()
            .map(|number| number as f64);
    }
    if let Some(binary) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        return u64::from_str_radix(binary, 2)
            .ok()
            .map(|number| number as f64);
    }
    if let Some(octal) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        return u64::from_str_radix(octal, 8)
            .ok()
            .map(|number| number as f64);
    }
    value.parse().ok()
}

fn number_value(value: Option<f64>) -> Value {
    let Some(value) = value else {
        return Value::Null;
    };
    let number = if value.fract() == 0.0
        && (i64::MIN as f64..9_223_372_036_854_775_808.0).contains(&value)
    {
        Number::from(value as i64)
    } else if value.fract() == 0.0 && (0.0..18_446_744_073_709_551_616.0).contains(&value) {
        Number::from(value as u64)
    } else {
        Number::from_f64(value).expect("finite number")
    };
    Value::Number(number)
}

fn label(value: Option<f64>, labels: &[(i64, &str)]) -> Value {
    let Some(value) = value else {
        return Value::Null;
    };
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        if let Some((_, label)) = labels.iter().find(|(id, _)| *id == value as i64) {
            return Value::String((*label).to_owned());
        }
    }
    Value::String(format!("Unknown ({})", number_string(value)))
}

fn number_string(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_wrapped_and_fallback_listings_and_count() {
        let wrapped = json!({"data":{"properties":[{"property_id":1}],"count":"12"},"properties":[{"property_id":2}],"count":3});
        assert_eq!(property_listings(&wrapped), vec![json!({"property_id":1})]);
        assert_eq!(search_count(&wrapped), Some(12.0));
        assert_eq!(
            property_listings(&json!({"properties":[{"property_id":2}]})),
            vec![json!({"property_id":2})]
        );
        assert!(property_listings(&json!({"data":{}})).is_empty());
        assert_eq!(search_count(&json!({"count":"Infinity"})), None);
    }

    #[test]
    fn normalizes_output_fields_and_preserves_unknown_upstream_fields() {
        let params = serde_json::from_value::<Map<String, Value>>(json!({
            "region_id":16163,"region_type":6,"market":"seattle","min_price":100000,
            "max_price":900000,"num_beds":2,"num_baths":1.5,"property_types":"1,2",
            "status":9,"sold_within_days":30,"num_homes":50,"page":1
        }))
        .unwrap();
        let item = dataset_item(
            &json!({
                "property_id":"12345","listing_id":"67890","address":"123 Main St","city":"Seattle",
                "state":"WA","zip":98101,"price":"850000","beds":"3","baths":"2.5",
                "sqft":"1800","lot_size":"5000","year_built":"1925","property_type":"1",
                "status":"Active","latitude":"47.6097","longitude":"-122.3331","url":"https://redfin.test/12345",
                "mls_number":123456,"future_field":"preserved"
            }),
            &params,
            1,
        );
        assert_eq!(item["property_id"], 12345);
        assert_eq!(item["zip"], "98101");
        assert_eq!(item["price"], 850000);
        assert_eq!(item["baths"], 2.5);
        assert_eq!(item["property_type_label"], "House");
        assert_eq!(item["request_status_label"], "All");
        assert_eq!(item["request_search_index"], 1);
        assert_eq!(item["future_field"], "preserved");
        assert_eq!(item["mls_number"], "123456");
    }

    #[test]
    fn invalid_numeric_strings_and_missing_request_filters_become_null() {
        let params = Map::from_iter([
            ("region_id".to_owned(), json!(16163)),
            ("region_type".to_owned(), json!(6)),
            ("market".to_owned(), json!("seattle")),
        ]);
        let item = dataset_item(
            &json!({"property_id":"Infinity","price":"-Infinity","latitude":"NaN"}),
            &params,
            0,
        );
        assert!(item["property_id"].is_null());
        assert!(item["price"].is_null());
        assert!(item["latitude"].is_null());
        assert!(item["request_num_homes"].is_null());
        assert!(item["request_status_label"].is_null());
    }
}
