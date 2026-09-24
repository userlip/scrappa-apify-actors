use super::*;
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use url::Url;

struct MockResponse {
    status: u16,
    body: String,
    delay: Duration,
}

struct MockServer {
    base_url: Url,
    requests: Receiver<String>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, requests) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            for response in responses {
                let (mut stream, _) = loop {
                    if stopped.load(Ordering::Relaxed) || std::time::Instant::now() >= deadline {
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
                let request = read_request(&mut stream).unwrap_or_default();
                if sender.send(request).is_err() {
                    return;
                }
                thread::sleep(response.delay);
                let reason = match response.status {
                    200 => "OK",
                    201 => "Created",
                    400 => "Bad Request",
                    401 => "Unauthorized",
                    429 => "Too Many Requests",
                    500 => "Internal Server Error",
                    504 => "Gateway Timeout",
                    _ => "Mock Response",
                };
                let message = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                if stream.write_all(message.as_bytes()).is_err() {
                    return;
                }
            }
        });
        Self {
            base_url: Url::parse(&format!("http://{address}")).unwrap(),
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

fn response(status: u16, body: &str) -> MockResponse {
    MockResponse {
        status,
        body: body.to_owned(),
        delay: Duration::ZERO,
    }
}

fn json_response(status: u16, body: &Value) -> MockResponse {
    response(status, &body.to_string())
}

fn test_config(apify_api_base_url: &Url, scrappa_api_base_url: &Url) -> ActorConfig {
    ActorConfig {
        apify_api_base_url: apify_api_base_url.clone(),
        scrappa_api_base_url: scrappa_api_base_url.clone(),
        default_key_value_store_id: "test-store".to_owned(),
        default_dataset_id: "test-dataset".to_owned(),
        input_key: "INPUT".to_owned(),
        actor_run_id: "test-run".to_owned(),
        apify_token: "test-token-not-a-real-credential".to_owned(),
        scrappa_api_key: "test-scrappa-key".to_owned(),
        scrappa_request_timeout: SCRAPPA_REQUEST_TIMEOUT,
    }
}

fn run_pricing(max_charge: f64, item_price: f64, charged_counts: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {
                    "actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": item_price},
                        "other-event": {"eventPriceUsd": 0.0001}
                    }
                }
            },
            "chargedEventCounts": charged_counts,
            "options": {"maxTotalChargeUsd": max_charge}
        }
    })
}

fn run_pricing_with_options(options: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {
                    "actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                        "other-event": {"eventPriceUsd": 0.0001}
                    }
                }
            },
            "chargedEventCounts": {},
            "options": options
        }
    })
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

fn has_header(request: &str, name: &str, expected: &str) -> bool {
    request
        .split_once("\r\n\r\n")
        .unwrap_or((request, ""))
        .0
        .lines()
        .skip(1)
        .any(|line| {
            line.split_once(':').is_some_and(|(key, value)| {
                key.eq_ignore_ascii_case(name) && value.trim().eq_ignore_ascii_case(expected)
            })
        })
}

fn query_pairs(request_path: &str) -> Vec<(String, String)> {
    let url = Url::parse(&format!("http://127.0.0.1{request_path}")).unwrap();
    url.query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}

#[test]
fn builds_username_url_and_numeric_lookup_params() {
    let mut warnings = Vec::new();
    let params = build_tiktok_followers_params(
        &json!({"profile":"@tiktok","count":10,"time":" 0 "}),
        &mut |warning| warnings.push(warning),
    )
    .unwrap();
    assert_eq!(params.lookup, TikTokLookup::UniqueId("@tiktok".to_owned()));
    assert_eq!(params.count.as_deref(), Some("10"));
    assert_eq!(params.time.as_deref(), Some("0"));
    assert!(warnings.is_empty());

    let url = normalize_tiktok_unique_id("https://www.tiktok.com/@tiktok?lang=en").unwrap();
    assert_eq!(url, "@tiktok");

    let numeric = build_tiktok_followers_params(&json!({"profile":"107955"}), &mut |_| {}).unwrap();
    assert_eq!(numeric.lookup, TikTokLookup::UserId("107955".to_owned()));

    let explicit =
        build_tiktok_followers_params(&json!({"unique_id":"tiktok","user_id":"abc"}), &mut |_| {})
            .unwrap();
    assert_eq!(
        explicit.lookup,
        TikTokLookup::UniqueId("@tiktok".to_owned())
    );

    let by_id =
        build_tiktok_followers_params(&json!({"user_id":" 107955 ","count":25}), &mut |_| {})
            .unwrap();
    assert_eq!(by_id.lookup, TikTokLookup::UserId("107955".to_owned()));
}

#[test]
fn preserves_cursor_alias_and_omits_invalid_optional_values() {
    let mut warnings = Vec::new();
    let params = build_tiktok_followers_params(
        &json!({"profile":"@tiktok","count":0,"time":"abc","cursor":"123"}),
        &mut |warning| warnings.push(warning),
    )
    .unwrap();
    assert_eq!(params.lookup, TikTokLookup::UniqueId("@tiktok".to_owned()));
    assert_eq!(params.count, None);
    assert_eq!(params.time, None);
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("count must be an integer between 1 and 50")));
    assert!(warnings
        .iter()
        .any(|warning| warning.contains("time must contain digits only")));

    let cursor =
        build_tiktok_followers_params(&json!({"profile":"@tiktok","cursor":"123"}), &mut |_| {})
            .unwrap();
    assert_eq!(cursor.time.as_deref(), Some("123"));

    let empty =
        build_tiktok_followers_params(&json!({"profile":"@tiktok","time":"   "}), &mut |_| {})
            .unwrap();
    assert_eq!(empty.time, None);

    assert!(
        build_tiktok_followers_params(&json!({"profile":" "}), &mut |_| {})
            .unwrap_err()
            .to_string()
            .contains("TikTok unique_id or user_id is required")
    );
}

#[test]
fn rejects_invalid_tiktok_urls_usernames_and_ids() {
    assert!(normalize_tiktok_unique_id("https://example.com/@tiktok")
        .unwrap_err()
        .to_string()
        .contains("must be on tiktok.com"));
    assert!(normalize_tiktok_unique_id("http://www.tiktok.com/@tiktok")
        .unwrap_err()
        .to_string()
        .contains("must use HTTPS"));
    assert!(normalize_tiktok_unique_id("https://www.tiktok.com/@")
        .unwrap_err()
        .to_string()
        .contains("must use the format https://www.tiktok.com/@username"));
    assert!(normalize_tiktok_unique_id("@tik-tok")
        .unwrap_err()
        .to_string()
        .contains("TikTok username must be"));
    assert!(normalize_tiktok_user_id("1234567890123456789012345678901")
        .unwrap_err()
        .to_string()
        .contains("30 digits or fewer"));
    assert!(normalize_tiktok_user_id("10x")
        .unwrap_err()
        .to_string()
        .contains("digits only"));
}

#[test]
fn extracts_follower_arrays_and_pagination_fallbacks() {
    let followers = vec![json!({"user_id":"1"})];
    assert_eq!(extract_followers(Some(&json!(followers))), followers);
    assert_eq!(
        extract_followers(Some(&json!({"followers":[{"user_id":"2"}]}))),
        vec![json!({"user_id":"2"})]
    );
    assert_eq!(
        extract_followers(Some(&json!({"users":[{"user_id":"3"}]}))),
        vec![json!({"user_id":"3"})]
    );
    assert_eq!(
        extract_followers(Some(&json!({"user_list":[{"user_id":"4"}]}))),
        vec![json!({"user_id":"4"})]
    );
    assert!(extract_followers(Some(&Value::Null)).is_empty());

    assert_eq!(
        extract_pagination(Some(&json!({"hasMore":true,"time":1711111111}))),
        (true, json!(1711111111))
    );
    assert_eq!(
        extract_pagination(Some(&json!({"has_more":false,"time":"0"}))),
        (false, json!("0"))
    );
    assert_eq!(
        extract_pagination(Some(
            &json!({"has_more":true,"min_time":"1","max_time":"2"})
        )),
        (true, json!("1"))
    );
    assert_eq!(
        extract_pagination(Some(&json!({"hasMore":true,"max_time":2}))),
        (true, json!(2))
    );
    assert_eq!(extract_pagination(Some(&json!([]))), (false, Value::Null));
}

#[test]
fn adds_lookup_fields_with_javascript_object_spread_behavior() {
    assert_eq!(
        follower_item(&json!({"user_id":"1"}), Some("@tiktok"), "107955"),
        json!({
            "user_id":"1",
            "lookup_unique_id":"@tiktok",
            "lookup_user_id":"107955"
        })
    );
    assert_eq!(
        follower_item(&json!({"user_id":"1"}), None, "107955")["lookup_unique_id"],
        Value::Null
    );
    assert_eq!(
        follower_item(&json!("ab"), None, "107955"),
        json!({"0":"a","1":"b","lookup_unique_id":null,"lookup_user_id":"107955"})
    );
}

#[test]
fn applies_positive_ppe_caps_to_remaining_run_budget() {
    let run = run_pricing(0.0002, 0.0001, json!({"other-event":1}));
    assert_eq!(affordable_dataset_items(&run, 5).unwrap(), 1);

    let no_charge = run_pricing(0.0002, 0.0, json!({}));
    assert_eq!(affordable_dataset_items(&no_charge, 5).unwrap(), 5);
}

#[test]
fn non_ppe_runs_can_save_all_requested_dataset_items() {
    let run = json!({"data":{"pricingInfo":{"pricingModel":"PAY_PER_RESULT"}}});
    assert_eq!(affordable_dataset_items(&run, 5).unwrap(), 5);
}

#[test]
fn zero_or_unspecified_ppe_spending_limit_is_unlimited() {
    for options in [
        json!({}),
        json!({"maxTotalChargeUsd": 0}),
        json!({"maxTotalChargeUsd": null}),
    ] {
        let run = run_pricing_with_options(options);
        assert_eq!(affordable_dataset_items(&run, 5).unwrap(), 5);
    }
}

#[test]
fn actor_schema_keeps_prefill_and_input_defaults() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    assert_eq!(schema["properties"]["profile"]["prefill"], "@tiktok");
    assert_eq!(schema["properties"]["count"]["default"], 10);
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("profile")));
}

#[tokio::test]
async fn resolves_tiktok_user_id_and_runs_authenticated_budgeted_output_flow() {
    let followers_response = json!({
        "code": 0,
        "data": {
            "followers": [
                {"user_id":"1","unique_id":"first","nickname":"First"},
                {"user_id":"2","unique_id":"second","nickname":"Second"}
            ],
            "hasMore": true,
            "time": "1711111111"
        },
        "processed_time": 0.5
    });
    let input = json!({"profile":"@tiktok","count":2,"cursor":"9"}).to_string();
    let pricing = run_pricing(0.0002, 0.0001, json!({"other-event":1}));
    let server = MockServer::start(vec![
        response(200, &input),
        response(200, r#"{"code":0,"data":{"user_id":"107955"}}"#),
        json_response(200, &followers_response),
        json_response(200, &pricing),
        response(201, ""),
        response(201, ""),
    ]);
    let mut scrappa_base_url = server.base_url.clone();
    scrappa_base_url.set_path("/api");
    let config = test_config(&server.base_url, &scrappa_base_url);

    run_actor(&Client::builder().build().unwrap(), &config)
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 6);
    assert_eq!(
        request_parts(&requests[0]).1,
        "/v2/key-value-stores/test-store/records/INPUT"
    );
    assert!(has_header(
        &requests[0],
        "authorization",
        "Bearer test-token-not-a-real-credential"
    ));
    assert_eq!(
        request_parts(&requests[1]).1,
        "/api/tiktok/user/profile?unique_id=%40tiktok"
    );
    assert!(has_header(&requests[1], "x-api-key", "test-scrappa-key"));
    assert!(has_header(&requests[1], "accept", "application/json"));
    let query = query_pairs(request_parts(&requests[2]).1);
    assert_eq!(
        query,
        vec![
            ("user_id".to_owned(), "107955".to_owned()),
            ("count".to_owned(), "2".to_owned()),
            ("time".to_owned(), "9".to_owned())
        ]
    );

    let (method, path, body) = request_parts(&requests[4]);
    assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
    assert_eq!(
        serde_json::from_str::<Value>(body).unwrap(),
        json!([{
            "user_id":"1",
            "unique_id":"first",
            "nickname":"First",
            "lookup_unique_id":"@tiktok",
            "lookup_user_id":"107955"
        }])
    );

    let (method, path, body) = request_parts(&requests[5]);
    assert_eq!(
        (method, path),
        ("PUT", "/v2/key-value-stores/test-store/records/OUTPUT")
    );
    assert_eq!(
        serde_json::from_str::<Value>(body).unwrap(),
        followers_response
    );
}

#[tokio::test]
async fn writes_all_followers_for_non_ppe_runs_and_keeps_full_output() {
    let followers_response = json!({
        "code": 0,
        "data": {
            "followers": [
                {"user_id":"1","unique_id":"first"},
                {"user_id":"2","unique_id":"second"}
            ],
            "hasMore": true,
            "time": "1711111111"
        }
    });
    let input = json!({"profile":"107955","count":2}).to_string();
    let non_ppe_run = json!({"data":{"pricingInfo":{"pricingModel":"PAY_PER_RESULT"}}});
    let server = MockServer::start(vec![
        response(200, &input),
        json_response(200, &followers_response),
        json_response(200, &non_ppe_run),
        response(201, ""),
        response(201, ""),
    ]);
    let mut scrappa_base_url = server.base_url.clone();
    scrappa_base_url.set_path("/api");
    let config = test_config(&server.base_url, &scrappa_base_url);

    run_actor(&Client::new(), &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    let (method, path, body) = request_parts(&requests[3]);
    assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
    assert_eq!(
        serde_json::from_str::<Value>(body).unwrap(),
        json!([
            {"user_id":"1","unique_id":"first","lookup_unique_id":null,"lookup_user_id":"107955"},
            {"user_id":"2","unique_id":"second","lookup_unique_id":null,"lookup_user_id":"107955"}
        ])
    );

    let (method, path, body) = request_parts(&requests[4]);
    assert_eq!(
        (method, path),
        ("PUT", "/v2/key-value-stores/test-store/records/OUTPUT")
    );
    assert_eq!(
        serde_json::from_str::<Value>(body).unwrap(),
        followers_response
    );
}

#[tokio::test]
async fn uses_nested_profile_id_and_does_not_resolve_explicit_user_ids() {
    let server = MockServer::start(vec![response(
        200,
        r#"{"code":0,"data":{"user":{"id":107955}}}"#,
    )]);
    let mut scrappa_base_url = server.base_url.clone();
    scrappa_base_url.set_path("/api");
    let config = test_config(&server.base_url, &scrappa_base_url);
    let params = build_tiktok_followers_params(&json!({"profile":"@tiktok"}), &mut |_| {}).unwrap();
    assert_eq!(
        resolve_tiktok_user_id(&Client::new(), &config, &params)
            .await
            .unwrap(),
        "107955"
    );
    assert_eq!(server.requests().len(), 1);

    let no_request_server = MockServer::start(vec![]);
    let mut scrappa_base_url = no_request_server.base_url.clone();
    scrappa_base_url.set_path("/api");
    let config = test_config(&no_request_server.base_url, &scrappa_base_url);
    let params = build_tiktok_followers_params(&json!({"profile":"107955"}), &mut |_| {}).unwrap();
    assert_eq!(
        resolve_tiktok_user_id(&Client::new(), &config, &params)
            .await
            .unwrap(),
        "107955"
    );
    assert!(no_request_server.requests().is_empty());
}

#[tokio::test]
async fn reports_nonzero_scrappa_codes_and_does_not_resolve_missing_ids() {
    let server = MockServer::start(vec![response(200, r#"{"code":-1,"msg":"not found"}"#)]);
    let mut scrappa_base_url = server.base_url.clone();
    scrappa_base_url.set_path("/api");
    let config = test_config(&server.base_url, &scrappa_base_url);
    let params =
        build_tiktok_followers_params(&json!({"profile":"@missing"}), &mut |_| {}).unwrap();
    let error = resolve_tiktok_user_id(&Client::new(), &config, &params)
        .await
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("Scrappa TikTok Profile API returned code -1: not found"));
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn reports_upstream_http_errors_without_retrying() {
    let server = MockServer::start(vec![response(
        429,
        r#"{"message":"rate limited","errors":{"profile":["try later"]}}"#,
    )]);
    let mut scrappa_base_url = server.base_url.clone();
    scrappa_base_url.set_path("/api");
    let config = test_config(&server.base_url, &scrappa_base_url);
    let error = get_scrappa_json(
        &Client::new(),
        &config,
        &["tiktok", "user", "followers"],
        &[("user_id", "107955".to_owned())],
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Scrappa API error (429): rate limited - profile: try later"
    );
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn request_deadline_is_sixty_seconds_and_timeout_is_not_retried() {
    let server = MockServer::start(vec![MockResponse {
        status: 200,
        body: r#"{"code":0}"#.to_owned(),
        delay: Duration::from_millis(100),
    }]);
    let mut scrappa_base_url = server.base_url.clone();
    scrappa_base_url.set_path("/api");
    let mut config = test_config(&server.base_url, &scrappa_base_url);
    config.scrappa_request_timeout = Duration::from_millis(20);
    let error = get_scrappa_json(
        &Client::new(),
        &config,
        &["tiktok", "user", "followers"],
        &[("user_id", "107955".to_owned())],
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Scrappa API request timed out after 20ms"
    );
    assert_eq!(server.requests().len(), 1);
}

#[tokio::test]
async fn stores_output_and_skips_dataset_and_pricing_requests_when_no_followers_exist() {
    let response_body = json!({"code":0,"data":{"followers":[],"has_more":false}});
    let server = MockServer::start(vec![
        response(200, r#"{"profile":"107955"}"#),
        json_response(200, &response_body),
        response(201, ""),
    ]);
    let mut scrappa_base_url = server.base_url.clone();
    scrappa_base_url.set_path("/api");
    let config = test_config(&server.base_url, &scrappa_base_url);

    run_actor(&Client::new(), &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        request_parts(&requests[2]).1,
        "/v2/key-value-stores/test-store/records/OUTPUT"
    );
    assert!(!requests.iter().any(|request| {
        let (method, path, _) = request_parts(request);
        method == "POST" && path == "/v2/datasets/test-dataset/items"
    }));
    assert!(!requests
        .iter()
        .any(|request| { request_parts(request).1 == "/v2/actor-runs/test-run" }));
}

#[test]
fn requires_a_scrappa_api_key() {
    assert_eq!(scrappa_api_key(Some("test-key")).unwrap(), "test-key");
    assert!(scrappa_api_key(None)
        .unwrap_err()
        .to_string()
        .contains("SCRAPPA_API_KEY environment variable is not set"));
    assert!(scrappa_api_key(Some("")).is_err());
}
