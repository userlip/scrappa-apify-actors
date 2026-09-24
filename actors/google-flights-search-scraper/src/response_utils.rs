use std::collections::HashSet;

use serde_json::{json, Map, Value};

use crate::request_params::TripType;

pub fn get_flights(response: &Value) -> Vec<Value> {
    response
        .get("flights")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

pub fn build_unavailable_search_response(
    params: &Map<String, Value>,
    trip_type: TripType,
    message: &str,
    attempts: usize,
    response_time_ms: u128,
) -> Value {
    json!({
        "flights": [],
        "search_metadata": {
            "origin": params.get("origin"),
            "destination": params.get("destination"),
            "departure_date": params.get("departure_date"),
            "return_date": params.get("return_date").unwrap_or(&Value::Null),
            "trip_type": trip_type.as_str(),
            "upstream_available": false,
            "attempts": attempts,
            "response_time_ms": response_time_ms,
        },
        "warning": {
            "code": "UPSTREAM_TEMPORARILY_UNAVAILABLE",
            "message": message,
            "retryable": true,
        }
    })
}

pub fn build_flight_dataset_items(
    response: &Value,
    params: &Map<String, Value>,
    trip_type: TripType,
) -> Vec<Value> {
    let flights = get_flights(response);
    let metadata = response
        .get("search_metadata")
        .filter(|metadata| !metadata.is_null())
        .cloned()
        .unwrap_or_else(|| json!({}));

    flights
        .iter()
        .enumerate()
        .map(|(index, flight)| {
            let legs = array_field(flight, "legs");
            let outbound_legs = array_field(flight, "outbound_legs");
            let return_legs = array_field(flight, "return_legs");
            let derived_legs = if legs.is_empty() {
                outbound_legs
                    .iter()
                    .chain(return_legs.iter())
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                legs.clone()
            };
            let display_legs = if outbound_legs.is_empty() {
                &legs
            } else {
                &outbound_legs
            };
            let stop_segments = if !outbound_legs.is_empty() || !return_legs.is_empty() {
                Some(vec![outbound_legs.clone(), return_legs.clone()])
            } else if trip_type == TripType::RoundTrip {
                segment_flat_round_trip_legs(&derived_legs, params)
            } else {
                None
            };
            let first_leg = display_legs.first();
            let last_outbound_leg = display_legs.last();

            json!({
                "position": index + 1,
                "trip_type": trip_type.as_str(),
                "price": first_number(flight.get("price")),
                "currency": first_string(flight.get("currency"), params.get("currency")),
                "total_duration_minutes": first_number(flight.get("total_duration_minutes")),
                "stops": count_stops(&derived_legs, stop_segments.as_deref()),
                "airline_names": flight_airlines(flight, &derived_legs),
                "flight_numbers": leg_flight_numbers(&derived_legs),
                "departure_airport": first_string(
                    first_leg.and_then(|leg| leg.get("departure_airport")),
                    params.get("origin"),
                ),
                "arrival_airport": first_string(
                    last_outbound_leg.and_then(|leg| leg.get("arrival_airport")),
                    params.get("destination"),
                ),
                "departure_time": first_string(
                    first_leg.and_then(|leg| leg.get("departure_time")),
                    None,
                ),
                "arrival_time": first_string(
                    last_outbound_leg.and_then(|leg| leg.get("arrival_time")),
                    None,
                ),
                "booking_token": first_string(flight.get("booking_token"), None),
                "legs": derived_legs,
                "outbound_legs": outbound_legs,
                "return_legs": return_legs,
                "search_metadata": metadata,
                "request_origin": params.get("origin"),
                "request_destination": params.get("destination"),
                "request_departure_date": params.get("departure_date"),
                "request_return_date": params.get("return_date").unwrap_or(&Value::Null),
                "request_cabin_class": params.get("cabin_class").unwrap_or(&Value::Null),
                "request_max_stops": params.get("max_stops").unwrap_or(&Value::Null),
                "request_sort_by": params.get("sort_by").unwrap_or(&Value::Null),
                "request_airlines": params.get("airlines").unwrap_or(&Value::Null),
                "request_hl": params.get("hl").unwrap_or(&Value::Null),
                "request_gl": params.get("gl").unwrap_or(&Value::Null),
            })
        })
        .collect()
}

fn array_field(value: &Value, field: &str) -> Vec<Value> {
    value
        .get(field)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn first_string(first: Option<&Value>, second: Option<&Value>) -> Option<String> {
    [first, second].into_iter().flatten().find_map(|value| {
        value
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
    })
}

fn first_number(value: Option<&Value>) -> Option<Value> {
    let value = value?;
    if let Some(number) = value.as_number() {
        let numeric = number.as_f64().filter(|number| number.is_finite())?;
        return Some(value.clone()).filter(|_| numeric.is_finite());
    }
    let value = value.as_str()?;
    let cleaned = value.replace(',', "");
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return None;
    }
    let number = cleaned.parse::<f64>().ok()?;
    if !number.is_finite() {
        return None;
    }
    json_number(number)
}

fn count_stops(legs: &[Value], segment_legs: Option<&[Vec<Value>]>) -> Option<Value> {
    let explicit = legs
        .iter()
        .filter_map(|leg| first_number(leg.get("stops")))
        .filter_map(|value| value.as_f64())
        .collect::<Vec<_>>();
    if let Some(maximum) = explicit.into_iter().reduce(f64::max) {
        return json_number(maximum);
    }

    if let Some(segments) =
        segment_legs.filter(|segments| segments.iter().any(|segment| !segment.is_empty()))
    {
        let maximum = segments
            .iter()
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.len().saturating_sub(1))
            .max()
            .unwrap_or(0);
        return Some(json!(maximum));
    }
    if !legs.is_empty() {
        return Some(json!(legs.len().saturating_sub(1)));
    }
    None
}

fn json_number(number: f64) -> Option<Value> {
    if number.fract() == 0.0 && number >= i64::MIN as f64 && number <= i64::MAX as f64 {
        Some(json!(number as i64))
    } else {
        serde_json::Number::from_f64(number).map(Value::Number)
    }
}

fn segment_flat_round_trip_legs(
    legs: &[Value],
    params: &Map<String, Value>,
) -> Option<Vec<Vec<Value>>> {
    let destination = params.get("destination").and_then(Value::as_str)?;
    let outbound_end_index = legs
        .iter()
        .position(|leg| leg.get("arrival_airport").and_then(Value::as_str) == Some(destination))?;
    if outbound_end_index >= legs.len() - 1 {
        return None;
    }
    Some(vec![
        legs[..=outbound_end_index].to_vec(),
        legs[outbound_end_index + 1..].to_vec(),
    ])
}

fn leg_flight_numbers(legs: &[Value]) -> Vec<String> {
    legs.iter()
        .filter_map(|leg| first_string(leg.get("flight_number"), None))
        .collect()
}

fn flight_airlines(flight: &Value, legs: &[Value]) -> Vec<String> {
    let top_level_airline = first_string(flight.get("airline_name"), flight.get("airline"));
    let mut seen = HashSet::new();
    let mut airlines = Vec::new();
    if let Some(airline) = &top_level_airline {
        seen.insert(airline.clone());
        airlines.push(airline.clone());
    }
    for leg in legs {
        let leg_airline = first_string(
            leg.get("airline_name"),
            if top_level_airline.is_none() {
                leg.get("airline")
            } else {
                None
            },
        );
        if let Some(airline) = leg_airline.filter(|airline| seen.insert(airline.clone())) {
            airlines.push(airline);
        }
    }
    airlines
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn params() -> Map<String, Value> {
        serde_json::from_value(json!({
            "origin": "JFK",
            "destination": "LAX",
            "departure_date": "2026-09-15",
            "return_date": "2026-09-22",
            "cabin_class": "economy",
            "max_stops": "nonstop",
            "sort_by": "cheapest"
        }))
        .unwrap()
    }

    #[test]
    fn unavailable_response_marks_retryable_outage_and_preserves_search_details() {
        let mut search_params = params();
        search_params.remove("return_date");
        let response = build_unavailable_search_response(
            &search_params,
            TripType::OneWay,
            "Scrappa API error (503): Service Unavailable",
            5,
            32_100,
        );
        assert_eq!(response["flights"], json!([]));
        assert_eq!(response["search_metadata"]["upstream_available"], false);
        assert_eq!(response["search_metadata"]["attempts"], 5);
        assert_eq!(response["search_metadata"]["return_date"], Value::Null);
        assert_eq!(response["warning"]["retryable"], true);
    }

    #[test]
    fn creates_one_flattened_dataset_item_per_flight() {
        let response = json!({
            "flights": [{
                "price": "326",
                "currency": "USD",
                "total_duration_minutes": 374,
                "airline_name": "Delta",
                "booking_token": "token-1",
                "legs": [{
                    "departure_airport": "JFK",
                    "arrival_airport": "LAX",
                    "departure_time": "2026-09-15T08:00:00",
                    "arrival_time": "2026-09-15T11:14:00",
                    "airline": "DL",
                    "flight_number": "DL123",
                    "stops": 0
                }]
            }],
            "search_metadata": {"response_time_ms": 1200}
        });
        let items = build_flight_dataset_items(&response, &params(), TripType::RoundTrip);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["position"], 1);
        assert_eq!(items[0]["trip_type"], "round_trip");
        assert_eq!(items[0]["price"], 326);
        assert_eq!(items[0]["stops"], 0);
        assert_eq!(items[0]["airline_names"], json!(["Delta"]));
        assert_eq!(items[0]["flight_numbers"], json!(["DL123"]));
        assert_eq!(items[0]["booking_token"], "token-1");
        assert_eq!(items[0]["search_metadata"]["response_time_ms"], 1200);
        assert_eq!(items[0]["request_return_date"], "2026-09-22");
        assert!(items[0].get("raw_flight").is_none());
    }

    #[test]
    fn preserves_null_numeric_values_and_derives_round_trip_stops() {
        let missing = build_flight_dataset_items(
            &json!({"flights":[{"price":"   ","total_duration_minutes":"","legs":[{"stops":""}]}]}),
            &params(),
            TripType::OneWay,
        );
        assert_eq!(missing[0]["price"], Value::Null);
        assert_eq!(missing[0]["total_duration_minutes"], Value::Null);
        assert_eq!(missing[0]["stops"], 0);

        let segmented = build_flight_dataset_items(
            &json!({"flights":[{"outbound_legs":[{},{}],"return_legs":[{}]}]}),
            &params(),
            TripType::RoundTrip,
        );
        assert_eq!(segmented[0]["stops"], 1);
        assert_eq!(segmented[0]["legs"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn excludes_round_trip_return_leg_from_outbound_stops() {
        let items = build_flight_dataset_items(
            &json!({"flights":[{"legs":[
                {"departure_airport":"JFK","arrival_airport":"LAX"},
                {"departure_airport":"LAX","arrival_airport":"JFK"}
            ]}]}),
            &params(),
            TripType::RoundTrip,
        );
        assert_eq!(items[0]["stops"], 0);
        assert_eq!(items[0]["arrival_airport"], "JFK");
    }
}
