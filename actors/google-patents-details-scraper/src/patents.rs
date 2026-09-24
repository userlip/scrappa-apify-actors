use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Map, Value};
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatentRequest {
    pub input_patent_id: String,
    pub normalized_patent_id: String,
}

pub fn collect_requests(input: &Value) -> Result<Vec<PatentRequest>> {
    let input = input
        .as_object()
        .ok_or_else(|| anyhow!("Input must be an object"))?;
    let mut raw_values = Vec::new();

    if let Some(value) = clean_string(input.get("patent_id"), "patent_id")? {
        raw_values.push(value);
    }
    raw_values.extend(clean_string_array(input.get("patent_ids"), "patent_ids")?);
    if let Some(value) = clean_string(input.get("url"), "url")? {
        raw_values.push(value);
    }
    raw_values.extend(clean_string_array(input.get("urls"), "urls")?);

    if raw_values.is_empty() {
        bail!("At least one patent ID or Google Patents URL is required. Provide patent_id, patent_ids, url, or urls.");
    }

    let mut seen = HashSet::new();
    let mut requests = Vec::new();
    for input_patent_id in raw_values {
        let normalized_patent_id = normalize_identifier(&input_patent_id)?;
        if seen.insert(normalized_patent_id.clone()) {
            requests.push(PatentRequest {
                input_patent_id,
                normalized_patent_id,
            });
        }
    }

    Ok(requests)
}

fn clean_string(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
    let value = value.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value.to_owned()))
    }
}

fn clean_string_array(value: Option<&Value>, field: &str) -> Result<Vec<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(Vec::new());
    };
    if value.as_str() == Some("") {
        return Ok(Vec::new());
    }
    let Some(values) = value.as_array() else {
        bail!("{field} must be an array of strings");
    };

    values
        .iter()
        .enumerate()
        .map(|(index, value)| clean_string(Some(value), &format!("{field}[{index}]")))
        .collect::<Result<Vec<_>>>()
        .map(|values| values.into_iter().flatten().collect())
}

pub fn normalize_identifier(raw_value: &str) -> Result<String> {
    let value = raw_value.trim();
    if value.is_empty() {
        bail!("Patent ID or URL cannot be empty");
    }

    if value
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || value
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
    {
        return normalize_url(value, raw_value);
    }

    normalize_full_patent_id(value)
}

fn normalize_url(value: &str, raw_value: &str) -> Result<String> {
    let parsed =
        Url::parse(value).map_err(|_| anyhow!("Invalid Google Patents URL: {raw_value}"))?;
    let Some(host) = parsed.host_str() else {
        bail!("Invalid Google Patents URL: {raw_value}");
    };
    if !host.eq_ignore_ascii_case("patents.google.com")
        && !host.to_ascii_lowercase().ends_with(".patents.google.com")
    {
        bail!("Invalid Google Patents URL: {raw_value}");
    }

    let path = parsed.path().strip_prefix('/').unwrap_or(parsed.path());
    let mut parts = path.split('/').collect::<Vec<_>>();
    if parts.last() == Some(&"") {
        parts.pop();
    }
    if (parts.len() != 2 && parts.len() != 3) || !parts[0].eq_ignore_ascii_case("patent") {
        bail!("Invalid Google Patents URL path: {raw_value}");
    }

    let language = parts.get(2).copied().unwrap_or("en");
    normalize_full_patent_id(&format!("patent/{}/{language}", parts[1]))
}

fn normalize_full_patent_id(value: &str) -> Result<String> {
    let parts = value.split('/').collect::<Vec<_>>();
    if parts.len() == 3 && parts[0].eq_ignore_ascii_case("patent") {
        if !valid_publication_number(parts[1]) || !valid_two_letter_code(parts[2]) {
            return Err(invalid_patent_id(value));
        }
        return Ok(format!(
            "patent/{}/{}",
            parts[1].to_ascii_uppercase(),
            parts[2].to_ascii_lowercase()
        ));
    }

    if !valid_publication_number(value) {
        return Err(invalid_patent_id(value));
    }

    Ok(format!("patent/{}/en", value.to_ascii_uppercase()))
}

fn invalid_patent_id(value: &str) -> anyhow::Error {
    anyhow!(
        "Invalid patent ID format: {value}. Expected US9789384B1, EP3892147A1, WO2020123456A1, or a Google Patents URL."
    )
}

fn valid_publication_number(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 3 || !bytes[..2].iter().all(u8::is_ascii_alphabetic) {
        return false;
    }

    let mut index = 2;
    let digit_start = index;
    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
    }
    if index == digit_start {
        return false;
    }
    if bytes.get(index).is_some_and(u8::is_ascii_alphabetic) {
        index += 1;
    }
    bytes[index..].iter().all(u8::is_ascii_digit)
}

fn valid_two_letter_code(value: &str) -> bool {
    value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_alphabetic())
}

pub fn describe_requests(requests: &[PatentRequest]) -> String {
    if requests.len() == 1 {
        requests[0].normalized_patent_id.clone()
    } else {
        format!("{} patents", requests.len())
    }
}

pub fn build_success_dataset_item(response: &Value, request: &PatentRequest) -> Value {
    let data = response.get("data").filter(|value| !value.is_null());
    let mut item = spread_object(data);
    let data_object = data.and_then(Value::as_object);

    let patent_id = data_object
        .and_then(|data| data.get("patent_id"))
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or_else(|| Value::String(request.normalized_patent_id.clone()));
    let inventor_count = data_object
        .and_then(|data| data.get("inventors"))
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let assignee_count = data_object
        .and_then(|data| data.get("assignees"))
        .map_or(0, count_nested_arrays);
    let citation_count = data_object
        .and_then(|data| data.get("citations"))
        .map_or(0, count_nested_arrays);

    item.insert("input_patent_id".to_owned(), json!(request.input_patent_id));
    item.insert(
        "normalized_patent_id".to_owned(),
        json!(request.normalized_patent_id),
    );
    item.insert("success".to_owned(), Value::Bool(true));
    item.insert("patent_id".to_owned(), patent_id.clone());
    item.insert(
        "publication_number".to_owned(),
        nullish(
            data_object.and_then(|data| data.get("publication_number")),
            Value::Null,
        ),
    );
    item.insert(
        "patent_page".to_owned(),
        patent_page_url(&js_string(&patent_id))
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    for field in [
        "title",
        "abstract",
        "country",
        "language",
        "application_number",
    ] {
        item.insert(
            field.to_owned(),
            nullish(data_object.and_then(|data| data.get(field)), Value::Null),
        );
    }
    item.insert(
        "prior_art_keywords".to_owned(),
        nullish(
            data_object.and_then(|data| data.get("prior_art_keywords")),
            json!([]),
        ),
    );
    item.insert(
        "links".to_owned(),
        nullish(data_object.and_then(|data| data.get("links")), json!({})),
    );
    item.insert(
        "citations".to_owned(),
        nullish(
            data_object.and_then(|data| data.get("citations")),
            json!({}),
        ),
    );
    item.insert("inventor_count".to_owned(), json!(inventor_count));
    item.insert("assignee_count".to_owned(), json!(assignee_count));
    item.insert("citation_count".to_owned(), json!(citation_count));
    item.insert(
        "cached".to_owned(),
        nullish(
            data_object.and_then(|data| data.get("cached")),
            Value::Bool(false),
        ),
    );
    item.insert(
        "response_time_ms".to_owned(),
        nullish(
            data_object.and_then(|data| data.get("response_time_ms")),
            Value::Null,
        ),
    );

    Value::Object(item)
}

pub fn build_error_dataset_item(error: &str, request: &PatentRequest) -> Value {
    json!({
        "input_patent_id": request.input_patent_id,
        "normalized_patent_id": request.normalized_patent_id,
        "success": false,
        "patent_id": null,
        "publication_number": null,
        "patent_page": patent_page_url(&request.normalized_patent_id),
        "title": null,
        "abstract": null,
        "country": null,
        "language": null,
        "application_number": null,
        "prior_art_keywords": [],
        "links": {},
        "citations": {},
        "inventor_count": 0,
        "assignee_count": 0,
        "citation_count": 0,
        "cached": null,
        "response_time_ms": null,
        "error": error,
        "status_code": extract_scrappa_status_code(error),
    })
}

pub fn error_from_response(response: &Value) -> String {
    response
        .get("error")
        .filter(|value| !value.is_null())
        .or_else(|| response.get("message").filter(|value| !value.is_null()))
        .map(js_string)
        .unwrap_or_else(|| "Scrappa API returned success=false".to_owned())
}

pub fn extract_scrappa_status_code(message: &str) -> Option<u16> {
    let prefix = "Scrappa API error (";
    let start = message.find(prefix)? + prefix.len();
    let code = message.get(start..start + 3)?;
    if !code.bytes().all(|byte| byte.is_ascii_digit())
        || message.as_bytes().get(start + 3) != Some(&b')')
    {
        return None;
    }
    code.parse().ok()
}

pub fn patent_page_url(patent_id: &str) -> Option<String> {
    let parts = patent_id.split('/').collect::<Vec<_>>();
    if parts.len() != 3
        || !parts[0].eq_ignore_ascii_case("patent")
        || parts[1].is_empty()
        || !valid_two_letter_code(parts[2])
    {
        return None;
    }
    Some(format!("https://patents.google.com/patent/{}", parts[1]))
}

fn nullish(value: Option<&Value>, fallback: Value) -> Value {
    value
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(fallback)
}

fn count_nested_arrays(value: &Value) -> usize {
    match value {
        Value::Array(values) => values.iter().map(count_array).sum(),
        Value::Object(values) => values.values().map(count_array).sum(),
        _ => 0,
    }
}

fn count_array(value: &Value) -> usize {
    value.as_array().map_or(0, Vec::len)
}

fn spread_object(value: Option<&Value>) -> Map<String, Value> {
    match value {
        Some(Value::Object(object)) => object.clone(),
        Some(Value::Array(values)) => values
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        Some(Value::String(value)) => value
            .chars()
            .enumerate()
            .map(|(index, character)| (index.to_string(), json!(character.to_string())))
            .collect(),
        _ => Map::new(),
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(number) => number
            .as_f64()
            .filter(|value| value.fract() == 0.0 && value.abs() < 1e21)
            .map(|value| {
                if value == 0.0 {
                    "0".to_owned()
                } else {
                    format!("{value:.0}")
                }
            })
            .unwrap_or_else(|| number.to_string()),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTOR_CONFIG: &str = include_str!("../.actor/actor.json");
    const INPUT_SCHEMA: &str = include_str!("../.actor/input_schema.json");

    fn request(input: &str) -> PatentRequest {
        PatentRequest {
            input_patent_id: input.to_owned(),
            normalized_patent_id: "patent/US9789384B1/en".to_owned(),
        }
    }

    #[test]
    fn normalizes_publication_ids_and_google_patents_urls() {
        assert_eq!(
            normalize_identifier(" us9789384b1 ").unwrap(),
            "patent/US9789384B1/en"
        );
        assert_eq!(
            normalize_identifier("patent/ep3892147a1/DE").unwrap(),
            "patent/EP3892147A1/de"
        );
        assert_eq!(
            normalize_identifier("https://patents.google.com/patent/WO2020123456A1?oq=test")
                .unwrap(),
            "patent/WO2020123456A1/en"
        );
        assert_eq!(
            normalize_identifier("http://research.patents.google.com/patent/EP3892147A1/de/")
                .unwrap(),
            "patent/EP3892147A1/de"
        );
    }

    #[test]
    fn collects_all_input_fields_in_order_and_deduplicates_normalized_ids() {
        let requests = collect_requests(&json!({
            "patent_id": "US9789384B1",
            "patent_ids": ["EP3892147A1", "us9789384b1"],
            "url": "https://patents.google.com/patent/WO2020123456A1",
            "urls": ["https://patents.google.com/patent/EP3892147A1"]
        }))
        .unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].input_patent_id, "US9789384B1");
        assert_eq!(requests[0].normalized_patent_id, "patent/US9789384B1/en");
        assert_eq!(requests[1].normalized_patent_id, "patent/EP3892147A1/en");
        assert_eq!(requests[2].normalized_patent_id, "patent/WO2020123456A1/en");
    }

    #[test]
    fn rejects_missing_invalid_and_non_string_inputs() {
        assert!(collect_requests(&json!({}))
            .unwrap_err()
            .to_string()
            .contains("At least one"));
        assert!(normalize_identifier("INVALID!@#").is_err());
        assert!(
            normalize_identifier("https://example.com/patent/US9789384B1")
                .unwrap_err()
                .to_string()
                .contains("Invalid Google Patents URL")
        );
        assert!(collect_requests(&json!({ "patent_id": 42 }))
            .unwrap_err()
            .to_string()
            .contains("patent_id must be a string"));
        assert!(collect_requests(&json!({ "patent_ids": "US9789384B1" }))
            .unwrap_err()
            .to_string()
            .contains("patent_ids must be an array"));
    }

    #[test]
    fn accepts_empty_optional_arrays_as_missing_input() {
        let error = collect_requests(&json!({ "patent_ids": [], "urls": [] }))
            .unwrap_err()
            .to_string();
        assert!(error.contains("At least one patent ID"));
        assert!(
            collect_requests(&json!({ "patent_id": "", "patent_ids": [null, " "] }))
                .unwrap_err()
                .to_string()
                .contains("At least one patent ID")
        );
    }

    #[test]
    fn actor_input_schema_keeps_the_existing_single_id_prefill() {
        let schema: Value = serde_json::from_str(INPUT_SCHEMA).unwrap();
        let config: Value = serde_json::from_str(ACTOR_CONFIG).unwrap();
        assert_eq!(schema["properties"]["patent_id"]["prefill"], "US9789384B1");
        assert_eq!(schema["properties"]["patent_ids"]["type"], "array");
        assert_eq!(schema["properties"]["urls"]["type"], "array");
        assert_eq!(config["defaultRunOptions"]["timeoutSecs"], 120);
        assert_eq!(config["resources"]["memoryMbytes"], 128);
    }

    #[test]
    fn success_dataset_item_preserves_data_and_adds_flattened_counts() {
        let item = build_success_dataset_item(
            &json!({
                "success": true,
                "data": {
                    "patent_id": "patent/US9789384B1/en",
                    "publication_number": "US9789384B1",
                    "title": "A board",
                    "inventors": ["Jane", "John"],
                    "assignees": { "original": ["A", "B"], "current": ["C"] },
                    "citations": { "patent": [{}, {}], "non_patent": [{}] },
                    "custom_field": "preserved",
                    "cached": false,
                    "response_time_ms": 12
                }
            }),
            &request("US9789384B1"),
        );
        assert_eq!(item["success"], true);
        assert_eq!(item["input_patent_id"], "US9789384B1");
        assert_eq!(
            item["patent_page"],
            "https://patents.google.com/patent/US9789384B1"
        );
        assert_eq!(item["inventor_count"], 2);
        assert_eq!(item["assignee_count"], 3);
        assert_eq!(item["citation_count"], 3);
        assert_eq!(item["custom_field"], "preserved");
        assert_eq!(item["prior_art_keywords"], json!([]));
        assert_eq!(item["links"], json!({}));
    }

    #[test]
    fn failed_response_and_status_code_are_preserved() {
        let error = error_from_response(
            &json!({ "success": false, "error": "Scrappa API error (404): missing" }),
        );
        let item = build_error_dataset_item(&error, &request("US9789384B1"));
        assert_eq!(item["success"], false);
        assert_eq!(item["status_code"], 404);
        assert_eq!(item["error"], "Scrappa API error (404): missing");
        assert_eq!(
            item["patent_page"],
            "https://patents.google.com/patent/US9789384B1"
        );
        assert_eq!(
            error_from_response(&json!({ "success": false })),
            "Scrappa API returned success=false"
        );
        assert_eq!(extract_scrappa_status_code("Network failed"), None);
    }
}
