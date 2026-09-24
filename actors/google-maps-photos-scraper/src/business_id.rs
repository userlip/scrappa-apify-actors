use anyhow::{bail, Result};
use serde_json::Value;
use std::collections::HashSet;
use url::Url;

pub const MAX_BUSINESS_IDS_PER_RUN: usize = 10;

#[derive(Debug, PartialEq, Eq)]
pub struct BusinessIdRequest {
    pub input_business_id: String,
    pub business_id: Option<String>,
    pub source: Option<&'static str>,
    pub validation_error: Option<String>,
}

pub fn get_business_id_requests(input: Option<&Value>) -> Result<Vec<BusinessIdRequest>> {
    let mut raw_business_ids = Vec::new();
    if let Some(business_id) = input
        .and_then(|input| input.get("business_id"))
        .and_then(Value::as_str)
    {
        raw_business_ids.push(business_id.to_owned());
    }
    if let Some(business_ids) = input
        .and_then(|input| input.get("business_ids"))
        .and_then(Value::as_array)
    {
        raw_business_ids.extend(
            business_ids
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned),
        );
    }

    let mut seen = HashSet::new();
    let mut requests = Vec::new();

    for raw_business_id in raw_business_ids {
        let input_business_id = raw_business_id.trim().to_owned();
        if input_business_id.is_empty() {
            continue;
        }

        match normalize_business_id(&input_business_id) {
            Ok((business_id, source)) => {
                if seen.insert(business_id.clone()) {
                    requests.push(BusinessIdRequest {
                        input_business_id,
                        business_id: Some(business_id),
                        source: Some(source),
                        validation_error: None,
                    });
                }
            }
            Err(error) => {
                let key = format!("invalid:{input_business_id}");
                if seen.insert(key) {
                    requests.push(BusinessIdRequest {
                        input_business_id,
                        business_id: None,
                        source: None,
                        validation_error: Some(error.to_string()),
                    });
                }
            }
        }
    }

    if requests.len() > MAX_BUSINESS_IDS_PER_RUN {
        bail!(
            "business_ids must contain {} unique items or fewer",
            MAX_BUSINESS_IDS_PER_RUN
        );
    }

    Ok(requests)
}

fn normalize_business_id(value: &str) -> Result<(String, &'static str)> {
    let value = value.trim();
    if value.is_empty() {
        bail!("Business ID is required");
    }

    let decoded = decode_repeatedly(value);
    if let Some(business_id) = find_google_business_id(&decoded) {
        let source = if value == business_id {
            "business_id"
        } else {
            "url"
        };
        return Ok((business_id, source));
    }
    if let Some(place_id) = find_place_id(&decoded) {
        let source = if value == place_id { "place_id" } else { "url" };
        return Ok((place_id, source));
    }
    if looks_like_google_maps_url(&decoded) {
        bail!(
            "Google Maps URL must contain an extractable 0x...:0x... business ID or ChIJ... place ID. Use a business_id from a Google Maps Search or Business Details actor run."
        );
    }

    Ok((value.to_owned(), "business_id"))
}

fn decode_repeatedly(value: &str) -> String {
    let mut decoded = value.to_owned();
    for _ in 0..3 {
        let Some(next) = decode_uri_component(&decoded) else {
            return decoded;
        };
        if next == decoded {
            return next;
        }
        decoded = next;
    }
    decoded
}

fn decode_uri_component(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            decoded.push(bytes[index]);
            index += 1;
            continue;
        }

        let high = bytes.get(index + 1).and_then(|byte| hex_value(*byte))?;
        let low = bytes.get(index + 2).and_then(|byte| hex_value(*byte))?;
        decoded.push((high << 4) | low);
        index += 3;
    }
    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn find_google_business_id(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    for start in 0..bytes.len().saturating_sub(1) {
        if !bytes.get(start..start + 2)?.eq_ignore_ascii_case(b"0x") {
            continue;
        }
        let mut index = start + 2;
        let first_id_start = index;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_hexdigit())
        {
            index += 1;
        }
        if index == first_id_start || bytes.get(index) != Some(&b':') {
            continue;
        }
        index += 1;
        if !bytes.get(index..index + 2)?.eq_ignore_ascii_case(b"0x") {
            continue;
        }
        index += 2;
        let second_id_start = index;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_hexdigit())
        {
            index += 1;
        }
        if index > second_id_start {
            return value.get(start..index).map(str::to_owned);
        }
    }
    None
}

fn find_place_id(value: &str) -> Option<String> {
    for (start, _) in value.match_indices("ChIJ") {
        let suffix_start = start + 4;
        let suffix_end = value[suffix_start..]
            .bytes()
            .position(|byte| !(byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'))
            .map_or(value.len(), |length| suffix_start + length);
        if suffix_end > suffix_start {
            return value.get(start..suffix_end).map(str::to_owned);
        }
    }
    None
}

fn looks_like_google_maps_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    if !matches!(url.scheme(), "http" | "https") || url.port().is_some() {
        return false;
    }

    let host = url.host_str().unwrap_or_default();
    let path = url.path();
    let google_host = host
        .strip_prefix("www.")
        .or_else(|| host.strip_prefix("maps."))
        .unwrap_or(host);
    let google_domain_matches = {
        let labels = google_host.split('.').collect::<Vec<_>>();
        labels.len() >= 2
            && labels.len() <= 3
            && labels[0] == "google"
            && (2..=3).contains(&labels[1].len())
            && labels[1].bytes().all(|byte| byte.is_ascii_alphabetic())
            && (labels.len() == 2
                || (labels[2].len() == 2
                    && labels[2].bytes().all(|byte| byte.is_ascii_alphabetic())))
    };
    let google_maps_path = path.eq_ignore_ascii_case("/maps")
        || path
            .get(..6)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("/maps/"));

    (google_domain_matches && google_maps_path)
        || (host == "maps.app.goo.gl" && path.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_raw_ids_place_ids_and_supported_urls() {
        let requests = get_business_id_requests(Some(&json!({
            "business_ids": [
                "0x808fba02425dad8f:0x6c296c66619367e0",
                "ChIJj61dQgK6j4AR4GeTYWZsKWw",
                "https://www.google.com/maps/place/example/data=!4m2!3m1!1s0x123:0x456"
            ]
        })))
        .unwrap();

        assert_eq!(
            requests[0].business_id.as_deref(),
            Some("0x808fba02425dad8f:0x6c296c66619367e0")
        );
        assert_eq!(requests[0].source, Some("business_id"));
        assert_eq!(
            requests[1].business_id.as_deref(),
            Some("ChIJj61dQgK6j4AR4GeTYWZsKWw")
        );
        assert_eq!(requests[1].source, Some("place_id"));
        assert_eq!(requests[2].business_id.as_deref(), Some("0x123:0x456"));
        assert_eq!(requests[2].source, Some("url"));
    }

    #[test]
    fn extracts_google_business_id_after_a_unicode_maps_path_segment() {
        let requests = get_business_id_requests(Some(&json!({
            "business_id": "https://www.google.com/maps/place/Café/data=!4m2!3m1!1s0x123:0x456"
        })))
        .unwrap();

        assert_eq!(requests[0].business_id.as_deref(), Some("0x123:0x456"));
        assert_eq!(requests[0].source, Some("url"));
    }

    #[test]
    fn decodes_nested_url_encoding_and_deduplicates_normalized_ids() {
        let requests = get_business_id_requests(Some(&json!({
            "business_id": "0xabc:0xdef",
            "business_ids": ["https%253A%252F%252Fmaps.google.com%252Fmaps%253Fq%253D0xabc%253A0xdef", "  "]
        })))
        .unwrap();

        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].business_id.as_deref(), Some("0xabc:0xdef"));
    }

    #[test]
    fn retains_unextractable_maps_urls_as_validation_results() {
        let requests = get_business_id_requests(Some(&json!({
            "business_id": "https://maps.app.goo.gl/short-link"
        })))
        .unwrap();

        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].business_id, None);
        assert_eq!(
            requests[0].validation_error.as_deref(),
            Some("Google Maps URL must contain an extractable 0x...:0x... business ID or ChIJ... place ID. Use a business_id from a Google Maps Search or Business Details actor run.")
        );
    }

    #[test]
    fn legacy_input_is_first_and_unique_limit_includes_invalid_ids() {
        let requests = get_business_id_requests(Some(&json!({
            "business_id": "first",
            "business_ids": ["first", "second", "third"]
        })))
        .unwrap();
        assert_eq!(
            requests
                .iter()
                .map(|request| request.input_business_id.as_str())
                .collect::<Vec<_>>(),
            ["first", "second", "third"]
        );

        let too_many =
            json!({"business_ids": ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"]});
        assert_eq!(
            get_business_id_requests(Some(&too_many))
                .unwrap_err()
                .to_string(),
            "business_ids must contain 10 unique items or fewer"
        );
    }

    #[test]
    fn keeps_existing_prefill_values_in_actor_input_schema() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema.pointer("/properties/business_ids/type"),
            Some(&json!("array"))
        );
        assert_eq!(
            schema.pointer("/properties/business_ids/minItems"),
            Some(&json!(1))
        );
        assert_eq!(
            schema.pointer("/properties/business_ids/maxItems"),
            Some(&json!(10))
        );
        assert_eq!(
            schema.pointer("/properties/business_id/type"),
            Some(&json!("string"))
        );
        assert_eq!(
            schema.pointer("/properties/business_ids/prefill"),
            Some(&json!([
                "0x808fba02425dad8f:0x6c296c66619367e0",
                "ChIJj61dQgK6j4AR4GeTYWZsKWw"
            ]))
        );
        assert_eq!(
            schema.pointer("/properties/business_id/prefill"),
            Some(&json!("0x808fba02425dad8f:0x6c296c66619367e0"))
        );
        assert_eq!(
            schema.pointer("/properties/use_cache/default"),
            Some(&json!(true))
        );
        assert_eq!(
            schema.pointer("/properties/maximum_cache_age/default"),
            Some(&json!(3600))
        );
        assert_eq!(
            schema.pointer("/properties/maximum_cache_age/minimum"),
            Some(&json!(0))
        );
        assert_eq!(
            schema["anyOf"],
            json!([
                { "required": ["business_ids"] },
                { "required": ["business_id"] }
            ])
        );
    }
}
