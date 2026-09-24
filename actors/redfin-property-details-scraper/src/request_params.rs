use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use url::Url;

const MAX_PROPERTIES_PER_RUN: usize = 100;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RedfinPropertyDetailsRequest {
    pub(crate) params: RedfinPropertyDetailsParams,
    pub(crate) index: usize,
    pub(crate) input: Value,
    pub(crate) source: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RedfinPropertyDetailsParams {
    pub(crate) property_id: u64,
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }

    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(trimmed.to_owned()))
}

fn clean_property_id(value: &Value, field: &str) -> Result<u64> {
    let property_id = match value {
        Value::String(value)
            if value
                .trim()
                .chars()
                .all(|character| character.is_ascii_digit()) =>
        {
            value.trim().parse::<f64>().ok()
        }
        Value::Number(value) => {
            let number = value.as_f64().unwrap_or(f64::NAN);
            if number.is_finite() && number.fract() == 0.0 {
                Some(number)
            } else {
                None
            }
        }
        _ => None,
    }
    .ok_or_else(|| anyhow!("{field} must be an integer"))?;

    if property_id <= 0.0 {
        bail!("{field} must be greater than 0");
    }
    if property_id > MAX_SAFE_INTEGER as f64 {
        bail!("{field} is too large");
    }
    Ok(property_id as u64)
}

fn extract_redfin_property_id_from_url(value: &Value, field: &str) -> Result<u64> {
    let url_string =
        clean_string(Some(value), field, 2048)?.ok_or_else(|| anyhow!("{field} is required"))?;
    let url = Url::parse(&url_string).map_err(|_| {
        anyhow!("{field} must be a valid Redfin URL containing /home/{{property_id}}")
    })?;
    let hostname = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if hostname != "redfin.com" && !hostname.ends_with(".redfin.com") {
        bail!("{field} must be a Redfin URL");
    }

    let path = url.path();
    let path = path.strip_suffix('/').unwrap_or(path);
    let Some((prefix, property_id)) = path.rsplit_once("/home/") else {
        bail!("{field} must contain a /home/{{property_id}} path");
    };
    if property_id.is_empty()
        || property_id.contains('/')
        || !property_id
            .chars()
            .all(|character| character.is_ascii_digit())
        || prefix.is_empty() && path != format!("/home/{property_id}")
    {
        bail!("{field} must contain a /home/{{property_id}} path");
    }

    let raw_id = json!(property_id);
    clean_property_id(&raw_id, field)
}

fn add_request(
    requests: &mut Vec<RedfinPropertyDetailsRequest>,
    seen_property_ids: &mut HashSet<u64>,
    property_id: u64,
    input: Value,
    source: &'static str,
) {
    if !seen_property_ids.insert(property_id) {
        return;
    }
    requests.push(RedfinPropertyDetailsRequest {
        params: RedfinPropertyDetailsParams { property_id },
        index: requests.len(),
        input,
        source,
    });
}

fn add_property_id_input(
    requests: &mut Vec<RedfinPropertyDetailsRequest>,
    seen_property_ids: &mut HashSet<u64>,
    value: &Value,
    source: &'static str,
    field: &str,
) -> Result<()> {
    let property_id = clean_property_id(value, field)?;
    let input = match value {
        Value::String(value) => Value::String(value.trim().to_owned()),
        _ => json!(property_id),
    };
    add_request(requests, seen_property_ids, property_id, input, source);
    Ok(())
}

fn add_url_input(
    requests: &mut Vec<RedfinPropertyDetailsRequest>,
    seen_property_ids: &mut HashSet<u64>,
    value: &Value,
    source: &'static str,
    field: &str,
) -> Result<()> {
    let url =
        clean_string(Some(value), field, 2048)?.ok_or_else(|| anyhow!("{field} is required"))?;
    let property_id = extract_redfin_property_id_from_url(&Value::String(url.clone()), field)?;
    add_request(
        requests,
        seen_property_ids,
        property_id,
        Value::String(url),
        source,
    );
    Ok(())
}

pub(crate) fn build_redfin_property_details_requests(
    input: &Value,
) -> Result<Vec<RedfinPropertyDetailsRequest>> {
    let fields = input.as_object();
    let mut requests = Vec::new();
    let mut seen_property_ids = HashSet::new();

    if let Some(value) = fields.and_then(|input| input.get("property_id")) {
        if !value.is_null() && value.as_str() != Some("") {
            add_property_id_input(
                &mut requests,
                &mut seen_property_ids,
                value,
                "property_id",
                "property_id",
            )?;
        }
    }

    if let Some(value) = fields.and_then(|input| input.get("url")) {
        if !value.is_null() && value.as_str() != Some("") {
            add_url_input(&mut requests, &mut seen_property_ids, value, "url", "url")?;
        }
    }

    if let Some(value) = fields.and_then(|input| input.get("property_ids")) {
        let Some(values) = value.as_array() else {
            bail!("property_ids must be an array");
        };
        for (index, property_id) in values.iter().enumerate() {
            add_property_id_input(
                &mut requests,
                &mut seen_property_ids,
                property_id,
                "property_ids",
                &format!("property_ids[{index}]"),
            )?;
        }
    }

    if let Some(value) = fields.and_then(|input| input.get("urls")) {
        let Some(values) = value.as_array() else {
            bail!("urls must be an array");
        };
        for (index, url) in values.iter().enumerate() {
            add_url_input(
                &mut requests,
                &mut seen_property_ids,
                url,
                "urls",
                &format!("urls[{index}]"),
            )?;
        }
    }

    if requests.is_empty() {
        bail!("Provide at least one property_id, property_ids item, url, or urls item");
    }
    if requests.len() > MAX_PROPERTIES_PER_RUN {
        bail!("Input cannot include more than {MAX_PROPERTIES_PER_RUN} unique properties per run");
    }
    Ok(requests)
}

pub(crate) fn describe_request(request: &RedfinPropertyDetailsRequest) -> String {
    format!(
        "property_id {} from {}",
        request.params.property_id, request.source
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_requests_in_input_order_and_deduplicates_ids_and_urls() {
        let input = json!({
            "property_id": "60791456",
            "url": "https://redfin.com/home/194191988?ref=one",
            "property_ids": [60791456, "194191988", 23232323],
            "urls": ["https://www.redfin.com/TN/Memphis/home/456456456/"]
        });

        let requests = build_redfin_property_details_requests(&input).unwrap();
        assert_eq!(
            requests
                .iter()
                .map(|request| request.params.property_id)
                .collect::<Vec<_>>(),
            vec![60791456, 194191988, 23232323, 456456456]
        );
        assert_eq!(requests[0].input, json!("60791456"));
        assert_eq!(requests[1].source, "url");
        assert_eq!(
            requests[1].input,
            json!("https://redfin.com/home/194191988?ref=one")
        );
        assert_eq!(
            describe_request(&requests[2]),
            "property_id 23232323 from property_ids"
        );
    }

    #[test]
    fn validates_property_ids_urls_and_batch_limit() {
        assert_eq!(
            build_redfin_property_details_requests(&json!({"property_id": 0}))
                .unwrap_err()
                .to_string(),
            "property_id must be greater than 0"
        );
        assert_eq!(
            build_redfin_property_details_requests(&json!({"url": "https://example.com/home/123"}))
                .unwrap_err()
                .to_string(),
            "url must be a Redfin URL"
        );
        assert_eq!(
            build_redfin_property_details_requests(
                &json!({"url": "https://redfin.com/not-a-property"})
            )
            .unwrap_err()
            .to_string(),
            "url must contain a /home/{property_id} path"
        );

        let ids = (1..=101).collect::<Vec<_>>();
        assert_eq!(
            build_redfin_property_details_requests(&json!({"property_ids": ids}))
                .unwrap_err()
                .to_string(),
            "Input cannot include more than 100 unique properties per run"
        );
    }

    #[test]
    fn preserves_schema_prefill_and_batch_constraints() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema.pointer("/properties/property_id/default"),
            Some(&json!(60791456))
        );
        assert_eq!(
            schema.pointer("/properties/url/prefill"),
            Some(&json!(
                "https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/60791456"
            ))
        );
        assert_eq!(
            schema.pointer("/properties/property_ids/maxItems"),
            Some(&json!(100))
        );
        assert_eq!(
            schema.pointer("/properties/urls/maxItems"),
            Some(&json!(100))
        );
    }
}
