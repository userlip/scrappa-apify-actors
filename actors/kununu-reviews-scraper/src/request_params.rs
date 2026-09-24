use serde_json::{Map, Value, json};
use url::Url;

const COUNTRIES: &[&str] = &["de", "at", "ch"];
const REVIEW_TYPES: &[&str] = &["employees", "candidates"];
const SORT_VALUES: &[&str] = &["worst", "best", "newest", "oldest"];
const SCORE_FILTERS: &[&str] = &["excellent", "good", "satisfactory", "subpar"];
const RECOMMENDED_FILTERS: &[&str] = &["yes", "no"];
const JOBSTATUS_FILTERS: &[&str] = &["current", "former"];
const POSITION_FILTERS: &[&str] = &[
    "employee",
    "manager",
    "apprentice",
    "student",
    "intern",
    "freelancer",
    "contractor",
];
const DEPARTMENT_FILTERS: &[&str] = &[
    "administration",
    "sales",
    "legal",
    "operations",
    "recruiting",
    "communication",
    "product",
    "logistic",
    "it",
    "management",
    "research",
    "controlling",
    "design",
    "procurement",
];
const RESPONSE_FILTERS: &[&str] = &["yes", "no"];
const DATE_FILTERS: &[&str] = &["24months", "12months", "6months", "30days"];
const DEFAULT_COUNTRY: &str = "de";
const DEFAULT_REVIEW_TYPE: &str = "employees";

pub type RequestParams = Map<String, Value>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KununuTarget {
    pub country: String,
    pub company_slug: String,
    pub company_id: Option<String>,
    pub input: String,
}

#[derive(Debug)]
pub struct RequestPlan {
    pub targets: Vec<KununuTarget>,
    pub base_params: RequestParams,
    pub start_page: u32,
    pub max_pages: u32,
    pub include_raw_review: bool,
    pub include_raw_responses: bool,
}

impl KununuTarget {
    pub fn to_value(&self) -> Value {
        let mut value = Map::new();
        value.insert("country".into(), Value::String(self.country.clone()));
        value.insert(
            "company_slug".into(),
            Value::String(self.company_slug.clone()),
        );
        if let Some(company_id) = &self.company_id {
            value.insert("company_id".into(), Value::String(company_id.clone()));
        }
        value.insert("input".into(), Value::String(self.input.clone()));
        Value::Object(value)
    }
}

impl RequestPlan {
    pub fn targets_value(&self) -> Value {
        Value::Array(self.targets.iter().map(KununuTarget::to_value).collect())
    }
}

fn get<'a>(input: &'a Value, key: &str) -> Option<&'a Value> {
    input.as_object()?.get(key)
}

fn nullish<'a>(first: Option<&'a Value>, second: Option<&'a Value>) -> Option<&'a Value> {
    first.filter(|value| !value.is_null()).or(second)
}

fn clean_optional_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.encode_utf16().count() > max_length {
        return Err(format!("{field} must be {max_length} characters or fewer"));
    }
    Ok(Some(trimmed.to_owned()))
}

fn clean_integer(
    value: Option<&Value>,
    field: &str,
    min: u32,
    max: u32,
) -> Result<Option<u32>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(number) = value.as_f64() else {
        return Err(format!("{field} must be an integer"));
    };
    if !number.is_finite() || number.fract() != 0.0 {
        return Err(format!("{field} must be an integer"));
    }
    if number < f64::from(min) || number > f64::from(max) {
        return Err(format!("{field} must be between {min} and {max}"));
    }
    Ok(Some(number as u32))
}

fn clean_boolean(value: Option<&Value>, field: &str) -> Result<Option<bool>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| format!("{field} must be a boolean"))
}

fn clean_enum(
    value: Option<&Value>,
    field: &str,
    allowed_values: &[&str],
) -> Result<Option<String>, String> {
    let Some(cleaned) = clean_optional_string(value, field, 100)? else {
        return Ok(None);
    };
    let cleaned = cleaned.to_lowercase();
    if !allowed_values.contains(&cleaned.as_str()) {
        return Err(format!(
            "{field} must be one of: {}",
            allowed_values.join(", ")
        ));
    }
    Ok(Some(cleaned))
}

fn clean_string_array(
    value: Option<&Value>,
    field: &str,
    allowed_values: &[&str],
) -> Result<Option<Vec<Value>>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    let values: Vec<&Value> = match value {
        Value::Array(values) => values.iter().collect(),
        value => vec![value],
    };
    let mut cleaned = Vec::new();
    for value in values {
        let value = clean_optional_string(Some(value), field, 100)?;
        let Some(value) = value else {
            continue;
        };
        let value = value.to_lowercase();
        if !allowed_values.contains(&value.as_str()) {
            return Err(format!(
                "{field} values must be one of: {}",
                allowed_values.join(", ")
            ));
        }
        cleaned.push(Value::String(value));
    }
    Ok(Some(cleaned))
}

fn clean_uuid(value: Option<&Value>, field: &str) -> Result<Option<String>, String> {
    let Some(value) = clean_optional_string(value, field, 100)? else {
        return Ok(None);
    };
    if !is_uuid(&value) {
        return Err(format!("{field} must be a valid UUID"));
    }
    Ok(Some(value))
}

fn is_uuid(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 36
        || bytes[8] != b'-'
        || bytes[13] != b'-'
        || bytes[18] != b'-'
        || bytes[23] != b'-'
    {
        return false;
    }
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            continue;
        }
        if !byte.is_ascii_hexdigit() {
            return false;
        }
    }
    matches!(bytes[14].to_ascii_lowercase(), b'1'..=b'5')
        && matches!(bytes[19].to_ascii_lowercase(), b'8' | b'9' | b'a' | b'b')
}

#[derive(Debug)]
struct UrlTargetParts {
    country: String,
    company_slug: String,
}

fn parse_kununu_url(value: &str) -> Option<UrlTargetParts> {
    let raw_value = value.trim();
    let has_protocol = raw_value
        .get(..7)
        .is_some_and(|protocol| protocol.eq_ignore_ascii_case("http://"))
        || raw_value
            .get(..8)
            .is_some_and(|protocol| protocol.eq_ignore_ascii_case("https://"));
    let candidate = if has_protocol {
        raw_value.to_owned()
    } else {
        format!("https://{raw_value}")
    };
    let url = Url::parse(&candidate).ok()?;
    let host = url.host_str()?;
    if !host.eq_ignore_ascii_case("kununu.com")
        && !host.to_ascii_lowercase().ends_with(".kununu.com")
    {
        return None;
    }
    let mut path_parts = url.path().split('/').filter(|part| !part.is_empty());
    Some(UrlTargetParts {
        country: path_parts.next()?.to_lowercase(),
        company_slug: path_parts.next()?.to_lowercase(),
    })
}

fn clean_slug(value: Option<&Value>, field: &str) -> Result<Option<String>, String> {
    let Some(cleaned) = clean_optional_string(value, field, 255)? else {
        return Ok(None);
    };
    let cleaned = cleaned.to_lowercase();
    let slug = parse_kununu_url(&cleaned)
        .map(|parts| parts.company_slug)
        .unwrap_or_else(|| {
            cleaned
                .trim_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_owned()
        });
    if slug.is_empty()
        || !slug.as_bytes()[0].is_ascii_alphanumeric()
        || !slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(format!(
            "{field} must be a Kununu company slug, for example bmwgroup"
        ));
    }
    Ok(Some(slug))
}

fn is_supported_slug(value: &str) -> bool {
    !value.is_empty()
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn normalize_target(target: &Value, default_country: &str) -> Result<KununuTarget, String> {
    if let Some(value) = target.as_str() {
        let trimmed = value.trim();
        if let Some(url_parts) = parse_kununu_url(trimmed) {
            let country_value = Value::String(url_parts.country);
            let slug_value = Value::String(url_parts.company_slug);
            let country = clean_enum(Some(&country_value), "country", COUNTRIES)?
                .unwrap_or_else(|| default_country.to_owned());
            let company_slug = clean_slug(Some(&slug_value), "company_slug")?.ok_or_else(|| {
                "company_slug must be a Kununu company slug, for example bmwgroup".to_owned()
            })?;
            return Ok(KununuTarget {
                country,
                company_slug,
                company_id: None,
                input: trimmed.to_owned(),
            });
        }

        if let Some((country, slug)) = trimmed.split_once('/') {
            if !slug.contains('/')
                && COUNTRIES.contains(&country.to_ascii_lowercase().as_str())
                && is_supported_slug(slug)
            {
                return Ok(KununuTarget {
                    country: country.to_lowercase(),
                    company_slug: slug.to_lowercase(),
                    company_id: None,
                    input: trimmed.to_owned(),
                });
            }
            if trimmed.contains('/') {
                return Err("targets with a slash must use a supported country/slug pair such as de/bmwgroup, at/example, or ch/example".into());
            }
        }

        return Ok(KununuTarget {
            country: default_country.to_owned(),
            company_slug: clean_slug(Some(&Value::String(trimmed.to_owned())), "company_slug")?
                .ok_or_else(|| {
                    "company_slug must be a Kununu company slug, for example bmwgroup".to_owned()
                })?,
            company_id: None,
            input: trimmed.to_owned(),
        });
    }

    let Some(object_target) = target.as_object() else {
        return Err(
            "Each target must be a string, Kununu URL, or object with country and company_slug"
                .into(),
        );
    };
    let url_value = clean_optional_string(object_target.get("url"), "url", 500)?;
    let url_parts = url_value.as_deref().and_then(parse_kununu_url);
    let country_value = nullish(
        object_target.get("country"),
        object_target.get("countryCode"),
    )
    .cloned()
    .or_else(|| {
        url_parts
            .as_ref()
            .map(|parts| Value::String(parts.country.clone()))
    })
    .unwrap_or_else(|| Value::String(default_country.to_owned()));
    let country = clean_enum(Some(&country_value), "country", COUNTRIES)?
        .unwrap_or_else(|| default_country.to_owned());

    let raw_slug = nullish(
        object_target.get("company_slug"),
        nullish(object_target.get("slug"), None),
    )
    .cloned()
    .or_else(|| {
        url_parts
            .as_ref()
            .map(|parts| Value::String(parts.company_slug.clone()))
    });
    let company_slug = clean_slug(raw_slug.as_ref(), "company_slug")?;
    let Some(company_slug) = company_slug else {
        return Err(
            "Each target object must include company_slug, slug, or a Kununu company URL".into(),
        );
    };
    let company_id = clean_uuid(
        nullish(object_target.get("company_id"), object_target.get("uuid")),
        "company_id",
    )?;
    let input = url_value.unwrap_or_else(|| format!("{country}/{company_slug}"));

    Ok(KununuTarget {
        country,
        company_slug,
        company_id,
        input,
    })
}

fn build_targets(input: &Value) -> Result<Vec<KununuTarget>, String> {
    let default_country_value = nullish(get(input, "country"), get(input, "countryCode"))
        .cloned()
        .unwrap_or_else(|| Value::String(DEFAULT_COUNTRY.to_owned()));
    let default_country = clean_enum(Some(&default_country_value), "country", COUNTRIES)?
        .unwrap_or_else(|| DEFAULT_COUNTRY.to_owned());

    let batch_input = nullish(get(input, "targets"), get(input, "companies"));
    let raw_targets: Vec<&Value> = match batch_input {
        None => {
            let target = nullish(
                get(input, "url"),
                nullish(get(input, "company_slug"), get(input, "slug")),
            );
            target.into_iter().collect()
        }
        Some(Value::String(value)) if value.is_empty() => {
            let target = nullish(
                get(input, "url"),
                nullish(get(input, "company_slug"), get(input, "slug")),
            );
            target.into_iter().collect()
        }
        Some(Value::Array(targets)) => targets.iter().collect(),
        Some(target) => vec![target],
    };
    let targets = raw_targets
        .into_iter()
        .filter(|target| !target.is_null() && target.as_str() != Some(""))
        .map(|target| normalize_target(target, &default_country))
        .collect::<Result<Vec<_>, _>>()?;
    if targets.is_empty() {
        return Err("Provide at least one Kununu company target in targets, companies, company_slug, slug, or url".into());
    }
    if targets.len() > 25 {
        return Err("targets cannot contain more than 25 companies per run".into());
    }
    Ok(targets)
}

pub fn build_request_plan(input: &Value) -> Result<RequestPlan, String> {
    let targets = build_targets(input)?;
    let start_page = clean_integer(get(input, "page"), "page", 1, 100)?.unwrap_or(1);
    let max_pages = clean_integer(get(input, "max_pages"), "max_pages", 1, 25)?.unwrap_or(1);
    let include_raw_review =
        clean_boolean(get(input, "include_raw_review"), "include_raw_review")?.unwrap_or(false);
    let include_raw_responses =
        clean_boolean(get(input, "include_raw_responses"), "include_raw_responses")?
            .unwrap_or(false);

    let mut base_params = Map::new();
    let review_type = clean_enum(get(input, "review_type"), "review_type", REVIEW_TYPES)?
        .unwrap_or_else(|| DEFAULT_REVIEW_TYPE.to_owned());
    if review_type != DEFAULT_REVIEW_TYPE {
        base_params.insert("review_type".into(), Value::String(review_type));
    }
    if let Some(sort) = clean_enum(get(input, "sort"), "sort", SORT_VALUES)? {
        base_params.insert("sort".into(), Value::String(sort));
    }
    if let Some(fetch_factor_scores) =
        clean_boolean(get(input, "fetch_factor_scores"), "fetch_factor_scores")?
    {
        base_params.insert(
            "fetch_factor_scores".into(),
            json!(u8::from(fetch_factor_scores)),
        );
    }

    for (key, values) in [
        (
            "score_filters",
            clean_string_array(get(input, "score_filters"), "score_filters", SCORE_FILTERS)?,
        ),
        (
            "recommended_filters",
            clean_string_array(
                get(input, "recommended_filters"),
                "recommended_filters",
                RECOMMENDED_FILTERS,
            )?,
        ),
        (
            "jobstatus_filters",
            clean_string_array(
                get(input, "jobstatus_filters"),
                "jobstatus_filters",
                JOBSTATUS_FILTERS,
            )?,
        ),
        (
            "position_filters",
            clean_string_array(
                get(input, "position_filters"),
                "position_filters",
                POSITION_FILTERS,
            )?,
        ),
        (
            "department_filters",
            clean_string_array(
                get(input, "department_filters"),
                "department_filters",
                DEPARTMENT_FILTERS,
            )?,
        ),
        (
            "response_filters",
            clean_string_array(
                get(input, "response_filters"),
                "response_filters",
                RESPONSE_FILTERS,
            )?,
        ),
        (
            "date_filters",
            clean_string_array(get(input, "date_filters"), "date_filters", DATE_FILTERS)?,
        ),
    ] {
        if let Some(values) = values.filter(|values| !values.is_empty()) {
            base_params.insert(key.into(), Value::Array(values));
        }
    }

    Ok(RequestPlan {
        targets,
        base_params,
        start_page,
        max_pages,
        include_raw_review,
        include_raw_responses,
    })
}

pub fn page_params(plan: &RequestPlan, target: &KununuTarget, page: u32) -> RequestParams {
    let mut params = plan.base_params.clone();
    params.insert("country".into(), Value::String(target.country.clone()));
    params.insert(
        "company_slug".into(),
        Value::String(target.company_slug.clone()),
    );
    if let Some(company_id) = &target.company_id {
        params.insert("company_id".into(), Value::String(company_id.clone()));
    }
    params.insert("page".into(), json!(page));
    params
}

pub fn describe_request(plan: &RequestPlan) -> String {
    let last_page = plan.start_page + plan.max_pages - 1;
    let pages = if plan.max_pages == 1 {
        format!("page {}", plan.start_page)
    } else {
        format!("pages {}-{last_page}", plan.start_page)
    };
    let targets = plan
        .targets
        .iter()
        .map(|target| format!("{}/{}", target.country, target.company_slug))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{targets} ({pages})")
}

#[cfg(test)]
mod tests {
    use super::{build_request_plan, describe_request, page_params};
    use serde_json::{Value, json};

    const INPUT_SCHEMA: &str = include_str!("../.actor/input_schema.json");

    #[test]
    fn keeps_the_automated_quality_test_prefill() {
        let schema: Value = serde_json::from_str(INPUT_SCHEMA).unwrap();
        let prefill = schema["properties"]["targets"]["prefill"].clone();
        assert_eq!(prefill, json!(["de/bmwgroup"]));
        assert_eq!(
            build_request_plan(&json!({"targets": prefill}))
                .unwrap()
                .targets,
            build_request_plan(&json!({"targets": ["de/bmwgroup"]}))
                .unwrap()
                .targets
        );
    }

    #[test]
    fn normalizes_batches_aliases_filters_and_page_parameters() {
        let plan = build_request_plan(&json!({
            "targets": ["de/bmwgroup", "https://www.kununu.com/de/sap-se?utm=test"],
            "page": 2,
            "max_pages": 3,
            "review_type": "candidates",
            "sort": "newest",
            "fetch_factor_scores": true,
            "include_raw_review": true,
            "include_raw_responses": true,
            "score_filters": ["Excellent", "good"],
            "recommended_filters": "yes"
        }))
        .unwrap();
        assert_eq!(plan.targets[0].country, "de");
        assert_eq!(plan.targets[0].company_slug, "bmwgroup");
        assert_eq!(
            plan.targets[1].input,
            "https://www.kununu.com/de/sap-se?utm=test"
        );
        assert_eq!(plan.start_page, 2);
        assert_eq!(plan.max_pages, 3);
        assert!(plan.include_raw_review);
        assert!(plan.include_raw_responses);
        assert_eq!(plan.base_params["review_type"], "candidates");
        assert_eq!(plan.base_params["fetch_factor_scores"], 1);
        assert_eq!(
            plan.base_params["score_filters"],
            json!(["excellent", "good"])
        );
        assert_eq!(plan.base_params["recommended_filters"], json!(["yes"]));
        assert_eq!(
            page_params(&plan, &plan.targets[0], 2),
            json!({
                "review_type": "candidates",
                "sort": "newest",
                "fetch_factor_scores": 1,
                "score_filters": ["excellent", "good"],
                "recommended_filters": ["yes"],
                "country": "de",
                "company_slug": "bmwgroup",
                "page": 2
            })
            .as_object()
            .unwrap()
            .clone()
        );
    }

    #[test]
    fn supports_single_target_compatibility_inputs_and_country_aliases() {
        let plan = build_request_plan(&json!({
            "company_slug": " BMWGROUP ",
            "country": "DE"
        }))
        .unwrap();
        assert_eq!(plan.targets[0].country, "de");
        assert_eq!(plan.targets[0].company_slug, "bmwgroup");
        assert_eq!(plan.targets[0].input, "BMWGROUP");
        assert_eq!(describe_request(&plan), "de/bmwgroup (page 1)");

        let company_id = "123e4567-e89b-12d3-a456-426614174000";
        let alias_plan = build_request_plan(&json!({
            "companies": [{"countryCode": "AT", "slug": "example-company", "uuid": company_id}]
        }))
        .unwrap();
        assert_eq!(alias_plan.targets[0].country, "at");
        assert_eq!(alias_plan.targets[0].company_slug, "example-company");
        assert_eq!(
            alias_plan.targets[0].company_id.as_deref(),
            Some(company_id)
        );
        assert_eq!(alias_plan.targets[0].input, "at/example-company");
    }

    #[test]
    fn validates_targets_and_filter_values() {
        for (input, expected) in [
            (json!({}), "Provide at least one Kununu company target"),
            (
                json!({"targets": ["fr/example"]}),
                "targets with a slash must use a supported country/slug pair",
            ),
            (
                json!({"targets": ["bmwgroup"], "country": "fr"}),
                "country must be one of:",
            ),
            (
                json!({"targets": ["bmwgroup"], "max_pages": 26}),
                "max_pages must be between 1 and 25",
            ),
            (
                json!({"targets": ["bmwgroup"], "review_type": "customers"}),
                "review_type must be one of:",
            ),
            (
                json!({"targets": ["bmwgroup"], "score_filters": ["bad"]}),
                "score_filters values must be one of:",
            ),
            (
                json!({"targets": ["bmwgroup"], "include_raw_review": "true"}),
                "include_raw_review must be a boolean",
            ),
        ] {
            let error = build_request_plan(&input).unwrap_err();
            assert!(
                error.contains(expected),
                "{error} did not contain {expected}"
            );
        }
        assert!(
            build_request_plan(&json!({"targets": ["bmwgroup"], "page": 1.5}))
                .unwrap_err()
                .contains("page must be an integer")
        );
    }
}
