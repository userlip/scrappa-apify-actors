use anyhow::{anyhow, bail, Result};
use serde_json::{json, Map, Number, Value};

const MAX_QUERY_LENGTH: usize = 500;
const MAX_FILTER_LENGTH: usize = 100;
const MAX_PAGE: i64 = 100;
const MAX_BATCH_SEARCHES: usize = 25;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug)]
pub(crate) struct SearchPlanItem {
    pub(crate) index: usize,
    pub(crate) params: Map<String, Value>,
}

fn decode_input_string(value: &str) -> String {
    let bytes = value.as_bytes();
    if !bytes.contains(&b'%') {
        return value.to_owned();
    }

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
        let Some(high) = hex_value(bytes[index + 1]) else {
            return value.to_owned();
        };
        let Some(low) = hex_value(bytes[index + 2]) else {
            return value.to_owned();
        };
        decoded.push((high << 4) | low);
        index += 3;
    }
    String::from_utf8(decoded).unwrap_or_else(|_| value.to_owned())
}

fn hex_value(byte: u8) -> Option<u8> {
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
    let cleaned = decode_input_string(value)
        .trim_matches(is_javascript_whitespace)
        .to_owned();
    if cleaned.is_empty() {
        return Ok(None);
    }
    if cleaned.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(cleaned))
}

fn is_javascript_whitespace(character: char) -> bool {
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
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_integer(value: Option<&Value>, field: &str, min: i64, max: u64) -> Result<Option<Value>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }

    let integer = if let Some(text) = value.as_str() {
        let text = text.trim();
        let digits = text.strip_prefix('-').unwrap_or(text);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            bail!("{field} must be an integer");
        }
        text.parse::<i128>()
            .map_err(|_| anyhow!("{field} must be an integer"))?
    } else if let Some(number) = value.as_number() {
        number
            .as_i64()
            .map(i128::from)
            .or_else(|| number.as_u64().map(i128::from))
            .or_else(|| {
                number
                    .as_f64()
                    .filter(|number| number.is_finite() && number.fract() == 0.0)
                    .map(|number| number as i128)
            })
            .ok_or_else(|| anyhow!("{field} must be an integer"))?
    } else {
        bail!("{field} must be an integer");
    };

    if integer < i128::from(min) || integer > i128::from(max) {
        bail!("{field} must be between {min} and {max}");
    }
    if integer < 0 {
        Ok(Some(Value::Number(Number::from(integer as i64))))
    } else {
        Ok(Some(Value::Number(Number::from(integer as u64))))
    }
}

fn build_single_search_params(input: &Map<String, Value>) -> Result<Map<String, Value>> {
    let price_min = clean_integer(input.get("price_min"), "price_min", 0, MAX_SAFE_INTEGER)?;
    let price_max = clean_integer(input.get("price_max"), "price_max", 0, MAX_SAFE_INTEGER)?;
    if let (Some(price_min), Some(price_max)) = (&price_min, &price_max) {
        if price_max.as_u64().unwrap_or(0) < price_min.as_u64().unwrap_or(0) {
            bail!("price_max cannot be less than price_min");
        }
    }

    let query = clean_required_string(input.get("query"), "query", MAX_QUERY_LENGTH)?;
    let page =
        clean_integer(input.get("page"), "page", 1, MAX_PAGE as u64)?.unwrap_or_else(|| json!(1));
    let location = clean_string(input.get("location"), "location", MAX_FILTER_LENGTH)?;
    let category = clean_string(input.get("category"), "category", MAX_FILTER_LENGTH)?;

    let mut params = Map::new();
    params.insert("query".to_owned(), Value::String(query));
    params.insert("page".to_owned(), page);
    if let Some(location) = location {
        params.insert("location".to_owned(), Value::String(location));
    }
    if let Some(category) = category {
        params.insert("category".to_owned(), Value::String(category));
    }
    if let Some(price_min) = price_min {
        params.insert("price_min".to_owned(), price_min);
    }
    if let Some(price_max) = price_max {
        params.insert("price_max".to_owned(), price_max);
    }
    Ok(params)
}

pub(crate) fn build_search_plan(input: &Value) -> Result<Vec<SearchPlanItem>> {
    let Some(input) = input.as_object() else {
        bail!("query is required");
    };
    let raw_searches = match input.get("searches") {
        None | Some(Value::Null) => vec![input],
        Some(Value::Array(searches)) => {
            if searches.is_empty() {
                bail!("searches must contain at least one search");
            }
            if searches.len() > MAX_BATCH_SEARCHES {
                bail!("searches cannot contain more than {MAX_BATCH_SEARCHES} search objects");
            }
            searches
                .iter()
                .enumerate()
                .map(|(index, search)| {
                    search
                        .as_object()
                        .ok_or_else(|| anyhow!("searches[{index}] must be an object"))
                })
                .collect::<Result<Vec<_>>>()?
        }
        Some(_) => bail!("searches must be an array"),
    };

    raw_searches
        .into_iter()
        .enumerate()
        .map(|(index, search)| {
            Ok(SearchPlanItem {
                index,
                params: build_single_search_params(search)?,
            })
        })
        .collect()
}

pub(crate) fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(number) => number.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

pub(crate) fn js_string(value: &Value) -> String {
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

pub(crate) fn describe_search_request(searches: &[SearchPlanItem]) -> String {
    if searches.len() != 1 {
        return format!("{} Kleinanzeigen searches", searches.len());
    }
    let params = searches.first().map(|search| &search.params);
    let query = params
        .and_then(|params| params.get("query"))
        .map(js_string)
        .unwrap_or_default();
    let location = params
        .and_then(|params| params.get("location"))
        .filter(|value| js_truthy(value))
        .map(|value| format!(" in {}", js_string(value)))
        .unwrap_or_default();
    let category = params
        .and_then(|params| params.get("category"))
        .filter(|value| js_truthy(value))
        .map(|value| format!(", category {}", js_string(value)))
        .unwrap_or_default();
    let page = params
        .and_then(|params| params.get("page"))
        .map(js_string)
        .unwrap_or_else(|| "1".to_owned());
    format!("\"{query}\"{location} (page {page}{category})")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_single_and_batch_searches() {
        let plan = build_search_plan(&json!({
            "query": " e-bike%20fully ",
            "page": "2",
            "location": " Berlin ",
            "category": " elektronik ",
            "price_min": "50",
            "price_max": 500
        }))
        .unwrap();
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].index, 0);
        assert_eq!(plan[0].params["query"], "e-bike fully");
        assert_eq!(plan[0].params["page"], 2);
        assert_eq!(plan[0].params["price_min"], 50);
        assert_eq!(plan[0].params["price_max"], 500);
        assert_eq!(
            describe_search_request(&plan),
            "\"e-bike fully\" in Berlin (page 2, category elektronik)"
        );
        assert_eq!(
            plan[0]
                .params
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "query",
                "page",
                "location",
                "category",
                "price_min",
                "price_max"
            ]
        );

        let batch = build_search_plan(&json!({
            "query": "ignored",
            "searches": [
                {"query": "iphone", "location": "Berlin"},
                {"query": "fahrrad", "location": "Hamburg", "price_max": "500"}
            ]
        }))
        .unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].params["query"], "iphone");
        assert_eq!(batch[1].params["query"], "fahrrad");
        assert_eq!(batch[1].params["page"], 1);
        assert_eq!(batch[1].params["price_max"], 500);
        assert_eq!(describe_search_request(&batch), "2 Kleinanzeigen searches");
    }

    #[test]
    fn validates_inputs_and_decodes_percent_encoding_like_uri_component() {
        assert!(build_search_plan(&json!({}))
            .unwrap_err()
            .to_string()
            .contains("query is required"));
        assert!(build_search_plan(&json!({"query": ""}))
            .unwrap_err()
            .to_string()
            .contains("query is required"));
        assert!(build_search_plan(&json!({"query": "iphone", "page": 101}))
            .unwrap_err()
            .to_string()
            .contains("page must be between 1 and 100"));
        assert!(
            build_search_plan(&json!({"query": "iphone", "location": 123}))
                .unwrap_err()
                .to_string()
                .contains("location must be a string")
        );
        assert!(
            build_search_plan(&json!({"query": "iphone", "price_min": "10.5"}))
                .unwrap_err()
                .to_string()
                .contains("price_min must be an integer")
        );
        assert!(
            build_search_plan(&json!({"query": "iphone", "price_min": 500, "price_max": 50}))
                .unwrap_err()
                .to_string()
                .contains("price_max cannot be less than price_min")
        );
        assert!(build_search_plan(&json!({"query": "😀".repeat(251)}))
            .unwrap_err()
            .to_string()
            .contains("query must be 500 characters or fewer"));
        assert_eq!(
            build_search_plan(&json!({"query": "\u{FEFF}iphone\u{FEFF}"})).unwrap()[0].params
                ["query"],
            "iphone"
        );
        assert!(
            build_search_plan(&json!({"query": "iphone", "searches": []}))
                .unwrap_err()
                .to_string()
                .contains("searches must contain at least one search")
        );
        assert!(build_search_plan(&json!({"searches": ["iphone"]}))
            .unwrap_err()
            .to_string()
            .contains("searches[0] must be an object"));
        assert!(build_search_plan(
            &json!({"searches": (0..26).map(|_| json!({"query": "x"})).collect::<Vec<_>>()})
        )
        .unwrap_err()
        .to_string()
        .contains("searches cannot contain more than 25"));
        assert!(
            build_search_plan(&json!({"query": "iphone", "searches": "nope"}))
                .unwrap_err()
                .to_string()
                .contains("searches must be an array")
        );
        assert_eq!(decode_input_string("100% baumwolle"), "100% baumwolle");
        assert_eq!(decode_input_string("shoe%2"), "shoe%2");
        assert_eq!(decode_input_string("invalid%FF"), "invalid%FF");
        assert_eq!(decode_input_string("a+b%20c"), "a+b c");
    }
}
