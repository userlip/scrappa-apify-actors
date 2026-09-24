use crate::apify_client::{ApifyClient, DatasetBudget};
use crate::params::{
    build_params, build_params_with_warning, format_lookup_for_log, normalize_tiktok_unique_id,
};
use crate::response::{enrich_post, extract_pagination, extract_posts, validate_scrappa_code};
use crate::scrappa_client::format_scrappa_error;
use reqwest::StatusCode;
use serde_json::{json, Value};

fn run_pricing(model: &str, max_charge: Value, charged_counts: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": model,
                "pricingPerEvent": { "actorChargeEvents": {
                    "apify-default-dataset-item": { "eventPriceUsd": 0.0003 },
                    "apify-actor-start": { "eventPriceUsd": 0.001 }
                }}
            },
            "options": { "maxTotalChargeUsd": max_charge },
            "chargedEventCounts": charged_counts
        }
    })
}

#[test]
fn builds_profile_params_and_keeps_profile_precedence() {
    let input = json!({
        "profile": " https://www.tiktok.com/@tik.tok_123/?lang=en ",
        "unique_id": "ignored",
        "user_id": "ignored",
        "count": 25,
        "cursor": "  next  "
    });
    let params = build_params(&input).unwrap();
    assert_eq!(params.unique_id.as_deref(), Some("@tik.tok_123"));
    assert_eq!(params.user_id, None);
    assert_eq!(params.count, Some(25));
    assert_eq!(params.cursor.as_deref(), Some("next"));
    assert_eq!(format_lookup_for_log(&input).unwrap(), "@tik.tok_123");
}

#[test]
fn handles_numeric_profile_and_explicit_user_id() {
    assert_eq!(
        build_params(&json!({ "profile": "107955" }))
            .unwrap()
            .user_id
            .as_deref(),
        Some("107955")
    );
    assert_eq!(
        build_params(&json!({ "user_id": " 107955 " }))
            .unwrap()
            .user_id
            .as_deref(),
        Some("107955")
    );
    assert_eq!(
        build_params(&json!({ "unique_id": "tiktok", "user_id": "abc" }))
            .unwrap()
            .unique_id
            .as_deref(),
        Some("@tiktok")
    );
}

#[test]
fn warns_and_omits_invalid_count_and_cursor() {
    let mut warnings = Vec::new();
    let params = build_params_with_warning(
        &json!({ "profile": "@tiktok", "count": 0, "cursor": 123 }),
        |message| warnings.push(message),
    )
    .unwrap();
    assert_eq!(params.count, None);
    assert_eq!(params.cursor, None);
    assert!(warnings[0].contains("count must be an integer between 1 and 50"));
    assert!(warnings[1].contains("cursor must be a string"));
}

#[test]
fn rejects_missing_lookup_and_invalid_tiktok_profiles() {
    assert!(build_params(&json!({ "profile": " " }))
        .unwrap_err()
        .to_string()
        .contains("TikTok unique_id or user_id is required"));
    assert!(normalize_tiktok_unique_id("https://example.com/@tiktok")
        .unwrap_err()
        .to_string()
        .contains("must be on tiktok.com"));
    assert!(normalize_tiktok_unique_id("http://www.tiktok.com/@tiktok")
        .unwrap_err()
        .to_string()
        .contains("must use HTTPS"));
    assert!(normalize_tiktok_unique_id("@tik-tok")
        .unwrap_err()
        .to_string()
        .contains("TikTok username must"));
}

#[test]
fn extracts_post_shapes_and_pagination_fallbacks() {
    let data = json!({
        "posts": [{ "aweme_id": "preferred" }],
        "videos": [{ "aweme_id": "1" }],
        "has_more": true,
        "max_cursor": "200"
    });
    assert_eq!(extract_posts(Some(&data))[0]["aweme_id"], "preferred");
    assert_eq!(extract_pagination(Some(&data)), (true, json!("200")));
    assert_eq!(
        extract_posts(Some(&json!({ "aweme_list": [{ "aweme_id": "3" }] })))[0]["aweme_id"],
        "3"
    );
    assert_eq!(
        extract_pagination(Some(&json!({
            "hasMore": true,
            "cursor": "100",
            "max_cursor": "200"
        }))),
        (true, json!("100"))
    );
    assert_eq!(
        extract_pagination(Some(&json!({ "has_more": false, "min_cursor": "300" }))),
        (false, json!("300"))
    );
    assert_eq!(extract_posts(Some(&json!([{ "aweme_id": "2" }]))).len(), 1);
    assert_eq!(extract_pagination(Some(&Value::Null)), (false, Value::Null));
}

#[test]
fn enriches_each_post_with_both_lookup_columns() {
    let post = json!({ "aweme_id": "1" });
    let params = build_params(&json!({ "profile": "@tiktok" })).unwrap();
    let row = enrich_post(&post, &params);
    assert_eq!(row["lookup_unique_id"], "@tiktok");
    assert!(row["lookup_user_id"].is_null());
    assert_eq!(row["aweme_id"], "1");
}

#[test]
fn applies_the_ppe_dataset_budget_and_leaves_other_pricing_unlimited() {
    let ppe = run_pricing(
        "PAY_PER_EVENT",
        json!(0.0019),
        json!({ "apify-actor-start": 1, "apify-default-dataset-item": 1 }),
    );
    let budget = DatasetBudget::from_run(&ppe).unwrap();
    assert_eq!(budget, DatasetBudget::Limited(2));
    assert_eq!(budget.limit(5), 2);

    let free = run_pricing("FREE", json!(0.0019), json!({}));
    assert_eq!(
        DatasetBudget::from_run(&free).unwrap(),
        DatasetBudget::Unlimited
    );
}

#[test]
fn returns_scrappa_code_errors() {
    let error = validate_scrappa_code(&json!({ "code": 12, "msg": "Blocked" })).unwrap_err();
    assert_eq!(
        error.to_string(),
        "Scrappa TikTok User Posts API returned code 12: Blocked"
    );
    assert!(validate_scrappa_code(&json!({ "code": 0 })).is_ok());
}

#[test]
fn formats_scrappa_http_error_details_and_plain_text() {
    assert_eq!(
        format_scrappa_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            r#"{"message":"Invalid input","errors":{"profile":["required"]}}"#
        ),
        "Invalid input - profile: required"
    );
    assert_eq!(
        format_scrappa_error(StatusCode::BAD_GATEWAY, "  upstream\n unavailable  "),
        "upstream unavailable"
    );
}

#[tokio::test]
async fn retries_transient_get_failures_twice_with_short_backoff() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut accepted_requests = 0;
        while accepted_requests < 3 && Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut request = [0_u8; 1024];
                    stream.read(&mut request).unwrap();
                    stream
                        .write_all(
                            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        )
                        .unwrap();
                    accepted_requests += 1;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("mock server failed while accepting request: {error}"),
            }
        }
        accepted_requests
    });

    let client = ApifyClient::new(
        url::Url::parse(&format!("http://{address}/")).unwrap(),
        "test-token".to_owned(),
    )
    .unwrap();
    let started = Instant::now();
    let error = client.get_run("run-id").await.unwrap_err();
    let elapsed = started.elapsed();
    let accepted_requests = tokio::task::spawn_blocking(move || server.join().unwrap())
        .await
        .unwrap();

    assert_eq!(accepted_requests, 3, "initial attempt plus two retries");
    assert!(elapsed < Duration::from_secs(2), "retry wait was too long");
    assert!(error.to_string().contains("Apify API error (503)"));
}

#[tokio::test]
async fn does_not_retry_dataset_post_when_the_accepted_request_loses_its_acknowledgment() {
    use std::io::Read;
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::{Duration, Instant};

    fn read_request(stream: &mut TcpStream) -> String {
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 1024];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(count) => {
                    bytes.extend_from_slice(&chunk[..count]);
                    let Some(header_end) =
                        bytes.windows(4).position(|window| window == b"\r\n\r\n")
                    else {
                        continue;
                    };
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
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("mock server failed to accept request: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let request = read_request(&mut stream);
        drop(stream);

        let deadline = Instant::now() + Duration::from_millis(700);
        let mut accepted_requests = 1;
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut retry, _)) => {
                    accepted_requests += 1;
                    retry
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let _ = read_request(&mut retry);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("mock server failed while checking retries: {error}"),
            }
        }
        (accepted_requests, request)
    });

    let base_url = url::Url::parse(&format!("http://{address}/")).unwrap();
    let client = ApifyClient::new(base_url, "test-token".to_owned()).unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        client.push_data("dataset-id", &[json!({ "aweme_id": "one" })]),
    )
    .await
    .expect("dataset request should fail promptly after the lost acknowledgment");
    assert!(result.is_err());

    let (accepted_requests, request) = tokio::task::spawn_blocking(move || server.join().unwrap())
        .await
        .unwrap();
    assert_eq!(accepted_requests, 1, "POST must not be retried");
    assert!(request.starts_with("POST /v2/datasets/dataset-id/items HTTP/1.1"));
    assert!(request.contains(r#"{"aweme_id":"one"}"#));
}
