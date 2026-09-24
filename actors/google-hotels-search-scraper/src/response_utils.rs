use anyhow::{anyhow, Result};
use serde_json::{Map, Value};

pub fn get_hotel_properties(response: &Value) -> &[Value] {
    if let Some(properties) = response.get("properties").and_then(Value::as_array) {
        return properties;
    }
    response
        .get("data")
        .and_then(|data| data.get("hotels"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

pub fn build_hotel_dataset_item(hotel: &Value, params: &Map<String, Value>) -> Result<Value> {
    let mut item = hotel
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("Google Hotels property must be an object"))?;

    set(&mut item, "name", hotel.get("name"));
    set(
        &mut item,
        "property_token",
        non_null(hotel.get("property_token")).or_else(|| non_null(hotel.get("entity_id"))),
    );
    set(&mut item, "entity_id", hotel.get("entity_id"));
    set(&mut item, "place_id", hotel.get("place_id"));
    set(
        &mut item,
        "latitude",
        hotel
            .get("gps_coordinates")
            .and_then(|coordinates| coordinates.get("latitude")),
    );
    set(
        &mut item,
        "longitude",
        hotel
            .get("gps_coordinates")
            .and_then(|coordinates| coordinates.get("longitude")),
    );
    set(
        &mut item,
        "rate_per_night_lowest",
        hotel
            .get("rate_per_night")
            .and_then(|rate| rate.get("lowest")),
    );
    set(
        &mut item,
        "rate_per_night_extracted_lowest",
        hotel
            .get("rate_per_night")
            .and_then(|rate| rate.get("extracted_lowest")),
    );
    set(
        &mut item,
        "total_rate_lowest",
        hotel.get("total_rate").and_then(|rate| rate.get("lowest")),
    );
    set(
        &mut item,
        "total_rate_extracted_lowest",
        hotel
            .get("total_rate")
            .and_then(|rate| rate.get("extracted_lowest")),
    );
    set(
        &mut item,
        "booking_link",
        non_null(hotel.get("property_link")).or_else(|| non_null(hotel.get("link"))),
    );
    item.insert(
        "price_sources_count".to_owned(),
        Value::from(
            hotel
                .get("prices")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
        ),
    );
    item.insert(
        "amenities_count".to_owned(),
        Value::from(
            hotel
                .get("amenities")
                .and_then(Value::as_array)
                .map_or(0, Vec::len),
        ),
    );

    for (input_field, output_field) in [
        ("q", "request_q"),
        ("check_in_date", "request_check_in_date"),
        ("check_out_date", "request_check_out_date"),
        ("adults", "request_adults"),
        ("children", "request_children"),
        ("currency", "request_currency"),
        ("gl", "request_gl"),
        ("hl", "request_hl"),
        ("sort_by", "request_sort_by"),
        ("next_page_token", "request_next_page_token"),
        ("property_token", "request_property_token"),
    ] {
        set(&mut item, output_field, params.get(input_field));
    }

    Ok(Value::Object(item))
}

fn non_null(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| !value.is_null())
}

fn set(item: &mut Map<String, Value>, field: &str, value: Option<&Value>) {
    item.insert(field.to_owned(), value.cloned().unwrap_or(Value::Null));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn supports_primary_legacy_and_empty_response_shapes() {
        let primary =
            json!({"properties": [{"name": "Hotel A"}], "data": {"hotels": [{"name": "Legacy"}]}});
        assert_eq!(
            get_hotel_properties(&primary),
            &[json!({"name": "Hotel A"})]
        );

        let legacy = json!({"data": {"hotels": [{"name": "Hotel B"}]}});
        assert_eq!(get_hotel_properties(&legacy), &[json!({"name": "Hotel B"})]);

        assert!(get_hotel_properties(&json!({})).is_empty());
        assert!(get_hotel_properties(
            &json!({"properties": [], "data": {"hotels": [{"name": "Hotel B"}]}})
        )
        .is_empty());
    }

    #[test]
    fn normalizes_dataset_fields_and_keeps_the_full_property() {
        let hotel = json!({
            "name": "Hotel Le Test",
            "entity_id": "entity-1",
            "place_id": "place-1",
            "gps_coordinates": {"latitude": 48.85, "longitude": 2.35},
            "rate_per_night": {"lowest": "$200", "extracted_lowest": 200},
            "total_rate": {"lowest": "$600", "extracted_lowest": 600},
            "property_link": "https://example.com/book",
            "prices": [{"source": "Booking"}],
            "amenities": ["Wi-Fi", "Pool"],
            "custom_upstream_field": {"kept": true}
        });
        let params = serde_json::from_value(json!({
            "q": "Paris",
            "check_in_date": "2026-08-01",
            "check_out_date": "2026-08-04",
            "adults": 2,
            "currency": "EUR"
        }))
        .unwrap();
        let item = build_hotel_dataset_item(&hotel, &params).unwrap();

        assert_eq!(item["name"], "Hotel Le Test");
        assert_eq!(item["property_token"], "entity-1");
        assert_eq!(item["latitude"], 48.85);
        assert_eq!(item["longitude"], 2.35);
        assert_eq!(item["rate_per_night_lowest"], "$200");
        assert_eq!(item["total_rate_extracted_lowest"], 600);
        assert_eq!(item["booking_link"], "https://example.com/book");
        assert_eq!(item["price_sources_count"], 1);
        assert_eq!(item["amenities_count"], 2);
        assert_eq!(item["request_q"], "Paris");
        assert_eq!(item["request_currency"], "EUR");
        assert_eq!(item["custom_upstream_field"]["kept"], true);
        assert_eq!(item["thumbnail"], Value::Null);
    }

    #[test]
    fn property_link_and_property_token_fall_back_to_legacy_fields() {
        let hotel = json!({
            "entity_id": "entity-2",
            "link": "https://example.com/legacy"
        });
        let item = build_hotel_dataset_item(&hotel, &Map::new()).unwrap();
        assert_eq!(item["property_token"], "entity-2");
        assert_eq!(item["booking_link"], "https://example.com/legacy");
        assert_eq!(item["name"], Value::Null);
        assert_eq!(item["price_sources_count"], 0);
        assert_eq!(item["amenities_count"], 0);
    }
}
