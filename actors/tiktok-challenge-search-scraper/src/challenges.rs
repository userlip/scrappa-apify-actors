use anyhow::{bail, Result};
use serde_json::{json, Map, Number, Value};
use std::collections::HashSet;

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

#[derive(Clone, Debug, PartialEq)]
pub struct SearchRequest {
    pub keyword: String,
    pub count: Option<u64>,
}

fn is_js_whitespace(character: char) -> bool {
    character.is_whitespace() || character == '\u{feff}'
}

fn normalize_keyword(value: &str) -> Result<Option<String>> {
    let keyword = value
        .trim_matches(is_js_whitespace)
        .split(is_js_whitespace)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if keyword.is_empty() {
        return Ok(None);
    }
    if keyword.encode_utf16().count() > 255 {
        bail!("TikTok challenge search keywords must be 255 characters or fewer");
    }
    if value
        .chars()
        .any(|character| matches!(character, '\r' | '\n' | '\t' | '\u{000c}' | '\u{000b}'))
    {
        bail!("TikTok challenge search keywords cannot contain tabs, line breaks, or control whitespace");
    }
    Ok(Some(keyword))
}

fn js_type(value: &Value) -> &'static str {
    match value {
        Value::Null | Value::Array(_) | Value::Object(_) => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
    }
}

pub(crate) fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    js_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

pub fn build_search_requests(input: &Value) -> Result<(Vec<SearchRequest>, Vec<String>)> {
    let mut warnings = Vec::new();
    let mut keywords = Vec::new();
    let mut seen = HashSet::new();
    let input_object = input.as_object();
    let raw_keywords = input_object.and_then(|input| input.get("keywords"));

    match raw_keywords {
        Some(Value::Array(values)) => {
            for value in values {
                let Some(value) = value.as_str() else {
                    if !value.is_null() {
                        warnings.push(format!(
                            "keywords entries must be strings, got {}. Omitting entry.",
                            js_type(value)
                        ));
                    }
                    continue;
                };
                if let Some(keyword) = normalize_keyword(value)? {
                    if seen.insert(keyword.clone()) {
                        keywords.push(keyword);
                    }
                }
            }
        }
        Some(Value::String(value)) => {
            if let Some(keyword) = normalize_keyword(value)? {
                seen.insert(keyword.clone());
                keywords.push(keyword);
            }
        }
        Some(Value::Null) | None => {}
        Some(value) => warnings.push(format!(
            "keywords must be an array of strings, got {}. Falling back to keyword.",
            js_type(value)
        )),
    }

    if keywords.is_empty() {
        match input_object.and_then(|input| input.get("keyword")) {
            Some(Value::String(value)) => {
                if let Some(keyword) = normalize_keyword(value)? {
                    if seen.insert(keyword.clone()) {
                        keywords.push(keyword);
                    }
                }
            }
            Some(Value::Null) | None => {}
            Some(value) => {
                warnings.push(format!("keyword must be a string, got {}.", js_type(value)))
            }
        }
    }

    if keywords.is_empty() {
        bail!("At least one TikTok challenge search keyword is required");
    }

    let raw_count = input_object.and_then(|input| input.get("count"));
    let count = match raw_count {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) if value.is_empty() => None,
        Some(Value::Number(number)) => valid_count(number),
        Some(_) => None,
    };
    if let Some(raw_count) = raw_count {
        let is_missing = raw_count.is_null() || raw_count.as_str().is_some_and(str::is_empty);
        if !is_missing && count.is_none() {
            warnings.push(format!(
                "count must be an integer between 1 and 50, got {}. Using Scrappa default.",
                js_string(raw_count)
            ));
        }
    }

    Ok((
        keywords
            .into_iter()
            .map(|keyword| SearchRequest { keyword, count })
            .collect(),
        warnings,
    ))
}

fn valid_count(number: &Number) -> Option<u64> {
    let value = number.as_f64()?;
    if value.is_finite() && value.fract() == 0.0 && (1.0..=50.0).contains(&value) {
        Some(value as u64)
    } else {
        None
    }
}

pub fn format_lookup(requests: &[SearchRequest]) -> String {
    match requests {
        [] => "unknown TikTok challenge search".to_owned(),
        [request] => request.keyword.clone(),
        requests => format!("{} TikTok challenge searches", requests.len()),
    }
}

fn coalesce_non_null<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Option<&'a Value> {
    values.into_iter().flatten().find(|value| !value.is_null())
}

fn challenge_id(challenge: &Value) -> Value {
    let id = coalesce_non_null([
        challenge.get("challenge_id"),
        challenge.get("id"),
        challenge.get("cid"),
    ]);
    match id {
        Some(Value::String(value)) => {
            let value = value.trim_matches(is_js_whitespace);
            if value.is_empty() {
                Value::Null
            } else {
                Value::String(value.to_owned())
            }
        }
        Some(Value::Number(number)) => {
            let Some(number) = number.as_f64() else { return Value::Null };
            if number.is_finite() && number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER {
                Value::String(if number == 0.0 {
                    "0".to_owned()
                } else {
                    number.to_string().trim_end_matches(".0").to_owned()
                })
            } else {
                Value::Null
            }
        }
        _ => Value::Null,
    }
}

fn challenge_name(challenge: &Value) -> Value {
    coalesce_non_null([
        challenge.get("challenge_name"),
        challenge.get("cha_name"),
        challenge.get("name"),
        challenge.get("title"),
    ])
    .and_then(Value::as_str)
    .map(|value| value.trim_matches(is_js_whitespace))
    .filter(|value| !value.is_empty())
    .map(|value| Value::String(value.to_owned()))
    .unwrap_or(Value::Null)
}

pub(crate) fn normalized_challenge(challenge: &Value, request: &SearchRequest) -> Result<Value> {
    if challenge.is_null() {
        bail!("Cannot normalize a null TikTok challenge result");
    }

    let mut result = match challenge {
        Value::Object(fields) => fields.clone(),
        Value::String(value) => value
            .chars()
            .enumerate()
            .map(|(index, character)| (index.to_string(), Value::String(character.to_string())))
            .collect(),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        _ => Map::new(),
    };
    let stats = challenge.get("stats");
    let description = challenge
        .get("description")
        .and_then(Value::as_str)
        .map(|value| Value::String(value.to_owned()))
        .or_else(|| {
            challenge
                .get("desc")
                .filter(|value| !value.is_null())
                .cloned()
        })
        .unwrap_or(Value::Null);

    result.insert("challenge_id".to_owned(), challenge_id(challenge));
    result.insert("challenge_name".to_owned(), challenge_name(challenge));
    result.insert("description".to_owned(), description);
    result.insert(
        "view_count".to_owned(),
        coalesce_non_null([
            challenge.get("view_count"),
            stats.and_then(|value| value.get("view_count")),
        ])
        .cloned()
        .unwrap_or(Value::Null),
    );
    result.insert(
        "video_count".to_owned(),
        coalesce_non_null([
            challenge.get("video_count"),
            stats.and_then(|value| value.get("video_count")),
        ])
        .cloned()
        .unwrap_or(Value::Null),
    );
    result.insert(
        "user_count".to_owned(),
        coalesce_non_null([
            challenge.get("user_count"),
            stats.and_then(|value| value.get("user_count")),
        ])
        .cloned()
        .unwrap_or(Value::Null),
    );
    result.insert("request_keyword".to_owned(), json!(request.keyword));
    result.insert(
        "request_count".to_owned(),
        request.count.map_or(Value::Null, |count| json!(count)),
    );
    Ok(Value::Object(result))
}

pub fn extract_challenges(data: Option<&Value>) -> Vec<Value> {
    let Some(data) = data.filter(|value| !value.is_null()) else {
        return Vec::new();
    };
    if let Some(challenges) = data.as_array() {
        return challenges.clone();
    }
    for field in [
        "challenges",
        "challenge_list",
        "challengeList",
        "items",
        "results",
    ] {
        if let Some(challenges) = data.get(field).and_then(Value::as_array) {
            return challenges.clone();
        }
    }
    Vec::new()
}

pub(crate) fn validate_scrappa_response(response: &Value) -> Result<()> {
    let Some(code) = response.get("code") else {
        return Ok(());
    };
    if code.as_f64() == Some(0.0) {
        return Ok(());
    }
    let message = response
        .get("msg")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or_else(|| "Unknown error".to_owned());
    bail!(
        "Scrappa TikTok Challenge Search API returned code {}: {message}",
        js_string(code)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_batch_legacy_deduplicated_and_prefilled_input() {
        let input = json!({
            "keywords": [" cosplay ", "fitness", "cosplay", 12, null],
            "keyword": "ignored",
            "count": 10
        });
        let (requests, warnings) = build_search_requests(&input).unwrap();
        assert_eq!(
            requests,
            vec![
                SearchRequest {
                    keyword: "cosplay".to_owned(),
                    count: Some(10)
                },
                SearchRequest {
                    keyword: "fitness".to_owned(),
                    count: Some(10)
                },
            ]
        );
        assert_eq!(
            warnings,
            ["keywords entries must be strings, got number. Omitting entry."]
        );

        let (legacy, _) =
            build_search_requests(&json!({ "keywords": [], "keyword": "  tea   trends " }))
                .unwrap();
        assert_eq!(legacy[0].keyword, "tea trends");
        let (string_value, _) =
            build_search_requests(&json!({ "keywords": " skincare " })).unwrap();
        assert_eq!(string_value[0].keyword, "skincare");
        let (fallback, _) = build_search_requests(&json!({
            "keywords": [" \n "],
            "keyword": "tea",
        }))
        .unwrap();
        assert_eq!(fallback[0].keyword, "tea");
        let (trimmed, _) = build_search_requests(&json!({
            "keywords": [format!("{}x", " ".repeat(300))],
        }))
        .unwrap();
        assert_eq!(trimmed[0].keyword, "x");

        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["keywords"]["prefill"],
            json!(["cosplay", "fitness"])
        );
        assert_eq!(schema["properties"]["count"]["default"], 10);
    }

    #[test]
    fn rejects_missing_invalid_and_oversized_keywords_and_defaults_invalid_count() {
        assert!(build_search_requests(&json!({}))
            .unwrap_err()
            .to_string()
            .contains("At least one"));
        assert!(
            build_search_requests(&json!({ "keywords": ["cosplay\nfitness"] }))
                .unwrap_err()
                .to_string()
                .contains("cannot contain tabs")
        );
        assert!(
            build_search_requests(&json!({ "keyword": "x".repeat(256) }))
                .unwrap_err()
                .to_string()
                .contains("255 characters")
        );
        assert!(
            build_search_requests(&json!({ "keyword": "😀".repeat(128) }))
                .unwrap_err()
                .to_string()
                .contains("255 characters")
        );

        let (requests, warnings) =
            build_search_requests(&json!({ "keyword": "trend", "count": 0 })).unwrap();
        assert_eq!(requests[0].count, None);
        assert!(warnings[0].contains("integer between 1 and 50"));
    }

    #[test]
    fn extracts_challenge_shapes_and_preserves_raw_fields_with_normalized_columns() {
        let challenge = json!({
            "id": 123,
            "cha_name": " cosplay ",
            "desc": "raw description",
            "stats": { "view_count": 100, "video_count": 20, "user_count": 5 },
            "extra": "kept"
        });
        for field in [
            "challenges",
            "challenge_list",
            "challengeList",
            "items",
            "results",
        ] {
            let data = json!({ (field): [challenge.clone()] });
            assert_eq!(extract_challenges(Some(&data)), [challenge.clone()]);
        }
        assert_eq!(
            extract_challenges(Some(&json!([challenge.clone()]))),
            [challenge.clone()]
        );
        assert!(extract_challenges(Some(&json!({}))).is_empty());
        assert!(extract_challenges(None).is_empty());

        let request = SearchRequest {
            keyword: "cosplay".to_owned(),
            count: Some(10),
        };
        let normalized = normalized_challenge(&challenge, &request).unwrap();
        assert_eq!(normalized["challenge_id"], "123");
        assert_eq!(normalized["challenge_name"], "cosplay");
        assert_eq!(normalized["description"], "raw description");
        assert_eq!(normalized["view_count"], 100);
        assert_eq!(normalized["request_count"], 10);
        assert_eq!(normalized["extra"], "kept");
    }

    #[test]
    fn preserves_safe_id_and_name_fallback_rules_and_api_error_code() {
        assert_eq!(
            challenge_id(&json!({ "challenge_id": "  ", "id": 5 })),
            Value::Null
        );
        assert_eq!(
            challenge_id(&json!({ "id": 9_007_199_254_740_992_u64 })),
            Value::Null
        );
        assert_eq!(
            challenge_name(&json!({ "challenge_name": " ", "title": "ignored" })),
            Value::Null
        );
        assert!(
            validate_scrappa_response(&json!({ "code": 7, "msg": "bad request" }))
                .unwrap_err()
                .to_string()
                .contains("code 7: bad request")
        );
        assert!(validate_scrappa_response(&json!({ "code": null }))
            .unwrap_err()
            .to_string()
            .contains("code null: Unknown error"));
        assert!(validate_scrappa_response(&json!({ "code": 0 })).is_ok());
    }
}
