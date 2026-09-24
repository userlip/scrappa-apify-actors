use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use url::Url;

#[derive(Clone)]
pub struct MockResponse {
    status: u16,
    reason: String,
    content_type: String,
    body: String,
    delay: Duration,
    body_delay: Duration,
}

impl MockResponse {
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            reason: reason_for(status).to_owned(),
            content_type: "application/json; charset=utf-8".to_owned(),
            body: body.to_owned(),
            delay: Duration::ZERO,
            body_delay: Duration::ZERO,
        }
    }

    pub fn text(status: u16, body: &str) -> Self {
        Self {
            status,
            reason: reason_for(status).to_owned(),
            content_type: "text/plain; charset=utf-8".to_owned(),
            body: body.to_owned(),
            delay: Duration::ZERO,
            body_delay: Duration::ZERO,
        }
    }

    pub fn text_with_status(status: u16, reason: &str, body: &str) -> Self {
        Self {
            status,
            reason: reason.to_owned(),
            content_type: "text/plain; charset=utf-8".to_owned(),
            body: body.to_owned(),
            delay: Duration::ZERO,
            body_delay: Duration::ZERO,
        }
    }

    pub fn delayed_text(delay_ms: u64, status: u16, reason: &str, body: &str) -> Self {
        Self {
            status,
            reason: reason.to_owned(),
            content_type: "text/plain; charset=utf-8".to_owned(),
            body: body.to_owned(),
            delay: Duration::from_millis(delay_ms),
            body_delay: Duration::ZERO,
        }
    }

    pub fn delayed_body_text(delay_ms: u64, status: u16, reason: &str, body: &str) -> Self {
        Self {
            status,
            reason: reason.to_owned(),
            content_type: "text/plain; charset=utf-8".to_owned(),
            body: body.to_owned(),
            delay: Duration::ZERO,
            body_delay: Duration::from_millis(delay_ms),
        }
    }
}

pub struct MockServer {
    base_url: Url,
    requests: Arc<Mutex<Vec<String>>>,
}

impl MockServer {
    pub fn base_url(&self) -> Url {
        self.base_url.clone()
    }

    pub async fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

pub async fn start_mock_server(responses: Vec<MockResponse>) -> MockServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let base_url = Url::parse(&format!("http://{address}/api")).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured_requests = Arc::clone(&requests);

    tokio::spawn(async move {
        for response in VecDeque::from(responses) {
            let (stream, _) = listener.accept().await.unwrap();
            let requests = Arc::clone(&captured_requests);
            tokio::spawn(async move {
                serve_request(stream, response, requests).await;
            });
        }
    });

    MockServer { base_url, requests }
}

async fn serve_request(
    mut stream: TcpStream,
    response: MockResponse,
    requests: Arc<Mutex<Vec<String>>>,
) {
    let request = read_http_request(&mut stream).await;
    requests.lock().unwrap().push(request);
    tokio::time::sleep(response.delay).await;
    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        response.reason,
        response.content_type,
        response.body.len()
    );
    let _ = stream.write_all(headers.as_bytes()).await;
    tokio::time::sleep(response.body_delay).await;
    let _ = stream.write_all(response.body.as_bytes()).await;
}

async fn read_http_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut chunk = [0_u8; 2048];
    loop {
        let read = stream.read(&mut chunk).await.unwrap_or_default();
        if read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..read]);
        let Some(header_end) = find_bytes(&request, b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.trim().parse::<usize>().ok())
            .unwrap_or(0);
        if request.len() >= header_end + 4 + content_length {
            break;
        }
    }
    String::from_utf8_lossy(&request).into_owned()
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn reason_for(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Mock Response",
    }
}
