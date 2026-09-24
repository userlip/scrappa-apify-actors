use crate::{
    apify::{
        affordable_dataset_items, send_apify_request, ActorConfig, APIFY_MAX_RETRIES,
        APIFY_REQUEST_TIMEOUT,
    },
    failure_message,
    input::{build_hashtag_posts_params, TikTokHashtagPostsParams},
    run_actor,
    scrappa::{fetch_scrappa_response, validate_scrappa_code, SCRAPPA_REQUEST_TIMEOUT},
    tiktok_response::{
        enrich_post, extract_challenges, extract_pagination, extract_posts, get_challenge_id,
        get_challenge_name, select_challenge_for_hashtag,
    },
};
use anyhow::anyhow;
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};
use url::Url;

fn value(source: &str) -> Value {
    serde_json::from_str(source).unwrap()
}

fn request_config(address: std::net::SocketAddr) -> ActorConfig {
    ActorConfig {
        apify_api_base_url: Url::parse(&format!("http://{address}/")).unwrap(),
        scrappa_api_base_url: Url::parse(&format!("http://{address}/api")).unwrap(),
        default_key_value_store_id: "store-test".to_owned(),
        default_dataset_id: "dataset-test".to_owned(),
        actor_run_id: "run-test".to_owned(),
        input_key: "INPUT".to_owned(),
        apify_token: "test-apify-token".to_owned(),
        scrappa_api_key: "test-scrappa-key".to_owned(),
    }
}

#[test]
fn keeps_actor_input_prefill_and_limits_in_schema() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    let properties = schema["properties"].as_object().unwrap();
    assert_eq!(schema["required"][0], "hashtag");
    assert_eq!(properties["hashtag"]["prefill"], "cosplay");
    assert_eq!(properties["region"]["prefill"], "US");
    assert_eq!(properties["count"]["default"], 10);
    assert_eq!(properties["cursor"]["prefill"], "0");
    assert_eq!(properties["count"]["minimum"], 1);
    assert_eq!(properties["count"]["maximum"], 50);
}

#[test]
fn builds_params_from_hashtag_alias_and_valid_optional_fields() {
    assert_eq!(
        build_hashtag_posts_params(&value(
            r##"{"hashtag":" #cosplay ","region":" us ","count":10,"cursor":" 0 "}"##
        ))
        .unwrap(),
        TikTokHashtagPostsParams {
            challenge_name: Some("cosplay".to_owned()),
            challenge_id: None,
            region: Some("US".to_owned()),
            count: Some(10),
            cursor: Some("0".to_owned()),
            lookup_label: "#cosplay".to_owned(),
        }
    );
    assert_eq!(
        build_hashtag_posts_params(&value(r#"{"hashtag":"33380"}"#))
            .unwrap()
            .challenge_id,
        Some("33380".to_owned())
    );
    assert_eq!(
        build_hashtag_posts_params(&value(
            r#"{"challenge_name":"BookTok","challenge_id":"abc"}"#
        ))
        .unwrap()
        .challenge_name,
        Some("BookTok".to_owned())
    );
}

#[test]
fn validates_urls_names_and_challenge_ids() {
    assert_eq!(
        build_hashtag_posts_params(&value(
            r#"{"hashtag":"https://www.tiktok.com/tag/cosplay?lang=en"}"#
        ))
        .unwrap()
        .challenge_name,
        Some("cosplay".to_owned())
    );
    assert_eq!(
        build_hashtag_posts_params(&value(&format!(r#"{{"hashtag":"{}"}}"#, "1".repeat(100))))
            .unwrap()
            .challenge_id,
        Some("1".repeat(100))
    );
    assert!(
        build_hashtag_posts_params(&value(r#"{"hashtag":"https://example.com/tag/fyp"}"#))
            .unwrap_err()
            .to_string()
            .contains("must be on tiktok.com")
    );
    assert!(
        build_hashtag_posts_params(&value(r#"{"hashtag":"http://tiktok.com/tag/fyp"}"#))
            .unwrap_err()
            .to_string()
            .contains("must use HTTPS")
    );
    assert!(
        build_hashtag_posts_params(&value(r#"{"challenge_id":"abc"}"#))
            .unwrap_err()
            .to_string()
            .contains("digits only")
    );
    assert!(build_hashtag_posts_params(&value(r#"{"hashtag":"x/y"}"#))
        .unwrap_err()
        .to_string()
        .contains("cannot contain whitespace or URL delimiter characters"));
}

#[test]
fn warns_and_omits_invalid_optional_values() {
    let params = build_hashtag_posts_params(&value(
        r#"{"hashtag":"fyp","region":12,"count":0,"cursor":true}"#,
    ))
    .unwrap();
    assert_eq!(params.region, None);
    assert_eq!(params.count, None);
    assert_eq!(params.cursor, None);
    assert!(build_hashtag_posts_params(&value(r#"{"hashtag":" "}"#))
        .unwrap_err()
        .to_string()
        .contains("TikTok challenge_id or challenge_name is required"));
}

#[test]
fn selects_only_exact_case_insensitive_challenge_matches() {
    let challenges =
        value(r#"[{"id":"1","cha_name":"cosplaygirl"},{"id":"33380","cha_name":"Cosplay"}]"#);
    let selected = select_challenge_for_hashtag(challenges.as_array().unwrap(), "#cosplay");
    assert_eq!(
        get_challenge_id(selected.unwrap()),
        Some("33380".to_owned())
    );
    assert_eq!(get_challenge_name(selected.unwrap()), "Cosplay");
    assert!(select_challenge_for_hashtag(challenges.as_array().unwrap(), "cos").is_none());
    assert_eq!(
        extract_challenges(Some(&value(r#"{"challenge_list":[{"id":"2"}]}"#))).len(),
        1
    );
    assert_eq!(
        get_challenge_id(&value(r#"{"challenge_id":33380}"#)),
        Some("33380".to_owned())
    );
}

#[test]
fn extracts_post_shapes_and_pagination_fields() {
    let data = value(
        r#"{"posts":[{"aweme_id":"1"}],"videos":[{"aweme_id":"ignored"}],"hasMore":true,"has_more":false,"cursor":"100","max_cursor":"200"}"#,
    );
    assert_eq!(extract_posts(Some(&data)).len(), 1);
    assert_eq!(
        extract_pagination(Some(&data)),
        (true, Value::String("100".to_owned()))
    );
    assert_eq!(
        extract_posts(Some(&value(r#"{"aweme_list":[{"aweme_id":"3"}]}"#))).len(),
        1
    );
    assert_eq!(
        extract_posts(Some(&value(r#"[{"aweme_id":"4"}]"#))).len(),
        1
    );
    assert_eq!(extract_pagination(Some(&Value::Null)), (false, Value::Null));
}

#[test]
fn preserves_lookup_metadata_and_raw_post_fields() {
    let params =
        build_hashtag_posts_params(&value(r#"{"hashtag":"cosplay","region":"us"}"#)).unwrap();
    let post = enrich_post(
        &value(r#"{"aweme_id":"1","lookup_region":"old"}"#),
        &params,
        Some("Cosplay"),
        Some("33380"),
    );
    assert_eq!(post["aweme_id"], "1");
    assert_eq!(post["lookup_challenge_name"], "cosplay");
    assert!(post["lookup_challenge_id"].is_null());
    assert_eq!(post["lookup_region"], "US");
    assert_eq!(post["resolved_challenge_name"], "Cosplay");
    assert_eq!(post["resolved_challenge_id"], "33380");
}

#[test]
fn validates_scrappa_api_codes() {
    assert!(validate_scrappa_code(&value(r#"{"code":0}"#), "test").is_ok());
    assert!(validate_scrappa_code(&value(r#"{"data":[]}"#), "test").is_ok());
    assert!(
        validate_scrappa_code(&value(r#"{"code":1,"msg":"bad"}"#), "test")
            .unwrap_err()
            .to_string()
            .contains("code 1: bad")
    );
}

#[test]
fn allows_all_dataset_items_for_non_ppe_runs() {
    let run = value(r#"{"data":{"pricingInfo":{"pricingModel":"PRICE_PER_DATASET_ITEM"}}}"#);
    assert_eq!(affordable_dataset_items(&run, 3, 0).unwrap(), 3);
}

#[test]
fn treats_zero_null_and_missing_ppe_limits_as_unlimited() {
    let runs = [
        r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"},"options":{"maxTotalChargeUsd":0}}}"#,
        r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"},"options":{"maxTotalChargeUsd":null}}}"#,
        r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"},"options":{}}}"#,
    ];
    for run in runs {
        assert_eq!(affordable_dataset_items(&value(run), 3, 0).unwrap(), 3);
    }
}

#[test]
fn enforces_positive_ppe_limits_using_charges_and_saved_rows() {
    let run = value(
        r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001},"other-event":{"eventPriceUsd":0.0001}}}},"options":{"maxTotalChargeUsd":0.00025},"chargedEventCounts":{"other-event":1}}}"#,
    );
    assert_eq!(affordable_dataset_items(&run, 3, 0).unwrap(), 1);
    assert_eq!(affordable_dataset_items(&run, 3, 1).unwrap(), 0);
}

#[derive(Debug)]
struct CapturedRequest {
    method_and_path: String,
    headers: HashMap<String, String>,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> CapturedRequest {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    let (header_end, content_length) = loop {
        let read = stream.read(&mut chunk).unwrap();
        assert_ne!(read, 0, "client closed before sending a complete request");
        bytes.extend_from_slice(&chunk[..read]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..index]);
            let content_length = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                .unwrap_or(0);
            if bytes.len() >= index + 4 + content_length {
                break (index, content_length);
            }
        }
    };
    let header_text = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let mut lines = header_text.lines();
    let method_and_path = lines.next().unwrap().to_owned();
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    let body = String::from_utf8_lossy(&bytes[header_end + 4..header_end + 4 + content_length])
        .to_string();
    CapturedRequest {
        method_and_path,
        headers,
        body,
    }
}

fn mock_response(stream: &mut TcpStream, status: &str, body: &str) {
    write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
}

#[tokio::test]
async fn resolves_fetches_caps_and_persists_raw_output_with_expected_auth() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let responses = [
            (
                "200 OK",
                r##"{"hashtag":"#Cosplay","region":"us","count":2,"cursor":"0"}"##,
            ),
            (
                "200 OK",
                r#"{"code":0,"data":{"challenge_list":[{"challenge_id":"33380","cha_name":"Cosplay"}]}}"#,
            ),
            (
                "200 OK",
                r#"{"code":0,"data":{"aweme_list":[{"aweme_id":"1","desc":"clip"},{"aweme_id":"2"}],"has_more":true,"max_cursor":"next"},"processed_time":99,"raw":"retained"}"#,
            ),
            (
                "200 OK",
                r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0003}}}},"options":{"maxTotalChargeUsd":0.0003},"chargedEventCounts":{"apify-default-dataset-item":0}}}"#,
            ),
            ("201 Created", ""),
            ("201 Created", ""),
        ];
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            requests.push(read_request(&mut stream));
            mock_response(&mut stream, status, body);
        }
        requests
    });

    run_actor(&reqwest::Client::new(), &request_config(address))
        .await
        .unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 6);
    assert!(requests[0]
        .method_and_path
        .starts_with("GET /v2/key-value-stores/store-test/records/INPUT "));
    assert!(requests[1]
        .method_and_path
        .starts_with("GET /api/tiktok/challenges/search?"));
    assert!(requests[2]
        .method_and_path
        .starts_with("GET /api/tiktok/challenges/posts?"));
    assert!(requests[3]
        .method_and_path
        .starts_with("GET /v2/actor-runs/run-test "));
    assert!(requests[4]
        .method_and_path
        .starts_with("POST /v2/datasets/dataset-test/items "));
    assert!(requests[5]
        .method_and_path
        .starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT "));
    for index in [0, 3, 4, 5] {
        assert_eq!(
            requests[index]
                .headers
                .get("authorization")
                .map(String::as_str),
            Some("Bearer test-apify-token")
        );
        assert!(!requests[index].headers.contains_key("x-api-key"));
    }
    for index in [1, 2] {
        assert_eq!(
            requests[index].headers.get("x-api-key").map(String::as_str),
            Some("test-scrappa-key")
        );
        assert_eq!(
            requests[index].headers.get("accept").map(String::as_str),
            Some("application/json")
        );
    }
    let search_url = Url::parse(&format!(
        "http://mock{}",
        requests[1]
            .method_and_path
            .split_whitespace()
            .nth(1)
            .unwrap()
    ))
    .unwrap();
    let search_query = search_url
        .query_pairs()
        .into_owned()
        .collect::<HashMap<_, _>>();
    assert_eq!(search_query["keywords"], "Cosplay");
    assert_eq!(search_query["count"], "10");
    let posts_url = Url::parse(&format!(
        "http://mock{}",
        requests[2]
            .method_and_path
            .split_whitespace()
            .nth(1)
            .unwrap()
    ))
    .unwrap();
    let posts_query = posts_url
        .query_pairs()
        .into_owned()
        .collect::<HashMap<_, _>>();
    assert_eq!(posts_query["challenge_id"], "33380");
    assert!(!posts_query.contains_key("challenge_name"));
    assert_eq!(posts_query["region"], "US");
    assert_eq!(posts_query["count"], "2");
    assert_eq!(posts_query["cursor"], "0");

    let rows: Value = serde_json::from_str(&requests[4].body).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["aweme_id"], "1");
    assert_eq!(rows[0]["lookup_challenge_name"], "Cosplay");
    assert!(rows[0]["lookup_challenge_id"].is_null());
    assert_eq!(rows[0]["resolved_challenge_name"], "Cosplay");
    assert_eq!(rows[0]["resolved_challenge_id"], "33380");
    assert_eq!(rows[0]["lookup_region"], "US");
    let output: Value = serde_json::from_str(&requests[5].body).unwrap();
    assert_eq!(output["raw"], "retained");
    assert_eq!(output["data"]["aweme_list"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn zero_ppe_limit_is_uncapped_and_keeps_raw_output() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let responses = [
            ("200 OK", r#"{"challenge_id":"33380"}"#),
            (
                "200 OK",
                r#"{"code":0,"data":{"posts":[{"aweme_id":"1"}]}}"#,
            ),
            (
                "200 OK",
                r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0003}}}},"options":{"maxTotalChargeUsd":0.0},"chargedEventCounts":{}}}"#,
            ),
            ("201 Created", ""),
            ("201 Created", ""),
        ];
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            requests.push(read_request(&mut stream));
            mock_response(&mut stream, status, body);
        }
        requests
    });
    run_actor(&reqwest::Client::new(), &request_config(address))
        .await
        .unwrap();
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 5);
    assert!(requests[3]
        .method_and_path
        .starts_with("POST /v2/datasets/dataset-test/items "));
    let rows: Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert!(requests[4]
        .method_and_path
        .starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT "));
}

#[test]
fn reports_scrappa_timeout_with_the_original_deadline() {
    assert!(
        failure_message(&anyhow!("Scrappa API request timed out after 60000ms"))
            .contains("60s Scrappa API timeout")
    );
    assert_eq!(SCRAPPA_REQUEST_TIMEOUT, Duration::from_secs(60));
    assert_eq!(APIFY_REQUEST_TIMEOUT, Duration::from_secs(360));
    assert_eq!(APIFY_MAX_RETRIES, 8);
}

#[tokio::test]
async fn retries_transient_apify_errors_but_returns_client_errors_directly() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body) in [("500 Internal Server Error", "{}"), ("200 OK", "{}")] {
            let (mut stream, _) = listener.accept().unwrap();
            requests.push(read_request(&mut stream));
            mock_response(&mut stream, status, body);
        }
        requests
    });
    let client = reqwest::Client::new();
    let url = format!("http://{address}/transient");
    let response = send_apify_request("test request", || client.get(&url))
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(server.join().unwrap().len(), 2);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_request(&mut stream);
        mock_response(&mut stream, "400 Bad Request", "{} ");
        request
    });
    let url = format!("http://{address}/permanent");
    let response = send_apify_request("test request", || client.get(&url))
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);
    server.join().unwrap();
}

#[tokio::test]
async fn returns_scrappa_http_errors_without_retrying() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_request(&mut stream);
        mock_response(
            &mut stream,
            "503 Service Unavailable",
            "upstream unavailable",
        );
        request
    });
    let config = request_config(address);
    let error = fetch_scrappa_response(
        &reqwest::Client::new(),
        &config,
        &["tiktok", "challenges", "posts"],
        &[],
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("Scrappa API error (503)"));
    assert!(server
        .join()
        .unwrap()
        .method_and_path
        .starts_with("GET /api/tiktok/challenges/posts "));
}
