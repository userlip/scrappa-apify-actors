use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

struct MockResponse {
    status: u16,
    body: String,
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
                let request = read_request(&mut stream).unwrap_or_default();
                if request_sender.send(request).is_err() {
                    return;
                }
                let reason = match response.status {
                    200 => "OK",
                    201 => "Created",
                    400 => "Bad Request",
                    401 => "Unauthorized",
                    403 => "Forbidden",
                    404 => "Not Found",
                    429 => "Too Many Requests",
                    500 => "Internal Server Error",
                    503 => "Service Unavailable",
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
            base_url: Url::parse(&format!("http://{address}/api")).unwrap(),
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
    }
}

fn pricing_response(max_charge: f64, counts: Value) -> MockResponse {
    response(
        200,
        &pricing_run(Some(serde_json::json!(max_charge)), counts).to_string(),
    )
}

fn pricing_run(max_charge: Option<Value>, counts: Value) -> Value {
    let mut run = serde_json::json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0002},
                    "apify-actor-start": {"eventPriceUsd": 0.0001}
                }}
            },
            "options": {},
            "chargedEventCounts": counts
        }
    });
    if let Some(max_charge) = max_charge {
        run["data"]["options"]["maxTotalChargeUsd"] = max_charge;
    }
    run
}

fn config(base_url: &Url) -> Config {
    Config {
        apify_api_base: base_url.clone(),
        scrappa_api_base: base_url.clone(),
        apify_token: "test-token-not-a-real-credential".to_owned(),
        actor_run_id: "test-run".to_owned(),
        key_value_store_id: "test-store".to_owned(),
        dataset_id: "test-dataset".to_owned(),
        input_key: "INPUT".to_owned(),
        scrappa_api_key: Some("test-scrappa-key".to_owned()),
    }
}

fn client() -> Client {
    Client::builder().build().unwrap()
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

fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
    request
        .split_once("\r\n\r\n")
        .unwrap_or((request, ""))
        .0
        .lines()
        .skip(1)
        .find_map(|line| {
            let (header_name, value) = line.split_once(':')?;
            header_name
                .eq_ignore_ascii_case(name)
                .then_some(value.trim())
        })
}

fn has_test_bearer_token(request: &str) -> bool {
    header_value(request, "authorization")
        == Some("Bearer test-token-not-a-real-credential")
}

fn input_response(body: &str) -> MockResponse {
    response(200, body)
}

#[test]
fn parses_input_and_builds_upstream_pagination_request() {
    let input = parse_input(&serde_json::json!({
        "username": "  @@natgeo  ",
        "max_id": "  next-page  "
    }))
    .unwrap();
    assert_eq!(
        input,
        ActorInput {
            username: "natgeo".to_owned(),
            max_id: "next-page".to_owned()
        }
    );
    let url = build_scrappa_url(&Url::parse("https://scrappa.co/api").unwrap(), &input)
        .unwrap();
    assert_eq!(url.path(), "/api/instagram/user/posts");
    let query = url.query_pairs().collect::<Vec<_>>();
    assert_eq!(query[0], ("username".into(), "natgeo".into()));
    assert_eq!(query[1], ("max_id".into(), "next-page".into()));

    let first_page = ActorInput {
        username: "natgeo".to_owned(),
        max_id: String::new(),
    };
    let url = build_scrappa_url(&Url::parse("https://scrappa.co/api").unwrap(), &first_page)
        .unwrap();
    assert_eq!(url.query(), Some("username=natgeo"));
}

#[test]
fn validates_missing_username_and_preserves_posts_shape_precedence() {
    for input in [Value::Null, serde_json::json!({}), serde_json::json!({"username": 7})] {
        assert_eq!(
            parse_input(&input).unwrap_err().to_string(),
            "Instagram username is required."
        );
    }
    assert_eq!(
        posts_from_response(&serde_json::json!({
            "posts": [],
            "data": {"posts": [{"id": "ignored"}]}
        })),
        Vec::<Value>::new()
    );
    assert_eq!(
        posts_from_response(&serde_json::json!({"data": {"posts": [{"id":"nested"}]}})),
        vec![serde_json::json!({"id":"nested"})]
    );
    assert_eq!(
        posts_from_response(&serde_json::json!({"data": [{"id":"array"}]})),
        vec![serde_json::json!({"id":"array"})]
    );
}

#[test]
fn spreads_post_fields_after_request_metadata() {
    assert_eq!(
        enrich_post(&serde_json::json!({"id":"1", "request_username":"source"}), "natgeo"),
        serde_json::json!({"request_username":"source", "id":"1"})
    );
    assert_eq!(
        enrich_post(&serde_json::json!(null), "natgeo"),
        serde_json::json!({"request_username":"natgeo"})
    );
    assert_eq!(
        enrich_post(&serde_json::json!(["a", "b"]), "natgeo"),
        serde_json::json!({"request_username":"natgeo", "0":"a", "1":"b"})
    );
}

#[test]
fn preserves_scrappa_response_error_classification() {
    assert!(validate_scrappa_response(401, r#"{"message":"bad key"}"#)
        .unwrap_err()
        .to_string()
        .contains("Scrappa API authentication failed: bad key"));
    assert!(validate_scrappa_response(503, r#"{"message":"upstream down"}"#)
        .unwrap_err()
        .to_string()
        .contains("Scrappa API returned HTTP 503: upstream down"));
    assert!(validate_scrappa_response(200, "<html>proxy</html>")
        .unwrap_err()
        .to_string()
        .contains("Scrappa API returned a non-JSON response: <html>proxy</html>"));
    assert!(validate_scrappa_response(200, r#"{"success":false,"error":"bad request"}"#)
        .unwrap_err()
        .to_string()
        .contains("Scrappa API returned an error response: bad request"));
    assert!(validate_scrappa_response(200, "").is_err());
}

#[test]
fn limits_dataset_items_by_all_charged_events_and_requires_pricing_data() {
    let run = serde_json::json!({"data": {
        "pricingInfo": {"pricingModel":"PAY_PER_EVENT", "pricingPerEvent":{"actorChargeEvents": {
            "apify-default-dataset-item":{"eventPriceUsd":0.0002},
            "apify-actor-start":{"eventPriceUsd":0.0001}
        }}},
        "options":{"maxTotalChargeUsd":0.0004},
        "chargedEventCounts":{"apify-actor-start":1}
    }});
    assert_eq!(affordable_dataset_items(&run, 5, 0).unwrap(), 1);
    assert_eq!(affordable_dataset_items(&run, 5, 1).unwrap(), 0);
    let missing_counts = serde_json::json!({"data": {
        "pricingInfo": {"pricingModel":"PAY_PER_EVENT", "pricingPerEvent":{"actorChargeEvents": {
            "apify-default-dataset-item":{"eventPriceUsd":0.0002}
        }}},
        "options":{"maxTotalChargeUsd":1}
    }});
    assert!(affordable_dataset_items(&missing_counts, 1, 0).is_err());
}

#[test]
fn treats_zero_omitted_and_null_total_charge_limits_as_unbounded() {
    let counts = serde_json::json!({"apify-actor-start": 1});
    for max_charge in [Some(serde_json::json!(0)), None, Some(Value::Null)] {
        let run = pricing_run(max_charge, counts.clone());
        assert_eq!(affordable_dataset_items(&run, 5, 0).unwrap(), 5);
    }
}

#[test]
fn positive_limit_accounts_for_custom_and_existing_dataset_charges_first() {
    let run = pricing_run(
        Some(serde_json::json!(0.0006)),
        serde_json::json!({
            "apify-actor-start": 1,
            "apify-default-dataset-item": 1
        }),
    );
    assert_eq!(affordable_dataset_items(&run, 5, 0).unwrap(), 1);
}

#[test]
fn non_ppe_runs_allow_all_requested_dataset_items() {
    let run = serde_json::json!({"data": {
        "pricingInfo": {"pricingModel": "PRICE_PER_RESULT"}
    }});
    assert_eq!(affordable_dataset_items(&run, 5, 0).unwrap(), 5);
}

#[test]
fn retries_only_safe_apify_methods_on_transient_statuses() {
    assert_eq!(
        apify_retry_delay("GET", StatusCode::TOO_MANY_REQUESTS, 0),
        Some(Duration::from_secs(1))
    );
    assert_eq!(
        apify_retry_delay("PUT", StatusCode::INTERNAL_SERVER_ERROR, 1),
        Some(Duration::from_secs(2))
    );
    assert_eq!(
        apify_retry_delay("GET", StatusCode::SERVICE_UNAVAILABLE, APIFY_MAX_RETRIES),
        None
    );
    assert_eq!(apify_retry_delay("POST", StatusCode::INTERNAL_SERVER_ERROR, 0), None);
}

#[tokio::test]
async fn local_smoke_preserves_input_auth_dataset_charge_and_output() {
    let upstream = serde_json::json!({
        "posts": [{"id":"post-1", "request_username":"upstream-value"}],
        "next_max_id":"next-cursor"
    });
    let server = MockServer::start(vec![
        input_response(r#"{"username":" @@natgeo ","max_id":" cursor-1 "}"#),
        response(200, &upstream.to_string()),
        pricing_response(0.0003, serde_json::json!({"apify-actor-start":1})),
        response(201, ""),
        response(200, ""),
    ]);
    let config = config(&server.base_url);
    run_actor(&client(), &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    assert!(has_test_bearer_token(&requests[0]));
    assert_eq!(request_parts(&requests[0]).1, "/api/v2/key-value-stores/test-store/records/INPUT");
    assert_eq!(
        request_parts(&requests[1]).1,
        "/api/instagram/user/posts?username=natgeo&max_id=cursor-1"
    );
    assert_eq!(header_value(&requests[1], "x-api-key"), Some("test-scrappa-key"));
    assert_eq!(header_value(&requests[1], "accept"), Some("application/json"));
    assert!(has_test_bearer_token(&requests[2]));
    assert_eq!(request_parts(&requests[2]).1, "/api/v2/actor-runs/test-run");
    let (method, path, body) = request_parts(&requests[3]);
    assert_eq!((method, path), ("POST", "/api/v2/datasets/test-dataset/items"));
    assert_eq!(
        serde_json::from_str::<Value>(body).unwrap(),
        serde_json::json!([{"request_username":"upstream-value", "id":"post-1"}])
    );
    assert!(has_test_bearer_token(&requests[4]));
    let (method, path, body) = request_parts(&requests[4]);
    assert_eq!((method, path), ("PUT", "/api/v2/key-value-stores/test-store/records/OUTPUT"));
    assert_eq!(serde_json::from_str::<Value>(body).unwrap(), upstream);
}

#[tokio::test]
async fn non_ppe_run_preserves_dataset_and_output_writes() {
    let upstream = serde_json::json!({
        "posts": [{"id":"post-1"}, {"id":"post-2"}]
    });
    let run = serde_json::json!({"data": {
        "pricingInfo": {"pricingModel": "PRICE_PER_RESULT"}
    }});
    let server = MockServer::start(vec![
        input_response(r#"{"username":"natgeo"}"#),
        response(200, &upstream.to_string()),
        response(200, &run.to_string()),
        response(201, ""),
        response(200, ""),
    ]);
    let config = config(&server.base_url);
    run_actor(&client(), &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    assert_eq!(request_parts(&requests[2]).1, "/api/v2/actor-runs/test-run");
    assert_eq!(request_parts(&requests[3]).0, "POST");
    assert_eq!(
        serde_json::from_str::<Value>(request_parts(&requests[3]).2).unwrap(),
        serde_json::json!([
            {"request_username":"natgeo", "id":"post-1"},
            {"request_username":"natgeo", "id":"post-2"}
        ])
    );
    assert_eq!(
        request_parts(&requests[4]).1,
        "/api/v2/key-value-stores/test-store/records/OUTPUT"
    );
}

#[tokio::test]
async fn zero_remaining_budget_skips_dataset_write_but_saves_output() {
    let upstream = serde_json::json!({"data":[{"id":"post-1"}]});
    let server = MockServer::start(vec![
        input_response(r#"{"username":"natgeo"}"#),
        response(200, &upstream.to_string()),
        pricing_response(0.0001, serde_json::json!({"apify-actor-start":1})),
        response(200, ""),
    ]);
    let config = config(&server.base_url);
    run_actor(&client(), &config).await.unwrap();
    let requests = server.requests();
    assert_eq!(requests.len(), 4);
    assert!(requests
        .iter()
        .all(|request| !request.starts_with("POST /api/v2/datasets/")));
    assert!(requests[3].starts_with("PUT /api/v2/key-value-stores/test-store/records/OUTPUT"));
    assert_eq!(
        serde_json::from_str::<Value>(request_parts(&requests[3]).2).unwrap(),
        upstream
    );
}

#[tokio::test]
async fn scrappa_auth_failure_stops_before_dataset_and_output_writes() {
    let server = MockServer::start(vec![
        input_response(r#"{"username":"natgeo"}"#),
        response(403, r#"{"message":"invalid API key"}"#),
    ]);
    let config = config(&server.base_url);
    let error = run_actor(&client(), &config).await.unwrap_err();
    assert!(error
        .to_string()
        .contains("Scrappa API authentication failed: invalid API key"));
    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests
        .iter()
        .all(|request| !request.starts_with("POST /api/v2/datasets/")
            && !request.starts_with("PUT /api/v2/key-value-stores/")));
}

#[tokio::test]
async fn apify_get_pricing_retries_but_dataset_post_does_not() {
    let upstream = serde_json::json!({"posts":[{"id":"post-1"}]});
    let server = MockServer::start(vec![
        input_response(r#"{"username":"natgeo"}"#),
        response(200, &upstream.to_string()),
        response(503, "temporarily unavailable"),
        pricing_response(1.0, serde_json::json!({})),
        response(500, "dataset write failed"),
        response(500, "unexpected dataset retry"),
    ]);
    let config = config(&server.base_url);
    let error = run_actor(&client(), &config).await.unwrap_err();
    assert!(error.to_string().contains("Apify dataset write failed"));
    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    assert_eq!(request_parts(&requests[2]).1, "/api/v2/actor-runs/test-run");
    assert_eq!(request_parts(&requests[3]).1, "/api/v2/actor-runs/test-run");
    assert_eq!(request_parts(&requests[4]).0, "POST");
    assert_eq!(
        requests
            .iter()
            .filter(|request| {
                let (method, path, _) = request_parts(request);
                method == "POST" && path == "/api/v2/datasets/test-dataset/items"
            })
            .count(),
        1
    );
}
