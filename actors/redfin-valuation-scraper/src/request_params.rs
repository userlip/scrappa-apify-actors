use regex::Regex;
use serde_json::{Number, Value};
use std::sync::OnceLock;

pub const MAX_VALUATIONS_PER_RUN: usize = 50;

#[derive(Clone, Debug, PartialEq)]
pub struct RedfinValuationRequest {
    pub property_id: Number,
    pub listing_id: Option<Number>,
    pub url: Option<String>,
    pub index: usize,
}

fn home_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"(?i)/home/(\d+)(?:[/?#]|$)").unwrap())
}

fn listing_query_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(?i)[?&](?:listing_id|listingId|listingIdOverride|listing)=([0-9]+)").unwrap()
    })
}

fn listing_path_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"(?i)/listing/(\d+)(?:[/?#]|$)").unwrap())
}

fn input_error(message: impl Into<String>) -> String {
    message.into()
}

fn clean_string(
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
        return Err(input_error(format!("{field} must be a string")));
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.encode_utf16().count() > max_length {
        return Err(input_error(format!(
            "{field} must be {max_length} characters or fewer"
        )));
    }
    Ok(Some(trimmed.to_owned()))
}

fn positive_integer(value: f64) -> Result<Number, String> {
    if !value.is_finite() || value.fract() != 0.0 {
        return Err("must be an integer".to_owned());
    }
    if value <= 0.0 {
        return Err("must be greater than 0".to_owned());
    }
    if value <= u64::MAX as f64 {
        let integer = value as u64;
        if integer as f64 == value {
            return Ok(Number::from(integer));
        }
    }
    Number::from_f64(value).ok_or_else(|| "must be an integer".to_owned())
}

fn clean_integer(value: Option<&Value>, field: &str) -> Result<Option<Number>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }

    let normalized = if let Some(value) = value.as_str() {
        let trimmed = value.trim();
        if !trimmed.is_empty() && trimmed.chars().all(|character| character.is_ascii_digit()) {
            Some(
                trimmed
                    .parse::<f64>()
                    .map_err(|_| format!("{field} must be an integer"))?,
            )
        } else {
            None
        }
    } else if let Some(value) = value.as_f64() {
        Some(value)
    } else {
        None
    };

    let Some(normalized) = normalized else {
        return Err(format!("{field} must be an integer"));
    };
    positive_integer(normalized)
        .map(Some)
        .map_err(|message| format!("{field} {message}"))
}

fn clean_required_integer(value: Option<&Value>, field: &str) -> Result<Number, String> {
    clean_integer(value, field)?.ok_or_else(|| format!("{field} is required"))
}

fn extract_id(pattern: &Regex, value: &str) -> Option<Number> {
    pattern
        .captures(value)
        .and_then(|captures| captures.get(1))
        .and_then(|capture| capture.as_str().parse::<u64>().ok())
        .map(Number::from)
}

pub fn extract_redfin_ids_from_url(
    value: Option<&Value>,
) -> Result<(Option<Number>, Option<Number>, Option<String>), String> {
    let Some(url) = clean_string(value, "url", 2000)? else {
        return Ok((None, None, None));
    };
    let property_id = extract_id(home_pattern(), &url);
    let listing_id = extract_id(listing_query_pattern(), &url)
        .or_else(|| extract_id(listing_path_pattern(), &url));
    Ok((property_id, listing_id, Some(url)))
}

fn build_single_request(
    input: &Value,
    index: usize,
    prefix: &str,
) -> Result<RedfinValuationRequest, String> {
    let object = input
        .as_object()
        .ok_or_else(|| format!("{prefix} must be an object"))?;
    let (url_property_id, url_listing_id, url) = extract_redfin_ids_from_url(object.get("url"))?;

    let property_id = match object.get("property_id") {
        Some(value) if !value.is_null() => {
            clean_required_integer(Some(value), &format!("{prefix}property_id"))?
        }
        _ => {
            let extracted = url_property_id.map(Value::Number);
            clean_required_integer(extracted.as_ref(), &format!("{prefix}property_id"))?
        }
    };
    let listing_id = match object.get("listing_id") {
        Some(value) if !value.is_null() => {
            clean_integer(Some(value), &format!("{prefix}listing_id"))?
        }
        _ => {
            let extracted = url_listing_id.map(Value::Number);
            clean_integer(extracted.as_ref(), &format!("{prefix}listing_id"))?
        }
    };

    Ok(RedfinValuationRequest {
        property_id,
        listing_id,
        url,
        index,
    })
}

fn assert_batch_size(length: usize, field: &str) -> Result<(), String> {
    if length == 0 {
        return Err(format!("{field} must include at least one property"));
    }
    if length > MAX_VALUATIONS_PER_RUN {
        return Err(format!(
            "{field} cannot include more than {MAX_VALUATIONS_PER_RUN} properties per run"
        ));
    }
    Ok(())
}

pub fn build_redfin_valuation_requests(
    input: &Value,
) -> Result<Vec<RedfinValuationRequest>, String> {
    let object = input
        .as_object()
        .ok_or_else(|| "Input must be an object".to_owned())?;

    if let Some(properties) = object.get("properties") {
        let Some(properties) = properties.as_array() else {
            return Err("properties must be an array of property objects".to_owned());
        };
        assert_batch_size(properties.len(), "properties")?;
        return properties
            .iter()
            .enumerate()
            .map(|(index, property)| {
                if !property.is_object() {
                    return Err(format!("properties[{index}] must be an object"));
                }
                build_single_request(property, index, &format!("properties[{index}]."))
            })
            .collect();
    }

    if let Some(property_ids) = object.get("property_ids") {
        let Some(property_ids) = property_ids.as_array() else {
            return Err("property_ids must be an array of property IDs".to_owned());
        };
        assert_batch_size(property_ids.len(), "property_ids")?;
        return property_ids
            .iter()
            .enumerate()
            .map(|(index, property_id)| {
                let item = serde_json::json!({"property_id": property_id});
                build_single_request(&item, index, &format!("property_ids[{index}]."))
            })
            .collect();
    }

    build_single_request(input, 0, "").map(|request| vec![request])
}

pub fn describe_redfin_valuation_request(request: &RedfinValuationRequest) -> String {
    match &request.listing_id {
        Some(listing_id) => format!("property {} with listing {listing_id}", request.property_id),
        None => format!("property {}", request.property_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_single_and_batch_requests_with_original_precedence() {
        let single = build_redfin_valuation_requests(&serde_json::json!({
            "property_id": "194191988", "listing_id": "207388793"
        }))
        .unwrap();
        assert_eq!(single[0].property_id, Number::from(194191988_u64));
        assert_eq!(single[0].listing_id, Some(Number::from(207388793_u64)));

        let batch = build_redfin_valuation_requests(&serde_json::json!({
            "property_ids": ["194191988", 123456789]
        }))
        .unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[1].property_id, Number::from(123456789_u64));

        let objects = build_redfin_valuation_requests(&serde_json::json!({
            "properties": [
                {"property_id": 194191988},
                {"property_id": "123456789", "listing_id": "207388793"},
                {"url": "https://www.redfin.com/WA/Seattle/example/home/987654321?listing_id=222333444"}
            ],
            "property_ids": []
        })).unwrap();
        assert_eq!(objects.len(), 3);
        assert_eq!(objects[2].property_id, Number::from(987654321_u64));
        assert_eq!(objects[2].listing_id, Some(Number::from(222333444_u64)));
        assert_eq!(
            objects[2].url.as_deref(),
            Some("https://www.redfin.com/WA/Seattle/example/home/987654321?listing_id=222333444")
        );
    }

    #[test]
    fn extracts_listing_id_from_query_or_listing_path() {
        let url = serde_json::json!("https://www.redfin.com/home/194191988?listingIdOverride=12");
        let (property_id, listing_id, _) = extract_redfin_ids_from_url(Some(&url)).unwrap();
        assert_eq!(property_id, Some(Number::from(194191988_u64)));
        assert_eq!(listing_id, Some(Number::from(12_u64)));

        let url = serde_json::json!("https://www.redfin.com/listing/333");
        assert_eq!(
            extract_redfin_ids_from_url(Some(&url)).unwrap().1,
            Some(Number::from(333_u64))
        );
    }

    #[test]
    fn validates_required_values_and_batch_sizes() {
        assert_eq!(
            build_redfin_valuation_requests(&serde_json::json!({})).unwrap_err(),
            "property_id is required"
        );
        assert_eq!(
            build_redfin_valuation_requests(&serde_json::json!({"property_id":"abc"})).unwrap_err(),
            "property_id must be an integer"
        );
        assert_eq!(
            build_redfin_valuation_requests(&serde_json::json!({"property_id":0})).unwrap_err(),
            "property_id must be greater than 0"
        );
        assert_eq!(
            build_redfin_valuation_requests(&serde_json::json!({"property_ids":[]})).unwrap_err(),
            "property_ids must include at least one property"
        );
        let ids = vec![serde_json::json!(1); MAX_VALUATIONS_PER_RUN + 1];
        assert_eq!(
            build_redfin_valuation_requests(&serde_json::json!({"property_ids":ids})).unwrap_err(),
            "property_ids cannot include more than 50 properties per run"
        );
        assert_eq!(
            build_redfin_valuation_requests(&serde_json::json!({"properties":["194191988"]}))
                .unwrap_err(),
            "properties[0] must be an object"
        );
        assert_eq!(
            build_redfin_valuation_requests(&serde_json::json!({"properties":null})).unwrap_err(),
            "properties must be an array of property objects"
        );
        assert_eq!(
            build_redfin_valuation_requests(
                &serde_json::json!({"url":"https://redfin.test/home/0"})
            )
            .unwrap_err(),
            "property_id must be greater than 0"
        );
        assert_eq!(
            build_redfin_valuation_requests(
                &serde_json::json!({"url":"https://redfin.test/home/1?listing_id=0"})
            )
            .unwrap_err(),
            "listing_id must be greater than 0"
        );
    }

    #[test]
    fn enforces_url_type_and_length() {
        assert_eq!(
            build_redfin_valuation_requests(&serde_json::json!({"url":12})).unwrap_err(),
            "url must be a string"
        );
        let url = "x".repeat(2001);
        assert_eq!(
            build_redfin_valuation_requests(&serde_json::json!({"property_id":1,"url":url}))
                .unwrap_err(),
            "url must be 2000 characters or fewer"
        );
    }

    #[test]
    fn describes_requests_with_optional_listing_ids() {
        let request =
            build_redfin_valuation_requests(&serde_json::json!({"property_id":194191988}))
                .unwrap()
                .remove(0);
        assert_eq!(
            describe_redfin_valuation_request(&request),
            "property 194191988"
        );
    }

    #[test]
    fn input_schema_keeps_prefill_and_batch_fields() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["properties"]["property_id"]["default"], 194191988);
        assert_eq!(schema["properties"]["property_id"]["minimum"], 1);
        assert_eq!(
            schema["properties"]["property_ids"]["maxItems"],
            MAX_VALUATIONS_PER_RUN
        );
        assert_eq!(
            schema["properties"]["properties"]["maxItems"],
            MAX_VALUATIONS_PER_RUN
        );
        assert!(schema["properties"]["url"].get("prefill").is_none());
        assert!(schema["properties"]["property_ids"]
            .get("prefill")
            .is_none());
        assert!(schema["properties"]["properties"].get("prefill").is_none());
    }
}
