use booking_search_scraper::{
    apify::ActorConfig,
    booking::{
        booking_search_url, build_booking_dataset_item, build_booking_search_requests,
        describe_booking_search_request, get_booking_search_results, today_utc,
        MAX_SEARCHES_PER_RUN,
    },
    pricing::{
        charge_plan, ChargeBudget, ChargePlan, BOOKING_RESULT_CHARGE_EVENT,
        DEFAULT_DATASET_ITEM_EVENT,
    },
    run_actor,
    scrappa::{
        parsed_scrappa_error_message, retry_delay_ms, ScrappaClient, SCRAPPA_MAX_ATTEMPTS,
        SCRAPPA_REQUEST_TIMEOUT,
    },
};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::{collections::BTreeMap, time::Duration};
use url::Url;

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

struct MockResponse {
    status: u16,
    body: String,
}

struct MockServer {
    base_url: Url,
    requests: Arc<Mutex<Vec<String>>>,
    stopped: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl MockServer {
    fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stopped = Arc::new(AtomicBool::new(false));
        let thread_requests = Arc::clone(&requests);
        let thread_stopped = Arc::clone(&stopped);
        let thread = thread::spawn(move || {
            let mut responses = responses.into_iter();
            while !thread_stopped.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let request = read_request(&mut stream).unwrap_or_default();
                        thread_requests.lock().unwrap().push(request);
                        let response = responses.next().unwrap_or_else(|| MockResponse {
                            status: 500,
                            body: "No mock response configured".to_owned(),
                        });
                        let reason = StatusCode::from_u16(response.status)
                            .ok()
                            .and_then(|status| status.canonical_reason())
                            .unwrap_or("Unknown status");
                        let message = format!(
                            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
                            response.status,
                            reason,
                            response.body.len(),
                            response.body
                        );
                        let _ = stream.write_all(message.as_bytes());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
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
        self.requests.lock().unwrap().clone()
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
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
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

fn mock_response(status: u16, body: &str) -> MockResponse {
    MockResponse {
        status,
        body: body.to_owned(),
    }
}

fn run_response(max_charge: Value, charged_counts: Value) -> String {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": { "actorChargeEvents": {
                    "booking-result": { "eventPriceUsd": 0.01 },
                    "apify-default-dataset-item": { "eventPriceUsd": 0.001 },
                    "apify-actor-start": { "eventPriceUsd": 0.00005 }
                }}
            },
            "options": { "maxTotalChargeUsd": max_charge },
            "chargedEventCounts": charged_counts
        }
    })
    .to_string()
}

fn config(server: &MockServer) -> ActorConfig {
    let mut scrappa_api_base_url = server.base_url.clone();
    scrappa_api_base_url.set_path("/api");
    ActorConfig::new(
        server.base_url.clone(),
        scrappa_api_base_url,
        "test-store",
        "test-dataset",
        "test-token",
        "test-run",
        "test-key",
    )
}

fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap()
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
        .skip(1)
        .any(|line| {
            line.split_once(':').is_some_and(|(header, actual)| {
                header.eq_ignore_ascii_case(name) && actual.trim() == value
            })
        })
}

fn input_response(input: &Value) -> MockResponse {
    mock_response(200, &input.to_string())
}

#[test]
fn preserves_schema_prefills_and_batch_limits() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    assert_eq!(schema["properties"]["ss"]["prefill"], "Paris");
    assert_eq!(schema["properties"]["checkin"]["prefill"], "2026-07-01");
    assert_eq!(schema["properties"]["checkout"]["prefill"], "2026-07-03");
    assert_eq!(
        schema["properties"]["searches"]["maxItems"],
        MAX_SEARCHES_PER_RUN
    );
    assert_eq!(
        schema["properties"]["searches"]["items"]["required"],
        json!(["ss"])
    );
}

#[test]
fn builds_single_and_batch_searches_in_order() {
    let checkin = today_utc();
    let checkout = "2099-12-31";
    let single = build_booking_search_requests(&json!({
        "ss": " Paris ", "checkin": checkin, "checkout": checkout,
        "group_adults": "2", "group_children": 1, "no_rooms": "1",
        "lang": "EN-US", "currency": "eur"
    }))
    .unwrap();
    assert_eq!(single.len(), 1);
    assert_eq!(single[0].params["ss"], "Paris");
    assert_eq!(single[0].params["group_adults"], 2);
    assert_eq!(single[0].params["lang"], "en-us");
    assert_eq!(single[0].params["currency"], "EUR");

    let batch = build_booking_search_requests(&json!({
        "searches": [{"ss": "Paris"}, {"ss": "Berlin"}]
    }))
    .unwrap();
    assert_eq!(
        batch
            .iter()
            .map(|request| request.index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(batch[0].params["ss"], "Paris");
    assert_eq!(batch[1].params["ss"], "Berlin");
}

#[test]
fn rejects_invalid_input_with_existing_messages() {
    assert_eq!(
        build_booking_search_requests(&json!({"ss": ""}))
            .unwrap_err()
            .to_string(),
        "ss is required"
    );
    assert_eq!(
        build_booking_search_requests(&json!({"ss": "Paris", "checkin": "2099-01-01"}))
            .unwrap_err()
            .to_string(),
        "checkin and checkout must be provided together"
    );
    assert_eq!(
        build_booking_search_requests(
            &json!({"ss": "Paris", "checkin": "2099-02-30", "checkout": "2099-03-02"})
        )
        .unwrap_err()
        .to_string(),
        "checkin must be a valid calendar date"
    );
    assert_eq!(
        build_booking_search_requests(
            &json!({"ss": "Paris", "checkin": "2099-03-02", "checkout": "2099-03-01"})
        )
        .unwrap_err()
        .to_string(),
        "checkout must be after checkin"
    );
    assert_eq!(
        build_booking_search_requests(
            &json!({"ss": "Paris", "checkin": "2099-03-02", "checkout": "2099-03-02"})
        )
        .unwrap_err()
        .to_string(),
        "checkout must be after checkin"
    );
    assert_eq!(
        build_booking_search_requests(
            &json!({"ss": "Paris", "checkin": "2020-01-01", "checkout": "2020-01-02"})
        )
        .unwrap_err()
        .to_string(),
        "checkin must be today or a future date"
    );
    assert_eq!(
        build_booking_search_requests(
            &json!({"ss": "Paris", "checkin": "2099/02/01", "checkout": "2099-03-02"})
        )
        .unwrap_err()
        .to_string(),
        "checkin must use YYYY-MM-DD format"
    );
    assert!(
        build_booking_search_requests(&json!({"ss": "Paris", "group_adults": 31}))
            .unwrap_err()
            .to_string()
            .contains("group_adults must be between 1 and 30")
    );
    assert!(
        build_booking_search_requests(&json!({"ss": "Paris", "group_children": -1}))
            .unwrap_err()
            .to_string()
            .contains("group_children must be between 0 and 20")
    );
    assert!(
        build_booking_search_requests(&json!({"ss": "Paris", "no_rooms": 0}))
            .unwrap_err()
            .to_string()
            .contains("no_rooms must be between 1 and 30")
    );
    assert!(
        build_booking_search_requests(&json!({"ss": "Paris", "lang": "english"}))
            .unwrap_err()
            .to_string()
            .contains("lang must be a valid language code")
    );
    assert!(
        build_booking_search_requests(&json!({"ss": "Paris", "currency": "EURO"}))
            .unwrap_err()
            .to_string()
            .contains("currency must be a 3-letter currency code")
    );
    assert!(build_booking_search_requests(&json!({"searches": []}))
        .unwrap_err()
        .to_string()
        .contains("searches must include at least one search"));
    assert!(
        build_booking_search_requests(&json!({"ss": "Paris", "searches": "Paris"}))
            .unwrap_err()
            .to_string()
            .contains("searches must be an array of search objects")
    );
    assert!(
        build_booking_search_requests(&json!({"searches": ["Paris"]}))
            .unwrap_err()
            .to_string()
            .contains("searches[0] must be an object")
    );
    assert!(build_booking_search_requests(
        &json!({"searches": Value::Array((0..26).map(|_| json!({"ss":"Paris"})).collect())})
    )
    .unwrap_err()
    .to_string()
    .contains("more than 25 searches per run"));
}

#[test]
fn descriptions_and_query_encoding_keep_request_fields() {
    let mut params = BTreeMap::new();
    params.insert("ss".to_owned(), json!("New York & Co"));
    params.insert("checkin".to_owned(), json!("2099-01-01"));
    params.insert("checkout".to_owned(), json!("2099-01-04"));
    params.insert("group_adults".to_owned(), json!(2));
    params.insert("currency".to_owned(), json!("EUR"));
    assert_eq!(
        describe_booking_search_request(&params),
        "\"New York & Co\" 2099-01-01 to 2099-01-04 (currency=EUR, group_adults=2)"
    );
    let url = booking_search_url(&Url::parse("https://scrappa.co/api").unwrap(), &params).unwrap();
    assert_eq!(url.path(), "/api/booking/search");
    let query = url.query_pairs().collect::<BTreeMap<_, _>>();
    assert_eq!(
        query.get("ss").map(|value| value.as_ref()),
        Some("New York & Co")
    );
    assert_eq!(
        query.get("group_adults").map(|value| value.as_ref()),
        Some("2")
    );
}

#[test]
fn prefers_data_results_then_falls_back_to_top_level_and_normalizes_output() {
    let rows = get_booking_search_results(&json!({
        "data": {"results": [
            {
                "title":"Hotel One",
                "link":"https://booking.example/one",
                "thumbnail":"https://example.test/one.jpg",
                "review_score":"8.7",
                "review_score_word":"Excellent",
                "review_count":"1240",
                "address":"Paris",
                "price_for_display":"EUR 420",
                "extra":"kept"
            },
            {"name":"Hotel Two"}
        ]},
        "results": [{"name":"ignored"}]
    }));
    assert_eq!(rows.len(), 2);
    let params = build_booking_search_requests(&json!({"ss":"Paris", "currency":"EUR"}))
        .unwrap()
        .remove(0)
        .params;
    let first = build_booking_dataset_item(&rows[0], &params, 0);
    assert_eq!(first["name"], "Hotel One");
    assert_eq!(first["url"], "https://booking.example/one");
    assert_eq!(first["image"], "https://example.test/one.jpg");
    assert_eq!(first["review_score"], 8.7);
    assert_eq!(first["review_score_word"], "Excellent");
    assert_eq!(first["review_count"], 1240);
    assert_eq!(first["location"], "Paris");
    assert_eq!(first["price"], "EUR 420");
    assert_eq!(first["currency"], "EUR");
    assert_eq!(first["extra"], "kept");
    assert_eq!(first["request_search_index"], 0);
    assert_eq!(first["request_ss"], "Paris");
    assert!(first["request_checkin"].is_null());
    assert_eq!(
        get_booking_search_results(&json!({"results":[{"name":"fallback"}]})).len(),
        1
    );
    assert!(get_booking_search_results(&json!({"data": {}})).is_empty());
}

#[test]
fn ignores_non_finite_numeric_strings_in_normalized_fields() {
    let params = build_booking_search_requests(&json!({"ss":"Paris"}))
        .unwrap()
        .remove(0)
        .params;
    let item = build_booking_dataset_item(
        &json!({"name":"Hotel Example", "review_score":"Infinity", "review_count":"-Infinity"}),
        &params,
        0,
    );

    assert!(item["review_score"].is_null());
    assert!(item["review_count"].is_null());
}

#[test]
fn validates_retry_policy_and_timeout() {
    assert_eq!(SCRAPPA_MAX_ATTEMPTS, 3);
    assert_eq!(SCRAPPA_REQUEST_TIMEOUT, Duration::from_secs(90));
    assert_eq!(retry_delay_ms(1, 0), 2_000);
    assert_eq!(retry_delay_ms(2, 500), 4_500);
    assert_eq!(retry_delay_ms(1, 1_000), 3_000);
    assert_eq!(retry_delay_ms(8, 1_000), 10_000);
}

#[test]
fn charge_plan_counts_existing_events_and_dataset_item_price() {
    let run: Value = serde_json::from_str(&run_response(
        json!(0.03105),
        json!({ "apify-actor-start": 1, "booking-result": 1, "apify-default-dataset-item": 1 }),
    ))
    .unwrap();
    let plan = charge_plan(
        &run,
        BOOKING_RESULT_CHARGE_EVENT,
        10,
        &mut ChargeBudget::default(),
    )
    .unwrap();
    assert_eq!(plan.charged_count, 1);
    assert!(plan.event_charge_limit_reached);

    let unlimited: Value = serde_json::from_str(&run_response(Value::Null, json!({}))).unwrap();
    assert_eq!(
        charge_plan(
            &unlimited,
            BOOKING_RESULT_CHARGE_EVENT,
            2,
            &mut ChargeBudget::default()
        )
        .unwrap(),
        ChargePlan {
            charged_count: 2,
            event_charge_limit_reached: false
        }
    );
}

#[test]
fn charge_plan_accounts_for_successful_local_writes_when_run_metadata_lags() {
    let stale_run: Value = serde_json::from_str(&run_response(
        json!(0.02205),
        json!({ "apify-actor-start": 1 }),
    ))
    .unwrap();
    let mut budget = ChargeBudget::default();

    let first_plan = charge_plan(&stale_run, BOOKING_RESULT_CHARGE_EVENT, 1, &mut budget).unwrap();
    assert_eq!(first_plan.charged_count, 1);
    assert!(!first_plan.event_charge_limit_reached);
    budget.record_dataset_items_saved(1).unwrap();
    budget
        .record_charged_event(BOOKING_RESULT_CHARGE_EVENT, 1)
        .unwrap();

    let second_plan = charge_plan(&stale_run, BOOKING_RESULT_CHARGE_EVENT, 1, &mut budget).unwrap();
    assert_eq!(second_plan.charged_count, 1);
    assert!(second_plan.event_charge_limit_reached);
    budget.record_dataset_items_saved(1).unwrap();
    budget
        .record_charged_event(BOOKING_RESULT_CHARGE_EVENT, 1)
        .unwrap();

    let third_plan = charge_plan(&stale_run, BOOKING_RESULT_CHARGE_EVENT, 1, &mut budget).unwrap();
    assert_eq!(third_plan.charged_count, 0);
    assert!(third_plan.event_charge_limit_reached);
}

#[test]
fn charge_plan_allows_dataset_writes_when_the_synthetic_item_event_is_disabled() {
    let mut run: Value = serde_json::from_str(&run_response(json!(1.0), json!({}))).unwrap();
    run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
        .as_object_mut()
        .unwrap()
        .remove(DEFAULT_DATASET_ITEM_EVENT);
    let mut budget = ChargeBudget::default();

    let first_plan = charge_plan(&run, BOOKING_RESULT_CHARGE_EVENT, 1, &mut budget).unwrap();
    assert_eq!(first_plan.charged_count, 1);
    budget.record_dataset_items_saved(1).unwrap();
    budget
        .record_charged_event(BOOKING_RESULT_CHARGE_EVENT, 1)
        .unwrap();
    let second_plan = charge_plan(&run, BOOKING_RESULT_CHARGE_EVENT, 1, &mut budget).unwrap();

    assert_eq!(second_plan.charged_count, 1);
    assert!(!second_plan.event_charge_limit_reached);
}

#[test]
fn parses_scrappa_validation_errors_using_the_original_response_shape() {
    assert_eq!(
        parsed_scrappa_error_message(
            r#"{"message":"Invalid request","errors":{"ss":["The destination is required."]}}"#,
            "Unprocessable Entity"
        ),
        Some("Invalid request - ss: The destination is required.".to_owned())
    );
    assert_eq!(
        parsed_scrappa_error_message(r#"{"error":{"message":"Nested error"}}"#, "Bad Request"),
        Some("Bad Request".to_owned())
    );
}

#[tokio::test]
async fn transient_scrappa_status_retries_and_preserves_auth_headers() {
    let server = MockServer::start(vec![
        mock_response(503, r#"{"message":"busy"}"#),
        mock_response(200, r#"{"data":{"results":[]}}"#),
    ]);
    let client = ScrappaClient::new(http_client(), server.base_url.clone(), "secret".to_owned());
    let params = build_booking_search_requests(&json!({"ss":"Paris"}))
        .unwrap()
        .remove(0)
        .params;
    assert!(client.get(&params).await.is_ok());
    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert!(has_header(&requests[0], "X-API-Key", "secret"));
    assert!(has_header(
        &requests[0],
        "User-Agent",
        "thescrappa-booking-search-scraper/1.0"
    ));
    assert!(request_parts(&requests[0])
        .1
        .starts_with("/booking/search?"));
}

#[tokio::test]
async fn scrappa_validation_errors_keep_the_original_message_and_field_details() {
    let server = MockServer::start(vec![mock_response(
        422,
        r#"{"message":"Invalid request","errors":{"ss":["The destination is required."]}}"#,
    )]);
    let client = ScrappaClient::new(http_client(), server.base_url.clone(), "secret".to_owned());
    let params = build_booking_search_requests(&json!({"ss":"Paris"}))
        .unwrap()
        .remove(0)
        .params;

    let error = client.get(&params).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "Scrappa API error (422): Invalid request - ss: The destination is required."
    );
}

#[tokio::test]
async fn paid_batch_writes_results_in_order_and_charges_named_event() {
    let server = MockServer::start(vec![
        input_response(&json!({"searches":[{"ss":"Paris"},{"ss":"Berlin"}]})),
        mock_response(
            200,
            r#"{"data":{"results":[{"name":"Paris A"},{"name":"Paris B"}]}}"#,
        ),
        mock_response(
            200,
            &run_response(json!(10.0), json!({"apify-actor-start":1})),
        ),
        mock_response(201, "{}"),
        mock_response(201, "{}"),
        mock_response(200, r#"{"results":[{"name":"Berlin A"}]}"#),
        mock_response(
            200,
            &run_response(
                json!(10.0),
                json!({"apify-actor-start":1,"booking-result":2,"apify-default-dataset-item":2}),
            ),
        ),
        mock_response(201, "{}"),
        mock_response(201, "{}"),
    ]);
    let config = config(&server);
    run_actor(&http_client(), &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 9);
    assert!(has_header(
        &requests[0],
        "Authorization",
        "Bearer test-token"
    ));
    assert_eq!(request_parts(&requests[3]).0, "POST");
    let first_dataset: Value = serde_json::from_str(request_parts(&requests[3]).2).unwrap();
    assert_eq!(first_dataset[0]["name"], "Paris A");
    assert_eq!(first_dataset[1]["name"], "Paris B");
    assert_eq!(first_dataset[0]["request_search_index"], 0);
    let first_charge: Value = serde_json::from_str(request_parts(&requests[4]).2).unwrap();
    assert_eq!(
        first_charge,
        json!({"eventName":"booking-result", "count":2})
    );
    assert!(has_header(
        &requests[4],
        "idempotency-key",
        "test-run-booking-result-0"
    ));
    let second_dataset: Value = serde_json::from_str(request_parts(&requests[7]).2).unwrap();
    assert_eq!(second_dataset[0]["name"], "Berlin A");
    assert_eq!(second_dataset[0]["request_search_index"], 1);
    assert_eq!(
        request_parts(&requests[5]).1.split('?').next(),
        Some("/api/booking/search")
    );
}

#[tokio::test]
async fn non_pay_per_event_runs_write_every_result_without_a_custom_charge() {
    let server = MockServer::start(vec![
        input_response(&json!({"ss":"Paris"})),
        mock_response(
            200,
            r#"{"data":{"results":[{"name":"One"},{"name":"Two"}]}}"#,
        ),
        mock_response(
            200,
            r#"{"data":{"pricingInfo":{"pricingModel":"PRICE_PER_RESULT"}}}"#,
        ),
        mock_response(201, "{}"),
    ]);
    let config = config(&server);

    run_actor(&http_client(), &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 4);
    let saved: Value = serde_json::from_str(request_parts(&requests[3]).2).unwrap();
    assert_eq!(saved.as_array().unwrap().len(), 2);
    assert_eq!(saved[0]["name"], "One");
    assert_eq!(saved[1]["name"], "Two");
    assert!(requests.iter().all(|request| !request.contains("/charge")));
}

#[tokio::test]
async fn budget_limit_saves_only_affordable_prefix_then_exits_with_status() {
    let server = MockServer::start(vec![
        input_response(&json!({"ss":"Paris"})),
        mock_response(
            200,
            r#"{"data":{"results":[{"name":"One"},{"name":"Two"},{"name":"Three"}]}}"#,
        ),
        mock_response(
            200,
            &run_response(json!(0.02205), json!({"apify-actor-start":1})),
        ),
        mock_response(201, "{}"),
        mock_response(201, "{}"),
        mock_response(200, "{}"),
    ]);
    let config = config(&server);
    run_actor(&http_client(), &config).await.unwrap();

    let requests = server.requests();
    assert_eq!(requests.len(), 6);
    let dataset: Value = serde_json::from_str(request_parts(&requests[3]).2).unwrap();
    assert_eq!(dataset.as_array().unwrap().len(), 2);
    let charge: Value = serde_json::from_str(request_parts(&requests[4]).2).unwrap();
    assert_eq!(charge["count"], 2);
    assert_eq!(request_parts(&requests[5]).0, "PUT");
    let status: Value = serde_json::from_str(request_parts(&requests[5]).2).unwrap();
    assert_eq!(status["isStatusMessageTerminal"], true);
    assert!(status["statusMessage"].as_str().unwrap().contains("2 of 3"));
}
