use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

const INPUT_SCHEMA: &str = include_str!("../.actor/input_schema.json");

struct MockRequest {
    method: String,
    target: String,
    headers: Map<String, Value>,
    body: String,
}

struct MockServer {
    base_url: String,
    requests: Receiver<MockRequest>,
    stop: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    fn start(responses: Vec<(u16, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let (sender, requests) = mpsc::channel();
        let (stop, stopped) = mpsc::channel();
        let thread = thread::spawn(move || {
            let mut served = 0;
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let request = read_mock_request(&mut stream);
                        sender.send(request).unwrap();
                        let (status, body) = responses
                            .get(served)
                            .cloned()
                            .unwrap_or((500, "unexpected mock request".to_owned()));
                        served += 1;
                        let reason = match status {
                            200 => "OK",
                            201 => "Created",
                            401 => "Unauthorized",
                            429 => "Too Many Requests",
                            500 => "Internal Server Error",
                            503 => "Service Unavailable",
                            _ => "Error",
                        };
                        write!(
                            stream,
                            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        )
                        .unwrap();
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if !matches!(stopped.try_recv(), Err(mpsc::TryRecvError::Empty)) {
                            break;
                        }
                        thread::sleep(std::time::Duration::from_millis(1));
                    }
                    Err(error) => panic!("mock server accept failed: {error}"),
                }
            }
        });
        Self {
            base_url: format!("http://{address}"),
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn finish(mut self) -> Vec<MockRequest> {
        let _ = self.stop.send(());
        self.thread.take().unwrap().join().unwrap();
        self.requests.try_iter().collect()
    }
}

fn read_mock_request(stream: &mut TcpStream) -> MockRequest {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
        if let Some(header_end) = header_end {
            let header_text = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = header_text
                .lines()
                .skip(1)
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
        let count = stream.read(&mut buffer).unwrap();
        assert!(count > 0, "mock request ended before its body was read");
        bytes.extend_from_slice(&buffer[..count]);
    }
    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap();
    let header_text = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = header_text.lines();
    let mut request_line = lines.next().unwrap().split_whitespace();
    let method = request_line.next().unwrap().to_owned();
    let target = request_line.next().unwrap().to_owned();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| {
            (
                name.to_ascii_lowercase(),
                Value::String(value.trim().to_owned()),
            )
        })
        .collect();
    let body = String::from_utf8(bytes[header_end + 4..].to_vec()).unwrap();
    MockRequest {
        method,
        target,
        headers,
        body,
    }
}

fn test_config(base_url: &str) -> Config {
    Config {
        apify_api_base: Url::parse(base_url).unwrap(),
        scrappa_api_base: Url::parse(base_url).unwrap(),
        apify_token: "apify-test-token".to_owned(),
        key_value_store_id: "store-id".to_owned(),
        dataset_id: "dataset-id".to_owned(),
        actor_run_id: "test-run".to_owned(),
        input_key: "INPUT".to_owned(),
        scrappa_api_key: "scrappa-test-key".to_owned(),
    }
}

fn mock_response(status: u16, body: Value) -> (u16, String) {
    (status, body.to_string())
}

fn run_response(pricing_info: Value, charged: Value, max_charge: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": pricing_info,
            "chargedEventCounts": charged,
            "options": { "maxTotalChargeUsd": max_charge },
        }
    })
}

fn ppe_pricing(events: Value) -> Value {
    json!({
        "pricingModel": "PAY_PER_EVENT",
        "pricingPerEvent": { "actorChargeEvents": events },
    })
}

fn ppe_run_body() -> Value {
    run_response(
        ppe_pricing(json!({
            "suggestion-result": { "eventPriceUsd": 0.1 },
            "apify-default-dataset-item": { "eventPriceUsd": 0.02 },
            "actor-start": { "eventPriceUsd": 0.15 },
        })),
        json!({ "actor-start": 1 }),
        json!(0.4),
    )
}

fn request_target(request: &MockRequest) -> String {
    request.target.clone()
}

#[test]
fn input_schema_keeps_prefill_alias_and_defaults() {
    let schema: Value = serde_json::from_str(INPUT_SCHEMA).unwrap();
    assert_eq!(schema["properties"]["query"]["prefill"], "tesla");
    assert_eq!(schema["properties"]["query"]["maxLength"], 100);
    assert_eq!(schema["properties"]["q"]["maxLength"], 100);
    assert!(schema.get("required").is_none());
    assert_eq!(schema["properties"]["geo"]["default"], "US");
    assert_eq!(schema["properties"]["hl"]["default"], "en");
}

#[test]
fn query_builder_preserves_alias_defaults_and_normalization() {
    let params = build_autocomplete_params(&json!({
        "query": " tesla ",
        "q": "ignored",
        "geo": " us ",
        "hl": "EN",
    }))
    .unwrap();
    assert_eq!(
        Value::Object(params.as_map()),
        json!({ "q": "tesla", "geo": "US", "hl": "en" })
    );
    assert_eq!(params.describe(), "\"tesla\" (geo=US, hl=en)");

    let alias =
        build_autocomplete_params(&json!({ "query": " ", "q": "bitcoin", "geo": "worldwide" }))
            .unwrap();
    assert_eq!(
        Value::Object(alias.as_map()),
        json!({ "q": "bitcoin", "geo": "Worldwide", "hl": "en" })
    );
    assert_eq!(
        Value::Object(
            build_autocomplete_params(&json!({ "query": "coffee" }))
                .unwrap()
                .as_map()
        ),
        json!({ "q": "coffee", "geo": "US", "hl": "en" })
    );
}

#[test]
fn query_builder_rejects_invalid_values_and_missing_queries() {
    assert_eq!(
        build_autocomplete_params(&json!({}))
            .unwrap_err()
            .to_string(),
        "query is required"
    );
    assert_eq!(
        build_autocomplete_params(&json!({ "query": 12 }))
            .unwrap_err()
            .to_string(),
        "query must be a string"
    );
    assert_eq!(
        build_autocomplete_params(&json!({ "query": "tesla", "geo": 12 }))
            .unwrap_err()
            .to_string(),
        "geo must be a string"
    );
    assert_eq!(
        build_autocomplete_params(&json!({ "query": "tesla", "hl": "eng" }))
            .unwrap_err()
            .to_string(),
        "hl must be 2 characters or fewer"
    );
    assert_eq!(
        build_autocomplete_params(&json!({ "query": "tesla", "hl": "e1" }))
            .unwrap_err()
            .to_string(),
        "hl must be a two-letter language code"
    );
}

#[test]
fn autocomplete_items_keep_raw_fields_and_normalize_common_names() {
    let params = build_autocomplete_params(&json!({ "query": "tesla" })).unwrap();
    let items = build_autocomplete_dataset_items(
        &json!({
            "search_parameters": { "q": "tesla", "geo": "US", "hl": "en" },
            "suggestions": [
                "tesla stock",
                { "query": "tesla price", "type": "query", "extra": "kept" },
                { "title": "Tesla, Inc.", "type": "company" },
                42,
            ],
            "response_time_ms": 623,
        }),
        &params,
    );
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["suggestion"], "tesla stock");
    assert_eq!(items[0]["position"], 1);
    assert_eq!(items[0]["type"], Value::Null);
    assert_eq!(items[0]["source_keyword"], "tesla");
    assert_eq!(items[0]["request_geo"], "US");
    assert_eq!(items[0]["request_hl"], "en");
    assert_eq!(items[0]["response_time_ms"], 623);
    assert_eq!(items[1]["query"], "tesla price");
    assert_eq!(items[1]["suggestion"], "tesla price");
    assert_eq!(items[1]["extra"], "kept");
    assert_eq!(items[2]["suggestion"], "Tesla, Inc.");
    assert_eq!(items[2]["position"], 3);
}

#[test]
fn response_accepts_nested_containers_and_empty_shapes() {
    let params = build_autocomplete_params(&json!({ "query": "coffee" })).unwrap();
    assert_eq!(
        build_autocomplete_dataset_items(
            &json!({ "data": { "suggestions": ["coffee shop"] } }),
            &params
        )[0]["suggestion"],
        "coffee shop"
    );
    assert_eq!(
        build_autocomplete_dataset_items(
            &json!({ "autocomplete": { "results": [{ "keyword": "coffee beans" }] } }),
            &params
        )[0]["suggestion"],
        "coffee beans"
    );
    assert!(
        build_autocomplete_dataset_items(&json!({ "suggestions": "none" }), &params).is_empty()
    );
    assert!(build_autocomplete_dataset_items(
        &json!({ "suggestions": null, "results": [] }),
        &params
    )
    .is_empty());
}

#[test]
fn ppe_budget_counts_all_priced_events_and_limits_rows() {
    let pricing = PricingState::from_run(&ppe_run_body()).unwrap();
    assert_eq!(pricing.affordable_suggestion_count(5).unwrap(), 2);

    let free_pricing = PricingState::from_run(&run_response(
        json!({ "pricingModel": "FLAT_RATE" }),
        json!({}),
        Value::Null,
    ))
    .unwrap();
    assert_eq!(free_pricing.affordable_suggestion_count(5).unwrap(), 5);
}

#[test]
fn retry_policy_covers_timeouts_transient_statuses_and_network_errors() {
    assert!(ScrappaRequestError::Timeout.is_retryable());
    for status in [408, 429, 500, 502, 503, 504] {
        assert!(ScrappaRequestError::Api {
            status,
            message: String::new()
        }
        .is_retryable());
    }
    for status in [400, 401, 403, 404, 422] {
        assert!(!ScrappaRequestError::Api {
            status,
            message: String::new()
        }
        .is_retryable());
    }
    assert_eq!(get_retry_delay_ms(1, 0), 2000);
    assert_eq!(get_retry_delay_ms(2, 250), 4250);
    assert_eq!(get_retry_delay_ms(10, 0), 10_000);
}

#[test]
fn scrappa_errors_keep_auth_and_validation_messages() {
    assert_eq!(
        read_scrappa_error_message(
            r#"{"message":"Validation failed","errors":{"q":["required","too long"]}}"#,
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        "Validation failed - q: required, too long"
    );
    assert_eq!(
        read_scrappa_error_message("upstream   unavailable", StatusCode::BAD_GATEWAY),
        "upstream unavailable"
    );
    assert!(
        !ScrappaRequestError::InvalidJson(serde_json::from_str::<Value>("{").unwrap_err())
            .is_retryable()
    );
}

#[tokio::test]
async fn scrappa_client_retries_transient_status_and_preserves_headers_and_query() {
    let server = MockServer::start(vec![
        mock_response(503, json!({ "message": "temporarily busy" })),
        mock_response(200, json!({ "suggestions": ["coffee shop"] })),
    ]);
    let server_base_url = server.base_url.clone();
    let base_url = Url::parse(&server_base_url).unwrap();
    let http = Client::new();
    let client = ScrappaClient {
        http: &http,
        base_url: &base_url,
        api_key: "scrappa-test-key",
    };
    let params =
        build_autocomplete_params(&json!({ "query": "coffee & tea", "geo": "us" })).unwrap();
    let response = client.get_autocomplete(&params).await.unwrap();
    assert_eq!(response["suggestions"][0], "coffee shop");

    let requests = server.finish();
    assert_eq!(requests.len(), 2);
    let url = Url::parse(&format!(
        "{server_base_url}{}",
        request_target(&requests[0])
    ))
    .unwrap();
    assert_eq!(url.path(), "/google-trends/autocomplete");
    assert_eq!(
        url.query_pairs().find(|(key, _)| key == "q").unwrap().1,
        "coffee & tea"
    );
    assert_eq!(
        url.query_pairs().find(|(key, _)| key == "geo").unwrap().1,
        "US"
    );
    assert_eq!(requests[0].headers["x-api-key"], "scrappa-test-key");
    assert_eq!(requests[0].headers["accept"], "application/json");
    assert_eq!(requests[0].headers["user-agent"], SCRAPPA_USER_AGENT);
}

#[tokio::test]
async fn actor_writes_dataset_and_raw_kv_output_for_non_ppe_runs() {
    let server = MockServer::start(vec![
        mock_response(
            200,
            run_response(
                json!({ "pricingModel": "FLAT_RATE" }),
                json!({}),
                Value::Null,
            ),
        ),
        mock_response(200, json!({ "query": "tesla", "geo": "US" })),
        mock_response(
            200,
            json!({
                "search_parameters": { "q": "tesla" },
                "suggestions": ["tesla stock", { "query": "tesla model y", "type": "query" }],
                "response_time_ms": 321,
            }),
        ),
        (200, String::new()),
        (200, String::new()),
    ]);
    let config = test_config(&server.base_url);
    let http = Client::new();
    run_actor(&http, &config).await.unwrap();

    let requests = server.finish();
    assert_eq!(requests.len(), 5);
    assert_eq!(request_target(&requests[0]), "/v2/actor-runs/test-run");
    assert_eq!(
        request_target(&requests[1]),
        "/v2/key-value-stores/store-id/records/INPUT"
    );
    assert_eq!(
        requests[0].headers["authorization"],
        "Bearer apify-test-token"
    );
    assert_eq!(
        request_target(&requests[3]),
        "/v2/datasets/dataset-id/items"
    );
    assert_eq!(requests[3].method, "POST");
    let rows: Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert_eq!(rows[1]["position"], 2);
    assert_eq!(
        request_target(&requests[4]),
        "/v2/key-value-stores/store-id/records/OUTPUT"
    );
    let output: Value = serde_json::from_str(&requests[4].body).unwrap();
    assert_eq!(output["suggestion_count"], 2);
    assert_eq!(output["saved_suggestion_count"], 2);
    assert_eq!(output["charge_limit_reached"], false);
    assert_eq!(output["raw_response"]["response_time_ms"], 321);
}

#[tokio::test]
async fn actor_charges_only_saved_rows_and_omits_raw_response_at_budget_limit() {
    let server = MockServer::start(vec![
        mock_response(200, ppe_run_body()),
        mock_response(200, json!({ "query": "tesla" })),
        mock_response(
            200,
            json!({
                "search_parameters": { "q": "tesla" },
                "suggestions": ["one", "two", "three"],
            }),
        ),
        mock_response(201, json!({})),
        (200, String::new()),
        (200, String::new()),
        (200, String::new()),
    ]);
    let config = test_config(&server.base_url);
    let http = Client::new();
    run_actor(&http, &config).await.unwrap();

    let requests = server.finish();
    assert_eq!(requests.len(), 7);
    assert_eq!(
        request_target(&requests[3]),
        "/v2/datasets/dataset-id/items"
    );
    assert_eq!(requests[3].method, "POST");
    let rows: Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert_eq!(
        request_target(&requests[4]),
        "/v2/actor-runs/test-run/charge"
    );
    assert_eq!(requests[4].method, "POST");
    assert_eq!(
        requests[4].headers["idempotency-key"],
        "google-trends-autocomplete-test-run-suggestions"
    );
    let charge: Value = serde_json::from_str(&requests[4].body).unwrap();
    assert_eq!(
        charge,
        json!({ "eventName": "suggestion-result", "count": 2 })
    );
    assert_eq!(
        request_target(&requests[5]),
        "/v2/key-value-stores/store-id/records/OUTPUT"
    );
    let output: Value = serde_json::from_str(&requests[5].body).unwrap();
    assert_eq!(output["suggestion_count"], 3);
    assert_eq!(output["saved_suggestion_count"], 2);
    assert_eq!(output["charge_limit_reached"], true);
    assert_eq!(output["raw_response_omitted"], true);
    assert_eq!(output["raw_response"], Value::Null);
    assert_eq!(request_target(&requests[6]), "/v2/actor-runs/test-run");
    let status_message: Value = serde_json::from_str(&requests[6].body).unwrap();
    assert_eq!(status_message["isStatusMessageTerminal"], true);
    assert!(status_message["statusMessage"]
        .as_str()
        .unwrap()
        .contains("Charge limit reached"));
}

#[tokio::test]
async fn actor_returns_error_when_suggestion_charge_fails_after_saving_rows() {
    let server = MockServer::start(vec![
        mock_response(200, ppe_run_body()),
        mock_response(200, json!({ "query": "tesla" })),
        mock_response(
            200,
            json!({
                "search_parameters": { "q": "tesla" },
                "suggestions": ["one", "two", "three"],
            }),
        ),
        (201, String::new()),
        (500, json!({ "error": "charge failed" }).to_string()),
    ]);
    let config = test_config(&server.base_url);
    let http = Client::new();

    let error = run_actor(&http, &config).await.unwrap_err();
    assert!(error
        .to_string()
        .contains("suggestion result charge request failed"));

    let requests = server.finish();
    assert_eq!(requests.len(), 5);
    assert_eq!(
        request_target(&requests[3]),
        "/v2/datasets/dataset-id/items"
    );
    assert_eq!(
        request_target(&requests[4]),
        "/v2/actor-runs/test-run/charge"
    );
}

#[tokio::test]
async fn actor_does_not_charge_or_retry_when_dataset_write_fails() {
    let server = MockServer::start(vec![
        mock_response(200, ppe_run_body()),
        mock_response(200, json!({ "query": "tesla" })),
        mock_response(
            200,
            json!({
                "search_parameters": { "q": "tesla" },
                "suggestions": ["one", "two", "three"],
            }),
        ),
        (500, json!({ "error": "dataset write failed" }).to_string()),
    ]);
    let config = test_config(&server.base_url);
    let http = Client::new();

    let error = run_actor(&http, &config).await.unwrap_err();
    assert!(error
        .to_string()
        .contains("Apify dataset write failed with 500"));

    let requests = server.finish();
    assert_eq!(requests.len(), 4);
    assert_eq!(
        request_target(&requests[3]),
        "/v2/datasets/dataset-id/items"
    );
    assert_eq!(requests[3].method, "POST");
    assert!(requests
        .iter()
        .all(|request| request.target != "/v2/actor-runs/test-run/charge"));
}
