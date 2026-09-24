use super::*;
use crate::{
    apify::{
        ApifyActor, ApifyClient, RetryPolicy, RunPricing, DEFAULT_DATASET_ITEM_EVENT, RESULT_EVENT,
    },
    input::{parse_input, ChallengeRequest},
    ports::{PushResult, ResultsSink},
    response::{get_video_id, js_string, parse_page},
    scrape::{is_total_failure, scrape_challenge, ChallengeSummary},
    scrappa::{PostsParams, ScrappaApi, ScrappaClient},
};
use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};
use std::{
    collections::{HashSet, VecDeque},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};
use url::Url;

fn request(input: Value) -> ChallengeRequest {
    parse_input(&input).unwrap().remove(0)
}

#[test]
fn input_batches_unique_ids_and_bounds_page_size_to_result_limit() {
    assert_eq!(
        parse_input(&json!({
            "challenge_ids": [" 1 ", "2", "1"],
            "region": "us",
            "results_per_challenge": 20,
            "page_size": 50
        }))
        .unwrap(),
        vec![
            ChallengeRequest {
                challenge_id: "1".to_owned(),
                region: Some("US".to_owned()),
                initial_cursor: None,
                result_limit: 20,
                page_size: 20,
            },
            ChallengeRequest {
                challenge_id: "2".to_owned(),
                region: Some("US".to_owned()),
                initial_cursor: None,
                result_limit: 20,
                page_size: 20,
            },
        ]
    );
}

#[test]
fn input_keeps_legacy_single_id_and_numeric_cursor_compatibility() {
    assert_eq!(
        parse_input(&json!({
            "challenge_ids": [],
            "challenge_id": 1622962893630470_u64,
            "cursor": 10.0
        }))
        .unwrap(),
        vec![ChallengeRequest {
            challenge_id: "1622962893630470".to_owned(),
            region: None,
            initial_cursor: Some("10".to_owned()),
            result_limit: 100,
            page_size: 10,
        }]
    );
    assert_eq!(
        parse_input(&json!({ "challenge_id": "1", "cursor": -10 })).unwrap()[0]
            .initial_cursor
            .as_deref(),
        Some("-10")
    );
}

#[test]
fn input_rejects_missing_ids_regions_and_invalid_limits() {
    assert!(parse_input(&json!({}))
        .unwrap_err()
        .to_string()
        .contains("numeric TikTok challenge ID"));
    assert!(parse_input(&json!({ "challenge_ids": ["abc"] }))
        .unwrap_err()
        .to_string()
        .contains("numeric TikTok challenge ID"));
    assert!(
        parse_input(&json!({ "challenge_id": "1", "region": "USA" }))
            .unwrap_err()
            .to_string()
            .contains("two-letter")
    );
    assert!(parse_input(&json!({ "challenge_id": "1", "region": 123 }))
        .unwrap_err()
        .to_string()
        .contains("two-letter"));
    assert!(parse_input(&json!({ "challenge_id": "1", "page_size": 0 }))
        .unwrap_err()
        .to_string()
        .contains("page_size"));
    assert!(
        parse_input(&json!({ "challenge_id": "1", "results_per_challenge": "10" }))
            .unwrap_err()
            .to_string()
            .contains("results_per_challenge")
    );
}

#[test]
fn input_enforces_batch_and_total_result_caps_after_deduplication() {
    let too_many_ids = (1..=21).map(|id| id.to_string()).collect::<Vec<_>>();
    assert!(parse_input(&json!({ "challenge_ids": too_many_ids }))
        .unwrap_err()
        .to_string()
        .contains("maximum of 20"));

    assert!(parse_input(&json!({
        "challenge_ids": ["1", "2", "3", "4", "5"],
        "results_per_challenge": 500
    }))
    .unwrap_err()
    .to_string()
    .contains("cannot exceed 2000"));

    assert_eq!(
        parse_input(&json!({
            "challenge_ids": ["1", "1"],
            "results_per_challenge": 500
        }))
        .unwrap()
        .len(),
        1
    );
}

#[test]
fn actor_configuration_preserves_input_prefill_budget_and_wrapper_resources() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    let actor: Value = serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();
    let dockerfile = include_str!("../.actor/Dockerfile");
    let readme = include_str!("../README.md");

    assert_eq!(
        schema.pointer("/properties/challenge_ids/prefill"),
        Some(&json!(["1622962893630470"]))
    );
    assert_eq!(
        schema.pointer("/properties/challenge_id/title"),
        Some(&json!("Single Challenge ID (legacy)"))
    );
    assert_eq!(actor.pointer("/resources/memoryMbytes"), Some(&json!(128)));
    assert_eq!(
        actor.pointer("/defaultRunOptions/timeoutSecs"),
        Some(&json!(300))
    );
    assert_eq!(
        actor.pointer("/defaultRunOptions/memoryMbytes"),
        Some(&json!(128))
    );
    assert_eq!(
        actor.pointer("/defaultRunOptions/maxItems"),
        Some(&json!(2000))
    );
    assert!(dockerfile.contains("FROM rust:1.90-slim-bookworm AS builder"));
    assert!(!dockerfile.contains("apify/actor-node"));
    assert!(readme.contains("challenge-post-result"));
    assert!(readme.contains("$0.00025 per video"));
    assert!(readme.contains("No per-video key-value-store records are written"));
}

#[test]
fn response_parsing_supports_primary_and_fallback_pagination_shapes() {
    assert_eq!(
        parse_page(Some(&json!({
            "videos": [{"video_id": "1"}],
            "cursor": 10,
            "hasMore": true
        }))),
        crate::response::Page {
            videos: vec![json!({"video_id": "1"}).as_object().unwrap().clone()],
            cursor: Some("10".to_owned()),
            has_more: true,
        }
    );
    assert_eq!(
        parse_page(Some(&json!({"posts": [{"video_id": "2"}]})))
            .videos
            .len(),
        1
    );
    assert_eq!(
        parse_page(Some(&json!({
            "aweme_list": [{"aweme_id": "3"}],
            "max_cursor": "20",
            "has_more": false
        })))
        .cursor
        .as_deref(),
        Some("20")
    );
    assert_eq!(
        parse_page(Some(&json!([{"id": "4"}, null, 3])))
            .videos
            .len(),
        1
    );
}

#[test]
fn response_parsing_preserves_nullish_fallback_and_video_identifier_rules() {
    assert!(
        parse_page(Some(&json!({
            "hasMore": null,
            "has_more": "1"
        })))
        .has_more
    );
    assert!(
        !parse_page(Some(&json!({
            "hasMore": false,
            "has_more": 1
        })))
        .has_more
    );
    assert!(parse_page(Some(&json!({"hasMore": 0}))).has_more == false);
    assert_eq!(
        get_video_id(&object(json!({"video_id": "1", "aweme_id": "2"}))),
        Some("1".to_owned())
    );
    assert_eq!(
        get_video_id(&object(json!({"aweme_id": 2.0}))),
        Some("2".to_owned())
    );
    assert_eq!(get_video_id(&object(json!({}))), None);
    assert_eq!(js_string(&json!([1, null, "x"])), "1,,x");
}

fn object(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

#[derive(Default)]
struct MockScrappa {
    responses: Mutex<VecDeque<Value>>,
    calls: Mutex<Vec<PostsParams>>,
    error: Option<String>,
}

impl MockScrappa {
    fn with_responses(responses: impl IntoIterator<Item = Value>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().collect()),
            calls: Mutex::new(Vec::new()),
            error: None,
        }
    }

    fn failing(message: &str) -> Self {
        Self {
            error: Some(message.to_owned()),
            ..Self::default()
        }
    }

    fn calls(&self) -> Vec<PostsParams> {
        self.calls.lock().unwrap().clone()
    }
}

impl ScrappaApi for MockScrappa {
    async fn get_posts(&self, params: &PostsParams) -> Result<Value> {
        self.calls.lock().unwrap().push(params.clone());
        if let Some(error) = &self.error {
            return Err(anyhow!(error.clone()));
        }
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| anyhow!("Mock Scrappa response queue is empty"))
    }
}

struct MockSink {
    is_pay_per_event: bool,
    capacity: usize,
    rows: Vec<Value>,
    charged_rows: usize,
}

impl MockSink {
    fn new(is_pay_per_event: bool, capacity: usize) -> Self {
        Self {
            is_pay_per_event,
            capacity,
            rows: Vec::new(),
            charged_rows: 0,
        }
    }
}

impl ResultsSink for MockSink {
    fn available_capacity(&self, requested: usize) -> usize {
        if !self.is_pay_per_event {
            return requested;
        }
        requested.min(self.capacity.saturating_sub(self.rows.len()))
    }

    async fn push_videos(&mut self, rows: &[Value]) -> Result<PushResult> {
        if !self.is_pay_per_event {
            self.rows.extend_from_slice(rows);
            return Ok(PushResult {
                saved: rows.len(),
                limit_reached: false,
            });
        }

        let mut saved = 0;
        for row in rows {
            if self.rows.len() >= self.capacity {
                return Ok(PushResult {
                    saved,
                    limit_reached: true,
                });
            }
            self.rows.push(row.clone());
            self.charged_rows += 1;
            saved += 1;
            if self.rows.len() >= self.capacity {
                return Ok(PushResult {
                    saved,
                    limit_reached: true,
                });
            }
        }
        Ok(PushResult {
            saved,
            limit_reached: false,
        })
    }
}

#[tokio::test]
async fn scrape_paginates_deduplicates_and_saves_one_row_per_charged_video() {
    let client = MockScrappa::with_responses([
        json!({
            "code": 0,
            "data": {
                "videos": [{"video_id": "1"}, {"video_id": "2"}],
                "cursor": 2,
                "hasMore": true
            }
        }),
        json!({
            "code": 0,
            "data": {
                "videos": [{"video_id": "2"}, {"video_id": "3"}],
                "cursor": 4,
                "hasMore": false
            }
        }),
    ]);
    let mut sink = MockSink::new(true, usize::MAX);
    let mut seen = HashSet::new();
    let result = scrape_challenge(
        &client,
        &mut sink,
        &request(json!({
            "challenge_id": "123",
            "region": "US",
            "results_per_challenge": 10,
            "page_size": 2
        })),
        &mut seen,
    )
    .await;

    assert_eq!(result.status, "succeeded");
    assert_eq!(result.videos_saved, 3);
    assert_eq!(result.pages_fetched, 2);
    assert_eq!(sink.rows.len(), 3);
    assert_eq!(sink.charged_rows, sink.rows.len());
    assert_eq!(
        sink.rows
            .iter()
            .map(|row| row["video_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["1", "2", "3"]
    );
    assert!(sink.rows.iter().all(|row| row["challenge_id"] == "123"));
    assert!(sink.rows.iter().all(|row| row["requested_region"] == "US"));
    assert!(sink
        .rows
        .iter()
        .all(|row| row["scraped_at"].as_str().unwrap().ends_with('Z')));
    assert_eq!(client.calls()[0].cursor, None);
    assert_eq!(client.calls()[1].cursor.as_deref(), Some("2"));
}

#[tokio::test]
async fn scrape_checks_charge_capacity_before_fetching_a_page() {
    let client = MockScrappa::with_responses([]);
    let mut sink = MockSink::new(true, 0);
    let mut seen = HashSet::new();
    let result = scrape_challenge(
        &client,
        &mut sink,
        &request(json!({"challenge_id": "123"})),
        &mut seen,
    )
    .await;

    assert_eq!(result.status, "charge-limit-reached");
    assert_eq!(result.pages_fetched, 0);
    assert!(client.calls().is_empty());
}

#[tokio::test]
async fn scrape_continues_over_duplicate_only_pages_when_cursor_advances() {
    let client = MockScrappa::with_responses([
        json!({"code": 0, "data": {"videos": [{"video_id": "1"}], "cursor": "a", "hasMore": true}}),
        json!({"code": 0, "data": {"videos": [{"video_id": "1"}], "cursor": "b", "hasMore": true}}),
        json!({"code": 0, "data": {"videos": [{"video_id": "2"}], "cursor": "c", "hasMore": false}}),
    ]);
    let mut sink = MockSink::new(true, usize::MAX);
    let mut seen = HashSet::new();
    let result = scrape_challenge(
        &client,
        &mut sink,
        &request(json!({
            "challenge_id": "123",
            "results_per_challenge": 2,
            "page_size": 1
        })),
        &mut seen,
    )
    .await;

    assert_eq!(result.status, "succeeded");
    assert_eq!(result.videos_saved, 2);
    assert_eq!(client.calls().len(), 3);
}

#[tokio::test]
async fn scrape_deduplicates_across_challenges_in_one_run() {
    let first_client = MockScrappa::with_responses([
        json!({"code": 0, "data": {"videos": [{"video_id": "shared"}]}}),
    ]);
    let second_client = MockScrappa::with_responses([
        json!({"code": 0, "data": {"videos": [{"video_id": "shared"}]}}),
    ]);
    let mut sink = MockSink::new(true, usize::MAX);
    let mut seen = HashSet::new();

    let first = scrape_challenge(
        &first_client,
        &mut sink,
        &request(json!({"challenge_id": "1"})),
        &mut seen,
    )
    .await;
    let second = scrape_challenge(
        &second_client,
        &mut sink,
        &request(json!({"challenge_id": "2"})),
        &mut seen,
    )
    .await;

    assert_eq!(first.videos_saved, 1);
    assert_eq!(second.videos_saved, 0);
    assert_eq!(sink.rows.len(), 1);
    assert_eq!(sink.charged_rows, 1);
}

#[tokio::test]
async fn scrape_reports_repeated_initial_cursors_and_the_hard_page_ceiling() {
    let repeated = MockScrappa::with_responses([json!({
        "code": 0,
        "data": {"videos": [], "cursor": "10", "hasMore": true}
    })]);
    let mut sink = MockSink::new(true, usize::MAX);
    let mut seen = HashSet::new();
    let stalled = scrape_challenge(
        &repeated,
        &mut sink,
        &request(json!({"challenge_id": "123", "cursor": "10"})),
        &mut seen,
    )
    .await;
    assert_eq!(stalled.status, "pagination-stalled");
    assert_eq!(stalled.pages_fetched, 1);

    let pages = (1..=100).map(
        |cursor| json!({"code": 0, "data": {"videos": [], "cursor": cursor, "hasMore": true}}),
    );
    let page_limited = MockScrappa::with_responses(pages);
    let mut sink = MockSink::new(true, usize::MAX);
    let limited = scrape_challenge(
        &page_limited,
        &mut sink,
        &request(json!({
            "challenge_id": "123",
            "results_per_challenge": 500,
            "page_size": 1
        })),
        &mut HashSet::new(),
    )
    .await;
    assert_eq!(limited.status, "page-limit-reached");
    assert_eq!(limited.pages_fetched, 100);
}

#[tokio::test]
async fn scrape_isolates_upstream_failures_and_marks_only_total_failure_as_actor_failure() {
    let failed_client = MockScrappa::with_responses([json!({"code": 429, "msg": "rate limited"})]);
    let mut sink = MockSink::new(true, usize::MAX);
    let failed = scrape_challenge(
        &failed_client,
        &mut sink,
        &request(json!({"challenge_id": "123"})),
        &mut HashSet::new(),
    )
    .await;
    assert_eq!(failed.status, "failed");
    assert_eq!(
        failed.error.as_deref(),
        Some("Scrappa API code 429: rate limited")
    );
    assert!(is_total_failure(std::slice::from_ref(&failed)));
    assert!(!is_total_failure(&[]));

    let mixed = [
        failed,
        ChallengeSummary {
            challenge_id: "456".to_owned(),
            status: "succeeded",
            videos_saved: 0,
            pages_fetched: 1,
            next_cursor: None,
            error: None,
        },
    ];
    assert!(!is_total_failure(&mixed));

    let network_client = MockScrappa::failing("Scrappa is unavailable");
    let network_failure = scrape_challenge(
        &network_client,
        &mut sink,
        &request(json!({"challenge_id": "789"})),
        &mut HashSet::new(),
    )
    .await;
    assert_eq!(network_failure.status, "failed");
    assert_eq!(
        network_failure.error.as_deref(),
        Some("Scrappa is unavailable")
    );
}

fn pricing_response(max_total_charge_usd: Value, counts: Value) -> Value {
    json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {
                    "actorChargeEvents": {
                        "challenge-post-result": {"eventPriceUsd": 0.00025},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                        "other-event": {"eventPriceUsd": 0.0002}
                    }
                }
            },
            "options": {"maxTotalChargeUsd": max_total_charge_usd},
            "chargedEventCounts": counts
        }
    })
}

#[test]
fn ppe_capacity_accounts_for_prior_charges_and_the_custom_event_price() {
    let pricing =
        RunPricing::from_run_response(&pricing_response(json!(0.001), json!({"other-event": 1})))
            .unwrap();

    assert_eq!(pricing.available_capacity(RESULT_EVENT, 10), 3);
    assert_eq!(
        pricing.available_capacity(DEFAULT_DATASET_ITEM_EVENT, 10),
        8
    );

    let zero_budget =
        RunPricing::from_run_response(&pricing_response(json!(0), json!({}))).unwrap();
    assert_eq!(zero_budget.available_capacity(RESULT_EVENT, 10), 0);
}

#[test]
fn non_ppe_runs_keep_unbounded_dataset_capacity() {
    let pricing = RunPricing::from_run_response(&json!({
        "data": {
            "pricingInfo": {"pricingModel": "FLAT_RATE"},
            "options": {},
            "chargedEventCounts": {}
        }
    }))
    .unwrap();
    assert_eq!(pricing.available_capacity(RESULT_EVENT, 2_000), 2_000);
}

#[derive(Debug, Clone)]
struct RecordedRequest {
    method: String,
    path: String,
    headers: std::collections::HashMap<String, String>,
    body: Vec<u8>,
}

struct MockServer {
    url: Url,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    task: JoinHandle<()>,
}

impl MockServer {
    async fn start<F>(handler: F) -> Self
    where
        F: Fn(RecordedRequest) -> (u16, String) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let shared_requests = requests.clone();
        let handler = Arc::new(handler);
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                let requests = shared_requests.clone();
                let handler = handler.clone();
                tokio::spawn(async move {
                    let Ok((stream, request)) = read_request(stream).await else {
                        return;
                    };
                    requests.lock().unwrap().push(request.clone());
                    let (status, body) = handler(request);
                    let reason = match status {
                        200 => "OK",
                        201 => "Created",
                        401 => "Unauthorized",
                        404 => "Not Found",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
                        503 => "Service Unavailable",
                        _ => "Mock Response",
                    };
                    let response = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let mut stream = stream;
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.flush().await;
                });
            }
        });
        Self {
            url: Url::parse(&format!("http://{address}")).unwrap(),
            requests,
            task,
        }
    }

    fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn read_request(mut stream: TcpStream) -> std::io::Result<(TcpStream, RecordedRequest)> {
    let mut reader = BufReader::new(&mut stream);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).await?;
    let mut headers = std::collections::HashMap::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line == "\r\n" || line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    let body_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0; body_length];
    reader.read_exact(&mut body).await?;
    drop(reader);
    let mut first_parts = first_line.split_whitespace();
    let method = first_parts.next().unwrap_or_default().to_owned();
    let path = first_parts.next().unwrap_or_default().to_owned();
    Ok((
        stream,
        RecordedRequest {
            method,
            path,
            headers,
            body,
        },
    ))
}

fn actor_config(url: Url) -> ActorConfig {
    ActorConfig {
        apify_api_base_url: url,
        default_key_value_store_id: "store-test".to_owned(),
        default_dataset_id: "dataset-test".to_owned(),
        actor_run_id: "run-test".to_owned(),
        input_key: "INPUT".to_owned(),
        apify_token: "test-apify-token".to_owned(),
    }
}

#[tokio::test]
async fn apify_input_retries_transient_errors_and_keeps_actor_auth() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let request_count = attempts.clone();
    let server = MockServer::start(move |_request| {
        let attempt = request_count.fetch_add(1, Ordering::SeqCst);
        if attempt < 2 {
            (503, r#"{"error":"temporary"}"#.to_owned())
        } else {
            (200, r#"{"challenge_id":"123"}"#.to_owned())
        }
    })
    .await;
    let api = ApifyClient::with_retry_policy(
        actor_config(server.url.clone()),
        RetryPolicy {
            retries: 2,
            minimum_delay: Duration::ZERO,
        },
        Duration::from_secs(2),
    )
    .unwrap();

    assert_eq!(
        api.get_input().await.unwrap(),
        Some(json!({"challenge_id":"123"}))
    );
    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert!(requests.iter().all(|request| {
        request.method == "GET"
            && request.path == "/v2/key-value-stores/store-test/records/INPUT"
            && request.headers.get("authorization").map(String::as_str)
                == Some("Bearer test-apify-token")
    }));
}

#[tokio::test]
async fn apify_input_does_not_retry_not_found_or_non_retryable_client_errors() {
    let not_found_server = MockServer::start(|_| (404, r#"{"error":"missing"}"#.to_owned())).await;
    let api = ApifyClient::with_retry_policy(
        actor_config(not_found_server.url.clone()),
        RetryPolicy {
            retries: 4,
            minimum_delay: Duration::ZERO,
        },
        Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(api.get_input().await.unwrap(), None);
    assert_eq!(not_found_server.requests().len(), 1);

    let unauthorized_server =
        MockServer::start(|_| (401, r#"{"error":"unauthorized"}"#.to_owned())).await;
    let api = ApifyClient::with_retry_policy(
        actor_config(unauthorized_server.url.clone()),
        RetryPolicy {
            retries: 4,
            minimum_delay: Duration::ZERO,
        },
        Duration::from_secs(2),
    )
    .unwrap();
    assert!(api.get_input().await.is_err());
    assert_eq!(unauthorized_server.requests().len(), 1);
}

#[tokio::test]
async fn scrappa_uses_the_api_key_contract_and_does_not_retry_upstream_errors() {
    let server = MockServer::start(|_| (429, r#"{"msg":"rate limited"}"#.to_owned())).await;
    let client = ScrappaClient::with_base_url(
        "secret".to_owned(),
        server.url.clone(),
        Duration::from_secs(2),
    )
    .unwrap();
    let result = client
        .get_posts(&PostsParams {
            challenge_id: "123".to_owned(),
            count: 10,
            region: Some("US".to_owned()),
            cursor: Some("0".to_owned()),
        })
        .await;

    assert!(result.unwrap_err().to_string().contains("HTTP 429"));
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(
        requests[0].headers.get("x-api-key").map(String::as_str),
        Some("secret")
    );
    assert!(requests[0].headers.get("authorization").is_none());
    assert!(requests[0].path.starts_with("/tiktok/challenges/posts?"));
    assert!(requests[0].path.contains("challenge_id=123"));
    assert!(requests[0].path.contains("count=10"));
    assert!(requests[0].path.contains("region=US"));
    assert!(requests[0].path.contains("cursor=0"));
}

#[tokio::test]
async fn ppe_push_stores_then_charges_each_result_once_with_retry_idempotency() {
    let charge_attempts = Arc::new(AtomicUsize::new(0));
    let charge_counter = charge_attempts.clone();
    let server = MockServer::start(move |request| {
        if request.path == "/v2/datasets/dataset-test/items" {
            return (200, String::new());
        }
        if request.path == "/v2/actor-runs/run-test/charge" {
            if charge_counter.fetch_add(1, Ordering::SeqCst) == 0 {
                return (503, r#"{"error":"temporary"}"#.to_owned());
            }
            return (201, "{}".to_owned());
        }
        (404, "{}".to_owned())
    })
    .await;
    let api = ApifyClient::with_retry_policy(
        actor_config(server.url.clone()),
        RetryPolicy {
            retries: 1,
            minimum_delay: Duration::ZERO,
        },
        Duration::from_secs(2),
    )
    .unwrap();
    let pricing = RunPricing::from_run_response(&json!({
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "challenge-post-result": {"eventPriceUsd": 0.00025},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.00025}
                }}
            },
            "options": {"maxTotalChargeUsd": 0.0005},
            "chargedEventCounts": {}
        }
    }))
    .unwrap();
    let mut actor = ApifyActor::new(api, pricing);
    let result = actor.push_videos(&[json!({"video_id":"1"})]).await.unwrap();

    assert_eq!(
        result,
        PushResult {
            saved: 1,
            limit_reached: true
        }
    );
    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].path, "/v2/datasets/dataset-test/items");
    assert_eq!(requests[1].path, "/v2/actor-runs/run-test/charge");
    assert_eq!(requests[2].path, "/v2/actor-runs/run-test/charge");
    assert_eq!(
        requests[1].headers.get("idempotency-key"),
        requests[2].headers.get("idempotency-key")
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[1].body).unwrap(),
        json!({"eventName": "challenge-post-result", "count": 1})
    );
    assert!(requests.iter().all(|request| request
        .headers
        .get("authorization")
        .map(String::as_str)
        == Some("Bearer test-apify-token")));
}

#[tokio::test]
async fn non_ppe_pushes_batch_rows_without_custom_charges_or_key_value_output() {
    let server = MockServer::start(|request| {
        if request.path == "/v2/datasets/dataset-test/items" {
            (200, String::new())
        } else {
            (404, "{}".to_owned())
        }
    })
    .await;
    let api = ApifyClient::with_retry_policy(
        actor_config(server.url.clone()),
        RetryPolicy {
            retries: 0,
            minimum_delay: Duration::ZERO,
        },
        Duration::from_secs(2),
    )
    .unwrap();
    let pricing = RunPricing::from_run_response(&json!({
        "data": {
            "pricingInfo": {"pricingModel": "FLAT_RATE"},
            "options": {},
            "chargedEventCounts": {}
        }
    }))
    .unwrap();
    let mut actor = ApifyActor::new(api, pricing);
    let rows = [json!({"video_id":"1"}), json!({"video_id":"2"})];

    assert_eq!(
        actor.push_videos(&rows).await.unwrap(),
        PushResult {
            saved: 2,
            limit_reached: false
        }
    );
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/v2/datasets/dataset-test/items");
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[0].body).unwrap(),
        json!(rows)
    );
    assert!(!requests.iter().any(|request| {
        request.path.contains("/charge") || request.path.contains("/records/OUTPUT")
    }));
}
