use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct MockRequest {
    pub method: String,
    pub target: String,
    pub headers: String,
    pub body: String,
}

pub struct MockResponse {
    status: u16,
    body: String,
    delay: Duration,
}

impl MockResponse {
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_owned(),
            delay: Duration::ZERO,
        }
    }

    pub fn json_after(status: u16, body: &str, delay: Duration) -> Self {
        Self {
            status,
            body: body.to_owned(),
            delay,
        }
    }
}

pub struct MockServer {
    pub base_url: String,
    requests: Receiver<MockRequest>,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl MockServer {
    pub fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let (request_sender, requests) = mpsc::channel();
        let stopped = Arc::new(AtomicBool::new(false));
        let stopped_thread = Arc::clone(&stopped);
        let thread = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            for response in responses {
                let (mut stream, _) = loop {
                    if stopped_thread.load(Ordering::Relaxed) || Instant::now() >= deadline {
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
                let request = read_request(&mut stream).unwrap_or_else(|error| {
                    panic!("could not read mock request: {error}");
                });
                if request_sender.send(request).is_err() {
                    return;
                }
                thread::sleep(response.delay);
                let reason = match response.status {
                    200 => "OK",
                    201 => "Created",
                    404 => "Not Found",
                    429 => "Too Many Requests",
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
            stopped,
            thread: Some(thread),
        }
    }

    pub fn finish(mut self) -> Vec<MockRequest> {
        self.stopped.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
        self.requests.try_iter().collect()
    }
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<MockRequest> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let mut header_end = None;
    let mut content_length = 0;
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
        if header_end.is_none() {
            if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let end = index + 4;
                let header_text = String::from_utf8_lossy(&bytes[..end]);
                content_length = header_text
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
                header_end = Some(end);
            }
        }
        if header_end.is_some_and(|end| bytes.len() >= end + content_length) {
            break;
        }
    }
    let header_end = header_end.unwrap_or(bytes.len());
    let headers = String::from_utf8_lossy(&bytes[..header_end]).into_owned();
    let mut lines = headers.lines();
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().unwrap_or_default().to_owned();
    let body = String::from_utf8_lossy(&bytes[header_end..]).into_owned();
    Ok(MockRequest {
        method,
        target,
        headers,
        body,
    })
}
