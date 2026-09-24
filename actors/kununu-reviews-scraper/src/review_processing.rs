use anyhow::{Result, bail};
use serde_json::{Map, Value, json};
use std::collections::HashSet;

use crate::request_params::KununuTarget;

fn nullish<'a>(first: Option<&'a Value>, second: Option<&'a Value>) -> Option<&'a Value> {
    first.filter(|value| !value.is_null()).or(second)
}

fn joined_review_text(review: &Value) -> Result<Value> {
    let Some(texts) = review.get("texts").and_then(Value::as_array) else {
        return Ok(Value::Null);
    };
    let mut parts = Vec::new();
    for text in texts {
        if text.is_null() {
            bail!("Cannot read properties of null (reading 'text')");
        }
        let Some(text) = text.get("text").and_then(Value::as_str) else {
            continue;
        };
        let text = text.trim();
        if !text.is_empty() {
            parts.push(text.to_owned());
        }
    }
    if parts.is_empty() {
        Ok(Value::Null)
    } else {
        Ok(Value::String(parts.join("\n\n")))
    }
}

pub fn collect_reviews(response: &Value) -> Vec<Value> {
    let Some(reviews) = response.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    reviews
        .iter()
        .filter(|review| {
            let Some(uuid) = review.get("uuid").and_then(Value::as_str) else {
                return true;
            };
            let uuid = uuid.trim();
            uuid.is_empty() || seen.insert(uuid.to_owned())
        })
        .cloned()
        .collect()
}

fn field_or_null(value: Option<&Value>) -> Value {
    value.cloned().unwrap_or(Value::Null)
}

pub fn enrich_review(
    review: &Value,
    target: &KununuTarget,
    request_params: &Map<String, Value>,
    response: &Value,
    include_raw_review: bool,
) -> Result<Value> {
    if review.is_null() {
        bail!("Cannot read properties of null (reading 'uuid')");
    }
    let company = review.get("company");
    let pagination = response.get("meta").and_then(|meta| meta.get("pagination"));
    let request = |key: &str| request_params.get(key);
    let review_type = nullish(review.get("type"), request("review_type"))
        .cloned()
        .unwrap_or_else(|| Value::String("employees".into()));
    let employment_status = nullish(review.get("employmentStatus"), review.get("jobStatus"));

    let mut item = Map::new();
    item.insert("review_id".into(), field_or_null(review.get("uuid")));
    item.insert("company_target".into(), Value::String(target.input.clone()));
    item.insert(
        "company_country".into(),
        Value::String(target.country.clone()),
    );
    item.insert(
        "company_slug".into(),
        Value::String(target.company_slug.clone()),
    );
    item.insert(
        "company_id".into(),
        target
            .company_id
            .as_ref()
            .map(|company_id| Value::String(company_id.clone()))
            .or_else(|| company.and_then(|company| company.get("uuid")).cloned())
            .unwrap_or(Value::Null),
    );
    item.insert(
        "company_name".into(),
        company
            .and_then(|company| company.get("name"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    item.insert("rating".into(), field_or_null(review.get("score")));
    item.insert(
        "rounded_rating".into(),
        field_or_null(review.get("roundedScore")),
    );
    item.insert("title".into(), field_or_null(review.get("title")));
    item.insert("text".into(), joined_review_text(review)?);
    item.insert("date".into(), field_or_null(review.get("createdAt")));
    item.insert("updated_at".into(), field_or_null(review.get("updatedAt")));
    item.insert("review_type".into(), review_type);
    item.insert(
        "reviewer_position".into(),
        field_or_null(review.get("position")),
    );
    item.insert(
        "reviewer_department".into(),
        field_or_null(review.get("department")),
    );
    item.insert(
        "reviewer_employment_type".into(),
        field_or_null(review.get("employmentType")),
    );
    item.insert(
        "reviewer_employment_status".into(),
        field_or_null(employment_status),
    );
    item.insert(
        "reviewer_current_employee".into(),
        field_or_null(review.get("isCurrentEmployee")),
    );
    item.insert(
        "reviewer_recommended".into(),
        field_or_null(review.get("isRecommended")),
    );
    item.insert("reviewer_city".into(), field_or_null(review.get("city")));
    item.insert(
        "reviewer_job_title".into(),
        field_or_null(review.get("jobTitle")),
    );
    item.insert("page".into(), field_or_null(request("page")));
    item.insert(
        "source_url".into(),
        Value::String(format!(
            "https://www.kununu.com/{}/{}",
            target.country, target.company_slug
        )),
    );
    for key in [
        "sort",
        "score_filters",
        "recommended_filters",
        "jobstatus_filters",
        "position_filters",
        "department_filters",
        "response_filters",
        "date_filters",
    ] {
        item.insert(format!("request_{key}"), field_or_null(request(key)));
    }
    item.insert(
        "page_total_results".into(),
        field_or_null(pagination.and_then(|pagination| pagination.get("totalResults"))),
    );
    item.insert(
        "page_total_pages".into(),
        field_or_null(pagination.and_then(|pagination| pagination.get("totalPages"))),
    );
    if include_raw_review {
        item.insert("raw_review".into(), review.clone());
    }
    Ok(Value::Object(item))
}

pub fn build_page_summary(
    target: &KununuTarget,
    page: u32,
    count: usize,
    response: &Value,
    include_raw_response: bool,
) -> Value {
    let mut summary = Map::new();
    summary.insert(
        "target".into(),
        Value::String(format!("{}/{}", target.country, target.company_slug)),
    );
    summary.insert("page".into(), json!(page));
    summary.insert("count".into(), json!(count));
    if let Some(pagination) = response.get("meta").and_then(|meta| meta.get("pagination")) {
        summary.insert("pagination".into(), pagination.clone());
    }
    if include_raw_response {
        summary.insert("response".into(), response.clone());
    }
    Value::Object(summary)
}

pub fn reported_total_pages(response: &Value) -> Option<f64> {
    let total_pages = response
        .get("meta")
        .and_then(|meta| meta.get("pagination"))?
        .get("totalPages")?;
    Some(match total_pages {
        Value::Null => 0.0,
        Value::Bool(value) => f64::from(u8::from(*value)),
        Value::Number(value) => value.as_f64().unwrap_or(f64::NAN),
        Value::String(value) => {
            if value.trim().is_empty() {
                0.0
            } else {
                value.trim().parse().unwrap_or(f64::NAN)
            }
        }
        Value::Array(values) if values.is_empty() => 0.0,
        Value::Array(values) if values.len() == 1 => match &values[0] {
            Value::Null => 0.0,
            Value::Number(value) => value.as_f64().unwrap_or(f64::NAN),
            Value::String(value) => value.trim().parse().unwrap_or(f64::NAN),
            Value::Bool(value) => f64::from(u8::from(*value)),
            _ => f64::NAN,
        },
        _ => f64::NAN,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_page_summary, collect_reviews, enrich_review, reported_total_pages};
    use crate::request_params::build_request_plan;
    use serde_json::json;

    #[test]
    fn collects_reviews_and_deduplicates_trimmed_uuids() {
        let reviews = collect_reviews(&json!({
            "data": [
                {"uuid": "review-1", "title": "First"},
                {"uuid": " review-1 ", "title": "Duplicate"},
                {"title": "Missing UUID"},
                {"uuid": "review-2", "title": "Second"}
            ]
        }));
        assert_eq!(reviews.len(), 3);
        assert_eq!(reviews[0]["title"], "First");
        assert_eq!(reviews[1]["title"], "Missing UUID");
        assert_eq!(reviews[2]["title"], "Second");
        assert!(collect_reviews(&json!({"success": true})).is_empty());
    }

    #[test]
    fn enriches_flat_rows_and_includes_raw_data_only_on_request() {
        let plan = build_request_plan(&json!({
            "targets": ["de/example-gmbh"],
            "sort": "newest",
            "score_filters": ["good"]
        }))
        .unwrap();
        let params = crate::request_params::page_params(&plan, &plan.targets[0], 2);
        let review = json!({
            "uuid": "review-1",
            "type": "employees",
            "title": "Good employer",
            "score": 4.2,
            "roundedScore": 4,
            "createdAt": "2026-05-01T12:00:00.000Z",
            "updatedAt": "2026-05-02T12:00:00.000Z",
            "texts": [{"id": "pros", "text": " Good team "}, {"id": "cons", "text": "Long meetings"}],
            "company": {"uuid": "company-1", "name": "Example GmbH"},
            "position": "employee",
            "department": "it",
            "employmentType": "full-time",
            "jobStatus": "current",
            "isCurrentEmployee": true,
            "isRecommended": true,
            "city": "Munich",
            "jobTitle": "Engineer",
            "ratings": [{"score": 4.5}]
        });
        let response = json!({"meta":{"pagination":{"totalResults":123,"totalPages":7}}});
        let item = enrich_review(&review, &plan.targets[0], &params, &response, false).unwrap();
        assert_eq!(item["review_id"], "review-1");
        assert_eq!(item["company_id"], "company-1");
        assert_eq!(item["company_name"], "Example GmbH");
        assert_eq!(item["rating"], 4.2);
        assert_eq!(item["rounded_rating"], 4);
        assert_eq!(item["text"], "Good team\n\nLong meetings");
        assert_eq!(item["reviewer_employment_status"], "current");
        assert_eq!(item["request_score_filters"], json!(["good"]));
        assert_eq!(item["page_total_results"], 123);
        assert_eq!(item["page_total_pages"], 7);
        assert!(item.get("ratings").is_none());
        assert!(item.get("raw_review").is_none());

        let item_with_raw =
            enrich_review(&review, &plan.targets[0], &params, &response, true).unwrap();
        assert_eq!(item_with_raw["raw_review"], review);
    }

    #[test]
    fn omits_missing_page_metadata_and_preserves_raw_response_option() {
        let plan = build_request_plan(&json!({"targets": ["bmwgroup"]})).unwrap();
        let summary = build_page_summary(&plan.targets[0], 1, 0, &json!({"success": true}), false);
        assert_eq!(summary, json!({"target":"de/bmwgroup","page":1,"count":0}));
        let raw = build_page_summary(&plan.targets[0], 1, 0, &json!({"success": true}), true);
        assert_eq!(raw["response"]["success"], true);
    }

    #[test]
    fn converts_pagination_total_pages_like_javascript_number() {
        assert_eq!(
            reported_total_pages(&json!({"meta":{"pagination":{"totalPages":"1"}}})),
            Some(1.0)
        );
        assert_eq!(
            reported_total_pages(&json!({"meta":{"pagination":{"totalPages":null}}})),
            Some(0.0)
        );
        assert!(
            reported_total_pages(&json!({"meta":{"pagination":{"totalPages":"unknown"}}}))
                .unwrap()
                .is_nan()
        );
        assert_eq!(reported_total_pages(&json!({})), None);
    }
}
