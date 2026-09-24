use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use url::Url;

const INVALID_AD_URL: &str = "A valid TikTok Creative Center ad URL is required";
const CREATIVE_CENTER_HOST: &str = "A TikTok Creative Center ad URL on ads.tiktok.com is required";
const CREATIVE_CENTER_PATH: &str = "TikTok Creative Center ad URLs must use the format https://ads.tiktok.com/business/creativecenter/topads/{ad_id}/pc/en";

#[derive(Debug, PartialEq, Eq)]
pub struct TikTokAdLookup {
    pub url: String,
    pub validation_error: Option<String>,
}

pub fn js_trim(value: &str) -> &str {
    value.trim_matches(is_js_whitespace)
}

fn is_js_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'..='\u{000d}'
            | '\u{0020}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

pub fn require_tiktok_ad_url(value: &str) -> Result<()> {
    let lookup = js_trim(value);
    if lookup.is_empty() {
        bail!("TikTok Creative Center ad URL is required");
    }

    let parsed = Url::parse(lookup).map_err(|_| anyhow!("{INVALID_AD_URL}"))?;
    if !parsed
        .host_str()
        .is_some_and(|host| host.eq_ignore_ascii_case("ads.tiktok.com"))
    {
        bail!("{CREATIVE_CENTER_HOST}");
    }
    if parsed.scheme() != "https" {
        bail!("TikTok Creative Center ad URLs must use HTTPS");
    }
    if !is_creative_center_ad_path(parsed.path()) {
        bail!("{CREATIVE_CENTER_PATH}");
    }

    Ok(())
}

fn is_creative_center_ad_path(path: &str) -> bool {
    let Some(path) = path.strip_prefix('/') else {
        return false;
    };
    let mut segments = path.split('/').collect::<Vec<_>>();
    if segments.last() == Some(&"") {
        segments.pop();
    }
    if !(segments.len() == 4 || segments.len() == 6) {
        return false;
    }
    if !segments[0].eq_ignore_ascii_case("business")
        || !segments[1].eq_ignore_ascii_case("creativecenter")
        || !segments[2].eq_ignore_ascii_case("topads")
        || segments[3].is_empty()
        || !segments[3].bytes().all(|byte| byte.is_ascii_digit())
    {
        return false;
    }
    segments.len() == 4
        || (segments[4].eq_ignore_ascii_case("pc") && segments[5].eq_ignore_ascii_case("en"))
}

pub fn normalize_tiktok_ad_url(value: &str) -> Result<String> {
    require_tiktok_ad_url(value)?;
    let mut parsed = Url::parse(js_trim(value)).map_err(|_| anyhow!("{INVALID_AD_URL}"))?;
    parsed.set_query(None);
    parsed.set_fragment(None);
    Ok(parsed.to_string())
}

pub fn extract_tiktok_ad_id(value: &str) -> Option<String> {
    let parsed = Url::parse(js_trim(value)).ok()?;
    let path = parsed.path().strip_prefix('/')?;
    let mut segments = path.split('/').collect::<Vec<_>>();
    if segments.last() == Some(&"") {
        segments.pop();
    }
    if !(segments.len() == 4 || segments.len() == 6)
        || !segments[0].eq_ignore_ascii_case("business")
        || !segments[1].eq_ignore_ascii_case("creativecenter")
        || !segments[2].eq_ignore_ascii_case("topads")
        || segments[3].is_empty()
        || !segments[3].bytes().all(|byte| byte.is_ascii_digit())
        || (segments.len() == 6
            && (!segments[4].eq_ignore_ascii_case("pc") || !segments[5].eq_ignore_ascii_case("en")))
    {
        return None;
    }
    Some(segments[3].to_owned())
}

pub fn format_tiktok_ad_lookup_for_log(value: &str) -> Result<String> {
    let normalized = normalize_tiktok_ad_url(value)?;
    Ok(match extract_tiktok_ad_id(&normalized) {
        Some(ad_id) => format!("ad_id:{ad_id}"),
        None => normalized,
    })
}

pub fn safe_format_tiktok_ad_lookup_for_log(value: &str) -> String {
    format_tiktok_ad_lookup_for_log(value).unwrap_or_else(|_| js_trim(value).to_owned())
}

pub fn resolve_tiktok_ad_requests(input: &Value) -> Result<Vec<TikTokAdLookup>> {
    let mut lookups = Vec::new();

    match input.get("urls") {
        Some(Value::Array(values)) => {
            for (index, value) in values.iter().enumerate() {
                let Some(value) = value.as_str() else {
                    eprintln!(
                        "Warning: urls[{index}] must be a string, got {}. Skipping.",
                        js_type(value)
                    );
                    continue;
                };
                let lookup = js_trim(value);
                if lookup.is_empty() {
                    eprintln!("Warning: urls[{index}] is empty. Skipping.");
                    continue;
                }
                match normalize_tiktok_ad_url(lookup) {
                    Ok(url) => lookups.push(TikTokAdLookup {
                        url,
                        validation_error: None,
                    }),
                    Err(error) => {
                        let message = error.to_string();
                        eprintln!("Warning: urls[{index}] is invalid: {message}");
                        lookups.push(TikTokAdLookup {
                            url: lookup.to_owned(),
                            validation_error: Some(message),
                        });
                    }
                }
            }
        }
        Some(Value::Null) | None => {}
        Some(value) => eprintln!(
            "Warning: urls must be an array of strings, got {}. Falling back to url.",
            js_type(value)
        ),
    }

    if lookups.is_empty() {
        match input.get("url") {
            Some(Value::String(value)) if !js_trim(value).is_empty() => {
                lookups.push(TikTokAdLookup {
                    url: normalize_tiktok_ad_url(js_trim(value))?,
                    validation_error: None,
                });
            }
            Some(Value::String(_)) | Some(Value::Null) | None => {}
            Some(value) => eprintln!("Warning: url must be a string, got {}.", js_type(value)),
        }
    }

    if lookups.is_empty() {
        bail!("At least one TikTok Creative Center ad URL is required");
    }
    Ok(lookups)
}

fn js_type(value: &Value) -> &'static str {
    match value {
        Value::Null | Value::Array(_) | Value::Object(_) => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const URL: &str =
        "https://ads.tiktok.com/business/creativecenter/topads/7543186103350427655/pc/en";

    #[test]
    fn strips_query_and_fragment_but_keeps_duplicate_requests() {
        let requests = resolve_tiktok_ad_requests(&json!({
            "urls": [format!("{URL}?period=30&region=US#details"), URL]
        }))
        .unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].url, URL);
        assert_eq!(requests[1].url, URL);
    }

    #[test]
    fn supports_legacy_input_and_prefers_batch_urls() {
        let legacy =
            resolve_tiktok_ad_requests(&json!({ "url": format!("{URL}?period=30") })).unwrap();
        assert_eq!(legacy[0].url, URL);

        let second =
            "https://ads.tiktok.com/business/creativecenter/topads/1234567890123456789/pc/en";
        let batch = resolve_tiktok_ad_requests(&json!({ "urls": [second], "url": URL })).unwrap();
        assert_eq!(batch[0].url, second);
    }

    #[test]
    fn skips_non_string_and_empty_batch_entries() {
        let requests = resolve_tiktok_ad_requests(&json!({
            "urls": [URL, 123, " "]
        }))
        .unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, URL);
    }

    #[test]
    fn keeps_invalid_string_batch_entries_as_failed_requests() {
        let requests = resolve_tiktok_ad_requests(&json!({
            "urls": ["not-a-url", URL],
            "url": "https://ads.tiktok.com/business/creativecenter/topads/1234567890123456789"
        }))
        .unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].url, "not-a-url");
        assert_eq!(
            requests[0].validation_error.as_deref(),
            Some(INVALID_AD_URL)
        );
        assert_eq!(requests[1].url, URL);
    }

    #[test]
    fn falls_back_to_legacy_url_when_batch_has_no_usable_values() {
        let requests = resolve_tiktok_ad_requests(&json!({
            "urls": [null, " "],
            "url": URL
        }))
        .unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, URL);
    }

    #[test]
    fn validates_hosts_schemes_and_creative_center_paths() {
        assert!(require_tiktok_ad_url(URL).is_ok());
        assert!(require_tiktok_ad_url(
            "https://ads.tiktok.com/business/creativecenter/topads/7221117041168252930/"
        )
        .is_ok());
        assert!(require_tiktok_ad_url(
            "HTTPS://ads.tiktok.com/business/creativecenter/topads/7543186103350427655/PC/EN"
        )
        .is_ok());
        assert!(require_tiktok_ad_url("https://www.tiktok.com/@tiktok/video/1").is_err());
        assert!(require_tiktok_ad_url(
            "http://ads.tiktok.com/business/creativecenter/topads/1/pc/en"
        )
        .unwrap_err()
        .to_string()
        .contains("must use HTTPS"));
        assert!(require_tiktok_ad_url(
            "https://ads.tiktok.com/business/creativecenter/pc/en/topads/1/"
        )
        .unwrap_err()
        .to_string()
        .contains("topads/{ad_id}/pc/en"));
    }

    #[test]
    fn trims_javascript_whitespace_around_input_values() {
        let wrapped = format!("\u{feff}{URL}\u{feff}");
        assert_eq!(normalize_tiktok_ad_url(&wrapped).unwrap(), URL);
        assert!(require_tiktok_ad_url("\u{feff} ").is_err());
    }

    #[test]
    fn extracts_ids_and_formats_log_lookups() {
        assert_eq!(
            extract_tiktok_ad_id(URL).as_deref(),
            Some("7543186103350427655")
        );
        assert_eq!(
            format_tiktok_ad_lookup_for_log(URL).unwrap(),
            "ad_id:7543186103350427655"
        );
        assert_eq!(extract_tiktok_ad_id("https://example.com/topads/1"), None);
    }

    #[test]
    fn input_schema_keeps_prefill_and_batch_and_legacy_patterns() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["urls"]["prefill"][0],
            "https://ads.tiktok.com/business/creativecenter/topads/7213160569871581185/pc/en?countryCode=US&period=30"
        );
        assert_eq!(schema["properties"]["urls"]["maxItems"], 100);
        assert_eq!(
            schema["properties"]["url"]["title"],
            "Single TikTok Creative Center Ad URL"
        );
    }
}
