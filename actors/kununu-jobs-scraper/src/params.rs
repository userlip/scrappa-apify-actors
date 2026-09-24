use serde_json::{json, Map, Number, Value};

const DEFAULT_QUERY: &str = "Software Engineer";
const DEFAULT_LOCATION: &str = "Berlin";
const DEFAULT_COUNTRY: &str = "de";
const COUNTRIES: &[&str] = &["de", "at", "ch"];
const RADIUS_VALUES: &[i64] = &[10, 20, 30, 50, 100, 200];
const SORT_VALUES: &[&str] = &["newest", "kununuScore"];
const WORKPLACE_VALUES: &[&str] = &["FULL_REMOTE", "PARTLY_REMOTE", "NON_REMOTE"];
const EMPLOYMENT_TYPE_VALUES: &[&str] = &[
    "FULL_TIME",
    "PART_TIME",
    "INTERN",
    "TEMPORARY",
    "CONTRACTOR",
    "SEASONAL",
    "VOLUNTARY",
];
const CAREER_LEVEL_VALUES: &[&str] = &["1", "2", "3", "4", "5", "6", "99"];
const KUNUNU_SCORE_VALUES: &[&str] = &["4-5", "3-4", "2-3", "1-2"];
const BENEFIT_VALUES: &[&str] = &[
    "flexWorkingHours",
    "pensionPlan",
    "coaching",
    "mobilePhone",
    "internet",
    "healthProgram",
    "reachability",
    "events",
    "discounts",
    "parking",
    "car",
    "meals",
    "dogs",
    "daycare",
    "cantine",
    "stockOptions",
    "doctor",
    "accessibility",
    "material",
    "clothes",
    "transportation",
];

#[derive(Debug, PartialEq)]
pub struct SearchPlan {
    pub params: Map<String, Value>,
    pub start_page: i64,
    pub max_pages: usize,
    pub include_raw_job: bool,
}

pub fn build_search_plan(input: Option<&Value>) -> Result<SearchPlan, String> {
    let source = input.and_then(Value::as_object);
    let start_page =
        clean_integer(source.and_then(|value| value.get("page")), "page", 1, 100)?.unwrap_or(1);
    let max_pages = clean_integer(
        source.and_then(|value| value.get("max_pages")),
        "max_pages",
        1,
        10,
    )?
    .unwrap_or(1) as usize;

    let mut params = Map::new();
    params.insert(
        "query".into(),
        Value::String(
            clean_optional_string(source.and_then(|value| value.get("query")), "query", 120)?
                .unwrap_or_else(|| DEFAULT_QUERY.to_owned()),
        ),
    );
    params.insert(
        "location".into(),
        Value::String(
            clean_optional_string(
                source.and_then(|value| value.get("location")),
                "location",
                120,
            )?
            .unwrap_or_else(|| DEFAULT_LOCATION.to_owned()),
        ),
    );

    let country = clean_enum(
        source.and_then(|value| value.get("country")),
        "country",
        COUNTRIES,
        CaseNormalization::Lower,
    )?
    .unwrap_or_else(|| DEFAULT_COUNTRY.to_owned());
    params.insert("country".into(), Value::String(country));
    params.insert("page".into(), json!(start_page));

    if let Some(radius) = clean_integer(
        source.and_then(|value| value.get("radius")),
        "radius",
        10,
        200,
    )? {
        if !RADIUS_VALUES.contains(&radius) {
            let values = RADIUS_VALUES
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!("radius must be one of: {values}"));
        }
        params.insert("radius".into(), json!(radius));
    }

    if let Some(sort) = clean_enum(
        source.and_then(|value| value.get("sort")),
        "sort",
        SORT_VALUES,
        CaseNormalization::None,
    )? {
        params.insert("sort".into(), Value::String(sort));
    }

    add_array_param(
        &mut params,
        "workplace",
        clean_enum_array(
            source.and_then(|value| value.get("workplace")),
            "workplace",
            WORKPLACE_VALUES,
            CaseNormalization::Upper,
        )?,
    );
    add_array_param(
        &mut params,
        "employment_types",
        clean_enum_array(
            source.and_then(|value| value.get("employment_types")),
            "employment_types",
            EMPLOYMENT_TYPE_VALUES,
            CaseNormalization::Upper,
        )?,
    );
    add_array_param(
        &mut params,
        "career_level",
        clean_enum_array(
            source.and_then(|value| value.get("career_level")),
            "career_level",
            CAREER_LEVEL_VALUES,
            CaseNormalization::None,
        )?,
    );
    add_array_param(
        &mut params,
        "kununu_score",
        clean_enum_array(
            source.and_then(|value| value.get("kununu_score")),
            "kununu_score",
            KUNUNU_SCORE_VALUES,
            CaseNormalization::None,
        )?,
    );
    add_array_param(
        &mut params,
        "industry",
        clean_integer_array(
            source.and_then(|value| value.get("industry")),
            "industry",
            1,
            44,
        )?,
    );
    add_array_param(
        &mut params,
        "discipline",
        clean_integer_array(
            source.and_then(|value| value.get("discipline")),
            "discipline",
            1001,
            1022,
        )?,
    );
    add_array_param(
        &mut params,
        "benefits",
        clean_enum_array(
            source.and_then(|value| value.get("benefits")),
            "benefits",
            BENEFIT_VALUES,
            CaseNormalization::None,
        )?,
    );

    if let Some(is_top_company) = clean_boolean(
        source.and_then(|value| value.get("is_top_company")),
        "is_top_company",
    )? {
        params.insert("is_top_company".into(), Value::Bool(is_top_company));
    }

    let include_raw_job = clean_boolean(
        source.and_then(|value| value.get("include_raw_job")),
        "include_raw_job",
    )?
    .unwrap_or(false);

    Ok(SearchPlan {
        params,
        start_page,
        max_pages,
        include_raw_job,
    })
}

pub fn describe_request(params: &Map<String, Value>) -> String {
    let query = params
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_QUERY);
    let location = params
        .get("location")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_LOCATION);
    let country = params
        .get("country")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_COUNTRY);
    format!("\"{query}\" in {location}, {}", country.to_uppercase())
}

fn add_array_param(params: &mut Map<String, Value>, key: &str, values: Option<Vec<Value>>) {
    if let Some(values) = values.filter(|values| !values.is_empty()) {
        params.insert(key.to_owned(), Value::Array(values));
    }
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
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        return Err(format!("{field} must be {max_length} characters or fewer"));
    }
    Ok(Some(value.to_owned()))
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
    if value.is_null() {
        return Ok(None);
    }

    let number = match value {
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            parse_js_number(trimmed).ok_or_else(|| format!("{field} must be an integer"))?
        }
        Value::Number(value) => value
            .as_f64()
            .ok_or_else(|| format!("{field} must be an integer"))?,
        _ => return Err(format!("{field} must be an integer")),
    };

    if !number.is_finite() || number.fract() != 0.0 {
        return Err(format!("{field} must be an integer"));
    }
    if number < min as f64 || number > max as f64 {
        return Err(format!("{field} must be between {min} and {max}"));
    }
    Ok(Some(number as i64))
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
    value.parse::<f64>().ok()
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

#[derive(Clone, Copy)]
enum CaseNormalization {
    None,
    Lower,
    Upper,
}

impl CaseNormalization {
    fn apply(self, value: &str) -> String {
        match self {
            Self::None => value.to_owned(),
            Self::Lower => value.to_lowercase(),
            Self::Upper => value.to_uppercase(),
        }
    }
}

fn clean_enum(
    value: Option<&Value>,
    field: &str,
    allowed_values: &[&str],
    normalization: CaseNormalization,
) -> Result<Option<String>, String> {
    let Some(value) = clean_optional_string(value, field, 100)? else {
        return Ok(None);
    };
    let normalized = normalization.apply(&value);
    if !allowed_values.contains(&normalized.as_str()) {
        return Err(format!(
            "{field} must be one of: {}",
            allowed_values.join(", ")
        ));
    }
    Ok(Some(normalized))
}

fn clean_enum_array(
    value: Option<&Value>,
    field: &str,
    allowed_values: &[&str],
    normalization: CaseNormalization,
) -> Result<Option<Vec<Value>>, String> {
    let Some(values) = clean_array_values(value) else {
        return Ok(None);
    };
    let mut cleaned = Vec::new();
    for value in values {
        let string_value = match value {
            Value::Number(number) => Some(Value::String(js_number_string(&number))),
            value => Some(value),
        };
        if let Some(value) =
            clean_enum(string_value.as_ref(), field, allowed_values, normalization)?
        {
            let value = Value::String(value);
            if !cleaned.contains(&value) {
                cleaned.push(value);
            }
        }
    }
    Ok(Some(cleaned))
}

fn js_number_string(number: &Number) -> String {
    let Some(value) = number.as_f64() else {
        return number.to_string();
    };
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e21 {
        format!("{value:.0}")
    } else {
        number.to_string()
    }
}

fn clean_integer_array(
    value: Option<&Value>,
    field: &str,
    min: i64,
    max: i64,
) -> Result<Option<Vec<Value>>, String> {
    let Some(values) = clean_array_values(value) else {
        return Ok(None);
    };
    let mut cleaned = Vec::new();
    for value in values {
        if let Some(value) = clean_integer(Some(&value), field, min, max)? {
            let value = Value::Number(Number::from(value));
            if !cleaned.contains(&value) {
                cleaned.push(value);
            }
        }
    }
    Ok(Some(cleaned))
}

fn clean_array_values(value: Option<&Value>) -> Option<Vec<Value>> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) if value.is_empty() => None,
        Some(Value::Array(values)) => Some(values.clone()),
        Some(value) => Some(vec![value.clone()]),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_search_plan, describe_request};
    use serde_json::{json, Value};

    #[test]
    fn builds_defaults_and_normalizes_all_filters() {
        assert_eq!(
            build_search_plan(None).unwrap(),
            build_search_plan(Some(&json!({}))).unwrap()
        );
        assert_eq!(
            build_search_plan(None).unwrap(),
            super::SearchPlan {
                params: serde_json::from_value(json!({
                    "query": "Software Engineer",
                    "location": "Berlin",
                    "country": "de",
                    "page": 1
                }))
                .unwrap(),
                start_page: 1,
                max_pages: 1,
                include_raw_job: false,
            }
        );

        let plan = build_search_plan(Some(&json!({
            "query": "  Product Manager ",
            "location": " Munich ",
            "country": " DE ",
            "page": "2",
            "max_pages": "3",
            "radius": 100,
            "sort": "kununuScore",
            "workplace": ["full_remote", "PARTLY_REMOTE", "FULL_REMOTE"],
            "employment_types": ["full_time", "part_time"],
            "career_level": [3.0, "4"],
            "kununu_score": ["4-5", "3-4"],
            "industry": [12, "13"],
            "discipline": ["1001", 1002],
            "benefits": ["flexWorkingHours", "pensionPlan"],
            "is_top_company": true,
            "include_raw_job": true
        })))
        .unwrap();

        assert_eq!(plan.params["query"], "Product Manager");
        assert_eq!(plan.params["location"], "Munich");
        assert_eq!(plan.params["country"], "de");
        assert_eq!(plan.params["page"], 2);
        assert_eq!(plan.params["radius"], 100);
        assert_eq!(
            plan.params["workplace"],
            json!(["FULL_REMOTE", "PARTLY_REMOTE"])
        );
        assert_eq!(
            plan.params["employment_types"],
            json!(["FULL_TIME", "PART_TIME"])
        );
        assert_eq!(plan.params["career_level"], json!(["3", "4"]));
        assert_eq!(plan.params["kununu_score"], json!(["4-5", "3-4"]));
        assert_eq!(plan.params["industry"], json!([12, 13]));
        assert_eq!(plan.params["discipline"], json!([1001, 1002]));
        assert_eq!(
            plan.params["benefits"],
            json!(["flexWorkingHours", "pensionPlan"])
        );
        assert_eq!(plan.params["is_top_company"], true);
        assert_eq!(plan.start_page, 2);
        assert_eq!(plan.max_pages, 3);
        assert!(plan.include_raw_job);
    }

    #[test]
    fn treats_empty_optional_inputs_as_omitted_and_keeps_false() {
        let plan = build_search_plan(Some(&json!({
            "page": "   ",
            "max_pages": " ",
            "radius": "",
            "query": " \n ",
            "is_top_company": false
        })))
        .unwrap();
        assert_eq!(plan.start_page, 1);
        assert_eq!(plan.max_pages, 1);
        assert_eq!(plan.params["query"], "Software Engineer");
        assert_eq!(plan.params["is_top_company"], false);
        assert!(!plan.params.contains_key("radius"));
    }

    #[test]
    fn validates_enums_ranges_types_and_unicode_string_lengths() {
        let invalid = [
            (json!({"country":"us"}), "country must be one of"),
            (json!({"radius":25}), "radius must be one of"),
            (
                json!({"max_pages":11}),
                "max_pages must be between 1 and 10",
            ),
            (json!({"workplace":["REMOTE"]}), "workplace must be one of"),
            (
                json!({"industry":[45]}),
                "industry must be between 1 and 44",
            ),
            (
                json!({"discipline":[999]}),
                "discipline must be between 1001 and 1022",
            ),
            (
                json!({"is_top_company":"true"}),
                "is_top_company must be a boolean",
            ),
            (json!({"query":true}), "query must be a string"),
            (
                json!({"query":"😀".repeat(61)}),
                "query must be 120 characters or fewer",
            ),
        ];

        for (input, expected) in invalid {
            let error = build_search_plan(Some(&input)).unwrap_err();
            assert!(
                error.contains(expected),
                "{error:?} did not contain {expected:?}"
            );
        }
    }

    #[test]
    fn handles_js_integer_string_forms_and_deduplicates_array_values() {
        let plan = build_search_plan(Some(&json!({
            "page": "0x2",
            "industry": ["12", 12, null, ""]
        })))
        .unwrap();
        assert_eq!(plan.start_page, 2);
        assert_eq!(plan.params["industry"], json!([12]));
    }

    #[test]
    fn formats_request_description() {
        let plan = build_search_plan(Some(&json!({
            "query": "Data Analyst",
            "location": "Vienna",
            "country": "at"
        })))
        .unwrap();
        assert_eq!(
            describe_request(&plan.params),
            "\"Data Analyst\" in Vienna, AT"
        );
        assert!(Value::Object(plan.params).is_object());
    }
}
