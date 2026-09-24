use crate::request_params::TrustedShopsTarget;
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{Map, Value, json};
use std::collections::HashSet;

pub fn collect_reviews(response: &Value) -> Vec<Value> {
    let data = response.get("data").filter(|value| value.is_object());
    let response_data = response.pointer("/response/data");
    let containers = [
        response.get("reviews"),
        response.get("items"),
        response.get("data").filter(|value| value.is_array()),
        data.and_then(|value| value.get("reviews")),
        data.and_then(|value| value.get("items")),
        data.and_then(|value| value.get("data")),
        data.and_then(|value| value.pointer("/shop/reviews")),
        response_data.and_then(|value| value.get("reviews")),
        response_data.and_then(|value| value.get("items")),
        response_data.and_then(|value| value.pointer("/shop/reviews")),
    ];

    let mut seen = HashSet::new();
    let mut reviews = Vec::new();
    for container in containers.into_iter().flatten() {
        let Some(items) = container.as_array() else {
            continue;
        };
        for review in items {
            if let Some(key) = review_dedupe_key(review)
                && !seen.insert(key)
            {
                continue;
            }
            reviews.push(review.clone());
        }
    }
    reviews
}

pub fn enrich_review(
    review: &Value,
    target: &TrustedShopsTarget,
    request_params: &Map<String, Value>,
    response: &Value,
    include_raw_review: bool,
) -> Value {
    let mut item = review.as_object().cloned().unwrap_or_default();
    item.insert("tsid".into(), json!(target.tsid));
    item.insert("shop_name".into(), shop_name(review, response));
    item.insert("rating".into(), review_rating(review));
    item.insert("review_text".into(), review_text(review).unwrap_or(Value::Null));
    item.insert("review_title".into(), review_title(review).unwrap_or(Value::Null));
    item.insert("created_at".into(), review_date(review).unwrap_or(Value::Null));
    item.insert("verified".into(), review_verified(review));
    item.insert(
        "criteria".into(),
        first_non_null(review, &["criteria", "ratings"]).cloned().unwrap_or(Value::Null),
    );
    item.insert("review_id".into(), review_id(review).map_or(Value::Null, Value::String));
    item.insert("page".into(), request_params.get("page").cloned().unwrap_or(Value::Null));
    item.insert("size".into(), request_params.get("size").cloned().unwrap_or(Value::Null));
    item.insert("source_url".into(), json!(target.source_url));
    item.insert("input".into(), json!(target.input));
    item.insert(
        "request_market".into(),
        request_params.get("market").cloned().unwrap_or(Value::Null),
    );
    item.insert("page_total_reviews".into(), total_reviews(response));
    item.insert("page_total_pages".into(), total_pages(response));
    if include_raw_review {
        item.insert("raw_review".into(), review.clone());
    }
    Value::Object(item)
}

pub fn has_next_page(response: &Value, page: u64) -> Option<bool> {
    let explicit = response
        .pointer("/pagination/has_next_page")
        .or_else(|| response.pointer("/pagination/hasNextPage"))
        .or_else(|| response.pointer("/meta/pagination/has_next_page"))
        .or_else(|| response.pointer("/meta/pagination/hasNextPage"));
    if let Some(explicit) = explicit.and_then(Value::as_bool) {
        return Some(explicit);
    }
    total_pages(response).as_u64().map(|pages| page < pages)
}

fn review_dedupe_key(review: &Value) -> Option<String> {
    if let Some(id) = review_id(review) {
        return Some(format!("id:{id}"));
    }
    let title = review_title(review).unwrap_or(Value::Null);
    let text = review_text(review).unwrap_or(Value::Null);
    let date = review_date(review).unwrap_or(Value::Null);
    if title.is_null() && text.is_null() && date.is_null() {
        return None;
    }
    serde_json::to_string(&json!({
        "rating": review_rating(review),
        "title": title,
        "text": text,
        "date": date,
        "verificationStatus": review.get("verificationStatus")
            .and_then(Value::as_str)
            .map(|value| value.trim().to_ascii_uppercase()),
    }))
    .ok()
}

fn review_id(review: &Value) -> Option<String> {
    first_non_null(review, &["review_id", "reviewId", "id"])
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn review_title(review: &Value) -> Option<Value> {
    nonempty_string(first_non_null(review, &["reviewTitle", "title"]))
}

fn review_text(review: &Value) -> Option<Value> {
    nonempty_string(first_non_null(review, &["reviewText", "text", "comment"]))
}

fn review_rating(review: &Value) -> Value {
    first_non_null(review, &["rating", "ratingValue", "stars"])
        .filter(|value| value.is_number())
        .cloned()
        .unwrap_or(Value::Null)
}

fn review_date(review: &Value) -> Option<Value> {
    let value = first_non_null(review, &["created_at", "createdAt", "submittedAt", "date"])?;
    if let Some(value) = value.as_str().filter(|value| !value.trim().is_empty()) {
        return Some(json!(value.trim()));
    }
    let timestamp = value.as_i64().or_else(|| {
        value.as_f64().filter(|number| number.is_finite()).map(|number| number as i64)
    })?;
    let millis = if timestamp > 100_000_000_000 {
        timestamp
    } else {
        timestamp.saturating_mul(1000)
    };
    DateTime::<Utc>::from_timestamp_millis(millis)
        .map(|date| json!(date.to_rfc3339_opts(SecondsFormat::Millis, true)))
}

fn review_verified(review: &Value) -> Value {
    if let Some(value) = first_non_null(review, &["verified", "isVerified", "verifiedReview"])
        .and_then(Value::as_bool)
    {
        return json!(value);
    }
    match review.get("verificationStatus").and_then(Value::as_str) {
        Some(value) if ["MEMBER_VERIFIED", "VERIFIED"].contains(&value.trim().to_ascii_uppercase().as_str()) => json!(true),
        Some(value) if ["UNVERIFIED", "NOT_VERIFIED", "NOT_MEMBER_VERIFIED"].contains(&value.trim().to_ascii_uppercase().as_str()) => json!(false),
        _ => Value::Null,
    }
}

fn shop_name(review: &Value, response: &Value) -> Value {
    let data = response.get("data").filter(|value| value.is_object());
    let value = first_non_null(review, &["shop_name", "shopName"])
        .or_else(|| response.pointer("/shop/name"))
        .or_else(|| response.pointer("/shop/shopName"))
        .or_else(|| data.and_then(|value| value.pointer("/shop/name")))
        .or_else(|| data.and_then(|value| value.pointer("/shop/shopName")))
        .or_else(|| response.pointer("/response/data/shop/name"))
        .or_else(|| response.pointer("/response/data/shop/shopName"));
    nonempty_string(value).unwrap_or(Value::Null)
}

fn total_pages(response: &Value) -> Value {
    first_non_null(response, &[
        "pagination.total_pages",
        "pagination.totalPages",
        "meta.pagination.total_pages",
        "meta.pagination.totalPages",
        "meta.total_pages",
        "meta.totalPages",
        "metaData.totalPageCount",
    ])
    .filter(|value| value.is_number())
    .cloned()
    .unwrap_or(Value::Null)
}

fn total_reviews(response: &Value) -> Value {
    first_non_null(response, &[
        "pagination.total_count",
        "pagination.totalCount",
        "meta.pagination.total_count",
        "meta.pagination.totalCount",
        "meta.total_count",
        "meta.totalCount",
        "metaData.totalReviewCount",
        "data.shop.reviewCount",
        "response.data.shop.reviewCount",
    ])
    .filter(|value| value.is_number())
    .cloned()
    .unwrap_or(Value::Null)
}

fn first_non_null<'a>(value: &'a Value, fields: &[&str]) -> Option<&'a Value> {
    fields
        .iter()
        .filter_map(|field| value.pointer(&format!("/{}", field.replace('.', "/"))))
        .find(|value| !value.is_null())
}

fn nonempty_string(value: Option<&Value>) -> Option<Value> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| json!(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_params::{build_request_plan, page_params};

    const TSID: &str = "XFB15FFBDE1DEE7A55D292A7D48598A6A";

    fn target() -> TrustedShopsTarget {
        build_request_plan(&json!({ "tsid": TSID })).unwrap().targets.remove(0)
    }

    #[test]
    fn collects_response_shapes_and_deduplicates_by_id() {
        let response = json!({
            "reviews": [{"id":"review-1", "title":"First"}, {"reviewId":"review-1", "title":"Duplicate"}],
            "data": {"reviews": [{"review_id":"review-2", "title":"Second"}, {"title":"Missing ID"}]}
        });
        let reviews = collect_reviews(&response);
        assert_eq!(reviews.iter().map(|review| review["title"].as_str().unwrap()).collect::<Vec<_>>(), ["First", "Second", "Missing ID"]);
        assert_eq!(collect_reviews(&json!({ "data": [{"id":"review-3"}] })).len(), 1);
        assert_eq!(collect_reviews(&json!({ "response": {"data": {"shop": {"reviews": [{"id":"review-4"}]}}} })).len(), 1);
        assert!(collect_reviews(&json!({})).is_empty());
    }

    #[test]
    fn deduplicates_reviews_without_ids_using_stable_fields() {
        let review = json!({
            "rating": 5,
            "title": "Same review",
            "comment": "Duplicate text",
            "createdAt": 1780254823000_i64,
            "verificationStatus": "MEMBER_VERIFIED"
        });
        let response = json!({
            "reviews": [review.clone()],
            "data": {"reviews": [review, {"rating": 4, "title": "Different review", "comment": "Different text", "createdAt": 1780254823000_i64}]}
        });
        let reviews = collect_reviews(&response);
        assert_eq!(reviews.len(), 2);
        assert_eq!(reviews[0]["title"], "Same review");
        assert_eq!(reviews[1]["title"], "Different review");
    }

    #[test]
    fn enriches_review_fields_and_preserves_raw_data() {
        let review = json!({
            "id": "review-1",
            "shopName": "Example Shop",
            "ratingValue": 4.8,
            "reviewTitle": "Fast delivery",
            "reviewText": "Everything arrived quickly.",
            "submittedAt": "2026-05-20T10:15:00Z",
            "verifiedReview": true,
            "criteria": {"delivery": 5},
            "customField": "preserved"
        });
        let params = page_params(&build_request_plan(&json!({ "tsid": TSID, "page": 2, "size": 10, "market": "DEU" })).unwrap(), 2);
        let response = json!({"pagination": {"total_count": 123, "total_pages": 13}});
        let item = enrich_review(&review, &target(), &params, &response, false);
        assert_eq!(item["tsid"], TSID);
        assert_eq!(item["shop_name"], "Example Shop");
        assert_eq!(item["rating"], 4.8);
        assert_eq!(item["review_title"], "Fast delivery");
        assert_eq!(item["review_text"], "Everything arrived quickly.");
        assert_eq!(item["created_at"], "2026-05-20T10:15:00Z");
        assert_eq!(item["verified"], true);
        assert_eq!(item["criteria"], json!({"delivery": 5}));
        assert_eq!(item["review_id"], "review-1");
        assert_eq!(item["page"], 2);
        assert_eq!(item["size"], 10);
        assert_eq!(item["source_url"], target().source_url);
        assert_eq!(item["request_market"], "DEU");
        assert_eq!(item["page_total_reviews"], 123);
        assert_eq!(item["page_total_pages"], 13);
        assert_eq!(item["customField"], "preserved");
        assert!(item.get("raw_review").is_none());
        assert_eq!(enrich_review(&review, &target(), &params, &response, true)["raw_review"], review);
    }

    #[test]
    fn normalizes_live_wrapper_fields_and_verification_statuses() {
        let item = enrich_review(
            &json!({"id":"rev-live", "rating":5, "title":"Alles Gut", "comment":"Schnelle Lieferung.", "createdAt":1780254823000_i64, "verificationStatus":"MEMBER_VERIFIED"}),
            &target(),
            &page_params(&build_request_plan(&json!({"tsid":TSID})).unwrap(), 1),
            &json!({"response":{"data":{"shop":{"reviewCount":100}}}}),
            false,
        );
        assert_eq!(item["review_id"], "rev-live");
        assert_eq!(item["review_title"], "Alles Gut");
        assert_eq!(item["review_text"], "Schnelle Lieferung.");
        assert_eq!(item["created_at"], "2026-05-31T19:13:43.000Z");
        assert_eq!(item["verified"], true);
        assert_eq!(item["page_total_reviews"], 100);
        assert_eq!(enrich_review(&json!({"verificationStatus":"UNVERIFIED"}), &target(), &Map::new(), &json!({}), false)["verified"], false);
        assert_eq!(enrich_review(&json!({"verificationStatus":"PENDING"}), &target(), &Map::new(), &json!({}), false)["verified"], Value::Null);
    }

    #[test]
    fn handles_pagination_stop_conditions() {
        assert_eq!(has_next_page(&json!({"pagination":{"has_next_page":false}}), 1), Some(false));
        assert_eq!(has_next_page(&json!({"meta":{"pagination":{"hasNextPage":true}}}), 1), Some(true));
        assert_eq!(has_next_page(&json!({"metaData":{"totalPageCount":2}}), 1), Some(true));
        assert_eq!(has_next_page(&json!({"metaData":{"totalPageCount":2}}), 2), Some(false));
        assert_eq!(has_next_page(&json!({}), 1), None);
    }
}
