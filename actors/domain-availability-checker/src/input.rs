use std::collections::HashSet;

use anyhow::{Result, anyhow};
use serde_json::Value;
use url::Url;

#[derive(Debug, PartialEq, Eq)]
pub struct DomainRequest {
    pub input_domain: String,
    pub domain: Option<String>,
    pub validation_error: Option<String>,
}

pub fn get_domain_requests(input: &Value) -> Result<Vec<DomainRequest>> {
    let single_domain = input
        .get("domain")
        .map(|value| clean_domain_input(value, "domain"))
        .transpose()?
        .flatten();
    let mut raw_domains = Vec::new();
    if let Some(domain) = single_domain {
        raw_domains.push(Value::String(domain));
    }

    if let Some(domains) = input.get("domains") {
        let domains = domains
            .as_array()
            .ok_or_else(|| anyhow!("domains must be an array of strings"))?;
        raw_domains.extend(domains.iter().cloned());
    }

    let mut seen = HashSet::new();
    let mut requests = Vec::new();
    for (index, raw_domain) in raw_domains.into_iter().enumerate() {
        let raw_key = js_string(&raw_domain);
        let input_domain = match clean_domain_input(&raw_domain, &format!("domains[{index}]")) {
            Ok(Some(value)) => value,
            Ok(None) => continue,
            Err(error) => {
                let key = format!("invalid:{raw_key}");
                if seen.insert(key) {
                    requests.push(DomainRequest {
                        input_domain: raw_key,
                        domain: None,
                        validation_error: Some(error.to_string()),
                    });
                }
                continue;
            }
        };

        match normalize_domain(&input_domain) {
            Ok(domain) => {
                if seen.insert(domain.clone()) {
                    requests.push(DomainRequest {
                        input_domain,
                        domain: Some(domain),
                        validation_error: None,
                    });
                }
            }
            Err(error) => {
                let key = format!("invalid:{raw_key}");
                if seen.insert(key) {
                    requests.push(DomainRequest {
                        input_domain,
                        domain: None,
                        validation_error: Some(error),
                    });
                }
            }
        }
    }

    Ok(requests)
}

fn clean_domain_input(value: &Value, field: &str) -> Result<Option<String>> {
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_str()
        .ok_or_else(|| anyhow!("{field} must be a string"))?;
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(value.to_owned()))
}

fn normalize_domain(value: &str) -> Result<String, String> {
    let original = value;
    let mut domain = value.trim().to_owned();

    if has_url_scheme(&domain) {
        domain = Url::parse(&domain)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .ok_or_else(|| format!("Invalid domain \"{original}\". Provide a valid fully qualified domain name."))?;
    } else {
        let end = domain.find(['/', '?', '#']).unwrap_or(domain.len());
        domain.truncate(end);
    }

    domain = domain.trim().trim_end_matches('.').to_lowercase();
    if !domain.contains('.') {
        return Err(format!(
            "Invalid domain \"{original}\". Provide a fully qualified domain such as example.com."
        ));
    }
    if domain.encode_utf16().count() > 253 {
        return Err(format!(
            "Invalid domain \"{original}\". Domain names must be 253 characters or fewer."
        ));
    }

    let labels: Vec<_> = domain.split('.').collect();
    if labels
        .iter()
        .any(|label| label.is_empty() || label.encode_utf16().count() > 63)
    {
        return Err(format!(
            "Invalid domain \"{original}\". Domain labels must be between 1 and 63 characters."
        ));
    }
    if !labels.iter().all(|label| {
        let bytes = label.as_bytes();
        bytes[0].is_ascii_alphanumeric()
            && bytes[bytes.len() - 1].is_ascii_alphanumeric()
            && bytes
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
    }) {
        return Err(format!(
            "Invalid domain \"{original}\". Provide a valid fully qualified domain name."
        ));
    }

    Ok(domain)
}

fn has_url_scheme(value: &str) -> bool {
    let Some(separator) = value.find("://") else {
        return false;
    };
    let scheme = &value[..separator];
    !scheme.is_empty()
        && scheme.as_bytes()[0].is_ascii_alphabetic()
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'))
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::Null => String::new(),
                value => js_string(value),
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

    #[test]
    fn normalizes_single_domain_urls_and_duplicates() {
        assert_eq!(
            get_domain_requests(&json!({
                "domain": " https://Example.com/path?x=1 ",
                "domains": ["https://example.com/pricing", "Sub.Example.org.", ""]
            }))
            .unwrap(),
            vec![
                DomainRequest {
                    input_domain: "https://Example.com/path?x=1".to_owned(),
                    domain: Some("example.com".to_owned()),
                    validation_error: None,
                },
                DomainRequest {
                    input_domain: "Sub.Example.org.".to_owned(),
                    domain: Some("sub.example.org".to_owned()),
                    validation_error: None,
                },
            ]
        );
    }

    #[test]
    fn keeps_invalid_domains_as_individual_failures() {
        let requests = get_domain_requests(&json!({
            "domains": ["localhost", "bad_domain.com", " localhost ", "localhost"]
        }))
        .unwrap();

        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].input_domain, "localhost");
        assert_eq!(
            requests[0].validation_error.as_deref(),
            Some("Invalid domain \"localhost\". Provide a fully qualified domain such as example.com.")
        );
        assert_eq!(
            requests[1].validation_error.as_deref(),
            Some("Invalid domain \"bad_domain.com\". Provide a valid fully qualified domain name.")
        );
        assert_eq!(requests[2].input_domain, "localhost");
    }

    #[test]
    fn skips_empty_values_and_rejects_invalid_field_types() {
        assert!(get_domain_requests(&Value::Null).unwrap().is_empty());
        assert!(get_domain_requests(&json!({})).unwrap().is_empty());
        assert!(get_domain_requests(&json!({ "domains": [null, "  "] }))
            .unwrap()
            .is_empty());
        assert_eq!(
            get_domain_requests(&json!({ "domains": "example.com" }))
                .unwrap_err()
                .to_string(),
            "domains must be an array of strings"
        );
        assert_eq!(
            get_domain_requests(&json!({ "domain": 7 }))
                .unwrap_err()
                .to_string(),
            "domain must be a string"
        );
    }

    #[test]
    fn records_non_string_array_values_as_failures() {
        let requests = get_domain_requests(&json!({ "domains": [7, true, {"key":"value"}] }))
            .unwrap();

        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].input_domain, "7");
        assert_eq!(requests[1].input_domain, "true");
        assert_eq!(requests[2].input_domain, "[object Object]");
        assert!(requests.iter().all(|request| request.domain.is_none()));
    }

    #[test]
    fn measures_domain_lengths_like_javascript_strings() {
        let domain = format!("{}.com", "é".repeat(32));
        let requests = get_domain_requests(&json!({
            "domains": [domain.clone()]
        }))
        .unwrap();

        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].validation_error.as_deref(),
            Some(
                format!(
                    "Invalid domain \"{domain}\". Provide a valid fully qualified domain name."
                )
                .as_str()
            )
        );
    }
}
