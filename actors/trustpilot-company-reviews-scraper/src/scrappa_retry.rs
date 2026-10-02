use std::{
    future::Future,
    pin::Pin,
    time::{Duration, Instant, SystemTime},
};

use reqwest::{
    Request, RequestBuilder, Response, StatusCode,
    header::{self, HeaderMap},
};
use serde_json::Value;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(45);
pub(crate) const ENTRY_TIME_BUDGET: Duration = Duration::from_secs(90);
const MAX_RETRY_BACKOFF: Duration = Duration::from_secs(15);
const MAX_SCRAPPA_RETRIES: usize = 6;
const MAX_SCRAPPA_ATTEMPTS: usize = MAX_SCRAPPA_RETRIES + 1;
const REQUEST_ATTEMPTS: usize = MAX_SCRAPPA_ATTEMPTS;

type RetryFuture = Pin<Box<dyn Future<Output = reqwest::Result<Response>> + Send>>;

pub(crate) trait ScrappaRetryExt {
    fn send_scrappa_with_retry(self, operation: &'static str) -> RetryFuture;
}

impl ScrappaRetryExt for RequestBuilder {
    fn send_scrappa_with_retry(self, operation: &'static str) -> RetryFuture {
        Box::pin(async move {
            let (client, request) = self.build_split();
            let request = request?;
            let request_timeout = request.timeout().copied();
            let expects_json = expects_json(&request);
            let deadline = Instant::now() + ENTRY_TIME_BUDGET;
            retry_with(
                operation,
                deadline,
                REQUEST_ATTEMPTS,
                |remaining| {
                    let mut request = request
                        .try_clone()
                        .expect("Scrappa API requests must be replayable");
                    let timeout = remaining.min(REQUEST_TIMEOUT);
                    if let Some(request_timeout) = request_timeout {
                        *request.timeout_mut() = Some(timeout.min(request_timeout));
                    }
                    let client = client.clone();
                    async move {
                        let attempt = async move {
                            let response = client.execute(request).await?;
                            let status = response.status();
                            let headers = response.headers().clone();
                            let retry_after = parse_retry_after_header(&headers);
                            let body = response.bytes().await?;
                            let valid_json = !expects_json
                                || !status.is_success()
                                || serde_json::from_slice::<Value>(&body).is_ok();
                            let response = response_with_body(status, headers, body.to_vec());

                            Ok::<_, reqwest::Error>(Attempt {
                                value: response,
                                status,
                                retry_after,
                                valid_json,
                            })
                        };
                        match tokio::time::timeout(timeout, attempt).await {
                            Ok(response) => response,
                            Err(_) => {
                                let mut headers = HeaderMap::new();
                                headers.insert(
                                    header::CONTENT_TYPE,
                                    reqwest::header::HeaderValue::from_static("application/json"),
                                );
                                let status = StatusCode::GATEWAY_TIMEOUT;
                                let response = response_with_body(
                                    status,
                                    headers,
                                    br#"{"message":"Scrappa API request timed out"}"#.to_vec(),
                                );
                                Ok(Attempt {
                                    value: response,
                                    status,
                                    retry_after: None,
                                    valid_json: true,
                                })
                            }
                        }
                    }
                },
                retry_wait,
            )
            .await
        })
    }
}

fn expects_json(request: &Request) -> bool {
    accepts_json(
        request
            .headers()
            .get(header::ACCEPT)
            .and_then(|value| value.to_str().ok()),
    )
}

fn accepts_json(accept: Option<&str>) -> bool {
    accept.is_none_or(|accept| {
        let accept = accept.to_ascii_lowercase();
        !accept.contains("text/") || accept.contains("application/json")
    })
}

async fn retry_wait(delay: Duration) {
    #[cfg(test)]
    let _ = delay;

    #[cfg(not(test))]
    tokio::time::sleep(delay).await;
}

#[derive(Debug)]
struct Attempt<T> {
    value: T,
    status: StatusCode,
    retry_after: Option<Duration>,
    valid_json: bool,
}

trait RetryableTransportError {
    fn retry_reason(&self) -> Option<&'static str>;
}

impl RetryableTransportError for reqwest::Error {
    fn retry_reason(&self) -> Option<&'static str> {
        if self.is_timeout() {
            return Some("timeout");
        }
        if self.is_connect() || self.is_body() {
            return Some("connection");
        }
        None
    }
}

async fn retry_with<T, E, Send, SendFuture, Wait, WaitFuture>(
    operation: &str,
    deadline: Instant,
    max_attempts: usize,
    mut send: Send,
    mut wait: Wait,
) -> Result<T, E>
where
    E: RetryableTransportError,
    Send: FnMut(Duration) -> SendFuture,
    SendFuture: Future<Output = Result<Attempt<T>, E>>,
    Wait: FnMut(Duration) -> WaitFuture,
    WaitFuture: Future<Output = ()>,
{
    let mut last_response = None;
    let mut last_error = None;

    for attempt in 0..max_attempts {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return match (last_response, last_error) {
                (Some(response), _) => Ok(response),
                (_, Some(error)) => Err(error),
                (None, None) => unreachable!("the request budget starts with time remaining"),
            };
        }

        match send(remaining.min(REQUEST_TIMEOUT)).await {
            Ok(response) => {
                let retry_for_status = should_retry_scrappa(response.status);
                let retry_for_invalid_json = response.status.is_success() && !response.valid_json;
                if !(retry_for_status || retry_for_invalid_json) || attempt + 1 == max_attempts {
                    return Ok(response.value);
                }

                let delay = retry_delay(attempt)
                    .max(response.retry_after.unwrap_or_default())
                    .min(MAX_RETRY_BACKOFF);
                if delay >= deadline.saturating_duration_since(Instant::now()) {
                    return Ok(response.value);
                }

                if retry_for_invalid_json {
                    eprintln!(
                        "{operation} returned HTTP {} with invalid JSON; retrying after {}ms",
                        response.status,
                        delay.as_millis()
                    );
                } else {
                    eprintln!(
                        "{operation} returned HTTP {}; retrying after {}ms",
                        response.status,
                        delay.as_millis()
                    );
                }
                last_response = Some(response.value);
                last_error = None;
                wait(delay).await;
            }
            Err(error) => {
                let Some(reason) = error.retry_reason() else {
                    if let Some(response) = last_response.take() {
                        return Ok(response);
                    }
                    if last_error
                        .as_ref()
                        .is_some_and(|previous| previous.retry_reason() == Some("timeout"))
                    {
                        return Err(last_error.take().unwrap());
                    }
                    return Err(error);
                };
                if attempt + 1 == max_attempts {
                    if let Some(response) = last_response.take() {
                        return Ok(response);
                    }
                    if last_error.as_ref().is_some_and(|previous| {
                        previous.retry_reason() == Some("timeout") && reason != "timeout"
                    }) {
                        return Err(last_error.take().unwrap());
                    }
                    return Err(error);
                }

                let delay = retry_delay(attempt);
                if delay >= remaining {
                    if let Some(response) = last_response.take() {
                        return Ok(response);
                    }
                    if last_error.as_ref().is_some_and(|previous| {
                        previous.retry_reason() == Some("timeout") && reason != "timeout"
                    }) {
                        return Err(last_error.take().unwrap());
                    }
                    return Err(error);
                }
                eprintln!(
                    "{operation} status unavailable ({reason} error); retrying after {}ms",
                    delay.as_millis()
                );
                let preserve_timeout = last_error.as_ref().is_none_or(|previous| {
                    previous.retry_reason() != Some("timeout") || reason == "timeout"
                });
                if preserve_timeout {
                    last_error = Some(error);
                }
                wait(delay).await;
            }
        }
    }

    unreachable!("the retry loop returns after its configured attempts")
}

fn should_retry_scrappa(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_secs(1_u64.saturating_mul(2_u64.saturating_pow(attempt as u32)))
        .min(MAX_RETRY_BACKOFF)
}

fn parse_retry_after_header(headers: &HeaderMap) -> Option<Duration> {
    let value = headers.get(header::RETRY_AFTER)?.to_str().ok()?;
    parse_retry_after(value)
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }

    let retry_at = httpdate::parse_http_date(value).ok()?;
    Some(
        retry_at
            .duration_since(SystemTime::now())
            .unwrap_or_default(),
    )
}

fn response_with_body(status: StatusCode, mut headers: HeaderMap, body: Vec<u8>) -> Response {
    headers.remove(header::CONTENT_LENGTH);
    headers.remove(header::CONTENT_ENCODING);
    headers.remove(header::TRANSFER_ENCODING);
    let mut response = http::Response::builder()
        .status(status)
        .body(body)
        .expect("received responses have valid HTTP status codes");
    *response.headers_mut() = headers;
    Response::from(response)
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        collections::VecDeque,
        future::ready,
        time::{Duration, Instant},
    };

    use reqwest::StatusCode;

    use super::{
        Attempt, MAX_RETRY_BACKOFF, MAX_SCRAPPA_ATTEMPTS, RetryableTransportError, accepts_json,
        parse_retry_after, retry_delay, retry_with, should_retry_scrappa,
    };

    #[derive(Debug)]
    struct MockError;

    impl RetryableTransportError for MockError {
        fn retry_reason(&self) -> Option<&'static str> {
            Some("connection")
        }
    }

    #[derive(Debug)]
    struct MockTimeoutError;

    impl RetryableTransportError for MockTimeoutError {
        fn retry_reason(&self) -> Option<&'static str> {
            Some("timeout")
        }
    }

    fn attempt(status: StatusCode, valid_json: bool) -> Attempt<StatusCode> {
        Attempt {
            value: status,
            status,
            retry_after: None,
            valid_json,
        }
    }

    #[tokio::test]
    async fn retries_service_unavailable_then_succeeds() {
        let requests = Cell::new(0);
        let mut responses = VecDeque::from([
            attempt(StatusCode::SERVICE_UNAVAILABLE, true),
            attempt(StatusCode::OK, true),
        ]);

        let response = retry_with(
            "test request",
            Instant::now() + Duration::from_secs(90),
            MAX_SCRAPPA_ATTEMPTS,
            |_| {
                requests.set(requests.get() + 1);
                ready(Ok::<_, MockError>(responses.pop_front().unwrap()))
            },
            |_| ready(()),
        )
        .await
        .unwrap();

        assert_eq!(response, StatusCode::OK);
        assert_eq!(requests.get(), 2);
    }

    #[tokio::test]
    async fn does_not_retry_bad_request() {
        let requests = Cell::new(0);
        let response = retry_with(
            "test request",
            Instant::now() + Duration::from_secs(90),
            MAX_SCRAPPA_ATTEMPTS,
            |_| {
                requests.set(requests.get() + 1);
                ready(Ok::<_, MockError>(attempt(StatusCode::BAD_REQUEST, false)))
            },
            |_| ready(()),
        )
        .await
        .unwrap();

        assert_eq!(response, StatusCode::BAD_REQUEST);
        assert_eq!(requests.get(), 1);
    }

    #[tokio::test]
    async fn retries_successful_response_with_invalid_json() {
        let requests = Cell::new(0);
        let mut responses = VecDeque::from([
            attempt(StatusCode::OK, false),
            attempt(StatusCode::OK, true),
        ]);

        let response = retry_with(
            "test request",
            Instant::now() + Duration::from_secs(90),
            MAX_SCRAPPA_ATTEMPTS,
            |_| {
                requests.set(requests.get() + 1);
                ready(Ok::<_, MockError>(responses.pop_front().unwrap()))
            },
            |_| ready(()),
        )
        .await
        .unwrap();

        assert_eq!(response, StatusCode::OK);
        assert_eq!(requests.get(), 2);
    }

    #[tokio::test]
    async fn stops_after_seven_attempts_with_the_capped_schedule() {
        let requests = Cell::new(0);
        let delays = RefCell::new(Vec::new());

        let response = retry_with(
            "test request",
            Instant::now() + Duration::from_secs(90),
            MAX_SCRAPPA_ATTEMPTS,
            |_| {
                requests.set(requests.get() + 1);
                ready(Ok::<_, MockError>(attempt(
                    StatusCode::SERVICE_UNAVAILABLE,
                    true,
                )))
            },
            |delay| {
                delays.borrow_mut().push(delay);
                ready(())
            },
        )
        .await
        .unwrap();

        assert_eq!(response, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(requests.get(), MAX_SCRAPPA_ATTEMPTS);
        assert_eq!(
            *delays.borrow(),
            vec![
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(4),
                Duration::from_secs(8),
                Duration::from_secs(15),
                Duration::from_secs(15),
            ]
        );
    }

    #[tokio::test]
    async fn retries_connection_errors_until_the_attempt_limit() {
        let requests = Cell::new(0);

        let error = retry_with(
            "test request",
            Instant::now() + Duration::from_secs(90),
            MAX_SCRAPPA_ATTEMPTS,
            |_| {
                requests.set(requests.get() + 1);
                ready(Err::<Attempt<StatusCode>, _>(MockError))
            },
            |_| ready(()),
        )
        .await
        .unwrap_err();

        assert!(matches!(error, MockError));
        assert_eq!(requests.get(), MAX_SCRAPPA_ATTEMPTS);
    }

    #[tokio::test]
    async fn retries_timeout_errors_until_the_attempt_limit() {
        let requests = Cell::new(0);

        let error = retry_with(
            "test request",
            Instant::now() + Duration::from_secs(90),
            MAX_SCRAPPA_ATTEMPTS,
            |_| {
                requests.set(requests.get() + 1);
                ready(Err::<Attempt<StatusCode>, _>(MockTimeoutError))
            },
            |_| ready(()),
        )
        .await
        .unwrap_err();

        assert!(matches!(error, MockTimeoutError));
        assert_eq!(requests.get(), MAX_SCRAPPA_ATTEMPTS);
    }

    #[test]
    fn retries_only_the_required_statuses_with_capped_backoff() {
        for status in [
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::BAD_GATEWAY,
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::GATEWAY_TIMEOUT,
        ] {
            assert!(should_retry_scrappa(status));
        }
        assert!(!should_retry_scrappa(StatusCode::BAD_REQUEST));
        assert!(!should_retry_scrappa(StatusCode::REQUEST_TIMEOUT));
        assert!(!should_retry_scrappa(StatusCode::NOT_FOUND));
        assert!(!should_retry_scrappa(StatusCode::NOT_IMPLEMENTED));
        assert_eq!(retry_delay(0), Duration::from_secs(1));
        assert_eq!(retry_delay(1), Duration::from_secs(2));
        assert_eq!(retry_delay(2), Duration::from_secs(4));
        assert_eq!(retry_delay(3), Duration::from_secs(8));
        assert_eq!(retry_delay(4), Duration::from_secs(15));
        assert_eq!(retry_delay(5), Duration::from_secs(15));
        assert_eq!(retry_delay(100), MAX_RETRY_BACKOFF);
        assert_eq!(parse_retry_after("3"), Some(Duration::from_secs(3)));
        assert_eq!(
            retry_delay(0)
                .max(Duration::from_secs(30))
                .min(MAX_RETRY_BACKOFF),
            MAX_RETRY_BACKOFF
        );
    }

    #[tokio::test]
    async fn honors_retry_after_and_caps_it_at_fifteen_seconds() {
        let delays = RefCell::new(Vec::new());
        let mut responses = VecDeque::from([
            Attempt {
                value: StatusCode::SERVICE_UNAVAILABLE,
                status: StatusCode::SERVICE_UNAVAILABLE,
                retry_after: Some(Duration::from_secs(30)),
                valid_json: true,
            },
            attempt(StatusCode::OK, true),
        ]);

        retry_with(
            "test request",
            Instant::now() + Duration::from_secs(90),
            MAX_SCRAPPA_ATTEMPTS,
            |_| ready(Ok::<_, MockError>(responses.pop_front().unwrap())),
            |delay| {
                delays.borrow_mut().push(delay);
                ready(())
            },
        )
        .await
        .unwrap();

        assert_eq!(*delays.borrow(), vec![MAX_RETRY_BACKOFF]);
        assert!(accepts_json(None));
        assert!(accepts_json(Some("application/json, */*")));
        assert!(!accepts_json(Some(
            "text/markdown, text/plain;q=0.9, */*;q=0.8"
        )));
    }
}
