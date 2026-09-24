use anyhow::{anyhow, bail, Result};
use serde_json::Value;

pub(crate) const MAX_QUERIES_PER_RUN: usize = 25;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GoogleFinanceSearchRequest {
    pub(crate) q: String,
    pub(crate) hl: Option<String>,
    pub(crate) gl: Option<String>,
}

pub(crate) fn build_search_requests(input: &Value) -> Result<Vec<GoogleFinanceSearchRequest>> {
    let input = input.as_object();
    let get = |field: &str| input.and_then(|input| input.get(field));
    let hl = clean_language_code(get("hl"))?;
    let gl = clean_country_code(get("gl"))?;
    let queries = match get("queries") {
        Some(value) if !value.is_null() => {
            let queries = value
                .as_array()
                .ok_or_else(|| anyhow!("queries must be an array of strings"))?;
            if queries.len() > MAX_QUERIES_PER_RUN {
                bail!("queries can include at most {MAX_QUERIES_PER_RUN} items per run");
            }
            let queries = queries
                .iter()
                .enumerate()
                .map(|(index, query)| {
                    clean_required_string(Some(query), &format!("queries[{index}]"), 255)
                })
                .collect::<Result<Vec<_>>>()?;
            if queries.is_empty() {
                bail!("queries must include at least one query");
            }
            queries
        }
        _ => vec![clean_required_string(get("q"), "q", 255)?],
    };

    Ok(queries
        .into_iter()
        .map(|q| GoogleFinanceSearchRequest {
            q,
            hl: hl.clone(),
            gl: gl.clone(),
        })
        .collect())
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .ok_or_else(|| anyhow!("{field} must be a string"))?;
    if value.is_empty() {
        return Ok(None);
    }
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(trimmed.to_owned()))
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_language_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(value) = clean_string(value, "hl", 10)? else {
        return Ok(None);
    };
    let mut parts = value.split('-');
    let language = parts.next().unwrap_or_default();
    let region = parts.next();
    if !language.is_ascii()
        || language.len() != 2
        || !language.bytes().all(|byte| byte.is_ascii_alphabetic())
        || region.is_some_and(|region| {
            !region.is_ascii()
                || region.len() != 2
                || !region.bytes().all(|byte| byte.is_ascii_alphabetic())
        })
        || parts.next().is_some()
    {
        bail!("hl must be a two-letter language code with an optional two-letter region");
    }
    Ok(Some(value.to_ascii_lowercase()))
}

fn clean_country_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(value) = clean_string(value, "gl", 10)? else {
        return Ok(None);
    };
    if !value.is_ascii()
        || value.len() != 2
        || !value.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        bail!("gl must be a two-letter country code");
    }
    Ok(Some(value.to_ascii_lowercase()))
}

pub(crate) fn describe_search_request(params: &GoogleFinanceSearchRequest) -> String {
    let filters = [
        params.hl.as_ref().map(|value| format!("hl={value}")),
        params.gl.as_ref().map(|value| format!("gl={value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    if filters.is_empty() {
        format!("\"{}\"", params.q)
    } else {
        format!("\"{}\" ({})", params.q, filters.join(", "))
    }
}
