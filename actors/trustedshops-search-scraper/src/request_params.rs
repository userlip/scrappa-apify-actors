use percent_encoding::percent_decode_str;
use serde_json::{Map, Value};

const VALID_MARKETS: [&str; 11] = [
    "DEU", "GBR", "AUT", "CHE", "NLD", "ESP", "ITA", "FRA", "BEL", "POL", "PRT",
];
const MAX_PAGE: i64 = 100;
const DEFAULT_MAX_PAGES: i64 = 1;
const MAX_PAGES_PER_RUN: i64 = 10;

#[derive(Debug, PartialEq)]
pub struct SearchPlan {
    pub base_params: Map<String, Value>,
    pub start_page: i64,
    pub max_pages: i64,
}

pub fn build_search_plan(input: &Value) -> std::result::Result<SearchPlan, String> {
    let input = input.as_object();
    let query = clean_required_string(input.and_then(|input| input.get("q")), "q", 2, 200)?;
    let market = clean_market(input.and_then(|input| input.get("market")))?
        .unwrap_or_else(|| "DEU".to_owned());
    let start_page = clean_integer(
        input.and_then(|input| input.get("page")),
        "page",
        0,
        MAX_PAGE,
    )?
    .unwrap_or(0);
    let max_pages = clean_integer(
        input.and_then(|input| input.get("max_pages")),
        "max_pages",
        1,
        MAX_PAGES_PER_RUN,
    )?
    .unwrap_or(DEFAULT_MAX_PAGES);

    if start_page + max_pages - 1 > MAX_PAGE {
        return Err("page plus max_pages cannot exceed page 100".to_owned());
    }

    let mut base_params = Map::new();
    base_params.insert("q".to_owned(), Value::String(query));
    base_params.insert("market".to_owned(), Value::String(market));
    Ok(SearchPlan {
        base_params,
        start_page,
        max_pages,
    })
}

pub fn page_params(plan: &SearchPlan, page: i64) -> Map<String, Value> {
    let mut params = plan.base_params.clone();
    params.insert("page".to_owned(), Value::from(page));
    params
}

pub fn describe_request(plan: &SearchPlan) -> String {
    let query = plan.base_params["q"].as_str().unwrap_or_default();
    let market = plan.base_params["market"].as_str().unwrap_or_default();
    let page_description = if plan.max_pages == 1 {
        format!("page {}", plan.start_page)
    } else {
        format!(
            "pages {}-{}",
            plan.start_page,
            plan.start_page + plan.max_pages - 1
        )
    };
    format!("\"{query}\" in {market} ({page_description})")
}

fn clean_required_string(
    value: Option<&Value>,
    field: &str,
    min_length: usize,
    max_length: usize,
) -> Result<String, String> {
    let value =
        clean_string(value, field, max_length)?.ok_or_else(|| format!("{field} is required"))?;
    if value.encode_utf16().count() < min_length {
        return Err(format!("{field} must be at least {min_length} characters"));
    }
    Ok(value)
}

fn clean_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };
    if value.is_empty() {
        return Ok(None);
    }

    let value = trim_javascript_whitespace(&decode_input_string(value)).to_owned();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        return Err(format!("{field} must be {max_length} characters or fewer"));
    }
    Ok(Some(value))
}

fn decode_input_string(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return value.to_owned();
            }
            index += 3;
        } else {
            index += 1;
        }
    }

    percent_decode_str(value)
        .decode_utf8()
        .map(|decoded| decoded.into_owned())
        .unwrap_or_else(|_| value.to_owned())
}

fn clean_market(value: Option<&Value>) -> Result<Option<String>, String> {
    let Some(value) = clean_string(value, "market", 3)? else {
        return Ok(None);
    };
    let market = value.to_uppercase();
    if !VALID_MARKETS.contains(&market.as_str()) {
        return Err(format!(
            "market must be one of: {}",
            VALID_MARKETS.join(", ")
        ));
    }
    Ok(Some(market))
}

fn clean_integer(
    value: Option<&Value>,
    field: &str,
    min: i64,
    max: i64,
) -> Result<Option<i64>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }

    let number = if let Some(value) = value.as_str() {
        let digits = trim_javascript_whitespace(value);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(format!("{field} must be an integer"));
        }
        digits
            .parse::<f64>()
            .map_err(|_| format!("{field} must be an integer"))?
    } else if let Some(number) = value.as_f64() {
        number
    } else {
        return Err(format!("{field} must be an integer"));
    };

    if !number.is_finite() || number.fract() != 0.0 {
        return Err(format!("{field} must be an integer"));
    }
    if number < min as f64 || number > max as f64 {
        return Err(format!("{field} must be between {min} and {max}"));
    }
    Ok(Some(number as i64))
}

fn trim_javascript_whitespace(value: &str) -> &str {
    value.trim_matches(|character| {
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_default_and_normalized_plans() {
        let plan = build_search_plan(&json!({"q": " zalando "})).unwrap();
        assert_eq!(
            plan.base_params,
            json!({"q": "zalando", "market": "DEU"})
                .as_object()
                .unwrap()
                .clone()
        );
        assert_eq!((plan.start_page, plan.max_pages), (0, 1));
        assert_eq!(
            page_params(&plan, 0),
            json!({"q": "zalando", "market": "DEU", "page": 0})
                .as_object()
                .unwrap()
                .clone()
        );

        let plan = build_search_plan(&json!({
            "q": "m%C3%BCller & partner",
            "market": "fra",
            "page": "2",
            "max_pages": "3"
        }))
        .unwrap();
        assert_eq!(plan.base_params["q"], "müller & partner");
        assert_eq!(plan.base_params["market"], "FRA");
        assert_eq!((plan.start_page, plan.max_pages), (2, 3));
        assert_eq!(
            describe_request(&plan),
            "\"müller & partner\" in FRA (pages 2-4)"
        );
    }

    #[test]
    fn decodes_only_valid_uri_escapes_and_preserves_ampersands() {
        let plan = build_search_plan(&json!({"q": " H&M "})).unwrap();
        assert_eq!(plan.base_params["q"], "H&M");

        let plan = build_search_plan(&json!({"q": "100% organic"})).unwrap();
        assert_eq!(plan.base_params["q"], "100% organic");

        let plan = build_search_plan(&json!({"q": "%20m%C3%BCller%ZZ"})).unwrap();
        assert_eq!(plan.base_params["q"], "%20m%C3%BCller%ZZ");

        let plan = build_search_plan(&json!({"q": "\u{FEFF}zalando\u{FEFF}"})).unwrap();
        assert_eq!(plan.base_params["q"], "zalando");
    }

    #[test]
    fn matches_input_schema_contract_and_prefill() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["required"], json!(["q"]));
        assert_eq!(schema["properties"]["q"]["prefill"], "zalando");
        assert_eq!(schema["properties"]["market"]["enum"], json!(VALID_MARKETS));
        assert_eq!(schema["properties"]["page"]["minimum"], 0);
        assert_eq!(schema["properties"]["page"]["maximum"], 100);
        assert_eq!(schema["properties"]["max_pages"]["minimum"], 1);
        assert_eq!(schema["properties"]["max_pages"]["maximum"], 10);
    }

    #[test]
    fn rejects_invalid_query_market_and_page_inputs() {
        assert_eq!(
            build_search_plan(&json!({"q": "a"})).unwrap_err(),
            "q must be at least 2 characters"
        );
        assert_eq!(
            build_search_plan(&json!({"q": "ok", "market": "USA"})).unwrap_err(),
            format!("market must be one of: {}", VALID_MARKETS.join(", "))
        );
        assert_eq!(
            build_search_plan(&json!({"q": "ok", "page": 101})).unwrap_err(),
            "page must be between 0 and 100"
        );
        assert_eq!(
            build_search_plan(&json!({"q": "ok", "page": "-1"})).unwrap_err(),
            "page must be an integer"
        );
        assert_eq!(
            build_search_plan(&json!({"q": "ok", "max_pages": 11})).unwrap_err(),
            "max_pages must be between 1 and 10"
        );
        assert_eq!(
            build_search_plan(&json!({"q": "ok", "page": 95, "max_pages": 10})).unwrap_err(),
            "page plus max_pages cannot exceed page 100"
        );
        assert_eq!(
            build_search_plan(&json!({"q": 42})).unwrap_err(),
            "q must be a string"
        );
    }

    #[test]
    fn javascript_string_length_counts_utf16_units() {
        assert_eq!(
            build_search_plan(&json!({"q": "é"})).unwrap_err(),
            "q must be at least 2 characters"
        );
        assert!(build_search_plan(&json!({"q": "😀"})).is_ok());
        let too_long = "😀".repeat(101);
        assert_eq!(
            build_search_plan(&json!({"q": too_long})).unwrap_err(),
            "q must be 200 characters or fewer"
        );
    }

    #[test]
    fn returns_a_concrete_error_for_non_object_input() {
        let error = build_search_plan(&json!(null)).unwrap_err();
        assert_eq!(error, "q is required");
    }
}
