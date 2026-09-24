use super::*;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

#[derive(Debug)]
struct MockResponse {
    status: u16,
    body: String,
    delay: Duration,
}

impl MockResponse {
    fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            body: body.to_string(),
            delay: Duration::ZERO,
        }
    }

    fn delayed_json(status: u16, body: Value, delay: Duration) -> Self {
        Self {
            status,
            body: body.to_string(),
            delay,
        }
    }
}

#[derive(Debug)]
struct CapturedRequest {
    method: String,
    path: String,
    headers: String,
    body: String,
}

async fn start_mock_server(
    responses: Vec<MockResponse>,
) -> (Url, JoinHandle<Vec<CapturedRequest>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut requests = Vec::new();
        for response in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            requests.push(read_request(&mut stream).await);
            if !response.delay.is_zero() {
                tokio::time::sleep(response.delay).await;
            }
            let reason = StatusCode::from_u16(response.status)
                .ok()
                .and_then(|status| status.canonical_reason())
                .unwrap_or("Unknown status");
            let response_bytes = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.status,
                reason,
                response.body.len(),
                response.body,
            );
            let _ = stream.write_all(response_bytes.as_bytes()).await;
        }
        requests
    });
    (Url::parse(&format!("http://{address}/")).unwrap(), server)
}

async fn read_request(stream: &mut TcpStream) -> CapturedRequest {
    let mut bytes = Vec::new();
    let mut chunk = [0; 2048];
    loop {
        let read = stream.read(&mut chunk).await.unwrap();
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
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

    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap();
    let headers = String::from_utf8_lossy(&bytes[..header_end]).into_owned();
    let first_line = headers.lines().next().unwrap();
    let mut request_line = first_line.split_whitespace();
    let method = request_line.next().unwrap().to_owned();
    let path = request_line.next().unwrap().to_owned();
    let body = String::from_utf8_lossy(&bytes[header_end + 4..]).into_owned();
    CapturedRequest {
        method,
        path,
        headers,
        body,
    }
}

fn scrappa_client(base_url: Url) -> ScrappaClient {
    ScrappaClient::new(Client::new(), base_url, "test-api-key".to_owned())
}

fn config(base_url: Url) -> Config {
    Config {
        apify_api_base: base_url.clone(),
        apify_token: "test-apify-token".to_owned(),
        actor_run_id: "run-1".to_owned(),
        key_value_store_id: "store-1".to_owned(),
        dataset_id: "dataset-1".to_owned(),
        input_key: INPUT_KEY_DEFAULT.to_owned(),
        scrappa_api_base: base_url,
        scrappa_api_key: "test-api-key".to_owned(),
    }
}

fn run_pricing(max_charge: f64, charged_item_count: u64) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0002},
                    "other-result": {"eventPriceUsd": 0.0001}
                }}
            },
            "options": {"maxTotalChargeUsd": max_charge},
            "chargedEventCounts": {
                "apify-default-dataset-item": charged_item_count,
                "other-result": 1
            }
        }
    })
}

#[test]
fn input_keeps_url_priority_and_legacy_shortcode_alias() {
    assert_eq!(
        resolve_input(&json!({
            "url": " https://www.instagram.com/natgeo/p/DXHKcyvEWfr/ ",
            "shortcode": "SHOULD_NOT_BE_USED"
        }))
        .unwrap(),
        PostRequest {
            identifier: "https://www.instagram.com/natgeo/p/DXHKcyvEWfr/".to_owned(),
            url: Some("https://www.instagram.com/natgeo/p/DXHKcyvEWfr/".to_owned()),
            shortcode: None,
        }
    );
    assert_eq!(
        resolve_input(&json!({"url": "", "shortcode": " ", "media_id": "DXHKcyvEWfr"})).unwrap(),
        PostRequest {
            identifier: "DXHKcyvEWfr".to_owned(),
            url: None,
            shortcode: Some("DXHKcyvEWfr".to_owned()),
        }
    );
    assert!(resolve_input(&Value::Null)
        .unwrap_err()
        .to_string()
        .contains("Instagram post URL or shortcode is required"));
}

#[test]
fn input_prefill_stays_a_url_without_becoming_an_api_default() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    let prefill = schema["properties"]["url"]["prefill"].as_str().unwrap();
    assert!(get_post_identity(Some(prefill)).is_some());
    assert!(schema["properties"]["url"].get("default").is_none());
    assert_eq!(
        resolve_input(&json!({"shortcode": "CUSTOM_POST"}))
            .unwrap()
            .shortcode
            .as_deref(),
        Some("CUSTOM_POST")
    );
}

#[test]
fn url_detection_and_identity_match_supported_instagram_post_urls() {
    assert!(looks_like_url("https://instagram.com/user/p/POST"));
    assert!(looks_like_url("instagram.com/user/p/POST"));
    assert!(!looks_like_url("POST"));
    assert_eq!(
        get_post_identity(Some("instagram.com/name.1/reels/ABC-123/")),
        Some(PostIdentity {
            username: "name.1".to_owned(),
            shortcode: "ABC-123".to_owned(),
        })
    );
    assert!(get_post_identity(Some("https://example.com/user/p/ABC")).is_none());
    assert!(get_post_identity(Some("https://instagram.com/user/story/ABC")).is_none());
}

#[test]
fn transient_retry_rules_keep_non_retryable_and_auth_responses_terminal() {
    let transient = ScrappaError::http(
        503,
        "temporarily unavailable".to_owned(),
        Some(json!({"message": "temporarily unavailable", "retryable": true})),
    );
    assert!(transient.is_transient());
    assert!(ScrappaError::http(
        429,
        "rate limited".to_owned(),
        Some(json!({"error": "Rate limited (HTTP 429)"})),
    )
    .is_rate_limit());
    assert!(!ScrappaError::http(
        429,
        "rate limited".to_owned(),
        Some(json!({"error": "Rate limited", "retryable": false})),
    )
    .is_transient());
    assert!(!ScrappaError::http(
        401,
        "Authentication timeout".to_owned(),
        Some(json!({"message": "Authentication timeout"})),
    )
    .is_transient());
    assert!(ScrappaError::http(
        401,
        "Authentication required".to_owned(),
        Some(json!({"message": "Authentication required"})),
    )
    .is_cooldown_auth());
    assert!(!ScrappaError::http(
        401,
        "Authentication required".to_owned(),
        Some(json!({"message": "Authentication required", "retryable": false})),
    )
    .is_cooldown_auth());
}

#[test]
fn positive_spending_limit_accounts_for_all_event_charges() {
    assert_eq!(
        affordable_dataset_items(&run_pricing(0.0006, 1), 1).unwrap(),
        1
    );
    assert_eq!(
        affordable_dataset_items(&run_pricing(0.0003, 1), 1).unwrap(),
        0
    );
    assert!(affordable_dataset_items(&json!({"data": {}}), 1).is_err());
}

#[test]
fn free_pricing_allows_requested_dataset_items() {
    let run = json!({"data": {"pricingInfo": {"pricingModel": "FREE"}}});
    assert_eq!(affordable_dataset_items(&run, 1).unwrap(), 1);
}

#[test]
fn missing_null_and_zero_spending_limits_are_unbounded() {
    let mut run = run_pricing(0.0001, 100);
    run["data"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("maxTotalChargeUsd");
    assert_eq!(affordable_dataset_items(&run, 1).unwrap(), 1);

    for max_charge in [Value::Null, json!(0)] {
        let mut run = run_pricing(0.0001, 100);
        run["data"]["options"]["maxTotalChargeUsd"] = max_charge;
        assert_eq!(affordable_dataset_items(&run, 1).unwrap(), 1);
    }
}

#[tokio::test]
async fn feed_fallback_returns_only_the_requested_post_with_api_auth() {
    let post = json!({"shortcode": "Dc30nJeRKKz", "caption": "Actual caption"});
    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(
            503,
            json!({"message": "Temporarily unavailable", "retryable": true}),
        ),
        MockResponse::json(
            200,
            json!({"success": true, "posts": [
                {"shortcode": "OTHER"}, post.clone()
            ]}),
        ),
    ])
    .await;
    let client = scrappa_client(base_url);
    let request =
        resolve_input(&json!({"url": "https://www.instagram.com/instagram/p/Dc30nJeRKKz/"}))
            .unwrap();
    let deadline = Instant::now() + REQUEST_TIMEOUT;
    let result = client.fetch_post(&request, deadline).await.unwrap();
    assert_eq!(
        result,
        json!({"success": true, "found": true, "data": post})
    );

    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].path.starts_with(
        "/instagram/post?url=https%3A%2F%2Fwww.instagram.com%2Finstagram%2Fp%2FDc30nJeRKKz%2F"
    ));
    assert!(requests[1]
        .path
        .starts_with("/instagram/user/posts?username=instagram"));
    for request in requests {
        assert!(request
            .headers
            .to_ascii_lowercase()
            .contains("x-api-key: test-api-key"));
        assert!(request
            .headers
            .to_ascii_lowercase()
            .contains("accept: application/json"));
    }
}

#[tokio::test]
async fn missing_feed_match_returns_the_original_single_post_error() {
    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(
            503,
            json!({"message": "Temporarily unavailable", "retryable": true}),
        ),
        MockResponse::json(
            200,
            json!({"success": true, "posts": [{"shortcode": "OTHER"}]}),
        ),
    ])
    .await;
    let client = scrappa_client(base_url);
    let request =
        resolve_input(&json!({"url": "https://www.instagram.com/instagram/p/REQUESTED/"})).unwrap();
    let error = client
        .fetch_post(&request, Instant::now() + REQUEST_TIMEOUT)
        .await
        .unwrap_err();
    assert_eq!(error.http_status, Some(503));
    assert_eq!(error.message, "Temporarily unavailable");
    assert_eq!(server.await.unwrap().len(), 2);
}

#[tokio::test]
async fn fallback_is_limited_to_transient_or_login_required_url_failures() {
    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(
            403,
            json!({"error_code": "instagram_login_required", "message": "Login required"}),
        ),
        MockResponse::json(
            200,
            json!({"success": true, "posts": [{"shortcode": "CODE"}]}),
        ),
    ])
    .await;
    let client = scrappa_client(base_url);
    let request = resolve_input(&json!({"url": "https://www.instagram.com/name/p/CODE/"})).unwrap();
    let result = client
        .fetch_post(&request, Instant::now() + REQUEST_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(result["data"]["shortcode"], "CODE");
    assert_eq!(server.await.unwrap().len(), 2);

    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(
            200,
            json!({
                "success": false,
                "status_code": 403,
                "error_code": "instagram_login_required",
                "message": "Login required"
            }),
        ),
        MockResponse::json(
            200,
            json!({"success": true, "posts": [{"shortcode": "CODE"}]}),
        ),
    ])
    .await;
    let client = scrappa_client(base_url);
    let request = resolve_input(&json!({"url": "https://www.instagram.com/name/p/CODE/"})).unwrap();
    let result = client
        .fetch_post(&request, Instant::now() + REQUEST_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(result["data"]["shortcode"], "CODE");
    assert_eq!(server.await.unwrap().len(), 2);

    for (input, failure) in [
        (
            json!({"url": "https://www.instagram.com/name/p/CODE/"}),
            json!({"error_code": "invalid_api_key", "message": "Invalid API key"}),
        ),
        (
            json!({"url": "https://www.instagram.com/name/p/CODE/"}),
            json!({"message": "Rate limited", "retryable": false}),
        ),
        (
            json!({"shortcode": "CODE"}),
            json!({"message": "Temporarily unavailable", "retryable": true}),
        ),
    ] {
        let (base_url, server) = start_mock_server(vec![MockResponse::json(503, failure)]).await;
        let client = scrappa_client(base_url);
        let request = resolve_input(&input).unwrap();
        assert!(client
            .fetch_post(&request, Instant::now() + REQUEST_TIMEOUT)
            .await
            .is_err());
        assert_eq!(server.await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn authentication_error_is_not_retried_without_a_rate_limit() {
    let (base_url, server) = start_mock_server(vec![MockResponse::json(
        401,
        json!({"message": "Authentication required"}),
    )])
    .await;
    let client = scrappa_client(base_url);
    let request = resolve_input(&json!({"shortcode": "CODE"})).unwrap();
    let error = request_with_retry_policy(
        &client,
        &request,
        &[Duration::ZERO, Duration::ZERO],
        REQUEST_TIMEOUT,
    )
    .await
    .unwrap_err();
    assert_eq!(error.http_status, Some(401));
    assert_eq!(server.await.unwrap().len(), 1);
}

#[tokio::test]
async fn rate_limit_can_be_followed_by_a_cooldown_auth_retry() {
    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(500, json!({"error": "Rate limited (HTTP 429)"})),
        MockResponse::json(401, json!({"error": "Authentication required (HTTP 401)"})),
        MockResponse::json(200, json!({"success": true, "data": {"shortcode": "CODE"}})),
    ])
    .await;
    let client = scrappa_client(base_url);
    let request = resolve_input(&json!({"shortcode": "CODE"})).unwrap();
    let result = request_with_retry_policy(
        &client,
        &request,
        &[Duration::ZERO, Duration::ZERO],
        REQUEST_TIMEOUT,
    )
    .await
    .unwrap();
    assert_eq!(result["data"]["shortcode"], "CODE");
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 3);
    assert!(requests
        .iter()
        .all(|request| request.path.starts_with("/instagram/post?shortcode=CODE")));
}

#[tokio::test]
async fn fallback_shares_the_single_attempt_deadline() {
    let (base_url, server) = start_mock_server(vec![
        MockResponse::delayed_json(
            503,
            json!({"message": "Temporarily unavailable", "retryable": true}),
            Duration::from_millis(35),
        ),
        MockResponse::delayed_json(
            200,
            json!({"success": true, "posts": []}),
            Duration::from_millis(100),
        ),
    ])
    .await;
    let client = scrappa_client(base_url);
    let request =
        resolve_input(&json!({"url": "https://www.instagram.com/instagram/p/CODE/"})).unwrap();
    let error = request_with_retry_policy(&client, &request, &[], Duration::from_millis(70))
        .await
        .unwrap_err();
    assert!(error.timed_out);
    assert_eq!(server.await.unwrap().len(), 2);
}

#[tokio::test]
async fn actor_publishes_raw_response_to_dataset_and_output_store() {
    let output = json!({"success": true, "data": {"shortcode": "CODE", "caption": "hello"}});
    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(200, json!({"url": "CODE"})),
        MockResponse::json(200, output.clone()),
        MockResponse::json(201, json!({})),
        MockResponse::json(200, run_pricing(1.0, 0)),
        MockResponse::json(201, json!({})),
    ])
    .await;
    run_actor_with_config(config(base_url)).await.unwrap();
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 5);
    assert_eq!(requests[0].method, "GET");
    assert!(requests[0].path.ends_with("/records/INPUT"));
    assert!(requests[1]
        .path
        .starts_with("/instagram/post?shortcode=CODE"));
    assert_eq!(requests[2].method, "PUT");
    assert!(requests[2].path.ends_with("/records/OUTPUT"));
    assert_eq!(
        serde_json::from_str::<Value>(&requests[2].body).unwrap(),
        output
    );
    assert_eq!(requests[3].path, "/v2/actor-runs/run-1");
    assert_eq!(requests[4].method, "POST");
    assert_eq!(requests[4].path, "/v2/datasets/dataset-1/items");
    assert_eq!(
        serde_json::from_str::<Value>(&requests[4].body).unwrap(),
        output
    );
    assert!(requests
        .iter()
        .filter(|request| request.path.starts_with("/v2/"))
        .all(|request| request
            .headers
            .to_ascii_lowercase()
            .contains("authorization: bearer test-apify-token")));
}

#[tokio::test]
async fn actor_publishes_dataset_item_for_free_and_unbounded_ppe_runs() {
    let free_run = json!({"data": {"pricingInfo": {"pricingModel": "FREE"}}});
    let mut ppe_without_limit = run_pricing(1.0, 0);
    ppe_without_limit["data"]["options"]
        .as_object_mut()
        .unwrap()
        .remove("maxTotalChargeUsd");
    let mut ppe_null_limit = run_pricing(1.0, 0);
    ppe_null_limit["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
    let ppe_zero_limit = run_pricing(1.0, 0);

    for run in [free_run, ppe_without_limit, ppe_null_limit, ppe_zero_limit] {
        let output = json!({"success": true, "data": {"shortcode": "CODE"}});
        let (base_url, server) = start_mock_server(vec![
            MockResponse::json(200, json!({"shortcode": "CODE"})),
            MockResponse::json(200, output.clone()),
            MockResponse::json(201, json!({})),
            MockResponse::json(200, run),
            MockResponse::json(201, json!({})),
        ])
        .await;

        run_actor_with_config(config(base_url)).await.unwrap();

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 5);
        assert!(requests[2].path.ends_with("/records/OUTPUT"));
        assert_eq!(requests[3].path, "/v2/actor-runs/run-1");
        assert_eq!(requests[4].path, "/v2/datasets/dataset-1/items");
        assert_eq!(
            serde_json::from_str::<Value>(&requests[4].body).unwrap(),
            output
        );
    }
}

#[tokio::test]
async fn exhausted_budget_skips_dataset_charge_but_keeps_output_record() {
    let output = json!({"success": true, "data": {"shortcode": "CODE"}});
    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(200, json!({"shortcode": "CODE"})),
        MockResponse::json(200, output.clone()),
        MockResponse::json(201, json!({})),
        MockResponse::json(200, run_pricing(0.0001, 0)),
    ])
    .await;
    run_actor_with_config(config(base_url)).await.unwrap();
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests
        .iter()
        .all(|request| !request.path.starts_with("/v2/datasets/")));
    assert!(requests[2].path.ends_with("/records/OUTPUT"));
    assert_eq!(requests[3].path, "/v2/actor-runs/run-1");
}

#[tokio::test]
async fn pricing_failure_keeps_successful_response_in_output_store() {
    let output = json!({"success": true, "data": {"shortcode": "CODE"}});
    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(200, json!({"shortcode": "CODE"})),
        MockResponse::json(200, output.clone()),
        MockResponse::json(201, json!({})),
        MockResponse::json(400, json!({"error": "pricing unavailable"})),
    ])
    .await;

    let error = run_actor_with_config(config(base_url)).await.unwrap_err();
    assert!(error
        .to_string()
        .contains("Apify run pricing request failed (400)"));

    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[2].path.ends_with("/records/OUTPUT"));
    assert_eq!(requests[2].method, "PUT");
    assert_eq!(
        serde_json::from_str::<Value>(&requests[2].body).unwrap(),
        output
    );
    assert_eq!(requests[3].path, "/v2/actor-runs/run-1");
    assert!(requests
        .iter()
        .all(|request| !request.path.starts_with("/v2/datasets/")));
}

#[tokio::test]
async fn failed_lookup_does_not_publish_a_dataset_item_or_output_record() {
    let (base_url, server) = start_mock_server(vec![
        MockResponse::json(200, json!({"shortcode": "CODE"})),
        MockResponse::json(
            503,
            json!({"message": "Upstream rejected the request", "retryable": false}),
        ),
    ])
    .await;
    let error = run_actor_with_config(config(base_url)).await.unwrap_err();
    assert_eq!(
        actor_failure_message(&error),
        "Scrappa Instagram Post API request failed (503): Upstream rejected the request"
    );
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|request| {
        !request.path.starts_with("/v2/datasets/") && !request.path.ends_with("/records/OUTPUT")
    }));
}
