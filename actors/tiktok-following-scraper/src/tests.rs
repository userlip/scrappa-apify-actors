use crate::{
    actor::run_actor,
    apify::{affordable_dataset_items, ActorConfig, DatasetBudget},
    input::{build_params, FollowingParams},
    scrappa::{
        extract_pagination, extract_profile_user_id, following_items, following_url, profile_url,
        Pagination,
    },
    REQUEST_TIMEOUT,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};
use url::Url;

fn base_url() -> Url {
    Url::parse("https://scrappa.co/api").unwrap()
}

fn test_config(server_url: Url, api_key: &str) -> ActorConfig {
    ActorConfig {
        apify_api_base_url: server_url.clone(),
        scrappa_api_base_url: server_url,
        default_key_value_store_id: "store-id".to_owned(),
        default_dataset_id: "dataset-id".to_owned(),
        input_key: "INPUT".to_owned(),
        actor_run_id: "run-id".to_owned(),
        apify_token: "test-apify-token".to_owned(),
        scrappa_api_key: api_key.to_owned(),
    }
}

#[test]
fn normalizes_profile_inputs_and_keeps_lookup_precedence() {
    let params = build_params(
        &json!({ "profile": "https://www.tiktok.com/@TikTok?lang=en", "unique_id": "other", "user_id": "bad" }),
        |_| {},
    )
    .unwrap();
    assert_eq!(
        params,
        FollowingParams {
            unique_id: Some("@TikTok".to_owned()),
            user_id: None,
            count: None,
            time: None,
        }
    );

    let params = build_params(&json!({ "profile": "107955" }), |_| {}).unwrap();
    assert_eq!(params.user_id.as_deref(), Some("107955"));
    assert_eq!(params.unique_id, None);

    let params = build_params(&json!({ "unique_id": "107955" }), |_| {}).unwrap();
    assert_eq!(params.unique_id.as_deref(), Some("@107955"));
}

#[test]
fn validates_urls_usernames_and_user_ids() {
    assert_eq!(
        build_params(&json!({ "profile": "@tiktok" }), |_| {})
            .unwrap()
            .unique_id
            .as_deref(),
        Some("@tiktok")
    );
    assert!(
        build_params(&json!({ "profile": "https://example.com/@tiktok" }), |_| {})
            .unwrap_err()
            .to_string()
            .contains("must be on tiktok.com")
    );
    assert!(
        build_params(&json!({ "profile": "http://tiktok.com/@tiktok" }), |_| {})
            .unwrap_err()
            .to_string()
            .contains("must use HTTPS")
    );
    assert!(build_params(&json!({ "unique_id": "@tik-tok" }), |_| {})
        .unwrap_err()
        .to_string()
        .contains("TikTok username must be"));
    assert!(build_params(&json!({ "user_id": "12x" }), |_| {})
        .unwrap_err()
        .to_string()
        .contains("user_id must contain digits only"));
}

#[test]
fn retains_count_defaulting_and_cursor_compatibility() {
    let mut warnings = Vec::new();
    let params = build_params(
        &json!({ "profile": "tiktok", "count": 0, "time": null, "cursor": "123" }),
        |warning| warnings.push(warning),
    )
    .unwrap();
    assert_eq!(params.requested_count(), 10.0);
    assert_eq!(params.time, Some(Value::String("123".to_owned())));
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("count must be a positive integer"));

    let params = build_params(
        &json!({ "profile": "@tiktok", "count": 100000, "time": 0 }),
        |_| {},
    )
    .unwrap();
    assert_eq!(params.requested_count(), 100000.0);
    assert_eq!(params.time, Some(json!(0)));

    let params = build_params(
        &json!({ "profile": "@tiktok", "time": "  ", "cursor": "123" }),
        |_| {},
    )
    .unwrap();
    assert_eq!(params.time, None);
}

#[test]
fn extracts_profile_ids_following_aliases_and_pagination() {
    assert_eq!(
        extract_profile_user_id(Some(&json!([{ "user_id": " 123 " }]))).as_deref(),
        Some("123")
    );
    assert_eq!(
        extract_profile_user_id(Some(&json!({ "user": { "id": 456 } }))).as_deref(),
        Some("456")
    );
    assert_eq!(
        following_items(Some(&json!({ "following": [{ "id": 1 }] }))).len(),
        1
    );
    assert_eq!(
        following_items(Some(&json!({ "followings": [{ "id": 2 }] }))).len(),
        1
    );
    assert_eq!(
        following_items(Some(&json!({ "users": [{ "id": 3 }] }))).len(),
        1
    );
    assert_eq!(
        following_items(Some(&json!({ "user_list": [{ "id": 4 }] }))).len(),
        1
    );

    let pagination = extract_pagination(Some(&json!({
        "hasMore": null,
        "has_more": true,
        "time": null,
        "min_time": "1711111111",
        "max_time": "later"
    })));
    assert_eq!(
        pagination,
        Pagination {
            has_next_page: true,
            next_time: Some(Value::String("1711111111".to_owned())),
        }
    );
    assert_eq!(
        extract_pagination(Some(&json!([{ "user_id": "1" }]))),
        Pagination {
            has_next_page: false,
            next_time: None,
        }
    );
}

#[test]
fn builds_encoded_api_urls_with_cursor_markers() {
    let params = FollowingParams {
        unique_id: Some("@tiktok".to_owned()),
        user_id: None,
        count: None,
        time: None,
    };
    assert_eq!(
        following_url(&base_url(), &params, 50, Some(&json!(0)))
            .unwrap()
            .as_str(),
        "https://scrappa.co/api/tiktok/user/following?unique_id=%40tiktok&count=50&time=0"
    );
    assert_eq!(
        profile_url(&base_url(), "@a & b").unwrap().as_str(),
        "https://scrappa.co/api/tiktok/user/profile?unique_id=%40a+%26+b"
    );
}

#[test]
fn calculates_ppe_capacity_from_existing_run_charges() {
    let run = json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {
                    "actorChargeEvents": {
                        "apify-default-dataset-item": { "eventPriceUsd": 0.0001 },
                        "other-event": { "eventPriceUsd": 0.0001 }
                    }
                }
            },
            "chargedEventCounts": { "other-event": 1 },
            "options": { "maxTotalChargeUsd": 0.00025 }
        }
    });
    assert_eq!(
        affordable_dataset_items(&run, 5, &mut DatasetBudget::default()).unwrap(),
        1
    );
    assert_eq!(
        affordable_dataset_items(
            &json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {
                            "actorChargeEvents": {
                                "apify-default-dataset-item": { "eventPriceUsd": 0 }
                            }
                        }
                    },
                    "chargedEventCounts": {},
                    "options": { "maxTotalChargeUsd": 0 }
                }
            }),
            5,
            &mut DatasetBudget::default()
        )
        .unwrap(),
        5
    );
}

#[test]
fn treats_missing_null_and_zero_spending_limits_as_unlimited() {
    let run_with_limit = |max_charge: Option<Value>| {
        let mut run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-default-dataset-item": { "eventPriceUsd": 0.0001 },
                            "other-event": { "eventPriceUsd": 0.0001 }
                        }
                    }
                },
                "chargedEventCounts": { "other-event": 1 },
                "options": {}
            }
        });
        if let Some(max_charge) = max_charge {
            run["data"]["options"]["maxTotalChargeUsd"] = max_charge;
        }
        run
    };

    let runs = [
        run_with_limit(None),
        run_with_limit(Some(Value::Null)),
        run_with_limit(Some(json!(0))),
        run_with_limit(Some(json!(0.00025))),
    ];

    let capacities = runs
        .iter()
        .map(|run| {
            affordable_dataset_items(run, 5, &mut DatasetBudget::default())
                .map_err(|error| error.to_string())
        })
        .collect::<Vec<_>>();
    assert_eq!(capacities, vec![Ok(5), Ok(5), Ok(5), Ok(1)]);
}

#[test]
fn does_not_limit_non_ppe_runs() {
    let run = json!({
        "data": {
            "pricingInfo": { "pricingModel": "PRICE_PER_DATASET_ITEM" }
        }
    });
    assert_eq!(
        affordable_dataset_items(&run, 5, &mut DatasetBudget::default()).unwrap(),
        5
    );
}

fn mock_server(
    responses: Vec<(&'static str, &'static str)>,
) -> (Url, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            requests.push(read_request(&mut stream));
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
        requests
    });
    (
        Url::parse(&format!("http://{address}/api")).unwrap(),
        server,
    )
}

fn mock_server_until_idle(
    responses: Vec<(&'static str, &'static str)>,
) -> (Url, thread::JoinHandle<Vec<String>>) {
    use std::time::{Duration, Instant};

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        let mut last_request = Instant::now();
        for (status, body) in responses {
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if !requests.is_empty()
                            && last_request.elapsed() >= Duration::from_millis(250)
                        {
                            return requests;
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => panic!("mock server accept failed: {error}"),
                }
            };
            requests.push(read_request(&mut stream));
            last_request = Instant::now();
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
        requests
    });
    (
        Url::parse(&format!("http://{address}/api")).unwrap(),
        server,
    )
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut reader = BufReader::new(stream);
    let mut request = String::new();
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    request.push_str(&line);
    let mut content_length = 0usize;
    loop {
        line.clear();
        reader.read_line(&mut line).unwrap();
        if line == "\r\n" || line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap();
            }
        }
        request.push_str(&line);
    }
    request.push_str("\r\n");
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).unwrap();
    request.push_str(&String::from_utf8(body).unwrap());
    request
}

const RUN_NORMAL_BUDGET: &str = r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001},"other-event":{"eventPriceUsd":0.0001}}}},"chargedEventCounts":{"other-event":1},"options":{"maxTotalChargeUsd":1.0}}}"#;
const RUN_ONE_ITEM_LEFT: &str = r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001},"other-event":{"eventPriceUsd":0.0001}}}},"chargedEventCounts":{"other-event":1},"options":{"maxTotalChargeUsd":0.00025}}}"#;
const RUN_TWO_ITEMS_MAX: &str = r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001}}}},"chargedEventCounts":{},"options":{"maxTotalChargeUsd":0.0002}}}"#;

#[tokio::test]
async fn resolves_profiles_paginates_and_writes_dataset_and_summary() {
    let (base_url, server) = mock_server(vec![
        ("200 OK", r#"{"profile":"tiktok","count":55}"#),
        ("200 OK", r#"{"code":0,"data":{"user_id":"107955"}}"#),
        (
            "200 OK",
            r#"{"code":0,"data":{"following":[{"user_id":"1","unique_id":"one"},{"user_id":"2","extra":true}],"hasMore":true,"time":1711111111},"processed_time":10}"#,
        ),
        ("200 OK", RUN_NORMAL_BUDGET),
        ("201 Created", ""),
        (
            "200 OK",
            r#"{"code":0,"data":{"users":[{"user_id":"3"}],"has_more":false,"time":0},"processed_time":20}"#,
        ),
        ("200 OK", RUN_NORMAL_BUDGET),
        ("201 Created", ""),
        ("201 Created", ""),
    ]);
    let config = test_config(base_url, "test-scrappa-key");
    let client = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();

    run_actor(&client, &config).await.unwrap();
    let requests = server.join().unwrap();

    assert_eq!(requests.len(), 9);
    assert!(requests[0].starts_with("GET /api/v2/key-value-stores/store-id/records/INPUT "));
    assert!(requests[0]
        .to_ascii_lowercase()
        .contains("authorization: bearer test-apify-token"));
    assert!(requests[1].contains("GET /api/tiktok/user/profile?unique_id=%40tiktok "));
    assert!(requests[1]
        .to_ascii_lowercase()
        .contains("x-api-key: test-scrappa-key"));
    assert!(requests[2].contains("GET /api/tiktok/user/following?user_id=107955&count=50 "));
    assert!(requests[3].contains("GET /api/v2/actor-runs/run-id "));
    assert!(requests[4].contains("POST /api/v2/datasets/dataset-id/items "));
    assert!(requests[4].contains("\"lookup_unique_id\":\"@tiktok\""));
    assert!(requests[4].contains("\"lookup_user_id\":\"107955\""));
    assert!(requests[5]
        .contains("GET /api/tiktok/user/following?user_id=107955&count=50&time=1711111111 "));
    assert!(requests[6].contains("GET /api/v2/actor-runs/run-id "));
    assert!(requests[7].contains("POST /api/v2/datasets/dataset-id/items "));
    assert!(requests[8].starts_with("PUT /api/v2/key-value-stores/store-id/records/OUTPUT "));
    assert!(requests[8].contains("\"following_extracted\":3"));
    assert!(requests[8].contains("\"requested_count\":55"));
    assert!(requests[8].contains("\"next_time\":0"));
}

#[tokio::test]
async fn stops_at_the_ppe_cap_and_keeps_small_run_output_response() {
    let (base_url, server) = mock_server(vec![
        ("200 OK", r#"{"profile":"107955","count":10}"#),
        (
            "200 OK",
            r#"{"code":0,"data":{"following":[{"user_id":"1"},{"user_id":"2"}],"hasMore":true,"time":12},"processed_time":30}"#,
        ),
        ("200 OK", RUN_ONE_ITEM_LEFT),
        ("201 Created", ""),
        ("201 Created", ""),
    ]);
    let config = test_config(base_url, "test-scrappa-key");
    let client = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();

    run_actor(&client, &config).await.unwrap();
    let requests = server.join().unwrap();

    assert_eq!(requests.len(), 5);
    assert!(requests[1].contains("GET /api/tiktok/user/following?user_id=107955&count=10 "));
    assert!(requests[3].contains("\"user_id\":\"1\""));
    assert!(requests[3].contains("\"lookup_unique_id\":null"));
    assert!(requests[3].contains("\"lookup_user_id\":\"107955\""));
    assert!(requests[4].contains("\"following\":[{\"user_id\":\"1\"},{\"user_id\":\"2\"}]"));
    assert!(!requests[4].contains("\"following_extracted\""));
}

#[tokio::test]
async fn zero_ppe_cap_writes_all_rows_and_preserves_output_count() {
    let (base_url, server) = mock_server_until_idle(vec![
        ("200 OK", r#"{"profile":"107955","count":10}"#),
        (
            "200 OK",
            r#"{"code":0,"data":{"following":[{"user_id":"1"},{"user_id":"2"}],"hasMore":false,"time":0},"processed_time":30}"#,
        ),
        (
            "200 OK",
            r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001}}}},"chargedEventCounts":{},"options":{"maxTotalChargeUsd":0}}}"#,
        ),
        ("201 Created", ""),
        ("201 Created", ""),
    ]);
    let config = test_config(base_url, "test-scrappa-key");
    let client = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();

    run_actor(&client, &config).await.unwrap();
    let requests = server.join().unwrap();

    assert_eq!(requests.len(), 5);
    let dataset_write = requests
        .iter()
        .find(|request| request.starts_with("POST /api/v2/datasets/dataset-id/items "))
        .unwrap();
    assert!(dataset_write.contains("\"user_id\":\"1\""));
    assert!(dataset_write.contains("\"user_id\":\"2\""));
    let output = requests
        .iter()
        .find(|request| {
            request.starts_with("PUT /api/v2/key-value-stores/store-id/records/OUTPUT ")
        })
        .unwrap();
    assert!(output.contains("\"following\":[{\"user_id\":\"1\"},{\"user_id\":\"2\"}]"));
    assert!(output.contains("\"processed_time\":30"));
    assert!(!output.contains("\"following_extracted\""));
}

#[tokio::test]
async fn stale_run_pricing_does_not_reopen_spent_dataset_capacity() {
    let (base_url, server) = mock_server_until_idle(vec![
        ("200 OK", r#"{"profile":"107955","count":55}"#),
        (
            "200 OK",
            r#"{"code":0,"data":{"following":[{"user_id":"1"},{"user_id":"2"}],"hasMore":true,"time":12},"processed_time":10}"#,
        ),
        ("200 OK", RUN_TWO_ITEMS_MAX),
        ("201 Created", ""),
        (
            "200 OK",
            r#"{"code":0,"data":{"following":[{"user_id":"3"}],"hasMore":false,"time":0},"processed_time":20}"#,
        ),
        ("200 OK", RUN_TWO_ITEMS_MAX),
        ("201 Created", ""),
        ("201 Created", ""),
    ]);
    let config = test_config(base_url, "test-scrappa-key");
    let client = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();

    run_actor(&client, &config).await.unwrap();
    let requests = server.join().unwrap();

    assert_eq!(requests.len(), 7);
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.starts_with("POST /api/v2/datasets/dataset-id/items "))
            .count(),
        1
    );
    let output = requests
        .iter()
        .find(|request| {
            request.starts_with("PUT /api/v2/key-value-stores/store-id/records/OUTPUT ")
        })
        .unwrap();
    assert!(output.contains("\"following_extracted\":2"));
    assert!(output.contains("\"requested_count\":55"));
}

#[tokio::test]
async fn reports_scrappa_http_errors_without_retrying() {
    let (base_url, server) = mock_server(vec![
        ("200 OK", r#"{"profile":"107955"}"#),
        (
            "429 Too Many Requests",
            r#"{"message":"Slow down","errors":{"profile":["blocked"]}}"#,
        ),
    ]);
    let config = test_config(base_url, "test-scrappa-key");
    let client = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();

    let error = run_actor(&client, &config).await.unwrap_err().to_string();
    let requests = server.join().unwrap();

    assert_eq!(requests.len(), 2);
    assert!(error.contains("Scrappa API error (429): Slow down - profile: blocked"));
}
