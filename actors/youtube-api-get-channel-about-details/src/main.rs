use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response, Url};
use serde_json::{json, Value};
use std::{env, time::Duration};

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
    fn from_env(scrappa_api_key: String) -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
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

fn scrappa_api_key(value: Option<&str>) -> Result<String> {
    value.filter(|key| !key.is_empty()).map(str::to_owned).ok_or_else(|| {
        anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
    })
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments.iter().copied());
    Ok(url)
}

fn build_channel_about_details_url(id: &str, api_base_url: &Url) -> Url {
    let mut url = api_base_url.clone();
    url.set_query(None);
    url.query_pairs_mut().append_pair("channel_id", id);
    url
}

fn parse_ids(value: Option<&Value>, ids: &mut Vec<String>) {
    match value {
        Some(Value::Array(values)) => {
            for value in values {
                parse_ids(Some(value), ids);
            }
        }
        Some(Value::String(value)) => {
            for id in value.split(',').map(str::trim).filter(|id| !id.is_empty()) {
                if !ids.iter().any(|existing| existing == id) {
                    ids.push(id.to_owned());
                }
            }
        }
        _ => {}
    }
}

fn get_channel_ids(input: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    parse_ids(input.get("ids"), &mut ids);
    parse_ids(input.get("id"), &mut ids);
    ids
}

fn nullish_field(data: &Value, field: &str) -> Value {
    data.get(field)
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn split_subscriber_and_video_count(value: &Value) -> (Value, Value) {
    let Some(value) = value.as_str() else {
        return (Value::Null, Value::Null);
    };
    let Some(index) = value.find(" subscribers") else {
        return (Value::String(value.to_owned()), Value::Null);
    };

    let subscriber_end = index + " subscribers".len();
    let remainder = &value[subscriber_end..];
    if remainder.is_empty() {
        return (
            Value::String(value[..subscriber_end].to_owned()),
            Value::Null,
        );
    }

    let count_and_videos = remainder.trim_start_matches(char::is_whitespace);
    if count_and_videos.len() < remainder.len()
        && count_and_videos
            .strip_suffix(" videos")
            .is_some_and(|count| !count.is_empty())
    {
        let video_count = count_and_videos;
        return (
            Value::String(value[..subscriber_end].to_owned()),
            Value::String(video_count.to_owned()),
        );
    }

    (Value::String(value.to_owned()), Value::Null)
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

fn to_channel_about_details(data: &Value) -> Result<Value> {
    if data.is_null() {
        bail!("Cannot read properties of null (reading 'channelId')");
    }

    let channel_id = data
        .get("channelId")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("id").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null);
    let (subscriber_count, parsed_video_count) =
        split_subscriber_and_video_count(data.get("subscriberCount").unwrap_or(&Value::Null));
    let video_count = data
        .get("videoCount")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(parsed_video_count);
    let channel_url = data
        .get("channelUrl")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("url").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or_else(|| {
            if js_truthy(&channel_id) {
                Value::String(format!(
                    "https://www.youtube.com/channel/{}",
                    js_string(&channel_id)
                ))
            } else {
                Value::Null
            }
        });

    Ok(json!({
        "channelId": channel_id,
        "stats": {
            "joinDate": nullish_field(data, "joinedDate"),
            "viewCount": nullish_field(data, "viewCount"),
            "country": nullish_field(data, "country"),
        },
        "links": data.get("links").and_then(Value::as_array).cloned().unwrap_or_default(),
        "details": {
            "description": nullish_field(data, "description"),
            "email": nullish_field(data, "email"),
            "name": nullish_field(data, "name"),
            "subscriberCount": subscriber_count,
            "videoCount": video_count,
            "channelUrl": channel_url,
        }
    }))
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

async fn fetch_channel_about_details(
    client: &Client,
    config: &ActorConfig,
    id: &str,
) -> Result<Value> {
    let url = build_channel_about_details_url(id, &config.scrappa_api_base_url);
    println!("Fetching from: {url}");
    let response = client
        .get(url)
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .header("X-API-Key", &config.scrappa_api_key)
        .header(reqwest::header::ACCEPT, "application/json")
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

    if !response.status().is_success() {
        bail!(
            "Request failed with status code {}",
            response.status().as_u16()
        );
    }
    let body = response
        .text()
        .await
        .context("Scrappa API response could not be read")?;
    Ok(serde_json::from_str(&body).unwrap_or(Value::String(body)))
}

async fn push_dataset_data(client: &Client, config: &ActorConfig, data: &Value) -> Result<()> {
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
            let data = fetch_channel_about_details(client, config, &id).await?;
            let row = to_channel_about_details(&data)?;
            push_dataset_data(client, config, &row).await
        }
        .await;

        match result {
            Ok(()) => success_count += 1,
            Err(error) => {
                failure_count += 1;
                let message = error.to_string();
                eprintln!("Failed to fetch YouTube channel about details for id {id}: {message}");
                push_dataset_data(
                    client,
                    config,
                    &json!({"id": id, "error": message, "success": false}),
                )
                .await?;
            }
        }
    }

    if success_count == 0 {
        bail!("Failed to fetch about details for all {failure_count} channel(s).");
    }
    println!("Successfully fetched about details for {success_count} channel(s); {failure_count} failed.");
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube channel about details: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let api_key = scrappa_api_key(env::var("SCRAPPA_API_KEY").ok().as_deref())?;
    let config = ActorConfig::from_env(api_key)?;
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
            let (sender, requests) = mpsc::channel();
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
                    if sender.send(request).is_err() {
                        return;
                    }
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
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

    fn config(apify_base_url: &Url, scrappa_api_base_url: &Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: apify_base_url.clone(),
            scrappa_api_base_url: scrappa_api_base_url.clone(),
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
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

    fn has_header(request: &str, name: &str, expected: &str) -> bool {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .skip(1)
            .any(|line| {
                line.split_once(':').is_some_and(|(key, value)| {
                    key.eq_ignore_ascii_case(name) && value.trim().eq_ignore_ascii_case(expected)
                })
            })
    }

    #[test]
    fn parses_batch_and_legacy_inputs_and_builds_encoded_endpoint_params() {
        let ids = get_channel_ids(&json!({ "ids": ["UC1, UC2", ["UC2", "UC3"]], "id": "UC3,UC4" }));
        assert_eq!(
            ids,
            vec![
                "UC1".to_owned(),
                "UC2".to_owned(),
                "UC3".to_owned(),
                "UC4".to_owned()
            ]
        );
        assert!(get_channel_ids(&json!({ "ids": [7, null], "id": false })).is_empty());

        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let url = build_channel_about_details_url("UC example", &base);
        assert_eq!(url.path(), "/api/youtube/channel");
        assert_eq!(
            url.query_pairs()
                .find(|(key, _)| key == "channel_id")
                .unwrap()
                .1,
            "UC example"
        );
        assert_eq!(
            url.as_str(),
            "https://scrappa.co/api/youtube/channel?channel_id=UC+example"
        );
    }

    #[test]
    fn actor_schema_prefills_parse_into_valid_channel_requests() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let ids_prefill = schema["properties"]["ids"]["prefill"].as_str().unwrap();
        let ids = get_channel_ids(&json!({ "ids": ids_prefill }));
        assert_eq!(ids.len(), 2);
        for id in ids {
            let url =
                build_channel_about_details_url(&id, &Url::parse(SCRAPPA_API_BASE_URL).unwrap());
            assert_eq!(
                url.query_pairs()
                    .find(|(key, _)| key == "channel_id")
                    .unwrap()
                    .1,
                id.as_str()
            );
        }
        let legacy_id = schema["properties"]["id"]["prefill"].as_str().unwrap();
        assert_eq!(
            get_channel_ids(&json!({ "id": legacy_id })),
            vec![legacy_id.to_owned()]
        );
    }

    #[test]
    fn maps_about_fields_counts_and_derived_channel_url() {
        assert_eq!(
            to_channel_about_details(&json!({
                "id": "UC123",
                "name": "Example Channel",
                "description": "About text",
                "subscriberCount": "2.64M subscribers 6.3K videos",
                "viewCount": null,
                "country": "United States",
                "joinedDate": "Joined Aug 23, 2007",
                "links": [{"title":"Website","url":"example.com"}]
            }))
            .unwrap(),
            json!({
                "channelId": "UC123",
                "stats": {"joinDate":"Joined Aug 23, 2007", "viewCount":null, "country":"United States"},
                "links": [{"title":"Website","url":"example.com"}],
                "details": {
                    "description":"About text", "email":null, "name":"Example Channel",
                    "subscriberCount":"2.64M subscribers", "videoCount":"6.3K videos",
                    "channelUrl":"https://www.youtube.com/channel/UC123"
                }
            })
        );

        let explicit = to_channel_about_details(&json!({
            "channelId":"UC456", "channelUrl":"", "url":"fallback", "subscriberCount":"not formatted", "videoCount":42
        })).unwrap();
        assert_eq!(explicit["channelId"], "UC456");
        assert_eq!(explicit["details"]["channelUrl"], "");
        assert_eq!(explicit["details"]["subscriberCount"], "not formatted");
        assert_eq!(explicit["details"]["videoCount"], 42);

        let empty = to_channel_about_details(&json!({ "id":"", "subscriberCount": 12 })).unwrap();
        assert_eq!(empty["channelId"], "");
        assert_eq!(empty["details"]["channelUrl"], Value::Null);
        assert_eq!(empty["details"]["subscriberCount"], Value::Null);
        assert_eq!(empty["details"]["videoCount"], Value::Null);
        assert!(to_channel_about_details(&Value::Null).is_err());
    }

    #[tokio::test]
    async fn reads_apify_input_fetches_each_channel_and_writes_success_and_error_rows() {
        let input = json!({"ids":"UC1,UC2"}).to_string();
        let row =
            json!({ "id":"UC2", "error":"Request failed with status code 500", "success":false })
                .to_string();
        let server = MockServer::start(vec![
            response(200, &input),
            response(
                200,
                r#"{"id":"UC1","subscriberCount":"10 subscribers 2 videos"}"#,
            ),
            response(201, ""),
            response(500, "upstream failure"),
            response(201, ""),
        ]);
        let mut scrappa_base = server.base_url.clone();
        scrappa_base.set_path("/api/youtube/channel");
        run_actor(&client(), &config(&server.base_url, &scrappa_base))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert!(has_header(
            &requests[0],
            "authorization",
            "Bearer test-token-not-a-real-credential"
        ));
        assert_eq!(
            request_parts(&requests[1]).1,
            "/api/youtube/channel?channel_id=UC1"
        );
        assert!(has_header(&requests[1], "x-api-key", "test-scrappa-key"));
        assert!(has_header(&requests[1], "accept", "application/json"));
        let (method, path, body) = request_parts(&requests[2]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap()["details"]["videoCount"],
            "2 videos"
        );
        let (method, path, body) = request_parts(&requests[4]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::from_str::<Value>(&row).unwrap()
        );
    }

    #[tokio::test]
    async fn all_channel_failures_write_error_rows_then_fail_the_actor() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC1"}"#),
            response(400, "bad request"),
            response(201, ""),
        ]);
        let mut scrappa_base = server.base_url.clone();
        scrappa_base.set_path("/api/youtube/channel");
        let error = run_actor(&client(), &config(&server.base_url, &scrappa_base))
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Failed to fetch about details for all 1 channel(s)."
        );

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        let (_, path, body) = request_parts(&requests[2]);
        assert_eq!(path, "/v2/datasets/test-dataset/items");
        let row: Value = serde_json::from_str(body).unwrap();
        assert_eq!(row["id"], "UC1");
        assert_eq!(row["error"], "Request failed with status code 400");
        assert_eq!(row["success"], false);
    }

    #[test]
    fn requires_scrappa_api_key() {
        assert_eq!(scrappa_api_key(Some("test-key")).unwrap(), "test-key");
        assert!(scrappa_api_key(None)
            .unwrap_err()
            .to_string()
            .contains("SCRAPPA_API_KEY environment variable is not set"));
        assert!(scrappa_api_key(Some("")).is_err());
    }
}
