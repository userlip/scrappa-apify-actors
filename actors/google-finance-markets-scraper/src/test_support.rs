#![cfg(test)]

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Clone)]
pub struct MockResponse {
    status: u16,
    body: String,
    delay: Duration,
    headers: Vec<(String, String)>,
}

impl MockResponse {
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_owned(),
            delay: Duration::ZERO,
            headers: Vec::new(),
        }
    }

    pub fn after(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

#[derive(Debug)]
pub struct MockRequest {
    pub method: String,
    pub target: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
}

pub struct MockServer {
    base_url: String,
    requests: Arc<Mutex<Vec<MockRequest>>>,
    server_thread: JoinHandle<()>,
}

impl MockServer {
    pub fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let shared_requests = requests.clone();
        let server_thread = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_request(&mut stream);
                shared_requests.lock().unwrap().push(request);
                if !response.delay.is_zero() {
                    thread::sleep(response.delay);
                }
                let reason = match response.status {
                    200 => "OK",
                    201 => "Created",
                    404 => "Not Found",
                    422 => "Unprocessable Entity",
                    429 => "Too Many Requests",
                    500 => "Internal Server Error",
                    502 => "Bad Gateway",
                    503 => "Service Unavailable",
                    504 => "Gateway Timeout",
                    _ => "Mock Response",
                };
                let extra_headers = response
                    .headers
                    .iter()
                    .map(|(name, value)| format!("{name}: {value}\r\n"))
                    .collect::<String>();
                let response_text = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{}Connection: close\r\n\r\n{}",
                    response.status,
                    reason,
                    response.body.len(),
                    extra_headers,
                    response.body
                );
                let _ = stream.write_all(response_text.as_bytes());
                let _ = stream.flush();
            }
        });
        Self {
            base_url: format!("http://{address}/api"),
            requests,
            server_thread,
        }
    }

    pub fn base_url(&self) -> String {
        self.base_url.clone()
    }

    pub fn join(self) -> Vec<MockRequest> {
        self.server_thread.join().unwrap();
        Arc::try_unwrap(self.requests)
            .unwrap_or_else(|_| panic!("test server still has request references"))
            .into_inner()
            .unwrap()
    }
}

fn read_request(stream: &mut TcpStream) -> MockRequest {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 2048];
    let header_end;
    loop {
        let count = stream.read(&mut chunk).unwrap();
        if count == 0 {
            panic!("client closed before sending request headers");
        }
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(position) = find_subslice(&bytes, b"\r\n\r\n") {
            header_end = position + 4;
            break;
        }
    }

    let head = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = head.split("\r\n");
    let mut first_line = lines.next().unwrap_or_default().split_whitespace();
    let method = first_line.next().unwrap_or_default().to_owned();
    let target = first_line.next().unwrap_or_default().to_owned();
    let mut headers = std::collections::HashMap::new();
    let mut content_length = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().to_owned();
        if name == "content-length" {
            content_length = value.parse().unwrap_or(0);
        }
        headers.insert(name, value);
    }

    while bytes.len().saturating_sub(header_end) < content_length {
        let count = stream.read(&mut chunk).unwrap();
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
    let body = String::from_utf8_lossy(&bytes[header_end..]).to_string();
    MockRequest {
        method,
        target,
        headers,
        body,
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
