use serde_json::{json, Value};
use std::{
    collections::HashSet,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tiktok_challenge_search_scraper::{run_actor, ActorClient, ActorConfig};
use url::Url;

#[derive(Clone)]
struct MockResponse {
    status: u16,
    body: String,
    disconnect: bool,
}

impl MockResponse {
    fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            body: body.to_string(),
            disconnect: false,
        }
    }

    fn disconnect() -> Self {
        Self {
            status: 0,
            body: String::new(),
            disconnect: true,
        }
    }
}

struct MockServer {
    base_url: Url,
    requests: Receiver<String>,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    fn start(handler: impl Fn(&str) -> MockResponse + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, requests) = mpsc::channel();
        let stopped = Arc::new(AtomicBool::new(false));
        let thread_stopped = Arc::clone(&stopped);
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(15);
            while !thread_stopped.load(Ordering::Relaxed) && Instant::now() < deadline {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(_) => break,
                };
                let request = match read_request(&mut stream) {
                    Ok(request) => request,
                    Err(_) => break,
                };
                if request_sender.send(request.clone()).is_err() {
                    break;
                }
                let response = handler(&request);
                if response.disconnect {
                    drop(stream);
                    continue;
                }
                let reason = match response.status {
                    200 => "OK",
                    201 => "Created",
                    400 => "Bad Request",
                    401 => "Unauthorized",
                    404 => "Not Found",
                    429 => "Too Many Requests",
                    500 => "Internal Server Error",
                    _ => "Mock Response",
                };
                let reply = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.status,
                    reason,
                    response.body.len(),
                    response.body
                );
                if stream.write_all(reply.as_bytes()).is_err() {
                    break;
                }
            }
        });
        Self {
            base_url: Url::parse(&format!("http://{address}")).unwrap(),
            requests,
            stopped,
            thread: Some(thread),
        }
    }

    fn requests(&self) -> Vec<String> {
        self.requests.try_iter().collect()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let mut content_length = None;
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            if content_length.is_none() {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                content_length = Some(
                    headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0),
                );
            }
            if bytes.len() >= header_end + 4 + content_length.unwrap_or(0) {
                break;
            }
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn test_client(server: &MockServer) -> ActorClient {
    ActorClient::new(ActorConfig {
        apify_api_base_url: server.base_url.clone(),
        scrappa_api_base_url: Url::parse(&format!(
            "{}/api",
            server.base_url.as_str().trim_end_matches('/')
        ))
        .unwrap(),
        default_key_value_store_id: "test-store".to_owned(),
        default_dataset_id: "test-dataset".to_owned(),
        actor_run_id: "test-run".to_owned(),
        input_key: "INPUT".to_owned(),
        apify_token: "test-token".to_owned(),
        scrappa_api_key: "test-scrappa-key".to_owned(),
    })
    .unwrap()
}

fn request_parts(request: &str) -> (&str, &str, &str, &str) {
    let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
    let mut first_line = headers
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    (
        first_line.next().unwrap_or_default(),
        first_line.next().unwrap_or_default(),
        headers,
        body,
    )
}

fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    let (_, _, headers, _) = request_parts(request);
    headers.lines().find_map(|line| {
        let (header, value) = line.split_once(':')?;
        header.eq_ignore_ascii_case(name).then_some(value.trim())
    })
}

fn ppe_run(max_total: f64) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": { "actorChargeEvents": {
                    "challenge-result": { "eventPriceUsd": 0.00025 },
                    "apify-actor-start": { "eventPriceUsd": 0.0001 }
                }}
            },
            "options": { "maxTotalChargeUsd": max_total },
            "chargedEventCounts": { "apify-actor-start": 1 }
        }
    })
}

fn ppr_run() -> Value {
    json!({
        "data": { "pricingInfo": { "pricingModel": "PRICE_PER_DATASET_ITEM" } }
    })
}

fn ppe_input() -> Value {
    json!({ "keywords": ["cosplay", "fitness"], "count": 2 })
}

fn one_keyword_input() -> Value {
    json!({ "keyword": "tea" })
}

fn challenge(keyword: &str) -> Value {
    json!({
        "id": keyword,
        "cha_name": keyword,
        "desc": "challenge description",
        "stats": { "view_count": 100 },
        "raw_metadata": "preserved"
    })
}

fn search_response(request: &str) -> Value {
    let (_, path, _, _) = request_parts(request);
    let keyword = path
        .split("keywords=")
        .nth(1)
        .unwrap_or_default()
        .split('&')
        .next()
        .unwrap_or_default();
    json!({
        "code": 0,
        "processed_time": 12,
        "data": { "challenges": [challenge(keyword)] }
    })
}

fn route<'a>(method: &str, request: &'a str) -> &'a str {
    let (request_method, path, _, _) = request_parts(request);
    if request_method == method {
        path.split('?').next().unwrap_or_default()
    } else {
        ""
    }
}

async fn assert_charge_retry_is_idempotent(first_response: MockResponse) {
    let charge_attempts = Arc::new(AtomicUsize::new(0));
    let charged_count = Arc::new(AtomicUsize::new(0));
    let processed_keys = Arc::new(Mutex::new(HashSet::<String>::new()));
    let dataset_rows = Arc::new(Mutex::new(Vec::<Value>::new()));
    let output = Arc::new(Mutex::new(None::<Value>));
    let attempts = Arc::clone(&charge_attempts);
    let count = Arc::clone(&charged_count);
    let keys = Arc::clone(&processed_keys);
    let rows = Arc::clone(&dataset_rows);
    let saved_output = Arc::clone(&output);
    let server = MockServer::start(move |request| match route("GET", request) {
        "/v2/actor-runs/test-run" => MockResponse::json(200, ppe_run(0.01)),
        "/v2/key-value-stores/test-store/records/INPUT" => MockResponse::json(200, ppe_input()),
        "/api/tiktok/challenges/search" => MockResponse::json(200, search_response(request)),
        _ => match route("POST", request) {
            "/v2/datasets/test-dataset/items" => {
                rows.lock()
                    .unwrap()
                    .extend(serde_json::from_str::<Vec<Value>>(request_parts(request).3).unwrap());
                MockResponse::json(201, json!({}))
            }
            "/v2/actor-runs/test-run/charge" => {
                let body: Value = serde_json::from_str(request_parts(request).3).unwrap();
                let key = header_value(request, "idempotency-key").unwrap().to_owned();
                if keys.lock().unwrap().insert(key) {
                    count.fetch_add(body["count"].as_u64().unwrap() as usize, Ordering::SeqCst);
                }
                if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    first_response.clone()
                } else {
                    MockResponse::json(201, json!({}))
                }
            }
            _ => match route("PUT", request) {
                "/v2/key-value-stores/test-store/records/OUTPUT" => {
                    *saved_output.lock().unwrap() =
                        Some(serde_json::from_str(request_parts(request).3).unwrap());
                    MockResponse::json(201, json!({}))
                }
                _ => MockResponse::json(404, json!({ "error": "unexpected request" })),
            },
        },
    });

    let result = run_actor(&test_client(&server)).await.unwrap();
    let requests = server.requests();
    let charge_requests = requests
        .iter()
        .filter(|request| route("POST", request) == "/v2/actor-runs/test-run/charge")
        .collect::<Vec<_>>();
    let keys = charge_requests
        .iter()
        .map(|request| header_value(request, "idempotency-key").unwrap())
        .collect::<Vec<_>>();
    let charge_bodies = charge_requests
        .iter()
        .map(|request| serde_json::from_str::<Value>(request_parts(request).3).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(result["keywords_completed"], 2);
    assert_eq!(result["challenges_extracted"], 2);
    assert_eq!(dataset_rows.lock().unwrap().len(), 2);
    assert_eq!(charge_attempts.load(Ordering::SeqCst), 3);
    assert_eq!(charge_requests.len(), 3);
    assert_eq!(keys[0], keys[1]);
    assert_ne!(keys[1], keys[2]);
    assert!(charge_bodies
        .iter()
        .all(|body| body == &json!({ "eventName": "challenge-result", "count": 1 })));
    assert_eq!(charged_count.load(Ordering::SeqCst), 2);
    assert_eq!(output.lock().unwrap().as_ref().unwrap(), &result);
}

#[tokio::test]
async fn charge_retries_reuse_the_same_key_for_server_errors_rate_limits_and_lost_responses() {
    assert_charge_retry_is_idempotent(MockResponse::json(
        500,
        json!({ "error": "charge succeeded but response failed" }),
    ))
    .await;
    assert_charge_retry_is_idempotent(MockResponse::json(429, json!({ "error": "rate limited" })))
        .await;
    assert_charge_retry_is_idempotent(MockResponse::disconnect()).await;
}

async fn assert_dataset_append_is_not_retried(failure: MockResponse) {
    let dataset_attempts = Arc::new(AtomicUsize::new(0));
    let attempts = Arc::clone(&dataset_attempts);
    let server = MockServer::start(move |request| match route("GET", request) {
        "/v2/actor-runs/test-run" => MockResponse::json(200, ppr_run()),
        "/v2/key-value-stores/test-store/records/INPUT" => {
            MockResponse::json(200, one_keyword_input())
        }
        "/api/tiktok/challenges/search" => {
            MockResponse::json(200, json!({ "data": [challenge("tea")] }))
        }
        _ if route("POST", request) == "/v2/datasets/test-dataset/items" => {
            attempts.fetch_add(1, Ordering::SeqCst);
            failure.clone()
        }
        _ => MockResponse::json(404, json!({ "error": "unexpected request" })),
    });

    assert!(run_actor(&test_client(&server)).await.is_err());
    let requests = server.requests();
    assert_eq!(dataset_attempts.load(Ordering::SeqCst), 1);
    assert_eq!(
        requests
            .iter()
            .filter(|request| route("POST", request) == "/v2/datasets/test-dataset/items")
            .count(),
        1
    );
    assert!(!requests
        .iter()
        .any(|request| route("POST", request) == "/v2/actor-runs/test-run/charge"));
}

#[tokio::test]
async fn does_not_replay_ambiguous_dataset_appends() {
    assert_dataset_append_is_not_retried(MockResponse::json(
        500,
        json!({ "error": "dataset append may have succeeded" }),
    ))
    .await;
    assert_dataset_append_is_not_retried(MockResponse::disconnect()).await;
}

#[tokio::test]
async fn does_not_retry_scrappa_http_failures() {
    let server = MockServer::start(|request| match route("GET", request) {
        "/v2/actor-runs/test-run" => MockResponse::json(200, ppr_run()),
        "/v2/key-value-stores/test-store/records/INPUT" => {
            MockResponse::json(200, one_keyword_input())
        }
        "/api/tiktok/challenges/search" => MockResponse::json(
            401,
            json!({ "message": "Invalid key", "errors": { "keywords": ["is invalid"] } }),
        ),
        _ => MockResponse::json(404, json!({ "error": "unexpected request" })),
    });

    let error = run_actor(&test_client(&server))
        .await
        .unwrap_err()
        .to_string();
    let requests = server.requests();
    assert!(error.contains("Scrappa API error (401): Invalid key - keywords: is invalid"));
    assert_eq!(
        requests
            .iter()
            .filter(|request| route("GET", request) == "/api/tiktok/challenges/search")
            .count(),
        1
    );
}

#[tokio::test]
async fn stops_before_the_next_keyword_when_the_ppe_budget_is_exhausted() {
    let saved_rows = Arc::new(Mutex::new(Vec::<Value>::new()));
    let output = Arc::new(Mutex::new(None::<Value>));
    let rows = Arc::clone(&saved_rows);
    let saved_output = Arc::clone(&output);
    let server = MockServer::start(move |request| match route("GET", request) {
        "/v2/actor-runs/test-run" => MockResponse::json(200, ppe_run(0.00035)),
        "/v2/key-value-stores/test-store/records/INPUT" => MockResponse::json(200, ppe_input()),
        "/api/tiktok/challenges/search" => MockResponse::json(
            200,
            json!({
                "code": 0,
                "data": { "challenges": [challenge("first"), challenge("second")] }
            }),
        ),
        _ if route("POST", request) == "/v2/datasets/test-dataset/items" => {
            rows.lock()
                .unwrap()
                .extend(serde_json::from_str::<Vec<Value>>(request_parts(request).3).unwrap());
            MockResponse::json(201, json!({}))
        }
        _ if route("POST", request) == "/v2/actor-runs/test-run/charge" => {
            MockResponse::json(201, json!({}))
        }
        _ if route("PUT", request) == "/v2/key-value-stores/test-store/records/OUTPUT" => {
            *saved_output.lock().unwrap() =
                Some(serde_json::from_str(request_parts(request).3).unwrap());
            MockResponse::json(201, json!({}))
        }
        _ => MockResponse::json(404, json!({ "error": "unexpected request" })),
    });

    let result = run_actor(&test_client(&server)).await.unwrap();
    let requests = server.requests();
    assert_eq!(result["keywords_completed"], 1);
    assert_eq!(result["challenges_extracted"], 1);
    assert_eq!(
        result["status_message"],
        "Charge limit reached after saving 1 of 2 TikTok challenge result(s) for keyword cosplay."
    );
    assert_eq!(saved_rows.lock().unwrap().len(), 1);
    assert_eq!(
        requests
            .iter()
            .filter(|request| route("GET", request) == "/api/tiktok/challenges/search")
            .count(),
        1
    );
    assert_eq!(output.lock().unwrap().as_ref().unwrap(), &result);
}

#[tokio::test]
async fn non_ppe_run_stores_rows_without_charge_and_omits_default_count() {
    let saved_rows = Arc::new(Mutex::new(Vec::<Value>::new()));
    let rows = Arc::clone(&saved_rows);
    let server = MockServer::start(move |request| match route("GET", request) {
        "/v2/actor-runs/test-run" => MockResponse::json(200, ppr_run()),
        "/v2/key-value-stores/test-store/records/INPUT" => {
            MockResponse::json(200, one_keyword_input())
        }
        "/api/tiktok/challenges/search" => MockResponse::json(
            200,
            json!({ "data": [challenge("first"), challenge("second")] }),
        ),
        _ if route("POST", request) == "/v2/datasets/test-dataset/items" => {
            *rows.lock().unwrap() = serde_json::from_str(request_parts(request).3).unwrap();
            MockResponse::json(201, json!({}))
        }
        _ if route("PUT", request) == "/v2/key-value-stores/test-store/records/OUTPUT" => {
            MockResponse::json(201, json!({}))
        }
        _ => MockResponse::json(404, json!({ "error": "unexpected request" })),
    });

    let result = run_actor(&test_client(&server)).await.unwrap();
    let requests = server.requests();
    assert_eq!(result["challenges_extracted"], 2);
    assert_eq!(saved_rows.lock().unwrap().len(), 2);
    assert!(!requests
        .iter()
        .any(|request| route("POST", request) == "/v2/actor-runs/test-run/charge"));
    let search = requests
        .iter()
        .find(|request| route("GET", request) == "/api/tiktok/challenges/search")
        .unwrap();
    assert!(!request_parts(search).1.contains("count="));
}
