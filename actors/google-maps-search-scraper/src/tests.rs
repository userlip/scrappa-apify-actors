use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread::{self, JoinHandle},
};

struct MockResponse {
    status: u16,
    body: String,
    delay: Duration,
    close_without_response: bool,
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
            let mut response_threads = Vec::new();
            for response in responses {
                let (mut stream, _) = loop {
                    if stopped.load(Ordering::Relaxed) {
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
                let _ = request_sender.send(request);
                response_threads.push(thread::spawn(move || {
                        if response.close_without_response {
                            return;
                        }
                        if !response.delay.is_zero() {
                            thread::sleep(response.delay);
                        }
                        let reason = match response.status {
                            200 => "OK",
                            201 => "Created",
                            400 => "Bad Request",
                            401 => "Unauthorized",
                            422 => "Unprocessable Entity",
                            429 => "Too Many Requests",
                            500 => "Internal Server Error",
                            503 => "Service Unavailable",
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
                        let _ = stream.write_all(message.as_bytes());
                    }));
            }
            for response_thread in response_threads {
                let _ = response_thread.join();
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
            if bytes.len() >= header_end + 4 + content_length.unwrap_or_default() {
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
        close_without_response: false,
    }
}

fn delayed_response(status: u16, body: &str, delay: Duration) -> MockResponse {
    MockResponse {
        status,
        body: body.to_owned(),
        delay,
        close_without_response: false,
    }
}

fn disconnected_response() -> MockResponse {
    MockResponse {
        status: 0,
        body: String::new(),
        delay: Duration::ZERO,
        close_without_response: true,
    }
}

fn pricing_response(max_charge: f64, charged_counts: Value) -> MockResponse {
    response(
        200,
        &json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                        "apify-actor-start": {"eventPriceUsd": 0.00005}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": charged_counts
            }
        })
        .to_string(),
    )
}

fn config(base_url: &Url) -> ActorConfig {
    let mut scrappa_api_base_url = base_url.clone();
    scrappa_api_base_url.set_path("/api");
    ActorConfig {
        apify_api_base_url: base_url.clone(),
        scrappa_api_base_url,
        default_key_value_store_id: "test-store".to_owned(),
        default_dataset_id: "test-dataset".to_owned(),
        actor_run_id: "test-run".to_owned(),
        input_key: "INPUT".to_owned(),
        apify_token: "test-token-not-a-real-credential".to_owned(),
        scrappa_api_key: "test-scrappa-key-not-a-real-credential".to_owned(),
        max_total_charge_usd: None,
        apify_request_timeout: Duration::from_secs(2),
        scrappa_request_timeout: Duration::from_secs(1),
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

fn has_header(request: &str, name: &str, value: &str) -> bool {
    request
        .split_once("\r\n\r\n")
        .unwrap_or((request, ""))
        .0
        .lines()
        .any(|line| {
            let Some((header_name, header_value)) = line.split_once(':') else {
                return false;
            };
            header_name.eq_ignore_ascii_case(name) && header_value.trim() == value
        })
}

fn has_apify_auth(request: &str) -> bool {
    has_header(
        request,
        "authorization",
        "Bearer test-token-not-a-real-credential",
    )
}

fn query_pairs(request: &str) -> Vec<(String, String)> {
    let (_, target, _) = request_parts(request);
    let url = Url::parse(&format!("http://localhost{target}")).unwrap();
    url.query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}

#[test]
fn search_request_keeps_prefilled_values_and_typescript_parameter_rules() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    let properties = &schema["properties"];
    let input = json!({
        "query": properties["query"]["prefill"],
        "hl": properties["hl"]["default"],
        "gl": properties["gl"]["prefill"],
        "fallback_zoom": properties["fallback_zoom"]["default"]
    });
    let request = build_search_request(&input).unwrap();

    assert_eq!(request.query, "starbucks times square new york");
    assert_eq!(request.fallback_zoom, "13");
    assert_eq!(
        request.params,
        vec![
            (
                "query".to_owned(),
                "starbucks times square new york".to_owned()
            ),
            ("hl".to_owned(), "en".to_owned()),
            ("gl".to_owned(), "us".to_owned()),
            ("use_cache".to_owned(), "1".to_owned()),
        ]
    );

    let no_cache = build_search_request(&json!({
        "query": "pizza",
        "hl": "",
        "gl": "us",
        "debug": false,
        "use_cache": false,
        "maximum_cache_age": 3600
    }))
    .unwrap();
    assert_eq!(
        no_cache.params,
        vec![
            ("query".to_owned(), "pizza".to_owned()),
            ("gl".to_owned(), "us".to_owned()),
        ]
    );
}

#[test]
fn actor_runtime_config_uses_the_rust_image_without_changing_run_limits() {
    let config: Value = serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();
    assert_eq!(config["dockerfile"], "./Dockerfile");
    assert_eq!(config["resources"]["memoryMbytes"], 256);
    assert!(config.get("defaultRunOptions").is_none());
    assert!(config.get("meta").is_none());
}

#[test]
fn adds_contact_aliases_without_overwriting_existing_fields() {
    let response = add_search_response_aliases(json!({
        "items": [
            {
                "full_address": "123 Main St",
                "phone_numbers": ["111", " ", "222"]
            },
            {
                "full_address": "Address",
                "address": "Existing address",
                "phone_numbers": ["333"],
                "phone": "Existing phone"
            }
        ],
        "query": "coffee"
    }));

    assert_eq!(response["items"][0]["address"], "123 Main St");
    assert_eq!(response["items"][0]["phone"], "111, 222");
    assert_eq!(response["items"][1]["address"], "Existing address");
    assert_eq!(response["items"][1]["phone"], "Existing phone");
    assert_eq!(response["query"], "coffee");
}

#[tokio::test]
async fn transient_simple_search_falls_back_and_obeys_ppe_budget_for_dataset_rows() {
    let input = json!({
        "query": "pizza in Manhattan",
        "hl": "en",
        "gl": "us",
        "debug": true,
        "use_cache": false,
        "maximum_cache_age": 3600,
        "fallback_zoom": 17
    });
    let upstream = json!({
        "query": "pizza in Manhattan",
        "items": [
            {"name": "A", "full_address": "Address A", "phone_numbers": ["111"]},
            {"name": "B", "full_address": "Address B", "phone_numbers": ["222"]},
            {"name": "C", "full_address": "Address C", "phone_numbers": ["333"]}
        ]
    });
    let server = MockServer::start(vec![
        response(200, &input.to_string()),
        response(503, "<html>Cloudflare temporarily unavailable</html>"),
        response(200, &upstream.to_string()),
        pricing_response(0.00065, json!({"apify-actor-start": 1})),
        response(201, "{}"),
        response(201, "{}"),
    ]);
    let config = config(&server.base_url);
    run_actor(&client(), &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 6);
    assert_eq!(
        request_parts(&requests[0]).1,
        "/v2/key-value-stores/test-store/records/INPUT"
    );
    assert!(has_apify_auth(&requests[0]));
    assert!(has_header(
        &requests[1],
        "x-api-key",
        "test-scrappa-key-not-a-real-credential"
    ));
    assert!(has_header(&requests[1], "accept", "application/json"));
    assert!(has_header(&requests[1], "user-agent", SCRAPPA_USER_AGENT));
    assert_eq!(
        query_pairs(&requests[1]),
        vec![
            ("query".to_owned(), "pizza in Manhattan".to_owned()),
            ("hl".to_owned(), "en".to_owned()),
            ("gl".to_owned(), "us".to_owned()),
            ("debug".to_owned(), "1".to_owned()),
        ]
    );
    assert!(has_header(
        &requests[2],
        "x-api-key",
        "test-scrappa-key-not-a-real-credential"
    ));
    assert_eq!(
        request_parts(&requests[2]).1.split('?').next(),
        Some("/api/maps/advanced-search")
    );
    assert_eq!(
        query_pairs(&requests[2]).last(),
        Some(&("zoom".to_owned(), "17".to_owned()))
    );
    assert!(!has_apify_auth(&requests[1]));

    assert_eq!(request_parts(&requests[3]).1, "/v2/actor-runs/test-run");
    assert!(has_apify_auth(&requests[3]));
    let (method, path, body) = request_parts(&requests[4]);
    assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
    assert!(has_apify_auth(&requests[4]));
    assert_eq!(
        serde_json::from_str::<Value>(body).unwrap(),
        json!([
            {"name":"A","full_address":"Address A","address":"Address A","phone_numbers":["111"],"phone":"111"},
            {"name":"B","full_address":"Address B","address":"Address B","phone_numbers":["222"],"phone":"222"}
        ])
    );

    let (method, path, body) = request_parts(&requests[5]);
    assert_eq!(
        (method, path),
        ("PUT", "/v2/key-value-stores/test-store/records/OUTPUT")
    );
    assert!(has_apify_auth(&requests[5]));
    let output: Value = serde_json::from_str(body).unwrap();
    assert_eq!(output["fallback_used"], "advanced-search");
    assert_eq!(output["items"].as_array().unwrap().len(), 3);
    assert_eq!(output["items"][2]["phone"], "333");
}

#[tokio::test]
async fn dataset_payloads_are_chunked_below_the_api_limit_in_input_order() {
    const MAX_REQUEST_BYTES: usize = 5_000_000;
    let large_description = format!(
        "{}{}",
        "🍕".repeat(MAX_REQUEST_BYTES / 16),
        "\"".repeat(MAX_REQUEST_BYTES / 8)
    );
    let items = vec![
        json!({"position": 1, "description": large_description}),
        json!({"position": 2, "description": large_description}),
    ];
    let server = MockServer::start(vec![
        pricing_response(1.0, json!({})),
        response(201, "{}"),
        response(201, "{}"),
    ]);

    let saved = push_dataset_items(&client(), &config(&server.base_url), &items)
        .await
        .unwrap();

    assert_eq!(saved, 2);
    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    let dataset_requests = requests
        .iter()
        .filter(|request| request_parts(request).0 == "POST")
        .collect::<Vec<_>>();
    assert_eq!(dataset_requests.len(), 2);

    let mut stored_positions = Vec::new();
    for request in dataset_requests {
        let (_, path, body) = request_parts(request);
        assert_eq!(path, "/v2/datasets/test-dataset/items");
        assert!(body.len() < MAX_REQUEST_BYTES);
        let chunk: Vec<Value> = serde_json::from_str(body).unwrap();
        stored_positions.extend(chunk.iter().map(|item| item["position"].as_u64().unwrap()));
    }
    assert_eq!(stored_positions, vec![1, 2]);
}

#[tokio::test]
async fn dataset_append_is_not_retried_after_a_lost_response() {
    let server = MockServer::start(vec![
        response(200, r#"{"query":"pizza"}"#),
        response(200, r#"{"items":[{"name":"Example"}]}"#),
        pricing_response(1.0, json!({})),
        disconnected_response(),
        response(201, "{}"),
        response(201, "{}"),
    ]);

    let error = run_actor(&client(), &config(&server.base_url))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("Apify dataset write failed"));
    let requests = server.requests();
    let dataset_writes = requests
        .iter()
        .filter(|request| request_parts(request).0 == "POST")
        .count();
    assert_eq!(dataset_writes, 1);
    assert_eq!(requests.len(), 4);
}

#[tokio::test]
async fn scrappa_timeout_uses_the_advanced_search_fallback() {
    let server = MockServer::start(vec![
        response(200, r#"{"query":"pizza","fallback_zoom":13}"#),
        delayed_response(
            200,
            r#"{"items":[{"name":"late"}]}"#,
            Duration::from_millis(100),
        ),
        response(200, r#"{"items":[{"name":"fallback"}]}"#),
        pricing_response(1.0, json!({})),
        response(201, "{}"),
        response(201, "{}"),
    ]);
    let mut config = config(&server.base_url);
    config.scrappa_request_timeout = Duration::from_millis(20);

    run_actor(&client(), &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 6);
    assert!(request_parts(&requests[1])
        .1
        .starts_with("/api/maps/simple-search?"));
    assert!(request_parts(&requests[2])
        .1
        .starts_with("/api/maps/advanced-search?"));
}

#[tokio::test]
async fn non_transient_scrappa_errors_fail_without_fallback_or_storage_writes() {
    let server = MockServer::start(vec![
        response(200, r#"{"query":"pizza"}"#),
        response(
            422,
            r#"{"message":"The given data was invalid.","errors":{"query":["The query field is required."]}}"#,
        ),
    ]);
    let error = run_actor(&client(), &config(&server.base_url))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("Scrappa API error (422)"));
    assert!(error
        .to_string()
        .contains("query: The query field is required."));
    assert_eq!(server.requests().len(), 2);
}

#[tokio::test]
async fn empty_results_skip_pricing_and_dataset_but_write_full_output() {
    let server = MockServer::start(vec![
        response(200, r#"{"query":"no match"}"#),
        response(200, r#"{"items":[],"query":"no match"}"#),
        response(201, "{}"),
    ]);
    run_actor(&client(), &config(&server.base_url))
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        request_parts(&requests[2]).1,
        "/v2/key-value-stores/test-store/records/OUTPUT"
    );
    assert_eq!(
        serde_json::from_str::<Value>(request_parts(&requests[2]).2).unwrap(),
        json!({"items": [], "query": "no match"})
    );
}

#[tokio::test]
async fn apify_storage_requests_retry_transient_api_errors() {
    let input = r#"{"query":"retry input"}"#;
    let server = MockServer::start(vec![
        response(503, r#"{"error":"temporarily unavailable"}"#),
        response(200, input),
        response(200, r#"{"items":[]}"#),
        response(201, "{}"),
    ]);

    run_actor(&client(), &config(&server.base_url))
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 4);
    assert_eq!(request_parts(&requests[0]).1, request_parts(&requests[1]).1);
    assert!(has_apify_auth(&requests[0]));
    assert!(has_apify_auth(&requests[1]));
}

#[tokio::test]
async fn apify_output_put_retries_transient_api_errors() {
    let server = MockServer::start(vec![
        response(200, r#"{"query":"empty results"}"#),
        response(200, r#"{"items":[]}"#),
        response(503, r#"{"error":"temporarily unavailable"}"#),
        response(201, "{}"),
    ]);

    run_actor(&client(), &config(&server.base_url))
        .await
        .unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 4);
    assert_eq!(request_parts(&requests[2]).0, "PUT");
    assert_eq!(request_parts(&requests[2]).1, request_parts(&requests[3]).1);
    assert!(has_apify_auth(&requests[2]));
    assert!(has_apify_auth(&requests[3]));
}

#[test]
fn ppe_budget_rejects_missing_or_invalid_charge_data() {
    assert!(DatasetBudget::from_run(&json!({"data": {}}), None)
        .unwrap()
        .is_none());
    assert!(DatasetBudget::from_run(
        &json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {}}
                }
            }
        }),
        None,
    )
    .is_err());
}
