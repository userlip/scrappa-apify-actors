use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use reqwest::Client;
use serde_json::{json, Value};
use url::Url;

use crate::{
    budget::DEFAULT_DATASET_ITEM_EVENT,
    config::{Config, SCRAPPA_REQUEST_TIMEOUT},
};

pub(super) struct MockResponse {
    status: u16,
    body: String,
    delay: Duration,
    disconnect: bool,
}

pub(super) struct MockServer {
    base_url: Url,
    requests: Receiver<String>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl MockServer {
    pub(super) fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (recorded_requests, requests) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut responses = responses.into_iter();
            while !stopped.load(Ordering::Relaxed) && Instant::now() < deadline {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(_) => break,
                };
                let request = read_request(&mut stream).unwrap_or_default();
                let _ = recorded_requests.send(request);
                let Some(response) = responses.next() else {
                    break;
                };
                if !response.delay.is_zero() {
                    thread::sleep(response.delay);
                }
                if response.disconnect {
                    continue;
                }
                let reason = match response.status {
                    200 => "OK",
                    201 => "Created",
                    400 => "Bad Request",
                    401 => "Unauthorized",
                    429 => "Too Many Requests",
                    503 => "Service Unavailable",
                    _ => "Mock Response",
                };
                let message = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.status,
                    reason,
                    response.body.len(),
                    response.body
                );
                if stream.write_all(message.as_bytes()).is_err() {
                    break;
                }
            }
        });
        Self {
            base_url: Url::parse(&format!("http://{address}")).unwrap(),
            requests,
            stop,
            thread: Some(thread),
        }
    }

    pub(super) fn requests(&self) -> Vec<String> {
        self.requests.try_iter().collect()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let mut content_length = 0;
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            if content_length == 0 {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
            }
            if bytes.len() >= header_end + 4 + content_length {
                break;
            }
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub(super) fn response(status: u16, body: &str) -> MockResponse {
    MockResponse {
        status,
        body: body.to_owned(),
        delay: Duration::ZERO,
        disconnect: false,
    }
}

pub(super) fn delayed_response(status: u16, body: &str, delay: Duration) -> MockResponse {
    MockResponse {
        status,
        body: body.to_owned(),
        delay,
        disconnect: false,
    }
}

pub(super) fn pricing_info() -> Value {
    json!({
        "pricingModel": "PAY_PER_EVENT",
        "pricingPerEvent": {"actorChargeEvents": {
            "search": {"eventPriceUsd": 0.001},
            "result": {"eventPriceUsd": 0.0003},
            "apify-actor-start": {"eventPriceUsd": 0.0001}
        }}
    })
}

pub(super) fn pricing_info_with_dataset_item_price() -> Value {
    let mut pricing = pricing_info();
    pricing["pricingPerEvent"]["actorChargeEvents"][DEFAULT_DATASET_ITEM_EVENT] =
        json!({"eventPriceUsd": 0.00075});
    pricing
}

pub(super) fn config(server: &MockServer) -> Config {
    let mut scrappa_api_base_url = server.base_url.clone();
    scrappa_api_base_url.set_path("/api");
    Config {
        apify_api_base_url: server.base_url.clone(),
        scrappa_api_base_url,
        key_value_store_id: "test-store".to_owned(),
        dataset_id: "test-dataset".to_owned(),
        actor_run_id: "test-run".to_owned(),
        input_key: "INPUT".to_owned(),
        apify_token: "apify-test-token".to_owned(),
        scrappa_api_key: "scrappa-test-key".to_owned(),
        scrappa_request_timeout: SCRAPPA_REQUEST_TIMEOUT,
        pricing_info: Some(pricing_info()),
        charged_event_counts: Some(json!({"apify-actor-start": 1})),
        max_total_charge_usd: Some(1.0),
    }
}

pub(super) fn client(timeout: Duration) -> Client {
    Client::builder().timeout(timeout).build().unwrap()
}

pub(super) fn request_parts(request: &str) -> (&str, &str, &str, &str) {
    let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
    let mut parts = headers
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    (
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
        headers,
        body,
    )
}

pub(super) fn header_value<'a>(headers: &'a str, header_name: &str) -> Option<&'a str> {
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case(header_name)
            .then_some(value.trim())
    })
}

pub(super) fn test_input() -> Value {
    json!({
        "query": "coffee shops",
        "zoom": 15,
        "latitude": 40.758,
        "longitude": -73.9855,
        "limit": 50,
        "hl": "de",
        "gl": "de"
    })
}
pub(super) fn lost_response() -> MockResponse {
    MockResponse {
        status: 0,
        body: String::new(),
        delay: Duration::ZERO,
        disconnect: true,
    }
}
