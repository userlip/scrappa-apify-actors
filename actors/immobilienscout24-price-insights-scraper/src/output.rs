use crate::input::{trim_javascript_whitespace, PriceInsightsRequest};
use serde_json::{json, Number, Value};

pub fn build_price_insight_item(response: &Value, request: &PriceInsightsRequest) -> Option<Value> {
    if response.get("success") != Some(&Value::Bool(true)) {
        return None;
    }

    let prices = response.get("prices")?.as_object()?;
    let location = clean_string(response.get("location"))?;
    let geocode = clean_string(response.get("geocode"))?;
    let currency = clean_string(response.get("currency"))?;
    let apartment_rent = clean_positive_number(prices.get("apartment_rent_per_m2"))?;
    let apartment_buy = clean_positive_number(prices.get("apartment_buy_per_m2"))?;
    let house_rent = clean_positive_number(prices.get("house_rent_per_m2"))?;
    let house_buy = clean_positive_number(prices.get("house_buy_per_m2"))?;

    Some(json!({
        "location": location,
        "geocode": geocode,
        "currency": currency,
        "apartment_rent_per_m2": apartment_rent,
        "apartment_buy_per_m2": apartment_buy,
        "house_rent_per_m2": house_rent,
        "house_buy_per_m2": house_buy,
        "request_location": request.location,
        "request_index": request.index,
    }))
}

fn clean_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(trim_javascript_whitespace)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn clean_positive_number(value: Option<&Value>) -> Option<Number> {
    let number = match value? {
        Value::Number(number) => number.as_f64()?,
        Value::String(number) => parse_javascript_number(number)?,
        _ => return None,
    };
    if !number.is_finite() || number <= 0.0 {
        return None;
    }
    if number.fract() == 0.0 && number < i64::MAX as f64 {
        return Some(Number::from(number as i64));
    }
    Number::from_f64(number)
}

fn parse_javascript_number(value: &str) -> Option<f64> {
    let value = trim_javascript_whitespace(value);
    if value.is_empty() {
        return Some(0.0);
    }

    for (prefix, radix) in [("0x", 16), ("0b", 2), ("0o", 8)] {
        if value.starts_with(prefix) || value.starts_with(&prefix.to_ascii_uppercase()) {
            return u128::from_str_radix(&value[2..], radix)
                .ok()
                .map(|number| number as f64);
        }
    }
    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> PriceInsightsRequest {
        PriceInsightsRequest {
            location: "berlin".to_owned(),
            index: 2,
        }
    }

    fn complete_response() -> Value {
        json!({
            "success": true,
            "location": " Berlin ",
            "geocode": "1276003001",
            "currency": " EUR ",
            "prices": {
                "apartment_rent_per_m2": 12.72,
                "apartment_buy_per_m2": "4189.04",
                "house_rent_per_m2": 16.51,
                "house_buy_per_m2": 4394.87
            }
        })
    }

    #[test]
    fn maps_complete_snapshots_to_dataset_items() {
        assert_eq!(
            build_price_insight_item(&complete_response(), &request()).unwrap(),
            json!({
                "location": "Berlin",
                "geocode": "1276003001",
                "currency": "EUR",
                "apartment_rent_per_m2": 12.72,
                "apartment_buy_per_m2": 4189.04,
                "house_rent_per_m2": 16.51,
                "house_buy_per_m2": 4394.87,
                "request_location": "berlin",
                "request_index": 2
            })
        );
    }

    #[test]
    fn drops_incomplete_snapshots_and_invalid_benchmarks() {
        let mut response = complete_response();
        response["prices"]["house_buy_per_m2"] = Value::Null;
        assert!(build_price_insight_item(&response, &request()).is_none());

        for price in [json!(true), json!(0), json!("Infinity"), json!("no price")] {
            let mut response = complete_response();
            response["prices"]["apartment_rent_per_m2"] = price;
            assert!(build_price_insight_item(&response, &request()).is_none());
        }
    }

    #[test]
    fn accepts_javascript_number_string_radices_and_rejects_unusable_strings() {
        let mut response = complete_response();
        response["prices"]["apartment_rent_per_m2"] = json!("0x10");
        assert_eq!(
            build_price_insight_item(&response, &request()).unwrap()["apartment_rent_per_m2"],
            json!(16)
        );
        response["prices"]["apartment_rent_per_m2"] = json!(" ");
        assert!(build_price_insight_item(&response, &request()).is_none());
    }
}
