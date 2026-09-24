use std::collections::HashSet;

use anyhow::{bail, Result};
use serde_json::Value;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseType {
    Json,
    Markdown,
}

impl ResponseType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Markdown => "markdown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlRequest {
    pub input_url: String,
    pub request_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebScraperParams {
    pub url: String,
    pub include_html: Option<bool>,
    pub response_type: ResponseType,
}

pub fn get_response_type(input: Option<&Value>) -> Result<ResponseType> {
    let response_type = input.and_then(|value| value.get("response_type"));
    match response_type {
        None | Some(Value::Null) => Ok(ResponseType::Json),
        Some(Value::String(value)) if value == "json" => Ok(ResponseType::Json),
        Some(Value::String(value)) if value == "markdown" => Ok(ResponseType::Markdown),
        _ => bail!("response_type must be either \"json\" or \"markdown\"."),
    }
}

pub fn get_input_urls(input: Option<&Value>) -> Vec<UrlRequest> {
    let mut raw_urls = Vec::new();
    if let Some(url) = input
        .and_then(|value| value.get("url"))
        .and_then(Value::as_str)
    {
        raw_urls.push(url);
    }
    if let Some(urls) = input
        .and_then(|value| value.get("urls"))
        .and_then(Value::as_array)
    {
        raw_urls.extend(urls.iter().filter_map(Value::as_str));
    }

    let mut seen = HashSet::new();
    raw_urls
        .into_iter()
        .filter_map(|raw_url| {
            let input_url = raw_url.trim();
            if input_url.is_empty() || !seen.insert(normalize_for_dedupe(input_url)) {
                return None;
            }
            Some(UrlRequest {
                input_url: input_url.to_owned(),
                request_url: input_url.to_owned(),
            })
        })
        .collect()
}

pub fn build_web_scraper_params(
    request: &UrlRequest,
    input: Option<&Value>,
    response_type: ResponseType,
) -> WebScraperParams {
    WebScraperParams {
        url: request.request_url.clone(),
        include_html: match response_type {
            ResponseType::Json => Some(
                input
                    .and_then(|value| value.get("include_html"))
                    .and_then(Value::as_bool)
                    == Some(true),
            ),
            ResponseType::Markdown => None,
        },
        response_type,
    }
}

pub fn describe_web_scraper_request(params: &WebScraperParams) -> String {
    let mut parts = vec![
        format!("url={}", params.url),
        format!("response_type={}", params.response_type.as_str()),
    ];
    if params.response_type == ResponseType::Json {
        parts.push(format!(
            "include_html={}",
            if params.include_html == Some(true) {
                "true"
            } else {
                "false"
            }
        ));
    }
    parts.join(", ")
}

fn normalize_for_dedupe(url: &str) -> String {
    let has_protocol = url
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        || url
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"));
    let candidate = if has_protocol {
        url.to_owned()
    } else {
        format!("https://{url}")
    };

    if Url::parse(&candidate).is_err() {
        return url.to_owned();
    }
    if !has_protocol {
        return format!("schemeless:{url}");
    }

    let Some((protocol, remainder)) = candidate.split_once("://") else {
        return candidate;
    };
    let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
    let (authority, path_and_search) = remainder.split_at(authority_end);
    format!(
        "{}://{}{}",
        protocol.to_lowercase(),
        normalize_authority_for_dedupe(authority),
        path_and_search
    )
}

fn normalize_authority_for_dedupe(authority: &str) -> String {
    let (user_info, host_and_port) = match authority.rfind('@') {
        Some(index) => (&authority[..=index], &authority[index + 1..]),
        None => ("", authority),
    };

    if host_and_port.starts_with('[') {
        if let Some(bracket_end) = host_and_port.find(']') {
            return format!(
                "{}{}{}",
                user_info,
                host_and_port[..=bracket_end].to_lowercase(),
                &host_and_port[bracket_end + 1..]
            );
        }
    }

    let port_separator = host_and_port.rfind(':');
    let has_single_colon =
        port_separator.is_some_and(|index| host_and_port[..index].find(':').is_none());
    if let Some(port_separator) = port_separator.filter(|_| has_single_colon) {
        return format!(
            "{}{}{}",
            user_info,
            host_and_port[..port_separator].to_lowercase(),
            &host_and_port[port_separator..]
        );
    }

    format!("{}{}", user_info, host_and_port.to_lowercase())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        build_web_scraper_params, describe_web_scraper_request, get_input_urls, get_response_type,
        ResponseType, UrlRequest,
    };

    #[test]
    fn combines_backward_compatible_url_before_batch_urls_and_deduplicates() {
        assert_eq!(
            get_input_urls(Some(&json!({
                "url": " https://example.com ",
                "urls": [
                    "HTTPS://EXAMPLE.COM",
                    "https://www.iana.org/domains/reserved",
                    "   ",
                    "example.org"
                ]
            }))),
            vec![
                UrlRequest {
                    input_url: "https://example.com".to_owned(),
                    request_url: "https://example.com".to_owned(),
                },
                UrlRequest {
                    input_url: "https://www.iana.org/domains/reserved".to_owned(),
                    request_url: "https://www.iana.org/domains/reserved".to_owned(),
                },
                UrlRequest {
                    input_url: "example.org".to_owned(),
                    request_url: "example.org".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn preserves_case_sensitive_paths_fragments_and_user_info() {
        let urls = get_input_urls(Some(&json!({
            "urls": [
                "HTTPS://EXAMPLE.COM/Docs",
                "https://example.com/Docs",
                "https://example.com/docs",
                "https://User:PaSs@EXAMPLE.COM/Docs",
                "https://user:pass@example.com/Docs",
                "https://example.com/#/one",
                "https://example.com/#/two"
            ]
        })));
        assert_eq!(urls.len(), 6);
        assert_eq!(urls[0].input_url, "HTTPS://EXAMPLE.COM/Docs");
        assert_eq!(urls[1].input_url, "https://example.com/docs");
    }

    #[test]
    fn keeps_schemeless_urls_distinct_from_protocol_urls() {
        let urls = get_input_urls(Some(&json!({
            "urls": ["example.com", "https://example.com", "HTTPS://EXAMPLE.COM"]
        })));
        assert_eq!(urls.len(), 2);
        assert_eq!(urls[0].input_url, "example.com");
    }

    #[test]
    fn ignores_non_string_urls_and_blank_values() {
        let urls = get_input_urls(Some(&json!({
            "urls": ["https://example.com", null, 42, {"url": "https://example.org"}, "  "]
        })));
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0].input_url, "https://example.com");
    }

    #[test]
    fn defaults_and_validates_response_type() {
        assert_eq!(get_response_type(None).unwrap(), ResponseType::Json);
        assert_eq!(
            get_response_type(Some(&json!({"response_type": "markdown"}))).unwrap(),
            ResponseType::Markdown
        );
        assert!(get_response_type(Some(&json!({"response_type": "xml"})))
            .unwrap_err()
            .to_string()
            .contains("response_type must be either"));
        assert!(get_response_type(Some(&json!({"response_type": 42}))).is_err());
    }

    #[test]
    fn builds_json_and_markdown_params_with_matching_include_html_behavior() {
        let request = UrlRequest {
            input_url: "https://example.com".to_owned(),
            request_url: "https://example.com".to_owned(),
        };
        let json_params = build_web_scraper_params(
            &request,
            Some(&json!({"include_html": true})),
            ResponseType::Json,
        );
        assert_eq!(json_params.include_html, Some(true));
        assert_eq!(
            describe_web_scraper_request(&json_params),
            "url=https://example.com, response_type=json, include_html=true"
        );

        let markdown_params = build_web_scraper_params(
            &request,
            Some(&json!({"include_html": true})),
            ResponseType::Markdown,
        );
        assert_eq!(markdown_params.include_html, None);
        assert_eq!(
            describe_web_scraper_request(&markdown_params),
            "url=https://example.com, response_type=markdown"
        );
    }
}
