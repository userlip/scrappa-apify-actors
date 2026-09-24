use crate::{
    apify::{affordable_dataset_items, apify_get, ActorConfig},
    run_actor,
    tiktok_video::{
        build_video_url, dataset_item, extract_video, format_video_lookup_for_log,
        require_video_lookup, resolve_video_requests, VideoRequest, VIDEO_LOOKUP_ERROR,
    },
};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread,
    time::Duration,
};
use url::Url;

const VIDEO_URL: &str = "https://www.tiktok.com/@tiktok/video/7568510388342443294";
const SECOND_VIDEO_URL: &str = "https://www.tiktok.com/@tiktok/video/1234567890123456789";

#[derive(Debug)]
struct CapturedRequest {
    method_and_path: String,
    headers: HashMap<String, String>,
    body: String,
}

fn request_config(address: SocketAddr) -> ActorConfig {
    let base_url = Url::parse(&format!("http://{address}")).unwrap();
    ActorConfig {
        apify_api_base_url: base_url.clone(),
        scrappa_api_base_url: Url::parse(&format!("http://{address}/api")).unwrap(),
        key_value_store_id: "store-test".to_owned(),
        dataset_id: "dataset-test".to_owned(),
        actor_run_id: "run-test".to_owned(),
        input_key: "INPUT".to_owned(),
        apify_token: "test-apify-token".to_owned(),
        scrappa_api_key: "test-scrappa-key".to_owned(),
    }
}

fn pricing_run(max_charge: f64, counts: Value) -> String {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0002},
                    "other-event": {"eventPriceUsd": 0.0001}
                }}
            },
            "options": {"maxTotalChargeUsd": max_charge},
            "chargedEventCounts": counts
        }
    })
    .to_string()
}

fn start_mock_server(
    responses: Vec<(String, String)>,
) -> (SocketAddr, thread::JoinHandle<Vec<CapturedRequest>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = thread::spawn(move || {
        let mut captured = Vec::new();
        for (status, body) in responses {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "timed out waiting for actor request"
                        );
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("mock server failed to accept connection: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let request = read_request(&mut stream);
            write_response(&mut stream, &status, &body);
            captured.push(request);
        }
        captured
    });
    (address, server)
}

fn read_request(stream: &mut TcpStream) -> CapturedRequest {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    let header_end = loop {
        let read = stream.read(&mut buffer).unwrap();
        assert_ne!(read, 0, "client closed before sending a complete request");
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index;
        }
    };

    let headers_text = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
    let mut lines = headers_text.split("\r\n");
    let method_and_path = lines.next().unwrap_or_default().to_owned();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect::<HashMap<_, _>>();
    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let body_start = header_end + 4;
    let body_end = body_start + content_length;
    while bytes.len() < body_end {
        let read = stream.read(&mut buffer).unwrap();
        assert_ne!(
            read, 0,
            "client closed before sending the full request body"
        );
        bytes.extend_from_slice(&buffer[..read]);
    }

    CapturedRequest {
        method_and_path,
        headers,
        body: String::from_utf8(bytes[body_start..body_end].to_vec()).unwrap(),
    }
}

fn write_response(stream: &mut TcpStream, status: &str, body: &str) {
    write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    stream.flush().unwrap();
}

#[test]
fn accepts_video_ids_and_supported_tiktok_urls() {
    for value in [
        VIDEO_URL,
        "https://www.tiktok.com/@tiktok/video/7568510388342443294?lang=en",
        "https://www.tiktok.com/@tiktok/photo/7568510388342443294",
        "https://vm.tiktok.com/ZGeqDY4yL/",
        "https://vt.tiktok.com/ZGeqDY4yL/",
        "https://tiktok.com/t/ZGeqDY4yL/",
        "7568510388342443294",
    ] {
        assert!(require_video_lookup(value).is_ok(), "should accept {value}");
    }
}

#[test]
fn rejects_wrong_host_protocol_and_non_content_paths() {
    assert_eq!(
        require_video_lookup("https://example.com/@tiktok/video/7568510388342443294"),
        Err("A TikTok URL is required".to_owned())
    );
    assert_eq!(
        require_video_lookup("http://www.tiktok.com/@tiktok/video/7568510388342443294"),
        Err("A TikTok URL must use HTTPS".to_owned())
    );
    for value in [
        "https://www.tiktok.com/@tiktok",
        "https://www.tiktok.com/tag/example",
        "https://www.tiktok.com/privacy",
        "https://www.tiktok.com/ZGeqDY4yL",
        "https://www.tiktok.com/@tiktok/video/not-a-number",
    ] {
        assert!(
            require_video_lookup(value).is_err(),
            "should reject {value}"
        );
    }
}

#[test]
fn batch_input_keeps_invalid_strings_as_error_requests_and_skips_other_values() {
    let input = json!({"urls": [VIDEO_URL, 123, " ", "not-a-url"]});
    let requests = resolve_video_requests(&input).unwrap();
    assert_eq!(
        requests,
        vec![
            VideoRequest {
                url: VIDEO_URL.to_owned(),
                validation_error: None,
            },
            VideoRequest {
                url: "not-a-url".to_owned(),
                validation_error: Some(VIDEO_LOOKUP_ERROR.to_owned()),
            },
        ]
    );
}

#[test]
fn batch_urls_take_precedence_and_keep_duplicates() {
    let input = json!({"urls": [VIDEO_URL, VIDEO_URL], "url": SECOND_VIDEO_URL});
    let requests = resolve_video_requests(&input).unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| request.url == VIDEO_URL));
}

#[test]
fn legacy_url_is_used_when_batch_input_has_no_valid_strings() {
    let input = json!({"urls": [null, 42], "url": SECOND_VIDEO_URL});
    let requests = resolve_video_requests(&input).unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url, SECOND_VIDEO_URL);
}

#[test]
fn invalid_legacy_url_fails_before_lookup() {
    assert!(resolve_video_requests(&json!({"url": "not-a-url"})).is_err());
    assert!(resolve_video_requests(&Value::Null).is_err());
}

#[test]
fn hd_query_is_only_sent_when_true_and_url_values_are_encoded() {
    let base = Url::parse("https://scrappa.co/api").unwrap();
    let normal = build_video_url(&base, VIDEO_URL, false).unwrap();
    let hd = build_video_url(&base, VIDEO_URL, true).unwrap();
    assert_eq!(normal.path(), "/api/tiktok/video");
    assert_eq!(normal.query_pairs().count(), 1);
    assert_eq!(normal.query_pairs().find(|(key, _)| key == "hd"), None);
    assert_eq!(
        hd.query_pairs().find(|(key, _)| key == "hd").unwrap().1,
        "1"
    );
    assert_eq!(
        hd.query_pairs().find(|(key, _)| key == "url").unwrap().1,
        VIDEO_URL
    );
}

#[test]
fn log_format_removes_query_and_fragment_and_labels_ids() {
    assert_eq!(
        format_video_lookup_for_log(&format!("{VIDEO_URL}?token=secret#comments")).unwrap(),
        VIDEO_URL
    );
    assert_eq!(
        format_video_lookup_for_log("7568510388342443294").unwrap(),
        "video_id:7568510388342443294"
    );
}

#[test]
fn extracts_first_video_and_treats_empty_data_as_not_found() {
    assert_eq!(
        extract_video(
            Some(&json!([{"aweme_id":"first"}, {"aweme_id":"second"}])),
            VIDEO_URL
        ),
        Some(json!({"aweme_id":"first"}))
    );
    assert_eq!(extract_video(Some(&json!([])), VIDEO_URL), None);
    assert_eq!(extract_video(Some(&Value::Null), VIDEO_URL), None);
}

#[test]
fn dataset_rows_preserve_upstream_fields_and_override_request_metadata() {
    let response = json!({"processed_time": 1.25});
    let item = dataset_item(
        Some(json!({"aweme_id":"123", "request_url":"upstream"})),
        VIDEO_URL,
        true,
        Some(&response),
        2,
        None,
    );
    assert_eq!(item["aweme_id"], "123");
    assert_eq!(item["request_url"], VIDEO_URL);
    assert_eq!(item["request_hd"], true);
    assert_eq!(item["request_index"], 2);
    assert_eq!(item["result_found"], true);
    assert_eq!(item["processed_time"], 1.25);
}

#[test]
fn missing_data_and_failed_lookups_keep_one_error_free_or_error_row_shape() {
    let not_found = dataset_item(
        None,
        VIDEO_URL,
        false,
        Some(&json!({"processed_time": 2})),
        1,
        None,
    );
    assert_eq!(not_found["result_found"], false);
    assert_eq!(not_found["processed_time"], 2);
    assert!(not_found.get("error_message").is_none());

    let failed = dataset_item(
        None,
        VIDEO_URL,
        false,
        None,
        1,
        Some("upstream failed".to_owned()),
    );
    assert_eq!(failed["result_found"], false);
    assert_eq!(failed["processed_time"], Value::Null);
    assert_eq!(failed["error_message"], "upstream failed");
}

#[test]
fn ppe_budget_accounts_for_other_events_and_existing_dataset_rows() {
    let run: Value = serde_json::from_str(&pricing_run(
        0.0005,
        json!({"other-event": 1, "apify-default-dataset-item": 0}),
    ))
    .unwrap();
    assert_eq!(affordable_dataset_items(&run, 3, 0).unwrap(), 2);
    assert_eq!(affordable_dataset_items(&run, 3, 1).unwrap(), 1);
}

#[test]
fn zero_priced_dataset_events_do_not_consume_budget() {
    let mut run: Value = serde_json::from_str(&pricing_run(1.0, json!({}))).unwrap();
    run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
        ["apify-default-dataset-item"]["eventPriceUsd"] = json!(0.0);
    assert_eq!(affordable_dataset_items(&run, 5, 0).unwrap(), 5);
}

#[test]
fn non_ppe_pricing_keeps_unbounded_dataset_capacity() {
    for pricing_model in ["FREE", "PRICE_PER_DATASET_ITEM", "PAY_PER_RESULT"] {
        let run = json!({"data": {"pricingInfo": {"pricingModel": pricing_model}}});
        assert_eq!(affordable_dataset_items(&run, 5, 9).unwrap(), 5);
    }
}

#[test]
fn ppe_without_a_max_charge_keeps_legacy_unbounded_capacity() {
    let mut missing_cap: Value =
        serde_json::from_str(&pricing_run(1.0, json!({"other-event": 1}))).unwrap();
    missing_cap["data"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("maxTotalChargeUsd");
    let mut null_cap: Value =
        serde_json::from_str(&pricing_run(1.0, json!({"other-event": 1}))).unwrap();
    null_cap["data"]["options"]["maxTotalChargeUsd"] = Value::Null;

    for run in [&missing_cap, &null_cap] {
        assert_eq!(affordable_dataset_items(run, 5, 0).unwrap(), 5);
    }
}

#[test]
fn ppe_zero_max_charge_disallows_paid_dataset_items() {
    let run: Value = serde_json::from_str(&pricing_run(0.0, json!({}))).unwrap();
    assert_eq!(affordable_dataset_items(&run, 5, 0).unwrap(), 0);
    assert_eq!(affordable_dataset_items(&run, 5, 1).unwrap(), 0);
}

#[test]
fn incomplete_ppe_pricing_fails_closed() {
    let run: Value = json!({
        "data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT"},
            "options": {"maxTotalChargeUsd": 1.0}
        }
    });
    assert!(affordable_dataset_items(&run, 1, 0).is_err());
}

#[tokio::test]
async fn apify_get_retries_a_temporary_server_error() {
    let (address, server) = start_mock_server(vec![
        ("503 Service Unavailable".to_owned(), "temporary".to_owned()),
        ("200 OK".to_owned(), "{}".to_owned()),
    ]);
    let url = Url::parse(&format!("http://{address}/v2/pricing")).unwrap();
    let response = apify_get(&Client::new(), url, "test-token", "pricing request")
        .await
        .unwrap();
    let requests = server.join().unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(requests.len(), 2);
    assert!(requests
        .iter()
        .all(|request| request.method_and_path.starts_with("GET /v2/pricing ")));
}

#[tokio::test]
async fn actor_reads_input_from_kv_and_writes_one_dataset_row_per_lookup() {
    let run = pricing_run(0.0004, json!({}));
    let (address, server) = start_mock_server(vec![
            (
                "200 OK".to_owned(),
                json!({"urls": [VIDEO_URL, SECOND_VIDEO_URL], "hd": true}).to_string(),
            ),
            ("200 OK".to_owned(), run),
            (
                "200 OK".to_owned(),
                json!({"code": 0, "data": [{"aweme_id":"first", "title":"clip"}, {"aweme_id":"ignored"}], "processed_time": 1.25}).to_string(),
            ),
            ("201 Created".to_owned(), String::new()),
            (
                "200 OK".to_owned(),
                json!({"code": 503, "msg":"upstream unavailable"}).to_string(),
            ),
            ("201 Created".to_owned(), String::new()),
        ]);

    run_actor(&Client::new(), &request_config(address))
        .await
        .unwrap();
    let requests = server.join().unwrap();

    assert_eq!(requests.len(), 6);
    assert!(requests[0]
        .method_and_path
        .starts_with("GET /v2/key-value-stores/store-test/records/INPUT "));
    assert!(requests[1]
        .method_and_path
        .starts_with("GET /v2/actor-runs/run-test "));
    assert!(
        requests[2]
            .method_and_path
            .contains("/api/tiktok/video?url="),
        "{}",
        requests[2].method_and_path
    );
    assert!(requests[3]
        .method_and_path
        .starts_with("POST /v2/datasets/dataset-test/items "));
    assert!(requests[4]
        .method_and_path
        .contains("/api/tiktok/video?url="));
    assert!(requests[5]
        .method_and_path
        .starts_with("POST /v2/datasets/dataset-test/items "));
    for index in [0, 1, 3, 5] {
        assert_eq!(
            requests[index]
                .headers
                .get("authorization")
                .map(String::as_str),
            Some("Bearer test-apify-token")
        );
    }
    for index in [2, 4] {
        assert_eq!(
            requests[index].headers.get("x-api-key").map(String::as_str),
            Some("test-scrappa-key")
        );
        assert!(requests[index].method_and_path.contains("&hd=1"));
    }

    let first_row: Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(first_row["aweme_id"], "first");
    assert_eq!(first_row["title"], "clip");
    assert_eq!(first_row["request_url"], VIDEO_URL);
    assert_eq!(first_row["request_index"], 1);
    assert_eq!(first_row["request_hd"], true);
    assert_eq!(first_row["result_found"], true);
    assert_eq!(first_row["processed_time"], 1.25);

    let second_row: Value = serde_json::from_str(&requests[5].body).unwrap();
    assert_eq!(second_row["request_url"], SECOND_VIDEO_URL);
    assert_eq!(second_row["request_index"], 2);
    assert_eq!(second_row["result_found"], false);
    assert_eq!(
        second_row["error_message"],
        "Scrappa TikTok Video API returned code 503 for https://www.tiktok.com/@tiktok/video/1234567890123456789: upstream unavailable"
    );
}

#[tokio::test]
async fn actor_writes_dataset_rows_under_free_pricing() {
    let free_run = json!({"data": {"pricingInfo": {"pricingModel": "FREE"}}}).to_string();
    let (address, server) = start_mock_server(vec![
        ("200 OK".to_owned(), json!({"url": VIDEO_URL}).to_string()),
        ("200 OK".to_owned(), free_run),
        (
            "200 OK".to_owned(),
            json!({"data": {"aweme_id":"free-video"}}).to_string(),
        ),
        ("201 Created".to_owned(), String::new()),
    ]);

    run_actor(&Client::new(), &request_config(address))
        .await
        .unwrap();
    let requests = server.join().unwrap();

    assert_eq!(requests.len(), 4);
    assert!(requests[2].method_and_path.contains("/api/tiktok/video?"));
    assert!(requests[3]
        .method_and_path
        .starts_with("POST /v2/datasets/dataset-test/items "));
    let row: Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(row["aweme_id"], "free-video");
    assert_eq!(row["request_url"], VIDEO_URL);
    assert_eq!(row["request_index"], 1);
    assert_eq!(row["result_found"], true);
}

#[tokio::test]
async fn budget_stops_before_scrappa_lookup_and_keeps_an_affordable_prefix() {
    let run = pricing_run(0.0003, json!({"other-event": 1}));
    let (address, server) = start_mock_server(vec![
        (
            "200 OK".to_owned(),
            json!({"urls": [VIDEO_URL, SECOND_VIDEO_URL]}).to_string(),
        ),
        ("200 OK".to_owned(), run),
        (
            "200 OK".to_owned(),
            json!({"data": {"aweme_id":"first"}}).to_string(),
        ),
        ("201 Created".to_owned(), String::new()),
    ]);

    run_actor(&Client::new(), &request_config(address))
        .await
        .unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[2].method_and_path.contains("/api/tiktok/video?"));
    let row: Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(row["request_index"], 1);
    assert_eq!(row["aweme_id"], "first");
}

#[tokio::test]
async fn zero_total_charge_cap_stops_before_lookup_or_output() {
    let run = pricing_run(0.0, json!({}));
    let (address, server) = start_mock_server(vec![
        ("200 OK".to_owned(), json!({"url": VIDEO_URL}).to_string()),
        ("200 OK".to_owned(), run),
    ]);

    run_actor(&Client::new(), &request_config(address))
        .await
        .unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0]
        .method_and_path
        .starts_with("GET /v2/key-value-stores/store-test/records/INPUT "));
    assert!(requests[1]
        .method_and_path
        .starts_with("GET /v2/actor-runs/run-test "));
    assert!(requests.iter().all(|request| {
        !request.method_and_path.contains("/api/tiktok/video")
            && !request.method_and_path.contains("/v2/datasets/")
    }));
}

#[tokio::test]
async fn scrappa_http_errors_become_dataset_error_rows() {
    let run = pricing_run(0.0002, json!({}));
    let (address, server) = start_mock_server(vec![
        ("200 OK".to_owned(), json!({"url": VIDEO_URL}).to_string()),
        ("200 OK".to_owned(), run),
        (
            "500 Internal Server Error".to_owned(),
            json!({"message":"upstream failed", "errors":{"url":["unavailable"]}}).to_string(),
        ),
        ("201 Created".to_owned(), String::new()),
    ]);

    run_actor(&Client::new(), &request_config(address))
        .await
        .unwrap();
    let requests = server.join().unwrap();
    let row: Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(row["result_found"], false);
    assert_eq!(
        row["error_message"],
        "Scrappa API error (500): upstream failed - url: unavailable"
    );
}

#[tokio::test]
async fn dataset_storage_errors_fail_the_actor() {
    let run = pricing_run(0.0002, json!({}));
    let (address, server) = start_mock_server(vec![
        ("200 OK".to_owned(), json!({"url": VIDEO_URL}).to_string()),
        ("200 OK".to_owned(), run),
        (
            "200 OK".to_owned(),
            json!({"data": {"aweme_id":"1"}}).to_string(),
        ),
        (
            "500 Internal Server Error".to_owned(),
            "dataset unavailable".to_owned(),
        ),
    ]);

    let error = run_actor(&Client::new(), &request_config(address))
        .await
        .unwrap_err();
    let requests = server.join().unwrap();
    assert!(error.to_string().contains("Apify dataset write failed"));
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[3].method_and_path.matches("POST").count(), 1);
}
