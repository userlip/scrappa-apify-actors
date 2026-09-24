use serde_json::{Map, Value};

use crate::params::GoogleJobsInput;

pub fn transform_indeed_fallback_response(
    response: &Value,
    input: &GoogleJobsInput,
    original_error: &str,
) -> Value {
    let empty_response = Value::Null;
    let fallback_response = if is_indeed_jobs_response(response) {
        response
    } else {
        &empty_response
    };
    let payload = fallback_response
        .get("data")
        .filter(|value| !value.is_null())
        .unwrap_or(fallback_response);
    let jobs = payload
        .get("jobs")
        .and_then(Value::as_array)
        .map(|jobs| {
            jobs.iter()
                .enumerate()
                .map(|(index, job)| transform_indeed_job(job, index))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut search_information = Map::new();
    if let Some(query) = &input.q {
        search_information.insert("query_displayed".to_owned(), Value::String(query.clone()));
    }
    search_information.insert("total_results".to_owned(), Value::from(jobs.len()));

    let mut transformed = Map::new();
    transformed.insert("jobs_results".to_owned(), Value::Array(jobs.clone()));
    transformed.insert("jobs".to_owned(), Value::Array(jobs));
    transformed.insert("filters".to_owned(), Value::Array(Vec::new()));
    transformed.insert(
        "search_information".to_owned(),
        Value::Object(search_information),
    );
    for field in ["pagination", "metadata"] {
        if let Some(value) = payload.get(field) {
            transformed.insert(field.to_owned(), value.clone());
        }
    }
    transformed.insert("service_used".to_owned(), Value::String("indeed".to_owned()));
    transformed.insert(
        "fallback_from".to_owned(),
        Value::String("google_jobs".to_owned()),
    );
    transformed.insert(
        "fallback_reason".to_owned(),
        Value::String(original_error.to_owned()),
    );
    Value::Object(transformed)
}

fn is_indeed_jobs_response(response: &Value) -> bool {
    response.is_object()
        && (response.get("data").is_some()
            || response
                .get("jobs")
                .and_then(Value::as_array)
                .is_some())
}

fn transform_indeed_job(job: &Value, index: usize) -> Value {
    let company_name = get_company_name(job.get("company"));
    let mut transformed = Map::new();
    transformed.insert("position".to_owned(), Value::from(index + 1));
    transformed.insert(
        "title".to_owned(),
        job.get("title")
            .filter(|title| !title.is_null())
            .cloned()
            .unwrap_or_else(|| Value::String("Unknown Title".to_owned())),
    );
    transformed.insert("company".to_owned(), Value::String(company_name.clone()));
    transformed.insert("company_name".to_owned(), Value::String(company_name));
    transformed.insert(
        "location".to_owned(),
        Value::String(get_location(job.get("location"))),
    );
    transformed.insert("via".to_owned(), Value::String("Indeed".to_owned()));
    transformed.insert(
        "description".to_owned(),
        Value::String(get_description(job)),
    );
    if let Some(link) = job.get("apply_url").and_then(Value::as_str) {
        transformed.insert("link".to_owned(), Value::String(link.to_owned()));
    }
    if let Some(job_id) = job.get("id").filter(|id| is_truthy(id)) {
        transformed.insert("job_id".to_owned(), Value::String(js_string(job_id)));
    }
    transformed.insert("extensions".to_owned(), Value::Array(build_extensions(job)));
    transformed.insert(
        "detected_extensions".to_owned(),
        serde_json::json!({ "source": "indeed_fallback" }),
    );
    if let Some(salary) = job.get("salary") {
        transformed.insert("salary".to_owned(), salary.clone());
    }
    Value::Object(transformed)
}

fn get_company_name(company: Option<&Value>) -> String {
    match company {
        Some(Value::String(name)) if !name.trim().is_empty() => name.clone(),
        Some(Value::Object(company)) => company
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("Unknown Company")
            .to_owned(),
        _ => "Unknown Company".to_owned(),
    }
}

fn get_location(location: Option<&Value>) -> String {
    match location {
        Some(Value::String(location)) => location.clone(),
        Some(Value::Object(location)) => {
            if let Some(formatted) = location
                .get("formatted")
                .and_then(Value::as_str)
                .filter(|formatted| !formatted.trim().is_empty())
            {
                return formatted.to_owned();
            }
            ["city", "state", "country"]
                .into_iter()
                .filter_map(|field| location.get(field).and_then(Value::as_str))
                .filter(|part| !part.trim().is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        }
        _ => String::new(),
    }
}

fn get_description(job: &Value) -> String {
    if let Some(description) = job
        .get("description")
        .and_then(Value::as_str)
        .filter(|description| !description.trim().is_empty())
    {
        return description.to_owned();
    }
    let Some(description_html) = job.get("description_html").and_then(Value::as_str) else {
        return String::new();
    };
    let without_tags = strip_html_tags(description_html);
    let decoded = without_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#039;", "'")
        .replace("&apos;", "'")
        .replace("&quot;", "\"");
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_html_tags(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(start) = remaining.find('<') {
        output.push_str(&remaining[..start]);
        let after_open = &remaining[start + 1..];
        let Some(end) = after_open.find('>') else {
            output.push_str(&remaining[start..]);
            return output;
        };
        if end == 0 {
            output.push('<');
            remaining = after_open;
            continue;
        }
        output.push(' ');
        remaining = &after_open[end + 1..];
    }
    output.push_str(remaining);
    output
}

fn build_extensions(job: &Value) -> Vec<Value> {
    let mut extensions = Vec::new();
    if let Some(date_published) = job
        .get("date_published")
        .and_then(Value::as_str)
        .filter(|date| !date.trim().is_empty())
    {
        extensions.push(Value::String(date_published.to_owned()));
    }
    if let Some(attributes) = job.get("attributes").and_then(Value::as_array) {
        extensions.extend(
            attributes
                .iter()
                .filter_map(Value::as_str)
                .filter(|attribute| !attribute.trim().is_empty())
                .map(|attribute| Value::String(attribute.to_owned())),
        );
    }
    extensions
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::normalize_jobs_input;
    use serde_json::json;

    #[test]
    fn transforms_nested_indeed_response_to_google_jobs_output() {
        let input = normalize_jobs_input(Some(json!({ "q": "nurse jobs in Austin" }))).unwrap();
        let transformed = transform_indeed_fallback_response(
            &json!({
                "success": true,
                "data": {
                    "jobs": [{
                        "id": "abc123",
                        "title": "Registered Nurse",
                        "company": { "name": "Example Health" },
                        "location": { "formatted": "Austin, TX" },
                        "description_html": "<p>Care &amp; coordinate shifts.</p>",
                        "date_published": "2026-05-01",
                        "attributes": ["Full-time", "Day shift"],
                        "apply_url": "https://example.com/apply",
                        "salary": { "min": 90_000 }
                    }],
                    "pagination": { "next_cursor": "cursor" },
                    "metadata": { "source": "indeed" }
                }
            }),
            &input,
            "Scrappa API error (504): Gateway Timeout",
        );
        assert_eq!(transformed["service_used"], "indeed");
        assert_eq!(transformed["fallback_from"], "google_jobs");
        assert_eq!(transformed["search_information"]["total_results"], 1);
        assert_eq!(transformed["pagination"]["next_cursor"], "cursor");
        assert_eq!(transformed["jobs_results"][0]["title"], "Registered Nurse");
        assert_eq!(transformed["jobs_results"][0]["company"], "Example Health");
        assert_eq!(transformed["jobs_results"][0]["company_name"], "Example Health");
        assert_eq!(transformed["jobs_results"][0]["via"], "Indeed");
        assert_eq!(
            transformed["jobs_results"][0]["description"],
            "Care & coordinate shifts."
        );
        assert_eq!(
            transformed["jobs_results"][0]["link"],
            "https://example.com/apply"
        );
        assert_eq!(transformed["jobs_results"][0]["salary"]["min"], 90_000);
        assert_eq!(
            transformed["jobs_results"][0]["extensions"],
            json!(["2026-05-01", "Full-time", "Day shift"])
        );
    }

    #[test]
    fn supports_top_level_responses_and_missing_job_fields() {
        let input = GoogleJobsInput::default();
        let transformed = transform_indeed_fallback_response(
            &json!({
                "jobs": [{ "id": "job-id", "company": "Example", "location": "Austin, TX" }]
            }),
            &input,
            "timeout",
        );
        assert_eq!(transformed["jobs_results"][0]["job_id"], "job-id");
        assert_eq!(transformed["jobs_results"][0]["title"], "Unknown Title");
        assert_eq!(transformed["jobs_results"][0]["company_name"], "Example");
        assert_eq!(transformed["jobs_results"][0]["location"], "Austin, TX");
        assert_eq!(transformed["fallback_reason"], "timeout");
    }

    #[test]
    fn handles_empty_or_unsuccessful_wrappers_as_empty_fallback_results() {
        for response in [Value::Null, json!({ "success": false, "message": "failed" })] {
            let transformed = transform_indeed_fallback_response(
                &response,
                &GoogleJobsInput::default(),
                "timeout",
            );
            assert_eq!(transformed["jobs_results"], json!([]));
            assert_eq!(transformed["search_information"]["total_results"], 0);
            assert!(transformed.get("metadata").is_none());
        }
    }

    #[test]
    fn preserves_plain_description_and_cleans_html_descriptions() {
        let plain = transform_indeed_fallback_response(
            &json!({
                "jobs": [{
                    "description": "Plain description wins.",
                    "description_html": "<p>HTML loses.</p>"
                }]
            }),
            &GoogleJobsInput::default(),
            "timeout",
        );
        assert_eq!(plain["jobs_results"][0]["description"], "Plain description wins.");

        let html = transform_indeed_fallback_response(
            &json!({
                "jobs": [{
                    "description_html": "<div>Use &lt;care&gt; &amp; coordinate &quot;shifts&quot; &apos;daily&apos;.</div>"
                }]
            }),
            &GoogleJobsInput::default(),
            "timeout",
        );
        assert_eq!(
            html["jobs_results"][0]["description"],
            "Use <care> & coordinate \"shifts\" 'daily'."
        );
    }

    #[test]
    fn filters_blank_attributes_and_keeps_fallback_pagination_without_a_google_token() {
        let transformed = transform_indeed_fallback_response(
            &json!({
                "data": {
                    "jobs": [{
                        "attributes": ["Full-time", "", null, 42, "Day shift"],
                        "date_published": ""
                    }],
                    "pagination": { "next_cursor": "indeed-cursor", "page": 1 }
                }
            }),
            &GoogleJobsInput::default(),
            "timeout",
        );
        assert_eq!(
            transformed["jobs_results"][0]["extensions"],
            json!(["Full-time", "Day shift"])
        );
        assert_eq!(transformed["pagination"]["next_cursor"], "indeed-cursor");
        assert!(transformed.get("next_page_token").is_none());
    }
}
