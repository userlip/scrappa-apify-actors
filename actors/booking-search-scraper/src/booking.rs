use anyhow::{anyhow, bail, Result};
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;

use crate::apify::endpoint_url;

pub const MAX_SEARCHES_PER_RUN: usize = 25;

#[derive(Debug, Clone, PartialEq)]
pub struct BookingSearchRequest {
    pub params: BTreeMap<String, Value>,
    pub index: usize,
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }

    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(trimmed.to_owned()))
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn has_date_format(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

fn is_valid_date(value: &str) -> bool {
    if !has_date_format(value) {
        return false;
    }

    let year = value[0..4].parse::<u32>().ok();
    let month = value[5..7].parse::<u32>().ok();
    let day = value[8..10].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day)) = (year, month, day) else {
        return false;
    };
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

fn civil_date_from_days_since_epoch(days_since_epoch: i64) -> (i64, i64, i64) {
    let shifted_days = days_since_epoch + 719_468;
    let era = if shifted_days >= 0 {
        shifted_days
    } else {
        shifted_days - 146_096
    } / 146_097;
    let day_of_era = shifted_days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

pub fn today_utc() -> String {
    let days_since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        / 86_400;
    let (year, month, day) = civil_date_from_days_since_epoch(days_since_epoch);
    format!("{year:04}-{month:02}-{day:02}")
}

fn clean_date(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    let Some(date) = clean_string(value, field, 10)? else {
        return Ok(None);
    };
    if !has_date_format(&date) {
        bail!("{field} must use YYYY-MM-DD format");
    }
    if !is_valid_date(&date) {
        bail!("{field} must be a valid calendar date");
    }
    if date < today_utc() {
        bail!("{field} must be today or a future date");
    }
    Ok(Some(date))
}

fn is_integer_string(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn clean_integer(value: Option<&Value>, field: &str, min: i64, max: i64) -> Result<Option<Value>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }

    let number = match value {
        Value::String(value) => {
            let trimmed = value.trim();
            if !is_integer_string(trimmed) {
                bail!("{field} must be an integer");
            }
            let number = trimmed.parse::<f64>().unwrap_or(f64::INFINITY);
            if !number.is_finite() {
                bail!("{field} must be an integer");
            }
            number
        }
        Value::Number(value) => {
            let Some(number) = value.as_f64() else {
                bail!("{field} must be an integer");
            };
            if !number.is_finite() || number.fract() != 0.0 {
                bail!("{field} must be an integer");
            }
            number
        }
        _ => bail!("{field} must be an integer"),
    };

    if number < min as f64 || number > max as f64 {
        bail!("{field} must be between {min} and {max}");
    }
    Ok(Some(json!(number as i64)))
}

fn clean_language(value: Option<&Value>) -> Result<Option<String>> {
    let Some(language) = clean_string(value, "lang", 10)? else {
        return Ok(None);
    };
    let normalized = language.to_ascii_lowercase();
    let parts = normalized.split('-').collect::<Vec<_>>();
    let valid = match parts.as_slice() {
        [language] => language.len() == 2 && language.bytes().all(|byte| byte.is_ascii_lowercase()),
        [language, region] => {
            language.len() == 2
                && region.len() == 2
                && language.bytes().all(|byte| byte.is_ascii_lowercase())
                && region.bytes().all(|byte| byte.is_ascii_lowercase())
        }
        _ => false,
    };
    if !valid {
        bail!("lang must be a valid language code such as en, en-us, de, or fr");
    }
    Ok(Some(normalized))
}

fn clean_currency(value: Option<&Value>) -> Result<Option<String>> {
    let Some(currency) = clean_string(value, "currency", 20)? else {
        return Ok(None);
    };
    let normalized = currency.to_ascii_uppercase();
    if normalized.len() != 3 || !normalized.bytes().all(|byte| byte.is_ascii_uppercase()) {
        bail!("currency must be a 3-letter currency code such as USD, EUR, or GBP");
    }
    Ok(Some(normalized))
}

fn add_if_defined(params: &mut BTreeMap<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        params.insert(key.to_owned(), value);
    }
}

fn build_single_booking_search_params(
    input: &Value,
    prefix: &str,
) -> Result<BTreeMap<String, Value>> {
    let field = |name: &str| format!("{prefix}{name}");
    let mut params = BTreeMap::new();
    params.insert(
        "ss".to_owned(),
        Value::String(clean_required_string(input.get("ss"), &field("ss"), 200)?),
    );

    let checkin = clean_date(input.get("checkin"), &field("checkin"))?;
    let checkout = clean_date(input.get("checkout"), &field("checkout"))?;
    if checkin.is_some() != checkout.is_some() {
        bail!(
            "{} and {} must be provided together",
            field("checkin"),
            field("checkout")
        );
    }
    if let (Some(checkin), Some(checkout)) = (&checkin, &checkout) {
        if checkout <= checkin {
            bail!("{} must be after {}", field("checkout"), field("checkin"));
        }
    }

    add_if_defined(&mut params, "checkin", checkin.map(Value::String));
    add_if_defined(&mut params, "checkout", checkout.map(Value::String));
    add_if_defined(
        &mut params,
        "group_adults",
        clean_integer(input.get("group_adults"), &field("group_adults"), 1, 30)?,
    );
    add_if_defined(
        &mut params,
        "group_children",
        clean_integer(input.get("group_children"), &field("group_children"), 0, 20)?,
    );
    add_if_defined(
        &mut params,
        "no_rooms",
        clean_integer(input.get("no_rooms"), &field("no_rooms"), 1, 30)?,
    );
    add_if_defined(
        &mut params,
        "lang",
        clean_language(input.get("lang"))?.map(Value::String),
    );
    add_if_defined(
        &mut params,
        "currency",
        clean_currency(input.get("currency"))?.map(Value::String),
    );
    Ok(params)
}

pub fn build_booking_search_requests(input: &Value) -> Result<Vec<BookingSearchRequest>> {
    if let Some(searches) = input.get("searches") {
        if let Some(searches) = searches.as_array() {
            if searches.is_empty() {
                bail!("searches must include at least one search");
            }
            if searches.len() > MAX_SEARCHES_PER_RUN {
                bail!("searches cannot include more than {MAX_SEARCHES_PER_RUN} searches per run");
            }

            return searches
                .iter()
                .enumerate()
                .map(|(index, search)| {
                    if !search.is_object() {
                        bail!("searches[{index}] must be an object");
                    }
                    Ok(BookingSearchRequest {
                        params: build_single_booking_search_params(
                            search,
                            &format!("searches[{index}]."),
                        )?,
                        index,
                    })
                })
                .collect();
        }
        bail!("searches must be an array of search objects");
    }

    Ok(vec![BookingSearchRequest {
        params: build_single_booking_search_params(input, "")?,
        index: 0,
    }])
}

pub fn describe_booking_search_request(params: &BTreeMap<String, Value>) -> String {
    let destination = params
        .get("ss")
        .and_then(Value::as_str)
        .unwrap_or("unknown destination");
    let core = format!("\"{destination}\"");
    let dates = match (params.get("checkin"), params.get("checkout")) {
        (Some(checkin), Some(checkout)) => format!(
            " {} to {}",
            checkin.as_str().unwrap_or_default(),
            checkout.as_str().unwrap_or_default()
        ),
        _ => String::new(),
    };
    let filters = params
        .iter()
        .filter(|(field, _)| !matches!(field.as_str(), "ss" | "checkin" | "checkout"))
        .map(|(field, value)| {
            format!(
                "{field}={}",
                value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string())
            )
        })
        .collect::<Vec<_>>();

    if filters.is_empty() {
        format!("{core}{dates}")
    } else {
        format!("{core}{dates} ({})", filters.join(", "))
    }
}

pub fn booking_search_url(base_url: &Url, params: &BTreeMap<String, Value>) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["booking", "search"])?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            query.append_pair(key, &value);
        }
    }
    Ok(url)
}

pub fn get_booking_search_results(response: &Value) -> Vec<Value> {
    if let Some(results) = response.pointer("/data/results").and_then(Value::as_array) {
        return results.clone();
    }
    response
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn first_string(property: &Value, names: &[&str]) -> Value {
    names
        .iter()
        .filter_map(|name| property.get(*name).and_then(Value::as_str))
        .find(|value| !value.trim().is_empty())
        .map(|value| Value::String(value.to_owned()))
        .unwrap_or(Value::Null)
}

fn parse_javascript_number(value: &str) -> Option<f64> {
    let value = value.trim();
    if value.is_empty() {
        return Some(0.0);
    }
    let radix_value = if let Some(value) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(value, 16)
            .ok()
            .map(|number| number as f64)
    } else if let Some(value) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        u64::from_str_radix(value, 2)
            .ok()
            .map(|number| number as f64)
    } else if let Some(value) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        u64::from_str_radix(value, 8)
            .ok()
            .map(|number| number as f64)
    } else {
        value.parse::<f64>().ok()
    }?;
    radix_value.is_finite().then_some(radix_value)
}

fn javascript_json_number(value: f64) -> Option<serde_json::Number> {
    if value.fract() == 0.0 {
        if value >= i64::MIN as f64 && value < 9_223_372_036_854_775_808.0 {
            return Some(serde_json::Number::from(value as i64));
        }
        if value >= 0.0 && value < 18_446_744_073_709_551_616.0 {
            return Some(serde_json::Number::from(value as u64));
        }
    }
    serde_json::Number::from_f64(value)
}

fn first_number(property: &Value, name: &str) -> Value {
    let Some(value) = property.get(name) else {
        return Value::Null;
    };
    let number = value.as_f64().or_else(|| {
        value
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .and_then(parse_javascript_number)
    });
    number
        .filter(|number| number.is_finite())
        .and_then(javascript_json_number)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn request_value(params: &BTreeMap<String, Value>, key: &str) -> Value {
    params.get(key).cloned().unwrap_or(Value::Null)
}

pub fn build_booking_dataset_item(
    property: &Value,
    params: &BTreeMap<String, Value>,
    search_index: usize,
) -> Value {
    let mut item = match property {
        Value::Object(property) => property.clone(),
        Value::Array(property) => property
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect::<Map<String, Value>>(),
        _ => Map::new(),
    };

    item.insert(
        "name".to_owned(),
        first_string(property, &["name", "title"]),
    );
    item.insert("url".to_owned(), first_string(property, &["url", "link"]));
    item.insert(
        "image".to_owned(),
        first_string(property, &["image", "thumbnail"]),
    );
    item.insert(
        "review_score".to_owned(),
        first_number(property, "review_score"),
    );
    item.insert(
        "review_score_word".to_owned(),
        first_string(property, &["review_score_word"]),
    );
    item.insert(
        "review_count".to_owned(),
        first_number(property, "review_count"),
    );
    item.insert(
        "location".to_owned(),
        first_string(property, &["location", "address"]),
    );
    item.insert(
        "price".to_owned(),
        first_string(property, &["price", "price_for_display"]),
    );
    let currency = first_string(property, &["currency"]);
    item.insert(
        "currency".to_owned(),
        if currency.is_null() {
            request_value(params, "currency")
        } else {
            currency
        },
    );
    item.insert("request_search_index".to_owned(), json!(search_index));
    item.insert("request_ss".to_owned(), request_value(params, "ss"));
    item.insert(
        "request_checkin".to_owned(),
        request_value(params, "checkin"),
    );
    item.insert(
        "request_checkout".to_owned(),
        request_value(params, "checkout"),
    );
    item.insert(
        "request_group_adults".to_owned(),
        request_value(params, "group_adults"),
    );
    item.insert(
        "request_group_children".to_owned(),
        request_value(params, "group_children"),
    );
    item.insert(
        "request_no_rooms".to_owned(),
        request_value(params, "no_rooms"),
    );
    item.insert("request_lang".to_owned(), request_value(params, "lang"));
    item.insert(
        "request_currency".to_owned(),
        request_value(params, "currency"),
    );
    Value::Object(item)
}
