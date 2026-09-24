use serde_json::{Map, Value};
use url::Url;

const JAMEDA_BASE_URL: &str = "https://www.jameda.de";
const MAX_DOCTOR_URLS_PER_RUN: usize = 100;
const MAX_PAGE: i64 = 500;
const DEFAULT_PAGE: i64 = 1;
const DEFAULT_PER_PAGE: i64 = 20;
const MAX_PER_PAGE: i64 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorUrlFailure {
    pub doctor_url: String,
    pub error: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestPlan {
    pub doctor_urls: Vec<String>,
    pub input_failures: Vec<DoctorUrlFailure>,
    pub page: i64,
    pub sort: Option<String>,
    pub rating: Option<String>,
    pub per_page: i64,
}

#[derive(Debug)]
struct DoctorUrlInput {
    field: &'static str,
    value: Value,
}

pub fn build_request_plan(input: &Value) -> Result<RequestPlan, String> {
    let input = input
        .as_object()
        .ok_or_else(|| "Input is required".to_owned())?;
    let values = parse_doctor_url_inputs(input)?;
    if values.is_empty() {
        return Err("Provide doctor_urls or doctor_url".to_owned());
    }

    let mut doctor_urls = Vec::new();
    let mut input_failures = Vec::new();
    for DoctorUrlInput { field, value } in values {
        match clean_jameda_doctor_url(&value, field) {
            Ok(url) => doctor_urls.push(url),
            Err(error) => input_failures.push(DoctorUrlFailure {
                doctor_url: javascript_string(&value),
                error,
            }),
        }
    }

    let mut unique_doctor_urls = Vec::new();
    for url in doctor_urls {
        if !unique_doctor_urls.contains(&url) {
            unique_doctor_urls.push(url);
        }
    }

    if unique_doctor_urls.is_empty() {
        return Err("No valid Jameda doctor URLs were provided".to_owned());
    }
    if unique_doctor_urls.len() > MAX_DOCTOR_URLS_PER_RUN {
        return Err(format!(
            "doctor_urls can include at most {MAX_DOCTOR_URLS_PER_RUN} doctor URLs per run"
        ));
    }

    Ok(RequestPlan {
        doctor_urls: unique_doctor_urls,
        input_failures,
        page: clean_integer(input.get("page"), "page", 1, MAX_PAGE)?.unwrap_or(DEFAULT_PAGE),
        sort: clean_sort(input.get("sort"))?,
        rating: clean_rating_filter(input.get("rating"))?,
        per_page: clean_integer(input.get("per_page"), "per_page", 1, MAX_PER_PAGE)?
            .unwrap_or(DEFAULT_PER_PAGE),
    })
}

pub fn clean_jameda_doctor_url(value: &Value, field: &str) -> Result<String, String> {
    let Some(raw_value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };
    let raw_value = raw_value.trim();
    if raw_value.is_empty() {
        return Err(format!("{field} cannot be empty"));
    }

    let parsed_url = if raw_value
        .get(..7)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
        || raw_value
            .get(..8)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
    {
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
    let hostname_without_www = hostname
        .strip_prefix("www.")
        .or_else(|| hostname.strip_prefix("WWW."))
        .unwrap_or(hostname);
    if !hostname_without_www.eq_ignore_ascii_case("jameda.de") {
        return Err(format!("{field} must use the jameda.de domain"));
    }

    let pathname = normalize_path(parsed_url.path());
    if pathname == "/" || !has_jameda_review_profile_shape(&pathname) {
        return Err(format!(
            "{field} must point to a Jameda doctor profile path like /markus-lietzau-msc/zahnarzt/berlin or a supported /gesundheitseinrichtungen facility path"
        ));
    }

    let fragment = parsed_url
        .fragment()
        .map(|fragment| format!("#{fragment}"))
        .unwrap_or_default();
    Ok(format!("{JAMEDA_BASE_URL}{pathname}{fragment}"))
}

pub fn clean_rating_filter(value: Option<&Value>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || matches!(value, Value::String(value) if value.is_empty()) {
        return Ok(None);
    }

    let values = match value {
        Value::Array(values) => values.iter().map(javascript_string).collect::<Vec<_>>(),
        _ => javascript_string(value)
            .split(',')
            .map(str::to_owned)
            .collect(),
    };
    let ratings = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if ratings.is_empty() {
        return Ok(None);
    }
    if ratings
        .iter()
        .any(|rating| rating.len() != 1 || !matches!(rating.as_bytes()[0], b'1'..=b'5'))
    {
        return Err("rating must contain only values from 1 to 5".to_owned());
    }

    let mut unique = Vec::new();
    for rating in ratings {
        if !unique.contains(&rating) {
            unique.push(rating);
        }
    }
    Ok(Some(unique.join(",")))
}

pub fn build_request_params(plan: &RequestPlan, doctor_url: &str) -> Map<String, Value> {
    let mut params = Map::new();
    params.insert(
        "doctor_url".to_owned(),
        Value::String(doctor_url.to_owned()),
    );
    params.insert("page".to_owned(), Value::from(plan.page));
    params.insert("per_page".to_owned(), Value::from(plan.per_page));
    if let Some(sort) = &plan.sort {
        params.insert("sort".to_owned(), Value::String(sort.clone()));
    }
    if let Some(rating) = &plan.rating {
        params.insert("rating".to_owned(), Value::String(rating.clone()));
    }
    params
}

pub fn describe_request(plan: &RequestPlan) -> String {
    let url_description = if plan.doctor_urls.len() == 1 {
        plan.doctor_urls[0].clone()
    } else {
        format!("{} doctor URLs", plan.doctor_urls.len())
    };
    let mut filters = vec![
        format!("page {}", plan.page),
        format!("{} per page", plan.per_page),
    ];
    if let Some(sort) = &plan.sort {
        filters.push(format!("sort {sort}"));
    }
    if let Some(rating) = &plan.rating {
        filters.push(format!("rating {rating}"));
    }
    format!("{url_description} ({})", filters.join(", "))
}

fn parse_doctor_url_inputs(input: &Map<String, Value>) -> Result<Vec<DoctorUrlInput>, String> {
    let mut values = Vec::new();
    if let Some(value) = input.get("doctor_url") {
        if !value.is_null() && !matches!(value, Value::String(value) if value.is_empty()) {
            values.push(DoctorUrlInput {
                field: "doctor_url",
                value: value.clone(),
            });
        }
    }

    if let Some(value) = input.get("doctor_urls") {
        if !value.is_null() && !matches!(value, Value::String(value) if value.is_empty()) {
            match value {
                Value::Array(values_array) => {
                    values.extend(values_array.iter().cloned().map(|value| DoctorUrlInput {
                        field: "doctor_urls",
                        value,
                    }))
                }
                Value::String(value) => values.extend(
                    value
                        .split(|character| character == ',' || character == '\n')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| DoctorUrlInput {
                            field: "doctor_urls",
                            value: Value::String(value.to_owned()),
                        }),
                ),
                _ => {
                    return Err(
                        "doctor_urls must be an array of strings or a comma/newline-separated string"
                            .to_owned(),
                    );
                }
            }
        }
    }
    Ok(values)
}

fn clean_integer(
    value: Option<&Value>,
    field: &str,
    min: i64,
    max: i64,
) -> Result<Option<i64>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || matches!(value, Value::String(value) if value.is_empty()) {
        return Ok(None);
    }

    let normalized = match value {
        Value::String(value)
            if !value.trim().is_empty()
                && value.trim().bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            value.trim().parse::<f64>().ok()
        }
        Value::Number(value) => value.as_f64(),
        _ => None,
    };
    let Some(number) = normalized.filter(|number| number.is_finite() && number.fract() == 0.0)
    else {
        return Err(format!("{field} must be an integer"));
    };
    if number < min as f64 || number > max as f64 {
        return Err(format!("{field} must be between {min} and {max}"));
    }
    Ok(Some(number as i64))
}

fn clean_sort(value: Option<&Value>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || matches!(value, Value::String(value) if value.is_empty()) {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        return Err("sort must be a string".to_owned());
    };
    let sort = value.trim().to_ascii_lowercase();
    if !matches!(sort.as_str(), "newest" | "oldest" | "highest" | "lowest") {
        return Err("sort must be one of newest, oldest, highest, lowest".to_owned());
    }
    Ok(Some(sort))
}

fn normalize_path(pathname: &str) -> String {
    let mut normalized = String::with_capacity(pathname.len());
    let mut previous_was_slash = false;
    for character in pathname.chars() {
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
    let normalized = normalized.trim_end_matches('/');
    if normalized.is_empty() {
        "/".to_owned()
    } else if normalized.starts_with('/') {
        normalized.to_owned()
    } else {
        format!("/{normalized}")
    }
}

fn has_jameda_review_profile_shape(pathname: &str) -> bool {
    let segments = pathname
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.first() == Some(&"gesundheitseinrichtungen") {
        return segments.len() >= 2;
    }
    segments.len() >= 3
}

pub fn javascript_string(value: &Value) -> String {
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
                    javascript_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const MARKUS_URL: &str = "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin";

    #[test]
    fn normalizes_single_url_and_builds_scrappa_parameters() {
        let plan = build_request_plan(&json!({
            "doctor_url": " https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin?utm_source=test ",
            "page": "2",
            "sort": "highest",
            "rating": "4,5",
            "per_page": "50"
        }))
        .unwrap();
        assert_eq!(plan.doctor_urls, [MARKUS_URL]);
        assert_eq!(plan.page, 2);
        assert_eq!(plan.sort.as_deref(), Some("highest"));
        assert_eq!(plan.rating.as_deref(), Some("4,5"));
        assert_eq!(plan.per_page, 50);
        assert_eq!(
            Value::Object(build_request_params(&plan, MARKUS_URL)),
            json!({"doctor_url": MARKUS_URL, "page": 2, "per_page": 50, "sort": "highest", "rating": "4,5"})
        );
        assert_eq!(
            describe_request(&plan),
            format!("{MARKUS_URL} (page 2, 50 per page, sort highest, rating 4,5)")
        );
    }

    #[test]
    fn accepts_and_deduplicates_batch_urls() {
        let plan = build_request_plan(&json!({
            "doctor_url": MARKUS_URL,
            "doctor_urls": [
                MARKUS_URL,
                "/markus-lietzau-msc/zahnarzt/berlin",
                "markus-lietzau-msc/zahnarzt/berlin/",
                "https://www.jameda.de/anna-example/aerztin/hamburg"
            ]
        }))
        .unwrap();
        assert_eq!(
            plan.doctor_urls,
            [
                MARKUS_URL,
                "https://www.jameda.de/anna-example/aerztin/hamburg"
            ]
        );
        assert_eq!(
            describe_request(&plan),
            "2 doctor URLs (page 1, 20 per page)"
        );
    }

    #[test]
    fn accepts_comma_and_newline_separated_urls() {
        let plan = build_request_plan(&json!({
            "doctor_urls": format!("{MARKUS_URL}, /anna-example/aerztin/hamburg\n/hans-example/orthopaede/muenchen")
        }))
        .unwrap();
        assert_eq!(plan.doctor_urls.len(), 3);
        assert_eq!(
            plan.doctor_urls[2],
            "https://www.jameda.de/hans-example/orthopaede/muenchen"
        );
    }

    #[test]
    fn accepts_host_style_paths_facilities_and_fragments() {
        assert_eq!(
            clean_jameda_doctor_url(
                &json!("http://jameda.de/markus-lietzau-msc/zahnarzt/berlin"),
                "doctor_url"
            )
            .unwrap(),
            MARKUS_URL
        );
        assert_eq!(
            clean_jameda_doctor_url(
                &json!("www.jameda.de/markus-lietzau-msc/zahnarzt/berlin"),
                "doctor_url"
            )
            .unwrap(),
            MARKUS_URL
        );
        assert_eq!(
            clean_jameda_doctor_url(
                &json!("/gesundheitseinrichtungen/example-klinik#filters[doctor_id]=123"),
                "doctor_url"
            )
            .unwrap(),
            "https://www.jameda.de/gesundheitseinrichtungen/example-klinik#filters[doctor_id]=123"
        );
    }

    #[test]
    fn records_invalid_urls_when_valid_urls_remain() {
        let plan = build_request_plan(&json!({
            "doctor_urls": [
                MARKUS_URL,
                "https://example.com/markus-lietzau-msc/zahnarzt/berlin",
                "/search",
                123
            ]
        }))
        .unwrap();
        assert_eq!(plan.doctor_urls, [MARKUS_URL]);
        assert_eq!(plan.input_failures.len(), 3);
        assert!(plan.input_failures[0].error.contains("jameda.de domain"));
        assert!(plan.input_failures[1].error.contains("doctor profile path"));
        assert!(plan.input_failures[2]
            .error
            .contains("doctor_urls must be a string"));
        assert_eq!(plan.input_failures[2].doctor_url, "123");
    }

    #[test]
    fn normalizes_and_validates_rating_filters() {
        assert_eq!(
            clean_rating_filter(Some(&json!("5, 4,4")))
                .unwrap()
                .as_deref(),
            Some("5,4")
        );
        assert_eq!(
            clean_rating_filter(Some(&json!([1, "2", " 5 "])))
                .unwrap()
                .as_deref(),
            Some("1,2,5")
        );
        assert_eq!(clean_rating_filter(Some(&json!(""))).unwrap(), None);
        assert!(clean_rating_filter(Some(&json!("0,5")))
            .unwrap_err()
            .contains("only values from 1 to 5"));
    }

    #[test]
    fn rejects_missing_invalid_and_oversized_inputs() {
        assert!(build_request_plan(&json!({}))
            .unwrap_err()
            .contains("Provide doctor_urls or doctor_url"));
        assert!(build_request_plan(&json!({"doctor_urls": 123}))
            .unwrap_err()
            .contains("array of strings"));
        assert!(
            build_request_plan(&json!({"doctor_url": "https://example.com/a/b/c"}))
                .unwrap_err()
                .contains("No valid Jameda doctor URLs")
        );
        assert!(
            build_request_plan(&json!({"doctor_url": MARKUS_URL, "sort": "popular"}))
                .unwrap_err()
                .contains("sort must be one of")
        );
        assert!(
            build_request_plan(&json!({"doctor_url": MARKUS_URL, "per_page": 101}))
                .unwrap_err()
                .contains("per_page must be between 1 and 100")
        );
        let too_many = (0..101)
            .map(|index| format!("/doctor-{index}/zahnarzt/berlin"))
            .collect::<Vec<_>>();
        assert!(build_request_plan(&json!({"doctor_urls": too_many}))
            .unwrap_err()
            .contains("at most 100 doctor URLs"));
    }
}
