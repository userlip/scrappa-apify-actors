use anyhow::{bail, Result};
use serde_json::{json, Map, Value};

use crate::request_params::DirectionsRequest;

pub type DirectionsAlternative = Map<String, Value>;

pub fn extract_route_alternatives(response: &Value) -> Result<Vec<DirectionsAlternative>> {
    let response = response
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Scrappa response was not an object"))?;
    if response.get("success") == Some(&Value::Bool(false)) {
        bail!("{}", response_message(response));
    }

    if let Some(status) = non_empty_string(response.get("status")) {
        if !status.eq_ignore_ascii_case("OK") && !status.eq_ignore_ascii_case("SUCCESS") {
            bail!("{} (status: {status})", response_message(response));
        }
    }

    let alternatives = find_alternative_array(response)
        .ok_or_else(|| anyhow::anyhow!("Scrappa response did not include route alternatives"))?;
    if alternatives.is_empty() {
        bail!("Scrappa response included no route alternatives");
    }

    alternatives
        .iter()
        .map(|alternative| {
            alternative.as_object().cloned().ok_or_else(|| {
                anyhow::anyhow!("Scrappa response included a malformed route alternative")
            })
        })
        .collect()
}

fn find_alternative_array<'a>(response: &'a Map<String, Value>) -> Option<&'a Vec<Value>> {
    let mut candidates = vec![response.get("directions"), response.get("routes")];
    if let Some(data) = response.get("data") {
        if let Some(data_object) = data.as_object() {
            candidates.extend([
                data_object.get("directions"),
                data_object.get("routes"),
                data_object.get("routes_data"),
            ]);
        } else {
            candidates.push(Some(data));
        }
    }
    candidates.into_iter().flatten().find_map(Value::as_array)
}

fn response_message(response: &Map<String, Value>) -> String {
    non_empty_string(response.get("message"))
        .or_else(|| non_empty_string(response.get("error")))
        .unwrap_or_else(|| "Scrappa response reported a directions failure".to_owned())
}

fn non_empty_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub fn build_directions_dataset_rows(
    response: &Value,
    request: &DirectionsRequest,
) -> Result<Vec<Value>> {
    let alternatives = extract_route_alternatives(response)?;
    let response_mode = response
        .get("search_parameters")
        .and_then(Value::as_object)
        .and_then(|parameters| parameters.get("travel_mode"));

    Ok(alternatives
        .into_iter()
        .enumerate()
        .map(|(alternative_index, alternative)| {
            let mut row = alternative;
            row.insert("alternative_index".to_owned(), json!(alternative_index));
            row.insert("request_index".to_owned(), json!(request.index));
            row.insert("request_origin".to_owned(), json!(request.origin));
            row.insert("request_destination".to_owned(), json!(request.destination));
            row.insert("request_mode".to_owned(), json!(request.mode));
            row.insert("request_hl".to_owned(), json!(request.hl));
            if let Some(region) = &request.gl {
                row.insert("request_gl".to_owned(), json!(region));
            }
            if !row.contains_key("travel_mode") {
                if let Some(mode) = response_mode {
                    row.insert("travel_mode".to_owned(), mode.clone());
                }
            }
            if let Some(coordinates) = step_coordinates(&row) {
                row.insert("step_coordinates".to_owned(), Value::Array(coordinates));
            }
            Value::Object(row)
        })
        .collect())
}

fn step_coordinates(alternative: &DirectionsAlternative) -> Option<Vec<Value>> {
    let trips = alternative.get("trips")?.as_array()?;
    let coordinates: Vec<_> = trips
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|trip| trip.get("details").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_object)
        .filter_map(|detail| detail.get("gps_coordinates"))
        .filter(|coordinates| coordinates.is_object())
        .cloned()
        .collect();
    (!coordinates.is_empty()).then_some(coordinates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> DirectionsRequest {
        DirectionsRequest {
            origin: "A".to_owned(),
            destination: "B".to_owned(),
            mode: "walking".to_owned(),
            hl: "en".to_owned(),
            gl: Some("de".to_owned()),
            params: Vec::new(),
            index: 2,
        }
    }

    #[test]
    fn extracts_and_enriches_multiple_alternatives_with_stable_indexes() {
        let rows = build_directions_dataset_rows(
            &json!({
                "status": "OK",
                "search_parameters": { "travel_mode": "Walking" },
                "directions": [
                    { "travel_mode": "Walking", "via": "Main road", "distance": 100, "duration": 20, "trips": [{ "details": [{ "gps_coordinates": { "latitude": 1, "longitude": 2 } }] }] },
                    { "travel_mode": "Walking", "via": "Side road", "distance": 120, "duration": 24, "trips": [{ "details": [{ "title": "Turn", "gps_coordinates": { "latitude": 3, "longitude": 4 } }] }] }
                ]
            }),
            &request(),
        )
        .unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["alternative_index"], 0);
        assert_eq!(rows[1]["alternative_index"], 1);
        assert_eq!(rows[0]["request_index"], 2);
        assert_eq!(rows[0]["request_origin"], "A");
        assert_eq!(rows[0]["via"], "Main road");
        assert_eq!(
            rows[0]["step_coordinates"],
            json!([{ "latitude": 1, "longitude": 2 }])
        );
        assert_eq!(rows[1]["request_gl"], "de");
    }

    #[test]
    fn preserves_sparse_alternatives_without_inventing_optional_fields() {
        let mut route = request();
        route.gl = None;
        let rows = build_directions_dataset_rows(
            &json!({ "status": "OK", "directions": [{ "distance": 10 }] }),
            &route,
        )
        .unwrap();
        assert_eq!(
            rows[0],
            json!({
                "distance": 10,
                "alternative_index": 0,
                "request_index": 2,
                "request_origin": "A",
                "request_destination": "B",
                "request_mode": "walking",
                "request_hl": "en"
            })
        );
    }

    #[test]
    fn rejects_failures_empty_results_and_malformed_alternatives() {
        assert!(
            extract_route_alternatives(&json!({ "success": false, "message": "No route" }))
                .unwrap_err()
                .to_string()
                .contains("No route")
        );
        assert!(extract_route_alternatives(
            &json!({ "status": "ZERO_RESULTS", "message": "No route" })
        )
        .unwrap_err()
        .to_string()
        .contains("status: ZERO_RESULTS"));
        assert!(
            extract_route_alternatives(&json!({ "status": "OK", "directions": [] }))
                .unwrap_err()
                .to_string()
                .contains("no route alternatives")
        );
        assert!(extract_route_alternatives(
            &json!({ "status": "OK", "directions": [{ "distance": 1 }, null] })
        )
        .unwrap_err()
        .to_string()
        .contains("malformed"));
        assert!(extract_route_alternatives(&json!({ "status": "OK" }))
            .unwrap_err()
            .to_string()
            .contains("did not include route alternatives"));
    }

    #[test]
    fn checks_response_array_candidates_in_the_original_order() {
        let rows = build_directions_dataset_rows(
            &json!({ "directions": [], "routes": [{ "distance": 1 }] }),
            &request(),
        )
        .unwrap_err();
        assert!(rows.to_string().contains("no route alternatives"));
    }
}
