use serde_json::{Map, Value};

use crate::request_params::SearchType;

pub fn get_businesses(response: &Value) -> &[Value] {
    if let Some(businesses) = response.get("businessUnits").and_then(Value::as_array) {
        return businesses;
    }
    if let Some(businesses) = response.get("businesses").and_then(Value::as_array) {
        return businesses;
    }
    response
        .pointer("/pageProps/businessUnits/businesses")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

pub fn has_next_page(response: &Value, page: u64) -> bool {
    for path in [
        "/pagination/has_next_page",
        "/pageProps/pagination/has_next_page",
    ] {
        if let Some(value) = response.pointer(path).and_then(Value::as_bool) {
            return value;
        }
    }
    for path in [
        "/pagination/totalPages",
        "/pagination/total_pages",
        "/pageProps/pagination/total_pages",
        "/pageProps/businessUnits/totalPages",
    ] {
        if let Some(total_pages) = response.pointer(path).and_then(Value::as_f64) {
            return (page as f64) < total_pages;
        }
    }
    eprintln!("No pagination data found in response; stopping after page {page}");
    false
}

pub fn build_dataset_item(
    business: &Value,
    search_type: SearchType,
    params: &Map<String, Value>,
    response: &Value,
) -> Value {
    let mut item = business.as_object().cloned().unwrap_or_default();
    let categories = business.get("categories").and_then(Value::as_array);
    let address = business.get("address").filter(|value| !value.is_null());

    item.insert(
        "business_id".into(),
        nullish(business.get("businessUnitId"), business.get("id")),
    );
    item.insert(
        "business_name".into(),
        nullish(business.get("displayName"), business.get("name")),
    );
    item.insert(
        "identifying_name".into(),
        business
            .get("identifyingName")
            .cloned()
            .filter(|value| !value.is_null())
            .unwrap_or(Value::Null),
    );
    item.insert(
        "website_url".into(),
        with_protocol(nullish_ref(
            business.pointer("/contact/website"),
            business.get("websiteUrl"),
        )),
    );
    item.insert("profile_url".into(), profile_url(business));
    item.insert(
        "logo_url".into(),
        with_protocol(first_non_null_ref([
            business.get("logo"),
            business.get("logoUrl"),
            business.get("profileImageUrl"),
        ])),
    );
    item.insert(
        "trust_score".into(),
        nullish_ref(
            business.get("trustScore"),
            business.pointer("/score/trustScore"),
        )
        .cloned()
        .unwrap_or(Value::Null),
    );
    item.insert(
        "stars".into(),
        nullish_ref(business.get("stars"), business.pointer("/score/stars"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    item.insert(
        "review_count".into(),
        nullish3(
            business.get("numberOfReviews"),
            business.get("totalNumberOfReviews"),
            None,
        ),
    );
    item.insert(
        "is_verified".into(),
        nullish3(business.get("isVerified"), business.get("verified"), None),
    );
    item.insert(
        "is_claimed".into(),
        nullish3(
            business.get("isClaimed"),
            business.get("isBusinessClaimed"),
            None,
        ),
    );
    item.insert(
        "country_code".into(),
        nullish_ref(
            business.get("countryCode"),
            address.and_then(|value| value.get("countryCode")),
        )
        .cloned()
        .unwrap_or(Value::Null),
    );
    item.insert(
        "country".into(),
        nullish_ref(
            business.pointer("/location/country"),
            address.and_then(|value| value.get("country")),
        )
        .cloned()
        .unwrap_or(Value::Null),
    );
    item.insert(
        "city".into(),
        nullish_ref(
            business.pointer("/location/city"),
            address.and_then(|value| value.get("city")),
        )
        .cloned()
        .unwrap_or(Value::Null),
    );
    item.insert(
        "email".into(),
        nullish_ref(business.pointer("/contact/email"), business.get("email"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    item.insert(
        "phone".into(),
        nullish_ref(business.pointer("/contact/phone"), business.get("phone"))
            .cloned()
            .unwrap_or(Value::Null),
    );
    item.insert(
        "category_names".into(),
        Value::String(join_categories(categories, true)),
    );
    item.insert(
        "category_slugs".into(),
        Value::String(join_categories(categories, false)),
    );
    item.insert(
        "request_search_type".into(),
        Value::String(search_type.as_str().into()),
    );
    for (output, input) in [
        ("request_query", "query"),
        ("request_category", "category"),
        ("request_country", "country"),
        ("request_page", "page"),
        ("request_locale", "locale"),
        ("request_min_rating", "min_rating"),
        ("request_min_review_count", "min_review_count"),
        ("request_sort", "sort"),
        ("request_claimed", "claimed"),
        ("request_trustscore", "trustscore"),
    ] {
        item.insert(
            output.into(),
            params.get(input).cloned().unwrap_or(Value::Null),
        );
    }
    item.insert(
        "total_results".into(),
        first_non_null([
            response.pointer("/pagination/totalResults"),
            response.pointer("/pagination/total_count"),
            response.pointer("/pageProps/pagination/total_count"),
            response.pointer("/pageProps/businessUnits/totalHits"),
        ]),
    );
    item.insert(
        "total_pages".into(),
        first_non_null([
            response.pointer("/pagination/totalPages"),
            response.pointer("/pagination/total_pages"),
            response.pointer("/pageProps/pagination/total_pages"),
            response.pointer("/pageProps/businessUnits/totalPages"),
        ]),
    );
    item.insert(
        "per_page".into(),
        first_non_null([
            response.pointer("/pagination/perPage"),
            response.pointer("/pagination/pageSize"),
            response.pointer("/pagination/per_page"),
            response.pointer("/pageProps/pagination/per_page"),
        ]),
    );
    item.insert(
        "search_mode".into(),
        response
            .get("searchMode")
            .cloned()
            .filter(|value| !value.is_null())
            .unwrap_or(Value::Null),
    );
    item.insert(
        "category_display_name".into(),
        response
            .pointer("/pageProps/categoryDisplayName")
            .cloned()
            .filter(|value| !value.is_null())
            .unwrap_or(Value::Null),
    );
    item.insert(
        "response_source".into(),
        response
            .pointer("/meta/source")
            .cloned()
            .filter(|value| !value.is_null())
            .unwrap_or(Value::Null),
    );
    item.insert(
        "scraped_at".into(),
        response
            .pointer("/meta/scraped_at")
            .cloned()
            .filter(|value| !value.is_null())
            .unwrap_or(Value::Null),
    );
    Value::Object(item)
}

fn profile_url(business: &Value) -> Value {
    business
        .get("identifyingName")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| Value::String(format!("https://www.trustpilot.com/review/{value}")))
        .unwrap_or(Value::Null)
}

fn with_protocol(value: Option<&Value>) -> Value {
    let Some(value) = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Value::Null;
    };
    if value.starts_with("//") {
        return Value::String(format!("https:{value}"));
    }
    let lowercase = value.to_ascii_lowercase();
    if lowercase.starts_with("http://") || lowercase.starts_with("https://") {
        Value::String(value.into())
    } else {
        Value::String(format!("https://{value}"))
    }
}

fn join_categories(categories: Option<&Vec<Value>>, use_name: bool) -> String {
    categories
        .into_iter()
        .flatten()
        .filter_map(|category| {
            let value = if use_name {
                nullish_ref(category.get("displayName"), category.get("name"))
            } else {
                first_non_null_ref([
                    category.get("slug"),
                    category.get("categoryId"),
                    category.get("id"),
                ])
            };
            value.filter(|value| js_truthy(value)).map(js_value_string)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_value_string(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(js_value_string)
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn nullish(first: Option<&Value>, second: Option<&Value>) -> Value {
    nullish_ref(first, second).cloned().unwrap_or(Value::Null)
}

fn nullish_ref<'a>(first: Option<&'a Value>, second: Option<&'a Value>) -> Option<&'a Value> {
    first
        .filter(|value| !value.is_null())
        .or_else(|| second.filter(|value| !value.is_null()))
}

fn nullish3<'a>(
    first: Option<&'a Value>,
    second: Option<&'a Value>,
    third: Option<&'a Value>,
) -> Value {
    first
        .filter(|value| !value.is_null())
        .or_else(|| second.filter(|value| !value.is_null()))
        .or_else(|| third.filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null)
}

fn first_non_null<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Value {
    first_non_null_ref(values).cloned().unwrap_or(Value::Null)
}

fn first_non_null_ref<'a>(
    values: impl IntoIterator<Item = Option<&'a Value>>,
) -> Option<&'a Value> {
    values.into_iter().flatten().find(|value| !value.is_null())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_params::{build_request_plan, page_params};
    use serde_json::json;

    #[test]
    fn extracts_all_supported_response_shapes_and_pagination() {
        assert_eq!(
            get_businesses(&json!({"businessUnits":[{"name":"A"}]})).len(),
            1
        );
        assert_eq!(
            get_businesses(&json!({"businesses":[{"name":"B"}]})).len(),
            1
        );
        assert_eq!(
            get_businesses(&json!({"pageProps":{"businessUnits":{"businesses":[{"name":"C"}]}}}))
                .len(),
            1
        );
        assert!(get_businesses(&json!({})).is_empty());
        assert!(has_next_page(&json!({"pagination":{"totalPages":3}}), 1));
        assert!(!has_next_page(
            &json!({"pagination":{"has_next_page":false,"total_pages":3}}),
            1
        ));
        assert!(has_next_page(
            &json!({"pageProps":{"pagination":{"total_pages":3}}}),
            1
        ));
        assert!(!has_next_page(
            &json!({"pageProps":{"businessUnits":{"totalPages":2}}}),
            2
        ));
    }

    #[test]
    fn normalizes_company_search_items_and_keeps_upstream_fields() {
        let plan = build_request_plan(&json!({"query":"amazon", "country":"US"})).unwrap();
        let params = page_params(&plan, 1);
        let response = json!({
            "pagination":{"total_count":42,"total_pages":3,"per_page":20},
            "searchMode":"keyword",
            "meta":{"source":"scrappa","scraped_at":"2026-05-19T00:00:00Z"}
        });
        let business = json!({
            "id":"abc123","displayName":"Amazon","identifyingName":"amazon.com",
            "websiteUrl":"www.amazon.com","score":{"trustScore":1.4,"stars":1.5},
            "numberOfReviews":38765,"logoUrl":"//example.com/logo.png","countryCode":"US",
            "location":{"country":"United States","city":"Seattle"},"address":{"city":"Seattle"},
            "isClaimed":true,"verified":false,
            "categories":[{"displayName":"Marketplace","slug":"marketplace"},{"name":"Electronics","id":"electronics"}],
            "upstream_extra":"preserved"
        });
        let item = build_dataset_item(&business, SearchType::CompanySearch, &params, &response);
        assert_eq!(item["business_id"], "abc123");
        assert_eq!(item["business_name"], "Amazon");
        assert_eq!(item["website_url"], "https://www.amazon.com");
        assert_eq!(
            item["profile_url"],
            "https://www.trustpilot.com/review/amazon.com"
        );
        assert_eq!(item["trust_score"], 1.4);
        assert_eq!(item["logo_url"], "https://example.com/logo.png");
        assert_eq!(item["is_verified"], false);
        assert_eq!(item["category_names"], "Marketplace, Electronics");
        assert_eq!(item["category_slugs"], "marketplace, electronics");
        assert_eq!(item["request_search_type"], "company_search");
        assert_eq!(item["request_query"], "amazon");
        assert_eq!(item["total_results"], 42);
        assert_eq!(item["total_pages"], 3);
        assert_eq!(item["per_page"], 20);
        assert_eq!(item["upstream_extra"], "preserved");
    }

    #[test]
    fn normalizes_category_items_and_preserves_false_values() {
        let plan = build_request_plan(&json!({"search_type":"category","category":"electronics"}))
            .unwrap();
        let item = build_dataset_item(
            &json!({"businessUnitId":"def456","name":"Example Store","contact":{"website":"https://example.com","email":"support@example.com","phone":"+1 555 0100"},"trustScore":4.7,"stars":5,"totalNumberOfReviews":1234,"isBusinessClaimed":true,"verified":false}),
            SearchType::Category,
            &page_params(&plan, 2),
            &json!({"pageProps":{"businessUnits":{"totalHits":80,"totalPages":4},"categoryDisplayName":"Electronics & Technology"}}),
        );
        assert_eq!(item["business_id"], "def456");
        assert_eq!(item["business_name"], "Example Store");
        assert_eq!(item["email"], "support@example.com");
        assert_eq!(item["phone"], "+1 555 0100");
        assert_eq!(item["request_category"], "electronics");
        assert_eq!(item["request_page"], 2);
        assert_eq!(item["category_display_name"], "Electronics & Technology");
        assert_eq!(item["total_results"], 80);
        assert_eq!(item["total_pages"], 4);
        assert_eq!(item["is_verified"], false);
    }
}
