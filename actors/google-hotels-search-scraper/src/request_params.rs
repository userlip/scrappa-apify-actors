use chrono::NaiveDate;
use serde_json::{Map, Number, Value};
use std::collections::BTreeMap;

const MAX_PRICE: i64 = 5000;
const SORT_BY_VALUES: &[i64] = &[3, 8, 13];
const HOTEL_CLASS_VALUES: &[i64] = &[2, 3, 4, 5];
const RATING_VALUES: &[i64] = &[7, 8, 9];

type Params = Map<String, Value>;
type RequestResult<T> = std::result::Result<T, String>;

pub fn build_google_hotels_search_params(input: &Value, today: NaiveDate) -> RequestResult<Params> {
    let input = input
        .as_object()
        .ok_or_else(|| "Input must be an object".to_owned())?;
    let mut params = Params::new();
    params.insert(
        "q".to_owned(),
        Value::String(clean_required_string(input.get("q"), "q", 200)?),
    );
    let check_in = clean_date(input.get("check_in_date"), "check_in_date", today, true)?;
    let check_out = clean_date(input.get("check_out_date"), "check_out_date", today, false)?;
    if check_out <= check_in {
        return Err("check_out_date must be after check_in_date".to_owned());
    }
    params.insert(
        "check_in_date".to_owned(),
        Value::String(check_in.format("%Y-%m-%d").to_string()),
    );
    params.insert(
        "check_out_date".to_owned(),
        Value::String(check_out.format("%Y-%m-%d").to_string()),
    );

    let adults = clean_integer(input.get("adults"), "adults", 1, Some(10))?;
    let children = clean_integer(input.get("children"), "children", 0, Some(6))?;
    let children_ages = clean_integer_array(
        input.get("children_ages"),
        "children_ages",
        Some(1),
        Some(17),
    )?;
    if children.unwrap_or(0) == 0 && children_ages.is_some() {
        return Err("children_ages requires children to be greater than 0".to_owned());
    }
    if let Some(children) = children {
        if children > 0 && children_ages.as_ref().map_or(0, Vec::len) != children as usize {
            return Err("children_ages length must match children".to_owned());
        }
    }

    let min_price = clean_integer(input.get("min_price"), "min_price", 0, None)?;
    let max_price = clean_integer(input.get("max_price"), "max_price", 1, Some(MAX_PRICE))?;
    if let (Some(min_price), Some(max_price)) = (min_price, max_price) {
        if max_price <= min_price {
            return Err("max_price must be greater than min_price".to_owned());
        }
        if max_price - min_price > MAX_PRICE {
            return Err("max_price cannot be more than 5000 above min_price".to_owned());
        }
    }

    let vacation_rentals = clean_boolean(input.get("vacation_rentals"), "vacation_rentals")?;
    let booleans = [
        (
            "free_cancellation",
            clean_boolean(input.get("free_cancellation"), "free_cancellation")?,
        ),
        (
            "eco_certified",
            clean_boolean(input.get("eco_certified"), "eco_certified")?,
        ),
        (
            "special_offers",
            clean_boolean(input.get("special_offers"), "special_offers")?,
        ),
    ];
    let has_boolean_filter = booleans.iter().any(|(_, value)| *value == Some(true));

    let filters = [
        ("min_price", optional_number(min_price)),
        ("max_price", optional_number(max_price)),
        (
            "hotel_class",
            clean_enum_number(input.get("hotel_class"), "hotel_class", HOTEL_CLASS_VALUES)?
                .map(|value| Value::Number(Number::from(value))),
        ),
        (
            "rating",
            clean_enum_number(input.get("rating"), "rating", RATING_VALUES)?
                .map(|value| Value::Number(Number::from(value))),
        ),
        (
            "amenities",
            clean_integer_array(input.get("amenities"), "amenities", None, None)?
                .map(join_integer_array),
        ),
        (
            "property_types",
            clean_integer_array(input.get("property_types"), "property_types", None, None)?
                .map(join_integer_array),
        ),
        (
            "brands",
            clean_integer_array(input.get("brands"), "brands", Some(1), None)?
                .map(join_integer_array),
        ),
    ];
    let active_filters = filters
        .iter()
        .filter(|(_, value)| value.is_some())
        .map(|(name, _)| *name)
        .collect::<Vec<_>>();
    if has_boolean_filter && !active_filters.is_empty() {
        return Err(format!(
            "Boolean filters cannot be combined with other filters: {}",
            active_filters.join(", ")
        ));
    }

    if vacation_rentals == Some(true) {
        if booleans.iter().any(|(_, value)| *value == Some(true)) {
            return Err(
                "free_cancellation, eco_certified, and special_offers are not available for vacation_rentals"
                    .to_owned(),
            );
        }
        if filters
            .iter()
            .any(|(name, value)| matches!(*name, "hotel_class" | "brands") && value.is_some())
        {
            return Err("hotel_class and brands are not available for vacation_rentals".to_owned());
        }
    } else if input.contains_key("bedrooms") || input.contains_key("bathrooms") {
        return Err(
            "bedrooms and bathrooms are only available when vacation_rentals is true".to_owned(),
        );
    }

    insert_optional_number(&mut params, "adults", adults);
    insert_optional_number(&mut params, "children", children);
    if let Some(ages) = children_ages {
        params.insert("children_ages".to_owned(), join_integer_array(ages));
    }
    insert_optional_string(
        &mut params,
        "currency",
        clean_code(input.get("currency"), "currency", 3)?.map(|code| code.to_ascii_uppercase()),
    );
    insert_optional_string(
        &mut params,
        "gl",
        clean_code(input.get("gl"), "gl", 2)?.map(|code| code.to_ascii_lowercase()),
    );
    insert_optional_string(
        &mut params,
        "hl",
        clean_code(input.get("hl"), "hl", 2)?.map(|code| code.to_ascii_lowercase()),
    );
    if let Some(sort_by) = clean_enum_number(input.get("sort_by"), "sort_by", SORT_BY_VALUES)? {
        params.insert("sort_by".to_owned(), Value::Number(Number::from(sort_by)));
    }
    insert_optional_bool(&mut params, "vacation_rentals", vacation_rentals);
    insert_optional_number(
        &mut params,
        "bedrooms",
        clean_integer(input.get("bedrooms"), "bedrooms", 1, Some(20))?,
    );
    insert_optional_number(
        &mut params,
        "bathrooms",
        clean_integer(input.get("bathrooms"), "bathrooms", 1, Some(20))?,
    );
    insert_optional_string(
        &mut params,
        "next_page_token",
        clean_string(input.get("next_page_token"), "next_page_token", 3000)?,
    );
    insert_optional_string(
        &mut params,
        "property_token",
        clean_string(input.get("property_token"), "property_token", 3000)?,
    );
    for (name, value) in booleans {
        insert_optional_bool(&mut params, name, value);
    }
    for (name, value) in filters {
        if let Some(value) = value {
            params.insert(name.to_owned(), value);
        }
    }

    Ok(params)
}

pub fn describe_google_hotels_search_request(params: &Params) -> String {
    let query = params
        .get("q")
        .and_then(Value::as_str)
        .unwrap_or("unknown location");
    let check_in = params
        .get("check_in_date")
        .map(value_to_string)
        .unwrap_or_else(|| "undefined".to_owned());
    let check_out = params
        .get("check_out_date")
        .map(value_to_string)
        .unwrap_or_else(|| "undefined".to_owned());
    let filters = params
        .iter()
        .filter(|(field, _)| !matches!(field.as_str(), "q" | "check_in_date" | "check_out_date"))
        .map(|(field, value)| (field.clone(), value_to_string(value)))
        .collect::<BTreeMap<_, _>>();
    if filters.is_empty() {
        format!("\"{query}\" {check_in} to {check_out}")
    } else {
        let filters = filters
            .iter()
            .map(|(field, value)| format!("{field}={value}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("\"{query}\" {check_in} to {check_out} ({filters})")
    }
}

fn clean_date(
    value: Option<&Value>,
    field: &str,
    today: NaiveDate,
    future_or_today: bool,
) -> RequestResult<NaiveDate> {
    let raw = clean_required_string(value, field, 18)?;
    let date_text = match raw.to_ascii_lowercase().as_str() {
        "today" => today,
        "tomorrow" => today
            .succ_opt()
            .ok_or_else(|| format!("{field} is outside the supported date range"))?,
        "day-after-tomorrow" => today
            .checked_add_days(chrono::Days::new(2))
            .ok_or_else(|| format!("{field} is outside the supported date range"))?,
        _ => {
            if !is_iso_date(&raw) {
                return Err(format!(
                    "{field} must use YYYY-MM-DD format or a supported relative date"
                ));
            }
            NaiveDate::parse_from_str(&raw, "%Y-%m-%d")
                .map_err(|_| format!("{field} must be a valid calendar date"))?
        }
    };
    if future_or_today && date_text < today {
        return Err(format!("{field} must be today or a future date"));
    }
    Ok(date_text)
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[0..4].iter().all(|byte| byte.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|byte| byte.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(|byte| byte.is_ascii_digit())
}

fn clean_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> RequestResult<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if matches!(value, Value::String(value) if value.is_empty()) {
        return Ok(None);
    }
    let value = value
        .as_str()
        .ok_or_else(|| format!("{field} must be a string"))?
        .trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_length {
        return Err(format!("{field} must be {max_length} characters or fewer"));
    }
    Ok(Some(value.to_owned()))
}

fn clean_required_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> RequestResult<String> {
    clean_string(value, field, max_length)?.ok_or_else(|| format!("{field} is required"))
}

fn clean_integer(
    value: Option<&Value>,
    field: &str,
    min: i64,
    max: Option<i64>,
) -> RequestResult<Option<i64>> {
    let Some(value) = value.filter(|value| !value.is_null() && !is_empty_string(value)) else {
        return Ok(None);
    };
    let number = value
        .as_f64()
        .filter(|number| number.is_finite() && number.fract() == 0.0)
        .ok_or_else(|| format!("{field} must be an integer"))?;
    if number < min as f64 || max.is_some_and(|max| number > max as f64) {
        return Err(match max {
            Some(max) => format!("{field} must be between {min} and {max}"),
            None => format!("{field} must be greater than or equal to {min}"),
        });
    }
    Ok(Some(number as i64))
}

fn clean_boolean(value: Option<&Value>, field: &str) -> RequestResult<Option<bool>> {
    let Some(value) = value.filter(|value| !value.is_null() && !is_empty_string(value)) else {
        return Ok(None);
    };
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| format!("{field} must be true or false"))
}

fn clean_code(value: Option<&Value>, field: &str, length: usize) -> RequestResult<Option<String>> {
    let Some(value) = clean_string(value, field, length)? else {
        return Ok(None);
    };
    if value.chars().count() != length
        || !value
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        return Err(format!("{field} must be a {length}-letter code"));
    }
    Ok(Some(value))
}

fn clean_enum_number(
    value: Option<&Value>,
    field: &str,
    allowed: &[i64],
) -> RequestResult<Option<i64>> {
    let normalized = match value {
        Some(Value::String(value))
            if !value.trim().is_empty() && is_integer_string(value.trim()) =>
        {
            Some(Value::Number(Number::from(
                value
                    .trim()
                    .parse::<i64>()
                    .map_err(|_| format!("{field} must be an integer"))?,
            )))
        }
        _ => value.cloned(),
    };
    let value = clean_integer(
        normalized.as_ref(),
        field,
        *allowed.iter().min().unwrap(),
        Some(*allowed.iter().max().unwrap()),
    )?;
    if let Some(value) = value {
        if !allowed.contains(&value) {
            return Err(format!(
                "{field} must be one of: {}",
                allowed
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    Ok(value)
}

fn clean_integer_array(
    value: Option<&Value>,
    field: &str,
    min: Option<i64>,
    max: Option<i64>,
) -> RequestResult<Option<Vec<i64>>> {
    let Some(value) = value.filter(|value| !value.is_null() && !is_empty_string(value)) else {
        return Ok(None);
    };
    let values = match value {
        Value::Array(values) => values.clone(),
        Value::String(value) => value
            .split(',')
            .enumerate()
            .map(|(index, part)| {
                let part = part.trim();
                if !is_integer_string(part) {
                    return Err(format!("{field}[{index}] must be an integer"));
                }
                Ok(Value::Number(Number::from(part.parse::<i64>().map_err(
                    |_| format!("{field}[{index}] must be an integer"),
                )?)))
            })
            .collect::<RequestResult<Vec<_>>>()?,
        _ => {
            return Err(format!(
                "{field} must be an array of integers or a comma-separated string"
            ));
        }
    };
    let mut integers = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let number = value
            .as_f64()
            .filter(|number| number.is_finite() && number.fract() == 0.0)
            .ok_or_else(|| format!("{field}[{index}] must be an integer"))?;
        if min.is_some_and(|min| number < min as f64) {
            return Err(format!(
                "{field}[{index}] must be greater than or equal to {}",
                min.unwrap()
            ));
        }
        if max.is_some_and(|max| number > max as f64) {
            return Err(format!(
                "{field}[{index}] must be less than or equal to {}",
                max.unwrap()
            ));
        }
        integers.push(number as i64);
    }
    Ok((!integers.is_empty()).then_some(integers))
}

fn set_param(params: &mut Params, field: &str, value: Option<Value>) {
    if let Some(value) = value.filter(|value| {
        !value.is_null() && !is_empty_string(value) && !matches!(value, Value::Bool(false))
    }) {
        params.insert(field.to_owned(), value);
    }
}

fn is_empty_string(value: &Value) -> bool {
    matches!(value, Value::String(value) if value.is_empty())
}

fn optional_number(value: Option<i64>) -> Option<Value> {
    value.map(Number::from).map(Value::Number)
}

fn insert_optional_number(params: &mut Params, field: &str, value: Option<i64>) {
    set_param(params, field, optional_number(value));
}

fn insert_optional_string(params: &mut Params, field: &str, value: Option<String>) {
    set_param(params, field, value.map(Value::String));
}

fn insert_optional_bool(params: &mut Params, field: &str, value: Option<bool>) {
    set_param(params, field, value.map(Value::Bool));
}

fn join_integer_array(values: Vec<i64>) -> Value {
    Value::String(
        values
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn is_integer_string(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values
            .iter()
            .map(value_to_string)
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 7, 10).unwrap()
    }

    fn dates() -> Value {
        json!({
            "q": "Paris, France",
            "check_in_date": "2026-08-01",
            "check_out_date": "2026-08-04"
        })
    }

    #[test]
    fn builds_query_params_and_normalizes_marketplace_selects() {
        let mut input = dates();
        input["q"] = json!(" Paris, France ");
        input["children"] = json!(1);
        input["children_ages"] = json!([7]);
        input["currency"] = json!("eur");
        input["gl"] = json!("FR");
        input["hl"] = json!("EN");
        input["sort_by"] = json!("3");
        input["hotel_class"] = json!("4");
        input["rating"] = json!("8");
        input["amenities"] = json!([35, 9]);
        input["brands"] = json!([1, 2]);
        input["property_token"] = json!(" token ");
        let params = build_google_hotels_search_params(&input, today()).unwrap();

        assert_eq!(params["q"], "Paris, France");
        assert_eq!(params["currency"], "EUR");
        assert_eq!(params["gl"], "fr");
        assert_eq!(params["hl"], "en");
        assert_eq!(params["sort_by"], 3);
        assert_eq!(params["hotel_class"], 4);
        assert_eq!(params["rating"], 8);
        assert_eq!(params["children_ages"], "7");
        assert_eq!(params["amenities"], "35,9");
        assert_eq!(params["brands"], "1,2");
        assert_eq!(params["property_token"], "token");
    }

    #[test]
    fn resolves_relative_dates_against_utc_date_and_keeps_prefills() {
        let params = build_google_hotels_search_params(
            &json!({
                "q": "Paris",
                "check_in_date": "tomorrow",
                "check_out_date": "day-after-tomorrow"
            }),
            today(),
        )
        .unwrap();
        assert_eq!(params["check_in_date"], "2026-07-11");
        assert_eq!(params["check_out_date"], "2026-07-12");

        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["properties"]["q"]["prefill"], "Paris, France");
        assert_eq!(schema["properties"]["check_in_date"]["prefill"], "tomorrow");
        assert_eq!(
            schema["properties"]["check_out_date"]["prefill"],
            "day-after-tomorrow"
        );
        assert_eq!(
            schema["required"],
            json!(["q", "check_in_date", "check_out_date"])
        );
    }

    #[test]
    fn validates_guests_prices_and_boolean_filter_combinations() {
        let mut input = dates();
        input["children"] = json!(2);
        input["children_ages"] = json!([7]);
        assert_eq!(
            build_google_hotels_search_params(&input, today()).unwrap_err(),
            "children_ages length must match children"
        );

        let mut input = dates();
        input["min_price"] = json!(500);
        input["max_price"] = json!(200);
        assert_eq!(
            build_google_hotels_search_params(&input, today()).unwrap_err(),
            "max_price must be greater than min_price"
        );

        let mut input = dates();
        input["free_cancellation"] = json!(true);
        input["hotel_class"] = json!(4);
        assert!(build_google_hotels_search_params(&input, today())
            .unwrap_err()
            .starts_with("Boolean filters cannot be combined"));
    }

    #[test]
    fn validates_date_ranges_and_filter_dependencies() {
        let mut input = dates();
        input["check_in_date"] = json!("2026-02-30");
        assert_eq!(
            build_google_hotels_search_params(&input, today()).unwrap_err(),
            "check_in_date must be a valid calendar date"
        );

        let mut input = dates();
        input["check_in_date"] = json!("2026-07-09");
        assert_eq!(
            build_google_hotels_search_params(&input, today()).unwrap_err(),
            "check_in_date must be today or a future date"
        );

        let mut input = dates();
        input["vacation_rentals"] = json!(true);
        input["bedrooms"] = json!(2);
        input["hotel_class"] = json!(4);
        assert_eq!(
            build_google_hotels_search_params(&input, today()).unwrap_err(),
            "hotel_class and brands are not available for vacation_rentals"
        );

        let mut input = dates();
        input["bedrooms"] = json!(2);
        assert_eq!(
            build_google_hotels_search_params(&input, today()).unwrap_err(),
            "bedrooms and bathrooms are only available when vacation_rentals is true"
        );
    }

    #[test]
    fn accepts_vacation_rentals_and_describes_sorted_filters() {
        let mut input = json!({
            "q": "Aspen cabins",
            "check_in_date": "2026-08-01",
            "check_out_date": "2026-08-04",
            "vacation_rentals": true,
            "bedrooms": 2,
            "bathrooms": 2,
            "next_page_token": "page-token"
        });
        input["free_cancellation"] = json!(false);
        let params = build_google_hotels_search_params(&input, today()).unwrap();
        assert_eq!(
            describe_google_hotels_search_request(&params),
            "\"Aspen cabins\" 2026-08-01 to 2026-08-04 (bathrooms=2, bedrooms=2, next_page_token=page-token, vacation_rentals=true)"
        );
    }
}
