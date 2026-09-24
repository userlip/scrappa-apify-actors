use super::*;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

struct MockResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl MockResponse {
    fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: serde_json::to_vec(&body).unwrap(),
        }
    }

    fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

async fn start_mock_server(
    responses: Vec<MockResponse>,
) -> (String, tokio::task::JoinHandle<Vec<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_request(&mut stream).await;
            write_response(&mut stream, response).await;
            requests.push(request);
        }
        requests
    });
    (format!("http://{address}"), server)
}

async fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut request = Vec::new();
    let mut chunk = [0; 2048];
    loop {
        let read = stream.read(&mut chunk).await.unwrap();
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
        let Some(body_start) = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
        else {
            continue;
        };
        let headers = std::str::from_utf8(&request[..body_start]).unwrap();
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap_or_default())
            })
            .unwrap_or_default();
        if request.len() >= body_start + content_length {
            break;
        }
    }
    request
}

async fn write_response(stream: &mut TcpStream, response: MockResponse) {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Test Response",
    };
    let mut headers = response.headers;
    headers.push(("Content-Length".to_owned(), response.body.len().to_string()));
    headers.push(("Connection".to_owned(), "close".to_owned()));
    let headers = headers
        .iter()
        .map(|(name, value)| format!("{name}: {value}\r\n"))
        .collect::<String>();
    let status_line = format!("HTTP/1.1 {} {reason}\r\n", response.status);
    stream.write_all(status_line.as_bytes()).await.unwrap();
    stream.write_all(headers.as_bytes()).await.unwrap();
    stream.write_all(b"\r\n").await.unwrap();
    stream.write_all(&response.body).await.unwrap();
}

fn request_line(request: &[u8]) -> String {
    String::from_utf8_lossy(request)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn request_header<'a>(request: &'a [u8], name: &str) -> Option<&'a str> {
    let request = std::str::from_utf8(request).ok()?;
    request
        .lines()
        .skip(1)
        .take_while(|line| !line.is_empty())
        .find_map(|line| {
            let (header_name, value) = line.split_once(':')?;
            header_name.eq_ignore_ascii_case(name).then(|| value.trim())
        })
}

fn request_body(request: &[u8]) -> Value {
    let body_start = request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|position| position + 4)
        .unwrap();
    serde_json::from_slice(&request[body_start..]).unwrap()
}

fn config(apify_api_base: String, scrappa_api_base: String) -> Config {
    Config {
        apify_api_base,
        apify_token: "test-token".to_owned(),
        actor_run_id: "test-run".to_owned(),
        key_value_store_id: "store".to_owned(),
        dataset_id: "dataset".to_owned(),
        input_key: "INPUT".to_owned(),
        scrappa_api_base,
        scrappa_api_key: "test-key".to_owned(),
    }
}

fn mock_priced_run(max_total_charge: f64, charged_event_counts: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {
                    "actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                        "other-event": {"eventPriceUsd": 0.0002}
                    }
                }
            },
            "chargedEventCounts": charged_event_counts,
            "options": {"maxTotalChargeUsd": max_total_charge}
        }
    })
}

fn mock_job(title: &str) -> Value {
    json!({
        "refnr": "12265-399943_JB5100405-S",
        "titel": title,
        "beruf": "Softwareentwickler/-in",
        "arbeitgeber": "TechGmbH",
        "arbeitsort": {
            "ort": "Berlin",
            "plz": "10115",
            "region": "Berlin",
            "land": "Deutschland",
            "entfernung": "3",
            "koordinaten": {"lat": 52.531976, "lon": 13.386737}
        },
        "aktuelleVeroeffentlichungsdatum": "2026-03-20",
        "eintrittsdatum": "2026-04-01",
        "externeUrl": "https://www.arbeitsagentur.de/jobsuche/jobdetail/10000001"
    })
}

#[test]
fn actor_schema_keeps_the_qa_prefill_and_single_page_pagination_defaults() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    let properties = schema.get("properties").unwrap();
    let mut qa_input = Map::new();
    for (name, property) in properties.as_object().unwrap() {
        if let Some(value) = property.get("prefill").or_else(|| property.get("default")) {
            qa_input.insert(name.clone(), value.clone());
        }
    }

    assert_eq!(qa_input["was"], "Software Entwickler");
    assert_eq!(qa_input["wo"], "Berlin");
    assert_eq!(qa_input["arbeitszeit"], "vz;ho");
    assert_eq!(qa_input["page"], 1);
    assert_eq!(qa_input["size"], 25);
    assert_eq!(
        normalize_input(Some(&Value::Object(qa_input.clone())))["arbeitszeit"],
        "vz;ho"
    );
    assert_eq!(
        build_jobs_params(&normalize_input(Some(&Value::Object(qa_input))))["page"],
        1
    );

    let actor: Value = serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();
    assert_eq!(
        actor.pointer("/defaultRunOptions/timeoutSecs"),
        Some(&json!(240))
    );
    assert_eq!(actor.pointer("/resources/memoryMbytes"), Some(&json!(128)));
    assert!(actor.get("disable").is_none());
}

#[test]
fn input_normalization_matches_defaults_and_preserves_false_filters() {
    assert_eq!(normalize_input(None), default_input());
    assert_eq!(
        normalize_input(Some(&json!({"helloWorld": 123}))),
        default_input()
    );

    let input = normalize_input(Some(&json!({
        "wo": " Hamburg ",
        "arbeitszeit": " VZ ; HO ",
        "zeitarbeit": false,
        "size": 10,
        "ignored": "field"
    })));
    assert_eq!(input["was"], "Software Entwickler");
    assert_eq!(input["wo"], "Hamburg");
    assert_eq!(input["arbeitszeit"], "vz;ho");
    assert_eq!(input["zeitarbeit"], false);
    assert_eq!(input["size"], 10);
    assert!(!input.contains_key("ignored"));
}

#[test]
fn query_params_keep_false_and_omit_empty_or_null_values() {
    let input = normalize_input(Some(&json!({
        "was": "Software Entwickler",
        "zeitarbeit": false,
        "pav": true,
        "berufsfeld": " ",
        "arbeitgeber": null,
        "page": 3,
        "size": 50
    })));
    let params = build_jobs_params(&input);
    let url = build_jobs_url("https://example.test/api", &params).unwrap();
    let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(query["was"], "Software Entwickler");
    assert_eq!(query["zeitarbeit"], "false");
    assert_eq!(query["pav"], "true");
    assert_eq!(query["page"], "3");
    assert_eq!(query["size"], "50");
    assert!(!query.contains_key("berufsfeld"));
    assert!(!query.contains_key("arbeitgeber"));
}

#[test]
fn response_shapes_and_metadata_match_the_node_actor() {
    let nested = json!({"data": {"stellenangebote": [mock_job("Nested")], "page": 2}});
    assert_eq!(get_jobs(&nested).len(), 1);
    assert_eq!(get_metadata(&nested)["page"], 2);

    let top_level = json!({"stellenangebote": [mock_job("Top-level")], "size": 10});
    assert_eq!(get_jobs(&top_level)[0]["titel"], "Top-level");
    assert_eq!(get_metadata(&top_level)["size"], 10);
    assert!(get_jobs(
        &json!({"data": {"stellenangebote": []}, "stellenangebote": [mock_job("Fallback")]})
    )
    .is_empty());
}

#[test]
fn dataset_job_preserves_raw_fields_and_adds_table_aliases() {
    let job = mock_job("Software Entwickler (m/w/d)");
    assert_eq!(
        to_dataset_job(&job),
        json!({
            "refnr": "12265-399943_JB5100405-S",
            "titel": "Software Entwickler (m/w/d)",
            "beruf": "Softwareentwickler/-in",
            "arbeitgeber": "TechGmbH",
            "arbeitsort": {
                "ort": "Berlin", "plz": "10115", "region": "Berlin", "land": "Deutschland",
                "entfernung": "3", "koordinaten": {"lat": 52.531976, "lon": 13.386737}
            },
            "aktuelleVeroeffentlichungsdatum": "2026-03-20",
            "eintrittsdatum": "2026-04-01",
            "externeUrl": "https://www.arbeitsagentur.de/jobsuche/jobdetail/10000001",
            "title": "Software Entwickler (m/w/d)",
            "occupation": "Softwareentwickler/-in",
            "company_name": "TechGmbH",
            "location_formatted": "10115, Berlin, Berlin, Deutschland",
            "location_city": "Berlin",
            "postal_code": "10115",
            "region": "Berlin",
            "country": "Deutschland",
            "published_date": "2026-03-20",
            "start_date": "2026-04-01",
            "job_url": "https://www.arbeitsagentur.de/jobsuche/jobdetail/10000001",
            "reference_number": "12265-399943_JB5100405-S",
            "distance_km": "3",
            "latitude": 52.531976,
            "longitude": 13.386737
        })
    );
    assert_eq!(
        get_formatted_location(Some(&json!("Berlin"))).as_deref(),
        Some("Berlin")
    );
    assert_eq!(get_formatted_location(Some(&json!({"ort": "  "}))), None);
    assert_eq!(
        to_dataset_job(&json!({"titel": "No location"}))["latitude"],
        Value::Null
    );
    assert_eq!(
        to_dataset_job(&json!({"arbeitsort": {"entfernung": false}}))["distance_km"],
        false
    );
}

#[test]
fn scrappa_retry_rules_and_retry_after_match_the_node_client() {
    for status in [408, 429, 500, 502, 503, 504] {
        assert!(ScrappaFailure::Api {
            status,
            message: String::new(),
            retry_after_ms: None,
        }
        .is_retryable());
    }
    assert!(ScrappaFailure::Timeout.is_retryable());
    assert!(!ScrappaFailure::Api {
        status: 401,
        message: String::new(),
        retry_after_ms: None,
    }
    .is_retryable());
    assert!(!ScrappaFailure::Transport("network error".to_owned()).is_retryable());
    assert!(!ScrappaFailure::InvalidJson("bad JSON".to_owned()).is_retryable());

    assert_eq!(get_retry_delay_ms(1, 0, None, 20_000), 2_000);
    assert_eq!(get_retry_delay_ms(2, 250, None, 20_000), 4_250);
    assert_eq!(get_retry_delay_ms(1, 0, Some(15_000), 20_000), 15_000);
    assert_eq!(get_retry_delay_ms(1, 0, Some(60_000), 20_000), 20_000);
    assert_eq!(parse_retry_after_ms("0.5"), Some(500));
    assert_eq!(parse_retry_after_ms("not a date"), None);
    assert_eq!(get_retry_delay_ms(3, 999, Some(20_000), 20_000), 20_000);
}

#[test]
fn retry_budget_leaves_the_actor_completion_reserve() {
    let maximum_request_ms = duration_millis(SCRAPPA_REQUEST_DEADLINE);
    let configured_maximum_ms = maximum_request_ms + ACTOR_COMPLETION_RESERVE_MS;
    assert_eq!(maximum_request_ms, 180_000);
    assert_eq!(configured_maximum_ms, 210_000);
    assert!(configured_maximum_ms < ACTOR_TIMEOUT_MS);
}

#[test]
fn scrappa_error_messages_keep_json_details_and_text_fallbacks() {
    assert_eq!(
        scrappa_error_message(StatusCode::NOT_FOUND, br#"{"message":"Not found"}"#),
        "Not found"
    );
    assert_eq!(
        scrappa_error_message(
            StatusCode::UNPROCESSABLE_ENTITY,
            br#"{"message":"Invalid","errors":{"was":["bad","required"]}}"#
        ),
        "Invalid - was: bad, required"
    );
    assert_eq!(
        scrappa_error_message(StatusCode::SERVICE_UNAVAILABLE, b""),
        "Service Unavailable"
    );
    assert_eq!(
        scrappa_error_message(StatusCode::SERVICE_UNAVAILABLE, b"  Busy\n now "),
        "Busy now"
    );
}

#[test]
fn zero_max_total_charge_is_unbounded() {
    assert_eq!(
        affordable_dataset_items(&mock_priced_run(0.0, json!({})), 3).unwrap(),
        3
    );
}

#[test]
fn omitted_max_total_charge_is_unbounded() {
    let mut run = mock_priced_run(0.0, json!({}));
    run["data"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("maxTotalChargeUsd");
    assert_eq!(affordable_dataset_items(&run, 3).unwrap(), 3);
}

#[test]
fn null_max_total_charge_is_unbounded() {
    let mut run = mock_priced_run(0.0, json!({}));
    run["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
    assert_eq!(affordable_dataset_items(&run, 3).unwrap(), 3);
}

#[test]
fn positive_max_total_charge_counts_existing_event_charges() {
    let run = mock_priced_run(0.0006, json!({"other-event": 1}));
    assert_eq!(affordable_dataset_items(&run, 3).unwrap(), 1);

    assert_eq!(
        affordable_dataset_items(&mock_priced_run(0.0006, json!({})), 3).unwrap(),
        2
    );
}

#[test]
fn free_dataset_items_fit_a_zero_charge_limit() {
    let free_items = json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0}
                }}
            },
            "chargedEventCounts": {},
            "options": {"maxTotalChargeUsd": 0.0}
        }
    });
    assert_eq!(affordable_dataset_items(&free_items, 3).unwrap(), 3);
}

#[test]
fn pay_per_event_budget_rejects_non_ppe_runs() {
    assert!(affordable_dataset_items(&json!({"data": {}}), 1)
        .unwrap_err()
        .to_string()
        .contains("pay-per-event"));
}

#[tokio::test]
async fn run_preserves_the_prefill_request_and_outputs_budgeted_rows_and_raw_response() {
    let input = json!({
        "was": "Software Entwickler",
        "wo": "Berlin",
        "umkreis": 25,
        "arbeitszeit": "vz;ho",
        "page": 1,
        "size": 25
    });
    let response = json!({
        "success": true,
        "data": {
            "stellenangebote": [mock_job("Job 1"), mock_job("Job 2")],
            "maxErgebnisse": 100,
            "page": 1,
            "size": 25,
            "facetten": {"beruf": []}
        }
    });
    let (apify_base, apify_server) = start_mock_server(vec![
        MockResponse::json(200, input),
        MockResponse::json(200, mock_priced_run(0.0006, json!({"other-event": 1}))),
        MockResponse::json(200, Value::Null),
        MockResponse::json(200, response.clone()),
    ])
    .await;
    let (scrappa_base, scrappa_server) =
        start_mock_server(vec![MockResponse::json(200, response.clone())]).await;

    run_actor(config(apify_base, format!("{scrappa_base}/api")))
        .await
        .unwrap();

    let apify_requests = apify_server.await.unwrap();
    assert_eq!(apify_requests.len(), 4);
    assert!(request_line(&apify_requests[0])
        .starts_with("GET /v2/key-value-stores/store/records/INPUT HTTP/1.1"));
    assert!(request_header(&apify_requests[0], "Authorization")
        .is_some_and(|value| value == "Bearer test-token"));
    assert_eq!(
        request_line(&apify_requests[1]),
        "GET /v2/actor-runs/test-run HTTP/1.1"
    );
    assert_eq!(
        request_line(&apify_requests[2]),
        "POST /v2/datasets/dataset/items HTTP/1.1"
    );
    assert_eq!(request_body(&apify_requests[2])[0]["title"], "Job 1");
    assert_eq!(
        request_line(&apify_requests[3]),
        "PUT /v2/key-value-stores/store/records/OUTPUT HTTP/1.1"
    );
    assert_eq!(request_body(&apify_requests[3]), response);

    let scrappa_requests = scrappa_server.await.unwrap();
    assert_eq!(scrappa_requests.len(), 1);
    assert!(request_line(&scrappa_requests[0]).starts_with("GET /api/arbeitsagentur/jobs?"));
    let scrappa_request = String::from_utf8_lossy(&scrappa_requests[0]);
    let request_url = Url::parse(&format!(
        "http://localhost{}",
        request_line(&scrappa_requests[0])
            .split_whitespace()
            .nth(1)
            .unwrap()
    ))
    .unwrap();
    let query: std::collections::HashMap<_, _> = request_url.query_pairs().into_owned().collect();
    assert_eq!(query["was"], "Software Entwickler");
    assert_eq!(query["wo"], "Berlin");
    assert_eq!(query["arbeitszeit"], "vz;ho");
    assert_eq!(query["page"], "1");
    assert_eq!(query["size"], "25");
    assert_eq!(
        request_header(&scrappa_requests[0], "X-API-Key"),
        Some("test-key")
    );
    assert!(scrappa_request.contains(SCRAPPA_USER_AGENT));
}

#[tokio::test]
async fn dataset_publisher_checks_pricing_and_does_not_retry_append_posts() {
    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(200, mock_priced_run(0.0003, json!({}))),
        MockResponse::json(503, json!({"message": "Unavailable"})),
    ])
    .await;
    let config = config(base_url, SCRAPPA_API_DEFAULT.to_owned());
    let apify = ApifyClient::new(Client::new(), &config);
    let rows = vec![json!({"title": "first"}), json!({"title": "second"})];

    let error = apify.push_dataset_items(&rows).await.unwrap_err();
    assert!(error
        .to_string()
        .contains("Apify dataset item publication failed (503)"));
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(request_line(&requests[1]).starts_with("POST /v2/datasets/dataset/items HTTP/1.1"));
    assert_eq!(request_body(&requests[1]), json!([{"title": "first"}]));
}

#[tokio::test]
async fn dataset_publisher_sends_affordable_rows_in_one_array_request() {
    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(200, mock_priced_run(0.0006, json!({}))),
        MockResponse::json(201, Value::Null),
    ])
    .await;
    let config = config(base_url, SCRAPPA_API_DEFAULT.to_owned());
    let apify = ApifyClient::new(Client::new(), &config);
    let rows = vec![
        json!({"title": "first"}),
        json!({"title": "second"}),
        json!({"title": "third"}),
    ];

    assert_eq!(apify.push_dataset_items(&rows).await.unwrap(), 2);
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        request_line(&requests[1]),
        "POST /v2/datasets/dataset/items HTTP/1.1"
    );
    assert_eq!(
        request_body(&requests[1]),
        json!([{"title": "first"}, {"title": "second"}])
    );
}

#[tokio::test]
async fn scrappa_retries_only_transient_http_responses_and_honors_retry_after() {
    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(503, json!({"message": "temporary"})).with_header("Retry-After", "0"),
        MockResponse::json(
            200,
            json!({"success": true, "data": {"stellenangebote": []}}),
        ),
    ])
    .await;
    let params = build_jobs_params(&normalize_input(None));
    let client = ScrappaClient {
        http: Client::new(),
        base_url: base_url.clone(),
        api_key: "test-key".to_owned(),
        policy: ScrappaRetryPolicy {
            attempts: 2,
            request_timeout: Duration::from_secs(1),
            max_retry_delay: Duration::ZERO,
            request_deadline: Duration::from_secs(2),
        },
    };

    assert_eq!(client.get_jobs(&params).await.unwrap()["success"], true);
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests
        .iter()
        .all(|request| request_line(request).starts_with("GET /arbeitsagentur/jobs?")));
}

#[tokio::test]
async fn scrappa_request_timeout_is_reported_as_retryable_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let _ = read_request(&mut stream).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    });
    let client = ScrappaClient {
        http: Client::new(),
        base_url: format!("http://{address}"),
        api_key: "test-key".to_owned(),
        policy: ScrappaRetryPolicy {
            attempts: 1,
            request_timeout: Duration::from_millis(20),
            max_retry_delay: Duration::ZERO,
            request_deadline: Duration::from_millis(50),
        },
    };

    let error = client
        .get_jobs(&build_jobs_params(&normalize_input(None)))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("timed out after 30000ms"));
    server.await.unwrap();
}
