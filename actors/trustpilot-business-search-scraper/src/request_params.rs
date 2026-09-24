use serde_json::{Map, Number, Value};

const LOCALES: &[&str] = &[
    "da-DK", "de-AT", "de-CH", "de-DE", "en-AU", "en-CA", "en-GB", "en-IE", "en-NZ", "en-US",
    "es-ES", "fi-FI", "fr-BE", "nl-BE", "fr-FR", "it-IT", "ja-JP", "nb-NO", "nl-NL", "pl-PL",
    "pt-BR", "pt-PT", "sv-SE",
];
const SEARCH_TYPES: &[&str] = &["company_search", "category"];
const SORT_VALUES: &[&str] = &["reviews_count", "latest_review"];
const DEFAULT_LOCALE: &str = "en-US";
const MAX_PAGE: u64 = 999;
const MAX_PAGES_PER_RUN: u64 = 10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchType {
    CompanySearch,
    Category,
}

impl SearchType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CompanySearch => "company_search",
            Self::Category => "category",
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct RequestPlan {
    pub search_type: SearchType,
    pub endpoint: &'static str,
    pub base_params: Map<String, Value>,
    pub start_page: u64,
    pub max_pages: u64,
}

pub fn build_request_plan(input: &Value) -> Result<RequestPlan, String> {
    let search_type = infer_search_type(input)?;
    let start_page = clean_integer(input.get("page"), "page", 1, MAX_PAGE)?.unwrap_or(1);
    let max_pages =
        clean_integer(input.get("max_pages"), "max_pages", 1, MAX_PAGES_PER_RUN)?.unwrap_or(1);

    if start_page + max_pages - 1 > MAX_PAGE {
        return Err("page plus max_pages cannot exceed page 999".into());
    }

    let mut base_params = Map::new();
    let endpoint = match search_type {
        SearchType::Category => {
            base_params.insert(
                "category".into(),
                Value::String(clean_required_string(
                    input.get("category"),
                    "category",
                    2,
                    120,
                )?),
            );
            base_params.insert(
                "limit".into(),
                json_number(clean_integer(input.get("limit"), "limit", 1, 50)?.unwrap_or(20)),
            );

            if let Some(country) = clean_country(input.get("country"))? {
                base_params.insert("country".into(), Value::String(country));
            }
            if let Some(sort) = clean_enum(input.get("sort"), "sort", SORT_VALUES)? {
                base_params.insert("sort".into(), Value::String(sort));
            }
            if let Some(claimed) = clean_boolean(input.get("claimed"), "claimed")? {
                base_params.insert("claimed".into(), json_number(if claimed { 1 } else { 0 }));
            }
            if let Some(trustscore) = clean_number(input.get("trustscore"), "trustscore", 0.0, 5.0)?
            {
                base_params.insert("trustscore".into(), json_float(trustscore));
            }
            "/trustpilot/businesses"
        }
        SearchType::CompanySearch => {
            base_params.insert(
                "query".into(),
                Value::String(clean_required_string(input.get("query"), "query", 2, 200)?),
            );
            base_params.insert(
                "per_page".into(),
                json_number(clean_integer(input.get("per_page"), "per_page", 1, 50)?.unwrap_or(20)),
            );
            base_params.insert(
                "locale".into(),
                Value::String(
                    clean_enum(input.get("locale"), "locale", LOCALES)?
                        .unwrap_or_else(|| DEFAULT_LOCALE.into()),
                ),
            );

            if let Some(country) = clean_country(input.get("country"))? {
                base_params.insert("country".into(), Value::String(country));
            }
            if let Some(min_rating) = clean_number(input.get("min_rating"), "min_rating", 0.0, 5.0)?
            {
                base_params.insert("min_rating".into(), json_float(min_rating));
            }
            if let Some(min_review_count) = clean_integer(
                input.get("min_review_count"),
                "min_review_count",
                0,
                100_000_000,
            )? {
                base_params.insert("min_review_count".into(), json_number(min_review_count));
            }
            "/trustpilot/company-search"
        }
    };

    Ok(RequestPlan {
        search_type,
        endpoint,
        base_params,
        start_page,
        max_pages,
    })
}

pub fn page_params(plan: &RequestPlan, page: u64) -> Map<String, Value> {
    let mut params = plan.base_params.clone();
    params.insert("page".into(), json_number(page));
    params
}

pub fn describe_request(plan: &RequestPlan) -> String {
    let last_page = plan.start_page + plan.max_pages - 1;
    let pages = if plan.max_pages == 1 {
        format!("page {}", plan.start_page)
    } else {
        format!("pages {}-{last_page}", plan.start_page)
    };
    let field = if plan.search_type == SearchType::Category {
        "category"
    } else {
        "query"
    };
    let value = plan
        .base_params
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default();
    format!("{} \"{value}\" ({pages})", field_label(plan.search_type))
}

fn field_label(search_type: SearchType) -> &'static str {
    match search_type {
        SearchType::CompanySearch => "company query",
        SearchType::Category => "category",
    }
}

fn infer_search_type(input: &Value) -> Result<SearchType, String> {
    if let Some(explicit) = clean_enum(input.get("search_type"), "search_type", SEARCH_TYPES)? {
        return Ok(if explicit == "category" {
            SearchType::Category
        } else {
            SearchType::CompanySearch
        });
    }
    if clean_string(input.get("category"), "category", 120)?.is_some() {
        return Ok(SearchType::Category);
    }
    Ok(SearchType::CompanySearch)
}

fn clean_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };
    let value = decode_input_string(value).trim().to_owned();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        return Err(format!("{field} must be {max_length} characters or fewer"));
    }
    Ok(Some(value))
}

fn clean_required_string(
    value: Option<&Value>,
    field: &str,
    min_length: usize,
    max_length: usize,
) -> Result<String, String> {
    let Some(value) = clean_string(value, field, max_length)? else {
        return Err(format!("{field} is required"));
    };
    if value.encode_utf16().count() < min_length {
        return Err(format!("{field} must be at least {min_length} characters"));
    }
    Ok(value)
}

fn clean_integer(
    value: Option<&Value>,
    field: &str,
    min: u64,
    max: u64,
) -> Result<Option<u64>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let number = match value {
        Value::String(value)
            if !value.trim().is_empty() && value.trim().bytes().all(|b| b.is_ascii_digit()) =>
        {
            value.trim().parse::<f64>().unwrap_or(f64::INFINITY)
        }
        Value::Number(_) => value.as_f64().unwrap_or(f64::NAN),
        _ => f64::NAN,
    };
    if !number.is_finite() || number.fract() != 0.0 || number < 0.0 {
        return Err(format!("{field} must be an integer"));
    }
    if number < min as f64 || number > max as f64 {
        return Err(format!("{field} must be between {min} and {max}"));
    }
    Ok(Some(number as u64))
}

fn clean_number(
    value: Option<&Value>,
    field: &str,
    min: f64,
    max: f64,
) -> Result<Option<f64>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value
        .as_str()
        .is_some_and(|value| value.is_empty() || value == "any")
    {
        return Ok(None);
    }
    let number = match value {
        Value::Number(_) => value.as_f64().unwrap_or(f64::NAN),
        Value::String(value) if !value.trim().is_empty() => {
            parse_js_number(value.trim()).unwrap_or(f64::NAN)
        }
        _ => f64::NAN,
    };
    if !number.is_finite() {
        return Err(format!("{field} must be a number"));
    }
    if number < min || number > max {
        return Err(format!(
            "{field} must be between {} and {}",
            min as i64, max as i64
        ));
    }
    Ok(Some(number))
}

fn clean_boolean(value: Option<&Value>, field: &str) -> Result<Option<bool>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    match value {
        Value::Bool(value) => Ok(Some(*value)),
        Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(Some(true)),
            "false" | "0" | "no" => Ok(Some(false)),
            _ => Err(format!("{field} must be a boolean")),
        },
        _ => Err(format!("{field} must be a boolean")),
    }
}

fn clean_country(value: Option<&Value>) -> Result<Option<String>, String> {
    let Some(country) = clean_string(value, "country", 10)? else {
        return Ok(None);
    };
    let country = country.to_uppercase();
    if country.len() != 2 || !country.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err("country must be an ISO-2 country code, for example US or GB".into());
    }
    Ok(Some(country))
}

fn clean_enum(
    value: Option<&Value>,
    field: &str,
    allowed: &[&str],
) -> Result<Option<String>, String> {
    let Some(value) = clean_string(value, field, 100)? else {
        return Ok(None);
    };
    if !allowed.contains(&value.as_str()) {
        return Err(format!("{field} must be one of: {}", allowed.join(", ")));
    }
    Ok(Some(value))
}

fn parse_js_number(value: &str) -> Option<f64> {
    if let Some(value) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return u64::from_str_radix(value, 16)
            .ok()
            .map(|value| value as f64);
    }
    if let Some(value) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        return u64::from_str_radix(value, 2).ok().map(|value| value as f64);
    }
    if let Some(value) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        return u64::from_str_radix(value, 8).ok().map(|value| value as f64);
    }
    value.parse::<f64>().ok()
}

fn decode_input_string(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return value.into();
            }
            let Some(high) = hex_digit(bytes[index + 1]) else {
                return value.into();
            };
            let Some(low) = hex_digit(bytes[index + 2]) else {
                return value.into();
            };
            decoded.push(high * 16 + low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).unwrap_or_else(|_| value.into())
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn json_number(value: u64) -> Value {
    Value::Number(Number::from(value))
}

fn json_float(value: f64) -> Value {
    Number::from_f64(value)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_company_search_defaults_and_decodes_query() {
        let plan = build_request_plan(
            &json!({"query": "  m%C3%BCller%20%26%20partner  ", "page": "2", "max_pages": 3}),
        )
        .unwrap();
        assert_eq!(plan.search_type, SearchType::CompanySearch);
        assert_eq!(plan.endpoint, "/trustpilot/company-search");
        assert_eq!(plan.base_params["query"], "müller & partner");
        assert_eq!(plan.base_params["per_page"], 20);
        assert_eq!(plan.base_params["locale"], "en-US");
        assert_eq!(page_params(&plan, 2)["page"], 2);
        assert_eq!(
            describe_request(&plan),
            "company query \"müller & partner\" (pages 2-4)"
        );
    }

    #[test]
    fn builds_category_search_and_infers_its_mode() {
        let plan = build_request_plan(&json!({
            "category": " electronics_technology ",
            "country": "us",
            "sort": "reviews_count",
            "claimed": "yes",
            "limit": "30",
            "trustscore": "4.5",
            "max_pages": 2
        }))
        .unwrap();
        assert_eq!(plan.search_type, SearchType::Category);
        assert_eq!(plan.endpoint, "/trustpilot/businesses");
        assert_eq!(plan.base_params["category"], "electronics_technology");
        assert_eq!(plan.base_params["limit"], 30);
        assert_eq!(plan.base_params["country"], "US");
        assert_eq!(plan.base_params["claimed"], 1);
        assert_eq!(plan.base_params["trustscore"], 4.5);
        assert_eq!(plan.max_pages, 2);
    }

    #[test]
    fn validates_required_fields_enums_and_page_bounds() {
        assert_eq!(
            build_request_plan(&json!({"query":"a"})).unwrap_err(),
            "query must be at least 2 characters"
        );
        assert_eq!(
            build_request_plan(&json!({"search_type":"category","category":"a"})).unwrap_err(),
            "category must be at least 2 characters"
        );
        assert!(
            build_request_plan(&json!({"query":"amazon","country":"USA"}))
                .unwrap_err()
                .contains("ISO-2")
        );
        assert_eq!(
            build_request_plan(&json!({"query":"amazon","page":0})).unwrap_err(),
            "page must be between 1 and 999"
        );
        assert_eq!(
            build_request_plan(&json!({"query":"amazon","page":999,"max_pages":2})).unwrap_err(),
            "page plus max_pages cannot exceed page 999"
        );
        assert!(
            build_request_plan(
                &json!({"search_type":"category","category":"electronics","sort":"rating"})
            )
            .unwrap_err()
            .starts_with("sort must be one of:")
        );
    }

    #[test]
    fn rejects_wrong_types_and_out_of_range_numbers() {
        assert_eq!(
            build_request_plan(&json!({"query":5})).unwrap_err(),
            "query must be a string"
        );
        assert_eq!(
            build_request_plan(&json!({"query":"amazon","max_pages":11})).unwrap_err(),
            "max_pages must be between 1 and 10"
        );
        assert_eq!(
            build_request_plan(&json!({"query":"amazon","min_rating":6})).unwrap_err(),
            "min_rating must be between 0 and 5"
        );
        assert_eq!(
            build_request_plan(&json!({"query":"amazon","min_review_count":1.5})).unwrap_err(),
            "min_review_count must be an integer"
        );
    }
}
