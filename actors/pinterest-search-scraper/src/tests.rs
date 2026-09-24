use reqwest::{Client, StatusCode};
use serde_json::{json, Map, Value};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;

use super::api_url::endpoint_url;
use super::apify_client::{apify_retry_delay, dataset_item_chunks};
use super::apify_client::{
    retryable_apify_status, ApifyClient, Config, APIFY_MAX_RETRIES, APIFY_REQUEST_TIMEOUT,
    MAX_DATASET_REQUEST_BYTES,
};
use super::charging::{
    pinterest_charged_save_result, ChargeBudget, DEFAULT_DATASET_ITEM_EVENT,
    PIN_RESULT_CHARGE_EVENT,
};
use super::pinterest_input::{
    build_pinterest_search_plan, cap_pinterest_search_params, decode_input_string,
    describe_pinterest_search_request, PinterestSearchParams,
};
use super::pinterest_response::{
    limit_pinterest_search_response, pinterest_dataset_item, pinterest_next_bookmark,
    select_pinterest_pins, PinterestPinsSource,
};
use super::scrappa_client::{
    is_retryable_scrappa_error, scrappa_api_error_message, scrappa_retry_delay, ScrappaApiError,
    ScrappaClient, SCRAPPA_API_DEFAULT,
};

async fn mock_http_response(status: u16, body: &str) -> (Url, tokio::task::JoinHandle<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let body = body.to_owned();
    let task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0u8; 4096];
        let mut headers_end = None;
        let mut content_length = 0usize;
        loop {
            let count = stream.read(&mut buffer).await.unwrap();
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            if headers_end.is_none() {
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    headers_end = Some(end + 4);
                    let headers = String::from_utf8_lossy(&request[..end]);
                    content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                }
            }
            if headers_end.is_some_and(|end| request.len() >= end + content_length) {
                break;
            }
        }
        let request = String::from_utf8_lossy(&request).into_owned();
        let response = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        stream.write_all(response.as_bytes()).await.unwrap();
        request
    });
    (Url::parse(&format!("http://{address}")).unwrap(), task)
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn ppe_run(max_total: f64, charged_event_counts: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "pin-result": {"eventPriceUsd": 0.002},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.001},
                    "actor-start": {"eventPriceUsd": 0.001}
                }}
            },
            "chargedEventCounts": charged_event_counts,
            "options": {"maxTotalChargeUsd": max_total}
        }
    })
}

fn apify_client(base_url: Url) -> ApifyClient {
    let config = Config {
        apify_api_base: base_url,
        apify_token: "test-token".to_owned(),
        actor_run_id: "run-1".to_owned(),
        key_value_store_id: "store-1".to_owned(),
        dataset_id: "dataset-1".to_owned(),
        input_key: "INPUT".to_owned(),
        scrappa_api_base: Url::parse(SCRAPPA_API_DEFAULT).unwrap(),
        scrappa_api_key: Some("test-key".to_owned()),
    };
    ApifyClient::new(Client::new(), &config)
}

#[test]
fn builds_batch_input_and_keeps_query_order() {
    let plan = build_pinterest_search_plan(&object(json!({
        "query": " home%20decor ",
        "queries": ["home decor", " Home Decor ", "kitchen ideas"],
        "limit": "25",
        "bookmark": "abc123"
    })))
    .unwrap();

    assert_eq!(plan.queries, ["home decor", "Home Decor", "kitchen ideas"]);
    assert_eq!(plan.limit, 25);
    assert_eq!(plan.bookmark.as_deref(), Some("abc123"));
    assert_eq!(
        describe_pinterest_search_request(&plan),
        "3 queries (25 pins/query, with bookmark)"
    );
}

#[test]
fn decodes_uri_components_only_when_the_entire_escape_sequence_is_valid() {
    assert_eq!(decode_input_string("home%20decor"), "home decor");
    assert_eq!(decode_input_string("plus+sign"), "plus+sign");
    assert_eq!(decode_input_string("100% ready"), "100% ready");
    assert_eq!(decode_input_string("home%20decor%ZZ"), "home%20decor%ZZ");
    assert_eq!(decode_input_string("%E0%A4%A"), "%E0%A4%A");
}

#[test]
fn validates_query_and_pagination_input() {
    assert!(build_pinterest_search_plan(&Map::new())
        .unwrap_err()
        .to_string()
        .contains("Provide at least one Pinterest search query"));
    assert!(
        build_pinterest_search_plan(&object(json!({"queries": "home decor"})))
            .unwrap_err()
            .to_string()
            .contains("queries must be an array of strings")
    );
    assert!(
        build_pinterest_search_plan(&object(json!({"query": "decor", "limit": 251})))
            .unwrap_err()
            .to_string()
            .contains("limit must be between 1 and 250")
    );
    assert!(
        build_pinterest_search_plan(&object(json!({"query": "decor", "bookmark": 12})))
            .unwrap_err()
            .to_string()
            .contains("bookmark must be a string")
    );
    assert!(build_pinterest_search_plan(&object(json!({"query": "x".repeat(201)}))).is_err());
}

#[test]
fn caps_upstream_limit_to_the_remaining_pin_event_capacity() {
    let params = PinterestSearchParams {
        query: "home decor".to_owned(),
        limit: 250,
        bookmark: Some("next/page".to_owned()),
    };
    let fetch = cap_pinterest_search_params(&params, 3);
    assert_eq!(fetch.requested_limit, 250);
    assert_eq!(fetch.fetch_limit, 3);
    assert_eq!(fetch.params.limit, 3);
    assert_eq!(fetch.params.bookmark.as_deref(), Some("next/page"));
}

#[test]
fn selects_response_shapes_in_the_same_priority_order() {
    assert_eq!(
        select_pinterest_pins(&json!({"pins": [], "data": {"pins": [{"id": 2}]}})).source,
        Some(PinterestPinsSource::Pins)
    );
    assert_eq!(
        select_pinterest_pins(&json!({"data": {"pins": [{"id": 2}]}, "results": [{"id": 3}]}))
            .source,
        Some(PinterestPinsSource::DataPins)
    );
    assert_eq!(
        select_pinterest_pins(&json!({"results": [{"id": 3}], "data": {"results": [{"id": 4}]}}))
            .source,
        Some(PinterestPinsSource::Results)
    );
    assert_eq!(
        select_pinterest_pins(&json!({"data": {"results": [{"id": 4}]}})).source,
        Some(PinterestPinsSource::DataResults)
    );
    assert!(select_pinterest_pins(&json!({})).pins.is_empty());
}

#[test]
fn normalizes_pin_fields_and_limits_only_the_selected_response_array() {
    let response = json!({
        "query": "home decor",
        "count": 25,
        "pins": [{"id": "123"}, {"id": "456"}],
        "results": [{"id": "other"}]
    });
    let params = PinterestSearchParams {
        query: "home decor".to_owned(),
        limit: 25,
        bookmark: Some("abc".to_owned()),
    };
    let pin = json!({
        "id": "123",
        "title": "Storage",
        "images": {"orig": {"url": "https://example.com/pin.jpg"}},
        "link": "https://example.com/storage",
        "pinner": {"id": "u1", "username": "homeideas"},
        "board": {"id": "b1", "name": "Home"},
        "video": {"duration": 12},
        "repin_count": 5,
        "unknown_field": true
    });

    let item = pinterest_dataset_item(&pin, &params, &response).unwrap();
    assert_eq!(item["id"], "123");
    assert_eq!(item["image_url"], "https://example.com/pin.jpg");
    assert_eq!(item["link"], "https://example.com/storage");
    assert_eq!(item["pinner_id"], "u1");
    assert_eq!(item["pinner_username"], "homeideas");
    assert_eq!(item["board_id"], "b1");
    assert_eq!(item["board_name"], "Home");
    assert_eq!(item["has_video"], true);
    assert_eq!(item["request_query"], "home decor");
    assert_eq!(item["request_bookmark"], "abc");
    assert_eq!(item["results_count"], 2);
    assert_eq!(item["unknown_field"], true);

    let limited = limit_pinterest_search_response(&response, 1, Some(PinterestPinsSource::Pins));
    assert_eq!(limited["pins"], json!([{"id": "123"}]));
    assert!(limited.get("results").is_none());
}

#[test]
fn preserves_empty_primary_results_and_bookmark_fallback() {
    assert!(select_pinterest_pins(&json!({
        "pins": [],
        "data": {"pins": [{"id": "fallback"}]}
    }))
    .pins
    .is_empty());
    assert_eq!(
        pinterest_next_bookmark(&json!({"bookmark": "fallback-token"})),
        json!("fallback-token")
    );
    assert_eq!(
        pinterest_next_bookmark(&json!({"nextBookmark": "", "bookmark": "fallback"})),
        json!("")
    );
}

#[test]
fn caps_ppe_results_using_combined_named_and_dataset_event_costs() {
    let mut budget =
        ChargeBudget::from_actor_run(&ppe_run(0.01, json!({"actor-start": 1}))).unwrap();
    assert_eq!(budget.chargeable_pin_capacity(), 4);
    assert_eq!(budget.pushable_pin_count(10), 3);

    let named = budget.prepare_event_charge(PIN_RESULT_CHARGE_EVENT, 3);
    let dataset = budget.prepare_event_charge(DEFAULT_DATASET_ITEM_EVENT, 3);
    assert_eq!((named, dataset), (3, 3));
    assert_eq!(budget.total_charged_amount(), 0.01);
    assert!(budget.event_charge_limit_reached(PIN_RESULT_CHARGE_EVENT));
    assert_eq!(
        pinterest_charged_save_result(named + dataset, true, 3).saved_count,
        3
    );
}

#[test]
fn treats_unpriced_pin_events_as_free_but_keeps_dataset_item_budgeting() {
    let mut run = ppe_run(0.01, json!({}));
    run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
        .as_object_mut()
        .unwrap()
        .remove(PIN_RESULT_CHARGE_EVENT);
    let budget = ChargeBudget::from_actor_run(&run).unwrap();
    assert_eq!(budget.chargeable_pin_capacity(), usize::MAX);
    assert_eq!(budget.pushable_pin_count(250), 10);

    run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
        .as_object_mut()
        .unwrap()
        .remove(DEFAULT_DATASET_ITEM_EVENT);
    let unpriced_budget = ChargeBudget::from_actor_run(&run).unwrap();
    assert_eq!(unpriced_budget.pushable_pin_count(250), 250);

    let free_budget = ChargeBudget::from_actor_run(&json!({"data": {
        "pricingInfo": {"pricingModel": "FREE"}
    }}))
    .unwrap();
    assert!(!free_budget.is_pay_per_event());
    assert_eq!(free_budget.pushable_pin_count(250), 250);
}

#[test]
fn formats_scrappa_errors_and_selects_only_transient_retry_statuses() {
    assert_eq!(
        scrappa_api_error_message(
            StatusCode::BAD_REQUEST,
            r#"{"message":"Invalid input","errors":{"query":["is required","is too short"]}}"#
        ),
        "Invalid input - query: is required, is too short"
    );
    assert_eq!(
        scrappa_api_error_message(StatusCode::BAD_REQUEST, " bad   request\nbody "),
        "bad request body"
    );
    for status in [408, 429, 500, 502, 503, 504] {
        let error = ScrappaApiError {
            status,
            message: "retry".to_owned(),
        };
        assert!(is_retryable_scrappa_error(&anyhow::Error::new(error)));
    }
    assert!(!is_retryable_scrappa_error(&anyhow::Error::new(
        ScrappaApiError {
            status: 400,
            message: "bad input".to_owned(),
        }
    )));
    assert_eq!(scrappa_retry_delay(1).as_millis() / 1000, 2);
    assert!(scrappa_retry_delay(2).as_millis() >= 4000);
}

#[test]
fn preserves_apify_client_retry_statuses_deadline_and_backoff() {
    assert_eq!(APIFY_MAX_RETRIES, 8);
    assert_eq!(APIFY_REQUEST_TIMEOUT.as_secs(), 360);
    assert_eq!(apify_retry_delay(0), Duration::from_millis(500));
    assert_eq!(apify_retry_delay(1), Duration::from_secs(1));
    assert!(retryable_apify_status(StatusCode::TOO_MANY_REQUESTS));
    assert!(retryable_apify_status(StatusCode::INTERNAL_SERVER_ERROR));
    assert!(!retryable_apify_status(StatusCode::REQUEST_TIMEOUT));
    assert!(!retryable_apify_status(StatusCode::BAD_REQUEST));
}

#[test]
fn chunks_dataset_items_without_exceeding_the_request_limit() {
    let item = json!({"payload": "x".repeat(1024 * 1024)});
    let chunks = dataset_item_chunks(&vec![item.clone(); 5]).unwrap();
    assert_eq!(chunks.len(), 2);
    assert!(chunks
        .iter()
        .all(|chunk| { serde_json::to_vec(chunk).unwrap().len() <= MAX_DATASET_REQUEST_BYTES }));
}

#[test]
fn actor_manifest_keeps_input_prefill_and_run_resources() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    let actor: Value = serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();
    assert_eq!(
        schema["properties"]["queries"]["prefill"],
        json!(["home decor", "kitchen ideas"])
    );
    assert_eq!(schema["properties"]["query"]["prefill"], "home decor");
    assert_eq!(schema["properties"]["limit"]["maximum"], 250);
    assert_eq!(actor["defaultMemoryMbytes"], 256);
    assert_eq!(actor["defaultRunOptions"]["timeoutSecs"], 360);
    assert!(actor.get("meta").is_none());
}

#[tokio::test]
async fn scrappa_request_keeps_auth_headers_and_pagination_query() {
    let (base_url, task) = mock_http_response(200, r#"{"pins":[]}"#).await;
    let base_url = endpoint_url(&base_url, &["api"]).unwrap();
    let client = ScrappaClient::new(base_url, "scrappa-secret".to_owned()).unwrap();
    let params = PinterestSearchParams {
        query: "home decor".to_owned(),
        limit: 3,
        bookmark: Some("next/page".to_owned()),
    };
    assert_eq!(
        client.pinterest_search(&params).await.unwrap(),
        json!({"pins": []})
    );
    let request = task.await.unwrap();
    let request = request.to_ascii_lowercase();
    assert!(request.starts_with(
        "get /api/pinterest/search?query=home+decor&limit=3&bookmark=next%2fpage http/1.1"
    ));
    assert!(request.contains("x-api-key: scrappa-secret"));
    assert!(request.contains("user-agent: thescrappa-pinterest-search-scraper/1.0"));
    assert!(request.contains("accept: application/json"));
}

#[tokio::test]
async fn apify_input_and_output_use_run_storage_and_bearer_auth() {
    let (base_url, task) = mock_http_response(200, r#"{"query":"home decor"}"#).await;
    let apify = apify_client(base_url);
    let input = apify.get_input().await.unwrap().unwrap();
    assert_eq!(input["query"], "home decor");
    let request = task.await.unwrap().to_ascii_lowercase();
    assert!(request.starts_with("get /v2/key-value-stores/store-1/records/input http/1.1"));
    assert!(request.contains("authorization: bearer test-token"));

    let (base_url, task) = mock_http_response(201, "{}").await;
    let apify = apify_client(base_url);
    apify.put_output(&json!({"pins_saved": 1})).await.unwrap();
    let request = task.await.unwrap().to_ascii_lowercase();
    assert!(request.starts_with("put /v2/key-value-stores/store-1/records/output http/1.1"));
    assert!(request.contains("\"pins_saved\":1"));
    assert!(request.contains("authorization: bearer test-token"));
}

#[tokio::test]
async fn custom_result_charge_uses_apify_run_charge_endpoint() {
    let (base_url, task) = mock_http_response(201, "{}").await;
    let apify = apify_client(base_url);
    apify
        .charge_event(PIN_RESULT_CHARGE_EVENT, 2)
        .await
        .unwrap();
    let request = task.await.unwrap().to_ascii_lowercase();
    assert!(request.starts_with("post /v2/actor-runs/run-1/charge http/1.1"));
    assert!(request.contains("\"eventname\":\"pin-result\""));
    assert!(request.contains("\"count\":2"));
    assert!(request.contains("idempotency-key: pinterest-search-run-1-"));
}

#[tokio::test]
async fn terminal_status_updates_include_required_run_id() {
    let (base_url, task) = mock_http_response(200, "{}").await;
    let apify = apify_client(base_url);
    apify.set_status_message("Actor failed").await.unwrap();
    let request = task.await.unwrap().to_ascii_lowercase();
    assert!(request.starts_with("put /v2/actor-runs/run-1 http/1.1"));
    assert!(request.contains("\"runid\":\"run-1\""));
    assert!(request.contains("\"statusmessage\":\"actor failed\""));
    assert!(request.contains("\"isstatusmessageterminal\":true"));
    assert!(!request.contains("\"level\""));
}
