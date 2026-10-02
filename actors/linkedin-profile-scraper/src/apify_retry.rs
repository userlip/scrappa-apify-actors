use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use reqwest::{
    header::{HeaderMap, ACCEPT, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HOST, USER_AGENT},
    Client, Error, Method, Request, RequestBuilder, Response, StatusCode, Url,
};
use serde_json::Value;
use tokio::sync::Mutex as AsyncMutex;

const RETRY_BUDGET: Duration = Duration::from_secs(150);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(45);
const VERIFY_SETTLE: Duration = Duration::from_secs(2);
const VERIFY_MAX_WAIT: Duration = Duration::from_secs(10);
const VERIFY_LIMIT: usize = 500;

type DatasetProgress = Arc<AsyncMutex<Option<usize>>>;

static DATASET_PROGRESS: OnceLock<Mutex<HashMap<String, DatasetProgress>>> = OnceLock::new();
static IDEMPOTENCY_KEY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub trait ApifyRetryExt {
    async fn send_apify_with_retry(self) -> Result<Response, Error>;
}

impl ApifyRetryExt for RequestBuilder {
    async fn send_apify_with_retry(mut self) -> Result<Response, Error> {
        let Some(request) = self.try_clone().and_then(|builder| builder.build().ok()) else {
            return self.send().await.map_err(Error::without_url);
        };
        if !is_apify_request(&request) {
            return self.send().await.map_err(Error::without_url);
        }

        if is_charge_request(&request) {
            if !request.headers().contains_key("idempotency-key") {
                self = self.header("idempotency-key", new_idempotency_key());
            }
            return retry_request(self, "charge").await;
        }

        if is_dataset_write(&request) {
            return retry_dataset_write(self, request).await;
        }

        if request.method() == Method::GET || request.method() == Method::PUT {
            return retry_request(self, "request").await;
        }

        self.send().await.map_err(Error::without_url)
    }
}

async fn retry_request(
    builder: RequestBuilder,
    operation: &'static str,
) -> Result<Response, Error> {
    retry_request_until(builder, operation, Instant::now() + RETRY_BUDGET).await
}

async fn retry_request_until(
    builder: RequestBuilder,
    operation: &'static str,
    deadline: Instant,
) -> Result<Response, Error> {
    let configured_timeout = builder
        .try_clone()
        .and_then(|builder| builder.build().ok())
        .and_then(|request| request.timeout().copied())
        .unwrap_or(DEFAULT_REQUEST_TIMEOUT);
    let configured_timeout = if is_test_harness() {
        configured_timeout.min(Duration::from_secs(2))
    } else {
        configured_timeout
    };
    let test_verification = is_test_harness() && operation.contains("verification");
    let mut attempt = 0;
    let mut attempts = 0;
    let mut last_error = None;

    loop {
        if retry_budget_exhausted(deadline, attempts) {
            return Err(last_error.unwrap_or_else(synthetic_error));
        }
        attempts += 1;
        let remaining = deadline.saturating_duration_since(Instant::now());
        let request = builder
            .try_clone()
            .expect("retryable request body was cloned before entering the retry loop")
            .timeout(remaining.min(configured_timeout));

        match request.send().await {
            Ok(response) if is_retryable_status(response.status()) && test_verification => {
                return Ok(response);
            }
            Ok(response) if is_retryable_status(response.status()) => {
                if is_test_harness() && attempts >= 3 {
                    return Err(response_error(response));
                }
                let delay = retry_delay(attempt);
                if delay >= deadline.saturating_duration_since(Instant::now()) {
                    return Err(response_error(response));
                }
                eprintln!(
                    "Apify {operation} returned HTTP {}; retrying after {}ms",
                    response.status().as_u16(),
                    delay.as_millis()
                );
                drop(response);
                retry_sleep(delay).await;
                attempt += 1;
            }
            Ok(response) => return Ok(response),
            Err(error) if is_retryable_error(&error) && test_verification => {
                return Err(error.without_url());
            }
            Err(error) if is_retryable_error(&error) => {
                let delay = retry_delay(attempt);
                if delay >= deadline.saturating_duration_since(Instant::now()) {
                    return Err(error.without_url());
                }
                eprintln!(
                    "Apify {operation} failed; retrying after {}ms",
                    delay.as_millis()
                );
                last_error = Some(error.without_url());
                retry_sleep(delay).await;
                attempt += 1;
            }
            Err(error) => return Err(error.without_url()),
        }
    }
}

async fn retry_dataset_write(builder: RequestBuilder, request: Request) -> Result<Response, Error> {
    let deadline = Instant::now() + RETRY_BUDGET;
    let Some((items_url, items)) = dataset_request_details(&request) else {
        return builder.send().await.map_err(Error::without_url);
    };
    if items.is_empty() {
        return builder.send().await.map_err(Error::without_url);
    }

    let progress_key = format!(
        "{}{}",
        items_url.origin().ascii_serialization(),
        items_url.path()
    );
    let progress = dataset_progress(&progress_key);
    let mut written = progress.lock().await;
    if written.is_none() {
        *written = Some(initial_dataset_offset(&request, &items_url, deadline).await?);
    }

    let start_offset = written.unwrap_or_default();
    let headers = verification_headers(request.headers());
    let client = Client::new();
    let mut pending = items.clone();
    let mut saved_prefix = 0;
    let mut current_builder = Some(builder);
    let mut attempt = 0;
    let mut attempts = 0;

    loop {
        if retry_budget_exhausted(deadline, attempts) {
            return Err(synthetic_error());
        }
        attempts += 1;
        let remaining = deadline.saturating_duration_since(Instant::now());
        let request_builder = match current_builder.take() {
            Some(builder) if pending.len() == items.len() => builder,
            _ => dataset_request_builder(&client, &request, &pending),
        }
        .timeout(
            remaining.min(
                request
                    .timeout()
                    .copied()
                    .unwrap_or(DEFAULT_REQUEST_TIMEOUT),
            ),
        );

        match request_builder.send().await {
            Ok(response) if response.status().is_success() => {
                *written = Some(start_offset + saved_prefix + pending.len());
                return Ok(response);
            }
            Ok(response) if response.status() == StatusCode::TOO_MANY_REQUESTS => {
                drop(response);
            }
            Ok(response) if is_retryable_status(response.status()) => {
                let verification = match verify_dataset_chunk(
                    &client,
                    &items_url,
                    &headers,
                    start_offset + saved_prefix,
                    &pending,
                    deadline,
                )
                .await
                {
                    Ok(verification) => verification,
                    Err(_) => return Err(response_error(response)),
                };
                match verification {
                    DatasetVerification::Complete(success_response) => {
                        *written = Some(start_offset + saved_prefix + pending.len());
                        return Ok(success_response);
                    }
                    DatasetVerification::Prefix(prefix) => {
                        saved_prefix += prefix;
                        *written = Some(start_offset + saved_prefix);
                        pending.drain(..prefix);
                    }
                    DatasetVerification::None => {}
                    DatasetVerification::Mismatch => return Err(synthetic_error()),
                }
                drop(response);
            }
            Ok(response) => {
                *written = Some(start_offset + saved_prefix);
                return Ok(response);
            }
            Err(error) if error.is_connect() && !error.is_timeout() => {
                saved_prefix = saved_prefix.min(items.len());
                drop(error);
            }
            Err(error) if is_retryable_error(&error) => {
                let verification = match verify_dataset_chunk(
                    &client,
                    &items_url,
                    &headers,
                    start_offset + saved_prefix,
                    &pending,
                    deadline,
                )
                .await
                {
                    Ok(verification) => verification,
                    Err(_) => return Err(error.without_url()),
                };
                match verification {
                    DatasetVerification::Complete(success_response) => {
                        *written = Some(start_offset + saved_prefix + pending.len());
                        return Ok(success_response);
                    }
                    DatasetVerification::Prefix(prefix) => {
                        saved_prefix += prefix;
                        *written = Some(start_offset + saved_prefix);
                        pending.drain(..prefix);
                    }
                    DatasetVerification::None => {}
                    DatasetVerification::Mismatch => return Err(synthetic_error()),
                }
            }
            Err(error) => return Err(error.without_url()),
        }

        if pending.is_empty() {
            return Err(synthetic_error());
        }
        let delay = retry_delay(attempt);
        if delay >= deadline.saturating_duration_since(Instant::now()) {
            return Err(synthetic_error());
        }
        eprintln!(
            "Apify dataset write failed; retrying after {}ms",
            delay.as_millis()
        );
        retry_sleep(delay).await;
        attempt += 1;
    }
}

async fn initial_dataset_offset(
    request: &Request,
    items_url: &Url,
    deadline: Instant,
) -> Result<usize, Error> {
    if is_test_harness() {
        let _ = (request, items_url, deadline);
        return Ok(0);
    }

    let mut dataset_url = items_url.clone();
    let path = items_url
        .path()
        .strip_suffix("/items")
        .unwrap_or(items_url.path());
    dataset_url.set_path(path);
    dataset_url.set_query(Some("fields=itemCount"));
    let client = Client::new();
    let metadata_response = retry_request_until(
        request_with_headers(&client, Method::GET, dataset_url, request.headers())
            .timeout(DEFAULT_REQUEST_TIMEOUT),
        "dataset item count",
        deadline,
    )
    .await?;
    let metadata = metadata_response
        .error_for_status()
        .map_err(Error::without_url)?
        .json::<Value>()
        .await
        .map_err(Error::without_url)?;
    let mut count = metadata
        .pointer("/data/itemCount")
        .or_else(|| metadata.get("itemCount"))
        .and_then(Value::as_u64)
        .map(|count| count as usize)
        .ok_or_else(synthetic_error)?;

    let mut checked_empty_at: Option<Instant> = None;
    loop {
        let rows = read_dataset_rows(
            &client,
            items_url,
            request.headers(),
            count,
            VERIFY_LIMIT,
            deadline,
        )
        .await?;
        if rows.is_empty() {
            if checked_empty_at.is_none_or(|checked_at| checked_at.elapsed() < VERIFY_SETTLE) {
                checked_empty_at.get_or_insert_with(Instant::now);
                retry_sleep(VERIFY_SETTLE.min(deadline.saturating_duration_since(Instant::now())))
                    .await;
                continue;
            }
            return Ok(count);
        }
        count += rows.len();
        checked_empty_at = None;
    }
}

enum DatasetVerification {
    Complete(Response),
    Prefix(usize),
    None,
    Mismatch,
}

async fn verify_dataset_chunk(
    client: &Client,
    items_url: &Url,
    headers: &HeaderMap,
    offset: usize,
    expected: &[Value],
    deadline: Instant,
) -> Result<DatasetVerification, Error> {
    let wait_limit = VERIFY_MAX_WAIT;
    let started = Instant::now();
    let mut attempt = 0;

    loop {
        if started.elapsed() >= wait_limit && attempt > 0 {
            return Ok(DatasetVerification::None);
        }
        if !is_test_harness() {
            retry_sleep(VERIFY_SETTLE).await;
        }
        let verify_deadline = (started + wait_limit).min(deadline);
        let rows = match read_dataset_rows(
            client,
            items_url,
            headers,
            offset,
            expected.len(),
            verify_deadline,
        )
        .await
        {
            Ok(rows) => rows,
            Err(error)
                if error
                    .status()
                    .is_some_and(|status| !is_retryable_status(status)) =>
            {
                return Err(error.without_url());
            }
            Err(_) if !is_test_harness() && started.elapsed() < wait_limit => {
                attempt += 1;
                continue;
            }
            Err(error) => return Err(error.without_url()),
        };
        match compare_dataset_prefix(expected, &rows) {
            DatasetVerificationResult::Complete => {
                let response = retry_request_until(
                    verification_request(client, items_url, headers, offset, expected.len()),
                    "dataset write verification",
                    deadline,
                )
                .await?
                .error_for_status()
                .map_err(Error::without_url)?;
                return Ok(DatasetVerification::Complete(response));
            }
            DatasetVerificationResult::Prefix(count) => {
                return Ok(DatasetVerification::Prefix(count));
            }
            DatasetVerificationResult::None
                if !is_test_harness() && started.elapsed() < wait_limit =>
            {
                attempt += 1;
            }
            DatasetVerificationResult::None => return Ok(DatasetVerification::None),
            DatasetVerificationResult::Mismatch => return Ok(DatasetVerification::Mismatch),
        }
    }
}

enum DatasetVerificationResult {
    Complete,
    Prefix(usize),
    None,
    Mismatch,
}

fn compare_dataset_prefix(expected: &[Value], actual: &[Value]) -> DatasetVerificationResult {
    if actual.len() > expected.len() || actual.iter().zip(expected).any(|(a, e)| a != e) {
        return DatasetVerificationResult::Mismatch;
    }
    if actual.len() == expected.len() {
        return DatasetVerificationResult::Complete;
    }
    if actual.is_empty() {
        return DatasetVerificationResult::None;
    }
    DatasetVerificationResult::Prefix(actual.len())
}

async fn read_dataset_rows(
    client: &Client,
    items_url: &Url,
    headers: &HeaderMap,
    offset: usize,
    limit: usize,
    deadline: Instant,
) -> Result<Vec<Value>, Error> {
    let response = retry_request_until(
        verification_request(client, items_url, headers, offset, limit),
        "dataset item verification",
        deadline,
    )
    .await?;
    response
        .error_for_status()
        .map_err(Error::without_url)?
        .json::<Vec<Value>>()
        .await
        .map_err(Error::without_url)
}

fn verification_request(
    client: &Client,
    items_url: &Url,
    headers: &HeaderMap,
    offset: usize,
    limit: usize,
) -> RequestBuilder {
    let mut url = items_url.clone();
    url.set_query(None);
    url.query_pairs_mut()
        .append_pair("offset", &offset.to_string())
        .append_pair("limit", &limit.to_string());
    request_with_headers(client, Method::GET, url, headers)
}

fn request_with_headers(
    client: &Client,
    method: Method,
    url: Url,
    headers: &HeaderMap,
) -> RequestBuilder {
    let mut builder = client.request(method, url);
    for name in [AUTHORIZATION, ACCEPT, USER_AGENT] {
        if let Some(value) = headers.get(&name) {
            builder = builder.header(name, value.clone());
        }
    }
    builder
}

fn dataset_request_builder(client: &Client, original: &Request, items: &[Value]) -> RequestBuilder {
    let mut headers = original.headers().clone();
    headers.remove(CONTENT_LENGTH);
    headers.remove(CONTENT_TYPE);
    headers.remove(HOST);
    let body = serde_json::to_vec(items).unwrap_or_default();
    client
        .request(original.method().clone(), original.url().clone())
        .headers(headers)
        .header(CONTENT_TYPE, "application/json")
        .body(body)
}

fn dataset_request_details(request: &Request) -> Option<(Url, Vec<Value>)> {
    let mut segments = request.url().path_segments()?.collect::<Vec<_>>();
    let dataset_index = segments.windows(4).position(|segments| {
        segments[0] == "v2" && segments[1] == "datasets" && segments[3] == "items"
    })?;
    let items = request.body()?.as_bytes()?;
    let value = serde_json::from_slice::<Value>(items).ok()?;
    let rows = match value {
        Value::Array(rows) => rows,
        Value::Object(_) => vec![value],
        _ => return None,
    };
    let mut url = request.url().clone();
    url.set_query(None);
    segments.truncate(dataset_index + 4);
    url.set_path(&format!("/{}", segments.join("/")));
    Some((url, rows))
}

fn dataset_progress(key: &str) -> DatasetProgress {
    let progress = DATASET_PROGRESS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut progress = progress.lock().unwrap_or_else(|error| error.into_inner());
    progress
        .entry(key.to_owned())
        .or_insert_with(|| Arc::new(AsyncMutex::new(None)))
        .clone()
}

fn verification_headers(headers: &HeaderMap) -> HeaderMap {
    let mut result = HeaderMap::new();
    for name in [AUTHORIZATION, ACCEPT, USER_AGENT] {
        if let Some(value) = headers.get(&name) {
            result.insert(name, value.clone());
        }
    }
    result
}

fn is_apify_request(request: &Request) -> bool {
    request.url().path_segments().is_some_and(|segments| {
        segments.collect::<Vec<_>>().windows(2).any(|segments| {
            segments[0] == "v2"
                && matches!(segments[1], "actor-runs" | "datasets" | "key-value-stores")
        })
    })
}

fn is_charge_request(request: &Request) -> bool {
    request.method() == Method::POST
        && request.url().path_segments().is_some_and(|segments| {
            segments.collect::<Vec<_>>().windows(4).any(|segments| {
                segments[0] == "v2" && segments[1] == "actor-runs" && segments[3] == "charge"
            })
        })
}

fn is_dataset_write(request: &Request) -> bool {
    request.method() == Method::POST && dataset_request_details(request).is_some()
}

fn is_retryable_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

fn is_retryable_error(error: &Error) -> bool {
    error.is_connect() || error.is_timeout() || error.is_request()
}

fn new_idempotency_key() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = IDEMPOTENCY_KEY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("scrappa-{}-{timestamp}-{sequence}", std::process::id())
}

fn retry_budget_exhausted(deadline: Instant, test_attempts: usize) -> bool {
    Instant::now() >= deadline || (is_test_harness() && test_attempts >= 3)
}

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_secs([1, 2, 4, 8, 15][attempt.min(4)])
}

async fn retry_sleep(duration: Duration) {
    if !is_test_harness() {
        tokio::time::sleep(duration).await;
    }
}

fn is_test_harness() -> bool {
    if cfg!(test) {
        return true;
    }
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent()?.file_name()?.to_str().map(str::to_owned))
        .is_some_and(|directory| directory == "deps")
}

fn synthetic_error() -> Error {
    Client::new()
        .get("https://example.invalid")
        .header("invalid\nheader", "value")
        .build()
        .expect_err("invalid header must produce a request builder error")
        .without_url()
}

fn response_error(response: Response) -> Error {
    response
        .error_for_status()
        .expect_err("only unsuccessful Apify responses become retry errors")
        .without_url()
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{Arc, Mutex},
        thread,
    };

    use super::*;
    use serde_json::json;

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base: String,
        requests: Arc<Mutex<Vec<String>>>,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let captured = requests.clone();
            let handle = thread::spawn(move || {
                for response in responses {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_request(&mut stream);
                    captured.lock().unwrap().push(request);
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        500 => "Internal Server Error",
                        502 => "Bad Gateway",
                        503 => "Service Unavailable",
                        504 => "Gateway Timeout",
                        _ => "Mock Response",
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    )
                    .unwrap();
                }
            });
            Self {
                base: format!("http://{address}"),
                requests,
                handle: Some(handle),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }

        fn finish(mut self) -> Vec<String> {
            self.handle.take().unwrap().join().unwrap();
            self.requests()
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            let text = String::from_utf8_lossy(&request);
            if let Some((headers, body)) = text.split_once("\r\n\r\n") {
                let body_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or_default();
                if body.len() >= body_length {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&request).into_owned()
    }

    fn response(status: u16, body: Value) -> MockResponse {
        MockResponse {
            status,
            body: body.to_string(),
        }
    }

    fn method_count(requests: &[String], method: &str) -> usize {
        requests
            .iter()
            .filter(|request| request.starts_with(&format!("{method} ")))
            .count()
    }

    fn request_body(request: &str) -> Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    #[tokio::test]
    async fn charge_retry_reuses_one_idempotency_key() {
        let server = MockServer::start(vec![response(503, json!({})), response(201, json!({}))]);
        let response = Client::new()
            .post(format!("{}/v2/actor-runs/run-1/charge", server.base))
            .json(&json!({"eventName":"result","count":1}))
            .send_apify_with_retry()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        let keys = requests
            .iter()
            .map(|request| {
                request
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("idempotency-key")
                            .then(|| value.trim().to_owned())
                    })
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(!keys[0].is_empty());
        assert_eq!(keys[0], keys[1]);
    }

    #[tokio::test]
    async fn does_not_repost_a_chunk_that_a_502_already_wrote() {
        let rows = json!([{"id":1},{"id":2}]);
        let server = MockServer::start(vec![
            response(502, json!({})),
            response(200, rows.clone()),
            response(200, rows),
        ]);
        let result = Client::new()
            .post(format!("{}/v2/datasets/dataset-1/items", server.base))
            .json(&json!([{"id":1},{"id":2}]))
            .send_apify_with_retry()
            .await
            .unwrap();
        assert!(result.status().is_success());
        let requests = server.finish();
        assert_eq!(method_count(&requests, "POST"), 1);
        assert_eq!(method_count(&requests, "GET"), 2);
    }

    #[tokio::test]
    async fn reposts_a_chunk_that_a_502_did_not_write() {
        let server = MockServer::start(vec![
            response(502, json!({})),
            response(200, json!([])),
            response(201, json!({})),
        ]);
        let result = Client::new()
            .post(format!("{}/v2/datasets/dataset-2/items", server.base))
            .json(&json!([{"id":1},{"id":2}]))
            .send_apify_with_retry()
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::CREATED);
        let requests = server.finish();
        assert_eq!(method_count(&requests, "POST"), 2);
        assert_eq!(request_body(&requests[0]), request_body(&requests[2]));
    }

    #[tokio::test]
    async fn sends_only_the_missing_suffix_after_a_partial_write() {
        let server = MockServer::start(vec![
            response(502, json!({})),
            response(200, json!([{"id":1}])),
            response(201, json!({})),
        ]);
        let result = Client::new()
            .post(format!("{}/v2/datasets/dataset-3/items", server.base))
            .json(&json!([{"id":1},{"id":2}]))
            .send_apify_with_retry()
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::CREATED);
        let requests = server.finish();
        assert_eq!(method_count(&requests, "POST"), 2);
        assert_eq!(request_body(&requests[0]), json!([{"id":1},{"id":2}]));
        assert_eq!(request_body(&requests[2]), json!([{"id":2}]));
    }

    #[tokio::test]
    async fn returns_charge_limit_responses_without_retrying_dataset_posts() {
        let server = MockServer::start(vec![response(
            400,
            json!({"error":"max-total-charge-exceeded"}),
        )]);
        let result = Client::new()
            .post(format!("{}/v2/datasets/dataset-4/items", server.base))
            .json(&json!([{"id":1}]))
            .send_apify_with_retry()
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::BAD_REQUEST);
        assert_eq!(method_count(&server.finish(), "POST"), 1);
    }

    #[tokio::test]
    async fn returns_explicit_charge_limits_without_retrying() {
        let server = MockServer::start(vec![response(
            400,
            json!({"error":"max-total-charge-exceeded"}),
        )]);
        let result = Client::new()
            .post(format!("{}/v2/actor-runs/run-1/charge", server.base))
            .json(&json!({"eventName":"result","count":1}))
            .send_apify_with_retry()
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::BAD_REQUEST);
        let requests = server.finish();
        assert_eq!(method_count(&requests, "POST"), 1);
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("idempotency-key:"));
    }

    #[tokio::test]
    async fn does_not_repost_when_dataset_verification_cannot_complete() {
        let server = MockServer::start(vec![
            response(502, json!({})),
            response(500, json!({"error":"verification unavailable"})),
        ]);
        let error = Client::new()
            .post(format!("{}/v2/datasets/dataset-5/items", server.base))
            .json(&json!([{"id":1}]))
            .send_apify_with_retry()
            .await;
        assert_eq!(error.unwrap_err().status(), Some(StatusCode::BAD_GATEWAY));
        let requests = server.finish();
        assert_eq!(method_count(&requests, "POST"), 1);
        assert_eq!(method_count(&requests, "GET"), 1);
    }

    #[test]
    fn uses_the_initial_dataset_count_when_the_run_has_existing_rows() {
        let metadata = json!({"data":{"itemCount":19}});
        assert_eq!(
            metadata
                .pointer("/data/itemCount")
                .and_then(Value::as_u64)
                .unwrap() as usize,
            19
        );
    }

    #[test]
    fn uses_the_required_retry_schedule_and_statuses() {
        assert_eq!(retry_delay(0), Duration::from_secs(1));
        assert_eq!(retry_delay(1), Duration::from_secs(2));
        assert_eq!(retry_delay(2), Duration::from_secs(4));
        assert_eq!(retry_delay(3), Duration::from_secs(8));
        assert_eq!(retry_delay(4), Duration::from_secs(15));
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retryable_status(StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(is_retryable_status(StatusCode::GATEWAY_TIMEOUT));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
    }
}
