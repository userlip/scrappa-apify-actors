use google_trends_interest_scraper::{
    apify::DEFAULT_DATASET_ITEM_EVENT,
    apify::{ppe_items_result, PpeBudget},
    config::Config,
    input::{build_interest_params, InterestParams},
    response::build_timeline_dataset_items,
    runtime::{actor_error_message, run_actor},
    scrappa::{
        fetch_interest, retry_delay_ms, retryable_scrappa_error, scrappa_error_message,
        ScrappaHttpError, ScrappaTimeoutError, SCRAPPA_USER_AGENT,
    },
};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};
use url::Url;

const TIMELINE_RESPONSE: &str = r#"{"search_parameters":{"keyword":"tesla","geo":"US"},"timeline_data":[{"timestamp":1704067200,"date":"2024-01-01","value":42},{"timestamp":1704672000,"date":"2024-01-08","value":58}],"interest_over_time":{"average":50,"max_value":100,"min_value":12},"response_time_ms":587}"#;
const THREE_POINT_RESPONSE: &str = r#"{"timeline_data":[{"value":1},{"value":2},{"value":3}]}"#;
const NON_PPE_RUN: &str = r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_ACTOR"},"chargedEventCounts":{},"options":{}}}"#;

fn test_config(server_url: Url, scrappa_api_key: &str) -> Config {
    Config {
        apify_api_base: server_url.clone(),
        scrappa_api_base: format!("{server_url}api"),
        apify_token: "test-apify-token".to_owned(),
        key_value_store_id: "store-id".to_owned(),
        dataset_id: "dataset-id".to_owned(),
        actor_run_id: "run-id".to_owned(),
        input_key: "INPUT".to_owned(),
        scrappa_api_key: scrappa_api_key.to_owned(),
    }
}

fn mock_server(
    responses: Vec<(&'static str, &'static str)>,
) -> (Url, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            requests.push(read_request(&mut stream));
            let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
            stream.write_all(response.as_bytes()).unwrap();
        }
        requests
    });
    (Url::parse(&format!("http://{address}/")).unwrap(), server)
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut reader = BufReader::new(stream);
    let mut headers = String::new();
    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();
    headers.push_str(&request_line);
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line == "\r\n" || line.is_empty() {
            break;
        }
        headers.push_str(&line);
    }
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).unwrap();
    format!("{headers}\r\n{}", String::from_utf8(body).unwrap())
}

fn request_body(request: &str) -> Value {
    serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
}

fn ppe_run(event_price: f64, dataset_item_price: Option<f64>, budget: f64) -> Value {
    let mut events = json!({
        "timeline-point": { "eventPriceUsd": event_price }
    });
    if let Some(price) = dataset_item_price {
        events[DEFAULT_DATASET_ITEM_EVENT] = json!({ "eventPriceUsd": price });
    }
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": { "actorChargeEvents": events }
            },
            "chargedEventCounts": {},
            "options": { "maxTotalChargeUsd": budget }
        }
    })
}

#[test]
fn builds_normalized_interest_params_and_log_description() {
    let params = build_interest_params(&json!({
        "q": " tesla ",
        "geo": " us ",
        "time_range": "1Y",
        "hl": "EN",
        "search_type": "YouTube"
    }))
    .unwrap();
    assert_eq!(
        params,
        InterestParams {
            q: "tesla".to_owned(),
            geo: Some("US".to_owned()),
            time_range: Some("1y".to_owned()),
            hl: Some("en".to_owned()),
            search_type: Some("youtube".to_owned()),
        }
    );
    assert_eq!(
        params.describe(),
        "\"tesla\" (geo=US, time_range=1y, hl=en, search_type=youtube)"
    );
}

#[test]
fn normalizes_worldwide_and_omits_empty_optional_values() {
    let params = build_interest_params(&json!({
        "q": " bitcoin ",
        "geo": " worldwide ",
        "hl": "   "
    }))
    .unwrap();
    assert_eq!(params.geo.as_deref(), Some("Worldwide"));
    assert_eq!(params.hl, None);
    assert_eq!(
        params.query_pairs(),
        vec![("q", "bitcoin"), ("geo", "Worldwide")]
    );
}

#[test]
fn matches_input_validation_messages_and_limits() {
    assert_eq!(
        build_interest_params(&json!({})).unwrap_err().to_string(),
        "q is required"
    );
    assert_eq!(
        build_interest_params(&json!({ "q": "tesla", "time_range": "2y" }))
            .unwrap_err()
            .to_string(),
        "time_range must be one of: 1h, 4h, 1d, 7d, 30d, 90d, 1y, 5y, all"
    );
    assert_eq!(
        build_interest_params(&json!({ "q": "tesla", "hl": "eng" }))
            .unwrap_err()
            .to_string(),
        "hl must be 2 characters or fewer"
    );
    assert_eq!(
        build_interest_params(&json!({ "q": "tesla", "search_type": "podcasts" }))
            .unwrap_err()
            .to_string(),
        "search_type must be one of: web, images, news, youtube, shopping"
    );
    assert!(build_interest_params(&json!({ "q": "x".repeat(101) })).is_err());
    assert!(build_interest_params(&json!({ "q": "tesla", "geo": 1 })).is_err());
}

#[test]
fn preserves_actor_schema_prefills_and_defaults() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    assert_eq!(schema["properties"]["q"]["prefill"], "tesla");
    assert_eq!(schema["properties"]["geo"]["default"], "US");
    assert_eq!(schema["properties"]["time_range"]["default"], "1y");
    assert_eq!(schema["properties"]["hl"]["default"], "en");
    assert_eq!(schema["properties"]["search_type"]["default"], "web");
}

#[test]
fn builds_dataset_rows_from_primary_timeline_and_summary() {
    let params = build_interest_params(&json!({
        "q": "tesla", "geo": "US", "time_range": "1y", "hl": "en", "search_type": "web"
    }))
    .unwrap();
    let response: Value = serde_json::from_str(TIMELINE_RESPONSE).unwrap();
    let items = build_timeline_dataset_items(&response, &params);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["position"], 1);
    assert_eq!(items[0]["timestamp"], 1704067200_i64);
    assert_eq!(items[0]["date"], "2024-01-01");
    assert_eq!(items[0]["value"], 42);
    assert_eq!(items[0]["average"], 50);
    assert_eq!(items[0]["max_value"], 100);
    assert_eq!(items[0]["min_value"], 12);
    assert_eq!(items[0]["request_q"], "tesla");
    assert_eq!(items[0]["request_search_type"], "web");
    assert_eq!(items[0]["response_time_ms"], 587);
    assert_eq!(
        items[0]["search_parameters"],
        json!({ "keyword": "tesla", "geo": "US" })
    );
}

#[test]
fn falls_back_to_interest_data_points_when_primary_has_no_objects() {
    let response = json!({
        "timeline_data": [null, ["not a point"]],
        "interest_over_time": {
            "data_points": [
                { "timestamp": 1, "date": "fallback", "value": 7 },
                null
            ]
        }
    });
    let params = build_interest_params(&json!({ "q": "tesla" })).unwrap();
    let items = build_timeline_dataset_items(&response, &params);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["date"], "fallback");
    assert_eq!(items[0]["position"], 1);
    assert_eq!(items[0]["request_geo"], Value::Null);
}

#[test]
fn keeps_an_over_budget_item_to_match_the_sdk_charge_limit_behavior() {
    let mut budget = PpeBudget::from_actor_run(&ppe_run(0.01, None, 0.005))
        .unwrap()
        .unwrap();
    let (kept, custom, dataset) = ppe_items_result(&mut budget, 3);
    assert_eq!(kept, 1);
    assert_eq!(custom.charged_count, 1);
    assert!(custom.event_charge_limit_reached);
    assert_eq!(dataset.charged_count, 1);
    assert_eq!(custom.merge(dataset).charged_count, 2);
}

#[test]
fn combined_ppe_prices_limit_rows_to_the_user_budget() {
    let run = ppe_run(0.0003, Some(0.0001), 0.001);
    let mut budget = PpeBudget::from_actor_run(&run).unwrap().unwrap();
    budget
        .charged_counts
        .insert("apify-actor-start".to_owned(), 1);
    // The start event has no configured price in this fixture and therefore matches the SDK's zero-priced fallback.
    assert_eq!(budget.dataset_item_limit(4), 2);
}

#[test]
fn matches_retryable_scrappa_statuses_and_backoff_schedule() {
    for status in [408, 429, 500, 502, 503, 504] {
        let error = anyhow::Error::new(ScrappaHttpError {
            status: StatusCode::from_u16(status).unwrap(),
            message: "retry".to_owned(),
        });
        assert!(retryable_scrappa_error(&error));
    }
    let permanent = anyhow::Error::new(ScrappaHttpError {
        status: StatusCode::BAD_REQUEST,
        message: "bad input".to_owned(),
    });
    assert!(!retryable_scrappa_error(&permanent));
    assert!(retryable_scrappa_error(&anyhow::Error::new(
        ScrappaTimeoutError
    )));
    assert_eq!(retry_delay_ms(1, 250), 2250);
    assert_eq!(retry_delay_ms(2, 250), 4250);
    assert_eq!(retry_delay_ms(3, 250), 8250);
    assert_eq!(retry_delay_ms(4, 250), 10_000);
}

#[test]
fn formats_timeout_message_with_the_actor_specific_guidance() {
    let message = actor_error_message(&anyhow::Error::new(ScrappaTimeoutError));
    assert_eq!(
            message,
            "Scrappa API request timed out after 60000ms. The Google Trends interest request exceeded the 60s Scrappa API timeout. Try a shorter time range, a more specific keyword, or run the request again."
        );
}

#[test]
fn formats_scrappa_validation_errors_like_the_typescript_client() {
    assert_eq!(
        scrappa_error_message(
            r#"{"message":"The given data was invalid.","errors":{"q":["The q field is required.","Another message"]}}"#,
            "Unprocessable Entity"
        ),
        "The given data was invalid. - q: The q field is required., Another message"
    );
    assert_eq!(
        scrappa_error_message(" upstream   unavailable ", "Service Unavailable"),
        "upstream unavailable"
    );
}

#[tokio::test]
async fn retries_retryable_scrappa_status_and_preserves_upstream_auth_and_query() {
    let (base_url, server) = mock_server(vec![
        (
            "503 Service Unavailable",
            r#"{"message":"upstream unavailable"}"#,
        ),
        ("200 OK", TIMELINE_RESPONSE),
    ]);
    let config = test_config(base_url.clone(), "scrappa-test-key");
    let params = build_interest_params(&json!({
        "q": "tesla model y", "geo": "US", "time_range": "1y", "hl": "en", "search_type": "web"
    }))
    .unwrap();
    let response = fetch_interest(&Client::new(), &config, &params)
        .await
        .unwrap();
    assert_eq!(response["timeline_data"].as_array().unwrap().len(), 2);
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 2);
    for request in requests {
        let request_lower = request.to_ascii_lowercase();
        assert!(request_lower.starts_with("get /api/google-trends/interest?q=tesla+model+y&geo=us&time_range=1y&hl=en&search_type=web"));
        assert!(request_lower.contains("x-api-key: scrappa-test-key"));
        assert!(request_lower.contains(SCRAPPA_USER_AGENT));
    }
}

#[tokio::test]
async fn writes_dataset_and_full_output_for_non_ppe_runs() {
    let input = r#"{"q":"tesla","geo":"US","time_range":"1y","hl":"en","search_type":"web"}"#;
    let (base_url, server) = mock_server(vec![
        ("200 OK", NON_PPE_RUN),
        ("200 OK", input),
        ("200 OK", TIMELINE_RESPONSE),
        ("201 Created", ""),
        ("201 Created", ""),
    ]);
    let config = test_config(base_url, "scrappa-test-key");
    run_actor(&Client::new(), &config).await.unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 5);
    assert!(requests[3]
        .to_ascii_lowercase()
        .starts_with("post /v2/datasets/dataset-id/items"));
    let rows = request_body(&requests[3]);
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert!(requests[4]
        .to_ascii_lowercase()
        .starts_with("put /v2/key-value-stores/store-id/records/output"));
    assert_eq!(
        request_body(&requests[4]),
        serde_json::from_str::<Value>(TIMELINE_RESPONSE).unwrap()
    );
    assert!(requests
        .iter()
        .all(|request| !request.to_ascii_lowercase().contains("/charge")));
}

#[tokio::test]
async fn charges_ppe_results_with_the_named_event_and_stores_output() {
    let input = r#"{"q":"tesla"}"#;
    let run = serde_json::to_string(&ppe_run(0.0001, Some(0.0), 1.0)).unwrap();
    let (base_url, server) = mock_server(vec![
        ("200 OK", Box::leak(run.into_boxed_str())),
        ("200 OK", input),
        ("200 OK", TIMELINE_RESPONSE),
        ("201 Created", ""),
        ("201 Created", "{}"),
        ("201 Created", ""),
    ]);
    let config = test_config(base_url, "scrappa-test-key");
    run_actor(&Client::new(), &config).await.unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 6);
    let charge_request = &requests[4];
    assert!(charge_request
        .to_ascii_lowercase()
        .starts_with("post /v2/actor-runs/run-id/charge"));
    assert!(charge_request
        .to_ascii_lowercase()
        .contains("idempotency-key: run-id-timeline-point-"));
    assert_eq!(
        request_body(charge_request),
        json!({ "eventName": "timeline-point", "count": 2 })
    );
    assert!(requests[5]
        .to_ascii_lowercase()
        .starts_with("put /v2/key-value-stores/store-id/records/output"));
}

#[tokio::test]
async fn marks_one_of_three_ppe_rows_as_partial() {
    let input = r#"{"q":"tesla"}"#;
    let run = serde_json::to_string(&ppe_run(0.01, None, 0.005)).unwrap();
    let (base_url, server) = mock_server(vec![
        ("200 OK", Box::leak(run.into_boxed_str())),
        ("200 OK", input),
        ("200 OK", THREE_POINT_RESPONSE),
        ("201 Created", ""),
        ("201 Created", "{}"),
        ("200 OK", "{}"),
    ]);
    let config = test_config(base_url, "scrappa-test-key");
    run_actor(&Client::new(), &config).await.unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 6);
    assert_eq!(request_body(&requests[3]).as_array().unwrap().len(), 1);
    assert_eq!(request_body(&requests[4])["count"], 1);
    assert!(requests[5]
        .to_ascii_lowercase()
        .starts_with("put /v2/actor-runs/run-id "));
    let status = request_body(&requests[5]);
    assert_eq!(
        status["statusMessage"],
        "Charge limit reached before saving all Google Trends timeline points."
    );
    assert_eq!(status["isStatusMessageTerminal"], true);
    assert!(requests
        .iter()
        .all(|request| !request.to_ascii_lowercase().contains("records/output")));
}

#[tokio::test]
async fn marks_two_of_three_ppe_rows_as_partial() {
    let input = r#"{"q":"tesla"}"#;
    let run = serde_json::to_string(&ppe_run(0.0003, Some(0.0001), 0.0008)).unwrap();
    let (base_url, server) = mock_server(vec![
        ("200 OK", Box::leak(run.into_boxed_str())),
        ("200 OK", input),
        ("200 OK", THREE_POINT_RESPONSE),
        ("201 Created", ""),
        ("201 Created", "{}"),
        ("200 OK", "{}"),
    ]);
    let config = test_config(base_url, "scrappa-test-key");
    run_actor(&Client::new(), &config).await.unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 6);
    assert_eq!(request_body(&requests[3]).as_array().unwrap().len(), 2);
    assert_eq!(request_body(&requests[4])["count"], 2);
    assert!(requests[5]
        .to_ascii_lowercase()
        .starts_with("put /v2/actor-runs/run-id "));
    let status = request_body(&requests[5]);
    assert_eq!(
        status["statusMessage"],
        "Charge limit reached before saving all Google Trends timeline points."
    );
    assert_eq!(status["isStatusMessageTerminal"], true);
    assert!(requests
        .iter()
        .all(|request| !request.to_ascii_lowercase().contains("records/output")));
}

#[tokio::test]
async fn writes_full_output_when_all_ppe_rows_are_free() {
    let input = r#"{"q":"tesla"}"#;
    let run = serde_json::to_string(&ppe_run(0.0, Some(0.0), 0.0)).unwrap();
    let (base_url, server) = mock_server(vec![
        ("200 OK", Box::leak(run.into_boxed_str())),
        ("200 OK", input),
        ("200 OK", THREE_POINT_RESPONSE),
        ("201 Created", ""),
        ("201 Created", "{}"),
        ("201 Created", ""),
    ]);
    let config = test_config(base_url, "scrappa-test-key");
    run_actor(&Client::new(), &config).await.unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 6);
    assert_eq!(request_body(&requests[3]).as_array().unwrap().len(), 3);
    assert_eq!(
        request_body(&requests[4]),
        json!({ "eventName": "timeline-point", "count": 3 })
    );
    assert!(requests[5]
        .to_ascii_lowercase()
        .starts_with("put /v2/key-value-stores/store-id/records/output"));
    assert_eq!(
        request_body(&requests[5]),
        serde_json::from_str::<Value>(THREE_POINT_RESPONSE).unwrap()
    );
}
