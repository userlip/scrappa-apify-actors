use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub struct MockResponse {
    pub status: u16,
    pub body: String,
    pub delay: Duration,
}

impl MockResponse {
    pub fn json(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_owned(),
            delay: Duration::ZERO,
        }
    }

    pub fn delayed_json(status: u16, body: &str, delay: Duration) -> Self {
        Self {
            status,
            body: body.to_owned(),
            delay,
        }
    }
}

pub struct MockServer {
    base_url: String,
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
        let thread_stop = Arc::clone(&stop);
        let thread = thread::spawn(move || serve(listener, responses, request_sender, thread_stop));

        Self {
            base_url: format!("http://{address}"),
            requests,
            stop,
            thread: Some(thread),
        }
    }

    pub fn base_url(&self) -> String {
        self.base_url.clone()
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

fn serve(
    listener: TcpListener,
    responses: Vec<MockResponse>,
    requests: Sender<String>,
    stop: Arc<AtomicBool>,
) {
    let deadline = Instant::now() + Duration::from_secs(20);
    for response in responses {
        if stop.load(Ordering::Relaxed) || Instant::now() >= deadline {
            return;
        }
        let mut stream = match accept(&listener, &stop, deadline) {
            Some(stream) => stream,
            None => return,
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
        let request = match read_request(&mut stream) {
            Ok(request) => request,
            Err(_) => return,
        };
        if requests.send(request).is_err() {
            return;
        }
        if !response.delay.is_zero() {
            thread::sleep(response.delay);
        }
        let reason = reason_phrase(response.status);
        let body = response.body.as_bytes();
        let headers = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response.status,
            reason,
            body.len()
        );
        let _ = stream.write_all(headers.as_bytes());
        let _ = stream.write_all(body);
    }
}

fn accept(listener: &TcpListener, stop: &AtomicBool, deadline: Instant) -> Option<TcpStream> {
    loop {
        if stop.load(Ordering::Relaxed) || Instant::now() >= deadline {
            return None;
        }
        match listener.accept() {
            Ok((stream, _)) => return Some(stream),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(1));
            }
            Err(_) => return None,
        }
    }
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut expected_length = None;
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..count]);
        if expected_length.is_none() {
            if let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                });
                expected_length = Some(header_end + 4 + content_length.unwrap_or_default());
            }
        }
        if expected_length.is_some_and(|expected| request.len() >= expected) {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&request).into_owned())
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "OK",
    }
}
