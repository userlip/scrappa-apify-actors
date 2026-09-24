use serde_json::{json, Map, Value};

use crate::request_params::js_string;

pub fn extract_patent_search_data(response: &Value) -> &Value {
    if is_wrapped_response(response) {
        &response["data"]
    } else {
        response
    }
}

pub fn extract_patent_results(response: &Value) -> &[Value] {
    match extract_patent_search_data(response)
        .get("patents")
        .and_then(Value::as_array)
    {
        Some(patents) => patents,
        None => {
            eprintln!("Scrappa Google Patents response did not include a patents result array");
            &[]
        }
    }
}

pub fn limit_patent_search_response(response: &Value, patent_limit: usize) -> Value {
    let mut limited = response.clone();
    let data = if is_wrapped_response(response) {
        limited.get_mut("data")
    } else {
        Some(&mut limited)
    };
    if let Some(data) = data.and_then(Value::as_object_mut) {
        if let Some(Value::Array(patents)) = data.get_mut("patents") {
            patents.truncate(patent_limit);
        }
    }
    limited
}

pub fn enrich_patent_result(result: &Value, params: &Map<String, Value>) -> Value {
    let mut enriched = result.as_object().cloned().unwrap_or_default();
    let dates = result.get("dates").filter(|value| !value.is_null());
    let family_status = result
        .get("family_status")
        .filter(|value| !value.is_null())
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();

    enriched.insert(
        "patent_id".to_owned(),
        value_or_null(result.get("patent_id")),
    );
    enriched.insert("patent_page".to_owned(), patent_page_url(result));
    for field in [
        "rank",
        "title",
        "snippet",
        "publication_number",
        "language",
        "inventor",
        "assignee",
        "thumbnail",
        "pdf",
    ] {
        enriched.insert(field.to_owned(), value_or_null(result.get(field)));
    }
    for (output_field, date_field) in [
        ("priority_date", "priority"),
        ("filing_date", "filing"),
        ("grant_date", "grant"),
        ("publication_date", "publication"),
    ] {
        enriched.insert(
            output_field.to_owned(),
            value_or_null(dates.and_then(|dates| dates.get(date_field))),
        );
    }

    let family_countries = family_status
        .iter()
        .filter_map(|status| status.get("country").and_then(Value::as_str))
        .filter(|country| !country.is_empty())
        .collect::<Vec<_>>()
        .join(",");
    enriched.insert("family_status_count".to_owned(), json!(family_status.len()));
    enriched.insert(
        "family_countries".to_owned(),
        if family_countries.is_empty() {
            Value::Null
        } else {
            Value::String(family_countries)
        },
    );

    for field in [
        "q", "page", "num", "sort", "before", "after", "country", "language", "status", "type",
        "inventor", "assignee",
    ] {
        enriched.insert(format!("request_{field}"), value_or_null(params.get(field)));
    }

    Value::Object(enriched)
}

pub fn build_summary(data: &Value, upstream_result_count: usize, saved_results: &[Value]) -> Value {
    let with_pdf = saved_results
        .iter()
        .filter(|result| {
            result
                .get("pdf")
                .and_then(Value::as_str)
                .is_some_and(|pdf| !pdf.is_empty())
        })
        .count();
    let with_family_status = saved_results
        .iter()
        .filter(|result| {
            result
                .get("family_status")
                .and_then(Value::as_array)
                .is_some_and(|statuses| !statuses.is_empty())
        })
        .count();

    json!({
        "patent_results": saved_results.len(),
        "upstream_patent_results": upstream_result_count,
        "total_results": value_or_null(data.get("total_results")),
        "total_pages": value_or_null(data.get("total_pages")),
        "current_page": value_or_null(data.get("current_page")),
        "many_results": data.get("many_results").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Bool(false)),
        "cached": data.get("cached").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Bool(false)),
        "stale": data.get("stale").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Bool(false)),
        "with_pdf": with_pdf,
        "with_family_status": with_family_status,
    })
}

fn is_wrapped_response(response: &Value) -> bool {
    response.get("success").is_some_and(Value::is_boolean)
        && response.get("data").is_some_and(Value::is_object)
}

fn value_or_null(value: Option<&Value>) -> Value {
    value
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn patent_page_url(result: &Value) -> Value {
    let publication_number = result
        .get("publication_number")
        .filter(|value| !value.is_null())
        .map(js_string)
        .or_else(|| {
            result
                .get("patent_id")
                .and_then(Value::as_str)
                .and_then(publication_number_from_patent_id)
                .map(str::to_owned)
        });
    match publication_number.filter(|number| !number.is_empty()) {
        Some(number) => Value::String(format!("https://patents.google.com/patent/{number}")),
        None => Value::Null,
    }
}

fn publication_number_from_patent_id(patent_id: &str) -> Option<&str> {
    let mut parts = patent_id.split('/');
    if parts.next()? != "patent" {
        return None;
    }
    let publication_number = parts.next()?;
    let language = parts.next()?;
    if parts.next().is_some()
        || publication_number.is_empty()
        || language.len() != 2
        || !language.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return None;
    }
    Some(publication_number)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_wrapped_and_direct_responses_and_limits_only_the_patent_array() {
        let data = json!({"patents":[{"patent_id":"patent/US1/en"}],"total_results":1});
        let wrapped = json!({"success":true,"data":data,"trace_id":"abc"});
        assert_eq!(extract_patent_search_data(&wrapped), &wrapped["data"]);
        assert_eq!(extract_patent_search_data(&data), &data);
        assert_eq!(
            extract_patent_results(&wrapped),
            &[json!({"patent_id":"patent/US1/en"})]
        );
        assert_eq!(
            limit_patent_search_response(
                &json!({"success":true,"data":{"patents":[1,2,3],"total_results":3},"trace_id":"abc"}),
                2,
            ),
            json!({"success":true,"data":{"patents":[1,2],"total_results":3},"trace_id":"abc"})
        );
        assert_eq!(
            limit_patent_search_response(&json!({"patents":[1,2,3],"total_results":3}), 1),
            json!({"patents":[1],"total_results":3})
        );
    }

    #[test]
    fn builds_google_patents_urls_from_publication_numbers_or_ids() {
        assert_eq!(
            patent_page_url(&json!({"publication_number":"US123B1"})),
            json!("https://patents.google.com/patent/US123B1")
        );
        assert_eq!(
            patent_page_url(&json!({"patent_id":"patent/EP123A1/en"})),
            json!("https://patents.google.com/patent/EP123A1")
        );
        for patent_id in [
            "EP123A1",
            "patent/EP123A1",
            "patent/EP123A1/eng",
            "patent/EP123A1/en/extra",
        ] {
            assert_eq!(
                patent_page_url(&json!({"patent_id":patent_id})),
                Value::Null
            );
        }
    }

    #[test]
    fn enriches_results_without_dropping_upstream_fields() {
        let params =
            serde_json::from_value(json!({"q":"charging","page":1,"num":10,"status":"GRANT"}))
                .unwrap();
        let result = enrich_patent_result(
            &json!({
                "patent_id":"patent/US123B1/en",
                "rank":1,
                "title":"Charging system",
                "upstream_score":92,
                "dates":{"priority":"2020-01-01","filing":"2021-01-01","grant":"2024-01-01","publication":"2022-01-01"},
                "family_status":[{"country":"US","status":"ACTIVE"},{"country":"EP","status":"PENDING"}]
            }),
            &params,
        );
        assert_eq!(result["upstream_score"], 92);
        assert_eq!(
            result["patent_page"],
            "https://patents.google.com/patent/US123B1"
        );
        assert_eq!(result["priority_date"], "2020-01-01");
        assert_eq!(result["family_status_count"], 2);
        assert_eq!(result["family_countries"], "US,EP");
        assert_eq!(result["request_q"], "charging");
        assert_eq!(result["request_status"], "GRANT");
        assert_eq!(result["request_assignee"], Value::Null);

        let boundary = enrich_patent_result(
            &json!({"patent_id":"patent/US123B1/eng","family_status":[]}),
            &serde_json::from_value(json!({"q":"charging"})).unwrap(),
        );
        assert_eq!(boundary["patent_page"], Value::Null);
        assert_eq!(boundary["family_status_count"], 0);
        assert_eq!(boundary["family_countries"], Value::Null);
    }

    #[test]
    fn summarizes_only_saved_results_and_keeps_upstream_metadata() {
        let summary = build_summary(
            &json!({"total_results":12,"total_pages":2,"current_page":1,"many_results":true,"cached":false}),
            3,
            &[json!({"pdf":"https://pdf","family_status":[{}]})],
        );
        assert_eq!(summary["patent_results"], 1);
        assert_eq!(summary["upstream_patent_results"], 3);
        assert_eq!(summary["total_results"], 12);
        assert_eq!(summary["many_results"], true);
        assert_eq!(summary["with_pdf"], 1);
        assert_eq!(summary["with_family_status"], 1);
    }
}
