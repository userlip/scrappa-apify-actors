use crate::stepstone_input::StepstoneJobsInput;
use serde_json::{Value, json};

pub fn jobs(response: &Value) -> Vec<Value> {
    if let Some(jobs) = response.pointer("/data/jobs").and_then(Value::as_array) {
        return jobs.clone();
    }
    if let Some(jobs) = response.get("jobs").and_then(Value::as_array) {
        return jobs.clone();
    }

    eprintln!(
        "Unexpected Stepstone Jobs response shape: expected \"data.jobs\" or \"jobs\" array."
    );
    Vec::new()
}

pub fn pagination(response: &Value) -> Option<&Value> {
    response
        .pointer("/data/pagination")
        .filter(|value| !value.is_null())
        .or_else(|| response.get("pagination"))
}

pub fn metadata(response: &Value) -> Option<&Value> {
    response
        .pointer("/data/metadata")
        .filter(|value| !value.is_null())
        .or_else(|| response.get("metadata"))
}

pub fn company_name(company: Option<&Value>) -> Option<String> {
    let company = company?;
    if let Some(name) = company.as_str() {
        return Some(name.to_owned());
    }
    company
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

pub fn company_url(company: Option<&Value>) -> Option<String> {
    company?
        .get("url")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

pub fn formatted_location(location: Option<&Value>) -> Option<String> {
    let location = location?;
    if let Some(location) = location.as_str() {
        return Some(location.to_owned());
    }
    let object = location.as_object()?;

    if let Some(formatted) = object.get("formatted").and_then(Value::as_str) {
        if !formatted.trim().is_empty() {
            return Some(formatted.to_owned());
        }
    }

    let parts = ["city", "region", "country"]
        .into_iter()
        .filter_map(|key| object.get(key).and_then(Value::as_str))
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join(", "))
}

pub fn dataset_job(job: &Value) -> Value {
    let mut dataset_job = job.as_object().cloned().unwrap_or_default();
    let company = job.get("company");
    let location = job.get("location");
    let location_object = location.and_then(Value::as_object);

    dataset_job.insert(
        "company_name".to_owned(),
        company_name(company).map_or(Value::Null, Value::String),
    );
    dataset_job.insert(
        "company_url".to_owned(),
        company_url(company).map_or(Value::Null, Value::String),
    );
    dataset_job.insert(
        "location_formatted".to_owned(),
        formatted_location(location).map_or(Value::Null, Value::String),
    );
    for key in ["city", "region", "country"] {
        dataset_job.insert(
            format!("location_{key}"),
            location_object
                .and_then(|location| location.get(key))
                .cloned()
                .unwrap_or(Value::Null),
        );
    }

    Value::Object(dataset_job)
}

pub fn summary(response: &Value, input: &StepstoneJobsInput, jobs: &[Value]) -> Value {
    let pagination = pagination(response);
    let metadata = metadata(response);
    let first_job = jobs.first().map(|job| {
        json!({
            "title": nullish(job.get("title")).cloned().unwrap_or(Value::Null),
            "company": company_name(job.get("company")),
            "location": formatted_location(job.get("location")),
        })
    });

    json!({
        "jobs": jobs.len(),
        "has_more": pagination
            .and_then(|pagination| pagination.get("has_more"))
            .is_some_and(javascript_truthy),
        "next_page": pagination
            .and_then(|pagination| pagination.get("next_page"))
            .and_then(|value| nullish(Some(value)))
            .cloned()
            .unwrap_or(Value::Null),
        "total_jobs": pagination
            .and_then(|pagination| pagination.get("total_jobs"))
            .and_then(|value| nullish(Some(value)))
            .cloned()
            .unwrap_or_else(|| json!(jobs.len())),
        "query": metadata
            .and_then(|metadata| metadata.get("query"))
            .and_then(|value| nullish(Some(value)))
            .cloned()
            .unwrap_or_else(|| input.query().clone()),
        "country": metadata
            .and_then(|metadata| metadata.get("country"))
            .and_then(|value| nullish(Some(value)))
            .cloned()
            .unwrap_or_else(|| input.get("country").cloned().unwrap_or(Value::Null)),
        "first_job": first_job.unwrap_or(Value::Null),
    })
}

fn nullish(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| !value.is_null())
}

fn javascript_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_wrapped_jobs_and_falls_back_to_top_level_jobs() {
        let wrapped_jobs = vec![json!({"title": "Engineer"})];
        assert_eq!(
            jobs(&json!({"success": true, "data": {"jobs": wrapped_jobs}})),
            wrapped_jobs
        );

        let top_level_jobs = vec![json!({"title": "Nurse"})];
        assert_eq!(
            jobs(&json!({"data": {"metadata": {}}, "jobs": top_level_jobs})),
            top_level_jobs
        );
    }

    #[test]
    fn returns_no_jobs_for_unexpected_response_shapes() {
        assert!(jobs(&json!({"success": false})).is_empty());
    }

    #[test]
    fn extracts_company_and_location_labels() {
        assert_eq!(
            company_name(Some(&json!("Example GmbH"))).as_deref(),
            Some("Example GmbH")
        );
        assert_eq!(
            company_name(Some(&json!({"name": "Example Health"}))).as_deref(),
            Some("Example Health")
        );
        assert_eq!(
            company_url(Some(&json!({"url": "https://example.test/company"}))).as_deref(),
            Some("https://example.test/company")
        );
        assert_eq!(
            formatted_location(Some(&json!("Berlin"))).as_deref(),
            Some("Berlin")
        );
        assert_eq!(
            formatted_location(Some(&json!({"formatted": "Amsterdam"}))).as_deref(),
            Some("Amsterdam")
        );
        assert_eq!(
            formatted_location(Some(
                &json!({"city": "Brussels", "region": "Brussels-Capital", "country": "BE"})
            ))
            .as_deref(),
            Some("Brussels, Brussels-Capital, BE")
        );
        assert_eq!(
            formatted_location(Some(&json!({"formatted": "  ", "city": "Berlin"}))).as_deref(),
            Some("Berlin")
        );
    }

    #[test]
    fn adds_table_aliases_without_dropping_raw_job_fields() {
        let job = json!({
            "title": "Software Engineer",
            "company": {"name": "Example GmbH", "url": "https://www.stepstone.de/cmp/example"},
            "location": {"formatted": "Berlin", "city": "Berlin", "region": null, "country": "DE"},
            "salary": {"min": 70000}
        });

        assert_eq!(
            dataset_job(&job),
            json!({
                "title": "Software Engineer",
                "company": {"name": "Example GmbH", "url": "https://www.stepstone.de/cmp/example"},
                "location": {"formatted": "Berlin", "city": "Berlin", "region": null, "country": "DE"},
                "salary": {"min": 70000},
                "company_name": "Example GmbH",
                "company_url": "https://www.stepstone.de/cmp/example",
                "location_formatted": "Berlin",
                "location_city": "Berlin",
                "location_region": null,
                "location_country": "DE"
            })
        );
    }

    #[test]
    fn creates_the_same_pagination_and_first_result_summary() {
        let input =
            StepstoneJobsInput::normalize(Some(&json!({"query": "Nurse", "country": "at"})));
        let response = json!({
            "data": {
                "jobs": [{"title": "Nurse", "company": "Clinic", "location": "Vienna"}],
                "pagination": {"has_more": "yes", "next_page": 3, "total_jobs": 40},
                "metadata": {"query": "Nurse", "country": "at"}
            }
        });

        assert_eq!(
            summary(&response, &input, &jobs(&response)),
            json!({
                "jobs": 1,
                "has_more": true,
                "next_page": 3,
                "total_jobs": 40,
                "query": "Nurse",
                "country": "at",
                "first_job": {"title": "Nurse", "company": "Clinic", "location": "Vienna"}
            })
        );
    }

    #[test]
    fn falls_back_to_input_metadata_and_nulls_first_job_when_empty() {
        let input =
            StepstoneJobsInput::normalize(Some(&json!({"query": "Nurse", "country": "at"})));
        assert_eq!(
            summary(&json!({"pagination": {"next_page": null}}), &input, &[]),
            json!({
                "jobs": 0,
                "has_more": false,
                "next_page": null,
                "total_jobs": 0,
                "query": "Nurse",
                "country": "at",
                "first_job": null
            })
        );
    }

    #[test]
    fn falls_back_from_null_nested_pagination_and_metadata() {
        let response = json!({
            "data": {"pagination": null, "metadata": null},
            "pagination": {"next_page": 2},
            "metadata": {"country": "be"}
        });
        assert_eq!(pagination(&response), response.get("pagination"));
        assert_eq!(metadata(&response), response.get("metadata"));
    }
}
