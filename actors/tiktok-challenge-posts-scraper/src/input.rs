use anyhow::{bail, Result};
use serde_json::Value;
use std::collections::HashSet;

pub const MAX_CHALLENGE_IDS: usize = 20;
pub const MAX_RESULTS_PER_CHALLENGE: usize = 500;
pub const MAX_TOTAL_RESULTS: usize = 2_000;
pub const MAX_PAGE_SIZE: usize = 50;

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChallengeRequest {
    pub challenge_id: String,
    pub region: Option<String>,
    pub initial_cursor: Option<String>,
    pub result_limit: usize,
    pub page_size: usize,
}

pub fn parse_input(input: &Value) -> Result<Vec<ChallengeRequest>> {
    let object = input.as_object();
    let batch_values = object
        .and_then(|object| object.get("challenge_ids"))
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty());
    let values = batch_values
        .map(|values| values.iter().collect::<Vec<_>>())
        .unwrap_or_else(|| {
            object
                .and_then(|object| object.get("challenge_id"))
                .into_iter()
                .collect()
        });

    let mut unique_ids = Vec::new();
    let mut seen_ids = HashSet::new();
    for value in values {
        let Some(id) = normalize_challenge_id(value) else {
            continue;
        };
        if seen_ids.insert(id.clone()) {
            unique_ids.push(id);
        }
    }

    if unique_ids.is_empty() {
        bail!("At least one numeric TikTok challenge ID is required");
    }
    if unique_ids.len() > MAX_CHALLENGE_IDS {
        bail!("A maximum of {MAX_CHALLENGE_IDS} challenge IDs is allowed per run");
    }

    let result_limit = positive_integer(
        object.and_then(|object| object.get("results_per_challenge")),
        100,
        MAX_RESULTS_PER_CHALLENGE,
        "results_per_challenge",
    )?;
    if unique_ids.len() * result_limit > MAX_TOTAL_RESULTS {
        bail!("Total requested results cannot exceed {MAX_TOTAL_RESULTS}");
    }
    let page_size = positive_integer(
        object.and_then(|object| object.get("page_size")),
        10,
        MAX_PAGE_SIZE,
        "page_size",
    )?
    .min(result_limit);

    let region = match object.and_then(|object| object.get("region")) {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => {
            let value = js_trim(value).to_uppercase();
            if value.is_empty() {
                None
            } else if value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_uppercase()) {
                Some(value)
            } else {
                bail!("region must be a two-letter country code");
            }
        }
        Some(_) => bail!("region must be a two-letter country code"),
    };

    let initial_cursor = match object.and_then(|object| object.get("cursor")) {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => {
            let cursor = js_trim(value);
            (!cursor.is_empty()).then(|| cursor.to_owned())
        }
        Some(value) => match safe_integer_string(value) {
            Some(cursor) => Some(cursor),
            None => bail!("cursor must be a string or safe integer"),
        },
    };

    Ok(unique_ids
        .into_iter()
        .map(|challenge_id| ChallengeRequest {
            challenge_id,
            region: region.clone(),
            initial_cursor: initial_cursor.clone(),
            result_limit,
            page_size,
        })
        .collect())
}

fn normalize_challenge_id(value: &Value) -> Option<String> {
    let normalized = match value {
        Value::String(value) => js_trim(value).to_owned(),
        _ => safe_integer_string(value)?,
    };
    let length = normalized.len();
    (1..=100)
        .contains(&length)
        .then_some(normalized.clone())
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_digit()))
}

fn safe_integer_string(value: &Value) -> Option<String> {
    let number = value.as_f64()?;
    if !number.is_finite() || number.fract() != 0.0 || number.abs() > MAX_SAFE_INTEGER {
        return None;
    }
    Some(format!("{number:.0}"))
}

fn positive_integer(
    value: Option<&Value>,
    fallback: usize,
    maximum: usize,
    name: &str,
) -> Result<usize> {
    let Some(value) = value else {
        return Ok(fallback);
    };
    let valid_number = value.as_f64().filter(|number| {
        number.is_finite() && number.fract() == 0.0 && *number >= 1.0 && *number <= maximum as f64
    });
    match valid_number {
        Some(number) => Ok(number as usize),
        None => bail!("{name} must be an integer between 1 and {maximum}"),
    }
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

fn js_trim(value: &str) -> &str {
    value.trim_matches(is_js_whitespace)
}
