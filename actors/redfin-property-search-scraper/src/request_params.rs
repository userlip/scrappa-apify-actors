use anyhow::{anyhow, Result};
use serde_json::{Map, Number, Value};

const MAX_SEARCHES_PER_RUN: usize = 25;
const MAX_NUM_HOMES: f64 = 450.0;
const VALID_REGION_TYPES: &[f64] = &[1.0, 2.0, 4.0, 5.0, 6.0];
const VALID_STATUSES: &[f64] = &[1.0, 9.0, 130.0, 131.0];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SearchRequest {
    pub(crate) params: Map<String, Value>,
    pub(crate) index: usize,
}

pub(crate) fn build_search_requests(input: &Value) -> Result<Vec<SearchRequest>> {
    let input = input.as_object();
    let searches = input.and_then(|input| input.get("searches"));

    if let Some(Value::Array(searches)) = searches {
        if searches.is_empty() {
            return Err(anyhow!("searches must include at least one search"));
        }
        if searches.len() > MAX_SEARCHES_PER_RUN {
            return Err(anyhow!(
                "searches cannot include more than {MAX_SEARCHES_PER_RUN} searches per run"
            ));
        }

        return searches
            .iter()
            .enumerate()
            .map(|(index, search)| {
                let search = search
                    .as_object()
                    .ok_or_else(|| anyhow!("searches[{index}] must be an object"))?;
                Ok(SearchRequest {
                    params: build_single_search_params(search, &format!("searches[{index}]."))?,
                    index,
                })
            })
            .collect();
    }

    if searches.is_some() {
        return Err(anyhow!("searches must be an array of search objects"));
    }

    let empty = Map::new();
    let params = match input {
        Some(input) => build_single_search_params(input, "")?,
        None => build_single_search_params(&empty, "")?,
    };
    Ok(vec![SearchRequest { params, index: 0 }])
}

fn build_single_search_params(
    input: &Map<String, Value>,
    prefix: &str,
) -> Result<Map<String, Value>> {
    let region_id = clean_required_integer(
        input.get("region_id"),
        &format!("{prefix}region_id"),
        1.0,
        None,
    )?;
    let region_type = clean_enum_integer(
        input.get("region_type"),
        &format!("{prefix}region_type"),
        VALID_REGION_TYPES,
    )?
    .ok_or_else(|| anyhow!("{prefix}region_type is required"))?;
    let market =
        clean_required_string(input.get("market"), &format!("{prefix}market"), 30)?.to_lowercase();
    if !market
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err(anyhow!(
            "{prefix}market must contain only lowercase letters and numbers"
        ));
    }

    let mut params = Map::new();
    params.insert("region_id".to_owned(), number_value(region_id));
    params.insert("region_type".to_owned(), number_value(region_type));
    params.insert("market".to_owned(), Value::String(market));

    let min_price = clean_integer(
        input.get("min_price"),
        &format!("{prefix}min_price"),
        0.0,
        None,
    )?;
    let max_price = clean_integer(
        input.get("max_price"),
        &format!("{prefix}max_price"),
        0.0,
        None,
    )?;
    if matches!((min_price, max_price), (Some(min), Some(max)) if max < min) {
        return Err(anyhow!(
            "{prefix}max_price must be greater than or equal to min_price"
        ));
    }
    insert_number(&mut params, "min_price", min_price);
    insert_number(&mut params, "max_price", max_price);
    insert_number(
        &mut params,
        "num_beds",
        clean_integer(
            input.get("num_beds"),
            &format!("{prefix}num_beds"),
            0.0,
            Some(10.0),
        )?,
    );
    insert_number(
        &mut params,
        "num_baths",
        clean_number(
            input.get("num_baths"),
            &format!("{prefix}num_baths"),
            0.0,
            10.0,
        )?,
    );
    if let Some(property_types) = clean_comma_separated_property_types(
        input.get("property_types"),
        &format!("{prefix}property_types"),
    )? {
        params.insert("property_types".to_owned(), Value::String(property_types));
    }
    insert_number(
        &mut params,
        "status",
        clean_enum_integer(
            input.get("status"),
            &format!("{prefix}status"),
            VALID_STATUSES,
        )?,
    );
    insert_number(
        &mut params,
        "sold_within_days",
        clean_integer(
            input.get("sold_within_days"),
            &format!("{prefix}sold_within_days"),
            1.0,
            Some(365.0),
        )?,
    );
    insert_number(
        &mut params,
        "num_homes",
        clean_integer(
            input.get("num_homes"),
            &format!("{prefix}num_homes"),
            1.0,
            Some(MAX_NUM_HOMES),
        )?,
    );
    insert_number(
        &mut params,
        "page",
        clean_integer(input.get("page"), &format!("{prefix}page"), 1.0, None)?,
    );

    Ok(params)
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    let value = value
        .as_str()
        .ok_or_else(|| anyhow!("{field} must be a string"))?
        .trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        return Err(anyhow!("{field} must be {max_length} characters or fewer"));
    }
    Ok(Some(value.to_owned()))
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_integer(
    value: Option<&Value>,
    field: &str,
    min: f64,
    max: Option<f64>,
) -> Result<Option<f64>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }

    let number = match value {
        Value::String(value) if is_integer_string(value.trim()) => value.trim().parse::<f64>().ok(),
        Value::Number(value) => value.as_f64(),
        _ => None,
    }
    .filter(|number| number.is_finite() && number.fract() == 0.0)
    .ok_or_else(|| anyhow!("{field} must be an integer"))?;

    validate_range(number, field, min, max)?;
    Ok(Some(number))
}

fn clean_required_integer(
    value: Option<&Value>,
    field: &str,
    min: f64,
    max: Option<f64>,
) -> Result<f64> {
    clean_integer(value, field, min, max)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_number(value: Option<&Value>, field: &str, min: f64, max: f64) -> Result<Option<f64>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    let number = match value {
        Value::Number(value) => value.as_f64(),
        Value::String(value) if !value.trim().is_empty() => parse_js_number(value.trim()),
        _ => None,
    }
    .filter(|number| number.is_finite())
    .ok_or_else(|| anyhow!("{field} must be a number"))?;
    validate_range(number, field, min, Some(max))?;
    Ok(Some(number))
}

fn parse_js_number(value: &str) -> Option<f64> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16)
            .ok()
            .map(|number| number as f64);
    }
    if let Some(binary) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        return u64::from_str_radix(binary, 2)
            .ok()
            .map(|number| number as f64);
    }
    if let Some(octal) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        return u64::from_str_radix(octal, 8)
            .ok()
            .map(|number| number as f64);
    }
    value.parse().ok()
}

fn validate_range(number: f64, field: &str, min: f64, max: Option<f64>) -> Result<()> {
    if number < min || max.is_some_and(|max| number > max) {
        let range = max.map_or_else(
            || format!("at least {min}"),
            |max| format!("between {min} and {max}"),
        );
        return Err(anyhow!("{field} must be {range}"));
    }
    Ok(())
}

fn is_integer_string(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn clean_enum_integer(value: Option<&Value>, field: &str, values: &[f64]) -> Result<Option<f64>> {
    let number = clean_integer(value, field, 0.0, None)?;
    if let Some(number) = number {
        if !values.contains(&number) {
            let allowed = values
                .iter()
                .map(|value| format_number(*value))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(anyhow!("{field} must be one of: {allowed}"));
        }
    }
    Ok(number)
}

fn clean_comma_separated_property_types(
    value: Option<&Value>,
    field: &str,
) -> Result<Option<String>> {
    let Some(value) = clean_string(value, field, 20)? else {
        return Ok(None);
    };
    if value.is_empty()
        || value
            .split(',')
            .all(|part| part.len() == 1 && matches!(part.as_bytes()[0], b'1'..=b'8'))
    {
        return Ok(Some(value));
    }
    Err(anyhow!("{field} must be comma-separated numbers 1-8"))
}

fn insert_number(params: &mut Map<String, Value>, key: &str, value: Option<f64>) {
    if let Some(value) = value {
        params.insert(key.to_owned(), number_value(value));
    }
}

fn number_value(value: f64) -> Value {
    let number = integral_number(value)
        .unwrap_or_else(|| Number::from_f64(value).expect("validated finite number"));
    Value::Number(number)
}

fn integral_number(value: f64) -> Option<Number> {
    if value.fract() != 0.0 {
        return None;
    }
    if (i64::MIN as f64..9_223_372_036_854_775_808.0).contains(&value) {
        return Some(Number::from(value as i64));
    }
    if (0.0..18_446_744_073_709_551_616.0).contains(&value) {
        return Some(Number::from(value as u64));
    }
    None
}

pub(crate) fn describe_search(params: &Map<String, Value>) -> String {
    let required = format!(
        "region {} ({}, type {})",
        value_string(params.get("region_id")),
        value_string(params.get("market")),
        value_string(params.get("region_type")),
    );
    let mut filters = params
        .iter()
        .filter(|(field, _)| !matches!(field.as_str(), "region_id" | "region_type" | "market"))
        .map(|(field, value)| (field, value))
        .collect::<Vec<_>>();
    filters.sort_by(|(left, _), (right, _)| left.cmp(right));
    if filters.is_empty() {
        return required;
    }
    let filters = filters
        .into_iter()
        .map(|(field, value)| format!("{field}={}", value_string(Some(value))))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{required} ({filters})")
}

fn value_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        Some(Value::Null) | None => "null".to_owned(),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| value_string(Some(value)))
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".to_owned(),
    }
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_single_and_batched_searches() {
        let requests = build_search_requests(&json!({
            "region_id":"16163", "region_type":"6", "market":" Seattle ",
            "min_price":"100000", "max_price":800000, "num_beds":"2", "num_baths":"1.5",
            "property_types":"1,2,3", "status":"9", "sold_within_days":"30", "num_homes":"50", "page":"2"
        })).unwrap();
        assert_eq!(requests[0].params["market"], "seattle");
        assert_eq!(requests[0].params["region_id"], 16163);
        assert_eq!(requests[0].params["num_baths"], 1.5);
        assert_eq!(requests[0].params["property_types"], "1,2,3");
        assert_eq!(requests[0].index, 0);

        let batch = build_search_requests(&json!({"market":"ignored", "searches":[
            {"region_id":16163,"region_type":6,"market":"seattle","num_homes":25},
            {"region_id":11203,"region_type":6,"market":"socal","min_price":500000}
        ]}))
        .unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[1].index, 1);
        assert_eq!(batch[1].params["region_id"], 11203);
        assert!(!batch[1].params.contains_key("num_homes"));
    }

    #[test]
    fn validates_required_and_optional_fields() {
        for (input, message) in [
            (
                json!({"region_type":6,"market":"seattle"}),
                "region_id is required",
            ),
            (
                json!({"region_id":16163,"market":"seattle"}),
                "region_type is required",
            ),
            (
                json!({"region_id":16163,"region_type":6,"market":""}),
                "market is required",
            ),
            (
                json!({"region_id":16163,"region_type":99,"market":"seattle"}),
                "region_type must be one of: 1, 2, 4, 5, 6",
            ),
            (
                json!({"region_id":16163,"region_type":6,"market":"Seattle WA"}),
                "market must contain only lowercase letters and numbers",
            ),
            (
                json!({"region_id":16163,"region_type":6,"market":"seattle","min_price":500,"max_price":400}),
                "max_price must be greater than or equal to min_price",
            ),
            (
                json!({"region_id":16163,"region_type":6,"market":"seattle","property_types":"1,9"}),
                "property_types must be comma-separated numbers 1-8",
            ),
            (
                json!({"region_id":16163,"region_type":6,"market":"seattle","num_homes":451}),
                "num_homes must be between 1 and 450",
            ),
        ] {
            assert!(build_search_requests(&input)
                .unwrap_err()
                .to_string()
                .contains(message));
        }
    }

    #[test]
    fn validates_batch_constraints_and_integer_forms() {
        for (input, message) in [
            (
                json!({"searches":[]}),
                "searches must include at least one search",
            ),
            (
                json!({"searches":["seattle"]}),
                "searches[0] must be an object",
            ),
            (
                json!({"region_id":1,"region_type":6,"market":"seattle","searches":"x"}),
                "searches must be an array of search objects",
            ),
            (
                json!({"searches":(0..26).map(|_| json!({"region_id":1,"region_type":6,"market":"seattle"})).collect::<Vec<_>>()}),
                "searches cannot include more than 25 searches",
            ),
            (
                json!({"region_id":"1.2","region_type":6,"market":"seattle"}),
                "region_id must be an integer",
            ),
        ] {
            assert!(build_search_requests(&input)
                .unwrap_err()
                .to_string()
                .contains(message));
        }
        let requests = build_search_requests(
            &json!({"region_id":" 001 ","region_type":"6","market":"seattle","num_baths":"0x2"}),
        )
        .unwrap();
        assert_eq!(requests[0].params["region_id"], 1);
        assert_eq!(requests[0].params["num_baths"], 2);
    }

    #[test]
    fn formats_request_description_with_sorted_filters() {
        let requests = build_search_requests(
            &json!({"region_id":16163,"region_type":6,"market":"seattle","page":2,"num_homes":50}),
        )
        .unwrap();
        assert_eq!(
            describe_search(&requests[0].params),
            "region 16163 (seattle, type 6) (num_homes=50, page=2)"
        );
    }

    #[test]
    fn actor_input_prefill_defaults_remain_unchanged() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let properties = schema.get("properties").unwrap();
        assert_eq!(properties["region_id"]["default"], 16163);
        assert_eq!(properties["region_type"]["default"], "6");
        assert_eq!(properties["market"]["default"], "seattle");
        assert_eq!(properties["property_types"]["prefill"], "1,2,3");
        assert_eq!(properties["status"]["default"], "9");
        assert_eq!(properties["num_homes"]["default"], 50);
        assert_eq!(properties["page"]["default"], 1);
        assert_eq!(properties["searches"]["maxItems"], 25);
    }
}
