use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
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
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded_requests = Arc::clone(&requests);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut responses = responses.into_iter();
            while !stopped.load(Ordering::Relaxed) && Instant::now() < deadline {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(_) => break,
                };
                let request = read_request(&mut stream).unwrap_or_default();
                recorded_requests.lock().unwrap().push(request);
                let Some(response) = responses.next() else {
                    break;
                };
                let reason = match response.status {
                    200 => "OK",
                    201 => "Created",
                    400 => "Bad Request",
                    401 => "Unauthorized",
                    422 => "Unprocessable Entity",
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
                    break;
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
        self.requests.lock().unwrap().clone()
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
    let mut content_length = 0;
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            if content_length == 0 {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
            }
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn response(status: u16, body: impl Into<String>) -> MockResponse {
    MockResponse {
        status,
        body: body.into(),
    }
}

fn config(base_url: &Url) -> Config {
    Config {
        apify_api_base: base_url.clone(),
        scrappa_api_base: endpoint_url(base_url, &["api"]).unwrap(),
        apify_token: "test-token-not-a-real-credential".to_owned(),
        key_value_store_id: "test-store".to_owned(),
        dataset_id: "test-dataset".to_owned(),
        actor_run_id: "test-run".to_owned(),
        input_key: "INPUT".to_owned(),
        scrappa_api_key: "test-scrappa-key".to_owned(),
        scrappa_request_timeout: Duration::from_secs(60),
    }
}

fn pricing_run(max_charge: Option<Value>, actor_start_charged: u64) -> Value {
    let mut run = json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                    "apify-actor-start": {"eventPriceUsd": 0.0001}
                }}
            },
            "options": {},
            "chargedEventCounts": {"apify-actor-start": actor_start_charged}
        }
    });
    if let Some(max_charge) = max_charge {
        run["data"]["options"]["maxTotalChargeUsd"] = max_charge;
    }
    run
}

fn pricing_response(max_charge: Option<Value>, actor_start_charged: u64) -> String {
    pricing_run(max_charge, actor_start_charged).to_string()
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

fn request_has_header(request: &str, expected_name: &str, expected_value: &str) -> bool {
    request
        .split_once("\r\n\r\n")
        .unwrap_or((request, ""))
        .0
        .lines()
        .skip(1)
        .any(|line| {
            let Some((name, value)) = line.split_once(':') else {
                return false;
            };
            name.eq_ignore_ascii_case(expected_name) && value.trim() == expected_value
        })
}

fn query_parameters(request: &str) -> Vec<(String, String)> {
    let (_, path, _) = request_parts(request);
    Url::parse(&format!("http://example.test{path}"))
        .unwrap()
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}

#[tokio::test]
async fn actor_preserves_search_options_retries_writes_affordable_rows_and_stores_full_output() {
    let input = json!({
        "query": "best restaurants in new york",
        "location": "New York, NY, USA",
        "gl": "us",
        "hl": "en",
        "google_domain": "google.com",
        "start": 20,
        "amount": 10,
        "safe": "active",
        "tbs": "qdr:w",
        "tbm": "nws",
        "lr": "lang_en",
        "cr": "countryUS",
        "uule": "w+CAIQIC",
        "nfpr": 1,
        "filter": 0
    });
    let full_response = json!({
        "search_information": {"query_displayed": "best restaurants in new york", "total_results": 1234},
        "organic_results": [
            {"position": 1, "title": "First", "link": "https://one.test", "snippet": "A", "source": "one.test"},
            {"position": 2, "title": "Second", "link": "https://two.test", "snippet": "B", "source": "two.test"}
        ],
        "related_searches": [{"query": "pizza", "link": "https://google.test/search?q=pizza"}],
        "related_questions": [{"question": "Where?"}],
        "knowledge_graph": {"title": "New York"}
    });
    let server = MockServer::start(vec![
        response(200, input.to_string()),
        response(503, ""),
        response(200, full_response.to_string()),
        response(200, pricing_response(Some(json!(0.00045)), 1)),
        response(201, ""),
        response(200, ""),
    ]);
    let config = config(&server.base_url);
    let http = Client::new();

    run_actor(&http, &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 6);
    assert!(requests[0].starts_with("GET /v2/key-value-stores/test-store/records/INPUT HTTP/1.1"));
    assert!(request_has_header(
        &requests[0],
        "authorization",
        "Bearer test-token-not-a-real-credential"
    ));
    let (_, first_scrappa_path, _) = request_parts(&requests[1]);
    let (_, second_scrappa_path, _) = request_parts(&requests[2]);
    assert_eq!(first_scrappa_path, second_scrappa_path);
    assert!(first_scrappa_path.starts_with("/api/search?"));
    assert!(request_has_header(
        &requests[2],
        "X-API-Key",
        "test-scrappa-key"
    ));
    assert_eq!(
        query_parameters(&requests[2]),
        vec![
            (
                "query".to_owned(),
                "best restaurants in new york".to_owned()
            ),
            ("location".to_owned(), "New York, NY, USA".to_owned()),
            ("gl".to_owned(), "us".to_owned()),
            ("hl".to_owned(), "en".to_owned()),
            ("google_domain".to_owned(), "google.com".to_owned()),
            ("start".to_owned(), "20".to_owned()),
            ("amount".to_owned(), "10".to_owned()),
            ("safe".to_owned(), "active".to_owned()),
            ("tbs".to_owned(), "qdr:w".to_owned()),
            ("tbm".to_owned(), "nws".to_owned()),
            ("lr".to_owned(), "lang_en".to_owned()),
            ("cr".to_owned(), "countryUS".to_owned()),
            ("uule".to_owned(), "w+CAIQIC".to_owned()),
            ("nfpr".to_owned(), "1".to_owned()),
            ("filter".to_owned(), "0".to_owned()),
        ]
    );
    assert!(requests[3].starts_with("GET /v2/actor-runs/test-run HTTP/1.1"));
    assert!(requests[4].starts_with("POST /v2/datasets/test-dataset/items HTTP/1.1"));
    let (_, _, dataset_body) = request_parts(&requests[4]);
    assert_eq!(
        serde_json::from_str::<Value>(dataset_body).unwrap(),
        json!([full_response["organic_results"][0]])
    );
    assert!(requests[5].starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT HTTP/1.1"));
    let (_, _, output_body) = request_parts(&requests[5]);
    assert_eq!(
        serde_json::from_str::<Value>(output_body).unwrap(),
        full_response
    );
}

#[tokio::test]
async fn actor_skips_pricing_and_dataset_calls_when_search_has_no_organic_results() {
    let server = MockServer::start(vec![
        response(200, json!({"query": "empty results"}).to_string()),
        response(
            200,
            json!({"organic_results": [], "related_searches": []}).to_string(),
        ),
        response(200, ""),
    ]);
    let config = config(&server.base_url);
    let http = Client::new();

    run_actor(&http, &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert!(requests[2].starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT HTTP/1.1"));
    assert!(requests
        .iter()
        .all(|request| !request.contains("/v2/actor-runs/") && !request.contains("/v2/datasets/")));
}

#[tokio::test]
async fn actor_saves_all_results_with_missing_null_or_zero_spending_limit() {
    let input = json!({"query": "restaurants"});
    let full_response = json!({
        "organic_results": [
            {"position": 1, "title": "First", "link": "https://one.test"},
            {"position": 2, "title": "Second", "link": "https://two.test"}
        ],
        "related_searches": [{"query": "pizza"}]
    });

    for (case, max_charge) in [
        ("missing", None),
        ("null", Some(Value::Null)),
        ("zero", Some(json!(0))),
    ] {
        let server = MockServer::start(vec![
            response(200, input.to_string()),
            response(200, full_response.to_string()),
            response(200, pricing_response(max_charge, 1)),
            response(201, ""),
            response(200, ""),
        ]);
        let config = config(&server.base_url);
        let http = Client::new();

        run_actor(&http, &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 5, "{case}");
        assert!(
            requests[3].starts_with("POST /v2/datasets/test-dataset/items HTTP/1.1"),
            "{case}"
        );
        let (_, _, dataset_body) = request_parts(&requests[3]);
        assert_eq!(
            serde_json::from_str::<Value>(dataset_body).unwrap(),
            full_response["organic_results"],
            "{case}"
        );
        assert!(
            requests[4].starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT HTTP/1.1"),
            "{case}"
        );
        let (_, _, output_body) = request_parts(&requests[4]);
        assert_eq!(
            serde_json::from_str::<Value>(output_body).unwrap(),
            full_response,
            "{case}"
        );
    }
}

#[tokio::test]
async fn permanent_scrappa_errors_are_not_retried_and_keep_validation_details() {
    let server = MockServer::start(vec![response(
        422,
        json!({
            "message": "The given data was invalid.",
            "errors": {"query": ["The query field is required.", "The query is invalid."]}
        })
        .to_string(),
    )]);
    let config = config(&server.base_url);
    let http = Client::new();
    let params = build_search_params(&json!({"query": "restaurants"})).unwrap();

    let error = fetch_google_search(&http, &config, &params)
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Scrappa API error (422): The given data was invalid. - query: The query field is required., The query is invalid."
    );
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn pay_per_event_capacity_accounts_for_other_charged_events() {
    let run = pricing_run(Some(json!(0.00075)), 1);

    assert_eq!(affordable_dataset_items(&run, 10).unwrap(), 2);
}

#[test]
fn non_ppe_runs_keep_the_existing_pricing_error_even_without_a_spending_limit() {
    let run = json!({
        "data": {
            "pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM"},
            "options": {}
        }
    });

    assert_eq!(
        affordable_dataset_items(&run, 10).unwrap_err().to_string(),
        "Apify run is not configured for pay-per-event pricing"
    );
}

#[test]
fn query_is_required_and_existing_prefill_is_unchanged() {
    assert!(build_search_params(&json!({"query": ""}))
        .unwrap_err()
        .to_string()
        .contains("Search query is required"));
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    assert_eq!(
        schema
            .pointer("/properties/query/prefill")
            .and_then(Value::as_str),
        Some("best restaurants in new york")
    );
}
