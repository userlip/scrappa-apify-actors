use anyhow::{bail, Result};
use serde_json::{Map, Number, Value};

const JAMEDA_BASE_URL: &str = "https://www.jameda.de";

pub fn get_jameda_doctors(response: &Value) -> Vec<Value> {
    response
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn non_null_field<'a>(object: &'a Value, key: &str) -> Option<&'a Value> {
    object.get(key).filter(|value| !value.is_null())
}

fn nested_non_null_field<'a>(object: &'a Value, parent: &str, key: &str) -> Option<&'a Value> {
    object
        .get(parent)
        .filter(|value| !value.is_null())
        .and_then(|value| value.get(key))
        .filter(|value| !value.is_null())
}

fn normalize_jameda_url(value: Option<&Value>) -> Value {
    let Some(value) = value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return Value::Null;
    };
    if value
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
    {
        return Value::String(value.to_owned());
    }
    if value
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
    {
        return Value::String(format!("https://{}", &value[7..]));
    }
    Value::String(format!(
        "{JAMEDA_BASE_URL}/{}",
        value.trim_start_matches('/')
    ))
}

fn parse_review_count(value: Option<&Value>) -> Value {
    let Some(value) = value
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return Value::Null;
    };
    let digits = value
        .replace('.', "")
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        return Value::Null;
    }
    if let Ok(integer) = digits.parse::<u64>() {
        return Value::Number(Number::from(integer));
    }
    digits
        .parse::<f64>()
        .ok()
        .and_then(Number::from_f64)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn nullable_field(object: &Value, key: &str) -> Value {
    non_null_field(object, key).cloned().unwrap_or(Value::Null)
}

pub fn build_jameda_doctor_dataset_item(
    doctor: &Value,
    params: &Map<String, Value>,
    response: &Value,
) -> Result<Value> {
    let Some(doctor) = doctor.as_object() else {
        bail!("Jameda doctor result must be an object");
    };
    let doctor_value = Value::Object(doctor.clone());
    let review_count = non_null_field(&doctor_value, "review_count")
        .or_else(|| nested_non_null_field(&doctor_value, "jameda_rating", "count"));
    let rating = non_null_field(&doctor_value, "rating")
        .or_else(|| nested_non_null_field(&doctor_value, "jameda_rating", "rating"));
    let mut item = doctor.clone();
    item.insert("name".to_owned(), nullable_field(&doctor_value, "name"));
    item.insert(
        "specialty".to_owned(),
        nullable_field(&doctor_value, "specialty"),
    );
    item.insert("url".to_owned(), nullable_field(&doctor_value, "url"));
    item.insert(
        "profile_url".to_owned(),
        normalize_jameda_url(non_null_field(&doctor_value, "url")),
    );
    item.insert("rating".to_owned(), rating.cloned().unwrap_or(Value::Null));
    item.insert(
        "review_count".to_owned(),
        review_count.cloned().unwrap_or(Value::Null),
    );
    item.insert(
        "review_count_number".to_owned(),
        parse_review_count(review_count),
    );
    item.insert(
        "address".to_owned(),
        nullable_field(&doctor_value, "address"),
    );
    item.insert(
        "image_url".to_owned(),
        nullable_field(&doctor_value, "image_url"),
    );
    item.insert(
        "jameda_rating".to_owned(),
        nullable_field(&doctor_value, "jameda_rating"),
    );
    for (output_key, input_key) in [
        ("request_q", "q"),
        ("request_loc", "loc"),
        ("request_page", "page"),
        ("request_per_page", "per_page"),
    ] {
        item.insert(
            output_key.to_owned(),
            params.get(input_key).cloned().unwrap_or(Value::Null),
        );
    }
    let meta = response.get("meta").filter(|value| !value.is_null());
    item.insert(
        "total_results".to_owned(),
        meta.and_then(|meta| non_null_field(meta, "total_results"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    item.insert(
        "total_pages".to_owned(),
        meta.and_then(|meta| non_null_field(meta, "total_pages"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    item.insert(
        "has_next_page".to_owned(),
        meta.and_then(|meta| non_null_field(meta, "has_next_page"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    Ok(Value::Object(item))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_doctors_from_the_scrappa_response_shape() {
        assert_eq!(
            get_jameda_doctors(&json!({"data":[{"name":"Dr. A"}]})),
            vec![json!({"name":"Dr. A"})]
        );
        assert!(get_jameda_doctors(&json!({"data":[]})).is_empty());
        assert!(get_jameda_doctors(&json!({})).is_empty());
        assert!(get_jameda_doctors(&json!({"data":{}})).is_empty());
    }

    #[test]
    fn builds_a_normalized_doctor_item_and_keeps_upstream_fields() {
        let params = serde_json::from_value(json!({
            "q":"Zahnarzt", "loc":"Berlin", "page":1, "per_page":28
        }))
        .unwrap();
        let item = build_jameda_doctor_dataset_item(
            &json!({
                "name":"Dr. med. Beispiel",
                "specialty":"Zahnarzt",
                "url":"/beispiel/zahnarzt/berlin",
                "rating":"1,2",
                "review_count":"1.234 Bewertungen",
                "address":"Beispielstr. 1, 10115 Berlin",
                "image_url":"https://www.jameda.de/example.jpg",
                "upstream_extra":{"kept":true}
            }),
            &params,
            &json!({"meta":{"total_results":120,"total_pages":5,"has_next_page":true}}),
        )
        .unwrap();

        assert_eq!(item["name"], "Dr. med. Beispiel");
        assert_eq!(
            item["profile_url"],
            "https://www.jameda.de/beispiel/zahnarzt/berlin"
        );
        assert_eq!(item["review_count_number"], 1234);
        assert_eq!(item["request_q"], "Zahnarzt");
        assert_eq!(item["request_loc"], "Berlin");
        assert_eq!(item["total_results"], 120);
        assert_eq!(item["total_pages"], 5);
        assert_eq!(item["upstream_extra"]["kept"], true);
    }

    #[test]
    fn uses_nested_rating_fallback_and_normalizes_http_profile_urls() {
        let params =
            serde_json::from_value(json!({"q":"Hausarzt", "page":2, "per_page":10})).unwrap();
        let item = build_jameda_doctor_dataset_item(
            &json!({
                "name":"Dr. A",
                "url":"http://www.jameda.de/a/hausarzt/hamburg",
                "jameda_rating":{"rating":"1,0", "count":"9"}
            }),
            &params,
            &json!({}),
        )
        .unwrap();
        assert_eq!(
            item["profile_url"],
            "https://www.jameda.de/a/hausarzt/hamburg"
        );
        assert_eq!(item["rating"], "1,0");
        assert_eq!(item["review_count"], "9");
        assert_eq!(item["review_count_number"], 9);
        assert_eq!(item["request_loc"], Value::Null);
    }

    #[test]
    fn treats_non_url_http_prefixes_as_relative_jameda_paths() {
        assert_eq!(
            normalize_jameda_url(Some(&json!("httpish-profile"))),
            "https://www.jameda.de/httpish-profile"
        );
    }
}
