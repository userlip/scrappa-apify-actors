use anyhow::{anyhow, bail, Result};
use serde_json::{json, Map, Number, Value};

pub(crate) const DEFAULT_LOCATION: &str = "1276003001";
pub(crate) const DEFAULT_TYPE: &str = "apartment-rent";
const MAX_LOCATION_LENGTH: usize = 120;
const MAX_PER_PAGE: i64 = 50;
const PROPERTY_TYPES: [&str; 4] = ["apartment-rent", "apartment-buy", "house-rent", "house-buy"];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SearchParams {
    pub(crate) location: String,
    pub(crate) property_type: String,
    pub(crate) price_min: Option<Number>,
    pub(crate) price_max: Option<Number>,
    pub(crate) rooms_min: Option<Number>,
    pub(crate) rooms_max: Option<Number>,
    pub(crate) size_min: Option<Number>,
    pub(crate) size_max: Option<Number>,
    pub(crate) page: i64,
    pub(crate) per_page: i64,
}

impl SearchParams {
    pub(crate) fn query_pairs(&self) -> Vec<(String, String)> {
        let mut params = vec![
            ("location".to_owned(), self.location.clone()),
            ("type".to_owned(), self.property_type.clone()),
        ];
        push_number_pair(&mut params, "price_min", &self.price_min);
        push_number_pair(&mut params, "price_max", &self.price_max);
        push_number_pair(&mut params, "rooms_min", &self.rooms_min);
        push_number_pair(&mut params, "rooms_max", &self.rooms_max);
        push_number_pair(&mut params, "size_min", &self.size_min);
        push_number_pair(&mut params, "size_max", &self.size_max);
        params.push(("page".to_owned(), self.page.to_string()));
        params.push(("per_page".to_owned(), self.per_page.to_string()));
        params
    }

    pub(crate) fn describe(&self) -> String {
        let filters = [
            describe_range("price", &self.price_min, &self.price_max),
            describe_range("rooms", &self.rooms_min, &self.rooms_max),
            describe_range("size", &self.size_min, &self.size_max),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let filter_text = if filters.is_empty() {
            String::new()
        } else {
            format!(", {}", filters.join(", "))
        };
        format!(
            "{} properties in {} (page {}, per_page {}{})",
            self.property_type, self.location, self.page, self.per_page, filter_text
        )
    }

    pub(crate) fn as_value(&self) -> Value {
        let mut params = Map::new();
        params.insert("location".to_owned(), Value::String(self.location.clone()));
        params.insert("type".to_owned(), Value::String(self.property_type.clone()));
        insert_optional_number(&mut params, "price_min", &self.price_min);
        insert_optional_number(&mut params, "price_max", &self.price_max);
        insert_optional_number(&mut params, "rooms_min", &self.rooms_min);
        insert_optional_number(&mut params, "rooms_max", &self.rooms_max);
        insert_optional_number(&mut params, "size_min", &self.size_min);
        insert_optional_number(&mut params, "size_max", &self.size_max);
        params.insert("page".to_owned(), json!(self.page));
        params.insert("per_page".to_owned(), json!(self.per_page));
        Value::Object(params)
    }
}

pub(crate) fn normalize_search_input(input: Option<&Value>) -> Result<SearchParams> {
    let input_object = input.and_then(Value::as_object);
    let has_known_input = input_object.is_some_and(|object| {
        [
            "location",
            "type",
            "price_min",
            "price_max",
            "rooms_min",
            "rooms_max",
            "size_min",
            "size_max",
            "per_page",
            "property_type",
            "page",
            "limit",
        ]
        .iter()
        .any(|key| object.contains_key(*key))
    });

    let mut normalized = json!({
        "location": DEFAULT_LOCATION,
        "type": DEFAULT_TYPE,
        "page": 1,
        "per_page": 20,
    });
    if !has_known_input {
        return build_search_params(&normalized);
    }

    let object = input_object.expect("known input requires an object");
    let fields = [
        ("location", None),
        ("type", Some("property_type")),
        ("price_min", None),
        ("price_max", None),
        ("rooms_min", None),
        ("rooms_max", None),
        ("size_min", None),
        ("size_max", None),
        ("page", None),
        ("per_page", Some("limit")),
    ];
    let normalized_object = normalized
        .as_object_mut()
        .expect("default input is a JSON object");
    for (field, alias) in fields {
        let value = object.get(field).or_else(|| {
            alias.and_then(|alias| {
                (!object.contains_key(field))
                    .then(|| object.get(alias))
                    .flatten()
            })
        });
        if let Some(value) = value {
            normalized_object.insert(field.to_owned(), trim_input_value(value));
        }
    }

    build_search_params(&normalized)
}

fn trim_input_value(value: &Value) -> Value {
    match value {
        Value::String(value) => Value::String(value.trim().to_owned()),
        _ => value.clone(),
    }
}

fn build_search_params(input: &Value) -> Result<SearchParams> {
    let location = clean_required_string(input.get("location"), "location", MAX_LOCATION_LENGTH)?;
    let property_type = clean_enum_string(input.get("type"), "type", &PROPERTY_TYPES)?;
    let page = clean_integer(input.get("page"), "page", 1, 10_000)?;
    let per_page = clean_integer(input.get("per_page"), "per_page", 1, MAX_PER_PAGE)?;
    let price_min = optional_integer(input.get("price_min"), "price_min", 0, 100_000_000)?;
    let price_max = optional_integer(input.get("price_max"), "price_max", 0, 100_000_000)?;
    let rooms_min = optional_number(input.get("rooms_min"), "rooms_min", 0.0, 100.0)?;
    let rooms_max = optional_number(input.get("rooms_max"), "rooms_max", 0.0, 100.0)?;
    let size_min = optional_integer(input.get("size_min"), "size_min", 0, 1_000_000)?;
    let size_max = optional_integer(input.get("size_max"), "size_max", 0, 1_000_000)?;
    validate_min_max("price_min", &price_min, "price_max", &price_max)?;
    validate_min_max("rooms_min", &rooms_min, "rooms_max", &rooms_max)?;
    validate_min_max("size_min", &size_min, "size_max", &size_max)?;

    Ok(SearchParams {
        location,
        property_type,
        price_min,
        price_max,
        rooms_min,
        rooms_max,
        size_min,
        size_max,
        page,
        per_page,
    })
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    let Some(Value::String(value)) = value else {
        bail!("{field} must be a string");
    };
    let value = value.trim();
    if value.is_empty() {
        bail!("{field} is required");
    }
    if value.chars().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(value.to_owned())
}

fn clean_enum_string(value: Option<&Value>, field: &str, allowed: &[&str]) -> Result<String> {
    let value = clean_required_string(value, field, 40)?;
    if !allowed.contains(&value.as_str()) {
        bail!("{field} must be one of: {}", allowed.join(", "));
    }
    Ok(value)
}

fn optional_integer(
    value: Option<&Value>,
    field: &str,
    min: i64,
    max: i64,
) -> Result<Option<Number>> {
    let Some(value) = nonempty_optional_value(value) else {
        return Ok(None);
    };
    let number = clean_number_value(value, field, true)?;
    if number.fract() != 0.0 {
        bail!("{field} must be an integer");
    }
    if number < min as f64 || number > max as f64 {
        bail!("{field} must be between {min} and {max}");
    }
    Ok(Some(Number::from(number as i64)))
}

fn optional_number(
    value: Option<&Value>,
    field: &str,
    min: f64,
    max: f64,
) -> Result<Option<Number>> {
    let Some(value) = nonempty_optional_value(value) else {
        return Ok(None);
    };
    let number = clean_number_value(value, field, false)?;
    if number < min || number > max {
        bail!(
            "{field} must be between {} and {}",
            format_number(min),
            format_number(max)
        );
    }
    let number = if number.fract() == 0.0 {
        Number::from(number as i64)
    } else {
        Number::from_f64(number).ok_or_else(|| anyhow!("{field} must be a number"))?
    };
    Ok(Some(number))
}

fn nonempty_optional_value(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| !value.is_null() && value.as_str() != Some(""))
}

fn clean_integer(value: Option<&Value>, field: &str, min: i64, max: i64) -> Result<i64> {
    let Some(value) = value else {
        bail!("{field} must be an integer");
    };
    let number = clean_number_value(value, field, true)?;
    if number.fract() != 0.0 {
        bail!("{field} must be an integer");
    }
    if number < min as f64 || number > max as f64 {
        bail!("{field} must be between {min} and {max}");
    }
    Ok(number as i64)
}

fn clean_number_value(value: &Value, field: &str, integer: bool) -> Result<f64> {
    let number = match value {
        Value::Number(number) => number.as_f64(),
        Value::String(value) => {
            let value = value.trim();
            let valid = if integer {
                valid_integer_string(value)
            } else {
                valid_decimal_string(value)
            };
            if valid {
                value.parse::<f64>().ok()
            } else {
                None
            }
        }
        _ => None,
    };
    let Some(number) = number.filter(|number| number.is_finite()) else {
        bail!(
            "{field} must be a {}",
            if integer { "integer" } else { "number" }
        );
    };
    Ok(number)
}

fn valid_integer_string(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_decimal_string(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    if value.is_empty() {
        return false;
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fractional = parts.next();
    !whole.is_empty()
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && parts.next().is_none()
        && fractional.is_none_or(|digits| {
            !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn validate_min_max(
    min_field: &str,
    min: &Option<Number>,
    max_field: &str,
    max: &Option<Number>,
) -> Result<()> {
    if let (Some(min), Some(max)) = (min, max) {
        if min.as_f64().unwrap_or_default() > max.as_f64().unwrap_or_default() {
            bail!("{min_field} must be less than or equal to {max_field}");
        }
    }
    Ok(())
}

fn push_number_pair(params: &mut Vec<(String, String)>, key: &str, value: &Option<Number>) {
    if let Some(value) = value {
        params.push((
            key.to_owned(),
            format_number(value.as_f64().unwrap_or_default()),
        ));
    }
}

fn insert_optional_number(params: &mut Map<String, Value>, key: &str, value: &Option<Number>) {
    params.insert(
        key.to_owned(),
        value.clone().map(Value::Number).unwrap_or(Value::Null),
    );
}

fn describe_range(label: &str, min: &Option<Number>, max: &Option<Number>) -> Option<String> {
    let min = min.as_ref().and_then(Number::as_f64);
    let max = max.as_ref().and_then(Number::as_f64);
    match (min, max) {
        (None, None) => None,
        (Some(min), Some(max)) => Some(format!(
            "{label} {}-{}",
            format_number(min),
            format_number(max)
        )),
        (Some(min), None) => Some(format!("{label} >= {}", format_number(min))),
        (None, Some(max)) => Some(format!("{label} <= {}", format_number(max))),
    }
}

fn format_number(number: f64) -> String {
    if number.fract() == 0.0 {
        format!("{number:.0}")
    } else {
        number.to_string()
    }
}
