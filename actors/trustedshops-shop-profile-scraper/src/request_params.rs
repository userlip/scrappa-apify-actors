use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;

const TSID_PATTERN: &str = r"^[A-Z0-9]{33}$";
const TSID_IN_URL_PATTERN: &str = r"(?i)(?:info_|tsid=)([A-Z0-9]{33})";
const BARE_TSID_IN_TEXT_PATTERN: &str = r"(?i)(?:^|[^A-Z0-9])([A-Z0-9]{33})";
const MAX_SHOPS_PER_RUN: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShopProfileRequest {
    pub tsid: Option<String>,
    pub source_url: Option<String>,
    pub validation_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShopProfilePlan {
    pub requests: Vec<ShopProfileRequest>,
    pub include_raw_response: bool,
}

#[derive(Clone, Copy)]
enum InputField {
    Tsid,
    Tsids,
    Url,
    Urls,
}

impl InputField {
    fn name(self) -> &'static str {
        match self {
            Self::Tsid => "tsid",
            Self::Tsids => "tsids",
            Self::Url => "url",
            Self::Urls => "urls",
        }
    }

    fn parses_list_string(self) -> bool {
        matches!(self, Self::Tsids | Self::Urls)
    }

    fn is_tsid(self) -> bool {
        matches!(self, Self::Tsid | Self::Tsids)
    }
}

fn collect_values(input: &Value, field: InputField) -> Vec<Value> {
    let Some(value) = input
        .as_object()
        .and_then(|object| object.get(field.name()))
    else {
        return Vec::new();
    };

    if value.is_null() || value.as_str() == Some("") {
        return Vec::new();
    }

    if let Some(values) = value.as_array() {
        return values.clone();
    }

    if field.parses_list_string()
        && let Some(value) = value.as_str()
    {
        return value
            .split(|character| character == ',' || character == '\n')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Value::String(value.to_owned()))
            .collect();
    }

    vec![value.clone()]
}

pub fn normalize_tsid(value: &Value, field: &str) -> Result<String, String> {
    let Some(value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };

    let tsid = value.trim().to_uppercase();
    if !Regex::new(TSID_PATTERN).unwrap().is_match(&tsid) {
        return Err(format!(
            "{field} must be a 33-character TrustedShops TSID using uppercase letters and numbers"
        ));
    }

    Ok(tsid)
}

pub fn extract_tsid_from_url(value: &Value, field: &str) -> Result<ShopProfileRequest, String> {
    let Some(value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };

    let source_url = value.trim();
    if source_url.is_empty() {
        return Err(format!("{field} cannot be empty"));
    }

    let Some(tsid) = find_tsid(source_url, TSID_IN_URL_PATTERN)
        .or_else(|| find_tsid(source_url, BARE_TSID_IN_TEXT_PATTERN))
    else {
        return Err(format!(
            "{field} must contain a TrustedShops TSID, for example https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html"
        ));
    };

    let tsid = normalize_tsid(&Value::String(tsid), field)?;

    Ok(ShopProfileRequest {
        tsid: Some(tsid),
        source_url: Some(source_url.to_owned()),
        validation_error: None,
    })
}

fn find_tsid(value: &str, pattern: &str) -> Option<String> {
    Regex::new(pattern)
        .unwrap()
        .captures_iter(value)
        .find_map(|captures| {
            let whole_match = captures.get(0)?;
            let tsid = captures.get(1)?;
            let next_character = value[whole_match.end()..].chars().next();
            if next_character.is_some_and(|character| character.is_ascii_alphanumeric()) {
                return None;
            }
            Some(tsid.as_str().to_owned())
        })
}

fn build_request(value: Value, field: InputField) -> ShopProfileRequest {
    let result = if field.is_tsid() {
        normalize_tsid(&value, field.name()).map(|tsid| ShopProfileRequest {
            tsid: Some(tsid),
            source_url: None,
            validation_error: None,
        })
    } else {
        extract_tsid_from_url(&value, field.name())
    };

    match result {
        Ok(request) => request,
        Err(validation_error) => ShopProfileRequest {
            tsid: None,
            source_url: value.as_str().map(|value| value.trim().to_owned()),
            validation_error: Some(validation_error),
        },
    }
}

pub fn build_plan(input: &Value) -> Result<ShopProfilePlan, String> {
    let fields = [
        InputField::Tsid,
        InputField::Tsids,
        InputField::Url,
        InputField::Urls,
    ];
    let values = fields
        .into_iter()
        .flat_map(|field| {
            collect_values(input, field)
                .into_iter()
                .map(move |value| (value, field))
        })
        .collect::<Vec<_>>();

    if values.is_empty() {
        return Err(
            "Provide tsids or urls. Batch multiple TrustedShops merchants in one run whenever possible."
                .to_owned(),
        );
    }

    let mut seen = HashSet::new();
    let mut requests = Vec::new();
    for (value, field) in values {
        let request = build_request(value, field);
        let key = request.tsid.clone().unwrap_or_else(|| {
            format!(
                "invalid:{}",
                request
                    .source_url
                    .as_deref()
                    .or(request.validation_error.as_deref())
                    .unwrap_or("")
            )
        });
        if seen.insert(key) {
            requests.push(request);
        }
    }

    if requests.len() > MAX_SHOPS_PER_RUN {
        return Err(format!(
            "A single run can include at most {MAX_SHOPS_PER_RUN} TrustedShops shops"
        ));
    }

    Ok(ShopProfilePlan {
        requests,
        include_raw_response: input
            .as_object()
            .and_then(|object| object.get("include_raw_response"))
            .and_then(Value::as_bool)
            == Some(true),
    })
}

pub fn describe_request(plan: &ShopProfilePlan) -> String {
    if plan.requests.len() == 1 {
        return plan.requests[0]
            .tsid
            .clone()
            .unwrap_or_else(|| "1 invalid input".to_owned());
    }

    format!("{} TrustedShops shop profile inputs", plan.requests.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TSID: &str = "XFB15FFBDE1DEE7A55D292A7D48598A6A";

    #[test]
    fn normalizes_valid_ids_and_rejects_malformed_ids() {
        assert_eq!(
            normalize_tsid(&json!(TSID.to_lowercase()), "tsid").unwrap(),
            TSID
        );
        assert!(
            normalize_tsid(&json!("X123"), "tsid")
                .unwrap_err()
                .contains("33-character TrustedShops TSID")
        );
    }

    #[test]
    fn extracts_tsid_from_profile_url_and_rejects_overlong_tokens() {
        let source_url = format!("https://www.trustedshops.de/bewertung/info_{TSID}.html");
        let request = extract_tsid_from_url(&json!(source_url), "url").unwrap();
        assert_eq!(request.tsid.as_deref(), Some(TSID));
        assert_eq!(request.source_url.as_deref(), Some(source_url.as_str()));

        let overlong_url = format!("https://www.trustedshops.de/bewertung/info_{TSID}A.html");
        let plan = build_plan(&json!({"urls": [overlong_url]})).unwrap();
        assert!(plan.requests[0].tsid.is_none());
        assert!(
            plan.requests[0]
                .validation_error
                .as_deref()
                .unwrap()
                .contains("must contain a TrustedShops TSID")
        );
    }

    #[test]
    fn builds_deduplicated_batch_from_single_and_list_inputs() {
        let plan = build_plan(&json!({
            "tsid": TSID.to_lowercase(),
            "tsids": [TSID],
            "urls": [format!("https://trustedshops.de/info_{TSID}.html")],
            "include_raw_response": true
        }))
        .unwrap();

        assert!(plan.include_raw_response);
        assert_eq!(
            plan.requests,
            vec![ShopProfileRequest {
                tsid: Some(TSID.to_owned()),
                source_url: None,
                validation_error: None,
            }]
        );
    }

    #[test]
    fn splits_string_lists_and_keeps_invalid_urls_as_request_failures() {
        let plan = build_plan(&json!({
            "tsids": format!("{TSID},\n{}", TSID.to_lowercase()),
            "urls": ["https://www.trustedshops.de/bewertung/no-tsid.html"]
        }))
        .unwrap();

        assert_eq!(plan.requests.len(), 2);
        assert_eq!(plan.requests[0].tsid.as_deref(), Some(TSID));
        assert_eq!(
            plan.requests[1].source_url.as_deref(),
            Some("https://www.trustedshops.de/bewertung/no-tsid.html")
        );
        assert!(plan.requests[1].validation_error.is_some());
    }

    #[test]
    fn requires_input_and_caps_deduplicated_batches_at_one_hundred() {
        assert!(
            build_plan(&json!({}))
                .unwrap_err()
                .contains("Provide tsids or urls")
        );
        assert!(
            build_plan(&json!({
                "tsids": (0..101).map(|index| format!("{index:033}")).collect::<Vec<_>>()
            }))
            .unwrap_err()
            .contains("at most 100")
        );
    }
}
