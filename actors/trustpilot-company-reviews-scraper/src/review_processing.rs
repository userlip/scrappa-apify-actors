use anyhow::{Result, bail};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Map, Value};
use std::collections::HashSet;

const REVIEW_ARRAYS: &[&str] = &["reviews", "relevantReviews", "aiSummaryReviews"];

pub fn collect_reviews(response: &Value) -> Result<Vec<(Value, &'static str)>> {
    let mut seen = HashSet::new();
    let mut collected = Vec::new();
    for &source in REVIEW_ARRAYS {
        let Some(reviews) = response.get(source).filter(|value| !value.is_null()) else {
            continue;
        };
        let Some(reviews) = reviews.as_array() else {
            bail!("{source} must be an array");
        };
        for review in reviews {
            let Some(review_object) = review.as_object() else {
                bail!("reviews in {source} must be objects");
            };
            let id = review_object
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|id| !id.is_empty());
            if let Some(id) = id {
                if !seen.insert(id.to_owned()) {
                    continue;
                }
            }
            collected.push((review.clone(), source));
        }
    }
    Ok(collected)
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn published_date(review: &Value) -> Result<Option<Value>> {
    if let Some(value) = review
        .get("dates")
        .and_then(|dates| dates.get("publishedDate"))
        .filter(|value| is_truthy(value))
    {
        return Ok(Some(value.clone()));
    }

    let Some(created_at) = review.get("createdAt") else {
        return Ok(None);
    };
    if let Some(seconds) = created_at.as_f64() {
        let millis = (seconds * 1000.0).trunc();
        if !millis.is_finite() || millis.abs() > 8_640_000_000_000_000.0 {
            bail!("Invalid time value");
        }
        let Some(date) = DateTime::<Utc>::from_timestamp_millis(millis as i64) else {
            bail!("Invalid time value");
        };
        return Ok(Some(Value::String(
            date.to_rfc3339_opts(SecondsFormat::Millis, true),
        )));
    }
    if let Some(value) = created_at.as_str().filter(|value| !value.trim().is_empty()) {
        return Ok(Some(Value::String(value.to_owned())));
    }
    Ok(None)
}

fn request_value(params: &Map<String, Value>, key: &str) -> Value {
    params
        .get(key)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn enrich_review(
    review: &Value,
    request_params: &Map<String, Value>,
    response: &Value,
    review_source: &str,
) -> Result<Value> {
    let Some(mut enriched) = review.as_object().cloned() else {
        bail!("review must be an object");
    };
    enriched.remove("consumer_name");
    enriched.remove("published_date");
    enriched.insert("review_source".into(), Value::String(review_source.into()));
    if let Some(consumer_name) = review
        .get("consumer")
        .and_then(|consumer| consumer.get("displayName"))
    {
        enriched.insert("consumer_name".into(), consumer_name.clone());
    }
    if let Some(date) = published_date(review)? {
        enriched.insert("published_date".into(), date);
    }
    enriched.insert(
        "company_domain".into(),
        request_value(request_params, "company_domain"),
    );
    enriched.insert(
        "request_locale".into(),
        request_value(request_params, "locale"),
    );
    enriched.insert("request_page".into(), request_value(request_params, "page"));
    enriched.insert(
        "request_sort".into(),
        request_params
            .get("sort")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or_else(|| Value::String("recency".into())),
    );
    for key in ["rating", "verified", "with_replies", "query"] {
        enriched.insert(format!("request_{key}"), request_value(request_params, key));
    }
    enriched.insert(
        "request_date_posted".into(),
        request_params
            .get("date_posted")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or_else(|| Value::String("any".into())),
    );
    let pagination = response.get("pagination");
    for (output_key, input_key) in [
        ("page_total_count", "total_count"),
        ("page_total_pages", "total_pages"),
    ] {
        enriched.insert(
            output_key.into(),
            pagination
                .and_then(|pagination| pagination.get(input_key))
                .filter(|value| !value.is_null())
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    Ok(Value::Object(enriched))
}

#[cfg(test)]
mod tests {
    use super::{collect_reviews, enrich_review};
    use serde_json::json;

    #[test]
    fn preserves_review_array_order_and_enrichment() {
        let response = json!({
            "reviews": [
                {"id": "review-1", "consumer": {"displayName": "Jane Doe"}, "createdAt": 1},
                {"title": "Repeated title"}
            ],
            "relevantReviews": [
                {"id": " review-1 ", "title": "Duplicate"},
                {"title": "Repeated title"}
            ],
            "aiSummaryReviews": [{"id": "review-2"}],
            "pagination": {"total_count": 123, "total_pages": 7}
        });
        let collected = collect_reviews(&response).unwrap();
        assert_eq!(collected.len(), 4);
        assert_eq!(collected[0].1, "reviews");
        assert_eq!(collected[2].1, "relevantReviews");
        assert_eq!(collected[3].1, "aiSummaryReviews");

        let params = json!({
            "company_domain": "example.com",
            "locale": "en-US",
            "page": 2
        });
        let enriched = enrich_review(
            &collected[0].0,
            params.as_object().unwrap(),
            &response,
            collected[0].1,
        )
        .unwrap();
        assert_eq!(enriched["consumer_name"], "Jane Doe");
        assert_eq!(enriched["published_date"], "1970-01-01T00:00:01.000Z");
        assert_eq!(enriched["request_sort"], "recency");
        assert_eq!(enriched["request_date_posted"], "any");
        assert_eq!(enriched["request_rating"], json!(null));
        assert_eq!(enriched["page_total_count"], 123);
    }
}
