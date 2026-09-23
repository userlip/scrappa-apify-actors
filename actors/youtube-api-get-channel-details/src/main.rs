use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, Url};
use serde_json::{json, Value};
use std::{collections::HashSet, env, time::Duration};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/youtube/channel";
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

fn append_ids(value: Option<&Value>, seen: &mut HashSet<String>, ids: &mut Vec<String>) {
    match value {
        Some(Value::String(value)) => {
            for id in value.split(',').map(str::trim).filter(|id| !id.is_empty()) {
                if seen.insert(id.to_owned()) {
                    ids.push(id.to_owned());
                }
            }
        }
        Some(Value::Array(values)) => {
            for value in values {
                append_ids(Some(value), seen, ids);
            }
        }
        _ => {}
    }
}

fn get_channel_ids(input: &Value) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    append_ids(input.get("ids"), &mut seen, &mut ids);
    append_ids(input.get("id"), &mut seen, &mut ids);
    ids
}

fn build_channel_details_url(id: &str, api_base_url: &Url) -> Result<Url> {
    if id.trim().is_empty() {
        bail!("Channel \"id\" not provided in input.");
    }

    let mut url = api_base_url.clone();
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

async fn fetch_channel_details(client: &Client, url: &Url, api_key: &str) -> Result<Value> {
    let response = client
        .get(url.clone())
        .header("X-API-Key", api_key)
        .header("Accept", "application/json")
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
        bail!("Request failed with status code {}", status.as_u16());
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

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let ids = get_channel_ids(&input);
    if ids.is_empty() {
        bail!("At least one YouTube channel ID must be provided in \"ids\" or \"id\".");
    }

    let mut success_count = 0;
    let mut failure_count = 0;
    for id in ids {
        let result = async {
            let url = build_channel_details_url(&id, &config.scrappa_api_base_url)?;
            println!("Fetching from: {url}");
            let data = fetch_channel_details(client, &url, &config.scrappa_api_key).await?;
            push_dataset_data(client, config, &data).await
        }
        .await;

        match result {
            Ok(()) => success_count += 1,
            Err(error) => {
                let message = error.to_string();
                eprintln!("Failed to fetch YouTube channel details for id {id}: {message}");
                let error_row = json!({ "id": id, "error": message, "success": false });
                push_dataset_data(client, config, &error_row).await?;
                failure_count += 1;
            }
        }
    }

    if success_count == 0 {
        bail!("Failed to fetch details for all {failure_count} channel(s).");
    }

    println!(
        "Successfully fetched details for {success_count} channel(s); {failure_count} failed."
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::builder()
        .build()
        .context("Failed to create HTTP client")?;
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
            mpsc, Arc,
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
        requests: mpsc::Receiver<String>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (recorded_requests, requests) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                let mut responses = responses.into_iter();
                while !stopped.load(Ordering::Relaxed) && Instant::now() < deadline {
                    let (mut stream, _) = match listener.accept() {
                        Ok(connection) => connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                            continue;
                        }
                        Err(_) => break,
                    };
                    let request = read_request(&mut stream).unwrap_or_default();
                    if recorded_requests.send(request).is_err() {
                        break;
                    }
                    let Some(response) = responses.next() else {
                        break;
                    };
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
                        break;
                    }
                }
            });
            Self {
                base_url: Url::parse(&format!("http://{address}/")).unwrap(),
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

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    fn config(base_url: &Url, scrappa_url: &Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: scrappa_url.clone(),
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
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

    fn has_header(request: &str, name: &str, value: &str) -> bool {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .skip(1)
            .any(|line| {
                let Some((header_name, header_value)) = line.split_once(':') else {
                    return false;
                };
                header_name.eq_ignore_ascii_case(name) && header_value.trim() == value
            })
    }

    #[test]
    fn channel_ids_keep_batch_order_and_legacy_alias_without_duplicates() {
        let input = json!({
            "ids": [" UC1, UC2 ", ["UC2", "UC3"]],
            "id": "UC3,UC4"
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
    fn channel_url_encodes_the_id_as_a_query_parameter() {
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let url = build_channel_details_url("UC example", &base).unwrap();
        assert_eq!(
            url.as_str(),
            "https://scrappa.co/api/youtube/channel?channel_id=UC+example"
        );
    }

    #[test]
    fn actor_input_schema_keeps_single_and_batch_prefills() {
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

    #[tokio::test]
    async fn batches_preserve_response_rows_and_write_one_failure_row_per_channel() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":" UC one , UC2 ","id":"UC2"}"#),
            response(
                200,
                r#"[{"id":"UC one","name":"first"},{"id":"UC one","name":"second"}]"#,
            ),
            response(201, ""),
            response(429, "{}"),
            response(201, ""),
        ]);
        let scrappa_url = server.base_url.join("api/youtube/channel").unwrap();
        let config = config(&server.base_url, &scrappa_url);
        let client = Client::builder().build().unwrap();

        run_actor(&client, &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert!(has_header(
            &requests[0],
            "Authorization",
            "Bearer test-token-not-a-real-credential"
        ));
        assert_eq!(
            request_parts(&requests[1]).1,
            "/api/youtube/channel?channel_id=UC+one"
        );
        assert!(has_header(&requests[1], "X-API-Key", "test-scrappa-key"));
        assert!(has_header(&requests[1], "Accept", "application/json"));

        let (method, path, body) = request_parts(&requests[2]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert!(has_header(
            &requests[2],
            "Authorization",
            "Bearer test-token-not-a-real-credential"
        ));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([
                {"id":"UC one","name":"first"},
                {"id":"UC one","name":"second"}
            ])
        );

        assert_eq!(
            request_parts(&requests[3]).1,
            "/api/youtube/channel?channel_id=UC2"
        );
        let (method, path, body) = request_parts(&requests[4]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!({
                "id": "UC2",
                "error": "Request failed with status code 429",
                "success": false
            })
        );
    }
}
