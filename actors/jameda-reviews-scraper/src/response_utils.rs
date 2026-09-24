use serde_json::{json, Map, Value};

use crate::request_params::{javascript_string, DoctorUrlFailure, RequestPlan};

pub fn get_reviews(response: &Value) -> Result<Vec<Value>, String> {
    if response.get("success") == Some(&Value::Bool(false)) {
        let message = response
            .get("message")
            .filter(|message| !message.is_null())
            .map(javascript_string)
            .unwrap_or_else(|| "Scrappa Jameda reviews request was not successful".to_owned());
        return Err(message);
    }
    Ok(response
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

pub fn build_review_dataset_item(
    review: &Value,
    input_doctor_url: &str,
    params: &Map<String, Value>,
    response: &Value,
) -> Result<Value, String> {
    if review.is_null() {
        return Err("Cannot read properties of null (reading 'id')".to_owned());
    }

    let mut item = review.as_object().cloned().unwrap_or_default();
    let pagination = nested_object(response, &["meta", "pagination"]);
    let doctor = nested_object(response, &["meta", "doctor"]);
    let review_id = nullish_value(review.get("id"));
    let review_text = nullish_value(review.get("text"));
    let rating = nullish_value(review.get("rating"));
    let date = nullish_value(review.get("date"));
    let date_formatted = nullish_value(review.get("date_formatted"));
    let verification_badge = nullish_value(review.get("verification_badge"));
    let categories = review
        .get("categories")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or_else(|| json!([]));

    item.insert("review_id".to_owned(), review_id);
    item.insert("review_text".to_owned(), review_text);
    item.insert("rating".to_owned(), rating.clone());
    item.insert("rating_number".to_owned(), decimal_number(&rating));
    item.insert("date".to_owned(), date);
    item.insert("date_formatted".to_owned(), date_formatted);
    item.insert("verification_badge".to_owned(), verification_badge);
    item.insert("categories".to_owned(), categories);
    item.insert(
        "doctor_name".to_owned(),
        first_non_empty_string(doctor.and_then(|doctor| doctor.get("name"))),
    );
    item.insert(
        "doctor_specializations".to_owned(),
        nullish_value(doctor.and_then(|doctor| doctor.get("specializations"))),
    );
    let doctor_overall_rating =
        nullish_value(doctor.and_then(|doctor| doctor.get("overall_rating")));
    item.insert(
        "doctor_overall_rating".to_owned(),
        doctor_overall_rating.clone(),
    );
    item.insert(
        "doctor_overall_rating_number".to_owned(),
        decimal_number(&doctor_overall_rating),
    );
    item.insert(
        "input_doctor_url".to_owned(),
        Value::String(input_doctor_url.to_owned()),
    );
    item.insert(
        "normalized_doctor_url".to_owned(),
        map_value(Some(params), "doctor_url"),
    );
    item.insert("request_page".to_owned(), map_value(Some(params), "page"));
    item.insert("request_sort".to_owned(), map_value(Some(params), "sort"));
    item.insert(
        "request_rating".to_owned(),
        map_value(Some(params), "rating"),
    );
    item.insert(
        "request_per_page".to_owned(),
        map_value(Some(params), "per_page"),
    );
    item.insert(
        "response_url".to_owned(),
        nested_value(response, &["meta", "url"]),
    );
    item.insert(
        "response_doctor_url".to_owned(),
        nested_value(response, &["meta", "doctor_url"]),
    );
    item.insert(
        "total_reviews".to_owned(),
        map_value(pagination, "totalReviews"),
    );
    item.insert(
        "total_pages".to_owned(),
        map_value(pagination, "totalPages"),
    );
    item.insert(
        "has_next_page".to_owned(),
        map_value(pagination, "hasNextPage"),
    );
    item.insert(
        "response_source".to_owned(),
        nested_value(response, &["meta", "source"]),
    );
    Ok(Value::Object(item))
}

pub fn build_output_summary(
    plan: &RequestPlan,
    saved_reviews: usize,
    failures: &[DoctorUrlFailure],
    status_message: Option<&str>,
) -> Value {
    json!({
        "request": {
            "endpoint": "/jameda/reviews",
            "doctor_urls": plan.doctor_urls,
        },
        "doctors_requested": plan.doctor_urls.len(),
        "reviews_saved": saved_reviews,
        "requests_failed": failures.len(),
        "status_message": status_message,
        "failures": failures.iter().map(|failure| json!({
            "doctor_url": failure.doctor_url,
            "error": failure.error,
        })).collect::<Vec<_>>(),
    })
}

fn decimal_number(value: &Value) -> Value {
    let number = match value {
        Value::Number(number) => number.as_f64(),
        Value::String(value) => {
            let normalized = value.trim().replacen(',', ".", 1);
            if normalized.is_empty() {
                None
            } else {
                normalized.parse::<f64>().ok()
            }
        }
        _ => None,
    };
    number
        .filter(|number| number.is_finite())
        .map(|number| json!(number))
        .unwrap_or(Value::Null)
}

fn first_non_empty_string(value: Option<&Value>) -> Value {
    match value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => Value::String(value.to_owned()),
        None => Value::Null,
    }
}

fn nullish_value(value: Option<&Value>) -> Value {
    value
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn nested_object<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Map<String, Value>> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_object()
}

fn nested_value(value: &Value, path: &[&str]) -> Value {
    let mut current = value;
    for key in path {
        let Some(next) = current.get(*key) else {
            return Value::Null;
        };
        current = next;
    }
    nullish_value(Some(current))
}

fn map_value(map: Option<&Map<String, Value>>, key: &str) -> Value {
    nullish_value(map.and_then(|map| map.get(key)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_params::{build_request_params, build_request_plan};
    use serde_json::json;

    const DOCTOR_URL: &str = "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin";

    #[test]
    fn enriches_reviews_with_doctor_request_and_pagination_data() {
        let response = json!({
            "success": true,
            "data": [{
                "id": "4986285",
                "text": "Sehr freundlicher und kompetenter Zahnarzt!",
                "rating": "4",
                "date": "2025-07-15T17:38:48+02:00",
                "date_formatted": "15. Juli 2025",
                "verification_badge": "Termin verifiziert",
                "categories": [{"label": "Behandlung", "rating": "5"}]
            }],
            "meta": {
                "url": DOCTOR_URL,
                "doctor_url": "markus-lietzau-msc/zahnarzt/berlin",
                "pagination": {
                    "currentPage": 1,
                    "totalPages": 44,
                    "totalReviews": 132,
                    "resultsPerPage": 3,
                    "hasNextPage": true,
                    "hasPreviousPage": false
                },
                "doctor": {
                    "name": " Markus Lietzau M.Sc. ",
                    "specializations": "Zahnarzt",
                    "overall_rating": "4,5"
                },
                "source": "jameda_ajax_api"
            }
        });
        let plan = build_request_plan(&json!({"doctor_url": DOCTOR_URL})).unwrap();
        let params = build_request_params(&plan, DOCTOR_URL);
        let item = build_review_dataset_item(&response["data"][0], DOCTOR_URL, &params, &response)
            .unwrap();

        assert_eq!(item["review_id"], "4986285");
        assert_eq!(
            item["review_text"],
            "Sehr freundlicher und kompetenter Zahnarzt!"
        );
        assert_eq!(item["rating_number"], 4.0);
        assert_eq!(item["categories"][0]["label"], "Behandlung");
        assert_eq!(item["doctor_name"], "Markus Lietzau M.Sc.");
        assert_eq!(item["doctor_overall_rating_number"], 4.5);
        assert_eq!(item["input_doctor_url"], DOCTOR_URL);
        assert_eq!(item["normalized_doctor_url"], DOCTOR_URL);
        assert_eq!(item["request_page"], 1);
        assert_eq!(item["request_sort"], Value::Null);
        assert_eq!(item["request_rating"], Value::Null);
        assert_eq!(item["request_per_page"], 20);
        assert_eq!(item["response_url"], DOCTOR_URL);
        assert_eq!(
            item["response_doctor_url"],
            "markus-lietzau-msc/zahnarzt/berlin"
        );
        assert_eq!(item["total_reviews"], 132);
        assert_eq!(item["total_pages"], 44);
        assert_eq!(item["has_next_page"], true);
        assert_eq!(item["response_source"], "jameda_ajax_api");
    }

    #[test]
    fn tolerates_sparse_review_data_and_omits_raw_response_from_output() {
        let response = json!({"data": [{"text": "Sparse review"}]});
        let plan = build_request_plan(&json!({"doctor_url": DOCTOR_URL})).unwrap();
        let params = build_request_params(&plan, DOCTOR_URL);
        let item = build_review_dataset_item(&response["data"][0], DOCTOR_URL, &params, &json!({}))
            .unwrap();
        assert_eq!(item["review_id"], Value::Null);
        assert_eq!(item["review_text"], "Sparse review");
        assert_eq!(item["rating_number"], Value::Null);
        assert_eq!(item["categories"], json!([]));
        assert_eq!(item["total_reviews"], Value::Null);

        let output = build_output_summary(&plan, 1, &[], None);
        assert_eq!(output["request"]["endpoint"], "/jameda/reviews");
        assert_eq!(output["reviews_saved"], 1);
        assert_eq!(output["requests_failed"], 0);
        assert!(output.get("responses").is_none());
    }

    #[test]
    fn rejects_unsuccessful_scrappa_responses_and_null_review_rows() {
        assert_eq!(
            get_reviews(&json!({"success": false, "message": "Doctor not found"})).unwrap_err(),
            "Doctor not found"
        );
        assert!(get_reviews(&json!({"success": false}))
            .unwrap_err()
            .contains("not successful"));
        assert!(
            build_review_dataset_item(&Value::Null, DOCTOR_URL, &Map::new(), &json!({})).is_err()
        );
        assert!(get_reviews(&json!({"success": true, "data": null}))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn preserves_original_review_fields_while_normalizing_common_fields() {
        let response = json!({"meta": {"doctor": {"name": ""}}});
        let params = Map::new();
        let item = build_review_dataset_item(
            &json!({"review_id": "old", "rating": "4,25", "extra": true}),
            DOCTOR_URL,
            &params,
            &response,
        )
        .unwrap();
        assert_eq!(item["review_id"], Value::Null);
        assert_eq!(item["rating_number"], 4.25);
        assert_eq!(item["extra"], true);
        assert_eq!(item["doctor_name"], Value::Null);
    }
}
