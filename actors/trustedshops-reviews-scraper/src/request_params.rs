use percent_encoding::percent_decode_str;
use serde_json::{Map, Value, json};
use std::collections::HashSet;

const VALID_MARKETS: &[&str] = &[
    "DEU", "GBR", "AUT", "CHE", "NLD", "ESP", "ITA", "FRA", "BEL", "POL", "PRT",
];
const TSID_LENGTH: usize = 33;
const DEFAULT_SIZE: u64 = 20;
const MAX_SIZE: u64 = 100;
const MAX_TARGETS_PER_RUN: usize = 50;
const MAX_PAGE: u64 = 100;
const MAX_PAGES_PER_TARGET: u64 = 25;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedShopsTarget {
    pub tsid: String,
    pub input: String,
    pub source_url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestPlan {
    pub targets: Vec<TrustedShopsTarget>,
    pub base_params: Map<String, Value>,
    pub start_page: u64,
    pub max_pages: u64,
    pub include_raw_responses: bool,
}

pub fn build_request_plan(input: &Value) -> Result<RequestPlan, String> {
    let mut targets = Vec::new();
    for (field, value) in [
        ("tsids", input.get("tsids")),
        ("urls", input.get("urls")),
        ("tsid", input.get("tsid")),
        ("url", input.get("url")),
    ] {
        for value in clean_string_array(value, field)? {
            targets.push(build_target(&value, field)?);
        }
    }

    let mut seen = HashSet::new();
    targets.retain(|target| seen.insert(target.tsid.clone()));
    if targets.is_empty() {
        return Err("Provide at least one Trusted Shops TSID or profile URL in tsids or urls".into());
    }
    if targets.len() > MAX_TARGETS_PER_RUN {
        return Err(format!("Provide {MAX_TARGETS_PER_RUN} or fewer Trusted Shops targets per run"));
    }

    let start_page = clean_integer(input.get("page"), "page", 1, MAX_PAGE)?.unwrap_or(1);
    let max_pages = clean_integer(input.get("max_pages"), "max_pages", 1, MAX_PAGES_PER_TARGET)?
        .unwrap_or(1);
    if start_page + max_pages - 1 > MAX_PAGE {
        return Err("page plus max_pages cannot exceed page 100".into());
    }

    let size = clean_integer(input.get("size"), "size", 1, MAX_SIZE)?.unwrap_or(DEFAULT_SIZE);
    let mut base_params = Map::new();
    base_params.insert("size".into(), json!(size));
    if let Some(market) = clean_market(input.get("market"))? {
        base_params.insert("market".into(), json!(market));
    }

    let include_raw_responses = clean_boolean(input.get("include_raw_responses"), "include_raw_responses")?
        .unwrap_or(false);

    Ok(RequestPlan {
        targets,
        base_params,
        start_page,
        max_pages,
        include_raw_responses,
    })
}

pub fn page_params(plan: &RequestPlan, page: u64) -> Map<String, Value> {
    let mut params = plan.base_params.clone();
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
    format!("{} Trusted Shops target(s) ({pages})", plan.targets.len())
}

pub fn extract_tsid(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if is_tsid(trimmed) {
        return Some(trimmed.to_ascii_uppercase());
    }

    let decoded = percent_decode_str(trimmed).decode_utf8_lossy();
    let lowered = decoded.to_ascii_lowercase();
    let prefixes = ["info_", "tsid=", "tsid/", "trustedshops/reviews/"];
    let mut earliest = None;
    for prefix in prefixes {
        if let Some(index) = lowered.find(prefix) {
            earliest = Some(earliest.map_or(index, |current: usize| current.min(index)));
        }
    }
    if let Some(index) = earliest {
        let prefix = prefixes
            .iter()
            .filter_map(|prefix| lowered[index..].starts_with(prefix).then_some(*prefix))
            .max_by_key(|prefix| prefix.len());
        if let Some(prefix) = prefix
            && let Some(tsid) = find_tsid_at(&decoded, index + prefix.len())
        {
            return Some(tsid);
        }
    }
    find_tsid_at(&decoded, 0)
}

fn find_tsid_at(value: &str, start: usize) -> Option<String> {
    let bytes = value.as_bytes();
    if start > bytes.len() {
        return None;
    }
    for offset in start..bytes.len().saturating_sub(TSID_LENGTH - 1) {
        if bytes[offset..offset + TSID_LENGTH]
            .iter()
            .all(u8::is_ascii_alphanumeric)
        {
            return Some(value[offset..offset + TSID_LENGTH].to_ascii_uppercase());
        }
    }
    None
}

fn is_tsid(value: &str) -> bool {
    value.len() == TSID_LENGTH && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn build_target(input: &str, field: &str) -> Result<TrustedShopsTarget, String> {
    let tsid = extract_tsid(input).ok_or_else(|| {
        format!("{field} must contain a 33-character Trusted Shops TSID, for example XFB15FFBDE1DEE7A55D292A7D48598A6A")
    })?;
    let source_url = normalize_source_url(input, &tsid);
    Ok(TrustedShopsTarget {
        tsid,
        input: input.to_owned(),
        source_url,
    })
}

fn normalize_source_url(input: &str, tsid: &str) -> String {
    let trimmed = input.trim();
    let lowered = trimmed.to_ascii_lowercase();
    if lowered.starts_with("http://") || lowered.starts_with("https://") {
        return trimmed.to_owned();
    }
    if lowered.contains("trustedshops.") {
        return format!("https://{}", trimmed.trim_start_matches('/'));
    }
    format!("https://www.trustedshops.de/bewertung/info_{tsid}.html")
}

fn clean_string_array(value: Option<&Value>, field: &str) -> Result<Vec<String>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(Vec::new());
    }
    if let Value::Array(values) = value {
        let mut result = Vec::new();
        for value in values {
            if let Some(value) = clean_optional_string(Some(value), field, 500)? {
                result.push(value);
            }
        }
        return Ok(result);
    }
    Ok(clean_optional_string(Some(value), field, 500)?.into_iter().collect())
}

fn clean_optional_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>, String> {
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

fn clean_integer(value: Option<&Value>, field: &str, min: u64, max: u64) -> Result<Option<u64>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    let integer = match value {
        Value::Number(number) => {
            let Some(number) = number.as_f64().filter(|number| number.is_finite() && number.fract() == 0.0) else {
                return Err(format!("{field} must be an integer"));
            };
            number
        }
        Value::String(value)
            if !value.trim().is_empty() && value.trim().bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            value.trim().parse::<f64>().map_err(|_| format!("{field} must be an integer"))?
        }
        _ => return Err(format!("{field} must be an integer")),
    };
    if integer < min as f64 || integer > max as f64 {
        return Err(format!("{field} must be between {min} and {max}"));
    }
    Ok(Some(integer as u64))
}

fn clean_boolean(value: Option<&Value>, field: &str) -> Result<Option<bool>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if value.is_empty() => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        _ => Err(format!("{field} must be a boolean")),
    }
}

fn clean_market(value: Option<&Value>) -> Result<Option<String>, String> {
    let Some(market) = clean_optional_string(value, "market", 3)? else {
        return Ok(None);
    };
    let normalized = market.to_ascii_uppercase();
    if !VALID_MARKETS.contains(&normalized.as_str()) {
        return Err(format!("market must be one of: {}", VALID_MARKETS.join(", ")));
    }
    Ok(Some(normalized))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TSID: &str = "XFB15FFBDE1DEE7A55D292A7D48598A6A";

    #[test]
    fn builds_batch_targets_and_page_parameters() {
        let plan = build_request_plan(&json!({
            "tsids": [TSID],
            "urls": [format!("https://www.trustedshops.de/bewertung/info_{TSID}.html?utm=test"), TSID.to_ascii_lowercase()],
            "page": 2,
            "max_pages": 3,
            "size": 50,
            "market": "deu",
            "include_raw_responses": true
        })).unwrap();
        assert_eq!(plan.targets, [TrustedShopsTarget {
            tsid: TSID.into(),
            input: TSID.into(),
            source_url: format!("https://www.trustedshops.de/bewertung/info_{TSID}.html"),
        }]);
        assert_eq!(plan.base_params["size"], json!(50));
        assert_eq!(plan.base_params["market"], json!("DEU"));
        assert_eq!(plan.start_page, 2);
        assert_eq!(plan.max_pages, 3);
        assert!(plan.include_raw_responses);
        assert_eq!(page_params(&plan, 2)["page"], json!(2));
        assert_eq!(describe_request(&plan), "1 Trusted Shops target(s) (pages 2-4)");
    }

    #[test]
    fn extracts_tsids_from_direct_values_and_urls() {
        assert_eq!(extract_tsid(&TSID.to_ascii_lowercase()).as_deref(), Some(TSID));
        assert_eq!(extract_tsid(&format!("www.trustedshops.de/bewertung/info_{TSID}.html")).as_deref(), Some(TSID));
        assert_eq!(extract_tsid(&format!("https://example.com/api/trustedshops/reviews/{TSID}")).as_deref(), Some(TSID));
        assert_eq!(extract_tsid(&format!("https://example.com/%E0%A4%A/info_{TSID}.html")).as_deref(), Some(TSID));
    }

    #[test]
    fn supports_single_input_compatibility_and_defaults() {
        let plan = build_request_plan(&json!({ "url": format!("https://www.trustedshops.eu/buyerrating/info_{TSID}.html") })).unwrap();
        assert_eq!(plan.targets[0].tsid, TSID);
        assert_eq!(plan.targets[0].source_url, format!("https://www.trustedshops.eu/buyerrating/info_{TSID}.html"));
        assert_eq!(plan.base_params["size"], json!(20));
        assert_eq!(plan.start_page, 1);
        assert_eq!(plan.max_pages, 1);
        assert!(!plan.include_raw_responses);
    }

    #[test]
    fn reports_the_same_validation_errors() {
        assert_eq!(build_request_plan(&json!({})).unwrap_err(), "Provide at least one Trusted Shops TSID or profile URL in tsids or urls");
        assert!(build_request_plan(&json!({ "tsids": ["not-a-tsid"] })).unwrap_err().contains("must contain a 33-character Trusted Shops TSID"));
        assert_eq!(build_request_plan(&json!({ "tsids": [TSID], "max_pages": 26 })).unwrap_err(), "max_pages must be between 1 and 25");
        assert_eq!(build_request_plan(&json!({ "tsids": [TSID], "page": 99, "max_pages": 3 })).unwrap_err(), "page plus max_pages cannot exceed page 100");
        assert!(build_request_plan(&json!({ "tsids": [TSID], "market": "USA" })).unwrap_err().starts_with("market must be one of:"));
        assert_eq!(build_request_plan(&json!({ "tsids": [TSID], "include_raw_responses": "true" })).unwrap_err(), "include_raw_responses must be a boolean");
    }

    #[test]
    fn validates_batch_limit_and_string_inputs() {
        let tsids = (0..51).map(|index| json!(format!("{index:033X}"))).collect::<Vec<_>>();
        assert_eq!(build_request_plan(&json!({ "tsids": tsids })).unwrap_err(), "Provide 50 or fewer Trusted Shops targets per run");
        assert_eq!(build_request_plan(&json!({ "tsids": 42 })).unwrap_err(), "tsids must be a string");
    }
}
