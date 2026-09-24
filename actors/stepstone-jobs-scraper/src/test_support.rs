use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub struct MockResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
    delay: Duration,
}

impl MockResponse {
    pub fn json(status: u16, body: serde_json::Value) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: body.to_string(),
            delay: Duration::ZERO,
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }

    pub fn delay_ms(mut self, delay_ms: u64) -> Self {
        self.delay = Duration::from_millis(delay_ms);
        self
    }
}

pub struct MockServer {
    pub base_url: String,
    requests: Receiver<String>,
    shutdown: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl MockServer {
    pub fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, requests) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = Arc::clone(&shutdown);
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            for response in responses {
                let (mut stream, _) = loop {
                    if worker_shutdown.load(Ordering::Relaxed) || Instant::now() >= deadline {
                        return;
                    }
                    match listener.accept() {
                        Ok(connection) => break connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => return,
                    }
                };

                let request = read_request(&mut stream).unwrap_or_default();
                if request_sender.send(request).is_err() {
                    return;
                }
                thread::sleep(response.delay);
                let _ = write_response(&mut stream, &response);
            }
        });

        Self {
            base_url: format!("http://{address}"),
            requests,
            shutdown,
            worker: Some(worker),
        }
    }

    pub fn requests(&self) -> Vec<String> {
        self.requests.try_iter().collect()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub fn request_parts(request: &str) -> (&str, &str, &str) {
    let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
    let mut parts = headers
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    (
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
        body,
    )
}

pub fn has_header(request: &str, header_name: &str, expected: &str) -> bool {
    request
        .split_once("\r\n\r\n")
        .unwrap_or((request, ""))
        .0
        .lines()
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .any(|(name, value)| {
            name.eq_ignore_ascii_case(header_name) && value.trim().eq_ignore_ascii_case(expected)
        })
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let mut content_length = None;
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            if content_length.is_none() {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                content_length = Some(
                    headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0),
                );
            }
            if bytes.len() >= header_end + 4 + content_length.unwrap_or_default() {
                break;
            }
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn write_response(stream: &mut TcpStream, response: &MockResponse) -> std::io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Mock Response",
    };
    let mut headers = response
        .headers
        .iter()
        .map(|(name, value)| format!("{name}: {value}\r\n"))
        .collect::<String>();
    headers.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n",
        response.body.len()
    ));
    let reply = format!(
        "HTTP/1.1 {} {}\r\n{}\r\n{}",
        response.status, reason, headers, response.body
    );
    stream.write_all(reply.as_bytes())
}
