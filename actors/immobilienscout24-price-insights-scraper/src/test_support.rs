use serde_json::Value;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use url::Url;

#[derive(Clone, Debug)]
pub struct CapturedRequest {
    pub method: String,
    pub target: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl CapturedRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn json_body(&self) -> Value {
        serde_json::from_slice(&self.body).expect("mock request contains valid JSON")
    }
}

#[derive(Clone, Debug)]
pub struct MockResponse {
    pub status: u16,
    pub body: String,
    pub delay: Duration,
}

impl MockResponse {
    pub fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            body: body.to_string(),
            delay: Duration::ZERO,
        }
    }

    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
            delay: Duration::ZERO,
        }
    }

    pub fn delayed(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

pub struct MockServer {
    pub base_url: Url,
    address: String,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    pub fn start(
        handler: impl Fn(&CapturedRequest) -> MockResponse + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock server port is available");
        listener
            .set_nonblocking(true)
            .expect("mock server can accept requests");
        let address = listener.local_addr().unwrap();
        let address_string = address.to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded_requests = Arc::clone(&requests);
        let stopped = Arc::new(AtomicBool::new(false));
        let stopped_on_thread = Arc::clone(&stopped);
        let handler = Arc::new(handler);
        let thread = thread::spawn(move || {
            while !stopped_on_thread.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let handler = Arc::clone(&handler);
                        let requests = Arc::clone(&recorded_requests);
                        thread::spawn(move || serve(stream, handler, requests));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(3));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            base_url: Url::parse(&format!("http://{address_string}")).unwrap(),
            address: address_string,
            requests,
            stopped,
            thread: Some(thread),
        }
    }

    pub fn requests(&self) -> Vec<CapturedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(&self.address);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(
    mut stream: TcpStream,
    handler: Arc<dyn Fn(&CapturedRequest) -> MockResponse + Send + Sync>,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
) {
    let Some(request) = read_request(&mut stream) else {
        return;
    };
    requests.lock().unwrap().push(request.clone());
    let response = handler(&request);
    if !response.delay.is_zero() {
        thread::sleep(response.delay);
    }
    let reason = match response.status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        408 => "Request Timeout",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Mock Response",
    };
    let message = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response.status,
        reason,
        response.body.len(),
        response.body
    );
    let _ = stream.write_all(message.as_bytes());
}

fn read_request(stream: &mut TcpStream) -> Option<CapturedRequest> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let mut header_end = None;
    let mut content_length = None;

    loop {
        let read = stream.read(&mut buffer).ok()?;
        if read == 0 {
            return None;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if header_end.is_none() {
            header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
            if let Some(end) = header_end {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                content_length = Some(
                    headers
                        .lines()
                        .skip(1)
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0),
                );
            }
        }
        if let (Some(end), Some(length)) = (header_end, content_length) {
            if bytes.len() >= end + 4 + length {
                break;
            }
        }
    }

    let end = header_end?;
    let header_text = String::from_utf8_lossy(&bytes[..end]);
    let mut lines = header_text.lines();
    let mut request_line = lines.next()?.split_whitespace();
    let method = request_line.next()?.to_owned();
    let target = request_line.next()?.to_owned();
    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_owned(), value.trim().to_owned()))
        })
        .collect();
    let body = bytes[end + 4..].to_vec();

    Some(CapturedRequest {
        method,
        target,
        headers,
        body,
    })
}
