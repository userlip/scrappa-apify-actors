use super::*;
use crate::apify::{
    affordable_dataset_items, apify_retry_delay, retryable_apify_status, send_apify_request,
    ApifyClient, APIFY_MAX_RETRIES,
};
use crate::scrappa::{build_photos_url, photo_results, ScrappaClient};
use crate::test_support::{header_value, mock_response, request_body, MockServer};
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::time::Duration;
use url::Url;

fn config(base_url: &Url) -> ActorConfig {
    let mut scrappa_api_base_url = base_url.clone();
    scrappa_api_base_url
        .path_segments_mut()
        .unwrap()
        .extend(["api"]);
    ActorConfig {
        apify_api_base_url: base_url.clone(),
        scrappa_api_base_url,
        default_key_value_store_id: "store-id".to_owned(),
        default_dataset_id: "dataset-id".to_owned(),
        actor_run_id: "run-id".to_owned(),
        input_key: "INPUT".to_owned(),
        apify_token: "test-apify-token".to_owned(),
        scrappa_api_key: "test-scrappa-key".to_owned(),
        max_total_charge_usd: None,
    }
}

fn pricing_run(max_charge: f64, counts: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {
                    "actorChargeEvents": {
                        "apify-default-dataset-item": { "eventPriceUsd": 0.05 },
                        "apify-actor-start": { "eventPriceUsd": 0.02 }
                    }
                }
            },
            "options": { "maxTotalChargeUsd": max_charge },
            "chargedEventCounts": counts
        }
    })
}

#[test]
fn ppe_budget_accounts_for_other_charges_and_trims_dataset_items() {
    let run = pricing_run(0.12, json!({ "apify-actor-start": 1 }));
    assert_eq!(affordable_dataset_items(&run, 5, None, 0).unwrap(), Some(2));

    let partially_caught_up_run = pricing_run(
        0.17,
        json!({
            "apify-actor-start": 1,
            "apify-default-dataset-item": 1
        }),
    );
    assert_eq!(
        affordable_dataset_items(&partially_caught_up_run, 5, None, 2).unwrap(),
        Some(1)
    );
}

#[test]
fn non_ppe_pricing_keeps_all_default_dataset_items() {
    let run = json!({ "data": { "pricingInfo": { "pricingModel": "PRICE_PER_DATASET_ITEM" } } });
    assert_eq!(affordable_dataset_items(&run, 5, None, 0).unwrap(), None);
}

#[test]
fn exhausted_ppe_budget_allows_no_chargeable_dataset_items() {
    let run = pricing_run(0.02, json!({ "apify-actor-start": 1 }));
    assert_eq!(affordable_dataset_items(&run, 5, None, 0).unwrap(), Some(0));
}

#[test]
fn ppe_budget_ignores_unpriced_events_and_treats_zero_as_unlimited() {
    let run = pricing_run(
        0.12,
        json!({ "apify-actor-start": 1, "unpriced-event": 100 }),
    );
    assert_eq!(affordable_dataset_items(&run, 5, None, 0).unwrap(), Some(2));

    let unlimited = pricing_run(0.0, json!({ "apify-actor-start": 100 }));
    assert_eq!(
        affordable_dataset_items(&unlimited, 5, Some(0.01), 0).unwrap(),
        Some(5)
    );
}

#[tokio::test]
async fn ppe_budget_tracks_dataset_rows_when_run_charges_are_stale_across_businesses() {
    let input = json!({ "business_ids": ["ChIJone", "ChIJtwo"] });
    let photos = json!({
        "items": [
            { "photo_id": "p1" },
            { "photo_id": "p2" },
            { "photo_id": "p3" }
        ]
    })
    .to_string();
    let stale_run = pricing_run(0.12, json!({ "apify-actor-start": 1 })).to_string();
    let server = MockServer::start(vec![
        mock_response(200, input.to_string()),
        mock_response(200, photos.clone()),
        mock_response(200, stale_run.clone()),
        mock_response(201, ""),
        mock_response(200, photos),
        mock_response(200, stale_run),
        mock_response(201, ""),
        mock_response(201, ""),
    ]);
    let config = config(&server.base_url);
    let client = reqwest::Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .unwrap();

    run_actor(&client, &config).await.unwrap();

    let requests = server.requests();
    let dataset_items_written = requests
        .iter()
        .filter(|request| request.starts_with("POST /v2/datasets/dataset-id/items "))
        .map(|request| request_body(request).as_array().unwrap().len())
        .sum::<usize>();
    assert_eq!(dataset_items_written, 2);
    assert!(requests
        .last()
        .unwrap()
        .starts_with("PUT /v2/key-value-stores/store-id/records/OUTPUT "));
}

#[test]
fn apify_retries_match_the_client_default_backoff() {
    assert_eq!(APIFY_MAX_RETRIES, 8);
    assert_eq!(apify_retry_delay(1), Duration::from_millis(500));
    assert_eq!(apify_retry_delay(2), Duration::from_secs(1));
    assert_eq!(apify_retry_delay(3), Duration::from_secs(2));
    assert!(retryable_apify_status(StatusCode::TOO_MANY_REQUESTS));
    assert!(retryable_apify_status(StatusCode::INTERNAL_SERVER_ERROR));
    assert!(!retryable_apify_status(StatusCode::BAD_REQUEST));
}

#[test]
fn maps_request_keeps_cache_query_compatibility() {
    let base_url = Url::parse("https://scrappa.example/api").unwrap();
    let without_cache = build_photos_url(
        &base_url,
        "0x123:0x456",
        Some(&json!({ "use_cache": false, "maximum_cache_age": 0 })),
    )
    .unwrap();
    assert_eq!(
        without_cache.query(),
        Some("business_id=0x123%3A0x456&maximum_cache_age=0")
    );

    let defaults = build_photos_url(&base_url, "ChIJtest", None).unwrap();
    assert_eq!(defaults.query(), Some("business_id=ChIJtest&use_cache=1"));
}

#[test]
fn wrapped_and_direct_photo_responses_keep_pagination_context() {
    let wrapped = json!({ "items": [{ "photo_id": "one" }], "data": [{ "photo_id": "fallback" }], "nextPage": "cursor" });
    let (photos, next_page) = photo_results(&wrapped).unwrap();
    assert_eq!(photos, vec![json!({ "photo_id": "one" })]);
    assert_eq!(next_page, json!("cursor"));

    let (photos, next_page) = photo_results(&json!([{ "photo_id": "direct" }])).unwrap();
    assert_eq!(photos, vec![json!({ "photo_id": "direct" })]);
    assert_eq!(next_page, Value::Null);
}

#[test]
fn mapped_dataset_photo_keeps_upstream_fields_and_overrides_business_ids() {
    assert_eq!(
        dataset_photo(
            &json!({ "photo_id": "p1", "business_id": "upstream" }),
            "input-value",
            "normalized-value"
        ),
        json!({
            "photo_id": "p1",
            "business_id": "normalized-value",
            "input_business_id": "input-value"
        })
    );
}

#[tokio::test]
async fn single_business_keeps_prefill_semantics_output_shape_auth_and_ppe_cap() {
    let input = json!({
        "business_id": "0x123:0x456",
        "use_cache": true,
        "maximum_cache_age": 0
    });
    let run = pricing_run(0.12, json!({ "apify-actor-start": 1 }));
    let server = MockServer::start(vec![
        mock_response(200, input.to_string()),
        mock_response(
            200,
            json!({
                "items": [
                    { "photo_id": "p1", "photo_url": "https://example.test/1" },
                    { "photo_id": "p2", "photo_url": "https://example.test/2" },
                    { "photo_id": "p3", "photo_url": "https://example.test/3" }
                ],
                "nextPage": "next-cursor"
            })
            .to_string(),
        ),
        mock_response(200, run.to_string()),
        mock_response(201, ""),
        mock_response(201, ""),
    ]);
    let config = config(&server.base_url);
    let client = reqwest::Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .unwrap();

    run_actor(&client, &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    assert!(requests[0].starts_with("GET /v2/key-value-stores/store-id/records/INPUT "));
    assert_eq!(
        header_value(&requests[0], "authorization"),
        Some("Bearer test-apify-token")
    );
    assert!(requests[1].starts_with(
        "GET /api/maps/photos?business_id=0x123%3A0x456&use_cache=1&maximum_cache_age=0 "
    ));
    assert_eq!(
        header_value(&requests[1], "x-api-key"),
        Some("test-scrappa-key")
    );
    assert_eq!(
        header_value(&requests[1], "accept"),
        Some("application/json")
    );
    assert!(requests[2].starts_with("GET /v2/actor-runs/run-id "));
    assert_eq!(
        request_body(&requests[3]),
        json!([
            {
                "photo_id": "p1",
                "photo_url": "https://example.test/1",
                "input_business_id": "0x123:0x456",
                "business_id": "0x123:0x456"
            },
            {
                "photo_id": "p2",
                "photo_url": "https://example.test/2",
                "input_business_id": "0x123:0x456",
                "business_id": "0x123:0x456"
            }
        ])
    );
    let output = request_body(&requests[4]);
    assert!(requests[4].starts_with("PUT /v2/key-value-stores/store-id/records/OUTPUT "));
    assert_eq!(output["total"], 3);
    assert_eq!(output["nextPage"], "next-cursor");
    assert_eq!(output["photos"], request_body(&requests[3]));
    assert_eq!(output["photos"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn apify_get_and_output_put_retry_but_dataset_post_and_scrappa_do_not() {
    let server = MockServer::start(vec![
        mock_response(500, "temporary"),
        mock_response(200, "{\"ok\":true}"),
    ]);
    let client = reqwest::Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .unwrap();
    let request = client.get(server.base_url.join("retry").unwrap());
    let response = send_apify_request(&client, request).await.unwrap();
    assert!(response.status().is_success());
    assert_eq!(server.requests().len(), 2);

    let post_server = MockServer::start(vec![
        mock_response(500, "temporary after possible append"),
        mock_response(201, ""),
    ]);
    let post_request = client
        .post(post_server.base_url.join("dataset/items").unwrap())
        .json(&json!([{ "photo_id": "one" }]));
    let response = send_apify_request(&client, post_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(post_server.requests().len(), 1);

    let put_server = MockServer::start(vec![
        mock_response(500, "temporary"),
        mock_response(201, ""),
    ]);
    let put_request = client
        .put(put_server.base_url.join("records/OUTPUT").unwrap())
        .json(&json!({ "photos": [] }));
    let response = send_apify_request(&client, put_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(put_server.requests().len(), 2);

    let error_server = MockServer::start(vec![mock_response(500, "upstream unavailable")]);
    let mut scrappa_base_url = error_server.base_url.clone();
    scrappa_base_url
        .path_segments_mut()
        .unwrap()
        .extend(["api"]);
    let scrappa = ScrappaClient::new(&client, &scrappa_base_url, "test-scrappa-key");
    let error = scrappa.get_photos("0x123:0x456", None).await.unwrap_err();
    assert!(error.to_string().contains("Scrappa API error (500)"));
    assert_eq!(error_server.requests().len(), 1);
}

#[tokio::test]
async fn dataset_append_transient_failure_is_not_retried() {
    let run = pricing_run(1.0, json!({ "apify-actor-start": 1 }));
    let server = MockServer::start(vec![
        mock_response(200, run.to_string()),
        mock_response(500, "append may already have succeeded"),
        mock_response(201, ""),
    ]);
    let config = config(&server.base_url);
    let client = reqwest::Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .unwrap();

    let mut apify = ApifyClient::new(&client, &config);
    let result = apify
        .push_dataset_items(&[json!({ "photo_id": "one" })])
        .await;

    assert!(result.is_err());
    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].starts_with("POST /v2/datasets/dataset-id/items "));
}

#[tokio::test]
async fn missing_apify_input_uses_the_actor_input_validation_error() {
    let server = MockServer::start(vec![mock_response(404, "input not found")]);
    let config = config(&server.base_url);
    let client = reqwest::Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .unwrap();

    let error = run_actor(&client, &config).await.unwrap_err();
    assert!(error.to_string().contains(
        "At least one Business ID is required. Provide business_ids or legacy business_id."
    ));
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn input_api_errors_stay_structured_and_preserve_summary_counts() {
    let input = json!({ "business_ids": ["not-found", "invalid"] });
    let run = pricing_run(1.0, json!({ "apify-actor-start": 1 }));
    let server = MockServer::start(vec![
        mock_response(200, input.to_string()),
        mock_response(404, "not found"),
        mock_response(200, run.to_string()),
        mock_response(201, ""),
        mock_response(422, "invalid"),
        mock_response(200, run.to_string()),
        mock_response(201, ""),
        mock_response(201, ""),
    ]);
    let config = config(&server.base_url);
    let client = reqwest::Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .unwrap();

    run_actor(&client, &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 8);
    assert_eq!(request_body(&requests[3])[0]["error"], "Business not found");
    assert_eq!(request_body(&requests[6])[0]["error"], "Invalid input");
    let output = request_body(&requests[7]);
    assert_eq!(output["requested"], 2);
    assert_eq!(output["succeeded"], 0);
    assert_eq!(output["failed"], 2);
    assert_eq!(output["total_photos"], 0);
    assert_eq!(output["results"][0]["error"], "Business not found");
    assert_eq!(output["results"][1]["error"], "Invalid input");
}

#[tokio::test]
async fn non_input_scrappa_errors_fail_without_writing_output() {
    let input = json!({ "business_id": "0x123:0x456" });
    let server = MockServer::start(vec![
        mock_response(200, input.to_string()),
        mock_response(503, "upstream unavailable"),
    ]);
    let config = config(&server.base_url);
    let client = reqwest::Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .unwrap();

    let error = run_actor(&client, &config).await.unwrap_err();
    assert!(error.to_string().contains("Scrappa API error (503)"));
    assert_eq!(server.requests().len(), 2);
}
