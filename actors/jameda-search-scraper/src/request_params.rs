use anyhow::{anyhow, bail, Result};
use serde_json::{json, Map, Value};
use std::collections::HashSet;

const MAX_PAGE: u64 = 500;
const DEFAULT_PAGE: u64 = 1;
const DEFAULT_PER_PAGE: u64 = 28;
const MAX_PER_PAGE: u64 = 28;
const DEFAULT_MAX_PAGES: u64 = 1;
const MAX_PAGES_PER_RUN: u64 = 2;
const MAX_SEARCHES_PER_RUN: usize = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct JamedaSearchPlan {
    pub base_params: Map<String, Value>,
    pub start_page: u64,
    pub per_page: u64,
    pub max_pages: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SearchLookup {
    query: String,
    location: Option<String>,
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

fn decode_input_string(value: &str) -> String {
    if !value.contains('%') {
        return value.to_owned();
    }

    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }
        if index + 2 >= bytes.len() {
            return value.to_owned();
        }
        let Some(high) = hex_digit(bytes[index + 1]) else {
            return value.to_owned();
        };
        let Some(low) = hex_digit(bytes[index + 2]) else {
            return value.to_owned();
        };
        decoded.push((high << 4) | low);
        index += 3;
    }

    String::from_utf8(decoded).unwrap_or_else(|_| value.to_owned())
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };

    let normalized = decode_input_string(value);
    let trimmed = normalized.trim_matches(is_js_whitespace);
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(trimmed.to_owned()))
}

fn clean_required_string(
    value: Option<&Value>,
    field: &str,
    min_length: usize,
    max_length: usize,
) -> Result<String> {
    let Some(value) = clean_string(value, field, max_length)? else {
        bail!("{field} is required");
    };
    if value.encode_utf16().count() < min_length {
        bail!("{field} must be at least {min_length} characters");
    }
    Ok(value)
}

fn clean_integer(value: Option<&Value>, field: &str, min: u64, max: u64) -> Result<Option<u64>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }

    let number = match value {
        Value::String(value) => {
            let value = value.trim_matches(is_js_whitespace);
            if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
                value.parse::<f64>().ok()
            } else {
                None
            }
        }
        Value::Number(value) => value.as_f64(),
        _ => None,
    };
    let Some(number) = number.filter(|number| number.is_finite() && number.fract() == 0.0) else {
        bail!("{field} must be an integer");
    };
    if number < min as f64 || number > max as f64 {
        bail!("{field} must be between {min} and {max}");
    }
    Ok(Some(number as u64))
}

fn build_pagination(input: &Value) -> Result<(u64, u64, u64)> {
    let start_page = clean_integer(input.get("page"), "page", 1, MAX_PAGE)?.unwrap_or(DEFAULT_PAGE);
    let per_page = clean_integer(input.get("per_page"), "per_page", 1, MAX_PER_PAGE)?
        .unwrap_or(DEFAULT_PER_PAGE);
    let max_pages = clean_integer(input.get("max_pages"), "max_pages", 1, MAX_PAGES_PER_RUN)?
        .unwrap_or(DEFAULT_MAX_PAGES);

    if start_page + max_pages - 1 > MAX_PAGE {
        bail!("page plus max_pages cannot exceed page 500");
    }
    Ok((start_page, per_page, max_pages))
}

fn has_input_value(value: Option<&Value>) -> bool {
    value.is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
}

fn search_lookups(input: &Value) -> Result<Vec<SearchLookup>> {
    let mut raw_lookups = Vec::new();
    if has_input_value(input.get("q")) {
        raw_lookups.push((input.get("q"), input.get("loc")));
    }

    if has_input_value(input.get("searches")) {
        let searches = input
            .get("searches")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("searches must be an array"))?;
        if searches.len() > MAX_SEARCHES_PER_RUN {
            bail!("searches must contain {MAX_SEARCHES_PER_RUN} items or fewer");
        }
        for search in searches {
            if !search.is_object() {
                bail!("Each searches item must be an object with q and optional loc");
            }
            raw_lookups.push((search.get("q"), search.get("loc")));
        }
    }

    if raw_lookups.is_empty() {
        bail!("Provide q or searches");
    }

    let mut seen = HashSet::new();
    let mut lookups = Vec::new();
    for (query, location) in raw_lookups {
        let query = clean_required_string(query, "q", 2, 255)?;
        let location = clean_string(location, "loc", 100)?;
        let lookup = SearchLookup { query, location };
        if seen.insert(lookup.clone()) {
            lookups.push(lookup);
        }
    }
    Ok(lookups)
}

pub fn build_jameda_search_plans(input: &Value) -> Result<Vec<JamedaSearchPlan>> {
    let lookups = search_lookups(input)?;
    let (start_page, per_page, max_pages) = build_pagination(input)?;

    Ok(lookups
        .into_iter()
        .map(|lookup| {
            let mut base_params = Map::new();
            base_params.insert("q".to_owned(), Value::String(lookup.query));
            if let Some(location) = lookup.location {
                base_params.insert("loc".to_owned(), Value::String(location));
            }
            base_params.insert("per_page".to_owned(), json!(per_page));
            JamedaSearchPlan {
                base_params,
                start_page,
                per_page,
                max_pages,
            }
        })
        .collect())
}

pub fn build_page_params(plan: &JamedaSearchPlan, page: u64) -> Map<String, Value> {
    let mut params = plan.base_params.clone();
    params.insert("page".to_owned(), json!(page));
    params
}

pub fn describe_jameda_search_request(plan: &JamedaSearchPlan) -> String {
    let query = plan
        .base_params
        .get("q")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let location = plan
        .base_params
        .get("loc")
        .and_then(Value::as_str)
        .map(|location| format!(" in {location}"))
        .unwrap_or_default();
    let page_description = if plan.max_pages == 1 {
        format!("page {}", plan.start_page)
    } else {
        format!(
            "pages {}-{}",
            plan.start_page,
            plan.start_page + plan.max_pages - 1
        )
    };
    format!(
        "\"{query}\"{location} ({page_description}, {} per page)",
        plan.per_page
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_default_and_batched_search_plans() {
        let plans = build_jameda_search_plans(&json!({
            "q": " Zahnarzt ",
            "loc": "Berlin",
            "searches": [
                {"q": "Zahnarzt", "loc": "Berlin"},
                {"q": "Hausarzt", "loc": "München"}
            ],
            "max_pages": 2
        }))
        .unwrap();

        assert_eq!(plans.len(), 2);
        assert_eq!(
            Value::Object(plans[0].base_params.clone()),
            json!({"q":"Zahnarzt", "loc":"Berlin", "per_page":28})
        );
        assert_eq!(plans[0].start_page, 1);
        assert_eq!(plans[0].per_page, 28);
        assert_eq!(plans[0].max_pages, 2);
        assert_eq!(plans[1].base_params["q"], "Hausarzt");
        assert_eq!(
            describe_jameda_search_request(&plans[1]),
            "\"Hausarzt\" in München (pages 1-2, 28 per page)"
        );
        assert_eq!(build_page_params(&plans[1], 2)["page"], 2);
    }

    #[test]
    fn accepts_encoded_input_and_preserves_ampersands_percent_and_malformed_encoding() {
        let plan = &build_jameda_search_plans(&json!({
            "q": "HNO%20Arzt",
            "loc": "M%C3%BCnchen",
            "page": "2",
            "per_page": "10",
            "max_pages": "2"
        }))
        .unwrap()[0];
        assert_eq!(plan.base_params["q"], "HNO Arzt");
        assert_eq!(plan.base_params["loc"], "München");
        assert_eq!(plan.start_page, 2);
        assert_eq!(plan.per_page, 10);
        assert_eq!(decode_input_string("Hals & Nase"), "Hals & Nase");
        assert_eq!(decode_input_string("100% privat"), "100% privat");
        assert_eq!(decode_input_string("bad%2"), "bad%2");
        assert_eq!(decode_input_string("bad%ff"), "bad%ff");
        assert_eq!(decode_input_string("100%2526 privat"), "100%26 privat");
    }

    #[test]
    fn validates_search_shape_and_pagination_bounds() {
        for (input, message) in [
            (json!({"searches":"Zahnarzt"}), "searches must be an array"),
            (
                json!({"searches":["Zahnarzt"]}),
                "Each searches item must be an object",
            ),
            (json!({"searches":[]}), "Provide q or searches"),
            (json!({"q":"a"}), "q must be at least 2 characters"),
            (
                json!({"q":"Zahnarzt", "page":0}),
                "page must be between 1 and 500",
            ),
            (
                json!({"q":"Zahnarzt", "per_page":29}),
                "per_page must be between 1 and 28",
            ),
            (
                json!({"q":"Zahnarzt", "max_pages":3}),
                "max_pages must be between 1 and 2",
            ),
            (
                json!({"q":"Zahnarzt", "page":500, "max_pages":2}),
                "page plus max_pages cannot exceed page 500",
            ),
        ] {
            assert!(
                build_jameda_search_plans(&input)
                    .unwrap_err()
                    .to_string()
                    .contains(message),
                "unexpected validation result for {input}"
            );
        }
        assert!(build_jameda_search_plans(&json!({"q":2}))
            .unwrap_err()
            .to_string()
            .contains("q must be a string"));
    }

    #[test]
    fn validates_deployable_schema_and_prefills() {
        let schema: Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/.actor/input_schema.json"
            ))
            .unwrap(),
        )
        .unwrap();
        assert!(schema.get("anyOf").is_none());
        assert_eq!(
            schema.pointer("/properties/searches/items/properties/q/title"),
            Some(&json!("Search Query"))
        );
        assert_eq!(
            schema.pointer("/properties/searches/items/properties/loc/title"),
            Some(&json!("Location"))
        );
        for field in ["q", "loc"] {
            assert!(schema
                .pointer(&format!(
                    "/properties/searches/items/properties/{field}/description"
                ))
                .and_then(Value::as_str)
                .is_some_and(|description| !description.is_empty()));
        }
        assert_eq!(
            schema["properties"]["searches"]["prefill"],
            json!([
                {"q":"Zahnarzt", "loc":"Berlin"},
                {"q":"Hausarzt", "loc":"München"}
            ])
        );
        assert_eq!(schema["properties"]["q"]["prefill"], "Zahnarzt");
        assert_eq!(schema["properties"]["loc"]["prefill"], "Berlin");
    }
}
