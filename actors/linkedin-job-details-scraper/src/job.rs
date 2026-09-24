use anyhow::{anyhow, Result};
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use url::Url;

pub(crate) const JOB_URL_ERROR: &str =
    "Invalid LinkedIn job URL. Expected format: https://www.linkedin.com/jobs/view/job-id";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UrlRequest {
    pub(crate) input_url: String,
    pub(crate) normalized_url: Option<String>,
    pub(crate) validation_error: Option<String>,
}

#[derive(Debug)]
pub(crate) struct ScrappaApiError {
    pub(crate) status: u16,
    pub(crate) message: String,
}

impl std::fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.message
        )
    }
}

impl std::error::Error for ScrappaApiError {}

pub(crate) fn is_recoverable_job_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| error.status == 404)
}

pub(crate) fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    js_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

pub(crate) fn build_job_params(
    normalized_url: &str,
    use_cache: Option<&Value>,
    maximum_cache_age: Option<&Value>,
) -> Vec<(String, String)> {
    let mut params = vec![("url".to_owned(), normalized_url.to_owned())];
    if !use_cache.is_some_and(js_truthy) {
        return params;
    }

    params.push(("use_cache".to_owned(), "1".to_owned()));
    if let Some(age) = maximum_cache_age.and_then(cache_age_string) {
        params.push(("maximum_cache_age".to_owned(), age));
    }
    params
}

pub(crate) fn cache_age_string(value: &Value) -> Option<String> {
    let number = value.as_number()?;
    if let Some(age) = number.as_u64().filter(|age| *age >= 1) {
        return Some(age.to_string());
    }

    let age = number.as_f64()?;
    if !age.is_finite() || age < 1.0 || age.fract() != 0.0 {
        return None;
    }
    Some(format!("{age:.0}"))
}

pub(crate) fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

pub(crate) fn get_input_urls(input: Option<&Value>) -> Result<Vec<UrlRequest>> {
    let mut raw_urls = Vec::new();
    if let Some(url) = input
        .and_then(|input| input.get("url"))
        .and_then(Value::as_str)
    {
        raw_urls.push(url);
    }
    if let Some(urls) = input
        .and_then(|input| input.get("urls"))
        .and_then(Value::as_array)
    {
        for url in urls {
            raw_urls.push(
                url.as_str()
                    .ok_or_else(|| anyhow!("LinkedIn job URLs must be strings"))?,
            );
        }
    }

    let mut seen = HashMap::new();
    let mut requests = Vec::new();
    for raw_url in raw_urls {
        let input_url = raw_url.trim().to_owned();
        if input_url.is_empty() {
            continue;
        }

        match normalize_linkedin_job_url(&input_url) {
            Ok(normalized_url) => {
                if seen.insert(normalized_url.clone(), ()).is_none() {
                    requests.push(UrlRequest {
                        input_url,
                        normalized_url: Some(normalized_url),
                        validation_error: None,
                    });
                }
            }
            Err(validation_error) => {
                let key = format!("invalid:{input_url}");
                if seen.insert(key, ()).is_none() {
                    requests.push(UrlRequest {
                        input_url,
                        normalized_url: None,
                        validation_error: Some(validation_error),
                    });
                }
            }
        }
    }

    Ok(requests)
}

pub(crate) fn normalize_linkedin_job_url(raw_url: &str) -> std::result::Result<String, String> {
    let candidate = raw_url.trim();
    if candidate.is_empty() {
        return Err(JOB_URL_ERROR.to_owned());
    }

    let has_protocol = candidate.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty() && scheme.bytes().all(|byte| byte.is_ascii_alphabetic())
    });
    let with_protocol = if has_protocol {
        candidate.to_owned()
    } else {
        format!("https://{candidate}")
    };
    let parsed = Url::parse(&with_protocol).map_err(|_| "Invalid URL".to_owned())?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(JOB_URL_ERROR.to_owned());
    }
    if !parsed.username().is_empty()
        || parsed
            .password()
            .is_some_and(|password| !password.is_empty())
        || parsed.port().is_some()
    {
        return Err(JOB_URL_ERROR.to_owned());
    }

    let hostname = parsed.host_str().unwrap_or_default();
    if !is_linkedin_hostname(hostname) {
        return Err(JOB_URL_ERROR.to_owned());
    }

    let path = parsed.path();
    let Some(rest) = strip_job_view_prefix(path) else {
        return Err(JOB_URL_ERROR.to_owned());
    };
    let job_id = rest.split('/').next().unwrap_or_default();
    if job_id.is_empty() {
        return Err(JOB_URL_ERROR.to_owned());
    }

    Ok(format!(
        "{}://www.linkedin.com/jobs/view/{job_id}",
        parsed.scheme()
    ))
}

pub(crate) fn strip_job_view_prefix(path: &str) -> Option<&str> {
    let prefix = "/jobs/view/";
    let path_prefix = path.get(..prefix.len())?;
    if !path_prefix.eq_ignore_ascii_case(prefix) {
        return None;
    }
    path.get(prefix.len()..)
}

pub(crate) fn is_linkedin_hostname(hostname: &str) -> bool {
    if hostname == "linkedin.com" {
        return true;
    }
    let Some(prefix) = hostname.strip_suffix(".linkedin.com") else {
        return false;
    };
    prefix == "www"
        || prefix == "m"
        || ((2..=3).contains(&prefix.len()) && prefix.bytes().all(|byte| byte.is_ascii_lowercase()))
}

pub(crate) fn first_present(response: &Map<String, Value>, keys: &[&str]) -> Option<Value> {
    keys.iter()
        .find_map(|key| {
            response.get(*key).filter(|value| {
                !value.is_null() && value.as_str().is_none_or(|value| !value.trim().is_empty())
            })
        })
        .cloned()
}

pub(crate) fn build_success_item(
    response: Value,
    input_url: &str,
    normalized_url: &str,
) -> Result<Value> {
    let mut fields = response
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("Scrappa job response was not a JSON object"))?;
    let response_url = fields
        .get("url")
        .and_then(Value::as_str)
        .filter(|url| !url.trim().is_empty())
        .unwrap_or(normalized_url)
        .to_owned();

    fields.insert(
        "success".to_owned(),
        fields
            .get("success")
            .filter(|success| !success.is_null())
            .cloned()
            .unwrap_or(Value::Bool(true)),
    );
    for (canonical, aliases) in [
        ("title", &["title", "job_title"][..]),
        ("company", &["company", "company_name"][..]),
        ("posted_date", &["posted_date", "date_posted"][..]),
        ("applicants", &["applicants", "applicant_count"][..]),
        ("apply_url", &["apply_url", "application_url"][..]),
    ] {
        if let Some(value) = first_present(&fields, aliases) {
            fields.insert(canonical.to_owned(), value);
        }
    }
    fields.insert("url".to_owned(), Value::String(response_url));
    fields.insert("input_url".to_owned(), Value::String(input_url.to_owned()));
    fields.insert(
        "normalized_url".to_owned(),
        Value::String(normalized_url.to_owned()),
    );
    Ok(Value::Object(fields))
}

pub(crate) fn build_failure_item(
    error_message: &str,
    api_status: Option<u16>,
    input_url: &str,
    normalized_url: Option<&str>,
) -> Value {
    let mut result = Map::new();
    result.insert("success".to_owned(), Value::Bool(false));
    result.insert("input_url".to_owned(), Value::String(input_url.to_owned()));
    if let Some(normalized_url) = normalized_url {
        result.insert(
            "normalized_url".to_owned(),
            Value::String(normalized_url.to_owned()),
        );
        result.insert("url".to_owned(), Value::String(normalized_url.to_owned()));
    }
    result.insert("error".to_owned(), Value::String(error_message.to_owned()));
    result.insert(
        "error_type".to_owned(),
        Value::String(
            if api_status.is_some() {
                "scrappa_api_error"
            } else {
                "error"
            }
            .to_owned(),
        ),
    );
    result.insert(
        "message".to_owned(),
        Value::String(if api_status == Some(404) {
            "Job not found".to_owned()
        } else {
            error_message.to_owned()
        }),
    );
    if let Some(status) = api_status {
        result.insert(
            "status_code".to_owned(),
            Value::Number(Number::from(status)),
        );
    }
    Value::Object(result)
}

pub(crate) fn is_success(result: &Value) -> bool {
    result.get("success").is_some_and(js_truthy)
}

pub(crate) fn should_charge_result(result: &Value) -> bool {
    result.get("success") == Some(&Value::Bool(true))
}

pub(crate) fn build_output(result: &Value) -> Value {
    let Some(fields) = result.as_object() else {
        return result.clone();
    };
    let mut output = fields.clone();
    for key in ["input_url", "normalized_url", "url", "error", "error_type"] {
        output.remove(key);
    }
    Value::Object(output)
}
