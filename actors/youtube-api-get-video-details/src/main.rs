use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, Url};
use serde_json::Value;
use std::{env, time::Duration};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://ytapi.scrappa.co";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    input_key: String,
    apify_token: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.trim().is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

fn build_video_details_url(input: &Value, api_base_url: &Url) -> Result<Url> {
    let id = input
        .as_object()
        .and_then(|object| object.get("id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| anyhow!("YouTube video ID \"id\" is required."))?;

    let mut url = endpoint_url(api_base_url, &["videos"])?;
    url.query_pairs_mut().append_pair("id", id);
    Ok(url)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = response.text().await.unwrap_or_default();
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "{operation} failed with {} {reason}{detail}",
            status.as_u16()
        );
    }

    response
        .json()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

async fn get_input(client: &Client, config: &ActorConfig) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify INPUT request failed")?;
    response_json(response, "Apify INPUT request").await
}

async fn fetch_video_details(client: &Client, url: &Url) -> Result<Value> {
    let response = client
        .get(url.clone())
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "Scrappa API request timed out after {}s",
                    SCRAPPA_REQUEST_TIMEOUT.as_secs()
                )
            } else {
                anyhow!("Scrappa API request failed: {error}")
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        bail!(
            "Scrappa API request failed with {} {reason}",
            status.as_u16()
        );
    }

    response
        .json()
        .await
        .context("Scrappa API response was not valid JSON")
}

async fn push_dataset_data(client: &Client, config: &ActorConfig, data: &Value) -> Result<()> {
    if !data.is_array() && !data.is_object() {
        bail!("Scrappa API response must be an object or array");
    }
    if data.as_array().is_some_and(Vec::is_empty) {
        return Ok(());
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(data)
        .send()
        .await
        .context("Apify dataset write failed")?;
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = response.text().await.unwrap_or_default();
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "Apify dataset write failed with {} {reason}{detail}",
            status.as_u16()
        );
    }

    Ok(())
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    js_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let url = build_video_details_url(&input, &config.scrappa_api_base_url)?;
    println!("Fetching from: {url}");

    let data = fetch_video_details(client, &url).await?;
    push_dataset_data(client, config, &data).await?;

    let result_count = data.as_array().map_or(1, Vec::len);
    let id = input
        .get("id")
        .map(js_string)
        .unwrap_or_else(|| "undefined".to_owned());
    println!("Successfully fetched {result_count} video detail result(s) for id: {id}");

    if let Some(continuation) = data.get("continuation").filter(|value| js_truthy(value)) {
        println!(
            "Continuation token available for next page: {}",
            js_string(continuation)
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube video details: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::builder()
        .build()
        .context("Could not create HTTP client")?;
    run_actor(&client, &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver},
            Arc,
        },
        thread::{self, JoinHandle},
        time::Instant,
    };

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: Url,
        requests: Receiver<String>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
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
                    let request = read_request(&mut stream).unwrap_or_default();
                    if request_sender.send(request).is_err() {
                        return;
                    }
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
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
                    if stream.write_all(message.as_bytes()).is_err() {
                        return;
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

        fn requests(&self) -> Vec<String> {
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
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    fn config(base_url: &Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: base_url.clone(),
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
        }
    }

    fn client() -> Client {
        Client::builder().build().unwrap()
    }

    fn request_parts(request: &str) -> (&str, &str, &str) {
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

    fn has_test_bearer_token(request: &str) -> bool {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .any(|line| {
                let Some((name, value)) = line.split_once(':') else {
                    return false;
                };
                name.eq_ignore_ascii_case("authorization")
                    && value.trim() == "Bearer test-token-not-a-real-credential"
            })
    }

    async fn assert_dataset_preserves_response(data: Value) {
        let input = serde_json::json!({"id":" video-id "});
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(200, &data.to_string()),
            response(201, ""),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(has_test_bearer_token(&requests[0]));
        assert!(!has_test_bearer_token(&requests[1]));
        assert!(has_test_bearer_token(&requests[2]));
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert_eq!(request_parts(&requests[1]).1, "/videos?id=video-id");
        let (method, path, body) = request_parts(&requests[2]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert_eq!(serde_json::from_str::<Value>(body).unwrap(), data);
    }

    #[test]
    fn builds_url_from_trimmed_video_id_and_rejects_missing_id() {
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let url =
            build_video_details_url(&serde_json::json!({"id":" dQw4w9WgXcQ "}), &base).unwrap();
        assert_eq!(url.path(), "/videos");
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "id").unwrap().1,
            "dQw4w9WgXcQ"
        );
        for input in [
            serde_json::json!({}),
            serde_json::json!({"id":"   "}),
            serde_json::json!({"id":7}),
        ] {
            assert!(build_video_details_url(&input, &base)
                .unwrap_err()
                .to_string()
                .contains("id"));
        }
    }

    #[test]
    fn qa_prefill_is_accepted_by_the_rust_url_builder() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let prefill = schema["properties"]["id"]["prefill"].as_str().unwrap();
        let url = build_video_details_url(
            &serde_json::json!({"id":prefill}),
            &Url::parse(SCRAPPA_API_BASE_URL).unwrap(),
        )
        .unwrap();
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "id").unwrap().1,
            prefill
        );
    }

    #[tokio::test]
    async fn local_storage_mock_preserves_object_and_array_dataset_rows() {
        assert_dataset_preserves_response(serde_json::json!({
            "videoId":"video-id",
            "title":"Video",
            "continuation":"next-page"
        }))
        .await;
        assert_dataset_preserves_response(serde_json::json!([
            {"videoId":"video-one"},
            {"videoId":"video-two"}
        ]))
        .await;
    }

    #[tokio::test]
    async fn upstream_and_apify_errors_fail_without_fabricating_rows() {
        let input = r#"{"id":"video-id"}"#;
        let upstream_error = MockServer::start(vec![response(200, input), response(400, "{}")]);
        let error = run_actor(&client(), &config(&upstream_error.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("400 Bad Request"));
        let requests = upstream_error.requests();
        assert_eq!(requests.len(), 2);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));

        let apify_error = MockServer::start(vec![response(401, "unauthorized")]);
        let error = run_actor(&client(), &config(&apify_error.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("401 Unauthorized"));
        assert_eq!(apify_error.requests().len(), 1);
    }

    #[test]
    fn continuation_uses_javascript_truthiness_and_string_coercion() {
        assert!(!js_truthy(&Value::Null));
        assert!(!js_truthy(&serde_json::json!(0)));
        assert!(!js_truthy(&serde_json::json!("")));
        assert!(js_truthy(&serde_json::json!([])));
        assert!(js_truthy(&serde_json::json!({})));
        assert_eq!(js_string(&serde_json::json!(["next", null, 2])), "next,,2");
        assert_eq!(js_string(&serde_json::json!({})), "[object Object]");
    }
}
