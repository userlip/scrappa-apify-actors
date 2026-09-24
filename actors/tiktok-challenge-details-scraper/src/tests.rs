use crate::{
    apify::{ApifyClient, PpeBudget},
    batch::{result_error, run_batch},
    challenge::{
        build_requests, challenge_url, extract_challenge_detail, normalize_challenge_detail,
        ChallengeRequest, RequestType,
    },
    config::{ActorConfig, CHALLENGE_DETAIL_CHARGE_EVENT, DEFAULT_DATASET_ITEM_EVENT, INPUT_KEY},
    scrappa::{challenge_error, scrappa_error_message, ScrappaClient},
};
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use url::Url;

fn request(request_type: RequestType, value: &str) -> ChallengeRequest {
    ChallengeRequest {
        request_type,
        value: value.to_owned(),
    }
}

#[test]
fn input_keeps_batch_prefill_and_legacy_fields() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    assert_eq!(
        schema["properties"]["challenge_names"]["prefill"],
        json!(["booktok", "fitness"])
    );
    assert_eq!(
        schema["properties"]["challenge_ids"]["editor"],
        "stringList"
    );
    assert_eq!(
        schema["properties"]["challenge_name"]["editor"],
        "textfield"
    );
    assert_eq!(schema["properties"]["challenge_id"]["editor"], "textfield");
}

#[test]
fn normalizes_and_deduplicates_batch_and_legacy_inputs() {
    let mut warnings = Vec::new();
    let requests = build_requests(
        &json!({
            "challenge_names": [" #BookTok ", "booktok", "fitness"],
            "challenge_ids": ["1622962893630470", "1622962893630470"],
            "challenge_name": "Fitness",
            "challenge_id": 42
        }),
        &mut warnings,
    )
    .unwrap();
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(
        requests,
        vec![
            request(RequestType::ChallengeName, "BookTok"),
            request(RequestType::ChallengeName, "fitness"),
            request(RequestType::ChallengeId, "1622962893630470"),
            request(RequestType::ChallengeId, "42"),
        ]
    );
}

#[test]
fn omits_invalid_entries_and_enforces_the_combined_limit() {
    let mut warnings = Vec::new();
    let requests = build_requests(
        &json!({"challenge_names": ["booktok", "bad/name", 42]}),
        &mut warnings,
    )
    .unwrap();
    assert_eq!(
        requests,
        vec![request(RequestType::ChallengeName, "booktok")]
    );
    assert_eq!(warnings.len(), 2);
    assert!(warnings[0].contains("omitted"));
    assert!(build_requests(&json!({"challenge_ids": ["not-an-id"]}), &mut Vec::new()).is_err());
    assert!(build_requests(
            &json!({"challenge_names": (0..101).map(|index| format!("tag{index}")).collect::<Vec<_>>() }),
            &mut Vec::new()
        )
        .unwrap_err()
        .to_string()
        .contains("maximum of 100"));
}

#[test]
fn creates_the_exact_scrappa_lookup_route_and_query() {
    let base = Url::parse("https://scrappa.co/api").unwrap();
    let url = challenge_url(&base, &request(RequestType::ChallengeName, "book tok")).unwrap();
    assert_eq!(url.path(), "/api/tiktok/challenges/details");
    assert_eq!(
        url.query_pairs().collect::<Vec<_>>(),
        vec![("challenge_name".into(), "book tok".into())]
    );
    let id_url = challenge_url(&base, &request(RequestType::ChallengeId, "42")).unwrap();
    assert_eq!(
        id_url.query_pairs().collect::<Vec<_>>(),
        vec![("challenge_id".into(), "42".into())]
    );
}

#[test]
fn validates_api_errors_and_canonical_identity_before_saving() {
    let error = challenge_error(&json!({"code": 404, "msg": "Not found"})).unwrap();
    assert!(error.contains("code 404: Not found"));

    let name_request = request(RequestType::ChallengeName, "booktok");
    let mismatch = json!({"id": "1", "challenge_name": "unrelated"});
    assert!(result_error(mismatch.as_object().unwrap(), &name_request)
        .unwrap()
        .contains("unrelated"));

    let id_request = request(RequestType::ChallengeId, "1");
    let mismatch = json!({"id": "2", "challenge_name": "booktok"});
    assert!(result_error(mismatch.as_object().unwrap(), &id_request)
        .unwrap()
        .contains("\"2\""));
    assert!(result_error(&Map::new(), &id_request)
        .unwrap()
        .contains("without a canonical challenge ID"));
}

#[test]
fn normalizes_records_while_preserving_raw_fields_and_null_metrics() {
    let challenge: Map<String, Value> = serde_json::from_value(json!({
        "id": "1622962893630470", "cha_name": "BookTok", "desc": " Books ",
        "stats": {"user_count": 5, "view_count": 10}, "video_count": 3,
        "cover_url": "https://example.test/cover.jpg", "is_commerce": false
    }))
    .unwrap();
    let item = normalize_challenge_detail(
        &challenge,
        &request(RequestType::ChallengeName, "booktok"),
        "2026-07-11T00:00:00.000Z".to_owned(),
    );
    assert_eq!(item["challenge_id"], "1622962893630470");
    assert_eq!(item["challenge_name"], "BookTok");
    assert_eq!(item["description"], "Books");
    assert_eq!(item["user_count"], 5);
    assert_eq!(item["view_count"], 10);
    assert_eq!(item["video_count"], 3);
    assert_eq!(item["cover"], "https://example.test/cover.jpg");
    assert_eq!(item["request_challenge_name"], "booktok");
    assert_eq!(item["request_challenge_id"], Value::Null);
    assert_eq!(item["is_commerce"], false);
    assert_eq!(item["retrieved_at"], "2026-07-11T00:00:00.000Z");

    let sparse = normalize_challenge_detail(
        &json!({"challenge_id": "1", "challenge_name": "one"})
            .as_object()
            .unwrap()
            .clone(),
        &request(RequestType::ChallengeId, "1"),
        "now".to_owned(),
    );
    assert_eq!(sparse["user_count"], Value::Null);
    assert_eq!(sparse["view_count"], Value::Null);
    assert_eq!(sparse["video_count"], Value::Null);
    assert!(extract_challenge_detail(&json!({"data": {"challenge": {"id": "1"}}})).is_some());
    assert!(extract_challenge_detail(&json!({ "data": null })).is_none());
}

fn ppe_run(max_total: f64, item_price: f64, event_price: f64, charged: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": item_price},
                "challenge-detail-result": {"eventPriceUsd": event_price}
            }}},
            "options": {"maxTotalChargeUsd": max_total},
            "chargedEventCounts": charged
        }
    })
}

#[test]
fn ppe_budget_counts_prior_events_caps_lookups_and_charges_each_saved_row() {
    let mut budget = PpeBudget::from_run(&ppe_run(
        0.001,
        0.0001,
        0.00025,
        json!({"challenge-detail-result": 2}),
    ))
    .unwrap();
    assert_eq!(budget.event_capacity(CHALLENGE_DETAIL_CHARGE_EVENT), 2);
    assert_eq!(budget.result_capacity(), 1);
    assert!(budget.can_push_dataset_item());

    budget.record_successful_charge(DEFAULT_DATASET_ITEM_EVENT, 1);
    budget.record_successful_charge(CHALLENGE_DETAIL_CHARGE_EVENT, 1);
    assert_eq!(budget.event_capacity(CHALLENGE_DETAIL_CHARGE_EVENT), 0);
    assert_eq!(budget.result_capacity(), 0);
    assert_eq!(
        budget.charged_event_counts[CHALLENGE_DETAIL_CHARGE_EVENT],
        3
    );
    assert_eq!(budget.charged_event_counts[DEFAULT_DATASET_ITEM_EVENT], 1);
}

#[test]
fn ppe_preflight_requires_room_for_custom_and_automatic_dataset_events() {
    let budget = PpeBudget::from_run(&ppe_run(0.0003, 0.0001, 0.00025, json!({}))).unwrap();
    assert_eq!(budget.event_capacity(DEFAULT_DATASET_ITEM_EVENT), 3);
    assert_eq!(budget.event_capacity(CHALLENGE_DETAIL_CHARGE_EVENT), 1);
    assert_eq!(budget.result_capacity(), 0);
    assert!(!budget.can_push_dataset_item());
}

#[test]
fn non_ppe_and_unconfigured_events_keep_dataset_output_unlimited() {
    let non_ppe =
        PpeBudget::from_run(&json!({"data": {"pricingInfo": {"pricingModel": "PAY_PER_RESULT"}}}))
            .unwrap();
    assert!(!non_ppe.is_pay_per_event);
    assert_eq!(
        non_ppe.event_capacity(CHALLENGE_DETAIL_CHARGE_EVENT),
        usize::MAX
    );
    assert!(non_ppe.can_push_dataset_item());

    let mut unconfigured = PpeBudget::from_run(&json!({
            "data": {"pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {}}},
            "options": {"maxTotalChargeUsd": 0.01}, "chargedEventCounts": {}}
        }))
        .unwrap();
    assert_eq!(
        unconfigured.event_capacity(CHALLENGE_DETAIL_CHARGE_EVENT),
        usize::MAX
    );
    assert_eq!(unconfigured.result_capacity(), usize::MAX);
    assert!(unconfigured.can_push_dataset_item());
    unconfigured.record_successful_charge(CHALLENGE_DETAIL_CHARGE_EVENT, 1);
    assert!(!unconfigured
        .charged_event_counts
        .contains_key(CHALLENGE_DETAIL_CHARGE_EVENT));
}

#[test]
fn scrappa_error_body_matches_message_and_validation_format() {
    assert_eq!(
        scrappa_error_message(
            r#"{"message":"Invalid input","errors":{"challenge_name":["required"]}}"#,
            "Bad Request",
            400
        ),
        "Invalid input - challenge_name: required"
    );
    assert_eq!(
        scrappa_error_message(" upstream  unavailable ", "Bad Gateway", 502),
        "upstream unavailable"
    );
    assert_eq!(scrappa_error_message("", "Bad Gateway", 502), "Bad Gateway");
}

struct MockRequest {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: String,
}

struct MockServer {
    base_url: Url,
    requests: Receiver<MockRequest>,
    thread: JoinHandle<()>,
}

impl MockServer {
    fn start<F>(request_count: usize, handler: F) -> Self
    where
        F: Fn(&MockRequest) -> (u16, String) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, requests) = mpsc::channel();
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            for _ in 0..request_count {
                let (mut stream, _) = loop {
                    match listener.accept() {
                        Ok(connection) => break connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(
                                Instant::now() < deadline,
                                "mock server timed out waiting for requests"
                            );
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(error) => panic!("mock server accept failed: {error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let request = read_mock_request(&mut stream);
                let (status, body) = handler(&request);
                sender.send(request).unwrap();
                let reason = match status {
                    200 => "OK",
                    201 => "Created",
                    204 => "No Content",
                    400 => "Bad Request",
                    404 => "Not Found",
                    500 => "Internal Server Error",
                    503 => "Service Unavailable",
                    _ => "Mock Response",
                };
                write!(
                        stream,
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();
            }
        });
        Self {
            base_url: Url::parse(&format!("http://{address}")).unwrap(),
            requests,
            thread,
        }
    }

    fn finish(self) -> Vec<MockRequest> {
        self.thread.join().unwrap();
        self.requests.into_iter().collect()
    }
}

fn read_mock_request(stream: &mut TcpStream) -> MockRequest {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let read = stream.read(&mut chunk).unwrap();
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&bytes[..header_end]);
        let content_length = headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        if bytes.len() >= header_end + 4 + content_length {
            break;
        }
    }

    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap();
    let header_text = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = header_text.lines();
    let mut request_line = lines.next().unwrap().split_whitespace();
    let method = request_line.next().unwrap().to_owned();
    let path = request_line.next().unwrap().to_owned();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    MockRequest {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&bytes[header_end + 4..]).into_owned(),
    }
}

fn test_config(base_url: Url) -> ActorConfig {
    let mut scrappa_api_base_url = base_url.clone();
    scrappa_api_base_url.set_path("/api");
    ActorConfig {
        apify_api_base_url: base_url,
        scrappa_api_base_url,
        key_value_store_id: "test-store".to_owned(),
        input_key: INPUT_KEY.to_owned(),
        dataset_id: "test-dataset".to_owned(),
        actor_run_id: "test-run".to_owned(),
        apify_token: "test-apify-token".to_owned(),
        scrappa_api_key: "test-scrappa-key".to_owned(),
    }
}

#[tokio::test]
async fn batch_continues_after_lookup_errors_and_charges_only_a_saved_unique_result() {
    let server = MockServer::start(5, |request| {
        match request.path.as_str() {
            path if path.contains("challenge_name=booktok") || path.contains("challenge_id=1") => (
                200,
                r#"{"code":0,"data":{"challenge":{"id":"1","challenge_name":"BookTok","desc":"Books"}}}"#.to_owned(),
            ),
            path if path.contains("challenge_name=missing") => (
                200,
                r#"{"code":404,"msg":"Not found"}"#.to_owned(),
            ),
            "/v2/datasets/test-dataset/items" => (201, "{}".to_owned()),
            "/v2/actor-runs/test-run/charge" => (201, "{}".to_owned()),
            _ => (404, "{}".to_owned()),
        }
    });
    let config = test_config(server.base_url.clone());
    let apify = ApifyClient::new(&config).unwrap();
    let scrappa = ScrappaClient::new(&config).unwrap();
    let mut budget = PpeBudget::from_run(&ppe_run(0.001, 0.0, 0.00025, json!({}))).unwrap();
    let requests = vec![
        request(RequestType::ChallengeName, "booktok"),
        request(RequestType::ChallengeName, "missing"),
        request(RequestType::ChallengeId, "1"),
    ];

    let summary = run_batch(&requests, &scrappa, &apify, &config, &mut budget)
        .await
        .unwrap();
    assert_eq!(summary["requested"], 3);
    assert_eq!(summary["attempted"], 3);
    assert_eq!(summary["saved"], 1);
    assert_eq!(summary["failed"], 1);
    assert_eq!(summary["outcomes"][0]["status"], "saved");
    assert_eq!(summary["outcomes"][1]["status"], "failed");
    assert_eq!(summary["outcomes"][2]["status"], "duplicate");

    let captured = server.finish();
    let dataset_writes = captured
        .iter()
        .filter(|request| request.path == "/v2/datasets/test-dataset/items")
        .collect::<Vec<_>>();
    let charges = captured
        .iter()
        .filter(|request| request.path == "/v2/actor-runs/test-run/charge")
        .collect::<Vec<_>>();
    assert_eq!(dataset_writes.len(), 1);
    assert_eq!(charges.len(), 1);
    assert_eq!(
        serde_json::from_str::<Value>(&dataset_writes[0].body).unwrap()["request_challenge_name"],
        "booktok"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&charges[0].body).unwrap(),
        json!({"eventName": CHALLENGE_DETAIL_CHARGE_EVENT, "count": 1})
    );
    assert_eq!(
        charges[0].headers["idempotency-key"],
        "test-run:challenge_name:1"
    );
    for request in captured
        .iter()
        .filter(|request| request.path.starts_with("/api/"))
    {
        assert_eq!(request.headers["x-api-key"], "test-scrappa-key");
    }
    for request in captured
        .iter()
        .filter(|request| request.path.starts_with("/v2/"))
    {
        assert_eq!(request.headers["authorization"], "Bearer test-apify-token");
    }
}

#[tokio::test]
async fn unsuccessful_custom_charge_is_not_counted_as_billed() {
    let server = MockServer::start(3, |request| match request.path.as_str() {
        path if path.contains("challenge_name=booktok") => (
            200,
            r#"{"code":0,"data":{"challenge":{"id":"1","challenge_name":"BookTok"}}}"#.to_owned(),
        ),
        "/v2/datasets/test-dataset/items" => (201, "{}".to_owned()),
        "/v2/actor-runs/test-run/charge" => (400, r#"{"error":"rejected"}"#.to_owned()),
        _ => (404, "{}".to_owned()),
    });
    let config = test_config(server.base_url.clone());
    let apify = ApifyClient::new(&config).unwrap();
    let scrappa = ScrappaClient::new(&config).unwrap();
    let mut budget = PpeBudget::from_run(&ppe_run(0.001, 0.0001, 0.00025, json!({}))).unwrap();
    let requests = vec![
        request(RequestType::ChallengeName, "booktok"),
        request(RequestType::ChallengeName, "fitness"),
    ];

    let summary = run_batch(&requests, &scrappa, &apify, &config, &mut budget)
        .await
        .unwrap();
    assert_eq!(summary["attempted"], 1);
    assert_eq!(summary["saved"], 0);
    assert_eq!(summary["failed"], 1);
    assert_eq!(summary["outcomes"][0]["status"], "failed");
    assert_eq!(summary["outcomes"][1]["status"], "not_attempted");
    assert_eq!(budget.charged_event_counts[DEFAULT_DATASET_ITEM_EVENT], 1);
    assert!(!budget
        .charged_event_counts
        .contains_key(CHALLENGE_DETAIL_CHARGE_EVENT));

    let captured = server.finish();
    assert!(captured
        .iter()
        .any(|request| request.path == "/v2/datasets/test-dataset/items"));
    assert!(captured
        .iter()
        .any(|request| request.path == "/v2/actor-runs/test-run/charge"));
}

#[tokio::test]
async fn exhausted_result_budget_skips_every_remaining_lookup() {
    let server = MockServer::start(0, |_| panic!("an exhausted budget must not make requests"));
    let config = test_config(server.base_url.clone());
    let apify = ApifyClient::new(&config).unwrap();
    let scrappa = ScrappaClient::new(&config).unwrap();
    let mut budget = PpeBudget::from_run(&ppe_run(0.0003, 0.0001, 0.00025, json!({}))).unwrap();
    assert_eq!(budget.event_capacity(CHALLENGE_DETAIL_CHARGE_EVENT), 1);
    assert_eq!(budget.event_capacity(DEFAULT_DATASET_ITEM_EVENT), 3);
    assert_eq!(budget.result_capacity(), 0);
    let requests = vec![
        request(RequestType::ChallengeName, "booktok"),
        request(RequestType::ChallengeId, "1"),
    ];

    let summary = run_batch(&requests, &scrappa, &apify, &config, &mut budget)
        .await
        .unwrap();
    assert_eq!(summary["attempted"], 0);
    assert_eq!(summary["charge_limit_reached"], true);
    assert_eq!(
        summary["status_message"],
        "Charge limit reached before fetching another TikTok challenge detail."
    );
    assert_eq!(summary["outcomes"][0]["status"], "not_attempted");
    assert_eq!(summary["outcomes"][1]["status"], "not_attempted");
    assert!(server.finish().is_empty());
}

#[tokio::test]
async fn apify_requests_retry_transient_server_errors() {
    let attempt_count = Arc::new(AtomicUsize::new(0));
    let attempts = Arc::clone(&attempt_count);
    let server = MockServer::start(2, move |request| {
        if request.method == "GET"
            && request.path == "/v2/key-value-stores/test-store/records/INPUT"
        {
            if request.headers.get("authorization").map(String::as_str)
                != Some("Bearer test-apify-token")
            {
                return (400, "{}".to_owned());
            }
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                return (503, r#"{"error":"temporary"}"#.to_owned());
            }
            return (200, r#"{"challenge_names":["booktok"]}"#.to_owned());
        }
        (404, "{}".to_owned())
    });
    let config = test_config(server.base_url.clone());
    let apify = ApifyClient::new(&config).unwrap();
    let input = apify.get_input(&config).await.unwrap().unwrap();
    assert_eq!(input["challenge_names"][0], "booktok");
    let captured = server.finish();
    assert_eq!(captured.len(), 2);
    assert!(captured.iter().all(|request| request.method == "GET"));
}
