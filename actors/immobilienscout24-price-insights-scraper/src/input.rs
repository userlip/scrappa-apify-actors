use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashSet;

const MAX_LOCATIONS_PER_RUN: usize = 100;
const MAX_LOCATION_LENGTH: usize = 120;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PriceInsightsRequest {
    pub location: String,
    pub index: usize,
}

pub fn normalize_locations(input: Option<&Value>) -> Result<Vec<PriceInsightsRequest>> {
    let input = input.ok_or_else(|| anyhow!("Input is required"))?;
    if input.is_null() {
        return Err(anyhow!("Input is required"));
    }
    let values = match input.get("locations") {
        Some(Value::Array(values)) => values.clone(),
        Some(Value::String(locations)) => locations
            .split(',')
            .map(|location| Value::String(location.to_owned()))
            .collect(),
        Some(_) => {
            return Err(anyhow!(
                "locations must be an array of strings or a comma-separated string"
            ));
        }
        None => input.get("location").cloned().into_iter().collect(),
    };

    let mut seen = HashSet::new();
    let mut locations = Vec::new();

    for (index, value) in values.iter().enumerate() {
        let location = trim_javascript_whitespace(
            value
                .as_str()
                .ok_or_else(|| anyhow!("locations[{index}] must be a string"))?,
        );
        if location.is_empty() {
            continue;
        }
        if location.encode_utf16().count() > MAX_LOCATION_LENGTH {
            return Err(anyhow!(
                "locations[{index}] must be {MAX_LOCATION_LENGTH} characters or fewer"
            ));
        }

        if seen.insert(location.to_lowercase()) {
            locations.push(location.to_owned());
        }
    }

    if locations.is_empty() {
        return Err(anyhow!(
            "Provide at least one non-empty location in locations or location"
        ));
    }
    if locations.len() > MAX_LOCATIONS_PER_RUN {
        return Err(anyhow!(
            "A run can include at most {MAX_LOCATIONS_PER_RUN} unique locations"
        ));
    }

    Ok(locations
        .into_iter()
        .enumerate()
        .map(|(index, location)| PriceInsightsRequest { location, index })
        .collect())
}

pub(crate) fn trim_javascript_whitespace(value: &str) -> &str {
    value.trim_matches(|character| {
        matches!(
            character,
            '\u{0009}'
                | '\u{000A}'
                | '\u{000B}'
                | '\u{000C}'
                | '\u{000D}'
                | '\u{0020}'
                | '\u{00A0}'
                | '\u{1680}'
                | '\u{2000}'
                ..='\u{200A}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202F}'
                    | '\u{205F}'
                    | '\u{3000}'
                    | '\u{FEFF}'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn trims_and_deduplicates_locations_without_reordering_them() {
        let requests = normalize_locations(Some(&json!({
            "locations": [" Berlin ", "Munich", "berlin", ""]
        })))
        .unwrap();

        assert_eq!(
            requests,
            vec![
                PriceInsightsRequest {
                    location: "Berlin".to_owned(),
                    index: 0,
                },
                PriceInsightsRequest {
                    location: "Munich".to_owned(),
                    index: 1,
                },
            ]
        );
    }

    #[test]
    fn accepts_comma_separated_and_singular_compatibility_inputs() {
        let batch = normalize_locations(Some(&json!({"locations": "Berlin, Munich"}))).unwrap();
        assert_eq!(
            batch
                .iter()
                .map(|request| request.location.as_str())
                .collect::<Vec<_>>(),
            ["Berlin", "Munich"]
        );

        let singular = normalize_locations(Some(&json!({"location": "Hamburg"}))).unwrap();
        assert_eq!(singular[0].location, "Hamburg");
    }

    #[test]
    fn trims_the_same_unicode_whitespace_as_javascript() {
        let input = serde_json::json!({"location": "\u{feff} Berlin \u{feff}"});
        assert_eq!(
            normalize_locations(Some(&input)).unwrap()[0].location,
            "Berlin"
        );

        let input = serde_json::json!({"location": "\u{0085}Berlin\u{0085}"});
        assert_eq!(
            normalize_locations(Some(&input)).unwrap()[0].location,
            "\u{0085}Berlin\u{0085}"
        );
    }

    #[test]
    fn rejects_missing_invalid_and_empty_inputs_with_existing_messages() {
        assert_eq!(
            normalize_locations(None).unwrap_err().to_string(),
            "Input is required"
        );
        assert_eq!(
            normalize_locations(Some(&Value::Null))
                .unwrap_err()
                .to_string(),
            "Input is required"
        );
        assert!(normalize_locations(Some(&json!({})))
            .unwrap_err()
            .to_string()
            .contains("at least one non-empty location"));
        assert_eq!(
            normalize_locations(Some(&json!({"locations": 42})))
                .unwrap_err()
                .to_string(),
            "locations must be an array of strings or a comma-separated string"
        );
        assert_eq!(
            normalize_locations(Some(&json!({"locations": ["Berlin", 4]})))
                .unwrap_err()
                .to_string(),
            "locations[1] must be a string"
        );
    }

    #[test]
    fn enforces_length_in_utf16_code_units_and_caps_unique_locations() {
        assert!(normalize_locations(Some(&json!({
            "locations": ["x".repeat(121)]
        })))
        .unwrap_err()
        .to_string()
        .contains("120 characters or fewer"));
        assert!(normalize_locations(Some(&json!({
            "locations": ["😀".repeat(61)]
        })))
        .unwrap_err()
        .to_string()
        .contains("120 characters or fewer"));

        let too_many = (0..101)
            .map(|index| format!("Location {index}"))
            .collect::<Vec<_>>();
        assert!(normalize_locations(Some(&json!({"locations": too_many})))
            .unwrap_err()
            .to_string()
            .contains("at most 100 unique locations"));
    }
}
