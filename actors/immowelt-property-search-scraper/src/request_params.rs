use anyhow::{anyhow, Result};
use serde_json::{json, Map, Number, Value};

const DEFAULT_LOCATION: &str = "Berlin";
const DEFAULT_TYPE: &str = "apartment-rent";
const DEFAULT_PAGE: u64 = 1;
const DEFAULT_PER_PAGE: u64 = 20;
const MAX_LOCATION_LENGTH: usize = 120;
const MAX_PER_PAGE: u64 = 50;
const MAX_PAGE: u64 = 10_000;
const PROPERTY_TYPES: [&str; 4] = ["apartment-rent", "apartment-buy", "house-rent", "house-buy"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchParams {
    pub location: String,
    pub property_type: String,
    pub page: u64,
    pub per_page: u64,
}

impl SearchParams {
    pub fn query_pairs(&self) -> [(&'static str, String); 4] {
        [
            ("location", self.location.clone()),
            ("type", self.property_type.clone()),
            ("page", self.page.to_string()),
            ("per_page", self.per_page.to_string()),
        ]
    }

    pub fn describe(&self) -> String {
        format!(
            "{} properties in {} (page {}, per_page {})",
            self.property_type, self.location, self.page, self.per_page
        )
    }

    pub fn output_metadata(&self) -> [(&'static str, Value); 4] {
        [
            ("request_location", Value::String(self.location.clone())),
            ("request_type", Value::String(self.property_type.clone())),
            ("request_page", Value::Number(Number::from(self.page))),
            (
                "request_per_page",
                Value::Number(Number::from(self.per_page)),
            ),
        ]
    }
}

pub fn build_search_params(input: Option<&Value>) -> Result<SearchParams> {
    let normalized = normalize_input(input);
    let location = required_string(normalized.get("location"), "location", MAX_LOCATION_LENGTH)?;
    let property_type = required_string(normalized.get("type"), "type", 40)?;
    if !PROPERTY_TYPES.contains(&property_type.as_str()) {
        return Err(anyhow!(
            "type must be one of: {}",
            PROPERTY_TYPES.join(", ")
        ));
    }

    Ok(SearchParams {
        location,
        property_type,
        page: integer(normalized.get("page"), "page", 1, MAX_PAGE)?,
        per_page: integer(normalized.get("per_page"), "per_page", 1, MAX_PER_PAGE)?,
    })
}

fn normalize_input(input: Option<&Value>) -> Map<String, Value> {
    let mut normalized = Map::new();
    let Some(input) = input.and_then(Value::as_object) else {
        return default_input();
    };

    for field in ["location", "type", "per_page", "page"] {
        if let Some(value) = input.get(field) {
            normalized.insert(field.to_owned(), trim_string(value));
        }
    }
    if !input.contains_key("type") {
        if let Some(value) = input.get("property_type") {
            normalized.insert("type".to_owned(), trim_string(value));
        }
    }
    if !input.contains_key("per_page") {
        if let Some(value) = input.get("limit") {
            normalized.insert("per_page".to_owned(), trim_string(value));
        }
    }

    if normalized.is_empty() {
        return default_input();
    }

    let mut defaults = default_input();
    defaults.extend(normalized);
    defaults
}

fn default_input() -> Map<String, Value> {
    json!({
        "location": DEFAULT_LOCATION,
        "type": DEFAULT_TYPE,
        "page": DEFAULT_PAGE,
        "per_page": DEFAULT_PER_PAGE,
    })
    .as_object()
    .expect("default input is an object")
    .clone()
}

fn trim_string(value: &Value) -> Value {
    match value {
        Value::String(value) => Value::String(js_trim(value).to_owned()),
        _ => value.clone(),
    }
}

fn required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    let Some(value) = value.and_then(Value::as_str) else {
        return Err(anyhow!("{field} must be a string"));
    };
    let value = js_trim(value);
    if value.is_empty() {
        return Err(anyhow!("{field} is required"));
    }
    if value.encode_utf16().count() > max_length {
        return Err(anyhow!("{field} must be {max_length} characters or fewer"));
    }
    Ok(value.to_owned())
}

fn integer(value: Option<&Value>, field: &str, min: u64, max: u64) -> Result<u64> {
    let number = match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(value)) if is_integer_string(js_trim(value)) => {
            js_trim(value).parse::<f64>().ok()
        }
        _ => None,
    };
    let Some(number) = number.filter(|number| number.is_finite() && number.fract() == 0.0) else {
        return Err(anyhow!("{field} must be an integer"));
    };
    if number < min as f64 || number > max as f64 {
        return Err(anyhow!("{field} must be between {min} and {max}"));
    }
    Ok(number as u64)
}

fn is_integer_string(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

pub fn js_trim(value: &str) -> &str {
    value.trim_matches(is_js_whitespace)
}

fn is_js_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'..='\u{000d}'
            | '\u{0020}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_missing_empty_and_unknown_input() {
        let expected = SearchParams {
            location: "Berlin".into(),
            property_type: "apartment-rent".into(),
            page: 1,
            per_page: 20,
        };
        assert_eq!(build_search_params(None).unwrap(), expected);
        assert_eq!(build_search_params(Some(&json!({}))).unwrap(), expected);
        assert_eq!(
            build_search_params(Some(&json!({"hello": "world"}))).unwrap(),
            expected
        );
    }

    #[test]
    fn trims_and_maps_legacy_aliases() {
        assert_eq!(
            build_search_params(Some(&json!({
                "location": " Berlin ",
                "property_type": " apartment-buy ",
                "page": "2",
                "limit": 25,
            })))
            .unwrap(),
            SearchParams {
                location: "Berlin".into(),
                property_type: "apartment-buy".into(),
                page: 2,
                per_page: 25,
            }
        );
    }

    #[test]
    fn canonical_fields_win_over_aliases_even_when_invalid() {
        let params = build_search_params(Some(&json!({
            "location": "Berlin",
            "type": "house-rent",
            "property_type": "apartment-buy",
            "per_page": 10,
            "limit": 25,
        })))
        .unwrap();
        assert_eq!(params.property_type, "house-rent");
        assert_eq!(params.per_page, 10);

        assert!(build_search_params(Some(&json!({
            "type": null,
            "property_type": "apartment-buy",
        })))
        .unwrap_err()
        .to_string()
        .contains("type must be a string"));
    }

    #[test]
    fn blank_and_null_known_inputs_override_defaults_and_fail_validation() {
        for (input, expected) in [
            (json!({"location": "  "}), "location is required"),
            (json!({"location": null}), "location must be a string"),
        ] {
            assert_eq!(
                build_search_params(Some(&input)).unwrap_err().to_string(),
                expected
            );
        }
    }

    #[test]
    fn validates_property_type_and_pagination_bounds() {
        for (input, expected) in [
            (
                json!({"type": "apartment"}),
                "type must be one of: apartment-rent, apartment-buy, house-rent, house-buy",
            ),
            (json!({"page": 0}), "page must be between 1 and 10000"),
            (json!({"per_page": 51}), "per_page must be between 1 and 50"),
            (json!({"page": "2.0"}), "page must be an integer"),
        ] {
            assert_eq!(
                build_search_params(Some(&input)).unwrap_err().to_string(),
                expected
            );
        }
    }

    #[test]
    fn counts_location_length_in_javascript_utf16_code_units() {
        let too_long = "😀".repeat(61);
        assert_eq!(
            build_search_params(Some(&json!({"location": too_long})))
                .unwrap_err()
                .to_string(),
            "location must be 120 characters or fewer"
        );
    }

    #[test]
    fn trims_javascript_bom_whitespace() {
        assert_eq!(js_trim("\u{feff} Berlin \u{feff}"), "Berlin");
    }
}
