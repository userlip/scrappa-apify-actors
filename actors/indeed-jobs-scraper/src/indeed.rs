use serde_json::{json, Map, Value};
use url::{form_urlencoded, Url};

pub const DEFAULT_QUERY: &str = "software engineer";
pub const DEFAULT_LOCATION: &str = "New York";
pub const DEFAULT_COUNTRY: &str = "US";
pub const DEFAULT_LIMIT: u64 = 20;

const INPUT_FIELDS: [&str; 11] = [
    "query",
    "location",
    "country",
    "radius",
    "radius_unit",
    "job_type",
    "sort",
    "limit",
    "cursor",
    "hl",
    "gl",
];

#[derive(Clone, Debug)]
pub struct IndeedJobsInput {
    values: Map<String, Value>,
}

impl IndeedJobsInput {
    pub fn query(&self) -> Option<&Value> {
        self.values.get("query")
    }

    pub fn query_for_log(&self) -> String {
        self.query().map(js_string).unwrap_or_default()
    }

    pub fn params(&self) -> Vec<(String, Value)> {
        INPUT_FIELDS
            .iter()
            .filter_map(|key| {
                self.values
                    .get(*key)
                    .cloned()
                    .map(|value| ((*key).to_owned(), value))
            })
            .collect()
    }
}

pub fn normalize_input(input: &Value) -> IndeedJobsInput {
    let mut normalized = Map::new();
    if let Some(fields) = input.as_object() {
        for key in INPUT_FIELDS {
            let Some(value) = fields.get(key) else {
                continue;
            };
            if value.is_null() {
                continue;
            }

            let value = if let Some(string) = value.as_str() {
                let trimmed = string.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let normalized = match key {
                    "country" | "gl" | "radius_unit" => trimmed.to_uppercase(),
                    "hl" => trimmed.to_lowercase(),
                    _ => trimmed.to_owned(),
                };
                Value::String(normalized)
            } else {
                value.clone()
            };
            normalized.insert(key.to_owned(), value);
        }
    }

    if normalized.is_empty() {
        normalized.insert("query".to_owned(), json!(DEFAULT_QUERY));
        normalized.insert("location".to_owned(), json!(DEFAULT_LOCATION));
        normalized.insert("country".to_owned(), json!(DEFAULT_COUNTRY));
        normalized.insert("limit".to_owned(), json!(DEFAULT_LIMIT));
    } else {
        normalized
            .entry("query".to_owned())
            .or_insert_with(|| json!(DEFAULT_QUERY));
        normalized
            .entry("location".to_owned())
            .or_insert_with(|| json!(DEFAULT_LOCATION));
        normalized
            .entry("country".to_owned())
            .or_insert_with(|| json!(DEFAULT_COUNTRY));
        normalized
            .entry("limit".to_owned())
            .or_insert_with(|| json!(DEFAULT_LIMIT));
    }

    IndeedJobsInput { values: normalized }
}

pub fn build_jobs_url(base_url: &Url, params: &[(String, Value)]) -> anyhow::Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("Scrappa API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(["indeed", "jobs"]);

    let mut query = form_urlencoded::Serializer::new(String::new());
    for (key, value) in params {
        if value.is_null() || value == "" || value == false {
            continue;
        }
        if value == true {
            query.append_pair(key, "1");
        } else {
            query.append_pair(key, &js_string(value));
        }
    }
    let query = query.finish();
    if !query.is_empty() {
        url.set_query(Some(&query));
    }
    Ok(url)
}

pub fn get_jobs(response: &Value) -> Vec<Value> {
    if let Some(jobs) = response
        .get("data")
        .and_then(|data| data.get("jobs"))
        .and_then(Value::as_array)
    {
        return jobs.clone();
    }
    if let Some(jobs) = response.get("jobs").and_then(Value::as_array) {
        return jobs.clone();
    }

    eprintln!("Unexpected Indeed Jobs response shape: expected \"data.jobs\" or \"jobs\" array.");
    Vec::new()
}

pub fn to_dataset_job(job: &Value) -> Value {
    let mut dataset_job = job.as_object().cloned().unwrap_or_default();
    dataset_job.insert(
        "company_name".to_owned(),
        company_name(job.get("company")).map_or(Value::Null, Value::String),
    );
    dataset_job.insert(
        "location_formatted".to_owned(),
        formatted_location(job.get("location")).map_or(Value::Null, Value::String),
    );
    Value::Object(dataset_job)
}

pub fn pagination(response: &Value) -> Option<&Value> {
    response
        .get("data")
        .and_then(|data| data.get("pagination"))
        .filter(|value| !value.is_null())
        .or_else(|| response.get("pagination").filter(|value| !value.is_null()))
}

pub fn metadata(response: &Value) -> Option<&Value> {
    response
        .get("data")
        .and_then(|data| data.get("metadata"))
        .filter(|value| !value.is_null())
        .or_else(|| response.get("metadata").filter(|value| !value.is_null()))
}

pub fn results_summary(response: &Value, jobs: &[Value]) -> Value {
    let page = pagination(response);
    let meta = metadata(response);
    let first_job = jobs.first().map(|job| {
        json!({
            "title": job.get("title").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
            "company": company_name(job.get("company")).map_or(Value::Null, Value::String),
            "location": formatted_location(job.get("location")).map_or(Value::Null, Value::String),
        })
    });

    json!({
        "jobs": jobs.len(),
        "has_more": page.and_then(|value| value.get("has_more")).is_some_and(js_truthy),
        "next_cursor": page
            .and_then(|value| value.get("next_cursor"))
            .filter(|value| js_truthy(value))
            .map(|_| json!("present"))
            .unwrap_or(Value::Null),
        "total_results": meta
            .and_then(|value| value.get("total_results"))
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or_else(|| json!(jobs.len())),
        "first_job": first_job.unwrap_or(Value::Null),
    })
}

fn company_name(company: Option<&Value>) -> Option<String> {
    match company? {
        Value::String(name) => Some(name.clone()),
        Value::Object(company) => company
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned),
        _ => None,
    }
}

fn formatted_location(location: Option<&Value>) -> Option<String> {
    match location? {
        Value::String(location) => Some(location.clone()),
        Value::Object(location) => {
            if let Some(formatted) = location
                .get("formatted")
                .and_then(Value::as_str)
                .filter(|formatted| !formatted.trim().is_empty())
            {
                return Some(formatted.to_owned());
            }

            let parts = ["city", "state", "country"]
                .iter()
                .filter_map(|field| {
                    location
                        .get(*field)
                        .and_then(Value::as_str)
                        .filter(|part| !part.trim().is_empty())
                })
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join(", "))
        }
        _ => None,
    }
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_none_or(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
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
                _ => js_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_unknown_inputs_use_the_prefill_defaults() {
        let expected = json!({
            "query": DEFAULT_QUERY,
            "location": DEFAULT_LOCATION,
            "country": DEFAULT_COUNTRY,
            "limit": DEFAULT_LIMIT
        });
        assert_eq!(
            Value::Object(normalize_input(&Value::Null).values),
            expected
        );
        assert_eq!(
            Value::Object(normalize_input(&json!({"unknown": "placeholder"})).values),
            expected
        );
    }

    #[test]
    fn normalizes_strings_and_applies_defaults_to_partial_input() {
        let input = normalize_input(&json!({
            "query": "  nurse  ",
            "location": "Berlin",
            "country": "de",
            "hl": " EN ",
            "gl": " us ",
            "radius_unit": "kilometers",
            "cursor": " "
        }));
        assert_eq!(
            Value::Object(input.values),
            json!({
                "query": "nurse",
                "location": "Berlin",
                "country": "DE",
                "hl": "en",
                "gl": "US",
                "radius_unit": "KILOMETERS",
                "limit": DEFAULT_LIMIT
            })
        );
    }

    #[test]
    fn builds_the_complete_scrappa_query_in_input_field_order() {
        let input = normalize_input(&json!({
            "query": "software engineer",
            "location": "New York",
            "country": "us",
            "radius": 25,
            "radius_unit": "miles",
            "job_type": "full_time",
            "sort": "date",
            "limit": 20,
            "cursor": "next cursor",
            "hl": "EN",
            "gl": "de"
        }));
        let url = build_jobs_url(
            &Url::parse("https://scrappa.co/api").unwrap(),
            &input.params(),
        )
        .unwrap();
        assert_eq!(
            url.query_pairs().collect::<Vec<_>>(),
            vec![
                ("query".into(), "software engineer".into()),
                ("location".into(), "New York".into()),
                ("country".into(), "US".into()),
                ("radius".into(), "25".into()),
                ("radius_unit".into(), "MILES".into()),
                ("job_type".into(), "full_time".into()),
                ("sort".into(), "date".into()),
                ("limit".into(), "20".into()),
                ("cursor".into(), "next cursor".into()),
                ("hl".into(), "en".into()),
                ("gl".into(), "DE".into()),
            ]
        );
    }

    #[test]
    fn filters_empty_parameters_and_encodes_true_as_one() {
        let params = vec![
            ("empty".to_owned(), json!("")),
            ("missing".to_owned(), Value::Null),
            ("disabled".to_owned(), json!(false)),
            ("enabled".to_owned(), json!(true)),
        ];
        let url = build_jobs_url(&Url::parse("https://scrappa.co/api").unwrap(), &params).unwrap();
        assert_eq!(
            url.query_pairs().collect::<Vec<_>>(),
            vec![("enabled".into(), "1".into())]
        );
    }

    #[test]
    fn reads_wrapped_and_top_level_jobs_and_falls_back_when_needed() {
        let wrapped = json!({"data": {"jobs": [{"title": "Engineer"}]}});
        assert_eq!(get_jobs(&wrapped), vec![json!({"title": "Engineer"})]);

        let top_level = json!({
            "data": {"metadata": {"total_results": 1}},
            "jobs": [{"title": "Nurse"}]
        });
        assert_eq!(get_jobs(&top_level), vec![json!({"title": "Nurse"})]);
        assert!(get_jobs(&json!({"success": false})).is_empty());
    }

    #[test]
    fn keeps_job_fields_and_adds_company_and_location_aliases() {
        assert_eq!(
            to_dataset_job(&json!({
                "id": "job-1",
                "company": {"name": "Example Corp"},
                "location": {"city": "Berlin", "country": "DE"}
            })),
            json!({
                "id": "job-1",
                "company": {"name": "Example Corp"},
                "company_name": "Example Corp",
                "location": {"city": "Berlin", "country": "DE"},
                "location_formatted": "Berlin, DE"
            })
        );
    }

    #[test]
    fn summary_supports_wrapped_metadata_and_cursor_values() {
        let response = json!({
            "data": {
                "pagination": {"has_more": [], "next_cursor": "next"},
                "metadata": {"total_results": 42}
            }
        });
        assert_eq!(
            results_summary(
                &response,
                &[json!({
                    "title": "Engineer",
                    "company": "Example",
                    "location": {"formatted": "Remote"}
                })]
            ),
            json!({
                "jobs": 1,
                "has_more": true,
                "next_cursor": "present",
                "total_results": 42,
                "first_job": {
                    "title": "Engineer",
                    "company": "Example",
                    "location": "Remote"
                }
            })
        );
    }
}
