use std::collections::BTreeMap;
use std::time::Duration;

use serde_json::{json, Value};
use url::Url;

use crate::test_support::*;
use crate::{
    apify::{dataset_batches, ApifyClient, APIFY_REQUEST_TIMEOUT},
    budget::{load_charge_budget, ChargeBudget, RESULT_EVENT, SEARCH_EVENT},
    config::SCRAPPA_API_DEFAULT,
    input::build_search_url,
    run_actor,
    scrappa::fetch_search,
};

#[test]
fn input_schema_and_prefill_contract_are_kept() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    assert_eq!(schema["required"], json!(["query", "zoom"]));
    assert_eq!(schema["properties"]["query"]["prefill"], "coffee shops");
    assert_eq!(schema["properties"]["zoom"]["default"], 15);
    assert_eq!(schema["properties"]["latitude"]["prefill"], 40.7128);
    assert_eq!(schema["properties"]["longitude"]["prefill"], -74.006);
    assert_eq!(schema["properties"]["limit"]["prefill"], 10);
    assert_eq!(schema["properties"]["gl"]["prefill"], "us");
    assert_eq!(schema["properties"]["zoom"]["minimum"], 3);
    assert_eq!(schema["properties"]["zoom"]["maximum"], 21);
    assert!(schema["properties"].get("page").is_none());
    assert_eq!(
        schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            "query",
            "zoom",
            "latitude",
            "longitude",
            "limit",
            "hl",
            "gl"
        ]
    );
}

#[test]
fn search_request_keeps_legacy_path_auth_parameters_and_default_page() {
    let base_url = Url::parse("https://scrappa.co/api").unwrap();
    let url = build_search_url(&test_input(), &base_url).unwrap();
    assert_eq!(url.path(), "/api/maps/advance-search");
    let query = url
        .query_pairs()
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(query.get("query").map(String::as_str), Some("coffee shops"));
    assert_eq!(query.get("zoom").map(String::as_str), Some("15"));
    assert_eq!(query.get("lat").map(String::as_str), Some("40.758"));
    assert_eq!(query.get("lon").map(String::as_str), Some("-73.9855"));
    assert_eq!(query.get("limit").map(String::as_str), Some("50"));
    assert_eq!(query.get("hl").map(String::as_str), Some("de"));
    assert_eq!(query.get("gl").map(String::as_str), Some("de"));
    assert!(!query.contains_key("page"));

    let defaults = build_search_url(
        &json!({"query":"coffee shops","zoom":15,"latitude":0,"longitude":0,"hl":"","gl":""}),
        &base_url,
    )
    .unwrap();
    let query = defaults.query_pairs().collect::<BTreeMap<_, _>>();
    assert_eq!(query.get("lat").map(|value| value.as_ref()), Some("0"));
    assert_eq!(query.get("lon").map(|value| value.as_ref()), Some("0"));
    assert_eq!(query.get("hl").map(|value| value.as_ref()), Some("en"));
    assert!(!query.contains_key("gl"));
    assert!(!query.contains_key("page"));
}

#[tokio::test]
async fn full_actor_flow_preserves_auth_charges_dataset_and_raw_output() {
    let input = test_input();
    let output = json!({
        "items": [
            {"name":"First Cafe","business_id":"first","rating":4.8},
            {"name":"Second Cafe","business_id":"second","phone_numbers":["+1 555 0100"]}
        ],
        "pagination": {"page": 0, "next": null},
        "provider_field": {"kept": true}
    });
    let server = MockServer::start(vec![
        response(200, &input.to_string()),
        response(201, "{}"),
        response(200, &output.to_string()),
        response(201, "{}"),
        response(201, "{}"),
        response(200, "{}"),
    ]);
    let config = config(&server);
    run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 6);
    assert_eq!(request_parts(&requests[0]).0, "GET");
    assert_eq!(
        request_parts(&requests[0]).1,
        "/v2/key-value-stores/test-store/records/INPUT"
    );
    assert_eq!(
        header_value(request_parts(&requests[0]).2, "authorization"),
        Some("Bearer apify-test-token")
    );

    assert_eq!(request_parts(&requests[1]).0, "POST");
    assert_eq!(
        request_parts(&requests[1]).1,
        "/v2/actor-runs/test-run/charge"
    );
    let search_charge: Value = serde_json::from_str(request_parts(&requests[1]).3).unwrap();
    assert_eq!(search_charge, json!({"eventName":"search","count":1}));
    assert!(header_value(request_parts(&requests[1]).2, "idempotency-key").is_some());

    assert_eq!(request_parts(&requests[2]).0, "GET");
    assert_eq!(
        request_parts(&requests[2]).1.split('?').next(),
        Some("/api/maps/advance-search")
    );
    let target = request_parts(&requests[2]).1;
    assert!(target.contains("query=coffee+shops"));
    assert!(target.contains("lat=40.758"));
    assert!(target.contains("page=") == false);
    assert_eq!(
        header_value(request_parts(&requests[2]).2, "x-api-key"),
        Some("scrappa-test-key")
    );

    assert_eq!(request_parts(&requests[3]).0, "POST");
    assert_eq!(
        request_parts(&requests[3]).1,
        "/v2/actor-runs/test-run/charge"
    );
    let result_charge: Value = serde_json::from_str(request_parts(&requests[3]).3).unwrap();
    assert_eq!(result_charge, json!({"eventName":"result","count":2}));

    assert_eq!(request_parts(&requests[4]).0, "POST");
    assert_eq!(
        request_parts(&requests[4]).1,
        "/v2/datasets/test-dataset/items"
    );
    assert_eq!(
        serde_json::from_str::<Value>(request_parts(&requests[4]).3).unwrap(),
        output["items"]
    );

    assert_eq!(request_parts(&requests[5]).0, "PUT");
    assert_eq!(
        request_parts(&requests[5]).1,
        "/v2/key-value-stores/test-store/records/OUTPUT"
    );
    assert_eq!(
        serde_json::from_str::<Value>(request_parts(&requests[5]).3).unwrap(),
        output
    );
}

#[tokio::test]
async fn result_budget_caps_dataset_rows_after_search_charge_and_keeps_full_output() {
    let input = test_input();
    let output = json!({
        "items": [
            {"name":"First Cafe","business_id":"first"},
            {"name":"Second Cafe","business_id":"second"},
            {"name":"Third Cafe","business_id":"third"},
            {"name":"Fourth Cafe","business_id":"fourth"}
        ],
        "pagination": {"page": 0}
    });
    let server = MockServer::start(vec![
        response(200, &input.to_string()),
        response(201, "{}"),
        response(200, &output.to_string()),
        response(201, "{}"),
        response(201, "{}"),
        response(200, "{}"),
    ]);
    let mut config = config(&server);
    config.max_total_charge_usd = Some(0.0021);
    run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 6);
    let result_charge: Value = serde_json::from_str(request_parts(&requests[3]).3).unwrap();
    assert_eq!(result_charge, json!({"eventName":"result","count":3}));
    let dataset: Value = serde_json::from_str(request_parts(&requests[4]).3).unwrap();
    assert_eq!(dataset.as_array().unwrap().len(), 3);
    assert_eq!(dataset[0]["business_id"], "first");
    assert_eq!(dataset[2]["business_id"], "third");
    assert_eq!(
        serde_json::from_str::<Value>(request_parts(&requests[5]).3).unwrap(),
        output
    );
}

#[tokio::test]
async fn default_dataset_event_cost_is_included_in_result_budget() {
    let input = test_input();
    let output = json!({
        "items": [
            {"name":"First Cafe","business_id":"first"},
            {"name":"Second Cafe","business_id":"second"}
        ],
        "pagination": {"page": 0}
    });
    let server = MockServer::start(vec![
        response(200, &input.to_string()),
        response(201, "{}"),
        response(200, &output.to_string()),
        response(201, "{}"),
        response(201, "{}"),
        response(200, "{}"),
    ]);
    let mut config = config(&server);
    config.pricing_info = Some(pricing_info_with_dataset_item_price());
    config.max_total_charge_usd = Some(0.00215);
    run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 6);
    let result_charge: Value = serde_json::from_str(request_parts(&requests[3]).3).unwrap();
    assert_eq!(result_charge, json!({"eventName":"result","count":1}));
    let dataset: Value = serde_json::from_str(request_parts(&requests[4]).3).unwrap();
    assert_eq!(dataset.as_array().unwrap().len(), 1);
    assert_eq!(dataset[0]["business_id"], "first");
    assert_eq!(
        serde_json::from_str::<Value>(request_parts(&requests[5]).3).unwrap(),
        output
    );
}

#[tokio::test]
async fn exhausted_search_budget_stops_before_scrappa_or_output_writes() {
    let server = MockServer::start(vec![response(200, &test_input().to_string())]);
    let mut config = config(&server);
    config.max_total_charge_usd = Some(0.0001);
    run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
        .await
        .unwrap();
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        request_parts(&requests[0]).1,
        "/v2/key-value-stores/test-store/records/INPUT"
    );
}

#[tokio::test]
async fn exact_search_budget_boundary_stops_before_scrappa_or_output_writes() {
    let server = MockServer::start(vec![
        response(200, &test_input().to_string()),
        response(201, "{}"),
    ]);
    let mut config = config(&server);
    config.max_total_charge_usd = Some(0.0011);
    run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(request_parts(&requests[0]).0, "GET");
    assert_eq!(request_parts(&requests[1]).0, "POST");
    assert_eq!(
        serde_json::from_str::<Value>(request_parts(&requests[1]).3).unwrap(),
        json!({"eventName":"search","count":1})
    );
}

#[tokio::test]
async fn run_metadata_is_loaded_when_apify_charge_environment_is_missing() {
    let input = test_input();
    let run = json!({
        "data": {
            "pricingInfo": pricing_info(),
            "options": {},
            "chargedEventCounts": {"apify-actor-start": 1}
        }
    });
    let server = MockServer::start(vec![
        response(200, &input.to_string()),
        response(200, &run.to_string()),
        response(201, "{}"),
        response(200, "{\"items\":[]}"),
        response(200, "{}"),
    ]);
    let mut config = config(&server);
    config.pricing_info = None;
    config.charged_event_counts = None;
    config.max_total_charge_usd = None;
    run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    assert_eq!(request_parts(&requests[1]).0, "GET");
    assert_eq!(request_parts(&requests[1]).1, "/v2/actor-runs/test-run");
    assert_eq!(
        request_parts(&requests[2]).1,
        "/v2/actor-runs/test-run/charge"
    );
    assert_eq!(
        request_parts(&requests[4]).1,
        "/v2/key-value-stores/test-store/records/OUTPUT"
    );
}

#[tokio::test]
async fn missing_spending_limit_uses_unbounded_budget_without_api_lookup() {
    let server = MockServer::start(vec![]);
    let mut config = config(&server);
    config.max_total_charge_usd = None;
    let http = client(APIFY_REQUEST_TIMEOUT);
    let budget = load_charge_budget(&http, &config).await.unwrap();
    assert_eq!(budget.max_total_charge_usd, f64::INFINITY);
    assert_eq!(budget.affordable_count(SEARCH_EVENT, 1).unwrap(), 1);
    assert!(server.requests().is_empty());
}

#[tokio::test]
async fn missing_input_record_keeps_actor_validation_error() {
    let server = MockServer::start(vec![response(404, "not found")]);
    let config = config(&server);
    let error = run_actor(&client(APIFY_REQUEST_TIMEOUT), &config)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Search query and zoom level are required"
    );
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn affordable_result_count_accounts_for_every_priced_event() {
    let budget =
        ChargeBudget::from_metadata(&pricing_info(), &json!({"apify-actor-start":1}), 0.0014)
            .unwrap();
    assert_eq!(budget.affordable_count(SEARCH_EVENT, 1).unwrap(), 1);
    let mut budget = budget;
    budget.charged_event_counts.insert("search".to_owned(), 1);
    assert_eq!(budget.affordable_count(RESULT_EVENT, 5).unwrap(), 1);

    let no_room =
        ChargeBudget::from_metadata(&pricing_info(), &json!({"apify-actor-start": 2}), 0.0001)
            .unwrap();
    assert_eq!(no_room.affordable_count(SEARCH_EVENT, 1).unwrap(), 0);
}

#[tokio::test]
async fn scrappa_http_errors_keep_structured_details_and_do_not_retry() {
    let server = MockServer::start(vec![response(
        503,
        r#"{"message":"upstream busy","errors":{"query":["try later"]}}"#,
    )]);
    let config = config(&server);
    let error = fetch_search(&client(APIFY_REQUEST_TIMEOUT), &config, &test_input())
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Scrappa API error (503): upstream busy - query: try later"
    );
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn scrappa_request_keeps_the_sixty_second_deadline_message() {
    let server = MockServer::start(vec![delayed_response(
        200,
        r#"{"items":[]}"#,
        Duration::from_millis(100),
    )]);
    let mut config = config(&server);
    config.scrappa_request_timeout = Duration::from_millis(20);
    let error = fetch_search(&client(APIFY_REQUEST_TIMEOUT), &config, &test_input())
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Scrappa API request timed out after 20ms"
    );
}

#[tokio::test]
async fn apify_storage_reads_retry_transient_server_errors() {
    let server = MockServer::start(vec![
        response(503, "temporary"),
        response(200, r#"{"query":"coffee shops","zoom":15}"#),
    ]);
    let config = config(&server);
    let http = client(APIFY_REQUEST_TIMEOUT);
    let input = ApifyClient::new(&http, &config).get_input().await.unwrap();
    assert_eq!(input["query"], "coffee shops");
    assert_eq!(server.requests().len(), 2);
}

#[tokio::test]
async fn dataset_writes_do_not_retry_ambiguous_or_transient_failures() {
    for responses in [
        vec![lost_response(), response(201, "{}")],
        vec![response(503, "temporary"), response(201, "{}")],
    ] {
        let server = MockServer::start(responses);
        let config = config(&server);
        let http = client(APIFY_REQUEST_TIMEOUT);
        let apify = ApifyClient::new(&http, &config);
        assert!(apify
            .push_dataset_items(&[json!({"id":"only-once"})])
            .await
            .is_err());
        assert_eq!(server.requests().len(), 1);
    }
}

#[test]
fn dataset_writes_split_batches_below_apify_payload_limit() {
    let items = vec![
        json!({"payload":"a".repeat(2_400_000)}),
        json!({"payload":"b".repeat(2_400_000)}),
    ];
    let batches = dataset_batches(&items).unwrap();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].len(), 1);
    assert_eq!(batches[1].len(), 1);
}

#[test]
fn missing_query_or_zoom_keeps_actor_validation_error() {
    let base_url = Url::parse(SCRAPPA_API_DEFAULT).unwrap();
    for input in [json!({}), json!({"query":""}), json!({"query":"coffee"})] {
        assert_eq!(
            build_search_url(&input, &base_url).unwrap_err().to_string(),
            "Search query and zoom level are required"
        );
    }
}
