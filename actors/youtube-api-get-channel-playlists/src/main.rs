use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{collections::HashSet, env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/youtube/channel-playlists";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    input_key: String,
    apify_token: String,
    scrappa_api_key: String,
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
            scrappa_api_key: required_env("SCRAPPA_API_KEY")?,
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

fn parse_ids(value: &Value, ids: &mut Vec<String>, seen: &mut HashSet<String>) {
    match value {
        Value::String(value) => {
            for id in value.split(',').map(str::trim).filter(|id| !id.is_empty()) {
                if seen.insert(id.to_owned()) {
                    ids.push(id.to_owned());
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                parse_ids(value, ids, seen);
            }
        }
        _ => {}
    }
}

fn get_channel_ids(input: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    parse_ids(&input["ids"], &mut ids, &mut seen);
    parse_ids(&input["id"], &mut ids, &mut seen);
    ids
}

fn assert_no_unsupported_continuation(input: &Value) -> Result<()> {
    if input
        .get("continuation")
        .and_then(Value::as_str)
        .is_some_and(|continuation| !continuation.trim().is_empty())
    {
        bail!("The \"continuation\" token is not supported by the Scrappa YouTube channel playlists endpoint.");
    }
    Ok(())
}

fn build_channel_playlists_url(id: &str, base_url: &Url) -> Result<Url> {
    if id.is_empty() {
        bail!("Search query \"id\" not provided. Please provide a value for \"id\" in the input.");
    }
    let mut url = base_url.clone();
    url.query_pairs_mut().append_pair("channel_id", id);
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

async fn fetch_playlists(client: &Client, config: &ActorConfig, url: &Url) -> Result<Value> {
    let response = client
        .get(url.clone())
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .header(reqwest::header::ACCEPT, "application/json")
        .header("X-API-Key", &config.scrappa_api_key)
        .send()
        .await
        .context("Scrappa API request failed")?;
    response_json(response, "Scrappa API request").await
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    playlists: &[Value],
) -> Result<()> {
    if playlists.is_empty() {
        return Ok(());
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(playlists)
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
        Value::Number(value) => value.as_f64().is_none_or(|number| number != 0.0),
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
            .map(|value| match value {
                Value::Null => String::new(),
                _ => js_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let ids = get_channel_ids(&input);
    if ids.is_empty() {
        bail!("At least one YouTube channel ID must be provided in \"ids\" or \"id\".");
    }
    assert_no_unsupported_continuation(&input)?;

    for id in ids {
        let url = build_channel_playlists_url(&id, &config.scrappa_api_base_url)?;
        println!("Fetching from: {url}");
        let response_data = fetch_playlists(client, config, &url).await?;
        let playlists = response_data
            .get("playlists")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("Scrappa API response is missing a playlists array"))?;
        push_dataset_items(client, config, playlists).await?;
        println!(
            "Successfully fetched {} playlists for query: {id}",
            playlists.len()
        );

        if let Some(continuation) = response_data
            .get("continuation")
            .filter(|continuation| js_truthy(continuation))
        {
            println!(
                "Continuation token available for next page: {}",
                js_string(continuation)
            );
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube channel playlists: {error:#}");
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
                let deadline = Instant::now() + Duration::from_secs(5);
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

        fn scrappa_url(&self) -> Url {
            let mut url = self.base_url.clone();
            url.set_path("/api/youtube/channel-playlists");
            url
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
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
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

    fn config(server: &MockServer) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: server.base_url.clone(),
            scrappa_api_base_url: server.scrappa_url(),
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    fn request_line(request: &str) -> &str {
        request.lines().next().unwrap_or_default()
    }

    fn request_body(request: &str) -> Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    #[test]
    fn preserves_input_schema_prefills() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["id"]["prefill"],
            "UCJZv4d5rbIKd4QHMPkcABCw"
        );
        assert_eq!(
            schema["properties"]["ids"]["prefill"],
            "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw"
        );
    }

    #[test]
    fn parses_batch_and_legacy_ids_in_order_without_duplicates() {
        let input = serde_json::json!({
            "ids": ["UC1, UC2", ["UC3", "UC2"]],
            "id": "UC4,UC1"
        });
        assert_eq!(
            get_channel_ids(&input),
            vec![
                "UC1".to_owned(),
                "UC2".to_owned(),
                "UC3".to_owned(),
                "UC4".to_owned()
            ]
        );
    }

    #[test]
    fn builds_encoded_channel_playlists_url() {
        let base_url = Url::parse("https://scrappa.co/api/youtube/channel-playlists").unwrap();
        let url = build_channel_playlists_url("UC example/1", &base_url).unwrap();
        assert_eq!(url.path(), "/api/youtube/channel-playlists");
        let pairs = url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect::<Vec<_>>();
        assert_eq!(
            pairs,
            vec![("channel_id".to_owned(), "UC example/1".to_owned())]
        );
    }

    #[test]
    fn rejects_only_nonempty_string_continuation_tokens() {
        assert!(
            assert_no_unsupported_continuation(&serde_json::json!({"continuation": " next "}))
                .is_err()
        );
        assert!(
            assert_no_unsupported_continuation(&serde_json::json!({"continuation": "  "})).is_ok()
        );
        assert!(
            assert_no_unsupported_continuation(&serde_json::json!({"continuation": 1})).is_ok()
        );
    }

    #[tokio::test]
    async fn maps_batch_playlists_to_ordered_dataset_rows_with_both_auth_modes() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":["UC1","UC2"],"id":"UC2,UC3"}"#),
            response(
                200,
                r#"{"playlists":[{"title":"one"},{"title":"two"}],"continuation":"next"}"#,
            ),
            response(200, "{}"),
            response(200, r#"{"playlists":[]}"#),
            response(200, r#"{"playlists":[{"title":"three"}]}"#),
            response(200, "{}"),
        ]);
        let client = Client::new();
        run_actor(&client, &config(&server)).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 6);
        assert!(request_line(&requests[0])
            .starts_with("GET /v2/key-value-stores/test-store/records/INPUT "));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token-not-a-real-credential"));
        assert!(
            request_line(&requests[1]).contains("/api/youtube/channel-playlists?channel_id=UC1 ")
        );
        assert!(requests[1]
            .to_ascii_lowercase()
            .contains("x-api-key: test-scrappa-key"));
        assert!(
            request_line(&requests[3]).contains("/api/youtube/channel-playlists?channel_id=UC2 ")
        );
        assert!(
            request_line(&requests[4]).contains("/api/youtube/channel-playlists?channel_id=UC3 ")
        );
        assert!(request_line(&requests[2]).starts_with("POST /v2/datasets/test-dataset/items "));
        assert!(requests[2]
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token-not-a-real-credential"));
        assert_eq!(
            request_body(&requests[2]),
            serde_json::json!([{"title":"one"}, {"title":"two"}])
        );
        assert_eq!(
            request_body(&requests[5]),
            serde_json::json!([{"title":"three"}])
        );
    }

    #[tokio::test]
    async fn rejects_unsupported_continuation_without_calling_scrappa() {
        let server =
            MockServer::start(vec![response(200, r#"{"id":"UC1","continuation":"next"}"#)]);
        let error = run_actor(&Client::new(), &config(&server))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("continuation"));
        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert!(request_line(&requests[0])
            .starts_with("GET /v2/key-value-stores/test-store/records/INPUT "));
    }

    #[tokio::test]
    async fn returns_scrappa_http_errors() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC1"}"#),
            response(429, r#"{"error":"rate limited"}"#),
        ]);
        let error = run_actor(&Client::new(), &config(&server))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("429 Too Many Requests"));
        assert!(error.to_string().contains("rate limited"));
    }
}
