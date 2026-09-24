use std::collections::HashSet;

use anyhow::{bail, Result};
use serde_json::{json, Value};
use url::Url;

pub(crate) const JAMEDA_BASE_URL: &str = "https://www.jameda.de";
pub(crate) const SCRAPPA_MAX_URLS: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputFailure {
    pub(crate) doctor_url: String,
    pub(crate) error: String,
}

#[derive(Debug)]
pub(crate) struct DoctorDetailsPlan {
    pub(crate) doctor_urls: Vec<String>,
    pub(crate) input_failures: Vec<InputFailure>,
}

pub(crate) fn build_doctor_details_plan(input: &Value) -> Result<DoctorDetailsPlan> {
    let mut values: Vec<(&str, Value)> = Vec::new();
    let object = input.as_object();

    if let Some(value) = object.and_then(|object| object.get("doctorUrl")) {
        if !value.is_null() && value.as_str() != Some("") {
            values.push(("doctorUrl", value.clone()));
        }
    }

    if let Some(value) = object.and_then(|object| object.get("doctorUrls")) {
        if !value.is_null() && value.as_str() != Some("") {
            if let Some(array) = value.as_array() {
                values.extend(array.iter().map(|value| ("doctorUrls", value.clone())));
            } else if let Some(text) = value.as_str() {
                values.extend(
                    text.split(|character| character == ',' || character == '\n')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| ("doctorUrls", Value::String(value.to_owned()))),
                );
            } else {
                bail!("doctorUrls must be an array of strings or a comma/newline-separated string");
            }
        }
    }

    if values.is_empty() {
        bail!("Provide doctorUrls or doctorUrl");
    }

    let mut doctor_urls = Vec::new();
    let mut seen = HashSet::new();
    let mut input_failures = Vec::new();
    for (field, value) in values {
        match clean_jameda_doctor_url(&value, field) {
            Ok(url) => {
                if seen.insert(url.clone()) {
                    doctor_urls.push(url);
                }
            }
            Err(error) => input_failures.push(InputFailure {
                doctor_url: js_string(&value),
                error,
            }),
        }
    }

    if doctor_urls.is_empty() {
        bail!("No valid Jameda doctor URLs were provided");
    }
    if doctor_urls.len() > SCRAPPA_MAX_URLS {
        bail!("doctorUrls can include at most {SCRAPPA_MAX_URLS} doctor URLs per run");
    }

    Ok(DoctorDetailsPlan {
        doctor_urls,
        input_failures,
    })
}

pub(crate) fn clean_jameda_doctor_url(
    value: &Value,
    field: &str,
) -> std::result::Result<String, String> {
    let Some(raw_value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };
    let raw_value = raw_value.trim();
    if raw_value.is_empty() {
        return Err(format!("{field} cannot be empty"));
    }

    let lower_value = raw_value.to_ascii_lowercase();
    let parsed_url = if lower_value.starts_with("http://") || lower_value.starts_with("https://") {
        Url::parse(raw_value)
    } else if raw_value
        .split('/')
        .next()
        .unwrap_or_default()
        .contains('.')
    {
        Url::parse(&format!("https://{raw_value}"))
    } else {
        let path = if raw_value.starts_with('/') {
            raw_value.to_owned()
        } else {
            format!("/{raw_value}")
        };
        Url::parse(JAMEDA_BASE_URL).and_then(|base| base.join(&path))
    }
    .map_err(|_| format!("{field} must be a valid Jameda doctor URL or path"))?;

    let hostname = parsed_url.host_str().unwrap_or_default();
    let domain = hostname.strip_prefix("www.").unwrap_or(hostname);
    if !domain.eq_ignore_ascii_case("jameda.de") {
        return Err(format!("{field} must use the jameda.de domain"));
    }

    let normalized_path = normalize_path(parsed_url.path());
    if normalized_path == "/"
        || normalized_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .count()
            < 3
    {
        return Err(format!(
            "{field} must point to a Jameda doctor profile path, for example /markus-lietzau-msc/zahnarzt/berlin"
        ));
    }

    Ok(format!("{JAMEDA_BASE_URL}{normalized_path}"))
}

pub(crate) fn normalize_path(path: &str) -> String {
    let mut normalized = String::with_capacity(path.len());
    let mut previous_was_slash = false;
    for character in path.chars() {
        if character == '/' {
            if !previous_was_slash {
                normalized.push(character);
            }
            previous_was_slash = true;
        } else {
            normalized.push(character);
            previous_was_slash = false;
        }
    }
    while normalized.ends_with('/') {
        normalized.pop();
    }
    if normalized.starts_with('/') {
        normalized
    } else {
        format!("/{normalized}")
    }
}

pub(crate) fn build_doctor_details_params(doctor_url: &str) -> Vec<(String, String)> {
    vec![("doctor_url".to_owned(), doctor_url.to_owned())]
}

pub(crate) fn describe_request(doctor_urls: &[String]) -> String {
    if doctor_urls.len() == 1 {
        doctor_urls[0].clone()
    } else {
        format!("{} doctor URLs", doctor_urls.len())
    }
}

pub(crate) fn build_dataset_item(
    response: &Value,
    doctor_url: &str,
    params: &[(String, String)],
) -> Value {
    let profile = response
        .get("data")
        .filter(|value| !value.is_null())
        .unwrap_or(response);
    let basic_info = profile.get("basic_info").unwrap_or(&Value::Null);
    let rating = profile.get("rating").unwrap_or(&Value::Null);
    let clinic = profile.get("clinic").unwrap_or(&Value::Null);
    let contact = profile.get("contact").unwrap_or(&Value::Null);
    let coordinates = profile.get("coordinates").unwrap_or(&Value::Null);
    let metadata = profile.get("metadata").unwrap_or(&Value::Null);
    let scrape_metadata = response.get("meta").unwrap_or(&Value::Null);
    let address = profile.get("address").unwrap_or(&Value::Null);
    let rating_value = first_present(&[
        rating.get("rating"),
        rating.get("score"),
        rating.get("overall_score"),
    ]);
    let review_count = first_present(&[rating.get("count"), rating.get("review_count")]);
    let requested_url = params
        .iter()
        .find(|(name, _)| name == "doctor_url")
        .map(|(_, value)| json!(value))
        .unwrap_or(Value::Null);

    let mut item = response.as_object().cloned().unwrap_or_default();
    let fields = [
        ("requested_doctor_url", json!(doctor_url)),
        (
            "doctor_url",
            first_non_empty_string(&[basic_info.get("profile_url"), basic_info.get("url")])
                .map(|value| json!(value))
                .unwrap_or_else(|| json!(doctor_url)),
        ),
        (
            "doctor_name",
            first_non_empty_string(&[basic_info.get("name"), profile.get("name")])
                .map(|value| json!(value))
                .unwrap_or(Value::Null),
        ),
        ("title", value_or_null(basic_info.get("title"))),
        (
            "specialty",
            first_non_empty_string(&[
                basic_info.get("specialty"),
                basic_info.get("specializations"),
                profile.get("specialty"),
            ])
            .map(|value| json!(value))
            .unwrap_or(Value::Null),
        ),
        ("description", value_or_null(profile.get("description"))),
        ("rating", rating_value.cloned().unwrap_or(Value::Null)),
        (
            "rating_number",
            number_or_null(to_decimal_number(rating_value)),
        ),
        ("review_count", review_count.cloned().unwrap_or(Value::Null)),
        (
            "review_count_number",
            number_or_null(to_count_number(review_count)),
        ),
        (
            "clinic_name",
            first_non_empty_string(&[clinic.get("name")])
                .map(|value| json!(value))
                .unwrap_or(Value::Null),
        ),
        (
            "phone",
            first_non_empty_string(&[contact.get("phone")])
                .map(|value| json!(value))
                .unwrap_or(Value::Null),
        ),
        (
            "website_url",
            with_protocol(contact.get("website"))
                .map(|value| json!(value))
                .unwrap_or(Value::Null),
        ),
        ("address", json!(build_full_address(address))),
        (
            "city",
            address
                .as_object()
                .and_then(|value| value.get("city"))
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "postal_code",
            address
                .as_object()
                .and_then(|value| first_present(&[value.get("postal_code"), value.get("zip")]))
                .cloned()
                .unwrap_or(Value::Null),
        ),
        (
            "latitude",
            number_or_null(to_decimal_number(first_present(&[
                coordinates.get("latitude"),
                coordinates.get("lat"),
            ]))),
        ),
        (
            "longitude",
            number_or_null(to_decimal_number(first_present(&[
                coordinates.get("longitude"),
                coordinates.get("lng"),
            ]))),
        ),
        (
            "image_url",
            with_protocol(basic_info.get("image_url"))
                .map(|value| json!(value))
                .unwrap_or(Value::Null),
        ),
        ("services_count", count_items(profile.get("services"))),
        (
            "focus_areas_count",
            count_items(first_present(&[
                profile.get("focus_areas"),
                profile.get("specialization_focus"),
            ])),
        ),
        (
            "conditions_count",
            count_items(first_present(&[
                profile.get("conditions"),
                profile.get("conditions_treated"),
            ])),
        ),
        ("languages_count", count_items(profile.get("languages"))),
        ("opening_hours", value_or_null(profile.get("opening_hours"))),
        ("services", value_or_null(profile.get("services"))),
        (
            "accepted_patients",
            value_or_null(profile.get("accepted_patients")),
        ),
        (
            "focus_areas",
            value_or_null(first_present(&[
                profile.get("focus_areas"),
                profile.get("specialization_focus"),
            ])),
        ),
        (
            "conditions",
            value_or_null(first_present(&[
                profile.get("conditions"),
                profile.get("conditions_treated"),
            ])),
        ),
        ("languages", value_or_null(profile.get("languages"))),
        ("booking_ids", value_or_null(profile.get("booking_ids"))),
        ("request_doctor_url", requested_url),
        (
            "response_source",
            value_or_null(first_present(&[
                metadata.get("source"),
                scrape_metadata.get("source"),
            ])),
        ),
        (
            "scraped_at",
            value_or_null(first_present(&[
                metadata.get("scraped_at"),
                scrape_metadata.get("scraped_at"),
            ])),
        ),
    ];
    item.extend(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value)),
    );
    Value::Object(item)
}

pub(crate) fn build_output_summary(
    doctor_urls: &[String],
    saved_profiles: usize,
    failures: &[InputFailure],
    status_message: Option<&str>,
) -> Value {
    json!({
        "request": {
            "endpoint": "/jameda/doctor-details",
            "doctor_urls": doctor_urls,
        },
        "doctors_requested": doctor_urls.len(),
        "doctors_saved": saved_profiles,
        "doctors_failed": failures.len(),
        "responses_saved": saved_profiles,
        "status_message": status_message,
        "failures": failures.iter().map(|failure| json!({
            "doctor_url": failure.doctor_url,
            "error": failure.error,
        })).collect::<Vec<_>>(),
    })
}

pub(crate) fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
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

pub(crate) fn value_or_null(value: Option<&Value>) -> Value {
    value.cloned().unwrap_or(Value::Null)
}

pub(crate) fn first_present<'a>(values: &[Option<&'a Value>]) -> Option<&'a Value> {
    values
        .iter()
        .copied()
        .find(|value| value.is_some_and(|value| !value.is_null()))
        .flatten()
}

pub(crate) fn first_non_empty_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().copied().find_map(|value| {
        value
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

pub(crate) fn with_protocol(value: Option<&Value>) -> Option<String> {
    let value = value?.as_str()?.trim();
    if value.is_empty() {
        return None;
    }
    if value.starts_with("//") {
        Some(format!("https:{value}"))
    } else if value.starts_with("http://") || value.starts_with("https://") {
        Some(value.to_owned())
    } else {
        Some(format!("https://{value}"))
    }
}

pub(crate) fn build_full_address(address: &Value) -> Option<String> {
    if let Some(address) = address.as_str() {
        let address = address.trim();
        return (!address.is_empty()).then(|| address.to_owned());
    }
    let address = address.as_object()?;
    first_non_empty_string(&[address.get("full_address")]).or_else(|| {
        let postal_code = first_present(&[address.get("postal_code"), address.get("zip")]);
        let components = [address.get("street"), postal_code, address.get("city")]
            .into_iter()
            .filter_map(|value| {
                value
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
            })
            .collect::<Vec<_>>();
        (!components.is_empty()).then(|| components.join(", "))
    })
}

pub(crate) fn extract_numeric_string(value: Option<&Value>) -> Option<String> {
    if let Some(number) = value.and_then(Value::as_f64) {
        return number.is_finite().then(|| number.to_string());
    }
    let text = value?.as_str()?;
    let bytes = text.as_bytes();
    let mut start = None;
    let mut end = None;
    for index in 0..bytes.len() {
        let is_start = bytes[index].is_ascii_digit()
            || (bytes[index] == b'-' && bytes.get(index + 1).is_some_and(u8::is_ascii_digit));
        if start.is_none() && is_start {
            start = Some(index);
            continue;
        }
        if let Some(start_index) = start {
            if bytes[index].is_ascii_digit() || bytes[index] == b'.' || bytes[index] == b',' {
                end = Some(index + 1);
            } else if index > start_index {
                break;
            }
        }
    }
    let range = start?..end?;
    text.get(range).map(str::to_owned)
}

pub(crate) fn to_decimal_number(value: Option<&Value>) -> Option<f64> {
    if let Some(number) = value.and_then(Value::as_f64) {
        return number.is_finite().then_some(number);
    }
    let numeric = extract_numeric_string(value)?;
    let has_comma = numeric.contains(',');
    let has_dot = numeric.contains('.');
    let normalized = if has_comma && has_dot {
        if numeric.rfind(',')? > numeric.rfind('.')? {
            numeric.replace('.', "").replacen(',', ".", 1)
        } else {
            numeric.replace(',', "")
        }
    } else if has_comma {
        if is_grouped_digits(&numeric, ',') {
            numeric.replace(',', "")
        } else {
            numeric.replacen(',', ".", 1)
        }
    } else if is_multi_grouped_digits(&numeric, '.') {
        numeric.replace('.', "")
    } else {
        numeric
    };
    normalized
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

pub(crate) fn to_count_number(value: Option<&Value>) -> Option<f64> {
    if let Some(number) = value.and_then(Value::as_f64) {
        return number.is_finite().then_some(number);
    }
    let numeric = extract_numeric_string(value)?;
    let normalized = if is_grouped_count(&numeric) {
        numeric.replace([',', '.'], "")
    } else {
        numeric.replacen(',', ".", 1)
    };
    normalized
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

pub(crate) fn is_grouped_digits(value: &str, separator: char) -> bool {
    let groups = value.split(separator).collect::<Vec<_>>();
    groups.len() > 1
        && (1..=3).contains(&groups[0].len())
        && groups[0]
            .chars()
            .all(|character| character.is_ascii_digit())
        && groups[1..].iter().all(|group| {
            group.len() == 3 && group.chars().all(|character| character.is_ascii_digit())
        })
}

pub(crate) fn is_multi_grouped_digits(value: &str, separator: char) -> bool {
    let groups = value.split(separator).collect::<Vec<_>>();
    groups.len() >= 3
        && (1..=3).contains(&groups[0].len())
        && groups[0]
            .chars()
            .all(|character| character.is_ascii_digit())
        && groups[1..].iter().all(|group| {
            group.len() == 3 && group.chars().all(|character| character.is_ascii_digit())
        })
}

pub(crate) fn is_grouped_count(value: &str) -> bool {
    [',', '.']
        .into_iter()
        .any(|separator| is_grouped_digits(value, separator))
}

pub(crate) fn count_items(value: Option<&Value>) -> Value {
    value
        .and_then(Value::as_array)
        .map(|items| json!(items.len()))
        .unwrap_or(Value::Null)
}

pub(crate) fn number_or_null(number: Option<f64>) -> Value {
    number
        .and_then(serde_json::Number::from_f64)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}
