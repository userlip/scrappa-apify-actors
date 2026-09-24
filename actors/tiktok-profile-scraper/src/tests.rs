use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

struct MockResponse {
    status: u16,
    body: String,
}

struct MockServer {
    base_url: Url,
    requests: mpsc::Receiver<String>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, requests) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            for response in responses {
                let (mut stream, _) = loop {
                    if stopped.load(Ordering::Relaxed) || Instant::now() >= deadline {
                        return;
                    }
                    match listener.accept() {
                        Ok(connection) => break connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => return,
                    }
                };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let request = read_request(&mut stream).unwrap_or_default();
                let _ = request_sender.send(request);
                let reason = match response.status {
                    200 => "OK",
                    201 => "Created",
                    400 => "Bad Request",
                    401 => "Unauthorized",
                    404 => "Not Found",
                    429 => "Too Many Requests",
                    500 => "Internal Server Error",
                    503 => "Service Unavailable",
                    _ => "Mock Response",
                };
                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.status,
                    reason,
                    response.body.len(),
                    response.body,
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        Self {
            base_url: Url::parse(&format!("http://{address}/")).unwrap(),
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn requests(&self) -> Vec<String> {
        self.requests.try_iter().collect()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn mock_response(status: u16, body: impl Into<String>) -> MockResponse {
    MockResponse {
        status,
        body: body.into(),
    }
}

fn config(server: &MockServer) -> ActorConfig {
    ActorConfig {
        apify_api_base_url: server.base_url.clone(),
        scrappa_api_base_url: server.base_url.join("api/").unwrap(),
        default_key_value_store_id: "test-store".to_owned(),
        default_dataset_id: "test-dataset".to_owned(),
        actor_run_id: "test-run".to_owned(),
        input_key: "INPUT".to_owned(),
        apify_token: "test-token-not-a-real-credential".to_owned(),
        scrappa_api_key: "test-scrappa-key".to_owned(),
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().build().unwrap()
}

fn pricing_run(max_total_charge_usd: Option<f64>, charged_counts: Value) -> Value {
    pricing_run_with_limit(
        max_total_charge_usd.map(|limit| json!(limit)),
        charged_counts,
    )
}

fn pricing_run_with_limit(max_total_charge_usd: Option<Value>, charged_counts: Value) -> Value {
    let mut options = json!({});
    if let Some(max_total_charge_usd) = max_total_charge_usd {
        options["maxTotalChargeUsd"] = max_total_charge_usd;
    }
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": { "actorChargeEvents": {
                    "apify-default-dataset-item": { "eventPriceUsd": 0.0003 },
                    "apify-actor-start": { "eventPriceUsd": 0.00005 },
                    "custom-event": { "eventPriceUsd": 0.0001 }
                }}
            },
            "options": options,
            "chargedEventCounts": charged_counts
        }
    })
}

fn standard_responses(input: &str, api_response: &str, run: &Value) -> Vec<MockResponse> {
    vec![
        mock_response(200, input),
        mock_response(200, api_response),
        mock_response(200, run.to_string()),
        mock_response(201, "{}"),
        mock_response(201, "{}"),
    ]
}

fn request_parts(request: &str) -> (&str, &str, &str) {
    let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
    let mut parts = headers
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    (
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
        body,
    )
}

fn request_header<'a>(request: &'a str, header_name: &str) -> Option<&'a str> {
    request
        .split_once("\r\n\r\n")
        .unwrap_or((request, ""))
        .0
        .lines()
        .skip(1)
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(header_name).then(|| value.trim())
        })
}

#[test]
fn accepts_username_url_numeric_id_and_legacy_fields_with_original_precedence() {
    assert_eq!(
        build_tiktok_profile_params(
            &json!({"profile": " https://www.tiktok.com/@tiktok?lang=en "})
        )
        .unwrap(),
        TikTokProfileParams {
            unique_id: Some("@tiktok".to_owned()),
            user_id: None,
        }
    );
    assert_eq!(
        build_tiktok_profile_params(&json!({"profile": "107955"}))
            .unwrap()
            .user_id,
        Some("107955".to_owned())
    );
    assert_eq!(
        build_tiktok_profile_params(&json!({"profile": "@107955"}))
            .unwrap()
            .unique_id,
        Some("@107955".to_owned())
    );
    assert_eq!(
        build_tiktok_profile_params(&json!({"unique_id": "legacy", "user_id": "bad"}))
            .unwrap()
            .unique_id,
        Some("@legacy".to_owned())
    );
    assert_eq!(
        build_tiktok_profile_params(&json!({"profile": "primary", "unique_id": "legacy"}))
            .unwrap()
            .unique_id,
        Some("@primary".to_owned())
    );
}

#[test]
fn validates_lookup_values_and_matches_the_input_prefill() {
    for invalid in ["tik tok", "tik-tok", "a", "1".repeat(31).as_str()] {
        assert!(build_tiktok_profile_params(&json!({ "profile": invalid })).is_err());
    }
    assert!(
        build_tiktok_profile_params(&json!({"profile": "http://www.tiktok.com/@tiktok"})).is_err()
    );
    assert!(
        build_tiktok_profile_params(&json!({"profile": "https://example.com/@tiktok"})).is_err()
    );
    assert!(
        build_tiktok_profile_params(&json!({"profile": "https://www.tiktok.com/tag/example"}))
            .is_err()
    );
    assert!(build_tiktok_profile_params(&json!({"user_id": "1".repeat(31)})).is_err());
    assert!(build_tiktok_profile_params(&json!({"unique_id": " ", "user_id": ""})).is_err());

    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    assert_eq!(schema["properties"]["profile"]["prefill"], "@tiktok");
    assert_eq!(schema["required"], json!(["profile"]));
}

#[test]
fn normalizes_nested_profile_fields_and_preserves_existing_values() {
    let profile = json!({
        "user": {
            "id": 107955,
            "uniqueId": "tiktok",
            "nickname": "TikTok",
            "avatarThumb": "thumb.webp",
            "avatarMedium": "medium.webp",
            "avatarLarger": "large.webp",
            "signature": "One TikTok can make a big impact",
            "verified": true,
            "privateAccount": false,
            "region": "US",
            "language": "en"
        },
        "stats": {
            "followingCount": 3,
            "followerCount": 93950516,
            "heartCount": 457861218,
            "videoCount": 1509,
            "diggCount": 0
        }
    });
    let normalized = normalize_tiktok_profile_record(&profile).unwrap();
    assert_eq!(normalized["user_id"], "107955");
    assert_eq!(normalized["unique_id"], "@tiktok");
    assert_eq!(normalized["nickname"], "TikTok");
    assert_eq!(normalized["avatar"], "large.webp");
    assert_eq!(normalized["signature"], "One TikTok can make a big impact");
    assert_eq!(normalized["verified"], true);
    assert_eq!(normalized["private_account"], false);
    assert_eq!(normalized["region"], "US");
    assert_eq!(normalized["language"], "en");
    assert_eq!(normalized["following_count"], 3);
    assert_eq!(normalized["follower_count"], 93950516);
    assert_eq!(normalized["heart_count"], 457861218);
    assert_eq!(normalized["video_count"], 1509);
    assert_eq!(normalized["digg_count"], 0);
    assert_eq!(normalized["user"], profile["user"]);

    let existing = normalize_tiktok_profile_record(&json!({
        "user_id": "existing-id",
        "unique_id": "@existing",
        "nickname": "Existing",
        "avatar": "existing.webp",
        "follower_count": 9,
        "user": { "id": "new-id", "uniqueId": "new", "nickname": "New", "avatarLarger": "new.webp" },
        "stats": { "followerCount": 100 }
    }))
    .unwrap();
    assert_eq!(existing["user_id"], "existing-id");
    assert_eq!(existing["unique_id"], "@existing");
    assert_eq!(existing["nickname"], "Existing");
    assert_eq!(existing["avatar"], "existing.webp");
    assert_eq!(existing["follower_count"], 9);
}

#[test]
fn falls_back_to_nested_unique_id_when_top_level_id_is_empty() {
    let normalized = normalize_tiktok_profile_record(&json!({
        "unique_id": "",
        "user": { "uniqueId": "tiktok" }
    }))
    .unwrap();

    assert_eq!(normalized["unique_id"], "@tiktok");

    let existing_top_level = normalize_tiktok_profile_record(&json!({
        "unique_id": "primary",
        "user": { "uniqueId": 123 }
    }))
    .unwrap();

    assert_eq!(existing_top_level["unique_id"], "@primary");
}

#[test]
fn extracts_first_profile_only_and_preserves_no_data() {
    assert!(extract_profile(None).is_none());
    assert!(extract_profile(Some(&Value::Null)).is_none());
    assert!(extract_profile(Some(&json!(false))).is_none());
    assert!(extract_profile(Some(&json!(0))).is_none());
    assert!(extract_profile(Some(&json!(""))).is_none());
    assert!(extract_profile(Some(&json!([]))).is_none());
    assert_eq!(
        extract_profile(Some(&json!([null, {"user_id": "2"}]))),
        None
    );
    assert_eq!(
        extract_profile(Some(&json!([{"user_id": "1"}, {"user_id": "2"}]))).unwrap()["user_id"],
        "1"
    );
}

#[test]
fn enforces_positive_ppe_spending_limits() {
    let at_limit = pricing_run(Some(0.00035), json!({"apify-actor-start": 1}));
    let below_limit = pricing_run(Some(0.00034), json!({"apify-actor-start": 1}));
    assert_eq!(affordable_dataset_items(&at_limit, 1).unwrap(), 1);
    assert_eq!(affordable_dataset_items(&below_limit, 1).unwrap(), 0);
}

#[test]
fn leaves_non_ppe_runs_unmetered_and_handles_free_items() {
    assert_eq!(
        affordable_dataset_items(
            &json!({"data":{"pricingInfo":{"pricingModel":"PRICE_PER_RESULT"}}}),
            1
        )
        .unwrap(),
        1
    );

    let mut free_run = pricing_run(None, json!({}));
    free_run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"][DATASET_ITEM_EVENT]
        ["eventPriceUsd"] = json!(0.0);
    assert_eq!(affordable_dataset_items(&free_run, 1).unwrap(), 1);

    let missing_counts = pricing_run(Some(1.0), Value::Null);
    assert!(affordable_dataset_items(&missing_counts, 1).is_err());
}

#[test]
fn treats_zero_missing_and_null_ppe_limits_as_unlimited() {
    let charged_counts = json!({"apify-actor-start": 1});
    let cases = [
        ("zero", Some(json!(0.0))),
        ("missing", None),
        ("null", Some(Value::Null)),
    ];
    let outcomes = cases.map(|(name, limit)| {
        (
            name,
            affordable_dataset_items(&pricing_run_with_limit(limit, charged_counts.clone()), 1)
                .map_err(|error| error.to_string()),
        )
    });

    assert_eq!(
        outcomes,
        [("zero", Ok(1)), ("missing", Ok(1)), ("null", Ok(1))]
    );
}

#[test]
fn counts_prior_dataset_and_custom_event_charges_toward_the_ppe_limit() {
    let run = pricing_run(
        Some(0.00065),
        json!({
            "apify-default-dataset-item": 1,
            "apify-actor-start": 1,
            "custom-event": 1
        }),
    );

    assert_eq!(affordable_dataset_items(&run, 1).unwrap(), 0);
}

#[test]
fn keeps_apify_retries_bounded_to_transient_failures_and_skips_dataset_timeouts() {
    assert!(should_retry_status(StatusCode::TOO_MANY_REQUESTS));
    assert!(should_retry_status(StatusCode::SERVICE_UNAVAILABLE));
    assert!(!should_retry_status(StatusCode::BAD_REQUEST));
    assert_eq!(apify_retry_delay(0), Duration::from_millis(500));
    assert_eq!(apify_retry_delay(1), Duration::from_secs(1));
    assert_eq!(apify_retry_delay(7), APIFY_MAX_RETRY_DELAY);
    assert_eq!(SCRAPPA_REQUEST_TIMEOUT, Duration::from_secs(60));
    assert_eq!(APIFY_MAX_RETRIES, 8);
}

#[test]
fn formats_scrappa_errors_like_the_typescript_client() {
    assert_eq!(
        scrappa_error_message(
            StatusCode::UNPROCESSABLE_ENTITY,
            r#"{"message":"Invalid input","errors":{"unique_id":["required","invalid"]}}"#
        ),
        "Invalid input - unique_id: required, invalid"
    );
    assert_eq!(
        scrappa_error_message(StatusCode::SERVICE_UNAVAILABLE, "upstream   unavailable"),
        "upstream unavailable"
    );
    assert_eq!(
        scrappa_error_message(StatusCode::NOT_FOUND, ""),
        "Not Found"
    );
}

#[tokio::test]
async fn publishes_one_normalized_item_and_the_full_response_with_expected_auth() {
    let api_response = json!({
        "code": 0,
        "processed_time": 1.25,
        "data": {
            "user": { "id": "107955", "uniqueId": "tiktok", "nickname": "TikTok" },
            "stats": { "followerCount": 93950516 }
        }
    });
    let server = MockServer::start(standard_responses(
        r#"{"profile":"https://www.tiktok.com/@tiktok?lang=en"}"#,
        &api_response.to_string(),
        &json!({"data":{"pricingInfo":{"pricingModel":"PRICE_PER_RESULT"}}}),
    ));

    run_actor(&config(&server), client()).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    assert!(request_parts(&requests[0])
        .1
        .starts_with("/v2/key-value-stores/test-store/records/INPUT"));
    assert_eq!(
        request_header(&requests[0], "authorization"),
        Some("Bearer test-token-not-a-real-credential")
    );

    let (method, path, _) = request_parts(&requests[1]);
    assert_eq!(method, "GET");
    let upstream_url = Url::parse(&format!("http://localhost{path}")).unwrap();
    assert_eq!(upstream_url.path(), "/api/tiktok/user/profile");
    assert_eq!(
        upstream_url.query_pairs().collect::<Vec<_>>(),
        vec![("unique_id".into(), "@tiktok".into())]
    );
    assert_eq!(
        request_header(&requests[1], "x-api-key"),
        Some("test-scrappa-key")
    );
    assert_eq!(
        request_header(&requests[1], "accept"),
        Some("application/json")
    );

    let (method, path, body) = request_parts(&requests[3]);
    assert_eq!(method, "POST");
    assert_eq!(path, "/v2/datasets/test-dataset/items");
    assert_eq!(
        request_header(&requests[3], "authorization"),
        Some("Bearer test-token-not-a-real-credential")
    );
    let item: Value = serde_json::from_str(body).unwrap();
    assert_eq!(item["unique_id"], "@tiktok");
    assert_eq!(item["user_id"], "107955");
    assert_eq!(item["follower_count"], 93950516);
    assert_eq!(item["lookup_unique_id"], "@tiktok");
    assert_eq!(item["lookup_user_id"], Value::Null);

    let (method, path, body) = request_parts(&requests[4]);
    assert_eq!(method, "PUT");
    assert_eq!(path, "/v2/key-value-stores/test-store/records/OUTPUT");
    assert_eq!(serde_json::from_str::<Value>(body).unwrap(), api_response);
}

#[tokio::test]
async fn ppe_writes_are_charged_once_and_respect_already_charged_actor_start() {
    let api_response = json!({"code": 0, "data": {"user_id": "107955"}});
    let run = pricing_run(Some(0.00035), json!({"apify-actor-start": 1}));
    let server = MockServer::start(standard_responses(
        r#"{"profile":"107955"}"#,
        &api_response.to_string(),
        &run,
    ));

    run_actor(&config(&server), client()).await.unwrap();
    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
    assert_eq!(request_parts(&requests[3]).0, "POST");
    assert_eq!(request_parts(&requests[4]).0, "PUT");
}

#[tokio::test]
async fn skips_a_profile_dataset_write_when_the_ppe_budget_is_exhausted_but_saves_output() {
    let api_response = json!({"code": 0, "data": {"user_id": "107955"}});
    let run = pricing_run(Some(0.00034), json!({"apify-actor-start": 1}));
    let server = MockServer::start(vec![
        mock_response(200, r#"{"profile":"107955"}"#),
        mock_response(200, api_response.to_string()),
        mock_response(200, run.to_string()),
        mock_response(201, "{}"),
    ]);

    run_actor(&config(&server), client()).await.unwrap();
    let requests = server.requests();
    assert_eq!(requests.len(), 4);
    assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
    assert_eq!(
        request_parts(&requests[3]).1,
        "/v2/key-value-stores/test-store/records/OUTPUT"
    );
}

#[tokio::test]
async fn stores_output_for_empty_results_without_pagination_or_dataset_writes() {
    let api_response = json!({"code": 0, "data": null, "next_cursor": "ignored"});
    let server = MockServer::start(vec![
        mock_response(200, r#"{"profile":"@tiktok"}"#),
        mock_response(200, api_response.to_string()),
        mock_response(201, "{}"),
    ]);

    run_actor(&config(&server), client()).await.unwrap();
    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(request_parts(&requests[1]).0, "GET");
    assert!(!request_parts(&requests[1]).1.contains("cursor"));
    assert_eq!(
        request_parts(&requests[2]).1,
        "/v2/key-value-stores/test-store/records/OUTPUT"
    );
}

#[tokio::test]
async fn does_not_retry_scrappa_failures_and_does_not_write_partial_output() {
    let server = MockServer::start(vec![
        mock_response(200, r#"{"profile":"@tiktok"}"#),
        mock_response(503, r#"{"message":"upstream unavailable"}"#),
    ]);

    let error = run_actor(&config(&server), client()).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Scrappa API error (503): upstream unavailable"
    );
    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(request_parts(&requests[1]).0, "GET");
}

#[tokio::test]
async fn retries_transient_apify_input_errors_before_continuing() {
    let api_response = json!({"code": 0, "data": null});
    let server = MockServer::start(vec![
        mock_response(503, r#"{"error":"temporary"}"#),
        mock_response(200, r#"{"profile":"@tiktok"}"#),
        mock_response(200, api_response.to_string()),
        mock_response(201, "{}"),
    ]);

    run_actor(&config(&server), client()).await.unwrap();
    let requests = server.requests();
    assert_eq!(requests.len(), 4);
    assert_eq!(
        request_parts(&requests[0]).1,
        "/v2/key-value-stores/test-store/records/INPUT"
    );
    assert_eq!(
        request_parts(&requests[1]).1,
        "/v2/key-value-stores/test-store/records/INPUT"
    );
}
