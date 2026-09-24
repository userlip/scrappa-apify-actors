use std::time::Duration;

use crate::apify::*;
use crate::doctor_details::*;
use crate::scrappa::*;
use reqwest::StatusCode;
use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};
const MARKUS_URL: &str = "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin";

struct MockRequest {
    head: String,
    body: String,
}

async fn start_mock_server(
    responses: Vec<(u16, String)>,
) -> (String, JoinHandle<Vec<MockRequest>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 2048];
            let body_start = loop {
                let read = stream.read(&mut buffer).await.unwrap();
                if read == 0 {
                    panic!("mock client closed before request completed");
                }
                request.extend_from_slice(&buffer[..read]);
                let Some(position) = request.windows(4).position(|window| window == b"\r\n\r\n")
                else {
                    continue;
                };
                let body_start = position + 4;
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
                    break body_start;
                }
            };
            let headers = String::from_utf8_lossy(&request[..body_start]).to_string();
            let request_body = String::from_utf8_lossy(&request[body_start..]).to_string();
            requests.push(MockRequest {
                head: headers,
                body: request_body,
            });
            let reason = match status {
                200 => "OK",
                201 => "Created",
                400 => "Bad Request",
                429 => "Too Many Requests",
                500 => "Internal Server Error",
                503 => "Service Unavailable",
                _ => "Mock Response",
            };
            let response_headers = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
            stream.write_all(response_headers.as_bytes()).await.unwrap();
            stream.write_all(body.as_bytes()).await.unwrap();
        }
        requests
    });
    (format!("http://{address}"), server)
}

fn mock_apify_client(base_url: String) -> ApifyClient {
    ApifyClient::new(ApifyConfig {
        api_base: base_url,
        token: "test-token".to_owned(),
        run_id: "test-run".to_owned(),
        key_value_store_id: "store".to_owned(),
        dataset_id: "dataset".to_owned(),
        input_key: "INPUT".to_owned(),
    })
    .unwrap()
}

fn mock_ppe_run(max_total_charge: Value, counts: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "doctor-profile-result": {"eventPriceUsd": 0.001},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                    "other-event": {"eventPriceUsd": 0.0002}
                }}
            },
            "chargedEventCounts": counts,
            "options": {"maxTotalChargeUsd": max_total_charge}
        }
    })
}

#[test]
fn normalizes_full_urls_paths_queries_and_host_style_urls() {
    assert_eq!(
        clean_jameda_doctor_url(
            &json!(format!(" {MARKUS_URL}?utm_source=test ")),
            "doctorUrl"
        )
        .unwrap(),
        MARKUS_URL
    );
    assert_eq!(
        clean_jameda_doctor_url(&json!("/markus-lietzau-msc/zahnarzt/berlin/"), "doctorUrl")
            .unwrap(),
        MARKUS_URL
    );
    assert_eq!(
        clean_jameda_doctor_url(
            &json!("jameda.de/markus-lietzau-msc/zahnarzt/berlin"),
            "doctorUrl"
        )
        .unwrap(),
        MARKUS_URL
    );
    assert_eq!(
        clean_jameda_doctor_url(
            &json!("http://jameda.de/markus-lietzau-msc/zahnarzt/berlin"),
            "doctorUrl"
        )
        .unwrap(),
        MARKUS_URL
    );
    assert_eq!(
        clean_jameda_doctor_url(
            &json!("/markus-lietzau-msc/zahnarzt/berlin%20mitte"),
            "doctorUrl"
        )
        .unwrap(),
        "https://www.jameda.de/markus-lietzau-msc/zahnarzt/berlin%20mitte"
    );
}

#[test]
fn rejects_invalid_urls_with_typescript_validation_messages() {
    assert_eq!(
        clean_jameda_doctor_url(&json!(3), "doctorUrls").unwrap_err(),
        "doctorUrls must be a string"
    );
    assert_eq!(
        clean_jameda_doctor_url(&json!(""), "doctorUrl").unwrap_err(),
        "doctorUrl cannot be empty"
    );
    assert!(clean_jameda_doctor_url(
        &json!("https://example.com/doctor/zahnarzt/berlin"),
        "doctorUrl"
    )
    .unwrap_err()
    .contains("jameda.de domain"));
    assert!(clean_jameda_doctor_url(&json!("/search"), "doctorUrl")
        .unwrap_err()
        .contains("doctor profile path"));
}

#[test]
fn combines_batch_and_legacy_input_deduplicates_and_keeps_bad_values() {
    let plan = build_doctor_details_plan(&json!({
        "doctorUrl": MARKUS_URL,
        "doctorUrls": [
            MARKUS_URL,
            "/markus-lietzau-msc/zahnarzt/berlin",
            "/anna-example/aerztin/hamburg",
            "https://example.com/doctor/zahnarzt/berlin",
            12
        ]
    }))
    .unwrap();
    assert_eq!(
        plan.doctor_urls,
        vec![
            MARKUS_URL,
            "https://www.jameda.de/anna-example/aerztin/hamburg"
        ]
    );
    assert_eq!(plan.input_failures.len(), 2);
    assert_eq!(
        plan.input_failures[0].doctor_url,
        "https://example.com/doctor/zahnarzt/berlin"
    );
    assert_eq!(plan.input_failures[1].doctor_url, "12");
    assert_eq!(describe_request(&plan.doctor_urls), "2 doctor URLs");
}

#[test]
fn parses_comma_and_newline_lists_and_rejects_missing_or_too_many_urls() {
    let plan = build_doctor_details_plan(&json!({
            "doctorUrls": format!("{MARKUS_URL}, /anna-example/aerztin/hamburg\n/hans-example/orthopaede/muenchen")
        })).unwrap();
    assert_eq!(plan.doctor_urls.len(), 3);
    assert!(build_doctor_details_plan(&json!({}))
        .unwrap_err()
        .to_string()
        .contains("Provide doctorUrls or doctorUrl"));
    assert!(build_doctor_details_plan(&json!({"doctorUrls": 12}))
        .unwrap_err()
        .to_string()
        .contains("doctorUrls must be an array"));
    let too_many = (0..101)
        .map(|index| format!("/doctor-{index}/zahnarzt/berlin"))
        .collect::<Vec<_>>();
    assert!(build_doctor_details_plan(&json!({"doctorUrls": too_many}))
        .unwrap_err()
        .to_string()
        .contains("at most 100 doctor URLs"));
}

#[test]
fn returns_params_and_describes_single_requests_as_before() {
    assert_eq!(
        build_doctor_details_params(MARKUS_URL),
        vec![("doctor_url".to_owned(), MARKUS_URL.to_owned())]
    );
    assert_eq!(describe_request(&[MARKUS_URL.to_owned()]), MARKUS_URL);
}

#[test]
fn normalizes_profile_response_fields_and_preserves_upstream_payload() {
    let response = json!({
        "success": true,
        "meta": {"source": "scrappa", "scraped_at": "2026-06-20T00:00:00Z"},
        "data": {
            "basic_info": {
                "name": " Markus Lietzau M.Sc. ", "title": "M.Sc.",
                "specialty": "Zahnarzt", "profile_url": MARKUS_URL,
                "image_url": "//images.example/doctor.jpg"
            },
            "description": "Zahnarzt in Berlin",
            "rating": {"rating": "1,0", "count": "1.234 Bewertungen"},
            "clinic": {"name": "Praxis Markus Lietzau M.Sc. Zahnarzt"},
            "contact": {"phone": "+49 30 123456", "website": "example.com"},
            "address": {"street": "Teststr. 1", "postal_code": "10115", "city": "Berlin"},
            "coordinates": {"latitude": "52,5200", "longitude": "13.4050"},
            "opening_hours": {"monday": "09:00-17:00"},
            "services": ["Implantologie", "Prophylaxe"],
            "accepted_patients": ["Privat"], "focus_areas": ["Zahnerhaltung"],
            "conditions": ["Karies"], "languages": ["Deutsch", "Englisch"],
            "booking_ids": {"doctor_id": "abc123"}
        }
    });
    let params = build_doctor_details_params(MARKUS_URL);
    let item = build_dataset_item(&response, MARKUS_URL, &params);
    assert_eq!(item["success"], true);
    assert_eq!(item["requested_doctor_url"], MARKUS_URL);
    assert_eq!(item["doctor_url"], MARKUS_URL);
    assert_eq!(item["doctor_name"], "Markus Lietzau M.Sc.");
    assert_eq!(item["title"], "M.Sc.");
    assert_eq!(item["specialty"], "Zahnarzt");
    assert_eq!(item["rating_number"], 1.0);
    assert_eq!(item["review_count_number"], 1234.0);
    assert_eq!(item["clinic_name"], "Praxis Markus Lietzau M.Sc. Zahnarzt");
    assert_eq!(item["website_url"], "https://example.com");
    assert_eq!(item["address"], "Teststr. 1, 10115, Berlin");
    assert_eq!(item["latitude"], 52.52);
    assert_eq!(item["longitude"], 13.405);
    assert_eq!(item["image_url"], "https://images.example/doctor.jpg");
    assert_eq!(item["services_count"], 2);
    assert_eq!(item["focus_areas_count"], 1);
    assert_eq!(item["conditions_count"], 1);
    assert_eq!(item["languages_count"], 2);
    assert_eq!(item["booking_ids"], json!({"doctor_id":"abc123"}));
    assert_eq!(item["request_doctor_url"], MARKUS_URL);
    assert_eq!(item["response_source"], "scrappa");
    assert_eq!(item["scraped_at"], "2026-06-20T00:00:00Z");
}

#[test]
fn handles_sparse_response_aliases_and_numeric_separators() {
    let response = json!({
        "basic_info": {"name": "Example Doctor"},
        "address": "Berlin",
        "rating": {"score": 1.7, "review_count": 4},
        "coordinates": {"latitude": "52.520", "longitude": "13.405"}
    });
    let item = build_dataset_item(
        &response,
        MARKUS_URL,
        &build_doctor_details_params(MARKUS_URL),
    );
    assert_eq!(item["doctor_name"], "Example Doctor");
    assert_eq!(item["rating_number"], 1.7);
    assert_eq!(item["review_count_number"], 4.0);
    assert_eq!(item["address"], "Berlin");
    assert_eq!(item["services_count"], Value::Null);
    assert_eq!(item["languages"], Value::Null);
    assert_eq!(
        to_decimal_number(Some(&json!("1.234.567"))),
        Some(1234567.0)
    );
    assert_eq!(to_count_number(Some(&json!("1,234 reviews"))), Some(1234.0));
    assert_eq!(to_decimal_number(Some(&json!("-1,234"))), Some(-1.234));
    assert_eq!(to_count_number(Some(&json!("-1,234"))), Some(-1.234));
}

#[test]
fn builds_compact_output_summary_and_keeps_failure_shapes() {
    let failures = vec![InputFailure {
        doctor_url: "bad".to_owned(),
        error: "invalid".to_owned(),
    }];
    let summary = build_output_summary(&[MARKUS_URL.to_owned()], 1, &failures, Some("partial"));
    assert_eq!(summary["request"]["endpoint"], "/jameda/doctor-details");
    assert_eq!(summary["doctors_requested"], 1);
    assert_eq!(summary["doctors_saved"], 1);
    assert_eq!(summary["doctors_failed"], 1);
    assert_eq!(summary["responses_saved"], 1);
    assert_eq!(summary["status_message"], "partial");
    assert_eq!(summary["failures"][0]["doctor_url"], "bad");
}

#[test]
fn formats_scrappa_http_errors_and_retries_only_transient_statuses() {
    assert_eq!(
        scrappa_error_message(
            422,
            r#"{"message":"Invalid","errors":{"doctor_url":["bad","required"]}}"#,
            "Unprocessable Entity"
        ),
        "Invalid - doctor_url: bad, required"
    );
    assert_eq!(
        scrappa_error_message(503, "Unavailable\ntry later", "Service Unavailable"),
        "Unavailable try later"
    );
    assert_eq!(
        scrappa_error_message(503, "", "Service Unavailable"),
        "Service Unavailable"
    );
    assert_eq!(
        scrappa_error_message(503, "[]", "Service Unavailable"),
        "Service Unavailable"
    );
    for status in [408, 429, 500, 502, 503, 504] {
        assert!(is_retryable_scrappa_status(
            StatusCode::from_u16(status).unwrap()
        ));
    }
    for status in [400, 401, 403, 404, 422] {
        assert!(!is_retryable_scrappa_status(
            StatusCode::from_u16(status).unwrap()
        ));
    }
}

#[test]
fn keeps_exponential_backoff_jitter_and_ten_second_cap() {
    assert_eq!(get_retry_delay_ms(1, 0), 2000);
    assert_eq!(get_retry_delay_ms(2, 250), 4250);
    assert_eq!(get_retry_delay_ms(3, 999), 8999);
    assert_eq!(get_retry_delay_ms(4, 999), 10000);
}

#[test]
fn accounts_for_custom_dataset_and_other_event_prices_under_budget() {
    let affordable =
        PricingState::from_run(&mock_ppe_run(json!(0.0013), json!({"other-event": 1}))).unwrap();
    assert!(affordable.can_save_one_result().unwrap());
    let too_expensive =
        PricingState::from_run(&mock_ppe_run(json!(0.00129), json!({"other-event": 1}))).unwrap();
    assert!(!too_expensive.can_save_one_result().unwrap());
    let mut state = affordable;
    state.record_dataset_item();
    state.record_custom_charge();
    assert!(!state.can_save_one_result().unwrap());
}

#[test]
fn zero_null_and_absent_spending_limits_are_unbounded() {
    for limit in [Value::Null, json!(0)] {
        let state = PricingState::from_run(&mock_ppe_run(limit, json!({}))).unwrap();
        assert!(state.can_save_one_result().unwrap());
    }

    let mut run = mock_ppe_run(json!(1.0), json!({}));
    run["data"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("maxTotalChargeUsd");
    let state = PricingState::from_run(&run).unwrap();
    assert!(state.can_save_one_result().unwrap());

    let positive_limit = PricingState::from_run(&mock_ppe_run(json!(0.001), json!({}))).unwrap();
    assert!(!positive_limit.can_save_one_result().unwrap());
}

#[test]
fn non_ppe_runs_skip_custom_event_budgeting() {
    let state =
        PricingState::from_run(&json!({"data":{"pricingInfo":{"pricingModel":"FREE"}}})).unwrap();
    assert!(state.can_save_one_result().unwrap());
}

#[test]
fn rejects_incomplete_or_unsafe_ppe_metadata() {
    assert!(PricingState::from_run(
        &json!({"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"}}})
    )
    .unwrap_err()
    .to_string()
    .contains("event prices"));
    let missing_custom = json!({
        "data": {
            "pricingInfo": {"pricingModel":"PAY_PER_EVENT", "pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001}}}},
            "chargedEventCounts": {}, "options":{"maxTotalChargeUsd":1.0}
        }
    });
    assert!(PricingState::from_run(&missing_custom)
        .unwrap_err()
        .to_string()
        .contains("doctor-profile-result"));
}

#[tokio::test]
async fn calls_scrappa_with_query_auth_and_retries_transient_http_errors() {
    let response_body = r#"{"success":true,"data":{"basic_info":{"name":"Doctor"}}}"#;
    let (base_url, server) = start_mock_server(vec![
        (503, r#"{"message":"temporarily unavailable"}"#.to_owned()),
        (200, response_body.to_owned()),
    ])
    .await;
    let client = ScrappaClient::new("scrappa-test-key".to_owned(), base_url).unwrap();
    let result = client
        .get_with_delay(MARKUS_URL, |_| Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(result["data"]["basic_info"]["name"], "Doctor");
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].head.starts_with("GET /jameda/doctor-details?doctor_url=https%3A%2F%2Fwww.jameda.de%2Fmarkus-lietzau-msc%2Fzahnarzt%2Fberlin HTTP/1.1"));
    assert!(requests[0]
        .head
        .to_ascii_lowercase()
        .contains("x-api-key: scrappa-test-key"));
    assert!(requests[0]
        .head
        .to_ascii_lowercase()
        .contains("user-agent: thescrappa-jameda-doctor-details-scraper/1.0"));
}

#[tokio::test]
async fn does_not_retry_validation_errors() {
    let (base_url, server) =
        start_mock_server(vec![(400, r#"{"message":"bad input"}"#.to_owned())]).await;
    let client = ScrappaClient::new("scrappa-test-key".to_owned(), base_url).unwrap();
    let error = client
        .get_with_delay(MARKUS_URL, |_| panic!("validation error must not retry"))
        .await
        .unwrap_err();
    assert_eq!(error.message, "Scrappa API error (400): bad input");
    assert!(!error.retryable);
    assert_eq!(server.await.unwrap().len(), 1);
}

#[tokio::test]
async fn apify_custom_charge_request_keeps_auth_and_idempotency() {
    let (base_url, server) = start_mock_server(vec![(201, "{}".to_owned())]).await;
    let apify = mock_apify_client(base_url);
    apify
        .charge_event(SCRAPPA_CHARGE_EVENT, "test-run-doctor-profile-result-1")
        .await
        .unwrap();
    let requests = server.await.unwrap();
    assert!(requests[0]
        .head
        .starts_with("POST /v2/actor-runs/test-run/charge HTTP/1.1"));
    assert!(requests[0].head.contains("Bearer test-token"));
    assert!(requests[0]
        .head
        .to_ascii_lowercase()
        .contains("idempotency-key: test-run-doctor-profile-result-1"));
    assert_eq!(
        serde_json::from_str::<Value>(&requests[0].body).unwrap(),
        json!({"eventName":"doctor-profile-result","count":1})
    );
}

#[tokio::test]
async fn retries_ambiguous_charge_responses_with_the_same_idempotency_key() {
    let (base_url, server) = start_mock_server(vec![
        (503, r#"{"error":"temporary failure"}"#.to_owned()),
        (201, "{}".to_owned()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    apify
        .charge_event(SCRAPPA_CHARGE_EVENT, "test-run-doctor-profile-result-1")
        .await
        .unwrap();
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests {
        assert!(request
            .head
            .to_ascii_lowercase()
            .contains("idempotency-key: test-run-doctor-profile-result-1"));
    }
}

#[tokio::test]
async fn spending_limit_skips_dataset_and_custom_event_when_result_does_not_fit() {
    let (base_url, server) = start_mock_server(vec![
        (200, mock_ppe_run(json!(0.001), json!({})).to_string()),
        (404, String::new()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    let mut pricing = None;
    let result = push_charged_item(
        &apify,
        &mut pricing,
        &json!({"doctor_name":"Doctor", "requested_doctor_url":MARKUS_URL}),
        1,
        MARKUS_URL,
        false,
    )
    .await
    .unwrap();

    assert_eq!(result.saved_count, 0);
    assert_eq!(
        result.status_message.as_deref(),
        Some("Charge limit reached after saving 0 of 1 Jameda doctor profile results.")
    );
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0]
        .head
        .starts_with("GET /v2/actor-runs/test-run HTTP/1.1"));
    assert!(requests[1]
        .head
        .starts_with("GET /v2/key-value-stores/store/records/PPE_RESULT_0001 HTTP/1.1"));
}

#[tokio::test]
async fn dataset_failure_keeps_the_charge_journal_and_does_not_replay() {
    let (base_url, server) = start_mock_server(vec![
        (200, mock_ppe_run(json!(0.0011), json!({})).to_string()),
        (404, String::new()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (400, r#"{"error":"dataset write rejected"}"#.to_owned()),
        (200, "[]".to_owned()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    let mut pricing = None;
    let error = push_charged_item(
        &apify,
        &mut pricing,
        &json!({"doctor_name":"Doctor", "requested_doctor_url":MARKUS_URL}),
        1,
        MARKUS_URL,
        false,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("dataset item publication failed"));
    let requests = server.await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.head.starts_with("POST /v2/actor-runs/test-run/charge"))
            .count(),
        1
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.head.starts_with("POST /v2/datasets/dataset/items"))
            .count(),
        1
    );
    assert!(requests[3]
        .head
        .starts_with("POST /v2/actor-runs/test-run/charge HTTP/1.1"));
    assert!(requests[6]
        .head
        .starts_with("POST /v2/datasets/dataset/items HTTP/1.1"));
    assert_eq!(
        serde_json::from_str::<Value>(&requests[5].body).unwrap()["status"],
        "publishing"
    );
}

#[tokio::test]
async fn does_not_replay_an_ambiguous_dataset_post_when_recovery_finds_no_row() {
    let (base_url, server) = start_mock_server(vec![
        (200, mock_ppe_run(json!(0.0011), json!({})).to_string()),
        (404, String::new()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (503, r#"{"error":"dataset response lost"}"#.to_owned()),
        (200, "[]".to_owned()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    let mut pricing = None;
    let error = push_charged_item(
        &apify,
        &mut pricing,
        &json!({"doctor_name":"Doctor", "requested_doctor_url":MARKUS_URL}),
        1,
        MARKUS_URL,
        false,
    )
    .await
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("will not be retried to avoid duplicates"));
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 8);
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.head.starts_with("POST /v2/datasets/dataset/items"))
            .count(),
        1
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.head.starts_with("POST /v2/actor-runs/test-run/charge"))
            .count(),
        1
    );
}

#[tokio::test]
async fn charges_before_publishing_and_journals_each_confirmed_step() {
    let (base_url, server) = start_mock_server(vec![
        (200, mock_ppe_run(json!(0.0011), json!({})).to_string()),
        (404, String::new()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    let mut pricing = None;
    let item = json!({"doctor_name":"Doctor", "requested_doctor_url":MARKUS_URL});
    let result = push_charged_item(&apify, &mut pricing, &item, 1, MARKUS_URL, false)
        .await
        .unwrap();

    assert_eq!(result.saved_count, 1);
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 8);
    assert!(requests[3]
        .head
        .starts_with("POST /v2/actor-runs/test-run/charge HTTP/1.1"));
    assert!(requests[6]
        .head
        .starts_with("POST /v2/datasets/dataset/items HTTP/1.1"));
    assert!(
        requests
            .iter()
            .position(|request| request
                .head
                .starts_with("POST /v2/actor-runs/test-run/charge"))
            < requests
                .iter()
                .position(|request| request.head.starts_with("POST /v2/datasets/dataset/items"))
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[6].body).unwrap(),
        item
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[2].body).unwrap()["status"],
        "pending"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[4].body).unwrap()["status"],
        "charged"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[5].body).unwrap()["status"],
        "publishing"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[7].body).unwrap()["status"],
        "saved"
    );
}

#[tokio::test]
async fn rejected_charge_never_publishes_a_dataset_item() {
    let (base_url, server) = start_mock_server(vec![
        (200, mock_ppe_run(json!(1.0), json!({})).to_string()),
        (404, String::new()),
        (201, "{}".to_owned()),
        (400, r#"{"error":"charge rejected"}"#.to_owned()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    let mut pricing = None;
    let error = push_charged_item(
        &apify,
        &mut pricing,
        &json!({"doctor_name":"Doctor", "requested_doctor_url":MARKUS_URL}),
        1,
        MARKUS_URL,
        false,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("event charge failed (400)"));
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[3]
        .head
        .starts_with("POST /v2/actor-runs/test-run/charge HTTP/1.1"));
    assert!(!requests
        .iter()
        .any(|request| request.head.starts_with("POST /v2/datasets/dataset/items")));
}

#[tokio::test]
async fn recovery_finds_a_row_after_ambiguous_dataset_failure_without_duplicating_it() {
    let item = json!({"doctor_name":"Doctor", "requested_doctor_url":MARKUS_URL});
    let (base_url, server) = start_mock_server(vec![
        (200, mock_ppe_run(json!(0.0011), json!({})).to_string()),
        (404, String::new()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (201, "{}".to_owned()),
        (503, r#"{"error":"write response lost"}"#.to_owned()),
        (200, json!([item]).to_string()),
        (201, "{}".to_owned()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    let mut pricing = None;
    let result = push_charged_item(&apify, &mut pricing, &item, 1, MARKUS_URL, false)
        .await
        .unwrap();

    assert_eq!(result.saved_count, 1);
    let requests = server.await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.head.starts_with("POST /v2/datasets/dataset/items"))
            .count(),
        1
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request
                .head
                .starts_with("POST /v2/actor-runs/test-run/charge"))
            .count(),
        1
    );
    assert_eq!(requests.len(), 9);
    assert!(requests[3]
        .head
        .starts_with("POST /v2/actor-runs/test-run/charge HTTP/1.1"));
    assert!(requests[6]
        .head
        .starts_with("POST /v2/datasets/dataset/items HTTP/1.1"));
    assert!(requests[7]
        .head
        .starts_with("GET /v2/datasets/dataset/items?format=json&clean=true&limit=1000 HTTP/1.1"));
    assert_eq!(
        serde_json::from_str::<Value>(&requests[4].body).unwrap()["status"],
        "charged"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[5].body).unwrap()["status"],
        "publishing"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&requests[8].body).unwrap()["status"],
        "saved"
    );
}

#[tokio::test]
async fn saved_result_recovers_an_already_recorded_charge_without_reposting() {
    let item = json!({"doctor_name":"Doctor", "requested_doctor_url":MARKUS_URL});
    let recovery_record = json!({
        "status":"saved",
        "doctor_url":MARKUS_URL,
        "item":item,
        "idempotency_key":"test-run-doctor-profile-result-1",
        "baseline_event_counts": {
            "doctor-profile-result":0,
            "apify-default-dataset-item":0
        }
    });
    let (base_url, server) = start_mock_server(vec![
        (
            200,
            mock_ppe_run(
                json!(0.0011),
                json!({"doctor-profile-result":1,"apify-default-dataset-item":1}),
            )
            .to_string(),
        ),
        (200, recovery_record.to_string()),
        (200, json!([item]).to_string()),
        (201, "{}".to_owned()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    let mut pricing = None;
    let result = push_charged_item(&apify, &mut pricing, &item, 1, MARKUS_URL, false)
        .await
        .unwrap();

    assert_eq!(result.saved_count, 1);
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(!requests.iter().any(|request| request
        .head
        .starts_with("POST /v2/actor-runs/test-run/charge")));
    assert!(!requests
        .iter()
        .any(|request| request.head.starts_with("POST /v2/datasets/dataset/items")));
    assert!(requests[2]
        .head
        .starts_with("GET /v2/datasets/dataset/items?format=json&clean=true&limit=1000 HTTP/1.1"));
    let finalized_record = serde_json::from_str::<Value>(&requests[3].body).unwrap();
    assert_eq!(finalized_record["journal_version"], 2);
    assert_eq!(finalized_record["status"], "saved");
}

#[tokio::test]
async fn resumes_a_charged_result_from_its_journal_before_upstream_fetch() {
    let item = json!({"doctor_name":"Doctor", "requested_doctor_url":MARKUS_URL});
    let recovery_record = json!({
        "status":"charged",
        "doctor_url":MARKUS_URL,
        "item":item,
        "idempotency_key":"test-run-doctor-profile-result-1",
        "baseline_event_counts": {
            "doctor-profile-result":0,
            "apify-default-dataset-item":0
        }
    });
    let (base_url, server) = start_mock_server(vec![
        (200, recovery_record.to_string()),
        (
            200,
            mock_ppe_run(
                json!(0.0011),
                json!({"doctor-profile-result":1,"apify-default-dataset-item":1}),
            )
            .to_string(),
        ),
        (200, json!([item]).to_string()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    let mut pricing = None;
    let result = resume_charged_item(&apify, &mut pricing, 1, MARKUS_URL)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(result.saved_count, 1);
    let requests = server.await.unwrap();
    assert!(requests[0]
        .head
        .starts_with("GET /v2/key-value-stores/store/records/PPE_RESULT_0001 HTTP/1.1"));
    assert!(requests[1]
        .head
        .starts_with("GET /v2/actor-runs/test-run HTTP/1.1"));
    assert!(requests[2]
        .head
        .starts_with("GET /v2/datasets/dataset/items?format=json&clean=true&limit=1000 HTTP/1.1"));
    assert!(!requests.iter().any(|request| request
        .head
        .starts_with("POST /v2/actor-runs/test-run/charge")));
    assert!(!requests
        .iter()
        .any(|request| request.head.starts_with("POST /v2/datasets/dataset/items")));
    assert_eq!(requests.len(), 3);
    assert!(!requests
        .iter()
        .any(|request| request.head.starts_with("GET /jameda/doctor-details")));
}

#[tokio::test]
async fn pending_recovery_without_a_confirmed_row_fails_without_reposting() {
    let item = json!({"doctor_name":"Doctor", "requested_doctor_url":MARKUS_URL});
    let recovery_record = json!({
        "status":"pending",
        "doctor_url":MARKUS_URL,
        "item":item,
        "idempotency_key":"test-run-doctor-profile-result-1",
        "baseline_event_counts": {
            "doctor-profile-result":0,
            "apify-default-dataset-item":0
        }
    });
    let (base_url, server) = start_mock_server(vec![
        (200, mock_ppe_run(json!(0.0011), json!({})).to_string()),
        (200, recovery_record.to_string()),
        (200, "[]".to_owned()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    let mut pricing = None;
    let error = push_charged_item(
        &apify,
        &mut pricing,
        &json!({"doctor_name":"Doctor", "requested_doctor_url":MARKUS_URL}),
        1,
        MARKUS_URL,
        false,
    )
    .await
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("will not be retried to avoid duplicates"));
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 3);
    assert!(!requests
        .iter()
        .any(|request| request.head.starts_with("POST /v2/datasets/dataset/items")));
    assert!(!requests.iter().any(|request| request
        .head
        .starts_with("POST /v2/actor-runs/test-run/charge")));
}

#[tokio::test]
async fn publishing_recovery_without_a_confirmed_row_fails_without_reposting() {
    let item = json!({"doctor_name":"Doctor", "requested_doctor_url":MARKUS_URL});
    let recovery_record = json!({
        "journal_version":2,
        "status":"publishing",
        "doctor_url":MARKUS_URL,
        "item":item,
        "idempotency_key":"test-run-doctor-profile-result-1",
        "baseline_event_counts": {
            "doctor-profile-result":0,
            "apify-default-dataset-item":0
        }
    });
    let (base_url, server) = start_mock_server(vec![
        (200, recovery_record.to_string()),
        (
            200,
            mock_ppe_run(json!(0.0011), json!({"doctor-profile-result":1})).to_string(),
        ),
        (200, "[]".to_owned()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    let mut pricing = None;
    let error = resume_charged_item(&apify, &mut pricing, 1, MARKUS_URL)
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("will not be retried to avoid duplicates"));
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 3);
    assert!(!requests
        .iter()
        .any(|request| request.head.starts_with("POST /v2/datasets/dataset/items")));
    assert!(!requests.iter().any(|request| request
        .head
        .starts_with("POST /v2/actor-runs/test-run/charge")));
}

#[tokio::test]
async fn writes_input_output_and_terminal_run_status_to_apify() {
    let (base_url, server) = start_mock_server(vec![
        (200, json!({"doctorUrl":MARKUS_URL}).to_string()),
        (201, "{}".to_owned()),
        (200, "{}".to_owned()),
    ])
    .await;
    let apify = mock_apify_client(base_url);
    assert_eq!(
        apify.get_input().await.unwrap(),
        Some(json!({"doctorUrl":MARKUS_URL}))
    );
    apify
        .put_record("OUTPUT", &json!({"doctors_saved":1}))
        .await
        .unwrap();
    apify
        .set_terminal_status_message("partial results")
        .await
        .unwrap();
    let requests = server.await.unwrap();
    assert!(requests[0]
        .head
        .starts_with("GET /v2/key-value-stores/store/records/INPUT HTTP/1.1"));
    assert!(requests[1]
        .head
        .starts_with("PUT /v2/key-value-stores/store/records/OUTPUT HTTP/1.1"));
    assert_eq!(
        serde_json::from_str::<Value>(&requests[1].body).unwrap(),
        json!({"doctors_saved":1})
    );
    assert!(requests[2]
        .head
        .starts_with("PUT /v2/actor-runs/test-run HTTP/1.1"));
    assert_eq!(
        serde_json::from_str::<Value>(&requests[2].body).unwrap()["isStatusMessageTerminal"],
        true
    );
}

#[test]
fn constructs_api_paths_without_dropping_a_base_path() {
    assert_eq!(
        endpoint_url(
            "http://127.0.0.1:8080/mock/",
            &["v2", "datasets", "dataset", "items"]
        )
        .unwrap()
        .as_str(),
        "http://127.0.0.1:8080/mock/v2/datasets/dataset/items"
    );
}
