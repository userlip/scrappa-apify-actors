use std::collections::HashSet;

use anyhow::{bail, Result};
use serde_json::Value;

pub const MAX_ROUTES_PER_RUN: usize = 10;

const MAX_LOCATION_LENGTH: usize = 200;
const MAX_LANGUAGE_LENGTH: usize = 5;
const MAX_REGION_LENGTH: usize = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectionsRequest {
    pub origin: String,
    pub destination: String,
    pub mode: String,
    pub hl: String,
    pub gl: Option<String>,
    pub params: Vec<(String, String)>,
    pub index: usize,
}

pub fn build_directions_requests(input: Option<&Value>) -> Result<Vec<DirectionsRequest>> {
    let input = input.ok_or_else(|| anyhow::anyhow!("Input is required"))?;
    let mut route_inputs = Vec::new();

    if let Some(routes) = input.get("routes") {
        let routes = routes
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("routes must be an array of route objects"))?;
        for (index, route) in routes.iter().enumerate() {
            if !route.is_object() {
                bail!("routes[{index}] must be an object");
            }
            route_inputs.push((route, Some(index)));
        }
    }

    if route_inputs.is_empty() {
        let has_origin = input.get("origin").is_some();
        let has_destination = input.get("destination").is_some();
        if has_origin || has_destination {
            if !has_origin || !has_destination {
                bail!("origin and destination must be provided together");
            }
            route_inputs.push((input, None));
        }
    }

    if route_inputs.is_empty() {
        bail!("Provide at least one route in routes or origin and destination");
    }

    let mut seen = HashSet::new();
    let mut requests = Vec::new();
    for (route, route_index) in route_inputs {
        let request = normalize_route(route, route_index)?;
        let key = deduplication_key(&request);
        if seen.insert(key) {
            let mut request = request;
            request.index = requests.len();
            requests.push(request);
        }
    }

    if requests.len() > MAX_ROUTES_PER_RUN {
        bail!("A run can include at most {MAX_ROUTES_PER_RUN} unique routes");
    }

    Ok(requests)
}

fn normalize_route(route: &Value, route_index: Option<usize>) -> Result<DirectionsRequest> {
    let field = |name: &str| match route_index {
        Some(index) => format!("routes[{index}].{name}"),
        None => name.to_owned(),
    };

    let origin = clean_required_string(route.get("origin"), &field("origin"), MAX_LOCATION_LENGTH)?;
    let destination = clean_required_string(
        route.get("destination"),
        &field("destination"),
        MAX_LOCATION_LENGTH,
    )?;
    let mode = clean_optional_string(route.get("mode"), &field("mode"), 10)?
        .unwrap_or_else(|| "driving".to_owned())
        .to_lowercase();
    let mode = if mode == "cycling" {
        "bicycling".to_owned()
    } else {
        mode
    };
    if !["driving", "walking", "bicycling", "transit"].contains(&mode.as_str()) {
        bail!(
            "{} must be one of driving, walking, bicycling, cycling, or transit",
            field("mode")
        );
    }

    let hl = clean_optional_string(route.get("hl"), &field("hl"), MAX_LANGUAGE_LENGTH)?
        .unwrap_or_else(|| "en".to_owned())
        .to_lowercase();
    if !is_language_code(&hl) {
        bail!(
            "{} must be a language code such as en or de-DE",
            field("hl")
        );
    }

    let gl = clean_optional_string(route.get("gl"), &field("gl"), MAX_REGION_LENGTH)?
        .map(|region| region.to_lowercase());
    if gl.as_deref().is_some_and(|region| !is_region_code(region)) {
        bail!(
            "{} must be a two-letter country or region code",
            field("gl")
        );
    }

    let mut params = vec![
        ("origin".to_owned(), origin.clone()),
        ("destination".to_owned(), destination.clone()),
        ("mode".to_owned(), mode.clone()),
        ("hl".to_owned(), hl.clone()),
    ];
    if let Some(region) = &gl {
        params.push(("gl".to_owned(), region.clone()));
    }

    Ok(DirectionsRequest {
        origin,
        destination,
        mode,
        hl,
        gl,
        params,
        index: 0,
    })
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    let value = value
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("{field} must be a string"))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("{field} must not be empty");
    }
    if utf16_length(trimmed) > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(trimmed.to_owned())
}

fn clean_optional_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    let value = value
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("{field} must be a string"))?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if utf16_length(trimmed) > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(trimmed.to_owned()))
}

fn utf16_length(value: &str) -> usize {
    value.encode_utf16().count()
}

fn is_language_code(value: &str) -> bool {
    let parts: Vec<_> = value.split('-').collect();
    matches!(parts.as_slice(), [language] if language.len() == 2 && language.bytes().all(|byte| byte.is_ascii_lowercase()))
        || matches!(parts.as_slice(), [language, region] if language.len() == 2 && region.len() == 2 && language.bytes().chain(region.bytes()).all(|byte| byte.is_ascii_lowercase()))
}

fn is_region_code(value: &str) -> bool {
    value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_lowercase())
}

fn deduplication_key(request: &DirectionsRequest) -> String {
    [
        request.origin.trim().to_lowercase(),
        request.destination.trim().to_lowercase(),
        request.mode.trim().to_lowercase(),
        request.hl.trim().to_lowercase(),
        request
            .gl
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_lowercase(),
    ]
    .join("\0")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_singular_compatibility_input_and_omits_empty_region() {
        let requests = build_directions_requests(Some(&json!({
            "origin": " Berlin Hauptbahnhof ",
            "destination": " Brandenburg Gate ",
            "mode": " cycling ",
            "hl": " DE ",
            "gl": " "
        })))
        .unwrap();

        assert_eq!(requests[0].origin, "Berlin Hauptbahnhof");
        assert_eq!(requests[0].destination, "Brandenburg Gate");
        assert_eq!(requests[0].mode, "bicycling");
        assert_eq!(requests[0].hl, "de");
        assert_eq!(requests[0].gl, None);
        assert_eq!(requests[0].params.len(), 4);
    }

    #[test]
    fn normalizes_deduplicates_and_indexes_routes_in_first_seen_order() {
        let requests = build_directions_requests(Some(&json!({
            "routes": [
                { "origin": "A", "destination": "B" },
                { "origin": " a ", "destination": " b ", "mode": "DRIVING", "hl": "EN" },
                { "origin": "C", "destination": "D", "gl": "DE" }
            ]
        })))
        .unwrap();

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].index, 0);
        assert_eq!(requests[1].index, 1);
        assert_eq!(requests[1].gl.as_deref(), Some("de"));
    }

    #[test]
    fn route_batch_takes_precedence_over_schema_injected_singular_defaults() {
        let requests = build_directions_requests(Some(&json!({
            "routes": [
                { "origin": "Berlin Hauptbahnhof", "destination": "Brandenburg Gate", "mode": "walking" },
                { "origin": "Berlin Hauptbahnhof", "destination": "Brandenburg Gate", "mode": "driving" }
            ],
            "origin": "Times Square, New York, NY",
            "destination": "Central Park, New York, NY"
        })))
        .unwrap();

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].mode, "walking");
        assert_eq!(requests[1].mode, "driving");
    }

    #[test]
    fn rejects_malformed_incomplete_and_over_limit_input() {
        assert!(build_directions_requests(Some(&json!({})))
            .unwrap_err()
            .to_string()
            .contains("Provide at least one route"));
        assert!(build_directions_requests(Some(&json!({ "origin": "A" })))
            .unwrap_err()
            .to_string()
            .contains("provided together"));
        assert!(build_directions_requests(Some(
            &json!({ "routes": [{ "origin": "", "destination": "B" }] })
        ))
        .unwrap_err()
        .to_string()
        .contains("must not be empty"));
        assert!(build_directions_requests(Some(
            &json!({ "routes": [{ "origin": "A", "destination": "B", "mode": "flight" }] })
        ))
        .unwrap_err()
        .to_string()
        .contains("mode must be one of"));
        assert!(build_directions_requests(Some(
            &json!({ "routes": [{ "origin": "A", "destination": "B", "hl": "english" }] })
        ))
        .unwrap_err()
        .to_string()
        .contains("5 characters"));
        let routes: Vec<_> = (0..=MAX_ROUTES_PER_RUN)
            .map(|index| json!({ "origin": format!("A{index}"), "destination": "B" }))
            .collect();
        assert!(
            build_directions_requests(Some(&json!({ "routes": routes })))
                .unwrap_err()
                .to_string()
                .contains("at most 10")
        );
    }
}
