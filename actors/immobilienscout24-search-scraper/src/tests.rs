use crate::{
    billing::ChargePricing,
    endpoint::endpoint_url,
    input::{normalize_search_input, DEFAULT_LOCATION, DEFAULT_TYPE},
    response::{
        dataset_item, get_listings, get_response_page, get_total_pages, get_total_results,
        limited_response,
    },
    scrappa::{
        is_handled_empty_search_error, is_retryable_scrappa_error, scrappa_error_message,
        scrappa_retry_delay, ScrappaError,
    },
};
use serde_json::{json, Number, Value};

#[test]
fn missing_empty_and_unknown_input_use_the_prefilled_defaults() {
    let defaults = normalize_search_input(None).unwrap();
    assert_eq!(defaults.location, DEFAULT_LOCATION);
    assert_eq!(defaults.property_type, DEFAULT_TYPE);
    assert_eq!(defaults.page, 1);
    assert_eq!(defaults.per_page, 20);
    assert_eq!(normalize_search_input(Some(&json!({}))).unwrap(), defaults);
    assert_eq!(
        normalize_search_input(Some(&json!({ "hello": "world" }))).unwrap(),
        defaults
    );
    assert_eq!(
        normalize_search_input(Some(&Value::Null)).unwrap(),
        defaults
    );
}

#[test]
fn trims_search_values_and_maps_legacy_aliases() {
    let params = normalize_search_input(Some(&json!({
        "location": " Berlin ",
        "property_type": " apartment-buy ",
        "limit": 25,
        "rooms_min": " 1.5 ",
        "page": "2"
    })))
    .unwrap();

    assert_eq!(params.location, "Berlin");
    assert_eq!(params.property_type, "apartment-buy");
    assert_eq!(params.per_page, 25);
    assert_eq!(params.page, 2);
    assert_eq!(params.rooms_min, Some(Number::from_f64(1.5).unwrap()));
}

#[test]
fn canonical_type_and_page_size_take_precedence_over_legacy_aliases() {
    let params = normalize_search_input(Some(&json!({
        "location": "Berlin",
        "property_type": "apartment-buy",
        "type": "house-rent",
        "limit": 25,
        "per_page": 10
    })))
    .unwrap();
    assert_eq!(params.property_type, "house-rent");
    assert_eq!(params.per_page, 10);
}

#[test]
fn blank_and_null_required_values_fail_validation_instead_of_using_defaults() {
    let blank = normalize_search_input(Some(&json!({
        "location": "   ",
        "type": "apartment-rent"
    })))
    .unwrap_err();
    assert!(blank.to_string().contains("location is required"));

    let null = normalize_search_input(Some(&json!({
        "location": null,
        "type": "apartment-rent"
    })))
    .unwrap_err();
    assert!(null.to_string().contains("location must be a string"));
}

#[test]
fn validates_property_types_pagination_and_filter_ranges() {
    for (input, expected) in [
        (
            json!({ "location": "Berlin", "type": "apartment", "page": 1, "per_page": 20 }),
            "type must be one of: apartment-rent, apartment-buy, house-rent, house-buy",
        ),
        (
            json!({ "location": "Berlin", "type": "apartment-rent", "page": 0, "per_page": 20 }),
            "page must be between 1 and 10000",
        ),
        (
            json!({ "location": "Berlin", "type": "apartment-rent", "page": 1, "per_page": 51 }),
            "per_page must be between 1 and 50",
        ),
        (
            json!({ "location": "Berlin", "type": "apartment-rent", "price_min": -1 }),
            "price_min must be between 0 and 100000000",
        ),
        (
            json!({ "location": "Berlin", "type": "apartment-rent", "rooms_min": "two" }),
            "rooms_min must be a number",
        ),
        (
            json!({ "location": "Berlin", "type": "apartment-rent", "size_min": 80, "size_max": 60 }),
            "size_min must be less than or equal to size_max",
        ),
    ] {
        assert!(
            normalize_search_input(Some(&input))
                .unwrap_err()
                .to_string()
                .contains(expected),
            "expected error containing {expected}"
        );
    }
}

#[test]
fn builds_encoded_upstream_params_and_request_description() {
    let params = normalize_search_input(Some(&json!({
        "location": "Berlin Mitte",
        "type": "apartment-rent",
        "price_min": "500",
        "price_max": 1500,
        "rooms_min": "1.5",
        "rooms_max": 4,
        "size_min": "45",
        "size_max": 120,
        "page": "2",
        "per_page": "25"
    })))
    .unwrap();
    let query = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params.query_pairs())
        .finish();
    assert_eq!(
        query,
        "location=Berlin+Mitte&type=apartment-rent&price_min=500&price_max=1500&rooms_min=1.5&rooms_max=4&size_min=45&size_max=120&page=2&per_page=25"
    );
    assert_eq!(
        params.describe(),
        "apartment-rent properties in Berlin Mitte (page 2, per_page 25, price 500-1500, rooms 1.5-4, size 45-120)"
    );
}

#[test]
fn reads_listings_from_top_level_and_wrapped_responses() {
    let listings = vec![json!({ "title": "Berlin flat" })];
    assert_eq!(
        get_listings(&json!({ "results": listings })),
        vec![json!({ "title": "Berlin flat" })]
    );
    assert_eq!(
        get_listings(&json!({ "data": { "results": listings } })),
        vec![json!({ "title": "Berlin flat" })]
    );
}

#[test]
fn falls_back_to_wrapped_results_when_top_level_results_are_empty() {
    let response = json!({
        "results": [],
        "data": { "results": [{ "title": "Nested Berlin flat" }] }
    });
    assert_eq!(
        get_listings(&response),
        vec![json!({ "title": "Nested Berlin flat" })]
    );
}

#[test]
fn missing_response_results_become_an_empty_listing_set() {
    assert!(get_listings(&json!({ "success": false })).is_empty());
    assert!(get_listings(&Value::Null).is_empty());
}

#[test]
fn pagination_totals_can_be_read_from_top_level_or_wrapped_data() {
    assert_eq!(
        get_total_results(&json!({ "total_results": 5473 })),
        Some(Number::from(5473))
    );
    assert_eq!(
        get_total_results(&json!({ "data": { "total_results": "25" } })),
        Some(Number::from(25))
    );
    assert_eq!(
        get_response_page(&json!({ "data": { "page": "3" } })),
        Some(Number::from(3))
    );
    assert_eq!(
        get_total_pages(&json!({ "total_pages": 274 })),
        Some(Number::from(274))
    );
    assert_eq!(get_response_page(&json!({ "page": "-2" })), None);
}

#[test]
fn dataset_rows_keep_raw_fields_and_add_normalized_fields_and_request_context() {
    let params = normalize_search_input(Some(&json!({
        "location": "Berlin",
        "type": "apartment-rent",
        "price_min": 500,
        "price_max": 1500,
        "rooms_min": 1.5,
        "rooms_max": 4,
        "size_min": 45,
        "size_max": 120,
        "page": 1,
        "per_page": 20
    })))
    .unwrap();
    let item = dataset_item(
        &json!({
            "id": "estate_123",
            "online_id": "2paau5t",
            "title": "Moderne 3-Zimmerwohnung",
            "lat": 52.511009,
            "lon": 13.402116,
            "custom": { "kept": true }
        }),
        &params,
    );
    assert_eq!(item["id"], "estate_123");
    assert_eq!(item["latitude"], 52.511009);
    assert_eq!(item["longitude"], 13.402116);
    assert_eq!(item["lat"], 52.511009);
    assert_eq!(item["custom"], json!({ "kept": true }));
    assert_eq!(item["price"], Value::Null);
    assert_eq!(item["is_private"], Value::Null);
    assert_eq!(item["request_location"], "Berlin");
    assert_eq!(item["request_type"], "apartment-rent");
    assert_eq!(item["request_price_min"], 500);
    assert_eq!(item["request_rooms_min"], 1.5);
    assert_eq!(item["request_page"], 1);
    assert_eq!(item["request_per_page"], 20);
}

#[test]
fn output_response_trims_both_result_shapes_without_changing_totals() {
    let response = json!({
        "success": true,
        "total_results": 5473,
        "results": [{ "id": "one" }, { "id": "two" }],
        "data": {
            "total_results": 4,
            "results": [{ "id": "three" }, { "id": "four" }]
        }
    });
    let limited = limited_response(&response, 1);
    assert_eq!(limited["results"], json!([{ "id": "one" }]));
    assert_eq!(limited["data"]["results"], json!([{ "id": "three" }]));
    assert_eq!(limited["total_results"], 5473);
    assert_eq!(response["results"].as_array().unwrap().len(), 2);
    assert_eq!(response["data"]["results"].as_array().unwrap().len(), 2);
}

#[test]
fn handles_only_location_400_and_upstream_502_as_empty_search_errors() {
    assert!(is_handled_empty_search_error(&ScrappaError::Api {
        status: 400,
        message: "Location 'Berlin' not found".to_owned()
    }));
    assert!(is_handled_empty_search_error(&ScrappaError::Api {
        status: 400,
        message: "INVALID_LOCATION".to_owned()
    }));
    assert!(!is_handled_empty_search_error(&ScrappaError::Api {
        status: 400,
        message: "Bad Request".to_owned()
    }));
    assert!(!is_handled_empty_search_error(&ScrappaError::Api {
        status: 400,
        message: "type must be one of the supported search types".to_owned()
    }));
    assert!(is_handled_empty_search_error(&ScrappaError::Api {
        status: 502,
        message: "Bad Gateway".to_owned()
    }));
    assert!(!is_handled_empty_search_error(&ScrappaError::Api {
        status: 503,
        message: "Service Unavailable".to_owned()
    }));
}

#[test]
fn retries_only_the_configured_upstream_statuses_and_timeouts() {
    for status in [408, 429, 500, 502, 503, 504] {
        assert!(is_retryable_scrappa_error(&ScrappaError::Api {
            status,
            message: "temporary".to_owned()
        }));
    }
    assert!(!is_retryable_scrappa_error(&ScrappaError::Api {
        status: 400,
        message: "invalid".to_owned()
    }));
    assert!(is_retryable_scrappa_error(&ScrappaError::Timeout));
    assert!(!is_retryable_scrappa_error(&ScrappaError::Request(
        "connection refused".to_owned()
    )));
    assert_eq!(scrappa_retry_delay(1).as_millis() >= 2000, true);
    assert_eq!(scrappa_retry_delay(1).as_millis() < 3000, true);
    assert_eq!(scrappa_retry_delay(2).as_millis() >= 4000, true);
    assert_eq!(scrappa_retry_delay(2).as_millis() < 5000, true);
}

#[test]
fn scrappa_error_messages_match_json_and_plain_text_responses() {
    assert_eq!(
        scrappa_error_message(
            r#"{ "message": "Invalid input", "errors": { "location": ["is required", "is invalid"] } }"#,
            "Unprocessable Entity"
        ),
        "Invalid input - location: is required, is invalid"
    );
    assert_eq!(
        scrappa_error_message(" upstream\n unavailable ", "Bad Gateway"),
        "upstream unavailable"
    );
    assert_eq!(scrappa_error_message("", "Bad Gateway"), "Bad Gateway");
}

fn ppe_run(max_total_charge: f64, prices: Value, charged: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": { "actorChargeEvents": prices }
            },
            "chargedEventCounts": charged,
            "options": { "maxTotalChargeUsd": max_total_charge }
        }
    })
}

#[test]
fn non_ppe_runs_publish_every_result_without_event_charges() {
    let pricing = ChargePricing::from_run(&json!({
        "data": { "pricingInfo": { "pricingModel": "PRICE_PER_DATASET_ITEM" } }
    }))
    .unwrap();
    let plan = pricing.plan_dataset_push(4);
    assert_eq!(plan.items_to_push, 4);
    assert_eq!(plan.custom_event_charge_count, 0);
    assert!(!plan.limit_reached);
    assert_eq!(plan.charged_count, 0);
    assert!(!plan.should_charge_custom_event);
}

#[test]
fn ppe_budget_accounts_for_prior_charges_and_both_dataset_event_prices() {
    let pricing = ChargePricing::from_run(&ppe_run(
        0.0025,
        json!({
            "property-result": { "eventPriceUsd": 0.001 },
            "apify-default-dataset-item": { "eventPriceUsd": 0.0002 },
            "apify-actor-start": { "eventPriceUsd": 0.0005 }
        }),
        json!({ "apify-actor-start": 1 }),
    ))
    .unwrap();
    let plan = pricing.plan_dataset_push(3);
    assert_eq!(plan.items_to_push, 1);
    assert_eq!(plan.custom_event_charge_count, 1);
    assert!(plan.limit_reached);
    assert_eq!(plan.charged_count, 2);
    assert!(plan.should_charge_custom_event);
}

#[test]
fn ppe_with_no_remaining_budget_keeps_the_sdk_single_item_limit_probe() {
    let pricing = ChargePricing::from_run(&ppe_run(
        0.001,
        json!({
            "property-result": { "eventPriceUsd": 0.001 }
        }),
        json!({}),
    ))
    .unwrap();
    let plan = pricing.plan_dataset_push(3);
    assert_eq!(plan.items_to_push, 1);
    assert_eq!(plan.custom_event_charge_count, 1);
    assert!(plan.limit_reached);
    assert_eq!(plan.charged_count, 2);
    assert!(plan.should_charge_custom_event);
}

#[test]
fn ppe_with_two_results_and_budget_for_one_reports_the_saved_item_count() {
    let pricing = ChargePricing::from_run(&ppe_run(
        0.001,
        json!({
            "property-result": { "eventPriceUsd": 0.001 }
        }),
        json!({}),
    ))
    .unwrap();
    let plan = pricing.plan_dataset_push(2);

    assert_eq!(plan.items_to_push, 1);
    assert!(plan.limit_reached);
    assert_eq!(plan.charged_count, 2);
    assert!(plan.items_to_push < 2);
}

#[test]
fn ppe_without_a_custom_event_keeps_rows_within_the_default_dataset_budget() {
    let pricing = ChargePricing::from_run(&ppe_run(
        0.003,
        json!({
            "apify-default-dataset-item": { "eventPriceUsd": 0.001 }
        }),
        json!({}),
    ))
    .unwrap();
    let plan = pricing.plan_dataset_push(5);
    assert_eq!(plan.items_to_push, 3);
    assert!(!plan.should_charge_custom_event);
    assert!(plan.limit_reached);
}

#[test]
fn endpoint_builder_keeps_configured_base_paths_and_encodes_segments() {
    let url = endpoint_url(
        "https://api.example.test/root",
        &["v2", "records", "key name"],
    )
    .unwrap();
    assert_eq!(
        url.as_str(),
        "https://api.example.test/root/v2/records/key%20name"
    );
}
