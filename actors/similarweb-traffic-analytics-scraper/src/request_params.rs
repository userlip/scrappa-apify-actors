use std::collections::HashSet;

use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use url::Url;

const MAX_DOMAINS_PER_RUN: usize = 100;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SimilarwebTrafficRequest {
    pub(crate) domain: String,
    pub(crate) input_domain: String,
}

fn clean_input_string(value: Option<&Value>, field: &str) -> Result<Option<String>> {
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
    Ok(Some(trimmed.to_owned()))
}

fn normalize_domain(value: &str) -> Result<String> {
    let lower = value.trim().to_lowercase();
    let candidate = if lower.contains("://") {
        lower.clone()
    } else {
        format!("https://{lower}")
    };

    let mut domain = match Url::parse(&candidate) {
        Ok(parsed) => parsed.host_str().map(str::to_owned).unwrap_or_else(|| {
            lower
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .split('/')
                .next()
                .unwrap_or_default()
                .to_owned()
        }),
        Err(_) => lower
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .split('/')
            .next()
            .unwrap_or_default()
            .to_owned(),
    };

    if let Some(stripped) = domain.strip_prefix("www.") {
        domain = stripped.to_owned();
    }
    if let Some(stripped) = domain.strip_suffix('.') {
        domain = stripped.to_owned();
    }

    let is_ipv4_shape = domain.split('.').count() == 4
        && domain.split('.').all(|part| {
            !part.is_empty() && part.len() <= 3 && part.bytes().all(|byte| byte.is_ascii_digit())
        });
    if is_ipv4_shape {
        bail!("domain \"{value}\" must be a domain name, not an IP address");
    }
    if domain.len() < 3 {
        bail!("domain \"{value}\" must be at least 3 characters after normalization");
    }
    if domain.len() > 253 {
        bail!("domain \"{value}\" cannot exceed 253 characters after normalization");
    }

    let labels: Vec<&str> = domain.split('.').collect();
    let valid_label = |label: &str| {
        !label.is_empty()
            && label.len() <= 63
            && label.as_bytes()[0].is_ascii_alphanumeric()
            && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    };
    if labels.len() < 2 || !labels.iter().all(|label| valid_label(label)) {
        bail!("domain \"{value}\" must be a valid domain name such as example.com");
    }

    Ok(domain)
}

pub(crate) fn build_similarweb_traffic_requests(
    input: &Value,
) -> Result<Vec<SimilarwebTrafficRequest>> {
    let mut raw_domains = Vec::new();
    if let Some(domain) = clean_input_string(input.get("domain"), "domain")? {
        raw_domains.push(domain);
    }

    match input.get("domains") {
        None | Some(Value::Null) => {}
        Some(Value::String(value)) if value.is_empty() => {}
        Some(Value::Array(domains)) => {
            for (index, value) in domains.iter().enumerate() {
                if let Some(domain) = clean_input_string(Some(value), &format!("domains[{index}]"))?
                {
                    raw_domains.push(domain);
                }
            }
        }
        Some(_) => bail!("domains must be an array of strings"),
    }

    if raw_domains.is_empty() {
        bail!("At least one domain is required. Provide domain or domains.");
    }

    let mut seen = HashSet::new();
    let mut requests = Vec::new();
    for input_domain in raw_domains {
        let domain = normalize_domain(&input_domain)?;
        if seen.insert(domain.clone()) {
            requests.push(SimilarwebTrafficRequest {
                domain,
                input_domain,
            });
        }
    }

    if requests.len() > MAX_DOMAINS_PER_RUN {
        bail!("domains cannot contain more than {MAX_DOMAINS_PER_RUN} unique domains per run");
    }

    Ok(requests)
}

pub(crate) fn describe_similarweb_traffic_requests(
    requests: &[SimilarwebTrafficRequest],
) -> String {
    if requests.len() == 1 {
        return requests[0].domain.clone();
    }

    let domains = requests
        .iter()
        .take(3)
        .map(|request| request.domain.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{} domains ({}{})",
        requests.len(),
        domains,
        if requests.len() > 3 { ", ..." } else { "" }
    )
}

pub(crate) fn input_required_error() -> anyhow::Error {
    anyhow!("Input is required")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn normalizes_unique_domains_from_single_and_batch_inputs() {
        let requests = build_similarweb_traffic_requests(&json!({
            "domain": " https://www.Google.com/search ",
            "domains": ["google.com", "GITHUB.com/features", "www.shopify.com"]
        }))
        .unwrap();

        assert_eq!(
            requests,
            vec![
                SimilarwebTrafficRequest {
                    domain: "google.com".to_owned(),
                    input_domain: "https://www.Google.com/search".to_owned(),
                },
                SimilarwebTrafficRequest {
                    domain: "github.com".to_owned(),
                    input_domain: "GITHUB.com/features".to_owned(),
                },
                SimilarwebTrafficRequest {
                    domain: "shopify.com".to_owned(),
                    input_domain: "www.shopify.com".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn normalizes_ports_paths_and_internationalized_domains() {
        let requests = build_similarweb_traffic_requests(&json!({
            "domains": [
                "https://example.com:8443/path?utm_source=test",
                "https://www.bücher.example/katalog"
            ]
        }))
        .unwrap();

        assert_eq!(requests[0].domain, "example.com");
        assert_eq!(requests[1].domain, "xn--bcher-kva.example");
    }

    #[test]
    fn rejects_missing_or_malformed_domains() {
        assert!(build_similarweb_traffic_requests(&json!({}))
            .unwrap_err()
            .to_string()
            .contains("At least one domain is required"));
        assert!(
            build_similarweb_traffic_requests(&json!({"domains":"google.com"}))
                .unwrap_err()
                .to_string()
                .contains("domains must be an array")
        );
        assert!(
            build_similarweb_traffic_requests(&json!({"domain":"not-a-valid-domain!!!"}))
                .unwrap_err()
                .to_string()
                .contains("must be a valid domain name")
        );
        assert!(
            build_similarweb_traffic_requests(&json!({"domain":"93.184.216.34"}))
                .unwrap_err()
                .to_string()
                .contains("not an IP address")
        );
    }

    #[test]
    fn skips_blank_values_and_deduplicates_before_the_run_limit() {
        let requests = build_similarweb_traffic_requests(&json!({
            "domain":" ",
            "domains":["google.com", "google.com", " "]
        }))
        .unwrap();
        assert_eq!(requests.len(), 1);

        let domains = (0..101)
            .map(|index| format!("site{index}.example"))
            .collect::<Vec<_>>();
        assert!(
            build_similarweb_traffic_requests(&json!({"domains":domains}))
                .unwrap_err()
                .to_string()
                .contains("more than 100 unique domains")
        );
    }

    #[test]
    fn describes_requests_for_logs() {
        let one = build_similarweb_traffic_requests(&json!({"domain":"google.com"})).unwrap();
        assert_eq!(describe_similarweb_traffic_requests(&one), "google.com");

        let many = build_similarweb_traffic_requests(&json!({
            "domains":["google.com", "github.com", "shopify.com", "stripe.com"]
        }))
        .unwrap();
        assert_eq!(
            describe_similarweb_traffic_requests(&many),
            "4 domains (google.com, github.com, shopify.com, ...)"
        );
    }
}
