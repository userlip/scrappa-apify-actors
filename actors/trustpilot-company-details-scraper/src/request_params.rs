use serde_json::{Map, Value};
use url::Url;

pub type RequestParams = Map<String, Value>;

#[derive(Debug, PartialEq)]
pub struct RequestPlan {
    pub domains: Vec<String>,
    pub base_params: RequestParams,
}

const LOCALES: &[&str] = &[
    "da-DK", "de-AT", "de-CH", "de-DE", "en-AU", "en-CA", "en-GB", "en-IE", "en-NZ", "en-US",
    "es-ES", "fi-FI", "fr-BE", "nl-BE", "fr-FR", "it-IT", "ja-JP", "nb-NO", "nl-NL", "pl-PL",
    "pt-BR", "pt-PT", "sv-SE",
];
const DEFAULT_LOCALE: &str = "en-US";
const MAX_DOMAINS_PER_RUN: usize = 100;

fn input_field<'a>(input: &'a Value, name: &str) -> Option<&'a Value> {
    input.as_object()?.get(name)
}

fn valid_domain_name(domain: &str) -> bool {
    if domain.len() > 253 || !domain.contains('.') {
        return false;
    }

    let labels = domain.split('.').collect::<Vec<_>>();
    if labels
        .iter()
        .any(|label| label.is_empty() || label.len() > 63)
    {
        return false;
    }

    let valid_label = |label: &str| {
        let bytes = label.as_bytes();
        bytes.first().is_some_and(u8::is_ascii_alphanumeric)
            && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
            && bytes
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
    };

    labels.iter().all(|label| valid_label(label))
        && labels.last().is_some_and(|suffix| {
            suffix.len() >= 2 && suffix.bytes().all(|byte| byte.is_ascii_lowercase())
        })
}

fn clean_domain(value: &Value, field: &str) -> Result<String, String> {
    let Some(value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };

    let raw_value = value.trim();
    if raw_value.is_empty() {
        return Err(format!("{field} cannot be empty"));
    }

    let has_http_scheme = raw_value
        .get(..7)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
        || raw_value
            .get(..8)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"));
    let candidate = if has_http_scheme {
        raw_value.to_owned()
    } else {
        format!("https://{raw_value}")
    };

    let domain = Url::parse(&candidate)
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .map(|host| host.strip_prefix("www.").unwrap_or(&host).to_owned())
        .filter(|host| valid_domain_name(host))
        .ok_or_else(|| {
            format!("{field} must be a valid domain name, for example trustpilot.com")
        })?;

    Ok(domain)
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

fn parse_domains(input: &Value) -> Result<Vec<String>, String> {
    let mut values: Vec<(&str, Value)> = Vec::new();

    if let Some(value) = input_field(input, "company_domain")
        .filter(|value| !value.is_null() && value.as_str() != Some(""))
    {
        values.push(("company_domain", value.clone()));
    }

    if let Some(value) = input_field(input, "company_domains")
        .filter(|value| !value.is_null() && value.as_str() != Some(""))
    {
        match value {
            Value::Array(domains) => {
                values.extend(
                    domains
                        .iter()
                        .cloned()
                        .map(|domain| ("company_domains", domain)),
                );
            }
            Value::String(domains) => {
                values.extend(
                    domains
                        .split([',', '\n'])
                        .map(str::trim)
                        .filter(|domain| !domain.is_empty())
                        .map(|domain| ("company_domains", Value::String(domain.to_owned()))),
                );
            }
            _ => {
                return Err(
                    "company_domains must be an array of strings or a comma/newline-separated string"
                        .into(),
                );
            }
        }
    }

    if values.is_empty() {
        return Err("Provide company_domain or company_domains".into());
    }

    let domains = values
        .into_iter()
        .map(|(field, value)| clean_domain(&value, field))
        .collect::<Result<Vec<_>, _>>()?;
    let mut unique_domains = Vec::with_capacity(domains.len());
    for domain in domains {
        if !unique_domains.contains(&domain) {
            unique_domains.push(domain);
        }
    }

    if unique_domains.len() > MAX_DOMAINS_PER_RUN {
        return Err(format!(
            "company_domains can include at most {MAX_DOMAINS_PER_RUN} domains per run"
        ));
    }

    Ok(unique_domains)
}

pub fn build_request_plan(input: &Value) -> Result<RequestPlan, String> {
    let domains = parse_domains(input)?;
    let locale =
        clean_enum(input_field(input, "locale"), "locale", LOCALES)?.unwrap_or(DEFAULT_LOCALE);
    let mut base_params = Map::new();
    base_params.insert("locale".into(), Value::String(locale.into()));

    if let Some(fields) = clean_optional_string(input_field(input, "fields"), "fields", 500)? {
        base_params.insert("fields".into(), Value::String(fields));
    }

    Ok(RequestPlan {
        domains,
        base_params,
    })
}

pub fn company_details_params(plan: &RequestPlan, domain: &str) -> RequestParams {
    let mut params = Map::new();
    params.insert("company_domain".into(), Value::String(domain.into()));
    params.extend(plan.base_params.clone());
    params
}

pub fn describe_request(plan: &RequestPlan) -> String {
    if plan.domains.len() == 1 {
        plan.domains[0].clone()
    } else {
        format!("{} company domains", plan.domains.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_single_and_batch_domains_and_deduplicates_them() {
        let plan = build_request_plan(&json!({
            "company_domain": " https://www.Trustpilot.com/review-path ",
            "company_domains": [
                "https://www.amazon.com/reviews",
                "amazon.com",
                "example.co.uk?ref=trustpilot"
            ],
            "locale": "en-US"
        }))
        .unwrap();

        assert_eq!(
            plan.domains,
            ["trustpilot.com", "amazon.com", "example.co.uk"]
        );
        assert_eq!(describe_request(&plan), "3 company domains");
        assert_eq!(
            company_details_params(&plan, "amazon.com"),
            json!({"company_domain":"amazon.com", "locale":"en-US"})
                .as_object()
                .unwrap()
                .clone()
        );
    }

    #[test]
    fn splits_comma_and_newline_input_and_applies_defaults() {
        let plan = build_request_plan(&json!({
            "company_domains": "trustpilot.com, https://www.amazon.com/reviews\nexample.com",
            "fields": " basic_info,ratings,metadata "
        }))
        .unwrap();

        assert_eq!(
            plan.domains,
            ["trustpilot.com", "amazon.com", "example.com"]
        );
        assert_eq!(
            plan.base_params,
            json!({"locale":"en-US", "fields":"basic_info,ratings,metadata"})
                .as_object()
                .unwrap()
                .clone()
        );
    }

    #[test]
    fn rejects_invalid_input_and_hostname_labels() {
        for (input, expected) in [
            (json!({}), "Provide company_domain or company_domains"),
            (
                json!({"company_domain":"invalid"}),
                "company_domain must be a valid domain name",
            ),
            (
                json!({"company_domains":[123]}),
                "company_domains must be a string",
            ),
            (
                json!({"company_domains":123}),
                "company_domains must be an array",
            ),
            (
                json!({"company_domain":"trustpilot.com", "locale":"en"}),
                "locale must be one of",
            ),
            (
                json!({"company_domain":"trustpilot.com", "fields":123}),
                "fields must be a string",
            ),
            (
                json!({"company_domains":(0..101).map(|i| format!("example{i}.com")).collect::<Vec<_>>() }),
                "at most 100 domains",
            ),
        ] {
            let error = build_request_plan(&input).unwrap_err();
            assert!(
                error.contains(expected),
                "{error} did not contain {expected}"
            );
        }

        for domain in [
            "-example.com",
            "example-.com",
            "example..com",
            "exa_mple.com",
        ] {
            let error = build_request_plan(&json!({ "company_domain": domain })).unwrap_err();
            assert!(error.contains("company_domain must be a valid domain name"));
        }
    }
}
