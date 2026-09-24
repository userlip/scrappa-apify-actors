use serde_json::{Map, Value, json};

pub fn has_shop_profile_data(response: &Value) -> bool {
    get_shop_profile_data(response).is_some()
}

pub fn build_dataset_item(
    response: &Value,
    requested_tsid: &str,
    source_url: Option<&str>,
    include_raw_response: bool,
) -> Result<Value, String> {
    let profile = get_shop_profile_data(response).ok_or_else(|| {
        "Scrappa response did not include a TrustedShops shop profile object".to_owned()
    })?;
    let review_summary = first_object([profile.get("reviewSummary"), profile.get("ratingSummary")]);
    let metadata = first_object([
        profile.get("metadata"),
        profile.get("profileMetadata"),
        response.pointer("/response/responseInfo"),
        response.get("meta"),
        response.get("metaData"),
    ]);
    let categories = match profile.get("categories").and_then(Value::as_array) {
        Some(categories) if !categories.is_empty() => categories.clone(),
        _ => profile
            .get("shopCategories")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    };
    let tsid = first_non_empty_string([
        profile.get("tsid"),
        profile.get("tsID"),
        profile.get("tsId"),
        Some(&Value::String(requested_tsid.to_owned())),
    ])
    .unwrap_or_else(|| requested_tsid.to_owned());
    let category_names = categories
        .iter()
        .filter_map(category_name)
        .collect::<Vec<_>>()
        .join(", ");
    let category_ids = categories
        .iter()
        .filter_map(category_id)
        .collect::<Vec<_>>()
        .join(", ");

    let mut item = json!({
        "tsid": tsid,
        "requested_tsid": requested_tsid,
        "name": first_non_empty_string([
            profile.get("name"),
            profile.get("displayName"),
            profile.get("accountName"),
            profile.get("shopName"),
        ]),
        "url": with_protocol(first_non_empty_string([
            profile.get("url"),
            profile.get("shopUrl"),
            profile.get("website"),
        ])),
        "profile_url": trustedshops_profile_url(
            first_non_empty_string([
                profile.get("profileUrl"),
                profile.get("profile_url"),
            ]),
            &tsid,
        ),
        "language": first_non_empty_string([
            profile.get("language"),
            profile.get("languageCode"),
            profile.get("languageISO2"),
        ]),
        "target_market": first_non_empty_string([
            profile.get("targetMarket"),
            profile.get("target_market"),
            profile.get("targetMarketISO3"),
            profile.get("market"),
        ]),
        "rating": first_number([
            profile.get("rating"),
            profile.get("averageRating"),
            review_summary.get("rating"),
            review_summary.get("averageRating"),
        ]),
        "review_count": first_number([
            profile.get("reviewCount"),
            profile.get("reviewsCount"),
            review_summary.get("reviewCount"),
            review_summary.get("reviewsCount"),
            review_summary.get("totalReviewCount"),
        ]),
        "certified": first_boolean([
            profile.get("certified"),
            profile.get("certificationState"),
        ]),
        "categories": categories,
        "category_names": category_names,
        "category_ids": category_ids,
        "source_url": source_url,
        "profile_metadata": if metadata.is_empty() { Value::Null } else { Value::Object(metadata) },
    });

    if include_raw_response {
        item.as_object_mut()
            .expect("constructed dataset item is an object")
            .insert("raw_response".to_owned(), response.clone());
    }

    Ok(item)
}

pub fn build_output_summary(
    requested: usize,
    saved_profiles: usize,
    failures: &[Value],
    status_message: Option<&str>,
) -> Value {
    json!({
        "request": { "endpoint": "/trustedshops/shop/{tsid}" },
        "profiles_requested": requested,
        "profiles_saved": saved_profiles,
        "profiles_failed": failures.len(),
        "responses_saved": saved_profiles,
        "status_message": status_message,
        "failures": failures,
    })
}

fn get_shop_profile_data(response: &Value) -> Option<&Map<String, Value>> {
    let candidates = [
        response.pointer("/response/data/shop"),
        response.pointer("/data/shop"),
        response.get("data"),
        response.get("shop"),
        Some(response),
    ];

    candidates.into_iter().flatten().find_map(|candidate| {
        let object = candidate.as_object()?;
        if !object.is_empty() && has_tsid_signal(object) && has_profile_signal(object) {
            Some(object)
        } else {
            None
        }
    })
}

fn has_tsid_signal(profile: &Map<String, Value>) -> bool {
    first_non_empty_string([
        profile.get("tsid"),
        profile.get("tsID"),
        profile.get("tsId"),
    ])
    .is_some()
}

fn has_profile_signal(profile: &Map<String, Value>) -> bool {
    first_non_empty_string([
        profile.get("name"),
        profile.get("displayName"),
        profile.get("accountName"),
        profile.get("shopName"),
        profile.get("url"),
        profile.get("shopUrl"),
        profile.get("website"),
    ])
    .is_some()
}

fn first_object<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Map<String, Value> {
    values
        .into_iter()
        .flatten()
        .find_map(|value| {
            let object = value.as_object()?;
            (!object.is_empty()).then(|| object.clone())
        })
        .unwrap_or_default()
}

fn first_non_empty_string<'a>(
    values: impl IntoIterator<Item = Option<&'a Value>>,
) -> Option<String> {
    values.into_iter().flatten().find_map(|value| {
        value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn first_number<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Option<Value> {
    values
        .into_iter()
        .flatten()
        .find(|value| value.as_f64().is_some())
        .cloned()
}

fn first_boolean<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Option<bool> {
    values.into_iter().flatten().find_map(Value::as_bool)
}

fn with_protocol(value: Option<String>) -> Option<String> {
    let value = value?;
    if value.starts_with("//") {
        return Some(format!("https:{value}"));
    }

    if value.starts_with("http://") || value.starts_with("https://") {
        Some(value)
    } else {
        Some(format!("https://{value}"))
    }
}

fn trustedshops_profile_url(value: Option<String>, tsid: &str) -> String {
    if let Some(value) = value {
        if value.starts_with('/') {
            return format!("https://www.trustedshops.de{value}");
        }
        if let Some(profile_url) = with_protocol(Some(value)) {
            return profile_url;
        }
    }

    format!("https://www.trustedshops.de/bewertung/info_{tsid}.html")
}

fn category_name(category: &Value) -> Option<String> {
    if let Some(value) = category.as_str() {
        return (!value.trim().is_empty()).then(|| value.trim().to_owned());
    }

    let object = category.as_object()?;
    first_non_empty_string([
        object.get("name"),
        object.get("displayName"),
        object.get("title"),
    ])
}

fn category_id(category: &Value) -> Option<String> {
    let object = category.as_object()?;
    let value = object
        .get("id")
        .filter(|value| !value.is_null())
        .or_else(|| object.get("categoryId").filter(|value| !value.is_null()))
        .or_else(|| object.get("urlPath").filter(|value| !value.is_null()))
        .or_else(|| object.get("slug").filter(|value| !value.is_null()))?;

    if let Some(number) = value.as_i64() {
        return Some(number.to_string());
    }
    if let Some(number) = value.as_u64() {
        return Some(number.to_string());
    }
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TSID: &str = "XFB15FFBDE1DEE7A55D292A7D48598A6A";

    #[test]
    fn flattens_profile_fields_and_uses_scrappa_metadata() {
        let response = json!({
            "response": {
                "data": { "shop": {
                    "tsId": TSID,
                    "name": "Example Shop",
                    "url": "example-shop.de",
                    "profileUrl": format!("/bewertung/info_{TSID}.html"),
                    "languageISO2": "de",
                    "targetMarketISO3": "DEU",
                    "averageRating": 4.8,
                    "reviewCount": 1234,
                    "certificationState": true,
                    "shopCategories": [
                        { "id": 1, "name": "Fashion" },
                        { "id": 2, "name": "Shoes" }
                    ]
                }},
                "responseInfo": { "apiVersion": "2.4.18" }
            }
        });
        let item = build_dataset_item(
            &response,
            TSID,
            Some(&format!("https://www.trustedshops.de/info_{TSID}.html")),
            false,
        )
        .unwrap();

        assert_eq!(item["tsid"], TSID);
        assert_eq!(item["name"], "Example Shop");
        assert_eq!(item["url"], "https://example-shop.de");
        assert_eq!(
            item["profile_url"],
            format!("https://www.trustedshops.de/bewertung/info_{TSID}.html")
        );
        assert_eq!(item["language"], "de");
        assert_eq!(item["target_market"], "DEU");
        assert_eq!(item["rating"], 4.8);
        assert_eq!(item["review_count"], 1234);
        assert_eq!(item["certified"], true);
        assert_eq!(item["category_names"], "Fashion, Shoes");
        assert_eq!(item["category_ids"], "1, 2");
        assert_eq!(item["profile_metadata"], json!({"apiVersion":"2.4.18"}));
        assert!(item.get("raw_response").is_none());
    }

    #[test]
    fn falls_back_to_canonical_profile_url_and_includes_raw_response_when_requested() {
        let response = json!({
            "tsid": TSID,
            "name": "Fallback Shop",
            "reviewSummary": {"rating": 4.5, "totalReviewCount": 99}
        });
        let item = build_dataset_item(&response, TSID, None, true).unwrap();

        assert_eq!(
            item["profile_url"],
            format!("https://www.trustedshops.de/bewertung/info_{TSID}.html")
        );
        assert_eq!(item["rating"], 4.5);
        assert_eq!(item["review_count"], 99);
        assert_eq!(item["raw_response"], response);
    }

    #[test]
    fn skips_empty_candidates_and_falls_back_to_populated_profile() {
        let response = json!({
            "response": {"data": {"shop": {}}, "responseInfo": {}},
            "shop": {"tsId": TSID, "name": "Populated Shop", "targetMarketISO3": "DEU", "metadata": {"source":"fallback"}},
            "metaData": {"source":"metadata fallback"}
        });
        let item = build_dataset_item(&response, TSID, None, false).unwrap();

        assert_eq!(item["name"], "Populated Shop");
        assert_eq!(item["target_market"], "DEU");
        assert_eq!(item["profile_metadata"], json!({"source":"fallback"}));
    }

    #[test]
    fn falls_back_to_shop_categories_and_rejects_missing_profile_signals() {
        let response = json!({
            "shop": {"tsId": TSID, "name": "Category Shop", "categories": [], "shopCategories": [{"id":7,"name":"Electronics"}]}
        });
        let item = build_dataset_item(&response, TSID, None, false).unwrap();
        assert_eq!(item["category_names"], "Electronics");
        assert_eq!(item["category_ids"], "7");

        let missing = json!({"response":{"data":{"shop":{"tsId":TSID}}}});
        assert!(!has_shop_profile_data(&missing));
        assert!(
            build_dataset_item(&missing, TSID, None, false)
                .unwrap_err()
                .contains("did not include a TrustedShops shop profile object")
        );
    }

    #[test]
    fn builds_output_with_saved_counts_and_failures() {
        let output = build_output_summary(
            2,
            1,
            &[json!({"tsid":"bad","error":"not found"})],
            Some("1 failed"),
        );
        assert_eq!(output["profiles_requested"], 2);
        assert_eq!(output["profiles_saved"], 1);
        assert_eq!(output["profiles_failed"], 1);
        assert_eq!(output["responses_saved"], 1);
        assert_eq!(output["status_message"], "1 failed");
    }
}
