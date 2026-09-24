use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use std::collections::HashSet;

const MAX_QUERIES: usize = 100;
const MAX_QUERY_LENGTH: usize = 200;
const SUGGESTION_TYPES: [&str; 3] = ["location", "hotel", "all"];

#[derive(Debug, PartialEq, Eq)]
pub struct AutocompleteRequest {
    pub queries: Vec<String>,
    pub common_params: Vec<(String, String)>,
}

impl AutocompleteRequest {
    pub fn params_for_query(&self, query: &str) -> Vec<(String, String)> {
        let mut params = Vec::with_capacity(self.common_params.len() + 1);
        params.push(("q".to_owned(), query.to_owned()));
        params.extend(self.common_params.iter().cloned());
        params
    }

    #[cfg(test)]
    pub fn param(&self, name: &str) -> Option<&str> {
        self.common_params
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value.as_str()))
    }
}

fn optional_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let value = value
        .as_str()
        .ok_or_else(|| anyhow!("{field} must be a string"))?;
    let cleaned = value.trim();
    if cleaned.is_empty() {
        return Ok(None);
    }
    if cleaned.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(cleaned.to_owned()))
}

fn code(value: Option<&Value>, field: &str, length: usize) -> Result<Option<String>> {
    let Some(value) = optional_string(value, field, 20)? else {
        return Ok(None);
    };
    if !value.is_ascii()
        || value.len() != length
        || !value.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        bail!("{field} must be a {length}-letter code");
    }
    Ok(Some(if length == 3 {
        value.to_ascii_uppercase()
    } else {
        value.to_ascii_lowercase()
    }))
}

fn raw_queries(value: Option<&Value>, field: &str) -> Result<Vec<Value>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(Vec::new());
    };
    if value.as_str() == Some("") {
        return Ok(Vec::new());
    }
    if let Some(queries) = value.as_array() {
        return Ok(queries.clone());
    }
    if let Some(queries) = value.as_str() {
        return Ok(queries
            .split(',')
            .map(|query| Value::String(query.to_owned()))
            .collect());
    }
    bail!("{field} must be an array of strings or a comma-separated string");
}

fn normalize_queries(input: &Value) -> Result<Vec<String>> {
    let mut candidates = raw_queries(input.get("queries"), "queries")?;
    if let Some(query) = optional_string(input.get("q"), "q", MAX_QUERY_LENGTH)? {
        candidates.push(Value::String(query));
    }

    let mut queries = Vec::new();
    let mut seen = HashSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let Some(query) = optional_string(
            Some(candidate),
            &format!("queries[{index}]"),
            MAX_QUERY_LENGTH,
        )?
        else {
            continue;
        };
        if seen.insert(query.to_lowercase()) {
            queries.push(query);
        }
    }

    if queries.is_empty() {
        bail!("At least one query is required in queries or q");
    }
    if queries.len() > MAX_QUERIES {
        bail!("A maximum of {MAX_QUERIES} unique queries is allowed per run");
    }
    Ok(queries)
}

pub fn build_request(input: &Value) -> Result<AutocompleteRequest> {
    let suggestion_type = optional_string(input.get("type"), "type", 20)?
        .unwrap_or_else(|| "all".to_owned())
        .to_lowercase();
    if !SUGGESTION_TYPES.contains(&suggestion_type.as_str()) {
        bail!("type must be one of: location, hotel, all");
    }

    let mut common_params = Vec::new();
    if let Some(gl) = code(input.get("gl"), "gl", 2)? {
        common_params.push(("gl".to_owned(), gl));
    }
    if let Some(hl) = code(input.get("hl"), "hl", 2)? {
        common_params.push(("hl".to_owned(), hl));
    }
    if let Some(currency) = code(input.get("currency"), "currency", 3)? {
        common_params.push(("currency".to_owned(), currency));
    }
    common_params.push(("type".to_owned(), suggestion_type));

    Ok(AutocompleteRequest {
        queries: normalize_queries(input)?,
        common_params,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn combines_alias_and_query_list_then_normalizes_values() {
        let request = build_request(&json!({
            "queries": [" Berlin ", "PARIS", "berlin", ""],
            "q": "Paris",
            "gl": " DE ",
            "hl": "EN",
            "currency": "eur",
            "type": "HOTEL"
        }))
        .unwrap();

        assert_eq!(request.queries, ["Berlin", "PARIS"]);
        assert_eq!(
            request.params_for_query("Berlin"),
            [
                ("q".to_owned(), "Berlin".to_owned()),
                ("gl".to_owned(), "de".to_owned()),
                ("hl".to_owned(), "en".to_owned()),
                ("currency".to_owned(), "EUR".to_owned()),
                ("type".to_owned(), "hotel".to_owned()),
            ]
        );
    }

    #[test]
    fn accepts_comma_separated_query_lists_and_type_defaults_to_all() {
        let request = build_request(&json!({"queries": "Berlin, Paris,, BERLIN"})).unwrap();

        assert_eq!(request.queries, ["Berlin", "Paris"]);
        assert_eq!(request.param("type"), Some("all"));
        assert_eq!(request.param("gl"), None);
    }

    #[test]
    fn accepts_the_single_query_alias() {
        let request = build_request(&json!({"q": "Berlin"})).unwrap();
        assert_eq!(request.queries, ["Berlin"]);
    }

    #[test]
    fn rejects_missing_queries_and_bad_types() {
        assert!(build_request(&json!({}))
            .unwrap_err()
            .to_string()
            .contains("At least one query"));
        assert!(build_request(&json!({"q": 4}))
            .unwrap_err()
            .to_string()
            .contains("q must be a string"));
        assert!(build_request(&json!({"q": "Berlin", "queries": 4}))
            .unwrap_err()
            .to_string()
            .contains("queries must be an array"));
        assert!(build_request(&json!({"q": "Berlin", "type": "places"}))
            .unwrap_err()
            .to_string()
            .contains("location, hotel, all"));
        assert!(build_request(&json!({"q": "Berlin", "gl": "germany"}))
            .unwrap_err()
            .to_string()
            .contains("2-letter code"));
    }

    #[test]
    fn rejects_too_many_queries_and_query_strings() {
        let queries: Vec<String> = (0..101).map(|index| format!("query {index}")).collect();
        assert!(build_request(&json!({"queries": queries}))
            .unwrap_err()
            .to_string()
            .contains("maximum of 100"));
        assert!(build_request(&json!({"q": "x".repeat(201)}))
            .unwrap_err()
            .to_string()
            .contains("200 characters or fewer"));
        assert!(build_request(&json!({"q": "é".repeat(201)}))
            .unwrap_err()
            .to_string()
            .contains("200 characters or fewer"));
    }
}
