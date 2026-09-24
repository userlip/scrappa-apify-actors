mod common;

use kleinanzeigen_search_scraper::{run_actor, ApifyClient};
use reqwest::Client;
use serde_json::{json, Value};

use common::*;

#[tokio::test]
async fn non_ppe_run_preserves_pagination_auth_and_dataset_kv_output() {
    let server = MockServer::start(vec![
        input_response(json!({"query": "iphone case", "page": 3, "location": "Berlin"})),
        flat_pricing(),
        listing_response(2),
        flat_pricing(),
        MockResponse::json(201, json!({})),
        MockResponse::json(200, json!({})),
    ])
    .await;
    let config = test_config(&server.base_url);
    let client = Client::new();
    let output = run_actor(&client, &config).await.unwrap();
    assert_eq!(output.value["searches_requested"], 1);
    assert_eq!(output.value["searches_completed"], 1);
    assert_eq!(output.value["listings_extracted"], 2);
    assert_eq!(
        output.value["responses"][0]["response"]["data"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let requests = server.finish();
    assert_eq!(
        requests.first().unwrap().target,
        "/v2/key-value-stores/test-store/records/INPUT"
    );
    let scrappa = requests_to(&requests, "/api/kleinanzeigen/search");
    assert_eq!(scrappa.len(), 1);
    assert_eq!(
        scrappa[0].headers.get("x-api-key").unwrap(),
        "scrappa-test-key"
    );
    assert_eq!(
        scrappa[0].headers.get("user-agent").unwrap(),
        "thescrappa-kleinanzeigen-search-scraper/1.0"
    );
    let query = query_values(&scrappa[0].target);
    assert!(query.contains(&("query".to_owned(), "iphone case".to_owned())));
    assert!(query.contains(&("page".to_owned(), "3".to_owned())));
    assert!(query.contains(&("location".to_owned(), "Berlin".to_owned())));
    let dataset = requests_to(&requests, "/v2/datasets/test-dataset/items");
    assert_eq!(dataset.len(), 1);
    let rows = serde_json::from_str::<Value>(&dataset[0].body).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert_eq!(rows[0]["request_page"], 3);
    assert_eq!(
        rows[0]["image_url"],
        "https://img.kleinanzeigen.de/example.jpg"
    );
    let output_write = requests_to(&requests, "/v2/key-value-stores/test-store/records/OUTPUT");
    assert_eq!(output_write.len(), 1);
    assert_eq!(
        serde_json::from_str::<Value>(&output_write[0].body).unwrap(),
        output.value
    );
    assert!(requests
        .iter()
        .filter(|request| !request.target.starts_with("/api/"))
        .all(
            |request| request.headers.get("authorization").map(String::as_str)
                == Some("Bearer test-token")
        ));
    assert!(requests_to(&requests, "/v2/actor-runs/test-run/charge").is_empty());
}

#[tokio::test]
async fn ppe_writes_affordable_rows_then_retries_transient_charge_with_same_key() {
    let server = MockServer::start(vec![
        input_response(json!({"query": "iphone"})),
        ppe_pricing(0.25, 0),
        listing_response(3),
        ppe_pricing(0.25, 0),
        MockResponse::json(201, json!({})),
        MockResponse::text(503, "temporarily unavailable"),
        MockResponse::json(201, json!({})),
        MockResponse::json(200, json!({})),
        MockResponse::json(200, json!({})),
    ])
    .await;
    let config = test_config(&server.base_url);
    let client = Client::new();
    let output = run_actor(&client, &config).await.unwrap();
    assert_eq!(output.value["listings_extracted"], 1);
    assert!(output
        .status_message
        .as_deref()
        .unwrap()
        .contains("saving 1 of 3"));
    assert_eq!(
        output.value["responses"][0]["response"]["data"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    ApifyClient::new(&client, &config)
        .set_terminal_status_message(output.status_message.as_deref().unwrap())
        .await
        .unwrap();

    let requests = server.finish();
    let charge = requests_to(&requests, "/v2/actor-runs/test-run/charge");
    assert_eq!(charge.len(), 2);
    assert_eq!(
        serde_json::from_str::<Value>(&charge[0].body).unwrap(),
        json!({"eventName": "listing-result", "count": 1})
    );
    let key = charge[0].headers.get("idempotency-key").unwrap();
    assert!(key.starts_with("test-run-listing-result-1-"));
    assert_eq!(
        charge[1].headers.get("idempotency-key").unwrap(),
        key,
        "charge retries must reuse the same idempotency key"
    );
    assert_eq!(charge[1].body, charge[0].body);
    let charge_position = requests
        .iter()
        .position(|request| request.target.starts_with("/v2/actor-runs/test-run/charge"))
        .unwrap();
    let dataset_position = requests
        .iter()
        .position(|request| {
            request
                .target
                .starts_with("/v2/datasets/test-dataset/items")
        })
        .unwrap();
    assert!(dataset_position < charge_position);
    let dataset = requests_to(&requests, "/v2/datasets/test-dataset/items");
    assert_eq!(
        serde_json::from_str::<Value>(&dataset[0].body)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let terminal = requests_to(&requests, "/v2/actor-runs/test-run");
    assert_eq!(terminal.last().unwrap().method, "PUT");
    let terminal_body = serde_json::from_str::<Value>(&terminal.last().unwrap().body).unwrap();
    assert_eq!(
        terminal_body["statusMessage"],
        output.status_message.unwrap()
    );
    assert_eq!(terminal_body["isStatusMessageTerminal"], true);
}

#[tokio::test]
async fn zero_ppe_budget_skips_fetch_but_writes_output_and_terminal_message() {
    let server = MockServer::start(vec![
        input_response(json!({"query": "iphone", "page": 4})),
        ppe_pricing(0.05, 0),
        MockResponse::json(200, json!({})),
        MockResponse::json(200, json!({})),
    ])
    .await;
    let config = test_config(&server.base_url);
    let client = Client::new();
    let output = run_actor(&client, &config).await.unwrap();
    assert_eq!(output.value["searches_completed"], 0);
    assert_eq!(output.value["listings_extracted"], 0);
    assert!(output
        .status_message
        .as_deref()
        .unwrap()
        .contains("before fetching"));
    ApifyClient::new(&client, &config)
        .set_terminal_status_message(output.status_message.as_deref().unwrap())
        .await
        .unwrap();

    let requests = server.finish();
    assert!(requests_to(&requests, "/api/kleinanzeigen/search").is_empty());
    assert!(requests_to(&requests, "/v2/actor-runs/test-run/charge").is_empty());
    assert_eq!(
        requests_to(&requests, "/v2/key-value-stores/test-store/records/OUTPUT").len(),
        1
    );
}

#[tokio::test]
async fn retries_transient_scrappa_errors_but_auth_errors_fail_without_output() {
    let server = MockServer::start(vec![
        input_response(json!({"query": "iphone"})),
        flat_pricing(),
        MockResponse::json(503, json!({"message": "temporarily unavailable"})),
        listing_response(0),
        flat_pricing(),
        MockResponse::json(200, json!({})),
    ])
    .await;
    let config = test_config(&server.base_url);
    let output = run_actor(&Client::new(), &config).await.unwrap();
    assert_eq!(output.value["searches_completed"], 1);
    let requests = server.finish();
    let scrappa = requests_to(&requests, "/api/kleinanzeigen/search");
    assert_eq!(scrappa.len(), 2);
    assert_eq!(scrappa[0].target, scrappa[1].target);
    assert_eq!(
        requests_to(&requests, "/v2/datasets/test-dataset/items").len(),
        0
    );

    let server = MockServer::start(vec![
        input_response(json!({"query": "iphone"})),
        flat_pricing(),
        MockResponse::json(403, json!({"message": "forbidden"})),
    ])
    .await;
    let config = test_config(&server.base_url);
    let error = run_actor(&Client::new(), &config).await.unwrap_err();
    assert!(error
        .to_string()
        .contains("Scrappa API error (403): forbidden"));
    let requests = server.finish();
    assert_eq!(requests_to(&requests, "/api/kleinanzeigen/search").len(), 1);
    assert!(requests_to(&requests, "/v2/key-value-stores/test-store/records/OUTPUT").is_empty());
}

#[tokio::test]
async fn missing_scrappa_key_fails_before_loading_input() {
    let server = MockServer::start(Vec::new()).await;
    let mut config = test_config(&server.base_url);
    config.scrappa_api_key = None;
    let error = run_actor(&Client::new(), &config).await.unwrap_err();
    assert!(error.to_string().contains(
        "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."
    ));
    assert!(server.finish().is_empty());
}

#[tokio::test]
async fn dataset_failure_does_not_charge_or_write_output() {
    let server = MockServer::start(vec![
        input_response(json!({"query": "iphone"})),
        ppe_pricing(1.0, 0),
        listing_response(1),
        ppe_pricing(1.0, 0),
        MockResponse::text(500, "dataset unavailable"),
    ])
    .await;
    let config = test_config(&server.base_url);
    let error = run_actor(&Client::new(), &config).await.unwrap_err();
    assert!(error
        .to_string()
        .contains("Apify dataset write failed with 500"));
    let requests = server.finish();
    assert!(requests_to(&requests, "/v2/actor-runs/test-run/charge").is_empty());
    assert_eq!(
        requests_to(&requests, "/v2/datasets/test-dataset/items").len(),
        1
    );
    assert!(requests_to(&requests, "/v2/key-value-stores/test-store/records/OUTPUT").is_empty());
}
