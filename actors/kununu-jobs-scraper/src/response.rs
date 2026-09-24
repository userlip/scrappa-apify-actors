use serde_json::{json, Map, Value};

pub fn get_jobs(response: &Value) -> Result<Vec<Value>, String> {
    if response.get("success") == Some(&Value::Bool(false)) {
        let message = response
            .get("message")
            .and_then(|value| nonempty_string(Some(value)))
            .or_else(|| {
                response
                    .get("error")
                    .and_then(|value| nonempty_string(Some(value)))
            })
            .unwrap_or("Scrappa returned success=false for Kununu Jobs.");
        return Err(message.to_owned());
    }

    let data = response.get("data");
    let jobs = data
        .and_then(Value::as_array)
        .or_else(|| {
            data.and_then(|data| data.get("jobs"))
                .and_then(Value::as_array)
        })
        .or_else(|| {
            data.and_then(|data| data.get("results"))
                .and_then(Value::as_array)
        })
        .or_else(|| response.get("jobs").and_then(Value::as_array))
        .or_else(|| response.get("results").and_then(Value::as_array));

    match jobs {
        Some(jobs) => Ok(jobs.clone()),
        None => {
            eprintln!("Unexpected Kununu Jobs response shape: expected \"data.jobs\", \"data.results\", \"jobs\", or \"results\" array.");
            Ok(Vec::new())
        }
    }
}

pub fn get_pagination(response: &Value) -> Option<Value> {
    let data_is_array = response.get("data").is_some_and(Value::is_array);
    if data_is_array {
        return nullish(response.get("pagination"), meta_pagination(response)).cloned();
    }

    response
        .get("data")
        .and_then(|data| data.get("pagination"))
        .filter(|value| !value.is_null())
        .or_else(|| response.get("pagination").filter(|value| !value.is_null()))
        .or_else(|| meta_pagination(response))
        .cloned()
}

pub fn last_page(pagination: Option<&Value>) -> Option<i64> {
    ["lastPage", "last_page", "totalPages", "total_pages"]
        .iter()
        .find_map(|key| {
            let value = pagination?.get(*key)?;
            let number = value.as_f64()?;
            (number.is_finite() && number.fract() == 0.0).then_some(number as i64)
        })
}

pub fn to_dataset_job(job: &Value, include_raw_job: bool) -> Value {
    let mut dataset_job = Map::new();
    if !include_raw_job {
        if let Some(fields) = job.as_object() {
            for (key, value) in fields {
                if !["company", "employment_types", "date_posted", "posted_at"]
                    .contains(&key.as_str())
                {
                    dataset_job.insert(key.clone(), value.clone());
                }
            }
        }
    }

    let company_value = job.get("company").filter(|value| !value.is_null());
    let company = company_parts(company_value, job);
    let location = job.get("location");
    let job_url = nonempty_string(nullish(job.get("url"), job.get("link")));
    let date_posted = nonempty_string(nullish(
        job.get("date_posted"),
        nullish(job.get("posted_at"), job.get("postedAt")),
    ));
    let posted_at = nonempty_string(nullish(
        job.get("posted_at"),
        nullish(job.get("postedAt"), job.get("date_posted")),
    ));
    let employment_types = nullish(job.get("employment_types"), job.get("employmentTypes"));

    dataset_job.insert("title".into(), value_or_null(job.get("title")));
    dataset_job.insert(
        "job_id".into(),
        value_or_null(nullish(job.get("id"), job.get("uuid"))),
    );
    dataset_job.insert(
        "job_url".into(),
        job_url.map_or(Value::Null, |value| Value::String(value.to_owned())),
    );
    dataset_job.insert("company".into(), value_or_null(company_value));
    dataset_job.insert(
        "company_name".into(),
        company.name.map_or(Value::Null, Value::String),
    );
    dataset_job.insert(
        "company_slug".into(),
        company.slug.map_or(Value::Null, Value::String),
    );
    dataset_job.insert(
        "company_url".into(),
        company.url.map_or(Value::Null, Value::String),
    );
    dataset_job.insert("company_score".into(), company.score.unwrap_or(Value::Null));
    dataset_job.insert(
        "company_is_top_company".into(),
        company.is_top_company.unwrap_or(Value::Null),
    );
    dataset_job.insert(
        "location_formatted".into(),
        formatted_location(location, job).map_or(Value::Null, Value::String),
    );
    dataset_job.insert("location_city".into(), location_part(location, job, "city"));
    dataset_job.insert(
        "location_region".into(),
        location_part(location, job, "region"),
    );
    dataset_job.insert(
        "location_country".into(),
        location_part(location, job, "country"),
    );
    dataset_job.insert("employment_types".into(), value_or_null(employment_types));
    dataset_job.insert(
        "date_posted".into(),
        date_posted.map_or(Value::Null, |value| Value::String(value.to_owned())),
    );
    dataset_job.insert(
        "posted_at".into(),
        posted_at.map_or(Value::Null, |value| Value::String(value.to_owned())),
    );

    if include_raw_job {
        dataset_job.insert("raw_job".into(), job.clone());
    }
    Value::Object(dataset_job)
}

pub fn company_name(company: Option<&Value>, job: &Value) -> Option<String> {
    company_parts(company, job).name
}

pub fn formatted_location(location: Option<&Value>, job: &Value) -> Option<String> {
    if let Some(location) = location.and_then(Value::as_str) {
        return Some(location.to_owned());
    }

    if let Some(location) = location.and_then(Value::as_object) {
        if let Some(formatted) = location
            .get("formatted")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            return Some(formatted.to_owned());
        }
    }

    let parts = [
        location_part_as_string(location, job, "city"),
        location_part_as_string(location, job, "region"),
        location_part_as_string(location, job, "country"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn meta_pagination(response: &Value) -> Option<&Value> {
    response.get("meta").and_then(|meta| meta.get("pagination"))
}

fn nullish<'a>(first: Option<&'a Value>, fallback: Option<&'a Value>) -> Option<&'a Value> {
    first.filter(|value| !value.is_null()).or(fallback)
}

fn value_or_null(value: Option<&Value>) -> Value {
    value.cloned().unwrap_or(Value::Null)
}

fn nonempty_string(value: Option<&Value>) -> Option<&str> {
    value?.as_str().filter(|value| !value.trim().is_empty())
}

#[derive(Default)]
struct CompanyParts {
    name: Option<String>,
    slug: Option<String>,
    url: Option<String>,
    score: Option<Value>,
    is_top_company: Option<Value>,
}

fn company_parts(company: Option<&Value>, job: &Value) -> CompanyParts {
    match company {
        Some(Value::String(company)) => CompanyParts {
            name: Some(company.clone()),
            score: number(nullish(job.get("company_score"), job.get("kununu_score"))),
            is_top_company: boolean(nullish(
                job.get("is_top_company"),
                nullish(job.get("isTopCompany"), job.get("top_company")),
            )),
            ..CompanyParts::default()
        },
        Some(Value::Object(company)) => CompanyParts {
            name: clean_string(nullish(company.get("name"), job.get("company_name"))),
            slug: clean_string(company.get("slug")),
            url: clean_string(nullish(company.get("url"), company.get("website"))),
            score: number(nullish(
                company.get("kununu_score"),
                nullish(
                    company.get("score"),
                    nullish(
                        company.get("rating"),
                        nullish(job.get("company_score"), job.get("kununu_score")),
                    ),
                ),
            )),
            is_top_company: boolean(nullish(
                company.get("is_top_company"),
                nullish(
                    company.get("isTopCompany"),
                    nullish(
                        company.get("top_company"),
                        nullish(
                            job.get("is_top_company"),
                            nullish(job.get("isTopCompany"), job.get("top_company")),
                        ),
                    ),
                ),
            )),
        },
        _ => CompanyParts {
            name: clean_string(job.get("company_name")),
            score: number(nullish(job.get("company_score"), job.get("kununu_score"))),
            is_top_company: boolean(nullish(
                job.get("is_top_company"),
                nullish(job.get("isTopCompany"), job.get("top_company")),
            )),
            ..CompanyParts::default()
        },
    }
}

fn clean_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn number(value: Option<&Value>) -> Option<Value> {
    let value = value?.as_f64()?;
    if !value.is_finite() {
        return None;
    }
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        Some(json!(value as i64))
    } else {
        Some(json!(value))
    }
}

fn boolean(value: Option<&Value>) -> Option<Value> {
    value.and_then(Value::as_bool).map(Value::Bool)
}

fn location_part(location: Option<&Value>, job: &Value, part: &str) -> Value {
    location_part_as_string(location, job, part).map_or(Value::Null, Value::String)
}

fn location_part_as_string(location: Option<&Value>, job: &Value, part: &str) -> Option<String> {
    let job_key = if part == "country" {
        "countryCode"
    } else {
        part
    };
    let value = nullish(
        location.and_then(|location| location.get(part)),
        job.get(job_key),
    );
    clean_string(value)
}

#[cfg(test)]
mod tests {
    use super::{
        company_name, formatted_location, get_jobs, get_pagination, last_page, to_dataset_job,
    };
    use serde_json::json;

    #[test]
    fn finds_jobs_in_supported_response_shapes_and_surfaces_failures() {
        let jobs = vec![json!({"title":"Software Engineer"})];
        for response in [
            json!({"data":{"jobs":jobs}}),
            json!({"data":{"results":jobs}}),
            json!({"data":jobs}),
            json!({"jobs":jobs}),
            json!({"results":jobs}),
        ] {
            assert_eq!(get_jobs(&response).unwrap(), jobs);
        }

        assert_eq!(
            get_jobs(&json!({"success":false,"message":"Invalid Kununu request"})).unwrap_err(),
            "Invalid Kununu request"
        );
        assert_eq!(
            get_jobs(&json!({"success":false,"error":{"message":"ignored"}})).unwrap_err(),
            "Scrappa returned success=false for Kununu Jobs."
        );
    }

    #[test]
    fn reads_wrapped_and_top_level_pagination() {
        assert_eq!(
            get_pagination(&json!({"data":{"pagination":{"page":2}}})),
            Some(json!({"page":2}))
        );
        assert_eq!(
            get_pagination(&json!({"data":[],"meta":{"pagination":{"page":3}}})),
            Some(json!({"page":3}))
        );
        assert_eq!(last_page(Some(&json!({"last_page":4}))), Some(4));
        assert_eq!(last_page(Some(&json!({"last_page":"4"}))), None);
    }

    #[test]
    fn extracts_company_names_and_formats_locations() {
        assert_eq!(
            company_name(Some(&json!("Example GmbH")), &json!({})).as_deref(),
            Some("Example GmbH")
        );
        assert_eq!(
            company_name(Some(&json!({"name":"Example Health"})), &json!({})).as_deref(),
            Some("Example Health")
        );
        assert_eq!(
            company_name(None, &json!({"company_name":"Fallback AG"})).as_deref(),
            Some("Fallback AG")
        );
        assert_eq!(
            formatted_location(Some(&json!("Berlin")), &json!({})).as_deref(),
            Some("Berlin")
        );
        assert_eq!(
            formatted_location(Some(&json!({"formatted":"Zurich"})), &json!({})).as_deref(),
            Some("Zurich")
        );
        assert_eq!(
            formatted_location(
                Some(&json!({"city":"Vienna","region":"Vienna","country":"AT"})),
                &json!({})
            )
            .as_deref(),
            Some("Vienna, Vienna, AT")
        );
        assert_eq!(
            formatted_location(
                None,
                &json!({"city":"Berlin","region":"Berlin","countryCode":"DE"})
            )
            .as_deref(),
            Some("Berlin, Berlin, DE")
        );
        assert_eq!(formatted_location(None, &json!({})), None);
    }

    #[test]
    fn maps_job_aliases_and_optional_raw_payload() {
        let job = json!({
            "id":"job-1",
            "title":"Software Engineer",
            "url":"https://www.kununu.com/de/example/jobs/job-1",
            "company":{
                "name":"Example GmbH",
                "slug":"example",
                "url":"https://www.kununu.com/de/example",
                "kununu_score":4.4,
                "is_top_company":true
            },
            "location":{"formatted":"Berlin","city":"Berlin","region":null,"country":"DE"}
        });
        let mapped = to_dataset_job(&job, true);
        assert_eq!(mapped["job_id"], "job-1");
        assert_eq!(mapped["job_url"], job["url"]);
        assert_eq!(mapped["company_name"], "Example GmbH");
        assert_eq!(mapped["company_slug"], "example");
        assert_eq!(mapped["company_score"], 4.4);
        assert_eq!(mapped["company_is_top_company"], true);
        assert_eq!(mapped["location_formatted"], "Berlin");
        assert_eq!(mapped["location_country"], "DE");
        assert_eq!(mapped["raw_job"], job);
        assert!(to_dataset_job(&job, false).get("raw_job").is_none());
    }

    #[test]
    fn maps_live_camel_case_fields_and_merges_partial_locations() {
        let job = json!({
            "id":"5ffd841a-edbd-43c5-b923-89df0e02534b",
            "title":"Senior Software Engineer",
            "url":"https://www.kununu.com/job-postings/de/5ffd841a-edbd-43c5-b923-89df0e02534b",
            "postedAt":"2026-07-03",
            "city":"Berlin",
            "region":"Berlin",
            "stateCode":"DE-BE",
            "employmentTypes":["JOB_EMPLOYMENT_FULLTIME"],
            "company":{"name":"FindYou Consulting GmbH","slug":"findyou-consulting","website":"https://www.findyou.de","score":5,"isTopCompany":true}
        });
        let mapped = to_dataset_job(&job, false);
        assert_eq!(mapped["company_url"], "https://www.findyou.de");
        assert_eq!(mapped["company_score"], 5);
        assert_eq!(mapped["company_is_top_company"], true);
        assert_eq!(mapped["location_formatted"], "Berlin, Berlin");
        assert_eq!(
            mapped["employment_types"],
            json!(["JOB_EMPLOYMENT_FULLTIME"])
        );
        assert_eq!(mapped["date_posted"], "2026-07-03");
        assert_eq!(mapped["posted_at"], "2026-07-03");

        let partial = to_dataset_job(
            &json!({
                "id":"job-2","city":"Berlin","region":"Berlin","countryCode":"DE",
                "location":{"city":null,"region":null,"country":null}
            }),
            false,
        );
        assert_eq!(partial["location_formatted"], "Berlin, Berlin, DE");
        assert_eq!(partial["location_country"], "DE");
    }
}
