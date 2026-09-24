use crate::{apify_client::*, app::*, job::*, pricing::*, scrappa::*};
use anyhow::anyhow;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

#[test]
fn normalizes_linkedin_job_urls_and_removes_tracking_data() {
    assert_eq!(
        normalize_linkedin_job_url(
            "linkedin.com/jobs/view/1234567890/?trk=public_jobs_topcard-title"
        )
        .unwrap(),
        "https://www.linkedin.com/jobs/view/1234567890"
    );
    assert_eq!(
        normalize_linkedin_job_url(
            "https://de.linkedin.com/jobs/view/software-engineer-at-example-1234567890/extra"
        )
        .unwrap(),
        "https://www.linkedin.com/jobs/view/software-engineer-at-example-1234567890"
    );
    assert_eq!(
        normalize_linkedin_job_url("http://m.linkedin.com/jobs/view/1234567890?refId=abc").unwrap(),
        "http://www.linkedin.com/jobs/view/1234567890"
    );
}

#[test]
fn rejects_non_job_linkedin_urls_and_unsafe_hosts() {
    for url in [
        "https://www.linkedin.com/in/example",
        "https://example.com/jobs/view/123",
        "ftp://linkedin.com/jobs/view/123",
        "https://user:pass@linkedin.com/jobs/view/123",
        "https://linkedin.com:444/jobs/view/123",
        "   ",
    ] {
        assert_eq!(
            normalize_linkedin_job_url(url),
            Err(JOB_URL_ERROR.to_owned())
        );
    }
    assert_eq!(
        normalize_linkedin_job_url("https://%"),
        Err("Invalid URL".to_owned())
    );
}

#[test]
fn combines_legacy_and_batch_inputs_and_deduplicates_normalized_urls() {
    let input = json!({
        "url": "https://linkedin.com/jobs/view/1234567890",
        "urls": [
            "https://de.linkedin.com/jobs/view/software-engineer-2345678901/?trk=foo",
            "https://www.linkedin.com/jobs/view/1234567890/?refId=abc",
            "https://example.com/jobs/view/123",
            "https://example.com/jobs/view/123"
        ]
    });
    let requests = get_input_urls(Some(&input)).unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        requests[0].normalized_url.as_deref(),
        Some("https://www.linkedin.com/jobs/view/1234567890")
    );
    assert_eq!(
        requests[1].input_url,
        "https://de.linkedin.com/jobs/view/software-engineer-2345678901/?trk=foo"
    );
    assert_eq!(requests[2].normalized_url, None);
    assert_eq!(requests[2].validation_error.as_deref(), Some(JOB_URL_ERROR));
}

#[test]
fn rejects_non_string_batch_urls_and_ignores_non_string_legacy_url() {
    assert!(get_input_urls(Some(&json!({ "url": 4 })))
        .unwrap()
        .is_empty());
    assert_eq!(
        get_input_urls(Some(
            &json!({ "urls": ["https://linkedin.com/jobs/view/1", 2] })
        ))
        .unwrap_err()
        .to_string(),
        "LinkedIn job URLs must be strings"
    );
}

#[test]
fn actor_input_schema_keeps_legacy_and_batch_prefills() {
    let schema_path = format!("{}/.actor/input_schema.json", env!("CARGO_MANIFEST_DIR"));
    let schema: Value =
        serde_json::from_str(&std::fs::read_to_string(schema_path).unwrap()).unwrap();
    assert!(schema["properties"]["urls"]["prefill"].as_array().is_some());
    assert!(schema["properties"]["url"]["prefill"].is_string());
    assert_eq!(schema["properties"]["use_cache"]["default"], true);
    assert_eq!(
        schema["properties"]["maximum_cache_age"]["default"],
        2_592_000
    );
}

#[test]
fn builds_cache_params_only_for_valid_enabled_cache_settings() {
    let url = "https://www.linkedin.com/jobs/view/123";
    assert_eq!(
        build_job_params(url, Some(&json!(true)), Some(&json!(3600))),
        vec![
            ("url".to_owned(), url.to_owned()),
            ("use_cache".to_owned(), "1".to_owned()),
            ("maximum_cache_age".to_owned(), "3600".to_owned()),
        ]
    );
    for age in [json!(0), json!(-1), json!(1.5), json!("3600"), Value::Null] {
        assert_eq!(
            build_job_params(url, Some(&json!(true)), Some(&age)),
            vec![
                ("url".to_owned(), url.to_owned()),
                ("use_cache".to_owned(), "1".to_owned()),
            ]
        );
    }
    assert_eq!(
        build_job_params(url, Some(&json!(false)), Some(&json!(3600))),
        vec![("url".to_owned(), url.to_owned())]
    );
}

#[test]
fn scrappa_error_messages_keep_api_fields_and_fallback_text() {
    assert_eq!(
        scrappa_error_message(
            422,
            r#"{"message":"Invalid","errors":{"url":["bad","required"]}}"#
        ),
        "Invalid - url: bad, required"
    );
    assert_eq!(scrappa_error_message(503, ""), "HTTP 503");
    assert_eq!(scrappa_error_message(503, "Unavailable"), "Unavailable");
}

#[test]
fn success_items_preserve_response_and_fill_canonical_aliases() {
    let result = build_success_item(
        json!({
            "job_title": "Software Engineer",
            "company_name": "Example Corp",
            "date_posted": "2026-06-01",
            "applicant_count": "23 applicants",
            "application_url": "https://example.com/apply",
            "location": "New York, NY"
        }),
        "linkedin.com/jobs/view/123",
        "https://www.linkedin.com/jobs/view/123",
    )
    .unwrap();
    assert_eq!(result["success"], true);
    assert_eq!(result["title"], "Software Engineer");
    assert_eq!(result["company"], "Example Corp");
    assert_eq!(result["posted_date"], "2026-06-01");
    assert_eq!(result["applicants"], "23 applicants");
    assert_eq!(result["apply_url"], "https://example.com/apply");
    assert_eq!(result["location"], "New York, NY");
    assert_eq!(result["url"], "https://www.linkedin.com/jobs/view/123");
    assert_eq!(result["input_url"], "linkedin.com/jobs/view/123");
}

#[test]
fn success_items_preserve_canonical_values_over_aliases_and_fill_blank_strings() {
    let result = build_success_item(
        json!({
            "title": "Senior Engineer",
            "job_title": "Engineer",
            "company": "  ",
            "company_name": "Example Corp",
            "posted_date": "",
            "date_posted": "2026-06-01",
            "apply_url": null,
            "application_url": "https://example.com/apply",
            "url": " "
        }),
        "input",
        "https://www.linkedin.com/jobs/view/123",
    )
    .unwrap();
    assert_eq!(result["title"], "Senior Engineer");
    assert_eq!(result["company"], "Example Corp");
    assert_eq!(result["posted_date"], "2026-06-01");
    assert_eq!(result["apply_url"], "https://example.com/apply");
    assert_eq!(result["url"], "https://www.linkedin.com/jobs/view/123");
}

#[test]
fn failure_items_keep_scrappa_status_and_single_output_strips_wrapper_fields() {
    let missing = build_failure_item(
        "Scrappa API error (404): Not found",
        Some(404),
        "linkedin.com/jobs/view/missing",
        Some("https://www.linkedin.com/jobs/view/missing"),
    );
    assert_eq!(missing["success"], false);
    assert_eq!(missing["error_type"], "scrappa_api_error");
    assert_eq!(missing["message"], "Job not found");
    assert_eq!(missing["status_code"], 404);
    assert_eq!(
        build_output(&json!({
            "success": true,
            "title": "Software Engineer",
            "url": "https://www.linkedin.com/jobs/view/123",
            "input_url": "input",
            "normalized_url": "normalized",
            "error": "wrapper error",
            "error_type": "wrapper_error"
        })),
        json!({ "success": true, "title": "Software Engineer" })
    );
    assert_eq!(
        build_failure_item(JOB_URL_ERROR, None, "invalid", None),
        json!({
            "success": false,
            "input_url": "invalid",
            "error": JOB_URL_ERROR,
            "error_type": "error",
            "message": JOB_URL_ERROR
        })
    );
}

#[test]
fn only_scrappa_not_found_errors_are_recoverable_and_charge_requires_true() {
    assert!(is_recoverable_job_error(&anyhow!(ScrappaApiError {
        status: 404,
        message: "Not found".to_owned(),
    })));
    assert!(!is_recoverable_job_error(&anyhow!(ScrappaApiError {
        status: 401,
        message: "Unauthorized".to_owned(),
    })));
    assert!(!is_recoverable_job_error(&anyhow!("timeout")));
    assert!(should_charge_result(&json!({ "success": true })));
    assert!(!should_charge_result(&json!({ "success": "true" })));
    assert!(is_success(&json!({ "success": "true" })));
}

fn ppe_run(max_total: f64, charged_event_counts: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {
                    "actorChargeEvents": {
                        "job-result": { "eventPriceUsd": 0.001 },
                        "apify-default-dataset-item": { "eventPriceUsd": 0.0002 }
                    }
                }
            },
            "chargedEventCounts": charged_event_counts,
            "options": { "maxTotalChargeUsd": max_total }
        }
    })
}

fn tiered_ppe_run(max_total: f64, charged_event_counts: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {
                    "actorChargeEvents": {
                        "job-result": { "eventTieredPricingUsd": {
                            "FREE": { "tieredEventPriceUsd": 0.001 },
                            "BRONZE": { "tieredEventPriceUsd": 0.0008 },
                            "GOLD": { "tieredEventPriceUsd": 0.0005 }
                        }},
                        "apify-default-dataset-item": { "eventTieredPricingUsd": {
                            "FREE": { "tieredEventPriceUsd": 0.0002 },
                            "BRONZE": { "tieredEventPriceUsd": 0.00016 },
                            "GOLD": { "tieredEventPriceUsd": 0.0001 }
                        }}
                    }
                }
            },
            "chargedEventCounts": charged_event_counts,
            "options": { "maxTotalChargeUsd": max_total }
        }
    })
}

#[test]
fn ppe_budget_includes_custom_and_default_dataset_item_charges() {
    let run = ppe_run(0.0012, json!({}));
    let mut pricing = PricingState::from_run(&run).unwrap();
    assert!(pricing.should_push_item(Some(JOB_RESULT_CHARGE_EVENT)));
    let custom = pricing.register_charge(JOB_RESULT_CHARGE_EVENT, 1);
    let dataset = pricing.register_charge(DEFAULT_DATASET_ITEM_EVENT, 1);
    assert_eq!(custom.charged_count, 1);
    assert_eq!(dataset.charged_count, 1);
    assert!(pricing.event_charge_limit_reached(JOB_RESULT_CHARGE_EVENT));
    assert!(pricing.event_charge_limit_reached(DEFAULT_DATASET_ITEM_EVENT));
}

#[test]
fn ppe_zero_limit_is_unbounded_and_missing_or_null_limits_are_unbounded() {
    let zero_limit = PricingState::from_run(&ppe_run(0.0, json!({}))).unwrap();
    assert_eq!(zero_limit.max_total_charge_usd, f64::INFINITY);
    assert_eq!(zero_limit.max_charges_for_price(0.0012), None);
    assert!(zero_limit.should_push_item(Some(JOB_RESULT_CHARGE_EVENT)));

    let mut missing_limit = ppe_run(1.0, json!({}));
    missing_limit["data"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("maxTotalChargeUsd");
    let missing_limit = PricingState::from_run(&missing_limit).unwrap();
    assert_eq!(missing_limit.max_total_charge_usd, f64::INFINITY);

    let mut null_limit = ppe_run(1.0, json!({}));
    null_limit["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
    let null_limit = PricingState::from_run(&null_limit).unwrap();
    assert_eq!(null_limit.max_total_charge_usd, f64::INFINITY);

    let positive_limit = PricingState::from_run(&ppe_run(0.0012, json!({}))).unwrap();
    assert_eq!(positive_limit.max_total_charge_usd, 0.0012);
    assert_eq!(positive_limit.item_limit(Some(JOB_RESULT_CHARGE_EVENT)), 1);

    let already_charged = PricingState::from_run(&ppe_run(
        0.0012,
        json!({
            "job-result": 1,
            "apify-default-dataset-item": 1
        }),
    ))
    .unwrap();
    assert_eq!(already_charged.total_charged_amount(), 0.0012);
}

#[test]
fn tiered_ppe_budget_uses_a_conservative_price_for_each_event() {
    let run = tiered_ppe_run(0.0012, json!({}));
    let pricing = PricingState::from_run(&run).unwrap();

    assert_eq!(pricing.event_price(JOB_RESULT_CHARGE_EVENT), 0.001);
    assert_eq!(pricing.event_price(DEFAULT_DATASET_ITEM_EVENT), 0.0002);
    assert!(pricing.should_push_item(Some(JOB_RESULT_CHARGE_EVENT)));
}

#[test]
fn malformed_tiered_prices_fail_closed() {
    let mut run = tiered_ppe_run(0.0012, json!({}));
    run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]["job-result"]
        ["eventTieredPricingUsd"]["BRONZE"] = json!({});

    assert!(PricingState::from_run(&run).is_err());
}

#[tokio::test]
async fn actor_run_charges_successful_results_with_tiered_pricing() {
    let input = json!({ "url": "https://linkedin.com/jobs/view/1234567890" });
    let run_info = tiered_ppe_run(0.0012, json!({}));
    let (base, server) = start_actor_mock(input, run_info, 6).await;
    let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
    let config = mock_config(&base);
    let apify = ApifyClient::new(http.clone(), &config.apify);

    run(&config, &apify, http).await.unwrap();

    let requests = server.await.unwrap();
    let charge_requests: Vec<_> = requests
        .iter()
        .filter(|request| request_line(request).starts_with("POST /v2/actor-runs/test-run/charge"))
        .collect();
    assert_eq!(charge_requests.len(), 1);
    assert_eq!(
        request_body(charge_requests[0]),
        json!({ "eventName": "job-result", "count": 1 })
    );
}

#[test]
fn ppe_budget_skips_rows_when_the_combined_charge_exceeds_the_limit() {
    let exact_limit = ppe_run(
        0.0012,
        json!({
            "job-result": 1,
            "apify-default-dataset-item": 1
        }),
    );
    let pricing = PricingState::from_run(&exact_limit).unwrap();
    assert!(!pricing.should_push_item(Some(JOB_RESULT_CHARGE_EVENT)));

    let insufficient_remaining_budget = ppe_run(0.0011, json!({}));
    let pricing = PricingState::from_run(&insufficient_remaining_budget).unwrap();
    assert!(!pricing.should_push_item(Some(JOB_RESULT_CHARGE_EVENT)));

    let mut charge_limited = PricingState::from_run(&ppe_run(0.0005, json!({}))).unwrap();
    let charge = charge_limited.register_charge(JOB_RESULT_CHARGE_EVENT, 1);
    assert_eq!(charge.charged_count, 0);

    let over_limit = ppe_run(
        0.0011,
        json!({
            "job-result": 1,
            "apify-default-dataset-item": 1
        }),
    );
    let pricing = PricingState::from_run(&over_limit).unwrap();
    assert!(!pricing.should_push_item(Some(JOB_RESULT_CHARGE_EVENT)));
}

#[test]
fn free_runs_do_not_limit_rows_or_charge_named_events() {
    let run = json!({ "data": { "pricingInfo": { "pricingModel": "FREE" } } });
    let pricing = PricingState::from_run(&run).unwrap();
    assert!(!pricing.is_pay_per_event);
    assert!(pricing.should_push_item(Some(JOB_RESULT_CHARGE_EVENT)));
}

#[test]
fn apify_retries_only_bounded_transient_responses() {
    assert_eq!(
        apify_retry_delay("GET", StatusCode::TOO_MANY_REQUESTS, 0),
        Some(Duration::from_secs(1))
    );
    assert_eq!(
        apify_retry_delay("PUT", StatusCode::INTERNAL_SERVER_ERROR, 1),
        Some(Duration::from_secs(2))
    );
    assert_eq!(
        apify_retry_delay("GET", StatusCode::SERVICE_UNAVAILABLE, 2),
        None
    );
    assert_eq!(
        apify_retry_delay("DELETE", StatusCode::BAD_REQUEST, 0),
        None
    );
}

#[tokio::test]
async fn charge_retries_reuse_idempotency_key_and_dataset_posts_are_not_retried() {
    let (base, charge_server) = start_response_sequence(vec![503, 200]).await;
    let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
    let config = mock_config(&base);
    let apify = ApifyClient::new(http, &config.apify);
    let idempotency_key = "test-run-job-result-4";

    apify
        .charge_event(JOB_RESULT_CHARGE_EVENT, 1, idempotency_key)
        .await
        .unwrap();

    let charge_requests = charge_server.await.unwrap();
    assert_eq!(charge_requests.len(), 2);
    assert!(charge_requests.iter().all(|request| {
        request_line(request).starts_with("POST /v2/actor-runs/test-run/charge ")
            && request_text(request).contains("idempotency-key: test-run-job-result-4")
    }));

    let (base, dataset_server) = start_response_sequence(vec![503]).await;
    let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
    let config = mock_config(&base);
    let apify = ApifyClient::new(http, &config.apify);
    let error = apify
        .push_dataset_item(&json!({ "success": true }))
        .await
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("Apify dataset item publication failed (503)"));
    assert_eq!(dataset_server.await.unwrap().len(), 1);
}

#[tokio::test]
async fn actor_run_preserves_auth_batch_results_dataset_output_and_success_charges() {
    let input = json!({
        "url": "linkedin.com/jobs/view/missing",
        "urls": [
            "https://de.linkedin.com/jobs/view/missing/?trk=foo",
            "https://www.linkedin.com/jobs/view/1234567890?refId=abc"
        ],
        "use_cache": true,
        "maximum_cache_age": 3600
    });
    let (base, server) = start_actor_mock(input, ppe_run(1.0, json!({})), 8).await;
    let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
    let config = mock_config(&base);
    let apify = ApifyClient::new(http.clone(), &config.apify);

    let status_message = run(&config, &apify, http).await.unwrap();
    assert_eq!(status_message, None);

    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 8);
    assert!(requests
        .iter()
        .filter(|request| !request_line(request).contains("/api/linkedin/job?"))
        .all(|request| request_text(request).contains("authorization: bearer test-token")));
    let scrappa_requests: Vec<_> = requests
        .iter()
        .filter(|request| request_line(request).contains("GET /api/linkedin/job?"))
        .collect();
    assert_eq!(scrappa_requests.len(), 2);
    assert!(request_text(scrappa_requests[0]).contains("x-api-key: test-api-key"));
    assert!(request_line(scrappa_requests[1]).contains("use_cache=1"));
    assert!(request_line(scrappa_requests[1]).contains("maximum_cache_age=3600"));

    let dataset_requests: Vec<_> = requests
        .iter()
        .filter(|request| request_line(request).starts_with("POST /v2/datasets/test-dataset/items"))
        .collect();
    assert_eq!(dataset_requests.len(), 2);
    assert_eq!(request_body(dataset_requests[0])["status_code"], 404);
    assert_eq!(
        request_body(dataset_requests[0])["message"],
        "Job not found"
    );
    assert_eq!(
        request_body(dataset_requests[1])["title"],
        "Senior Engineer"
    );

    let charge_requests: Vec<_> = requests
        .iter()
        .filter(|request| request_line(request).starts_with("POST /v2/actor-runs/test-run/charge"))
        .collect();
    assert_eq!(charge_requests.len(), 1);
    assert_eq!(
        request_body(charge_requests[0]),
        json!({
            "eventName": "job-result",
            "count": 1
        })
    );
    assert!(request_text(charge_requests[0]).contains("idempotency-key: test-run-job-result-1"));

    let output = requests
        .iter()
        .find(|request| {
            request_line(request).starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT")
        })
        .unwrap();
    assert_eq!(
        request_body(output),
        json!({ "requested": 2, "succeeded": 1, "failed": 1 })
    );
    assert!(requests
        .iter()
        .all(|request| { !request_line(request).starts_with("PUT /v2/actor-runs/test-run ") }));
}

#[tokio::test]
async fn actor_run_zero_ppe_limit_is_unbounded_and_publishes_results() {
    let input = json!({
        "urls": [
            "linkedin.com/jobs/view/first",
            "linkedin.com/jobs/view/second"
        ]
    });
    let (base, server) = start_actor_mock(input, ppe_run(0.0, json!({})), 9).await;
    let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
    let config = mock_config(&base);
    let apify = ApifyClient::new(http.clone(), &config.apify);

    assert_eq!(run(&config, &apify, http).await.unwrap(), None);

    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 9);
    assert_eq!(
        requests
            .iter()
            .filter(|request| request_line(request).contains("GET /api/linkedin/job?"))
            .count(),
        2
    );

    let dataset_requests: Vec<_> = requests
        .iter()
        .filter(|request| {
            request_line(request).starts_with("POST /v2/datasets/test-dataset/items ")
        })
        .collect();
    assert_eq!(dataset_requests.len(), 2);
    assert_eq!(
        request_body(dataset_requests[0])["title"],
        "Senior Engineer"
    );
    assert_eq!(
        request_body(dataset_requests[1])["title"],
        "Senior Engineer"
    );

    let charge_requests: Vec<_> = requests
        .iter()
        .filter(|request| request_line(request).starts_with("POST /v2/actor-runs/test-run/charge "))
        .collect();
    assert_eq!(charge_requests.len(), 2);
    for (index, request) in charge_requests.iter().enumerate() {
        assert_eq!(
            request_body(request),
            json!({ "eventName": "job-result", "count": 1 })
        );
        assert!(request_text(request)
            .contains(&format!("idempotency-key: test-run-job-result-{index}")));
    }

    let output = requests
        .iter()
        .find(|request| {
            request_line(request).starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT")
        })
        .unwrap();
    assert_eq!(
        request_body(output),
        json!({ "requested": 2, "succeeded": 2, "failed": 0 })
    );
    assert!(!requests
        .iter()
        .any(|request| request_line(request).starts_with("PUT /v2/actor-runs/test-run ")));
}

#[tokio::test]
async fn actor_run_stops_at_the_ppe_limit_and_sets_terminal_status() {
    let input = json!({
        "urls": [
            "linkedin.com/jobs/view/first",
            "linkedin.com/jobs/view/second"
        ]
    });
    let (base, server) = start_actor_mock(input, ppe_run(0.0012, json!({})), 7).await;
    let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
    let config = mock_config(&base);
    let apify = ApifyClient::new(http.clone(), &config.apify);

    let status_message = run(&config, &apify, http).await.unwrap().unwrap();
    assert_eq!(
        status_message,
        "Charge limit reached after saving 1 of 1 LinkedIn job detail results."
    );
    apify.set_terminal_status_message(&status_message).await;

    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 7);
    assert_eq!(
        requests
            .iter()
            .filter(|request| request_line(request).contains("GET /api/linkedin/job?"))
            .count(),
        1
    );
    let output = requests
        .iter()
        .find(|request| {
            request_line(request).starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT")
        })
        .unwrap();
    assert_eq!(
        request_body(output),
        json!({ "requested": 2, "succeeded": 1, "failed": 0 })
    );
    let status = requests
        .iter()
        .find(|request| request_line(request).starts_with("PUT /v2/actor-runs/test-run "))
        .unwrap();
    assert_eq!(request_body(status)["statusMessage"], status_message);
    assert_eq!(request_body(status)["isStatusMessageTerminal"], true);
}

#[tokio::test]
async fn actor_run_does_not_publish_when_combined_charge_exceeds_limit() {
    let input = json!({ "url": "linkedin.com/jobs/view/1234567890" });
    let (base, server) = start_actor_mock(input, ppe_run(0.0011, json!({})), 4).await;
    let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
    let config = mock_config(&base);
    let apify = ApifyClient::new(http.clone(), &config.apify);

    let status_message = run(&config, &apify, http).await.unwrap().unwrap();
    assert_eq!(
        status_message,
        "Charge limit reached after saving 0 of 1 LinkedIn job detail results."
    );

    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 4);
    assert_eq!(
        requests
            .iter()
            .filter(|request| request_line(request)
                .starts_with("POST /v2/datasets/test-dataset/items "))
            .count(),
        0
    );
    assert_eq!(
        requests
            .iter()
            .filter(
                |request| request_line(request).starts_with("POST /v2/actor-runs/test-run/charge ")
            )
            .count(),
        0
    );
    let output = requests
        .iter()
        .find(|request| {
            request_line(request).starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT")
        })
        .unwrap();
    assert_eq!(
        request_body(output),
        json!({ "requested": 1, "succeeded": 0, "failed": 0 })
    );
}

fn mock_config(base: &str) -> Config {
    Config {
        apify: ApifyConfig {
            api_base: base.to_owned(),
            token: "test-token".to_owned(),
            actor_run_id: "test-run".to_owned(),
            key_value_store_id: "test-store".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            input_key: "INPUT".to_owned(),
        },
        scrappa_api_base: format!("{base}/api"),
        scrappa_api_key: "test-api-key".to_owned(),
    }
}

async fn start_response_sequence(statuses: Vec<u16>) -> (String, JoinHandle<Vec<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for status in statuses {
            let accepted = tokio::time::timeout(Duration::from_secs(5), listener.accept()).await;
            let Ok(Ok((mut stream, _))) = accepted else {
                break;
            };
            let request = read_http_request(&mut stream).await;
            let reason = if status == 200 {
                "OK"
            } else if status == 503 {
                "Service Unavailable"
            } else {
                "Bad Request"
            };
            let body = if status == 200 {
                "{}"
            } else {
                r#"{"message":"temporary"}"#
            };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            requests.push(request);
        }
        requests
    });
    (format!("http://{address}"), server)
}

async fn start_actor_mock(
    input: Value,
    pricing: Value,
    expected_requests: usize,
) -> (String, JoinHandle<Vec<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for _ in 0..expected_requests {
            let accepted = tokio::time::timeout(Duration::from_secs(5), listener.accept()).await;
            let Ok(Ok((mut stream, _))) = accepted else {
                break;
            };
            let request = read_http_request(&mut stream).await;
            let line = request_line(&request).to_owned();
            let (status, body) = if line.starts_with("GET /v2/actor-runs/test-run ") {
                (200, pricing.to_string())
            } else if line.starts_with("GET /v2/key-value-stores/test-store/records/INPUT ") {
                (200, input.to_string())
            } else if line.starts_with("GET /api/linkedin/job?") {
                if line.contains("missing") {
                    (404, json!({ "message": "Not found" }).to_string())
                } else {
                    (
                        200,
                        json!({
                            "success": true,
                            "title": "Senior Engineer",
                            "job_title": "Engineer",
                            "company": "Example Corp"
                        })
                        .to_string(),
                    )
                }
            } else if line.starts_with("POST /v2/datasets/test-dataset/items ")
                || line.starts_with("POST /v2/actor-runs/test-run/charge ")
                || line.starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT ")
                || line.starts_with("PUT /v2/actor-runs/test-run ")
            {
                (200, "{}".to_owned())
            } else {
                (
                    404,
                    json!({ "message": "Unexpected mock request", "path": line }).to_string(),
                )
            };
            let response = format!(
                "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                if status == 200 { "OK" } else { "Not Found" },
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            requests.push(request);
        }
        requests
    });
    (format!("http://{address}"), server)
}

async fn read_http_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut request = Vec::new();
    let mut chunk = [0; 2048];
    loop {
        let read = stream.read(&mut chunk).await.unwrap();
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
        let Some(body_start) = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
        else {
            continue;
        };
        let headers = String::from_utf8_lossy(&request[..body_start]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap_or_default())
            })
            .unwrap_or_default();
        if request.len() >= body_start + content_length {
            break;
        }
    }
    request
}

fn request_line(request: &[u8]) -> &str {
    std::str::from_utf8(request)
        .unwrap()
        .lines()
        .next()
        .unwrap_or_default()
}

fn request_text(request: &[u8]) -> String {
    String::from_utf8_lossy(request).to_lowercase()
}

fn request_body(request: &[u8]) -> Value {
    let body_start = request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
        .unwrap();
    serde_json::from_slice(&request[body_start..]).unwrap()
}
