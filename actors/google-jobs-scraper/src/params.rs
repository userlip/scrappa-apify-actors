use anyhow::{bail, Result};
use serde::Deserialize;
use serde_json::Value;

pub const DEFAULT_JOB_SEARCH_QUERY: &str = "nurse jobs in Austin";

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct GoogleJobsInput {
    pub q: Option<String>,
    pub next_page_token: Option<String>,
    pub hl: Option<String>,
    pub gl: Option<String>,
    pub google_domain: Option<String>,
    pub uule: Option<String>,
    pub lrad: Option<Value>,
    pub uds: Option<String>,
}

impl GoogleJobsInput {
    fn default_search() -> Self {
        Self {
            q: Some(DEFAULT_JOB_SEARCH_QUERY.to_owned()),
            gl: Some("us".to_owned()),
            hl: Some("en".to_owned()),
            google_domain: Some("google.com".to_owned()),
            ..Self::default()
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.q.is_none() && self.next_page_token.is_none() {
            bail!("Job search query is required unless next_page_token is provided.");
        }
        if self.lrad.is_some() && self.uule.is_none() {
            bail!("Search radius (lrad) requires an encoded location (uule).");
        }
        Ok(())
    }
}

pub fn normalize_jobs_input(raw_input: Option<Value>) -> Result<GoogleJobsInput> {
    let Some(raw_input) = raw_input else {
        return Ok(GoogleJobsInput::default_search());
    };
    if raw_input.is_null() {
        return Ok(GoogleJobsInput::default_search());
    }

    let mut input: GoogleJobsInput = serde_json::from_value(raw_input)?;
    input.q = trim_nonempty(input.q);
    input.next_page_token = trim_nonempty(input.next_page_token);
    input.hl = trim_nonempty(input.hl);
    input.gl = trim_nonempty(input.gl);
    input.google_domain = trim_nonempty(input.google_domain);
    input.uule = trim_nonempty(input.uule);
    input.uds = trim_nonempty(input.uds);
    if input.lrad.as_ref().is_some_and(Value::is_null) {
        input.lrad = None;
    }

    let has_known_input = input.q.is_some()
        || input.next_page_token.is_some()
        || input.hl.is_some()
        || input.gl.is_some()
        || input.google_domain.is_some()
        || input.uule.is_some()
        || input.lrad.is_some()
        || input.uds.is_some();
    if !has_known_input {
        return Ok(GoogleJobsInput::default_search());
    }
    if input.q.is_some() || input.next_page_token.is_some() {
        return Ok(input);
    }

    let mut defaults = GoogleJobsInput::default_search();
    defaults.hl = input.hl.or(defaults.hl);
    defaults.gl = input.gl.or(defaults.gl);
    defaults.google_domain = input.google_domain.or(defaults.google_domain);
    defaults.uule = input.uule;
    defaults.lrad = input.lrad;
    defaults.uds = input.uds;
    Ok(defaults)
}

pub fn build_jobs_params(input: &GoogleJobsInput) -> Vec<(String, String)> {
    let mut params = Vec::new();
    push_string_param(&mut params, "q", input.q.as_deref());
    push_string_param(
        &mut params,
        "next_page_token",
        input.next_page_token.as_deref(),
    );
    push_string_param(&mut params, "hl", input.hl.as_deref());
    push_string_param(&mut params, "gl", input.gl.as_deref());
    push_string_param(&mut params, "google_domain", input.google_domain.as_deref());
    push_string_param(&mut params, "uule", input.uule.as_deref());
    if let Some(value) = &input.lrad {
        if !value.is_null() && value.as_str() != Some("") {
            params.push(("lrad".to_owned(), value_to_string(value)));
        }
    }
    push_string_param(&mut params, "uds", input.uds.as_deref());
    params
}

pub fn build_indeed_fallback_params(input: &GoogleJobsInput) -> Vec<(String, String)> {
    let (query, location) = parse_jobs_query(input.q.as_deref().unwrap_or_default());
    let mut params = vec![
        (
            "query".to_owned(),
            if query.is_empty() {
                input.q.clone().unwrap_or_default()
            } else {
                query
            },
        ),
        ("limit".to_owned(), "10".to_owned()),
    ];
    if let Some(location) = location {
        params.push(("location".to_owned(), location));
    }
    if let Some(gl) = &input.gl {
        params.push(("country".to_owned(), gl.to_uppercase()));
        params.push(("gl".to_owned(), gl.clone()));
    }
    push_string_param(&mut params, "hl", input.hl.as_deref());
    params
}

fn parse_jobs_query(query: &str) -> (String, Option<String>) {
    let normalized = query.split_whitespace().collect::<Vec<_>>().join(" ");
    let lower = normalized.to_ascii_lowercase();
    let marker = [" jobs in ", " job in "]
        .into_iter()
        .filter_map(|marker| lower.find(marker).map(|index| (index, marker.len())))
        .min_by_key(|(index, _)| *index);
    let Some((index, marker_len)) = marker else {
        return (normalized, None);
    };
    let query_part = normalized[..index].trim();
    let location = normalized[index + marker_len..].trim();
    if query_part.is_empty() || location.is_empty() {
        return (normalized, None);
    }
    (query_part.to_owned(), Some(location.to_owned()))
}

fn trim_nonempty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

fn push_string_param(params: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        params.push((key.to_owned(), value.to_owned()));
    }
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".to_owned(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn applies_default_search_when_input_is_missing_or_blank() {
        let expected = GoogleJobsInput::default_search();
        assert_eq!(normalize_jobs_input(None).unwrap(), expected);
        assert_eq!(normalize_jobs_input(Some(json!({}))).unwrap(), expected);
        assert_eq!(
            normalize_jobs_input(Some(json!({ "unknown": "value" }))).unwrap(),
            expected
        );
    }

    #[test]
    fn preserves_next_page_token_without_injecting_default_query() {
        assert_eq!(
            normalize_jobs_input(Some(json!({ "next_page_token": " token " }))).unwrap(),
            GoogleJobsInput {
                next_page_token: Some("token".to_owned()),
                ..GoogleJobsInput::default()
            }
        );
    }

    #[test]
    fn adds_defaults_to_partial_targeting_input_and_trims_strings() {
        assert_eq!(
            normalize_jobs_input(Some(json!({ "gl": " de ", "uds": " filter " }))).unwrap(),
            GoogleJobsInput {
                q: Some(DEFAULT_JOB_SEARCH_QUERY.to_owned()),
                gl: Some("de".to_owned()),
                hl: Some("en".to_owned()),
                google_domain: Some("google.com".to_owned()),
                uds: Some("filter".to_owned()),
                ..GoogleJobsInput::default()
            }
        );
    }

    #[test]
    fn forwards_all_google_jobs_parameters_and_omits_empty_fields() {
        let input = GoogleJobsInput {
            q: Some("software engineer".to_owned()),
            gl: Some("us".to_owned()),
            hl: Some("en".to_owned()),
            google_domain: Some("google.com".to_owned()),
            uule: Some("encoded location".to_owned()),
            lrad: Some(json!(25)),
            uds: Some("filter-token".to_owned()),
            ..GoogleJobsInput::default()
        };
        assert_eq!(
            build_jobs_params(&input),
            vec![
                ("q".to_owned(), "software engineer".to_owned()),
                ("hl".to_owned(), "en".to_owned()),
                ("gl".to_owned(), "us".to_owned()),
                ("google_domain".to_owned(), "google.com".to_owned()),
                ("uule".to_owned(), "encoded location".to_owned()),
                ("lrad".to_owned(), "25".to_owned()),
                ("uds".to_owned(), "filter-token".to_owned()),
            ]
        );
    }

    #[test]
    fn requires_query_unless_pagination_token_is_present_and_requires_uule_for_radius() {
        assert_eq!(
            GoogleJobsInput::default()
                .validate()
                .unwrap_err()
                .to_string(),
            "Job search query is required unless next_page_token is provided."
        );
        assert_eq!(
            GoogleJobsInput {
                q: Some("nurse".to_owned()),
                lrad: Some(json!(25)),
                ..GoogleJobsInput::default()
            }
            .validate()
            .unwrap_err()
            .to_string(),
            "Search radius (lrad) requires an encoded location (uule)."
        );
        assert!(GoogleJobsInput {
            next_page_token: Some("next".to_owned()),
            ..GoogleJobsInput::default()
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn builds_indeed_fallback_params_from_query_and_location() {
        let input = GoogleJobsInput {
            q: Some("nurse jobs in Austin".to_owned()),
            gl: Some("us".to_owned()),
            hl: Some("en".to_owned()),
            ..GoogleJobsInput::default()
        };
        assert_eq!(
            build_indeed_fallback_params(&input),
            vec![
                ("query".to_owned(), "nurse".to_owned()),
                ("limit".to_owned(), "10".to_owned()),
                ("location".to_owned(), "Austin".to_owned()),
                ("country".to_owned(), "US".to_owned()),
                ("gl".to_owned(), "us".to_owned()),
                ("hl".to_owned(), "en".to_owned()),
            ]
        );
    }

    #[test]
    fn keeps_full_fallback_query_without_location_phrase() {
        let input = GoogleJobsInput {
            q: Some("remote product manager".to_owned()),
            ..GoogleJobsInput::default()
        };
        assert_eq!(
            build_indeed_fallback_params(&input),
            vec![
                ("query".to_owned(), "remote product manager".to_owned()),
                ("limit".to_owned(), "10".to_owned()),
            ]
        );
    }
}
