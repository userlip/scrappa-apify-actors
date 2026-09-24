use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use percent_encoding::percent_decode_str;
use serde_json::{Map, Value};

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 250;
const MAX_QUERIES_PER_RUN: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinterestSearchParams {
    pub(crate) query: String,
    pub(crate) limit: usize,
    pub(crate) bookmark: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinterestSearchPlan {
    pub(crate) queries: Vec<String>,
    pub(crate) limit: usize,
    pub(crate) bookmark: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinterestSearchFetchParams {
    pub(crate) params: PinterestSearchParams,
    pub(crate) requested_limit: usize,
    pub(crate) fetch_limit: usize,
}

pub(crate) fn decode_input_string(value: &str) -> String {
    if !value.contains('%') {
        return value.to_owned();
    }
    if !has_valid_percent_escapes(value) {
        return value.to_owned();
    }
    percent_decode_str(value)
        .decode_utf8()
        .map(|decoded| decoded.into_owned())
        .unwrap_or_else(|_| value.to_owned())
}

fn has_valid_percent_escapes(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };

    let trimmed = decode_input_string(value).trim().to_owned();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(trimmed))
}

fn clean_integer(
    value: Option<&Value>,
    field: &str,
    min: usize,
    max: usize,
) -> Result<Option<usize>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }

    let normalized = match value {
        Value::String(value) => {
            let value = value.trim();
            if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                bail!("{field} must be an integer");
            }
            value
                .parse::<usize>()
                .with_context(|| format!("{field} must be an integer"))?
        }
        Value::Number(value) => {
            let value = value.as_f64().filter(|value| value.is_finite());
            let Some(value) = value.filter(|value| value.fract() == 0.0) else {
                bail!("{field} must be an integer");
            };
            if value < 0.0 || value > usize::MAX as f64 {
                bail!("{field} must be an integer");
            }
            value as usize
        }
        _ => bail!("{field} must be an integer"),
    };

    if normalized < min || normalized > max {
        bail!("{field} must be between {min} and {max}");
    }
    Ok(Some(normalized))
}

pub(crate) fn build_pinterest_search_plan(
    input: &Map<String, Value>,
) -> Result<PinterestSearchPlan> {
    let mut queries = Vec::new();
    if let Some(query) = clean_string(input.get("query"), "query", 200)? {
        queries.push(query);
    }

    if let Some(value) = input.get("queries") {
        if !value.is_null() && value.as_str() != Some("") {
            let Some(values) = value.as_array() else {
                bail!("queries must be an array of strings");
            };
            for (index, value) in values.iter().enumerate() {
                if let Some(query) = clean_string(Some(value), &format!("queries[{index}]"), 200)? {
                    queries.push(query);
                }
            }
        }
    }

    let mut seen = HashSet::new();
    queries.retain(|query| seen.insert(query.clone()));
    if queries.is_empty() {
        bail!("Provide at least one Pinterest search query using queries or query");
    }
    if queries.len() > MAX_QUERIES_PER_RUN {
        bail!("queries cannot contain more than {MAX_QUERIES_PER_RUN} values per run");
    }

    let limit = clean_integer(input.get("limit"), "limit", 1, MAX_LIMIT)?.unwrap_or(DEFAULT_LIMIT);
    let bookmark = clean_string(input.get("bookmark"), "bookmark", 2000)?;
    Ok(PinterestSearchPlan {
        queries,
        limit,
        bookmark,
    })
}

pub(crate) fn describe_pinterest_search_request(plan: &PinterestSearchPlan) -> String {
    let query_label = if plan.queries.len() == 1 {
        format!("\"{}\"", plan.queries[0])
    } else {
        format!("{} queries", plan.queries.len())
    };
    let bookmark_label = if plan.bookmark.is_some() {
        ", with bookmark"
    } else {
        ""
    };
    format!("{query_label} ({} pins/query{bookmark_label})", plan.limit)
}

pub(crate) fn cap_pinterest_search_params(
    params: &PinterestSearchParams,
    chargeable_pin_capacity: usize,
) -> PinterestSearchFetchParams {
    let requested_limit = params.limit;
    let fetch_limit = requested_limit.min(chargeable_pin_capacity);
    let mut params = params.clone();
    params.limit = fetch_limit;
    PinterestSearchFetchParams {
        params,
        requested_limit,
        fetch_limit,
    }
}
