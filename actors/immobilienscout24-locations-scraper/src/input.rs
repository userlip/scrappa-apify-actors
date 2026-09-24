use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};
use serde_json::Value;

const DEFAULT_LIMIT: usize = 10;
const MAX_QUERIES: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationRequest {
    pub query: String,
    pub limit: usize,
}

pub fn build_location_requests(input: &Value) -> Result<Vec<LocationRequest>> {
    let object = input
        .as_object()
        .ok_or_else(|| anyhow!("Input must be an object"))?;
    let limit = parse_limit(object.get("limit"))?;
    let raw_queries = match object.get("queries") {
        Some(Value::Array(queries)) => queries.iter().collect::<Vec<_>>(),
        Some(_) => bail!("queries must be an array of strings"),
        None => object.get("query").into_iter().collect::<Vec<_>>(),
    };

    let mut seen = HashSet::new();
    let mut queries = Vec::new();
    for raw_query in raw_queries {
        let query = raw_query
            .as_str()
            .ok_or_else(|| anyhow!("Each query must be a string"))?
            .trim();
        if query.is_empty() {
            bail!("Queries must not be empty");
        }
        if query.encode_utf16().count() > 120 {
            bail!("Each query must be at most 120 characters");
        }

        if seen.insert(query.to_lowercase()) {
            queries.push(query.to_owned());
        }
    }

    if queries.is_empty() {
        bail!("Provide at least one location in queries or query");
    }
    if queries.len() > MAX_QUERIES {
        bail!("A run supports at most {MAX_QUERIES} unique queries");
    }

    Ok(queries
        .into_iter()
        .map(|query| LocationRequest { query, limit })
        .collect())
}

fn parse_limit(value: Option<&Value>) -> Result<usize> {
    let Some(value) = value else {
        return Ok(DEFAULT_LIMIT);
    };
    let number = value
        .as_f64()
        .ok_or_else(|| anyhow!("limit must be an integer"))?;
    if number.fract() != 0.0 {
        bail!("limit must be an integer");
    }
    if !(1.0..=20.0).contains(&number) {
        bail!("limit must be between 1 and 20");
    }
    Ok(number as usize)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{build_location_requests, LocationRequest};

    #[test]
    fn normalizes_and_deduplicates_batch_queries_in_input_order() {
        assert_eq!(
            build_location_requests(&json!({
                "queries": [" Berlin ", "berlin", " Hamburg "],
                "query": "ignored",
                "limit": 5
            }))
            .unwrap(),
            vec![
                LocationRequest {
                    query: "Berlin".into(),
                    limit: 5
                },
                LocationRequest {
                    query: "Hamburg".into(),
                    limit: 5
                },
            ]
        );
    }

    #[test]
    fn supports_singular_query_compatibility_and_integer_float_limits() {
        assert_eq!(
            build_location_requests(&json!({ "query": " 10115 ", "limit": 10.0 })).unwrap(),
            vec![LocationRequest {
                query: "10115".into(),
                limit: 10
            }]
        );
        assert_eq!(
            build_location_requests(&json!({ "query": "Berlin" })).unwrap()[0].limit,
            10
        );
    }

    #[test]
    fn validates_shape_query_text_query_count_and_limit() {
        assert!(build_location_requests(&json!({}))
            .unwrap_err()
            .to_string()
            .contains("Provide at least one"));
        assert!(build_location_requests(&json!({ "queries": "Berlin" }))
            .unwrap_err()
            .to_string()
            .contains("must be an array"));
        assert!(build_location_requests(&json!({ "queries": [null] }))
            .unwrap_err()
            .to_string()
            .contains("Each query must be a string"));
        assert!(build_location_requests(&json!({ "queries": [" "] }))
            .unwrap_err()
            .to_string()
            .contains("must not be empty"));
        assert!(
            build_location_requests(&json!({ "query": "x".repeat(121) }))
                .unwrap_err()
                .to_string()
                .contains("at most 120")
        );
        assert!(build_location_requests(
            &json!({ "queries": (0..101).map(|i| format!("q{i}")).collect::<Vec<_>>() })
        )
        .unwrap_err()
        .to_string()
        .contains("at most 100"));
        assert!(
            build_location_requests(&json!({ "query": "x", "limit": 0 }))
                .unwrap_err()
                .to_string()
                .contains("between 1 and 20")
        );
        assert!(
            build_location_requests(&json!({ "query": "x", "limit": 21 }))
                .unwrap_err()
                .to_string()
                .contains("between 1 and 20")
        );
        assert!(
            build_location_requests(&json!({ "query": "x", "limit": 1.5 }))
                .unwrap_err()
                .to_string()
                .contains("must be an integer")
        );
    }

    #[test]
    fn applies_unicode_case_folding_before_duplicate_checks() {
        let requests =
            build_location_requests(&json!({ "queries": ["Straße", "STRAẞE"] })).unwrap();
        assert_eq!(requests.len(), 1);
    }
}
