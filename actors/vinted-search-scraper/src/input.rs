use anyhow::{anyhow, bail, Result};
use percent_encoding::percent_decode_str;
use serde_json::{Map, Number, Value};

const VALID_COUNTRIES: &[&str] = &[
    "FR", "DE", "ES", "IT", "NL", "BE", "AT", "PL", "CZ", "LT", "LU", "SK", "HU", "RO", "PT", "SE",
    "DK", "FI", "US",
];
const VALID_ORDERS: &[&str] = &[
    "relevance",
    "newest_first",
    "price_low_to_high",
    "price_high_to_low",
];
const FILTER_FIELDS: &[&str] = &[
    "brand_ids",
    "catalog_ids",
    "color_ids",
    "size_ids",
    "material_ids",
    "status_ids",
];
const MAX_PAGE: u64 = 999;
const DEFAULT_PER_PAGE: u64 = 24;
const DEFAULT_MAX_PAGES: u64 = 1;
const MAX_PAGES_PER_RUN: u64 = 20;

#[derive(Debug, PartialEq)]
pub struct VintedSearchPlan {
    pub base_params: Map<String, Value>,
    pub start_page: u64,
    pub per_page: u64,
    pub max_pages: u64,
}

fn decode_input_string(value: &str) -> String {
    if !value.contains('%') {
        return value.to_owned();
    }

    percent_decode_str(value)
        .decode_utf8()
        .map(|decoded| decoded.into_owned())
        .unwrap_or_else(|_| value.to_owned())
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
    let trimmed = decode_input_string(value).trim().to_owned();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(trimmed))
}

fn clean_integer(value: Option<&Value>, field: &str, min: u64, max: u64) -> Result<Option<u64>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }

    let normalized = match value {
        Value::String(raw)
            if !raw.trim().is_empty() && raw.trim().chars().all(|ch| ch.is_ascii_digit()) =>
        {
            raw.trim().parse::<f64>().ok()
        }
        Value::Number(number) => number.as_f64(),
        _ => None,
    }
    .filter(|number| number.is_finite() && number.fract() == 0.0)
    .ok_or_else(|| anyhow!("{field} must be an integer"))?;

    if normalized < min as f64 || normalized > max as f64 {
        bail!("{field} must be between {min} and {max}");
    }
    Ok(Some(normalized as u64))
}

fn clean_number(value: Option<&Value>, field: &str, min: f64) -> Result<Option<Number>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }

    let normalized = match value {
        Value::String(raw) if raw.trim().is_empty() => Some(0.0),
        Value::String(raw) => raw.trim().parse::<f64>().ok(),
        Value::Number(number) => number.as_f64(),
        _ => None,
    }
    .filter(|number| number.is_finite())
    .ok_or_else(|| anyhow!("{field} must be a number"))?;

    if normalized < min {
        bail!("{field} must be at least {min}");
    }
    let number = if normalized.fract() == 0.0 && normalized < u64::MAX as f64 {
        Number::from(normalized as u64)
    } else {
        Number::from_f64(normalized).ok_or_else(|| anyhow!("{field} must be a number"))?
    };
    Ok(Some(number))
}

fn clean_country(value: Option<&Value>) -> Result<String> {
    let Some(country) = clean_string(value, "country", 2)? else {
        return Ok("FR".to_owned());
    };
    let normalized = country.to_uppercase();
    if !VALID_COUNTRIES.contains(&normalized.as_str()) {
        bail!("country must be one of: {}", VALID_COUNTRIES.join(", "));
    }
    Ok(normalized)
}

fn clean_order(value: Option<&Value>) -> Result<Option<String>> {
    let Some(order) = clean_string(value, "order", 30)? else {
        return Ok(None);
    };
    if !VALID_ORDERS.contains(&order.as_str()) {
        bail!("order must be one of: {}", VALID_ORDERS.join(", "));
    }
    Ok(Some(order))
}

fn clean_id_list(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    let max_length = if field == "brand_ids" || field == "catalog_ids" {
        500
    } else {
        200
    };
    let Some(raw) = clean_string(value, field, max_length)? else {
        return Ok(None);
    };
    let ids = raw
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Ok(None);
    }
    if !ids
        .iter()
        .all(|id| id.chars().all(|ch| ch.is_ascii_digit()))
    {
        bail!("{field} must be a comma-separated list of numeric IDs");
    }
    Ok(Some(ids.join(",")))
}

fn object_input(input: &Value) -> &Map<String, Value> {
    input.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

pub fn build_search_plan(input: &Value) -> Result<VintedSearchPlan> {
    let input = object_input(input);
    let mut base_params = Map::new();
    base_params.insert(
        "country".to_owned(),
        Value::String(clean_country(input.get("country"))?),
    );

    let per_page =
        clean_integer(input.get("per_page"), "per_page", 1, 100)?.unwrap_or(DEFAULT_PER_PAGE);
    base_params.insert("per_page".to_owned(), Value::from(per_page));

    if let Some(query) = clean_string(input.get("query"), "query", 500)? {
        base_params.insert("query".to_owned(), Value::String(query));
    }
    if let Some(order) = clean_order(input.get("order"))? {
        base_params.insert("order".to_owned(), Value::String(order));
    }
    for field in FILTER_FIELDS {
        if let Some(value) = clean_id_list(input.get(*field), field)? {
            base_params.insert((*field).to_owned(), Value::String(value));
        }
    }

    let price_from = clean_number(input.get("price_from"), "price_from", 0.0)?;
    let price_to = clean_number(input.get("price_to"), "price_to", 0.0)?;
    if let Some(value) = &price_from {
        base_params.insert("price_from".to_owned(), Value::Number(value.clone()));
    }
    if let Some(value) = &price_to {
        base_params.insert("price_to".to_owned(), Value::Number(value.clone()));
    }
    if let (Some(price_from), Some(price_to)) = (&price_from, &price_to) {
        if price_from.as_f64().unwrap_or_default() > price_to.as_f64().unwrap_or_default() {
            bail!("price_from cannot be greater than price_to");
        }
    }

    let start_page = clean_integer(input.get("page"), "page", 1, MAX_PAGE)?.unwrap_or(1);
    let max_pages = clean_integer(input.get("max_pages"), "max_pages", 1, MAX_PAGES_PER_RUN)?
        .unwrap_or(DEFAULT_MAX_PAGES);
    if start_page + max_pages - 1 > MAX_PAGE {
        bail!("page plus max_pages cannot exceed page 999");
    }

    Ok(VintedSearchPlan {
        base_params,
        start_page,
        per_page,
        max_pages,
    })
}

pub fn build_page_params(plan: &VintedSearchPlan, page: u64) -> Map<String, Value> {
    let mut params = plan.base_params.clone();
    params.insert("page".to_owned(), Value::from(page));
    params
}

pub fn describe_search_request(plan: &VintedSearchPlan) -> String {
    let query = plan
        .base_params
        .get("query")
        .and_then(Value::as_str)
        .map(|query| format!("\"{query}\""))
        .unwrap_or_else(|| "all listings".to_owned());
    let page_description = if plan.max_pages == 1 {
        format!("page {}", plan.start_page)
    } else {
        format!(
            "pages {}-{}",
            plan.start_page,
            plan.start_page + plan.max_pages - 1
        )
    };
    format!(
        "{query} in {} ({page_description}, {}/page)",
        plan.base_params
            .get("country")
            .and_then(Value::as_str)
            .unwrap_or("FR"),
        plan.per_page
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_default_plan_and_page_params() {
        let plan = build_search_plan(&json!({"query": " nike shoes "})).unwrap();
        assert_eq!(
            plan.base_params,
            serde_json::from_value(json!({
                "country": "FR",
                "per_page": 24,
                "query": "nike shoes"
            }))
            .unwrap()
        );
        assert_eq!(plan.start_page, 1);
        assert_eq!(plan.per_page, 24);
        assert_eq!(plan.max_pages, 1);
        assert_eq!(
            build_page_params(&plan, 1).get("page"),
            Some(&Value::from(1))
        );
        assert_eq!(
            describe_search_request(&plan),
            "\"nike shoes\" in FR (page 1, 24/page)"
        );
    }

    #[test]
    fn normalizes_encoded_query_numeric_strings_filters_and_prices() {
        let plan = build_search_plan(&json!({
            "query": "zara%20dress",
            "country": "de",
            "page": "2",
            "per_page": "50",
            "max_pages": "3",
            "order": "newest_first",
            "brand_ids": " 53,  88 ",
            "price_from": "10.5",
            "price_to": "80"
        }))
        .unwrap();

        assert_eq!(plan.base_params.get("country"), Some(&json!("DE")));
        assert_eq!(plan.base_params.get("per_page"), Some(&json!(50)));
        assert_eq!(plan.base_params.get("query"), Some(&json!("zara dress")));
        assert_eq!(plan.base_params.get("brand_ids"), Some(&json!("53,88")));
        assert_eq!(plan.base_params.get("price_from"), Some(&json!(10.5)));
        assert_eq!(plan.base_params.get("price_to"), Some(&json!(80)));
        assert_eq!(plan.start_page, 2);
        assert_eq!(plan.max_pages, 3);
        assert_eq!(
            describe_search_request(&plan),
            "\"zara dress\" in DE (pages 2-4, 50/page)"
        );
    }

    #[test]
    fn supports_filter_only_searches_and_preserves_literal_percent() {
        let filter_only = build_search_plan(&json!({
            "catalog_ids": "5",
            "brand_ids": " ",
            "price_to": 25
        }))
        .unwrap();
        assert_eq!(filter_only.base_params.get("query"), None);
        assert_eq!(
            describe_search_request(&filter_only),
            "all listings in FR (page 1, 24/page)"
        );

        let literal_percent = build_search_plan(&json!({"query": "100% cotton"})).unwrap();
        assert_eq!(
            literal_percent.base_params.get("query"),
            Some(&json!("100% cotton"))
        );
    }

    #[test]
    fn validates_country_sorting_ids_price_and_pagination() {
        assert!(build_search_plan(&json!({"country": "GB"}))
            .unwrap_err()
            .to_string()
            .contains("country must be one of"));
        assert!(build_search_plan(&json!({"order": "oldest_first"}))
            .unwrap_err()
            .to_string()
            .contains("order must be one of"));
        assert!(build_search_plan(&json!({"brand_ids": "12,nike"}))
            .unwrap_err()
            .to_string()
            .contains("brand_ids must be a comma-separated list of numeric IDs"));
        assert!(
            build_search_plan(&json!({"price_from": 90, "price_to": 20}))
                .unwrap_err()
                .to_string()
                .contains("price_from cannot be greater than price_to")
        );
        assert!(build_search_plan(&json!({"page": 0}))
            .unwrap_err()
            .to_string()
            .contains("page must be between 1 and 999"));
        assert!(build_search_plan(&json!({"max_pages": 21}))
            .unwrap_err()
            .to_string()
            .contains("max_pages must be between 1 and 20"));
        assert!(build_search_plan(&json!({"page": 990, "max_pages": 20}))
            .unwrap_err()
            .to_string()
            .contains("page plus max_pages cannot exceed page 999"));
    }

    #[test]
    fn string_fields_and_number_fields_reject_wrong_types() {
        assert!(build_search_plan(&json!({"query": 12}))
            .unwrap_err()
            .to_string()
            .contains("query must be a string"));
        assert!(build_search_plan(&json!({"price_from": "not a number"}))
            .unwrap_err()
            .to_string()
            .contains("price_from must be a number"));
        assert_eq!(
            build_search_plan(&json!({"price_from": "  "}))
                .unwrap()
                .base_params
                .get("price_from"),
            Some(&json!(0))
        );
    }
}
