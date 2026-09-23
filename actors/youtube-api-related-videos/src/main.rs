use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{env, fmt::Write as _, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://ytapi.scrappa.co";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

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

fn encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
        {
            encoded.push(byte as char);
        } else {
            write!(encoded, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    encoded
}

fn build_related_videos_url(input: &Value, api_base_url: &Url) -> Result<Url> {
    let id = input
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| {
            anyhow!("YouTube video ID \"id\" not provided. Please provide a value for \"id\" in the input.")
        })?;
    let continuation = input
        .get("continuation")
        .and_then(Value::as_str)
        .filter(|continuation| !continuation.trim().is_empty());

    let mut url = endpoint_url(api_base_url, &["videos", "related"])?;
    let mut query = format!("id={}", encode_component(id));
    if let Some(continuation) = continuation {
        query.push_str("&continuation=");
        query.push_str(&encode_component(continuation));
    }
    url.set_query(Some(&query));
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

async fn fetch_related_videos(client: &Client, url: &Url) -> Result<Value> {
    let response = client.get(url.clone()).send().await.map_err(|error| {
        if error.is_timeout() {
            anyhow!(
                "Scrappa API request timed out after {}s",
                REQUEST_TIMEOUT.as_secs()
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
    response.json().await.map_err(|error| {
        if error.is_timeout() {
            anyhow!(
                "Scrappa API request timed out after {}s",
                REQUEST_TIMEOUT.as_secs()
            )
        } else {
            anyhow!("Scrappa API response was not valid JSON: {error}")
        }
    })
}

fn related_videos(response_data: &Value) -> Value {
    response_data
        .get("videos")
        .filter(|videos| !videos.is_null())
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()))
}

async fn push_dataset_items(client: &Client, config: &ActorConfig, videos: &Value) -> Result<()> {
    if videos.as_array().is_some_and(Vec::is_empty) {
        return Ok(());
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(videos)
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

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let url = build_related_videos_url(&input, &config.scrappa_api_base_url)?;
    println!("Fetching from: {url}");

    let response_data = fetch_related_videos(client, &url).await?;
    let videos = related_videos(&response_data);
    push_dataset_items(client, config, &videos).await?;
    let video_id = input.get("id").and_then(Value::as_str).unwrap_or_default();
    let count = videos
        .as_array()
        .map(Vec::len)
        .or_else(|| videos.is_object().then_some(1))
        .unwrap_or(0);
    println!("Successfully fetched {count} related videos for video id: {video_id}");

    if let Some(continuation) = response_data
        .get("continuation")
        .and_then(Value::as_str)
        .filter(|continuation| !continuation.is_empty())
    {
        println!("Continuation token available for next page: {continuation}");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube related videos: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("Could not create HTTP client")?;
    run_actor(&client, &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
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
                for response in responses {
                    let deadline = Instant::now() + Duration::from_secs(5);
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
                    let _ = request_sender.send(request);
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        429 => "Too Many Requests",
                        500 => "Internal Server Error",
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
        Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap()
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

    #[test]
    fn url_builder_encodes_components_and_preserves_continuation_rules() {
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let url = build_related_videos_url(
            &json!({"id":"video id!*'()/é", "continuation":"next page/token"}),
            &base,
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://ytapi.scrappa.co/videos/related?id=video%20id!*%27()%2F%C3%A9&continuation=next%20page%2Ftoken"
        );

        let url =
            build_related_videos_url(&json!({"id":"dQw4w9WgXcQ", "continuation":"  "}), &base)
                .unwrap();
        assert_eq!(
            url.as_str(),
            "https://ytapi.scrappa.co/videos/related?id=dQw4w9WgXcQ"
        );
    }

    #[test]
    fn url_builder_requires_a_nonempty_video_id() {
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let error = build_related_videos_url(&json!({}), &base).unwrap_err();
        assert_eq!(
            error.to_string(),
            "YouTube video ID \"id\" not provided. Please provide a value for \"id\" in the input."
        );
        assert!(build_related_videos_url(&json!({"id":""}), &base).is_err());
    }

    #[test]
    fn response_selection_preserves_arrays_and_objects_and_defaults_nullish_values() {
        assert_eq!(related_videos(&json!({})), json!([]));
        assert_eq!(related_videos(&json!({"videos":null})), json!([]));
        assert_eq!(
            related_videos(&json!({"videos":[{"videoId":"one"},{"videoId":"two"}]})),
            json!([{"videoId":"one"},{"videoId":"two"}])
        );
        assert_eq!(
            related_videos(&json!({"videos":{"videoId":"one"}})),
            json!({"videoId":"one"})
        );
    }

    #[tokio::test]
    async fn actor_fetches_related_videos_and_posts_each_as_dataset_rows() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"video id","continuation":"next page/token"}"#),
            response(
                200,
                r#"{"videos":[{"videoId":"one"},{"videoId":"two"}],"continuation":"later"}"#,
            ),
            response(201, ""),
        ]);
        let config = config(&server.base_url);
        run_actor(&client(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        let (method, path, _) = request_parts(&requests[0]);
        assert_eq!(method, "GET");
        assert_eq!(path, "/v2/key-value-stores/test-store/records/INPUT");
        assert!(has_test_bearer_token(&requests[0]));

        let (method, path, _) = request_parts(&requests[1]);
        assert_eq!(method, "GET");
        assert_eq!(
            path,
            "/videos/related?id=video%20id&continuation=next%20page%2Ftoken"
        );
        assert!(!has_test_bearer_token(&requests[1]));

        let (method, path, body) = request_parts(&requests[2]);
        assert_eq!(method, "POST");
        assert_eq!(path, "/v2/datasets/test-dataset/items");
        assert!(has_test_bearer_token(&requests[2]));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{"videoId":"one"},{"videoId":"two"}])
        );
    }

    #[tokio::test]
    async fn empty_related_videos_do_not_write_a_dataset_item() {
        let server = MockServer::start(Vec::new());
        let config = config(&server.base_url);
        push_dataset_items(&client(), &config, &json!([]))
            .await
            .unwrap();
        assert!(server.requests().is_empty());
    }

    #[tokio::test]
    async fn object_related_videos_are_posted_as_one_dataset_item() {
        let server = MockServer::start(vec![response(201, "")]);
        let config = config(&server.base_url);
        let video = json!({"videoId":"single"});
        push_dataset_items(&client(), &config, &video)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        let (method, path, body) = request_parts(&requests[0]);
        assert_eq!(method, "POST");
        assert_eq!(path, "/v2/datasets/test-dataset/items");
        assert_eq!(serde_json::from_str::<Value>(body).unwrap(), video);
    }

    #[tokio::test]
    async fn scrappa_http_errors_fail_without_retrying() {
        let server = MockServer::start(vec![response(429, r#"{"error":"limited"}"#)]);
        let error = fetch_related_videos(&client(), &server.base_url)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API request failed with 429 Too Many Requests"
        );
        assert_eq!(server.requests().len(), 1);
    }
}
