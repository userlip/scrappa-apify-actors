use super::*;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread,
};

#[derive(Debug)]
struct RecordedRequest {
    method_and_path: String,
    headers: String,
    body: String,
}

fn value(source: &str) -> Value {
    serde_json::from_str(source).unwrap()
}

fn request_config(address: SocketAddr) -> ActorConfig {
    ActorConfig {
        apify_api_base_url: Url::parse(&format!("http://{address}")).unwrap(),
        scrappa_api_base_url: Url::parse(&format!("http://{address}/api")).unwrap(),
        default_key_value_store_id: "store-test".to_owned(),
        default_dataset_id: "dataset-test".to_owned(),
        actor_run_id: "run-test".to_owned(),
        input_key: "INPUT".to_owned(),
        apify_token: "actor-token".to_owned(),
        scrappa_api_key: "scrappa-key".to_owned(),
    }
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn read_request(stream: &mut TcpStream) -> RecordedRequest {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut buffer = Vec::new();
    let mut chunk = [0; 4096];
    let (header_end, content_length) = loop {
        let bytes_read = stream.read(&mut chunk).unwrap();
        assert_ne!(bytes_read, 0, "request ended before its headers arrived");
        buffer.extend_from_slice(&chunk[..bytes_read]);
        if let Some(header_end) = find_header_end(&buffer) {
            let headers = String::from_utf8_lossy(&buffer[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            if buffer.len() >= header_end + 4 + content_length {
                break (header_end, content_length);
            }
        }
    };
    let header_text = String::from_utf8_lossy(&buffer[..header_end]).into_owned();
    let mut lines = header_text.lines();
    let method_and_path = lines.next().unwrap_or_default().to_owned();
    let headers = lines.collect::<Vec<_>>().join("\n");
    let body = String::from_utf8_lossy(&buffer[header_end + 4..header_end + 4 + content_length])
        .into_owned();
    RecordedRequest {
        method_and_path,
        headers,
        body,
    }
}

fn mock_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
    stream.write_all(response.as_bytes()).unwrap();
}

fn start_mock_server(
    responses: Vec<(String, String)>,
) -> (SocketAddr, thread::JoinHandle<Vec<RecordedRequest>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        responses
            .into_iter()
            .map(|(status, body)| {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_request(&mut stream);
                mock_response(&mut stream, &status, &body);
                request
            })
            .collect()
    });
    (address, server)
}

fn start_ambiguous_dataset_write_server() -> (
    SocketAddr,
    thread::JoinHandle<(Vec<RecordedRequest>, usize)>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        let mut committed_writes = 0;
        let mut last_request_at = None;
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let request = read_request(&mut stream);
                    assert!(request
                        .method_and_path
                        .starts_with("POST /v2/datasets/dataset-test/items "));

                    // Treat each received POST as persisted before sending its response.
                    committed_writes += 1;
                    let (status, body) = if committed_writes == 1 {
                        (
                            "503 Service Unavailable",
                            "write persisted but response failed",
                        )
                    } else {
                        ("201 Created", "")
                    };
                    mock_response(&mut stream, status, body);
                    requests.push(request);
                    last_request_at = Some(std::time::Instant::now());
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if last_request_at.is_some_and(|last_request_at| {
                        last_request_at.elapsed() >= Duration::from_millis(750)
                    }) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("mock server failed to accept a request: {error}"),
            }
        }
        (requests, committed_writes)
    });
    (address, server)
}

fn pricing_body(max_charge: Option<f64>, counts: Value) -> String {
    let max_charge = max_charge.map(Value::from).unwrap_or(Value::Null);
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {
                    "actorChargeEvents": {
                        "apify-default-dataset-item": { "eventPriceUsd": 0.0003 },
                        "start-event": { "eventPriceUsd": 0.1 }
                    }
                }
            },
            "options": { "maxTotalChargeUsd": max_charge },
            "chargedEventCounts": counts
        }
    })
    .to_string()
}

fn input_response(input: &str) -> (String, String) {
    ("200 OK".to_owned(), input.to_owned())
}

fn posts_response(posts: &str, pagination: &str) -> (String, String) {
    (
        "200 OK".to_owned(),
        format!(r#"{{"code":0,"data":{{"posts":{posts},{pagination}}},"processed_time":99}}"#),
    )
}

fn success_response() -> (String, String) {
    ("201 Created".to_owned(), String::new())
}

#[test]
fn builds_requests_for_multiple_and_legacy_ids() {
    assert_eq!(
        build_music_requests(&value(
            r#"{"musicIds":[" 7001 ","7002","7001"],"music_id":"7003","count":10,"cursor":" 0 "}"#
        ))
        .unwrap(),
        vec![
            MusicRequest {
                music_id: "7001".to_owned(),
                count: Some(10),
                cursor: Some("0".to_owned()),
            },
            MusicRequest {
                music_id: "7002".to_owned(),
                count: Some(10),
                cursor: Some("0".to_owned()),
            },
        ]
    );
    assert_eq!(
        build_music_requests(&value(r#"{"musicIds":[],"music_id":7003}"#)).unwrap(),
        vec![MusicRequest {
            music_id: "7003".to_owned(),
            count: None,
            cursor: None,
        }]
    );
}

#[test]
fn uses_music_id_fallback_when_music_ids_is_not_an_array() {
    assert_eq!(
        build_music_requests(&value(r#"{"musicIds":"7001","music_id":"7002"}"#)).unwrap(),
        vec![MusicRequest {
            music_id: "7002".to_owned(),
            count: None,
            cursor: None,
        }]
    );
}

#[test]
fn rejects_missing_invalid_and_oversized_music_ids() {
    assert!(
        build_music_requests(&value(r#"{"musicIds":[],"music_id":""}"#))
            .unwrap_err()
            .to_string()
            .contains("At least one TikTok music_id")
    );
    assert!(normalize_music_id("track-name")
        .unwrap_err()
        .to_string()
        .contains("digits only"));
    assert!(normalize_music_id(&"1".repeat(101))
        .unwrap_err()
        .to_string()
        .contains("100 digits or fewer"));
}

#[test]
fn invalid_count_and_cursor_values_fall_back_to_scrappa_defaults() {
    assert_eq!(
        build_music_requests(&value(
            r#"{"music_id":"7001","count":51,"cursor":9007199254740992}"#
        ))
        .unwrap(),
        vec![MusicRequest {
            music_id: "7001".to_owned(),
            count: None,
            cursor: None,
        }]
    );
    assert_eq!(
        build_music_requests(&value(r#"{"music_id":"7001","cursor":123}"#)).unwrap()[0].cursor,
        Some("123".to_owned())
    );
}

#[test]
fn extracts_post_variants_and_pagination_fields() {
    let posts = value(r#"[{"aweme_id":"1"}]"#);
    assert_eq!(extract_posts(Some(&posts)), vec![&posts[0]]);
    let response = value(
        r#"{"videos":[{"aweme_id":"2"}],"hasMore":true,"has_more":false,"cursor":"10","max_cursor":"20"}"#,
    );
    assert_eq!(extract_posts(Some(&response)), vec![&response["videos"][0]]);
    assert_eq!(
        extract_pagination(Some(&response)),
        (true, Value::String("10".to_owned()))
    );
    let fallback = value(r#"{"aweme_list":[{"aweme_id":"3"}],"has_more":true,"max_cursor":"30"}"#);
    assert_eq!(
        extract_pagination(Some(&fallback)),
        (true, Value::String("30".to_owned()))
    );
    assert!(extract_posts(Some(&Value::Null)).is_empty());
}

#[test]
fn numeric_zero_is_the_only_success_scrappa_code() {
    assert!(validate_scrappa_code(&value(r#"{"code":0}"#)).is_ok());
    assert!(validate_scrappa_code(&value(r#"{"code":"0"}"#))
        .unwrap_err()
        .to_string()
        .contains("code 0"));
}

#[test]
fn positive_spending_limit_accounts_for_other_charged_events_and_saved_rows() {
    let run = value(&pricing_body(Some(0.101), json!({ "start-event": 1 })));
    assert_eq!(affordable_dataset_items(&run, 5, 0).unwrap(), 3);
    assert_eq!(affordable_dataset_items(&run, 1, 3).unwrap(), 0);
}

#[test]
fn allows_non_ppe_runs_without_pricing_event_details() {
    let run = json!({ "data": { "pricingInfo": { "pricingModel": "FREE" } } });
    assert_eq!(affordable_dataset_items(&run, 50, 0).unwrap(), 50);
}

#[test]
fn treats_missing_null_and_zero_spending_limits_as_uncapped() {
    let mut missing_limit = value(&pricing_body(Some(1.0), json!({})));
    missing_limit["data"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("maxTotalChargeUsd");
    assert_eq!(affordable_dataset_items(&missing_limit, 50, 0).unwrap(), 50);

    let uncapped = value(&pricing_body(None, json!({})));
    assert_eq!(affordable_dataset_items(&uncapped, 50, 0).unwrap(), 50);

    let zero_limit = value(&pricing_body(Some(0.0), json!({})));
    assert_eq!(affordable_dataset_items(&zero_limit, 50, 0).unwrap(), 50);

    let mut free_event = value(&pricing_body(Some(0.0), json!({})));
    free_event["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
        [DEFAULT_DATASET_ITEM_EVENT]["eventPriceUsd"] = json!(0);
    assert_eq!(affordable_dataset_items(&free_event, 2, 0).unwrap(), 2);
}

#[test]
fn preserves_apify_prefill_and_schema_contract() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    assert_eq!(
        schema["properties"]["musicIds"]["prefill"],
        json!(["7002634556977908485"])
    );
    assert!(schema["properties"]["music_id"].get("prefill").is_none());
    assert_eq!(schema["properties"]["cursor"]["type"], "string");
    assert_eq!(schema["properties"]["cursor"]["editor"], "textfield");
}

#[tokio::test]
async fn fetches_posts_with_existing_auth_and_pagination_query_and_writes_output() {
    let input = r#"{"musicIds":["7001"],"count":2,"cursor":"0"}"#;
    let responses = vec![
        input_response(input),
        posts_response(
            r#"[{"aweme_id":"1"},{"aweme_id":"2"}]"#,
            r#""hasMore":true,"cursor":"20""#,
        ),
        ("200 OK".to_owned(), pricing_body(Some(1.0), json!({}))),
        success_response(),
        success_response(),
    ];
    let (address, server) = start_mock_server(responses);
    let config = request_config(address);
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    run_actor(&client, &client, &config).await.unwrap();
    let requests = server.join().unwrap();

    assert_eq!(requests.len(), 5);
    assert!(requests[0]
        .method_and_path
        .starts_with("GET /v2/key-value-stores/store-test/records/INPUT "));
    assert!(requests[1]
        .method_and_path
        .contains("/api/tiktok/music/posts?music_id=7001&count=2&cursor=0"));
    assert!(requests[1].headers.contains("x-api-key: scrappa-key"));
    assert!(requests[2]
        .method_and_path
        .starts_with("GET /v2/actor-runs/run-test "));
    assert!(requests[3]
        .method_and_path
        .starts_with("POST /v2/datasets/dataset-test/items "));
    assert!(requests[3]
        .headers
        .contains("authorization: Bearer actor-token"));
    let rows: Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert_eq!(rows[0]["request_music_id"], "7001");
    assert!(requests[4]
        .method_and_path
        .starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT "));
    let summary: Value = serde_json::from_str(&requests[4].body).unwrap();
    assert_eq!(summary["music_ids_processed"], 1);
    assert_eq!(summary["posts_extracted"], 2);
    assert_eq!(summary["results"][0]["posts_returned"], 2);
    assert_eq!(summary["results"][0]["has_next_page"], true);
    assert_eq!(summary["results"][0]["next_cursor"], "20");
}

#[tokio::test]
async fn caps_dataset_writes_at_the_ppe_budget_and_stops_before_the_next_music_id() {
    let responses = vec![
        input_response(r#"{"musicIds":["7001","7002"],"count":2}"#),
        posts_response(
            r#"[{"aweme_id":"1"},{"aweme_id":"2"}]"#,
            r#""hasMore":false"#,
        ),
        ("200 OK".to_owned(), pricing_body(Some(0.0003), json!({}))),
        success_response(),
        success_response(),
    ];
    let (address, server) = start_mock_server(responses);
    let config = request_config(address);
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    run_actor(&client, &client, &config).await.unwrap();
    let requests = server.join().unwrap();

    assert_eq!(requests.len(), 5);
    let rows: Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["aweme_id"], "1");
    let summary: Value = serde_json::from_str(&requests[4].body).unwrap();
    assert_eq!(summary["posts_extracted"], 1);
    assert_eq!(summary["charge_limit_reached"], true);
    assert_eq!(summary["results"][0]["posts_returned"], 2);
    assert_eq!(summary["results"][0]["charge_limit_reached"], true);
}

#[tokio::test]
async fn exact_budget_exhaustion_stops_before_requesting_another_music_id() {
    let responses = vec![
        input_response(r#"{"musicIds":["7001","7002"],"count":2}"#),
        posts_response(
            r#"[{"aweme_id":"1"},{"aweme_id":"2"}]"#,
            r#""hasMore":false"#,
        ),
        ("200 OK".to_owned(), pricing_body(Some(0.0006), json!({}))),
        success_response(),
        success_response(),
    ];
    let (address, server) = start_mock_server(responses);
    let config = request_config(address);
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    run_actor(&client, &client, &config).await.unwrap();
    let requests = server.join().unwrap();

    assert_eq!(requests.len(), 5);
    let summary: Value = serde_json::from_str(&requests[4].body).unwrap();
    assert_eq!(summary["posts_extracted"], 2);
    assert_eq!(summary["charge_limit_reached"], true);
    assert_eq!(summary["music_ids_processed"], 1);
}

#[tokio::test]
async fn scrappa_error_fails_without_dataset_or_output_writes_or_retries() {
    let responses = vec![
        input_response(r#"{"music_id":"7001"}"#),
        (
            "503 Service Unavailable".to_owned(),
            r#"{"message":"Try again later","errors":{"music_id":["busy"]}}"#.to_owned(),
        ),
    ];
    let (address, server) = start_mock_server(responses);
    let config = request_config(address);
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let error = run_actor(&client, &client, &config).await.unwrap_err();
    let requests = server.join().unwrap();

    assert!(error
        .to_string()
        .contains("Scrappa API error (503): Try again later - music_id: busy"));
    assert_eq!(requests.len(), 2);
}

#[tokio::test]
async fn dataset_write_error_fails_without_output_write() {
    let responses = vec![
        input_response(r#"{"music_id":"7001"}"#),
        posts_response(r#"[{"aweme_id":"1"}]"#, r#""hasMore":false"#),
        ("200 OK".to_owned(), pricing_body(Some(1.0), json!({}))),
        (
            "400 Bad Request".to_owned(),
            "dataset write failed".to_owned(),
        ),
    ];
    let (address, server) = start_mock_server(responses);
    let config = request_config(address);
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let error = run_actor(&client, &client, &config).await.unwrap_err();
    let requests = server.join().unwrap();

    assert!(error.to_string().contains("Apify dataset write failed"));
    assert_eq!(requests.len(), 4);
    assert!(requests
        .iter()
        .all(|request| !request.method_and_path.contains("/records/OUTPUT")));
}

#[tokio::test]
async fn does_not_retry_a_dataset_write_after_an_ambiguous_server_error() {
    let (address, server) = start_ambiguous_dataset_write_server();
    let config = request_config(address);
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let mut budget = DatasetBudget {
        run: Some(value(&pricing_body(Some(1.0), json!({})))),
        saved_rows: 0,
    };

    let result =
        push_dataset_items(&client, &config, &mut budget, &[json!({ "aweme_id": "1" })]).await;
    let (requests, committed_writes) = server.join().unwrap();

    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Apify dataset write failed"));
    assert_eq!(requests.len(), 1);
    assert_eq!(committed_writes, 1);
}

#[tokio::test]
async fn apify_api_retries_server_errors_with_a_rebuilt_request() {
    let (address, server) = start_mock_server(vec![
        (
            "503 Service Unavailable".to_owned(),
            r#"{"message":"retry"}"#.to_owned(),
        ),
        ("200 OK".to_owned(), r#"{"ok":true}"#.to_owned()),
    ]);
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let url = Url::parse(&format!("http://{address}/v2/test")).unwrap();
    let response = send_apify_request(|| client.get(url.clone()), "test Apify request")
        .await
        .unwrap();

    assert_eq!(
        response_json(response, "test Apify request").await.unwrap()["ok"],
        true
    );
    assert_eq!(server.join().unwrap().len(), 2);
}
