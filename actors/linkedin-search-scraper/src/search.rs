use anyhow::{bail, Result};
use serde_json::{json, Map, Number, Value};
use url::Url;

pub const DEFAULT_QUERY: &str = "site:linkedin.com/in founder AI Berlin";
const DEFAULT_NUM: i64 = 10;
const KNOWN_INPUT_KEYS: &[&str] = &[
    "query",
    "num",
    "page",
    "start",
    "hl",
    "lr",
    "gl",
    "cr",
    "safe",
    "dateRestrict",
    "sort",
    "filter",
    "rights",
];

pub fn normalize_input(input: Option<Value>) -> Value {
    let Some(Value::Object(input)) = input else {
        return default_input();
    };

    let mut normalized = Map::new();
    for key in KNOWN_INPUT_KEYS {
        let Some(value) = input.get(*key) else {
            continue;
        };
        if value.is_null() {
            continue;
        }

        if let Some(value) = value.as_str() {
            let value = js_trim(value);
            if !value.is_empty() {
                normalized.insert((*key).to_owned(), Value::String(value.to_owned()));
            }
        } else {
            normalized.insert((*key).to_owned(), value.clone());
        }
    }

    let has_known_input = normalized
        .iter()
        .any(|(key, value)| KNOWN_INPUT_KEYS.contains(&key.as_str()) && !value.is_null());
    if !has_known_input {
        return default_input();
    }

    let mut merged = default_input()
        .as_object()
        .expect("default input is an object")
        .clone();
    let has_query = normalized.get("query").is_some_and(js_truthy);
    for (key, value) in normalized {
        merged.insert(key, value);
    }
    if !has_query {
        merged.insert("query".to_owned(), Value::String(DEFAULT_QUERY.to_owned()));
    }
    Value::Object(merged)
}

fn default_input() -> Value {
    json!({
        "query": DEFAULT_QUERY,
        "num": DEFAULT_NUM,
        "hl": "en",
        "gl": "us",
        "safe": "off",
    })
}

pub fn validate_input(input: &Value) -> Result<()> {
    if input.get("query").is_none_or(|query| !js_truthy(query)) {
        bail!("LinkedIn search query is required.");
    }

    validate_integer_range(input.get("num"), "num", 1, 20)?;
    validate_integer_range(input.get("page"), "page", 1, 10)?;
    validate_integer_range(input.get("start"), "start", 0, 170)?;
    validate_integer_range(input.get("filter"), "filter", 0, 1)?;

    if input.get("page").is_some() && input.get("start").is_some() {
        bail!("Use either page or start for pagination, not both.");
    }
    Ok(())
}

fn validate_integer_range(value: Option<&Value>, field: &str, min: i64, max: i64) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    let number = js_integer(value).filter(|number| *number >= min && *number <= max);
    if number.is_none() {
        bail!("{field} must be an integer from {min} to {max}.");
    }
    Ok(())
}

pub fn limit_result_count(input: &Value, max_results: Option<usize>) -> Result<Value> {
    let Some(max_results) = max_results else {
        return Ok(input.clone());
    };

    let requested_num = input.get("num").and_then(js_integer).unwrap_or(DEFAULT_NUM) as usize;
    let capped_num = requested_num.min(max_results);
    if capped_num >= requested_num {
        return Ok(input.clone());
    }

    let mut limited = input
        .as_object()
        .cloned()
        .expect("normalized input is an object");
    if let Some(page) = input.get("page").and_then(js_integer) {
        let start = (page - 1) * requested_num as i64;
        if start > 170 {
            bail!("Charge limit prevents preserving the requested page offset. Use start pagination with a lower offset or increase the run charge limit.");
        }
        limited.remove("page");
        limited.insert("start".to_owned(), Value::Number(Number::from(start)));
    }
    limited.insert("num".to_owned(), Value::Number(Number::from(capped_num)));
    Ok(Value::Object(limited))
}

pub fn append_search_params(url: &mut Url, input: &Value) {
    const PARAMS: &[&str] = &[
        "query",
        "num",
        "page",
        "start",
        "hl",
        "lr",
        "gl",
        "cr",
        "safe",
        "dateRestrict",
        "sort",
        "filter",
        "rights",
    ];

    let mut query = url.query_pairs_mut();
    for key in PARAMS {
        let Some(value) = input.get(*key) else {
            continue;
        };
        if value.is_null() || value.as_str().is_some_and(str::is_empty) {
            continue;
        }
        if let Some(value) = value.as_bool() {
            if value {
                query.append_pair(key, "1");
            }
            continue;
        }
        query.append_pair(key, &js_string(value));
    }
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_none_or(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => js_number_string(value),
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

fn js_integer(value: &Value) -> Option<i64> {
    let number = value.as_f64()?;
    (number.is_finite()
        && number.fract() == 0.0
        && number >= i64::MIN as f64
        && number <= i64::MAX as f64)
        .then_some(number as i64)
}

fn js_number_string(value: &Number) -> String {
    if let Some(integer) = value.as_i64() {
        return integer.to_string();
    }
    if let Some(integer) = value.as_u64() {
        return integer.to_string();
    }
    if let Some(number) = value.as_f64() {
        if number.is_finite() && number.fract() == 0.0 && number.abs() < 1e21 {
            return format!("{number:.0}");
        }
    }
    value.to_string()
}

fn js_trim(value: &str) -> &str {
    value.trim_matches(|character| {
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
    })
}

pub fn organic_results(response: &Value) -> &[Value] {
    response
        .get("organic_results")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

pub fn output_summary(response: Option<&Value>, saved_results: usize) -> Value {
    let response = response.unwrap_or(&Value::Null);
    let search_information = response.get("search_information").unwrap_or(&Value::Null);
    let pagination = response.get("pagination").unwrap_or(&Value::Null);
    let total_results = response
        .get("total_results")
        .filter(|value| !value.is_null())
        .or_else(|| {
            search_information
                .get("total_results")
                .filter(|value| !value.is_null())
        })
        .cloned()
        .unwrap_or(Value::Null);
    let current_page = pagination
        .get("current_page")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);
    let pages = pagination
        .get("pages")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    json!({
        "results": saved_results,
        "total_results": total_results,
        "current_page": current_page,
        "pages": pages,
        "search_information": search_information,
        "pagination": pagination,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_missing_empty_and_unknown_input_to_prefill_values() {
        let expected = json!({
            "query": "site:linkedin.com/in founder AI Berlin",
            "num": 10,
            "hl": "en",
            "gl": "us",
            "safe": "off",
        });
        assert_eq!(normalize_input(None), expected);
        assert_eq!(normalize_input(Some(json!({}))), expected);
        assert_eq!(normalize_input(Some(json!({"unused": 1}))), expected);
    }

    #[test]
    fn trims_javascript_whitespace_and_preserves_integer_float_inputs() {
        let input = normalize_input(Some(json!({"query": "\u{feff}cto\u{feff}", "num": 10.0})));
        assert_eq!(input.get("query").and_then(Value::as_str), Some("cto"));
        assert!(validate_input(&input).is_ok());
        assert_eq!(
            limit_result_count(&json!({"query":"cto","num":10.0,"page":2.0}), Some(3)).unwrap(),
            json!({"query":"cto","num":3,"start":10})
        );
        let mut url = Url::parse("https://scrappa.co/api/linkedin/search").unwrap();
        append_search_params(&mut url, &json!({"query":"cto","num":2.0}));
        assert_eq!(url.query(), Some("query=cto&num=2"));
    }

    #[test]
    fn applies_defaults_to_partial_and_trims_string_fields() {
        assert_eq!(
            normalize_input(Some(json!({"gl": " de ", "hl": " "}))),
            json!({
                "query": DEFAULT_QUERY,
                "num": 10,
                "hl": "en",
                "gl": "de",
                "safe": "off",
            })
        );
        assert_eq!(
            normalize_input(Some(
                json!({"query": " site:linkedin.com/company fintech Berlin ", "num": 5})
            )),
            json!({
                "query": "site:linkedin.com/company fintech Berlin",
                "num": 5,
                "hl": "en",
                "gl": "us",
                "safe": "off",
            })
        );
    }

    #[test]
    fn forwards_all_supported_parameters_and_omits_missing_values() {
        let input = json!({
            "query": "site:linkedin.com/in founder AI Berlin",
            "num": 20,
            "page": 2,
            "hl": "en",
            "lr": "lang_en",
            "gl": "us",
            "cr": "countryUS",
            "safe": "off",
            "dateRestrict": "m1",
            "sort": "date",
            "filter": 1,
            "rights": "cc_publicdomain"
        });
        let mut url = Url::parse("https://scrappa.co/api/linkedin/search").unwrap();
        append_search_params(&mut url, &input);
        assert_eq!(
            url.query().unwrap(),
            "query=site%3Alinkedin.com%2Fin+founder+AI+Berlin&num=20&page=2&hl=en&lr=lang_en&gl=us&cr=countryUS&safe=off&dateRestrict=m1&sort=date&filter=1&rights=cc_publicdomain"
        );

        let mut sparse = Url::parse("https://scrappa.co/api/linkedin/search").unwrap();
        append_search_params(
            &mut sparse,
            &json!({"query": "cto", "start": null, "gl": ""}),
        );
        assert_eq!(sparse.query(), Some("query=cto"));
    }

    #[test]
    fn validates_numeric_ranges_and_exclusive_pagination() {
        assert!(validate_input(&json!({"query": "cto", "num": 3, "gl": "de"})).is_ok());
        assert!(validate_input(&json!({"query": "cto", "num": 21}))
            .unwrap_err()
            .to_string()
            .contains("num must be an integer from 1 to 20"));
        assert!(validate_input(&json!({"query": "cto", "page": 11}))
            .unwrap_err()
            .to_string()
            .contains("page must be an integer from 1 to 10"));
        assert!(validate_input(&json!({"query": "cto", "start": 171}))
            .unwrap_err()
            .to_string()
            .contains("start must be an integer from 0 to 170"));
        assert!(
            validate_input(&json!({"query": "cto", "page": 1, "start": 0}))
                .unwrap_err()
                .to_string()
                .contains("Use either page or start")
        );
    }

    #[test]
    fn caps_results_without_losing_page_offset() {
        let input = json!({"query": "cto", "num": 10, "page": 2});
        assert_eq!(
            limit_result_count(&input, Some(3)).unwrap(),
            json!({"query": "cto", "num": 3, "start": 10})
        );
        assert_eq!(limit_result_count(&input, Some(10)).unwrap(), input);
        assert_eq!(limit_result_count(&input, None).unwrap(), input);
        assert!(
            limit_result_count(&json!({"query": "cto", "num": 20, "page": 10}), Some(3))
                .unwrap_err()
                .to_string()
                .contains("Charge limit prevents preserving the requested page offset")
        );
    }

    #[test]
    fn extracts_only_array_organic_results_and_summarizes_response_metadata() {
        let response = json!({
            "organic_results": [{"title":"Founder"}],
            "search_information": {"total_results": 120, "query_displayed": "cto"},
            "pagination": {"current_page": 2, "pages": [{"page":1},{"page":2}]}
        });
        assert_eq!(organic_results(&response), &[json!({"title":"Founder"})]);
        assert!(organic_results(&json!({"organic_results":null})).is_empty());
        assert_eq!(
            output_summary(Some(&response), 1),
            json!({
                "results": 1,
                "total_results": 120,
                "current_page": 2,
                "pages": 2,
                "search_information": {"total_results":120,"query_displayed":"cto"},
                "pagination": {"current_page":2,"pages":[{"page":1},{"page":2}]}
            })
        );
        assert_eq!(
            output_summary(None, 0),
            json!({"results":0,"total_results":null,"current_page":null,"pages":0,"search_information":null,"pagination":null})
        );
    }
}
