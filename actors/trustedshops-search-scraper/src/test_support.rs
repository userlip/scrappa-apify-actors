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
    pub status: u16,
    pub body: String,
}

pub struct MockServer {
    pub base_url: String,
    requests: Receiver<String>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    pub fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, requests) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            for response in responses {
                let (mut stream, _) = loop {
                    if stopped.load(Ordering::Relaxed) || Instant::now() >= deadline {
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
                let request = match read_request(&mut stream) {
                    Ok(request) => request,
                    Err(_) => return,
                };
                if request_sender.send(request).is_err() {
                    return;
                }
                let reason = match response.status {
                    200 => "OK",
                    201 => "Created",
                    400 => "Bad Request",
                    404 => "Not Found",
                    500 => "Internal Server Error",
                    _ => "Mock Response",
                };
                let reply = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.status,
                    reason,
                    response.body.len(),
                    response.body
                );
                if stream.write_all(reply.as_bytes()).is_err() {
                    return;
                }
            }
        });
        Self {
            base_url: format!("http://{address}"),
            requests,
            stop,
            thread: Some(thread),
        }
    }

    pub fn requests(&self) -> Vec<String> {
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

pub fn response(status: u16, body: impl Into<String>) -> MockResponse {
    MockResponse {
        status,
        body: body.into(),
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

pub fn has_bearer_token(request: &str, token: &str) -> bool {
    request
        .split_once("\r\n\r\n")
        .unwrap_or((request, ""))
        .0
        .lines()
        .any(|line| {
            let Some((name, value)) = line.split_once(':') else {
                return false;
            };
            name.eq_ignore_ascii_case("authorization") && value.trim() == format!("Bearer {token}")
        })
}
