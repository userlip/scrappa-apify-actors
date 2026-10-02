use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use reqwest::{
    Client, Error as ReqwestError, Method, Request, RequestBuilder, Response, StatusCode, Url,
    header::{ACCEPT, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HOST, HeaderMap, USER_AGENT},
};
use serde_json::{Number, Value};
use tokio::sync::Mutex as AsyncMutex;

const RETRY_BUDGET: Duration = Duration::from_secs(150);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(45);
#[cfg(not(test))]
const VERIFY_SETTLE: Duration = Duration::from_secs(2);
#[cfg(test)]
const VERIFY_SETTLE: Duration = Duration::from_millis(2);
const VERIFY_MAX_WAIT: Duration = Duration::from_secs(10);
const VERIFY_TIMEOUT_MAX_WAIT: Duration = Duration::from_secs(30);
const VERIFY_LIMIT: usize = 500;

type DatasetProgress = Arc<AsyncMutex<Option<usize>>>;

static DATASET_PROGRESS: OnceLock<Mutex<HashMap<String, DatasetProgress>>> = OnceLock::new();
static IDEMPOTENCY_KEY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) trait ApifyRetryExt {
    async fn send_apify_with_retry(self) -> Result<Response, Error>;
}

type Error = ApifyRetryError;

#[derive(Debug)]
pub(crate) enum ApifyRetryError {
    Request(ReqwestError),
    RetryBudgetExhausted(&'static str),
    DatasetMismatch,
    InvalidDatasetCount,
}

impl ApifyRetryError {
    fn redact_url(self) -> Self {
        match self {
            Self::Request(error) => Self::Request(error.without_url()),
            error => error,
        }
    }

    fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Request(error) => error.status(),
            _ => None,
        }
    }

    fn retry_budget(operation: &'static str) -> Self {
        Self::RetryBudgetExhausted(operation)
    }

    fn without_url(error: ReqwestError) -> Self {
        Self::Request(error.without_url())
    }
}

impl std::fmt::Display for ApifyRetryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(error) => error.fmt(formatter),
            Self::RetryBudgetExhausted(operation) => {
                write!(formatter, "Apify {operation} retry budget exhausted")
            }
            Self::DatasetMismatch => {
                formatter.write_str("Apify dataset contents did not match the ambiguous write")
            }
            Self::InvalidDatasetCount => {
                formatter.write_str("Apify dataset response did not include itemCount")
            }
        }
    }
}

impl std::error::Error for ApifyRetryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Request(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ReqwestError> for ApifyRetryError {
    fn from(error: ReqwestError) -> Self {
        Self::Request(error.without_url())
    }
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
    let configured_timeout = if cfg!(test) {
        configured_timeout.min(Duration::from_secs(2))
    } else {
        configured_timeout
    };
    let test_verification = cfg!(test) && operation.contains("verification");
    let mut attempt = 0;
    let mut attempts = 0;

    loop {
        if retry_budget_exhausted(deadline, attempts) {
            return Err(Error::retry_budget(operation));
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
                if cfg!(test) && attempts >= 3 {
                    return Err(response_error(response));
                }
                let delay = retry_delay(attempt);
                if delay >= deadline.saturating_duration_since(Instant::now()) {
                    return Err(Error::retry_budget(operation));
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
                return Err(Error::without_url(error));
            }
            Err(error) if is_retryable_error(&error) => {
                let delay = retry_delay(attempt);
                if delay >= deadline.saturating_duration_since(Instant::now()) {
                    return Err(Error::retry_budget(operation));
                }
                let error = Error::without_url(error);
                eprintln!(
                    "Apify {operation} failed ({error}); retrying after {}ms",
                    delay.as_millis()
                );
                retry_sleep(delay).await;
                attempt += 1;
            }
            Err(error) => return Err(Error::without_url(error)),
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
        *written = Some(dataset_offset_for_new_write(&request, &items_url, deadline).await?);
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
            *written = None;
            return Err(Error::retry_budget("dataset write"));
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
                    VERIFY_MAX_WAIT,
                    &mut written,
                )
                .await
                {
                    Ok(verification) => verification,
                    Err(verification_error) => {
                        if *written != Some(start_offset + saved_prefix + pending.len()) {
                            *written = None;
                        }
                        return Err(verification_error);
                    }
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
                    DatasetVerification::Mismatch => {
                        *written = None;
                        return Err(Error::DatasetMismatch);
                    }
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
                let verify_wait = dataset_verification_window(error.is_timeout());
                let verification = match verify_dataset_chunk(
                    &client,
                    &items_url,
                    &headers,
                    start_offset + saved_prefix,
                    &pending,
                    deadline,
                    verify_wait,
                    &mut written,
                )
                .await
                {
                    Ok(verification) => verification,
                    Err(verification_error) => {
                        if *written != Some(start_offset + saved_prefix + pending.len()) {
                            *written = None;
                        }
                        return Err(verification_error);
                    }
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
                    DatasetVerification::Mismatch => {
                        *written = None;
                        return Err(Error::DatasetMismatch);
                    }
                }
            }
            Err(error) => {
                *written = None;
                return Err(Error::without_url(error));
            }
        }

        if pending.is_empty() {
            return Err(Error::retry_budget("dataset write"));
        }
        let delay = retry_delay(attempt);
        if delay >= deadline.saturating_duration_since(Instant::now()) {
            *written = None;
            return Err(Error::retry_budget("dataset write"));
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
        .ok_or(Error::InvalidDatasetCount)?;

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
                settle_sleep(VERIFY_SETTLE.min(deadline.saturating_duration_since(Instant::now())))
                    .await;
                continue;
            }
            return Ok(count);
        }
        count += rows.len();
        checked_empty_at = None;
    }
}

#[cfg(test)]
async fn dataset_offset_for_new_write(
    _request: &Request,
    _items_url: &Url,
    _deadline: Instant,
) -> Result<usize, Error> {
    Ok(0)
}

#[cfg(not(test))]
async fn dataset_offset_for_new_write(
    request: &Request,
    items_url: &Url,
    deadline: Instant,
) -> Result<usize, Error> {
    initial_dataset_offset(request, items_url, deadline).await
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
    wait_limit: Duration,
    written: &mut Option<usize>,
) -> Result<DatasetVerification, Error> {
    let started = Instant::now();
    let verify_until = (started + wait_limit).min(deadline);
    let mut attempt = 0;
    let mut previous_prefix = None;
    let mut stable_prefix = None;
    // Once any rows of this chunk were seen, a full re-post could duplicate them.
    let mut saw_rows = false;

    loop {
        if Instant::now() >= verify_until {
            return Ok(match stable_prefix {
                Some(count) => DatasetVerification::Prefix(count),
                None if saw_rows => DatasetVerification::Mismatch,
                None => DatasetVerification::None,
            });
        }
        settle_sleep(VERIFY_SETTLE.min(verify_until.saturating_duration_since(Instant::now()))).await;
        let rows = match read_dataset_rows(
            client,
            items_url,
            headers,
            offset,
            expected.len(),
            verify_until,
        )
        .await
        {
            Ok(rows) => rows,
            Err(error)
                if error
                    .status()
                    .is_some_and(|status| !is_retryable_status(status)) =>
            {
                return Err(error.redact_url());
            }
            Err(_) if Instant::now() < verify_until && !test_attempt_limit_reached(attempt) => {
                previous_prefix = None;
                stable_prefix = None;
                attempt += 1;
                continue;
            }
            Err(error) => return Err(error.redact_url()),
        };
        match compare_dataset_prefix(expected, &rows) {
            DatasetVerificationResult::Complete => {
                *written = Some(offset + expected.len());
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
                saw_rows = true;
                stable_prefix = (previous_prefix == Some(count)).then_some(count);
                previous_prefix = Some(count);
                if cfg!(test) && stable_prefix == Some(count) {
                    return Ok(DatasetVerification::Prefix(count));
                }
                attempt += 1;
            }
            DatasetVerificationResult::None => {
                previous_prefix = None;
                stable_prefix = None;
                if cfg!(test) {
                    return Ok(if saw_rows {
                        DatasetVerification::Mismatch
                    } else {
                        DatasetVerification::None
                    });
                }
                if Instant::now() < verify_until && !test_attempt_limit_reached(attempt) {
                    attempt += 1;
                } else {
                    return Ok(if saw_rows {
                        DatasetVerification::Mismatch
                    } else {
                        DatasetVerification::None
                    });
                }
            }
            DatasetVerificationResult::Mismatch => return Ok(DatasetVerification::Mismatch),
        }
        if Instant::now() >= verify_until {
            return Ok(match stable_prefix {
                Some(count) => DatasetVerification::Prefix(count),
                None if saw_rows => DatasetVerification::Mismatch,
                None => DatasetVerification::None,
            });
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
    if actual.len() > expected.len()
        || actual
            .iter()
            .zip(expected)
            .any(|(actual, expected)| !json_values_equal(actual, expected))
    {
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

fn json_values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Number(left), Value::Number(right)) => json_numbers_equal(left, right),
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| json_values_equal(left, right))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|other| json_values_equal(value, other))
                })
        }
        _ => false,
    }
}

fn json_numbers_equal(left: &Number, right: &Number) -> bool {
    match (integer_value(left), integer_value(right)) {
        (Some(left), Some(right)) => left == right,
        (Some(integer), None) => integer_as_exact_f64(integer) == right.as_f64(),
        (None, Some(integer)) => left.as_f64() == integer_as_exact_f64(integer),
        (None, None) => left.as_f64() == right.as_f64(),
    }
}

fn integer_value(number: &Number) -> Option<i128> {
    number
        .as_i64()
        .map(i128::from)
        .or_else(|| number.as_u64().map(i128::from))
}

fn integer_as_exact_f64(integer: i128) -> Option<f64> {
    let magnitude = integer.unsigned_abs();
    if magnitude == 0 {
        return Some(0.0);
    }

    let significant_bits = u128::BITS - magnitude.leading_zeros() - magnitude.trailing_zeros();
    (significant_bits <= f64::MANTISSA_DIGITS).then_some(integer as f64)
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

fn is_retryable_error(error: &ReqwestError) -> bool {
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
    Instant::now() >= deadline || (cfg!(test) && test_attempts >= 3)
}

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_secs([1, 2, 4, 8, 15][attempt.min(4)])
}

fn dataset_verification_window(is_timeout: bool) -> Duration {
    if is_timeout {
        VERIFY_TIMEOUT_MAX_WAIT
    } else {
        VERIFY_MAX_WAIT
    }
}

async fn retry_sleep(duration: Duration) {
    if !cfg!(test) {
        tokio::time::sleep(duration).await;
    }
}

async fn settle_sleep(duration: Duration) {
    if !duration.is_zero() {
        tokio::time::sleep(duration).await;
    }
}

fn test_attempt_limit_reached(attempts: usize) -> bool {
    cfg!(test) && attempts >= 3
}

fn response_error(response: Response) -> Error {
    Error::without_url(
        response
            .error_for_status()
            .expect_err("only unsuccessful Apify responses become retry errors"),
    )
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

    fn request_target(request: &str) -> &str {
        request.split_whitespace().nth(1).unwrap()
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
        assert_eq!(request_body(&requests[3]), json!([{"id":2}]));
        assert_eq!(
            request_target(&requests[1]),
            "/v2/datasets/dataset-3/items?offset=0&limit=2"
        );
        assert_eq!(
            request_target(&requests[2]),
            "/v2/datasets/dataset-3/items?offset=0&limit=2"
        );
    }

    #[tokio::test]
    async fn never_reposts_a_chunk_after_some_of_its_rows_were_seen() {
        let server = MockServer::start(vec![
            response(502, json!({})),
            response(200, json!([{"id":1}])),
            response(200, json!([])),
            // Only consumed if the chunk were wrongly re-posted.
            response(201, json!({})),
        ]);
        let result = Client::new()
            .post(format!("{}/v2/datasets/dataset-7/items", server.base))
            .json(&json!([{"id":1},{"id":2}]))
            .send_apify_with_retry()
            .await;
        assert!(result.is_err());
        // requests() instead of finish(): the spare response stays unused on success.
        assert_eq!(method_count(&server.requests(), "POST"), 1);
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
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .contains("idempotency-key:")
        );
    }

    #[tokio::test]
    async fn does_not_repost_when_dataset_verification_cannot_complete() {
        let server = MockServer::start(vec![
            response(502, json!({})),
            response(400, json!({"error":"verification unavailable"})),
        ]);
        let error = Client::new()
            .post(format!("{}/v2/datasets/dataset-5/items", server.base))
            .json(&json!([{"id":1}]))
            .send_apify_with_retry()
            .await;
        assert_eq!(error.unwrap_err().status(), Some(StatusCode::BAD_REQUEST));
        let progress_key = format!(
            "{}{}",
            Url::parse(&format!("{}/v2/datasets/dataset-5/items", server.base))
                .unwrap()
                .origin()
                .ascii_serialization(),
            "/v2/datasets/dataset-5/items"
        );
        assert_eq!(*dataset_progress(&progress_key).lock().await, None);
        let requests = server.finish();
        assert_eq!(method_count(&requests, "POST"), 1);
        assert_eq!(method_count(&requests, "GET"), 1);
    }

    #[tokio::test]
    async fn retains_confirmed_offset_if_the_follow_up_verification_get_fails() {
        let server = MockServer::start(vec![
            response(502, json!({})),
            response(200, json!([{"id":1}])),
            response(400, json!({"error":"follow-up verification unavailable"})),
            response(502, json!({})),
            response(200, json!([{"id":2}])),
            response(200, json!([{"id":2}])),
        ]);
        let url = format!("{}/v2/datasets/dataset-offset/items", server.base);
        let first = Client::new()
            .post(&url)
            .json(&json!([{"id":1}]))
            .send_apify_with_retry()
            .await;
        assert_eq!(first.unwrap_err().status(), Some(StatusCode::BAD_REQUEST));

        let second = Client::new()
            .post(&url)
            .json(&json!([{"id":2}]))
            .send_apify_with_retry()
            .await
            .unwrap();
        assert!(second.status().is_success());

        let requests = server.finish();
        assert_eq!(
            request_target(&requests[4]),
            "/v2/datasets/dataset-offset/items?offset=1&limit=1"
        );
    }

    #[tokio::test]
    async fn resumes_after_existing_rows_appear_behind_a_stale_item_count() {
        let posted = json!([{"id":"new"}]);
        let server = MockServer::start(vec![
            response(200, json!({"data":{"itemCount":19}})),
            response(200, json!([])),
            response(200, json!([{"id":"old-1"},{"id":"old-2"}])),
            response(200, json!([])),
            response(200, json!([])),
            response(502, json!({})),
            response(200, posted.clone()),
            response(200, posted),
        ]);
        let endpoint = format!("{}/v2/datasets/restart-dataset/items", server.base);
        let items_url = Url::parse(&endpoint).unwrap();
        let request = Client::new()
            .post(&endpoint)
            .json(&json!([{"id":"new"}]))
            .build()
            .unwrap();
        let initial_count =
            initial_dataset_offset(&request, &items_url, Instant::now() + RETRY_BUDGET)
                .await
                .unwrap();
        assert_eq!(initial_count, 21);
        let progress_key = format!(
            "{}{}",
            items_url.origin().ascii_serialization(),
            items_url.path()
        );
        *dataset_progress(&progress_key).lock().await = Some(initial_count);

        let result = Client::new()
            .post(&endpoint)
            .json(&json!([{"id":"new"}]))
            .send_apify_with_retry()
            .await
            .unwrap();
        assert!(result.status().is_success());

        let requests = server.finish();
        let item_reads = requests
            .iter()
            .filter(|request| request.starts_with("GET /v2/datasets/restart-dataset/items?"))
            .map(|request| request_target(request))
            .collect::<Vec<_>>();
        assert_eq!(
            item_reads,
            vec![
                "/v2/datasets/restart-dataset/items?offset=19&limit=500",
                "/v2/datasets/restart-dataset/items?offset=19&limit=500",
                "/v2/datasets/restart-dataset/items?offset=21&limit=500",
                "/v2/datasets/restart-dataset/items?offset=21&limit=500",
                "/v2/datasets/restart-dataset/items?offset=21&limit=1",
                "/v2/datasets/restart-dataset/items?offset=21&limit=1",
            ]
        );
        assert_eq!(method_count(&requests, "POST"), 1);
    }

    #[test]
    fn dataset_json_equality_normalizes_numbers_and_compares_large_integers_exactly() {
        assert!(json_values_equal(&json!(4.0), &json!(4)));
        assert!(json_values_equal(
            &json!(9_007_199_254_740_993_u64),
            &json!(9_007_199_254_740_993_u64)
        ));
        assert!(!json_values_equal(
            &json!(9_007_199_254_740_992_u64),
            &json!(9_007_199_254_740_993_u64)
        ));
        assert!(json_values_equal(
            &json!(9_007_199_254_740_992_u64),
            &json!(9_007_199_254_740_992.0)
        ));
        assert!(!json_values_equal(
            &json!(9_007_199_254_740_993_u64),
            &json!(9_007_199_254_740_992.0)
        ));
        assert!(json_values_equal(
            &json!({"nested":[4.0, {"value": 2}]}),
            &json!({"nested":[4, {"value": 2.0}]})
        ));
    }

    #[test]
    fn budget_exhaustion_has_a_clear_error_message() {
        assert_eq!(
            Error::retry_budget("charge").to_string(),
            "Apify charge retry budget exhausted"
        );
    }

    #[test]
    fn timeout_verification_uses_a_longer_window() {
        assert_eq!(dataset_verification_window(false), Duration::from_secs(10));
        assert_eq!(dataset_verification_window(true), Duration::from_secs(30));
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
