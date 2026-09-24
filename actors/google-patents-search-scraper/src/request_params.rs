use anyhow::{anyhow, bail, Result};
use serde_json::{Map, Number, Value};

pub fn build_google_patents_search_params(input: &Value) -> Result<Map<String, Value>> {
    let input = input
        .as_object()
        .ok_or_else(|| anyhow!("Input must be an object"))?;
    let mut params = Map::new();
    params.insert(
        "q".to_owned(),
        Value::String(clean_required_string(input.get("q"), "q", 500)?),
    );

    add_if_some(
        &mut params,
        "page",
        clean_integer(input.get("page"), "page", 1, 100)?,
    );
    add_if_some(
        &mut params,
        "num",
        clean_integer(input.get("num"), "num", 1, 100)?,
    );
    add_if_some(
        &mut params,
        "sort",
        clean_enum(input.get("sort"), "sort", &["new", "old"], false)?,
    );
    add_if_some(
        &mut params,
        "before",
        clean_date_filter(input.get("before"), "before")?,
    );
    add_if_some(
        &mut params,
        "after",
        clean_date_filter(input.get("after"), "after")?,
    );
    add_if_some(
        &mut params,
        "country",
        clean_csv_string(input.get("country"), "country", 200)?,
    );
    add_if_some(
        &mut params,
        "language",
        clean_csv_string(input.get("language"), "language", 200)?,
    );
    add_if_some(
        &mut params,
        "status",
        clean_enum(
            input.get("status"),
            "status",
            &["GRANT", "APPLICATION"],
            true,
        )?,
    );
    add_if_some(
        &mut params,
        "type",
        clean_enum(input.get("type"), "type", &["PATENT", "DESIGN"], true)?,
    );
    add_if_some(
        &mut params,
        "inventor",
        clean_csv_string(input.get("inventor"), "inventor", 300)?,
    );
    add_if_some(
        &mut params,
        "assignee",
        clean_csv_string(input.get("assignee"), "assignee", 300)?,
    );

    Ok(params)
}

pub fn describe_google_patents_search_request(params: &Map<String, Value>) -> String {
    let filters = [
        "country", "language", "status", "type", "before", "after", "inventor", "assignee",
    ]
    .iter()
    .filter_map(|field| {
        params
            .get(*field)
            .map(|value| format!("{field}={}", js_string(value)))
    })
    .collect::<Vec<_>>();
    let page = params
        .get("page")
        .map(|value| format!(" page {}", js_string(value)))
        .unwrap_or_default();
    let num = params
        .get("num")
        .map(|value| format!(" ({} results per page)", js_string(value)))
        .unwrap_or_default();
    let sort = params
        .get("sort")
        .map(|value| format!(" sorted by {}", js_string(value)))
        .unwrap_or_default();
    let filter_suffix = if filters.is_empty() {
        String::new()
    } else {
        format!(" with {}", filters.join(", "))
    };

    format!(
        "query \"{}\"{page}{num}{sort}{filter_suffix}",
        params.get("q").map(js_string).unwrap_or_default()
    )
}

pub fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                integer.to_string()
            } else if let Some(integer) = value.as_u64() {
                integer.to_string()
            } else if let Some(number) = value.as_f64() {
                if number.fract() == 0.0 && number.abs() < 1e21 {
                    format!("{number:.0}")
                } else {
                    number.to_string()
                }
            } else {
                value.to_string()
            }
        }
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

fn add_if_some(params: &mut Map<String, Value>, field: &str, value: Option<Value>) {
    if let Some(value) = value {
        params.insert(field.to_owned(), value);
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
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(value.to_owned()))
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_integer(value: Option<&Value>, field: &str, min: i64, max: i64) -> Result<Option<Value>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(number) = value.as_number() else {
        bail!("{field} must be an integer");
    };
    let numeric = number
        .as_i64()
        .map(|number| number as f64)
        .or_else(|| number.as_u64().map(|number| number as f64))
        .or_else(|| number.as_f64().filter(|number| number.is_finite()))
        .ok_or_else(|| anyhow!("{field} must be an integer"))?;
    if numeric.fract() != 0.0 {
        bail!("{field} must be an integer");
    }
    if numeric < min as f64 || numeric > max as f64 {
        bail!("{field} must be between {min} and {max}");
    }
    let integer = numeric as i64;
    Ok(Some(Value::Number(Number::from(integer))))
}

fn clean_enum(
    value: Option<&Value>,
    field: &str,
    allowed: &[&str],
    uppercase: bool,
) -> Result<Option<Value>> {
    let Some(value) = clean_string(value, field, 40)? else {
        return Ok(None);
    };
    let normalized = if uppercase {
        value.to_ascii_uppercase()
    } else {
        value.to_ascii_lowercase()
    };
    if !allowed.contains(&normalized.as_str()) {
        bail!("{field} must be one of: {}", allowed.join(", "));
    }
    Ok(Some(Value::String(normalized)))
}

fn clean_date_filter(value: Option<&Value>, field: &str) -> Result<Option<Value>> {
    let Some(value) = clean_string(value, field, 30)? else {
        return Ok(None);
    };
    let Some((raw_type, date)) = value.split_once(':') else {
        bail!("{field} must use format filing:YYYYMMDD or publication:YYYYMMDD");
    };
    let valid_type =
        raw_type.eq_ignore_ascii_case("filing") || raw_type.eq_ignore_ascii_case("publication");
    if !valid_type || date.len() != 8 || !date.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("{field} must use format filing:YYYYMMDD or publication:YYYYMMDD");
    }
    if !is_valid_date(date) {
        bail!("{field} must include a valid calendar date");
    }
    let date_type = raw_type.to_ascii_lowercase();
    Ok(Some(Value::String(format!("{date_type}:{date}"))))
}

fn is_valid_date(date: &str) -> bool {
    let year = date[..4].parse::<u32>().unwrap_or_default();
    let month = date[4..6].parse::<u32>().unwrap_or_default();
    let day = date[6..8].parse::<u32>().unwrap_or_default();
    if !(1..=12).contains(&month) {
        return false;
    }
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month).contains(&day)
}

fn clean_csv_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<Value>> {
    let Some(value) = clean_string(value, field, max_length)? else {
        return Ok(None);
    };
    let normalized = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(",");
    if normalized.is_empty() {
        return Ok(None);
    }
    Ok(Some(Value::String(normalized)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_and_describes_filtered_search_params() {
        let params = build_google_patents_search_params(&json!({
            "q": " wireless charging ",
            "page": 2,
            "num": 25,
            "sort": "NEW",
            "before": "FILING:20231231",
            "after": "publication:20200101",
            "country": " US, EP , WO ",
            "language": " ENGLISH ",
            "status": "grant",
            "type": "patent",
            "inventor": "Ada Lovelace, Grace Hopper",
            "assignee": "Tesla, Toyota"
        }))
        .unwrap();

        assert_eq!(
            params,
            serde_json::from_value(json!({
                "q": "wireless charging",
                "page": 2,
                "num": 25,
                "sort": "new",
                "before": "filing:20231231",
                "after": "publication:20200101",
                "country": "US,EP,WO",
                "language": "ENGLISH",
                "status": "GRANT",
                "type": "PATENT",
                "inventor": "Ada Lovelace,Grace Hopper",
                "assignee": "Tesla,Toyota"
            }))
            .unwrap()
        );
        assert_eq!(
            describe_google_patents_search_request(&params),
            "query \"wireless charging\" page 2 (25 results per page) sorted by new with country=US,EP,WO, language=ENGLISH, status=GRANT, type=PATENT, before=filing:20231231, after=publication:20200101, inventor=Ada Lovelace,Grace Hopper, assignee=Tesla,Toyota"
        );
    }

    #[test]
    fn validates_required_query_numbers_enums_and_dates() {
        assert_eq!(
            build_google_patents_search_params(&json!({"page":1}))
                .unwrap_err()
                .to_string(),
            "q is required"
        );
        assert!(
            build_google_patents_search_params(&json!({"q":"battery", "page":0}))
                .unwrap_err()
                .to_string()
                .contains("page must be between 1 and 100")
        );
        assert!(
            build_google_patents_search_params(&json!({"q":"battery", "num":1.5}))
                .unwrap_err()
                .to_string()
                .contains("num must be an integer")
        );
        assert!(
            build_google_patents_search_params(&json!({"q":"battery", "sort":"relevance"}))
                .unwrap_err()
                .to_string()
                .contains("sort must be one of: new, old")
        );
        assert!(
            build_google_patents_search_params(&json!({"q":"battery", "status":"expired"}))
                .unwrap_err()
                .to_string()
                .contains("status must be one of: GRANT, APPLICATION")
        );
        assert!(build_google_patents_search_params(
            &json!({"q":"battery", "before":"filing:20230230"})
        )
        .unwrap_err()
        .to_string()
        .contains("before must include a valid calendar date"));
        assert!(build_google_patents_search_params(
            &json!({"q":"battery", "after":"priority:20230201"})
        )
        .unwrap_err()
        .to_string()
        .contains("after must use format filing:YYYYMMDD or publication:YYYYMMDD"));
    }

    #[test]
    fn checks_query_length_in_javascript_characters() {
        let too_long = "😀".repeat(251);
        assert!(build_google_patents_search_params(&json!({"q":too_long}))
            .unwrap_err()
            .to_string()
            .contains("q must be 500 characters or fewer"));
    }
}
