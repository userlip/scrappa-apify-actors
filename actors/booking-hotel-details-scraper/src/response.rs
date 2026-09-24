use serde_json::{json, Map, Value};

use crate::input::BookingHotelRequest;

pub fn get_booking_hotel_details(response: &Value) -> Map<String, Value> {
    if let Some(details) = response.get("data").and_then(Value::as_object) {
        return details.clone();
    }
    response.as_object().cloned().unwrap_or_default()
}

pub fn build_booking_hotel_dataset_item(
    mut details: Map<String, Value>,
    request: &BookingHotelRequest,
) -> Value {
    let hotel_schema = details.get("hotel_schema").and_then(Value::as_object);
    let aggregate_rating = details
        .get("aggregate_rating")
        .filter(|value| value.is_object())
        .or_else(|| {
            hotel_schema.and_then(|schema| {
                schema
                    .get("aggregateRating")
                    .filter(|value| value.is_object())
            })
        })
        .cloned()
        .unwrap_or(Value::Null);
    let title = first_string(
        details.get("title"),
        hotel_schema.and_then(|schema| schema.get("name")),
    );
    let canonical_url = first_string(details.get("canonical_url"), details.get("url"));

    details.insert("title".into(), title);
    details.insert("canonical_url".into(), canonical_url);
    details.insert(
        "hotel_schema".into(),
        details
            .get("hotel_schema")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    );
    details.insert("aggregate_rating".into(), aggregate_rating);
    details.insert(
        "json_ld".into(),
        details
            .get("json_ld")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    );
    details.insert(
        "parsed".into(),
        Value::Bool(details.get("parsed").is_some_and(js_truthy)),
    );
    details.insert("request_index".into(), json!(request.index));
    details.insert("request_input_type".into(), json!(request.input_type));
    details.insert("request_url".into(), param_or_null(request, "url"));
    details.insert("request_country".into(), param_or_null(request, "country"));
    details.insert("request_slug".into(), param_or_null(request, "slug"));
    details.insert("request_success".into(), Value::Bool(true));
    Value::Object(details)
}

pub fn build_booking_hotel_error_item(request: &BookingHotelRequest, message: &str) -> Value {
    json!({
        "request_index": request.index,
        "request_input_type": request.input_type,
        "request_url": param_or_null(request, "url"),
        "request_country": param_or_null(request, "country"),
        "request_slug": param_or_null(request, "slug"),
        "request_success": false,
        "error_message": message,
    })
}

fn param_or_null(request: &BookingHotelRequest, name: &str) -> Value {
    request.params.get(name).cloned().unwrap_or(Value::Null)
}

fn first_string(first: Option<&Value>, second: Option<&Value>) -> Value {
    [first, second]
        .into_iter()
        .flatten()
        .find(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
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
    use crate::input::build_booking_hotel_requests;
    use serde_json::json;

    #[test]
    fn chooses_data_payload_and_falls_back_to_top_level_response() {
        let response = json!({"success": true, "data": {"title": "Ritz Paris"}});
        assert_eq!(
            get_booking_hotel_details(&response).get("title"),
            Some(&json!("Ritz Paris"))
        );
        let response = json!({"title": "Top level", "parsed": true});
        assert_eq!(
            get_booking_hotel_details(&response).get("title"),
            Some(&json!("Top level"))
        );
    }

    #[test]
    fn builds_normalized_dataset_item_without_dropping_upstream_fields() {
        let request =
            &build_booking_hotel_requests(&json!({"country": "fr", "slug": "ritz-paris"})).unwrap()
                [0];
        let details = get_booking_hotel_details(&json!({"data": {
            "canonical_url": "https://www.booking.com/hotel/fr/ritz-paris.html",
            "hotel_schema": {
                "@type": "Hotel",
                "name": "Ritz Paris",
                "aggregateRating": {"ratingValue": "9.4"}
            },
            "json_ld": [{"@type": "Hotel"}],
            "parsed": true,
            "upstream_extra": "preserved"
        }}));
        let item = build_booking_hotel_dataset_item(details, request);
        assert_eq!(item["title"], "Ritz Paris");
        assert_eq!(
            item["canonical_url"],
            "https://www.booking.com/hotel/fr/ritz-paris.html"
        );
        assert_eq!(item["aggregate_rating"]["ratingValue"], "9.4");
        assert_eq!(item["json_ld"][0]["@type"], "Hotel");
        assert_eq!(item["upstream_extra"], "preserved");
        assert_eq!(item["request_input_type"], "country_slug");
        assert_eq!(item["request_index"], 0);
        assert_eq!(item["request_country"], "fr");
        assert_eq!(item["request_slug"], "ritz-paris");
        assert!(item["request_url"].is_null());
        assert_eq!(item["request_success"], true);
    }

    #[test]
    fn retains_explicit_rating_and_null_fields_and_builds_error_rows() {
        let request = &build_booking_hotel_requests(&json!({
            "url": "https://www.booking.com/hotel/fr/ritz-paris.html"
        }))
        .unwrap()[0];
        let details = get_booking_hotel_details(&json!({
            "title": "Hotel Example",
            "aggregate_rating": {"ratingValue": "8.8"},
            "parsed": "yes"
        }));
        let item = build_booking_hotel_dataset_item(details, request);
        assert_eq!(item["aggregate_rating"]["ratingValue"], "8.8");
        assert_eq!(item["hotel_schema"], Value::Null);
        assert_eq!(item["json_ld"], Value::Null);
        assert_eq!(item["parsed"], true);
        assert_eq!(item["request_url"], request.params["url"]);

        let error =
            build_booking_hotel_error_item(request, "Scrappa API error (422): Invalid request");
        assert_eq!(error["request_success"], false);
        assert_eq!(
            error["error_message"],
            "Scrappa API error (422): Invalid request"
        );
        assert!(error["request_country"].is_null());
        assert!(error["request_slug"].is_null());
    }
}
