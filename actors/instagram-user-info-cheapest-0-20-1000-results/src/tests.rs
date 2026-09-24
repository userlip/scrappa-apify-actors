use super::*;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

#[derive(Clone, Debug)]
struct RecordedRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String,
}

struct MockResponse {
    status: u16,
    body: String,
}

impl MockResponse {
    fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            body: body.to_string(),
        }
    }
}

struct MockServer {
    base_url: Url,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    fn start(handler: impl Fn(&RecordedRequest) -> MockResponse + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let base_url = Url::parse(&format!("http://{address}/")).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_for_thread = Arc::clone(&requests);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = Arc::clone(&stop);
        let handler = Arc::new(handler);
        let thread = thread::spawn(move || {
            while !stop_for_thread.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                        let request = read_request(&mut stream).unwrap();
                        requests_for_thread.lock().unwrap().push(request.clone());
                        let response = handler(&request);
                        write_response(&mut stream, response).unwrap();
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("Mock server accept failed: {error}"),
                }
            }
        });

        Self {
            base_url,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<RecordedRequest> {
    let mut bytes = Vec::new();
    let mut header_end = None;
    let mut content_length = 0;
    loop {
        let mut chunk = [0_u8; 4096];
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if header_end.is_none() {
            if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                header_end = Some(index);
                let headers = String::from_utf8_lossy(&bytes[..index]);
                content_length = headers
                    .lines()
                    .skip(1)
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
            }
        }
        if header_end.is_some_and(|index| bytes.len() >= index + 4 + content_length) {
            break;
        }
    }

    let header_end = header_end.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "HTTP request had no headers",
        )
    })?;
    let headers_text = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = headers_text.lines();
    let request_line = lines.next().unwrap_or_default();
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_owned();
    let path = request_parts.next().unwrap_or_default().to_owned();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    let body_start = header_end + 4;
    let body_end = (body_start + content_length).min(bytes.len());
    let body = String::from_utf8_lossy(&bytes[body_start..body_end]).to_string();

    Ok(RecordedRequest {
        method,
        path,
        headers,
        body,
    })
}

fn write_response(stream: &mut TcpStream, response: MockResponse) -> std::io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Response",
    };
    write!(
            stream,
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response.status,
            reason,
            response.body.len(),
            response.body
        )?;
    stream.flush()
}

fn config(apify_api_base: Url, scrappa_api_base: Url) -> ActorConfig {
    ActorConfig {
        apify_api_base,
        scrappa_api_base,
        apify_token: "apify-test-token".to_owned(),
        scrappa_api_key: "scrappa-test-key".to_owned(),
        actor_run_id: "test-run".to_owned(),
        key_value_store_id: "test-store".to_owned(),
        dataset_id: "test-dataset".to_owned(),
        input_key: "INPUT".to_owned(),
    }
}

fn pricing_response(max_charge: f64, charged_events: Value) -> Value {
    pricing_response_with_options(json!({ "maxTotalChargeUsd": max_charge }), charged_events)
}

fn pricing_response_with_options(options: Value, charged_events: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": { "actorChargeEvents": {
                    "apify-default-dataset-item": { "eventPriceUsd": 0.0002 },
                    "apify-actor-start": { "eventPriceUsd": 0.00005 }
                }}
            },
            "options": options,
            "chargedEventCounts": charged_events
        }
    })
}

fn mock_apify(input: Value, max_charge: f64, charged_events: Value) -> MockServer {
    MockServer::start(move |request| {
        if request
            .headers
            .get("authorization")
            .map(String::as_str)
            != Some("Bearer apify-test-token")
        {
            return MockResponse::json(401, json!({ "message": "Unauthorized" }));
        }

        if request.path.ends_with("/records/INPUT") {
            return MockResponse::json(200, input.clone());
        }
        if request.path.ends_with("/actor-runs/test-run") {
            return MockResponse::json(200, pricing_response(max_charge, charged_events.clone()));
        }
        MockResponse::json(200, json!({ "ok": true }))
    })
}

fn api_base(base_url: &Url) -> Url {
    let mut url = base_url.clone();
    url.path_segments_mut().unwrap().push("api");
    url
}

#[test]
fn get_usernames_normalizes_legacy_input_and_deduplicates_case_insensitively() {
    assert_eq!(
        get_usernames(&json!({
            "usernames": ["@NatGeo", "instagram", "natgeo"],
            "username": "@legacy"
        }))
        .unwrap(),
        ["NatGeo", "instagram", "legacy"]
    );
}

#[test]
fn get_usernames_rejects_empty_invalid_non_string_and_oversized_batches() {
    assert!(get_usernames(&json!({}))
        .unwrap_err()
        .to_string()
        .contains("At least one"));
    assert!(get_usernames(&json!({ "usernames": ["invalid username"] }))
        .unwrap_err()
        .to_string()
        .contains("Invalid Instagram username"));
    assert!(get_usernames(&json!({ "usernames": [3] }))
        .unwrap_err()
        .to_string()
        .contains("must be a string"));
    let usernames = (0..=MAX_USERNAMES)
        .map(|index| format!("user{index}"))
        .collect::<Vec<_>>();
    assert!(get_usernames(&json!({ "usernames": usernames }))
        .unwrap_err()
        .to_string()
        .contains("maximum of 100"));
}

#[test]
fn flatten_profile_merges_user_fields_and_falls_through_null_user_values() {
    assert_eq!(
        flatten_profile(&json!({
            "success": true,
            "user": null,
            "data": { "user": { "username": "natgeo", "follower_count": 5 } }
        })),
        json!({
            "success": true,
            "user": null,
            "data": { "user": { "username": "natgeo", "follower_count": 5 } },
            "username": "natgeo",
            "follower_count": 5
        })
    );
}

#[test]
fn authentication_detection_matches_status_code_and_message_forms() {
    assert!(is_authentication_failure(
        401,
        &json!({ "message": "bad key" })
    ));
    assert!(is_authentication_failure(
        200,
        &json!({ "code": "FORBIDDEN_ACCESS" })
    ));
    assert!(is_authentication_failure(
        200,
        &json!({ "error": "Unauthorized" })
    ));
    assert!(!is_authentication_failure(
        500,
        &json!({ "message": "upstream down" })
    ));
}

#[test]
fn apify_retry_policy_retries_only_get_and_put_transient_statuses() {
    assert_eq!(
        apify_retry_delay("GET", StatusCode::INTERNAL_SERVER_ERROR, 0),
        Some(Duration::from_secs(1))
    );
    assert_eq!(
        apify_retry_delay("PUT", StatusCode::TOO_MANY_REQUESTS, 1),
        Some(Duration::from_secs(2))
    );
    assert_eq!(apify_retry_delay("GET", StatusCode::BAD_REQUEST, 0), None);
    assert_eq!(
        apify_retry_delay("POST", StatusCode::INTERNAL_SERVER_ERROR, 0),
        None
    );
    assert_eq!(
        apify_retry_delay("GET", StatusCode::INTERNAL_SERVER_ERROR, APIFY_MAX_RETRIES),
        None
    );
    assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(90));
}

#[test]
fn dataset_budget_includes_previously_charged_events() {
    let run = pricing_response(
        0.001,
        json!({ "apify-default-dataset-item": 1, "apify-actor-start": 1 }),
    );
    let budget = DatasetBudget::from_run(&run).unwrap();
    assert_eq!(budget.affordable_items(5), 3);
}

#[test]
fn dataset_budget_treats_zero_omitted_and_null_limits_as_unbounded() {
    let charges = json!({
        "apify-default-dataset-item": 3,
        "apify-actor-start": 1
    });
    let options = [
        json!({ "maxTotalChargeUsd": 0 }),
        json!({}),
        json!({ "maxTotalChargeUsd": null }),
    ];

    for options in options {
        let run = pricing_response_with_options(options, charges.clone());
        let budget = DatasetBudget::from_run(&run).unwrap();
        assert_eq!(budget.affordable_items(5), 5);
    }
}

#[test]
fn dataset_budget_applies_positive_limit_after_dataset_and_custom_charges() {
    let mut run = pricing_response_with_options(
        json!({ "maxTotalChargeUsd": 0.001 }),
        json!({
            "apify-default-dataset-item": 1,
            "apify-actor-start": 1,
            "custom-lookup": 1
        }),
    );
    run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]["custom-lookup"] =
        json!({ "eventPriceUsd": 0.0003 });

    let budget = DatasetBudget::from_run(&run).unwrap();
    assert_eq!(budget.affordable_items(5), 2);
}

#[test]
fn dataset_budget_keeps_rejecting_non_ppe_runs() {
    let mut run = pricing_response(1.0, json!({}));
    run["data"]["pricingInfo"]["pricingModel"] = json!("PRICE_PER_DATASET_ITEM");

    assert!(DatasetBudget::from_run(&run).is_err());
}

#[test]
fn actor_metadata_keeps_prefill_and_minimal_memory_configuration() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    let actor: Value = serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();
    assert_eq!(
        schema["properties"]["usernames"]["prefill"],
        json!(["natgeo", "instagram"])
    );
    assert_eq!(schema["properties"]["username"]["prefill"], "natgeo");
    assert_eq!(actor["defaultMemoryMbytes"], 128);
    assert_eq!(actor["defaultRunOptions"]["timeoutSecs"], 120);
}

#[tokio::test]
async fn actor_preserves_auth_batch_output_and_failure_rows() {
    let input = json!({
        "usernames": ["@user0", "user1", "user2", "user3", "user4", "user5", "user6"]
    });
    let apify = mock_apify(input, 1.0, json!({}));
    let scrappa = MockServer::start(|request| {
        if request.path.contains("username=user1") {
            MockResponse::json(404, json!({ "message": "not found" }))
        } else {
            MockResponse::json(
                200,
                json!({ "success": true, "data": { "user": { "username": "profile", "follower_count": 5 } } }),
            )
        }
    });
    let config = config(apify.base_url.clone(), api_base(&scrappa.base_url));
    let summary = run_actor(&Client::new(), &config).await.unwrap();
    assert_eq!(
        summary,
        BatchSummary {
            requested: 7,
            succeeded: 6,
            failed: 1,
            saved: 7
        }
    );

    let apify_requests = apify.requests();
    let input_request = apify_requests
        .iter()
        .find(|request| request.path.ends_with("/records/INPUT"))
        .unwrap();
    assert_eq!(input_request.method, "GET");
    assert_eq!(
        input_request
            .headers
            .get("authorization")
            .map(String::as_str),
        Some("Bearer apify-test-token")
    );
    let dataset_writes = apify_requests
        .iter()
        .filter(|request| request.method == "POST" && request.path.ends_with("/items"))
        .collect::<Vec<_>>();
    assert_eq!(dataset_writes.len(), 2);
    let first_batch: Vec<Value> = serde_json::from_str(&dataset_writes[0].body).unwrap();
    let second_batch: Vec<Value> = serde_json::from_str(&dataset_writes[1].body).unwrap();
    assert_eq!((first_batch.len(), second_batch.len()), (5, 2));
    let saved_items = first_batch
        .into_iter()
        .chain(second_batch)
        .collect::<Vec<_>>();
    assert_eq!(saved_items.len(), 7);
    assert_eq!(saved_items[0]["input_username"], "user0");
    assert_eq!(saved_items[1]["success"], false);
    assert_eq!(saved_items[1]["username"], "user1");
    assert_eq!(
        saved_items[1]["error"],
        "Scrappa API returned HTTP 404: not found"
    );
    assert_eq!(saved_items[2]["username"], "profile");
    assert_eq!(saved_items[2]["follower_count"], 5);
    assert!(apify_requests
        .iter()
        .all(|request| !(request.method == "PUT" && request.path.contains("/records/OUTPUT"))));

    let scrappa_requests = scrappa.requests();
    assert_eq!(scrappa_requests.len(), 7);
    for request in scrappa_requests {
        assert_eq!(request.method, "GET");
        assert!(request.path.starts_with("/api/instagram/user?username="));
        assert_eq!(
            request.headers.get("x-api-key").map(String::as_str),
            Some("scrappa-test-key")
        );
        assert_eq!(
            request.headers.get("accept").map(String::as_str),
            Some("application/json")
        );
    }
}

#[tokio::test]
async fn actor_writes_only_dataset_items_that_fit_the_remaining_ppe_budget() {
    let apify = mock_apify(
        json!({ "usernames": ["one", "two", "three", "four", "five"] }),
        0.001,
        json!({ "apify-default-dataset-item": 1, "apify-actor-start": 1 }),
    );
    let scrappa = MockServer::start(|_| {
        MockResponse::json(200, json!({ "user": { "username": "profile" } }))
    });
    let config = config(apify.base_url.clone(), api_base(&scrappa.base_url));

    let summary = run_actor(&Client::new(), &config).await.unwrap();
    assert_eq!(summary.saved, 3);
    let writes = apify
        .requests()
        .into_iter()
        .filter(|request| request.method == "POST" && request.path.ends_with("/items"))
        .collect::<Vec<_>>();
    assert_eq!(writes.len(), 1);
    let saved: Vec<Value> = serde_json::from_str(&writes[0].body).unwrap();
    assert_eq!(saved.len(), 3);
    assert_eq!(saved[0]["input_username"], "one");
    assert_eq!(saved[2]["input_username"], "three");
}

#[tokio::test]
async fn authentication_failure_fails_without_writing_the_current_batch() {
    let apify = mock_apify(json!({ "usernames": ["good", "denied"] }), 1.0, json!({}));
    let scrappa = MockServer::start(|request| {
        if request.path.contains("username=denied") {
            MockResponse::json(401, json!({ "message": "Invalid API key" }))
        } else {
            MockResponse::json(200, json!({ "user": { "username": "good" } }))
        }
    });
    let config = config(apify.base_url.clone(), api_base(&scrappa.base_url));
    let error = run_actor(&Client::new(), &config).await.unwrap_err();

    assert!(error
        .to_string()
        .contains("Scrappa API authentication failed: Invalid API key"));
    assert!(apify
        .requests()
        .iter()
        .all(|request| !(request.method == "POST" && request.path.ends_with("/items"))));
}
