use crate::request_params::RequestParams;
use serde_json::{json, Map, Value};

fn object_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_object()?.get(key)
}

fn non_null_field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    object_field(value, key).filter(|value| !value.is_null())
}

fn nullish_choice(values: &[Option<&Value>]) -> Value {
    values
        .iter()
        .flatten()
        .find(|value| !value.is_null())
        .map(|value| (*value).clone())
        .unwrap_or(Value::Null)
}

fn first_non_empty_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().flatten().find_map(|value| {
        value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn with_protocol(value: Option<&Value>) -> Value {
    let Some(value) = value.and_then(Value::as_str).map(str::trim) else {
        return Value::Null;
    };
    if value.is_empty() {
        return Value::Null;
    }
    if value.starts_with("//") {
        return Value::String(format!("https:{value}"));
    }
    if value
        .get(..7)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"))
        || value
            .get(..8)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
    {
        return Value::String(value.to_owned());
    }
    Value::String(format!("https://{value}"))
}

fn trustpilot_profile_url(value: Option<&Value>, domain: &str) -> String {
    let Some(value) = value.and_then(Value::as_str).map(str::trim) else {
        return format!("https://www.trustpilot.com/review/{domain}");
    };
    if value.is_empty() {
        return format!("https://www.trustpilot.com/review/{domain}");
    }
    if value.starts_with("//") {
        return with_protocol(Some(&Value::String(value.to_owned())))
            .as_str()
            .unwrap_or_default()
            .to_owned();
    }
    if value.starts_with('/') {
        return format!("https://www.trustpilot.com{value}");
    }
    with_protocol(Some(&Value::String(value.to_owned())))
        .as_str()
        .unwrap_or_default()
        .to_owned()
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

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    js_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn category_values(categories: Option<&Value>, keys: &[&str]) -> String {
    let Some(categories) = categories.and_then(Value::as_array) else {
        return String::new();
    };
    categories
        .iter()
        .filter_map(|category| {
            category.as_object()?;
            let values = keys
                .iter()
                .map(|key| object_field(category, key))
                .collect::<Vec<_>>();
            let value = nullish_choice(&values);
            js_truthy(&value).then(|| js_string(&value))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn build_dataset_item(response: &Value, company_domain: &str, params: &RequestParams) -> Value {
    let details = non_null_field(response, "data").unwrap_or(response);
    let basic_info = object_field(details, "basic_info").unwrap_or(&Value::Null);
    let ratings = object_field(details, "ratings").unwrap_or(&Value::Null);
    let location = object_field(details, "location").unwrap_or(&Value::Null);
    let contact = object_field(details, "contact").unwrap_or(&Value::Null);
    let metadata = object_field(details, "metadata").unwrap_or(&Value::Null);
    let response_metadata = object_field(response, "meta").unwrap_or(&Value::Null);

    let domain = first_non_empty_string(&[
        object_field(basic_info, "domain"),
        object_field(basic_info, "identifying_name"),
    ])
    .unwrap_or_else(|| company_domain.to_owned());
    let website = first_non_empty_string(&[
        object_field(contact, "website"),
        object_field(basic_info, "website"),
        object_field(basic_info, "website_url"),
        Some(&Value::String(domain.clone())),
    ]);

    let mut item = response.as_object().cloned().unwrap_or_else(Map::new);
    item.insert("company_domain".into(), Value::String(domain.clone()));
    item.insert(
        "requested_company_domain".into(),
        Value::String(company_domain.to_owned()),
    );
    item.insert(
        "company_name".into(),
        nullish_choice(&[
            object_field(basic_info, "name"),
            object_field(basic_info, "display_name"),
        ]),
    );
    item.insert(
        "business_unit_id".into(),
        nullish_choice(&[object_field(basic_info, "business_unit_id")]),
    );
    item.insert(
        "website_url".into(),
        with_protocol_string(website.as_deref()),
    );
    item.insert(
        "profile_url".into(),
        Value::String(trustpilot_profile_url(
            non_null_field(basic_info, "profile_url")
                .or_else(|| object_field(basic_info, "profileUrl")),
            &domain,
        )),
    );
    item.insert(
        "logo_url".into(),
        with_protocol(
            non_null_field(basic_info, "logo_url")
                .or_else(|| object_field(basic_info, "image_url"))
                .or_else(|| object_field(basic_info, "logo")),
        ),
    );
    item.insert(
        "trust_score".into(),
        nullish_choice(&[
            object_field(ratings, "trustscore"),
            object_field(ratings, "trust_score"),
        ]),
    );
    item.insert(
        "stars".into(),
        nullish_choice(&[object_field(ratings, "stars")]),
    );
    item.insert(
        "review_count".into(),
        nullish_choice(&[
            object_field(ratings, "count"),
            object_field(ratings, "review_count"),
            object_field(ratings, "total_reviews"),
        ]),
    );
    item.insert(
        "is_claimed".into(),
        nullish_choice(&[
            object_field(basic_info, "is_claimed"),
            object_field(basic_info, "claimed"),
        ]),
    );
    item.insert(
        "is_verified".into(),
        nullish_choice(&[
            object_field(basic_info, "is_verified"),
            object_field(basic_info, "verified"),
        ]),
    );
    for key in ["country", "country_code", "city", "address"] {
        item.insert(key.into(), nullish_choice(&[object_field(location, key)]));
    }
    for key in ["email", "phone"] {
        item.insert(key.into(), nullish_choice(&[object_field(contact, key)]));
    }
    item.insert(
        "category_names".into(),
        Value::String(category_values(
            object_field(details, "categories"),
            &["displayName", "name"],
        )),
    );
    item.insert(
        "category_slugs".into(),
        Value::String(category_values(
            object_field(details, "categories"),
            &["slug", "categoryId", "id"],
        )),
    );
    item.insert(
        "social_media".into(),
        nullish_choice(&[object_field(details, "social_media")]),
    );
    item.insert(
        "request_locale".into(),
        nullish_choice(&[params.get("locale")]),
    );
    item.insert(
        "response_source".into(),
        nullish_choice(&[
            object_field(metadata, "source"),
            object_field(response_metadata, "source"),
        ]),
    );
    item.insert(
        "scraped_at".into(),
        nullish_choice(&[
            object_field(metadata, "scraped_at"),
            object_field(response_metadata, "scraped_at"),
        ]),
    );
    Value::Object(item)
}

fn with_protocol_string(value: Option<&str>) -> Value {
    value
        .map(|value| with_protocol(Some(&Value::String(value.to_owned()))))
        .unwrap_or(Value::Null)
}

pub fn build_output_summary(
    domains: &[String],
    base_params: &RequestParams,
    saved_companies: usize,
    failures: &[Value],
    status_message: Option<&str>,
) -> Value {
    let mut request = Map::new();
    request.insert(
        "endpoint".into(),
        Value::String("/trustpilot/company-details".into()),
    );
    request.insert(
        "company_domains".into(),
        Value::Array(domains.iter().cloned().map(Value::String).collect()),
    );
    request.extend(base_params.clone());

    json!({
        "request": request,
        "companies_requested": domains.len(),
        "companies_saved": saved_companies,
        "companies_failed": failures.len(),
        "responses_saved": saved_companies,
        "status_message": status_message,
        "failures": failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_live_response_fields_and_preserves_source_payload() {
        let params = json!({"locale":"en-US"}).as_object().unwrap().clone();
        let response = json!({
            "success":true,
            "data":{
                "basic_info":{
                    "name":"Trustpilot",
                    "domain":"trustpilot.com",
                    "business_unit_id":"abc123",
                    "website":"www.trustpilot.com",
                    "profileUrl":"/review/trustpilot.com",
                    "logo":"//example.com/logo.png",
                    "claimed":true,
                    "verified":false
                },
                "ratings":{"trustscore":4.4,"stars":4.5,"count":501587},
                "location":{"country":"United States","country_code":"US","city":"New York","address":"New York, NY"},
                "categories":[
                    {"displayName":"Review Site","slug":"review-site"},
                    {"name":"Software Company","id":"software-company"}
                ],
                "contact":{"email":"support@trustpilot.com","phone":"+1 555 0100"},
                "social_media":{"linkedin":"https://www.linkedin.com/company/trustpilot"},
                "metadata":{"source":"trustpilot","scraped_at":"2026-06-06T05:03:22.364719Z"}
            },
            "meta":{"source":"fallback"}
        });

        let item = build_dataset_item(&response, "trustpilot.com", &params);
        assert_eq!(item["success"], true);
        assert_eq!(item["company_domain"], "trustpilot.com");
        assert_eq!(item["requested_company_domain"], "trustpilot.com");
        assert_eq!(item["company_name"], "Trustpilot");
        assert_eq!(item["business_unit_id"], "abc123");
        assert_eq!(item["website_url"], "https://www.trustpilot.com");
        assert_eq!(
            item["profile_url"],
            "https://www.trustpilot.com/review/trustpilot.com"
        );
        assert_eq!(item["logo_url"], "https://example.com/logo.png");
        assert_eq!(item["trust_score"], 4.4);
        assert_eq!(item["stars"], 4.5);
        assert_eq!(item["review_count"], 501587);
        assert_eq!(item["is_claimed"], true);
        assert_eq!(item["is_verified"], false);
        assert_eq!(item["country"], "United States");
        assert_eq!(item["country_code"], "US");
        assert_eq!(item["city"], "New York");
        assert_eq!(item["category_names"], "Review Site, Software Company");
        assert_eq!(item["category_slugs"], "review-site, software-company");
        assert_eq!(
            item["social_media"],
            json!({"linkedin":"https://www.linkedin.com/company/trustpilot"})
        );
        assert_eq!(item["request_locale"], "en-US");
        assert_eq!(item["response_source"], "trustpilot");
        assert_eq!(item["scraped_at"], "2026-06-06T05:03:22.364719Z");
    }

    #[test]
    fn handles_sparse_payloads_and_filters_invalid_categories() {
        let params = Map::new();
        let item = build_dataset_item(
            &json!({
                "basic_info":{"domain":"", "profile_url":"//www.trustpilot.com/review/example.com", "website":""},
                "ratings":{"trust_score":3.2,"review_count":10},
                "categories":[null,"bad",{"name":"Review Site","id":"review_site"},42,{"displayName":"Media Company","slug":"media-company"}]
            }),
            "example.com",
            &params,
        );

        assert_eq!(item["company_domain"], "example.com");
        assert_eq!(
            item["profile_url"],
            "https://www.trustpilot.com/review/example.com"
        );
        assert_eq!(item["website_url"], "https://example.com");
        assert_eq!(item["trust_score"], 3.2);
        assert_eq!(item["review_count"], 10);
        assert_eq!(item["category_names"], "Review Site, Media Company");
        assert_eq!(item["category_slugs"], "review_site, media-company");
        assert_eq!(item["company_name"], Value::Null);
        assert_eq!(item["social_media"], Value::Null);
    }

    #[test]
    fn builds_the_same_compact_output_summary() {
        let params = json!({"locale":"en-US"}).as_object().unwrap().clone();
        let domains = vec!["trustpilot.com".into(), "example.com".into()];
        let failures = vec![json!({"company_domain":"example.com", "error":"Not found"})];
        let summary = build_output_summary(
            &domains,
            &params,
            1,
            &failures,
            Some("1 of 2 Trustpilot company detail request(s) failed."),
        );

        assert_eq!(
            summary["request"]["endpoint"],
            "/trustpilot/company-details"
        );
        assert_eq!(summary["request"]["company_domains"], json!(domains));
        assert_eq!(summary["companies_requested"], 2);
        assert_eq!(summary["companies_saved"], 1);
        assert_eq!(summary["companies_failed"], 1);
        assert_eq!(summary["responses_saved"], 1);
        assert_eq!(summary["failures"][0]["error"], "Not found");
        assert!(summary.get("responses").is_none());
    }
}
