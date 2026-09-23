use serde_json::{Map, Number, Value, json};
use url::Url;

pub type RequestParams = Map<String, Value>;

#[derive(Debug)]
pub struct RequestPlan {
    pub base_params: RequestParams,
    pub start_page: u32,
    pub max_pages: u32,
}

const LOCALES: &[&str] = &[
    "da-DK", "de-AT", "de-CH", "de-DE", "en-AU", "en-CA", "en-GB", "en-IE", "en-NZ", "en-US",
    "es-ES", "fi-FI", "fr-BE", "nl-BE", "fr-FR", "it-IT", "ja-JP", "nb-NO", "nl-NL", "pl-PL",
    "pt-BR", "pt-PT", "sv-SE",
];
const SORT_VALUES: &[&str] = &["relevance", "recency"];
const DATE_POSTED_VALUES: &[&str] = &[
    "any",
    "last_12_months",
    "last_6_months",
    "last_3_months",
    "last_30_days",
];
const DEFAULT_LOCALE: &str = "en-US";
const DEFAULT_PER_PAGE: u32 = 20;
const DEFAULT_SORT: &str = "recency";
const DEFAULT_DATE_POSTED: &str = "any";

fn clean_domain(value: Option<&Value>) -> Result<String, String> {
    let Some(value) = value.and_then(Value::as_str) else {
        return Err("company_domain is required and must be a string".into());
    };

    let raw_value = value.trim();
    let has_scheme = raw_value
        .get(..7)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
        || raw_value
            .get(..8)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"));
    let candidate = if has_scheme {
        raw_value.to_owned()
    } else {
        format!("https://{raw_value}")
    };
    let domain = Url::parse(&candidate)
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .map(|host| host.strip_prefix("www.").unwrap_or(&host).to_owned())
        .filter(|host| {
            let Some((_, suffix)) = host.rsplit_once('.') else {
                return false;
            };
            suffix.len() >= 2
                && suffix.bytes().all(|byte| byte.is_ascii_lowercase())
                && host.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b".-".contains(&byte)
                })
        })
        .ok_or_else(|| {
            "company_domain must be a valid domain name, for example amazon.com".to_owned()
        })?;

    if domain.len() > 255 {
        return Err("company_domain must be 255 characters or fewer".into());
    }
    Ok(domain)
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

    let Some(number) = value.as_number() else {
        return Err(format!("{field} must be an integer"));
    };
    let Some(number_value) = number.as_f64() else {
        return Err(format!("{field} must be an integer"));
    };
    if number_value.fract() != 0.0 {
        return Err(format!("{field} must be an integer"));
    }
    if number_value < f64::from(min) || number_value > f64::from(max) {
        return Err(format!("{field} must be between {min} and {max}"));
    }
    Ok(Some(number_value as u32))
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

fn clean_enum<'a>(
    value: Option<&Value>,
    field: &str,
    allowed_values: &'a [&'a str],
) -> Result<Option<&'a str>, String> {
    let Some(value) = clean_optional_string(value, field, 100)? else {
        return Ok(None);
    };
    allowed_values
        .iter()
        .copied()
        .find(|allowed| *allowed == value)
        .map(Some)
        .ok_or_else(|| format!("{field} must be one of: {}", allowed_values.join(", ")))
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

fn clean_rating(value: Option<&Value>) -> Result<Option<String>, String> {
    let Some(rating) = clean_optional_string(value, "rating", 20)? else {
        return Ok(None);
    };
    let rating: String = rating
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let valid = !rating.is_empty()
        && rating
            .split(',')
            .all(|part| matches!(part, "1" | "2" | "3" | "4" | "5"));
    if !valid {
        return Err(
            "rating must be a comma-separated list of ratings from 1 to 5, for example 4,5".into(),
        );
    }
    Ok(Some(rating))
}

pub fn build_request_plan(input: &Value) -> Result<RequestPlan, String> {
    let fields = input.as_object();
    let company_domain = clean_domain(fields.and_then(|fields| fields.get("company_domain")))?;
    let start_page =
        clean_integer(fields.and_then(|fields| fields.get("page")), "page", 1, 10)?.unwrap_or(1);
    let max_pages = clean_integer(
        fields.and_then(|fields| fields.get("max_pages")),
        "max_pages",
        1,
        10,
    )?
    .unwrap_or(1);
    if start_page + max_pages - 1 > 10 {
        return Err("page plus max_pages cannot request beyond page 10".into());
    }

    let mut base_params = Map::new();
    base_params.insert("company_domain".into(), Value::String(company_domain));
    let per_page = clean_integer(
        fields.and_then(|fields| fields.get("per_page")),
        "per_page",
        1,
        100,
    )?
    .unwrap_or(DEFAULT_PER_PAGE);
    base_params.insert("per_page".into(), json!(per_page));
    let locale = clean_enum(
        fields.and_then(|fields| fields.get("locale")),
        "locale",
        LOCALES,
    )?
    .unwrap_or(DEFAULT_LOCALE);
    base_params.insert("locale".into(), Value::String(locale.into()));

    let sort = clean_enum(
        fields.and_then(|fields| fields.get("sort")),
        "sort",
        SORT_VALUES,
    )?
    .unwrap_or(DEFAULT_SORT);
    if sort != DEFAULT_SORT {
        base_params.insert("sort".into(), Value::String(sort.into()));
    }
    if let Some(rating) = clean_rating(fields.and_then(|fields| fields.get("rating")))? {
        base_params.insert("rating".into(), Value::String(rating));
    }
    if let Some(verified) =
        clean_boolean(fields.and_then(|fields| fields.get("verified")), "verified")?
    {
        base_params.insert("verified".into(), json!(u8::from(verified)));
    }
    if let Some(with_replies) = clean_boolean(
        fields.and_then(|fields| fields.get("with_replies")),
        "with_replies",
    )? {
        base_params.insert("with_replies".into(), json!(u8::from(with_replies)));
    }
    if let Some(query) =
        clean_optional_string(fields.and_then(|fields| fields.get("query")), "query", 200)?
    {
        base_params.insert("query".into(), Value::String(query));
    }
    let date_posted = clean_enum(
        fields.and_then(|fields| fields.get("date_posted")),
        "date_posted",
        DATE_POSTED_VALUES,
    )?
    .unwrap_or(DEFAULT_DATE_POSTED);
    if date_posted != DEFAULT_DATE_POSTED {
        base_params.insert("date_posted".into(), Value::String(date_posted.into()));
    }
    if let Some(fields) = clean_optional_string(
        fields.and_then(|fields| fields.get("fields")),
        "fields",
        500,
    )? {
        base_params.insert("fields".into(), Value::String(fields));
    }

    Ok(RequestPlan {
        base_params,
        start_page,
        max_pages,
    })
}

pub fn page_params(plan: &RequestPlan, page: u32) -> RequestParams {
    let mut params = plan.base_params.clone();
    params.insert("page".into(), Value::Number(Number::from(page)));
    params
}

pub fn describe_request(plan: &RequestPlan) -> String {
    let company_domain = plan.base_params["company_domain"]
        .as_str()
        .unwrap_or_default();
    let last_page = plan.start_page + plan.max_pages - 1;
    let pages = if plan.max_pages == 1 {
        format!("page {}", plan.start_page)
    } else {
        format!("pages {}-{last_page}", plan.start_page)
    };
    format!("{company_domain} ({pages})")
}

#[cfg(test)]
mod tests {
    use super::{build_request_plan, describe_request, page_params};
    use serde_json::json;

    #[test]
    fn normalizes_input_and_maps_page_parameters() {
        let plan = build_request_plan(&json!({
            "company_domain": " https://www.Amazon.com/review-path ",
            "page": 2,
            "max_pages": 3,
            "sort": "recency",
        }))
        .unwrap();
        assert_eq!(
            json!(plan.base_params),
            json!({
                "company_domain": "amazon.com",
                "locale": "en-US",
                "per_page": 20,
            })
        );
        assert_eq!(
            json!(page_params(&plan, 3)),
            json!({
                "company_domain": "amazon.com",
                "locale": "en-US",
                "per_page": 20,
                "page": 3,
            })
        );
        assert_eq!(describe_request(&plan), "amazon.com (pages 2-4)");
    }

    #[test]
    fn preserves_filters_and_rejects_invalid_ranges() {
        let plan = build_request_plan(&json!({
            "company_domain": "example.com",
            "rating": " 1, 2 ",
            "verified": true,
            "with_replies": false,
            "query": " refund ",
            "date_posted": "last_30_days",
            "fields": "reviews,pagination",
            "sort": "relevance",
        }))
        .unwrap();
        assert_eq!(
            json!(plan.base_params),
            json!({
                "company_domain": "example.com",
                "locale": "en-US",
                "per_page": 20,
                "sort": "relevance",
                "rating": "1,2",
                "verified": 1,
                "with_replies": 0,
                "query": "refund",
                "date_posted": "last_30_days",
                "fields": "reviews,pagination",
            })
        );
        assert_eq!(
            build_request_plan(
                &json!({"company_domain": "example.com", "page": 8, "max_pages": 4})
            )
            .unwrap_err(),
            "page plus max_pages cannot request beyond page 10"
        );
    }
}
