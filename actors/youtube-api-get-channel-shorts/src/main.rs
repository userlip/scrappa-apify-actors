use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{collections::HashSet, env, future::Future, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const TARGET_SHORT_COUNT: usize = 10;
const MAX_FILTER_SCAN_PAGES: usize = 10;

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

fn parse_ids(value: &Value, ids: &mut Vec<String>) {
    match value {
        Value::String(value) => ids.extend(
            value
                .split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_owned),
        ),
        Value::Array(values) => values.iter().for_each(|value| parse_ids(value, ids)),
        _ => {}
    }
}

fn channel_ids(input: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(value) = input.get("ids") {
        parse_ids(value, &mut ids);
    }
    if let Some(value) = input.get("id") {
        parse_ids(value, &mut ids);
    }
    let mut seen = HashSet::with_capacity(ids.len());
    ids.retain(|id| seen.insert(id.clone()));
    ids
}

fn assert_continuation_matches_batch(input: &Value, ids: &[String]) -> Result<()> {
    if ids.len() > 1
        && input
            .get("continuation")
            .and_then(Value::as_str)
            .is_some_and(|token| !token.trim().is_empty())
    {
        bail!("The \"continuation\" token can only be used with a single YouTube channel ID.");
    }
    Ok(())
}

fn build_channel_shorts_url(
    input: &Value,
    channel_id: &str,
    continuation: &Value,
    api_base_url: &Url,
) -> Result<Url> {
    if channel_id.is_empty() {
        bail!("Search query \"id\" not provided. Please provide a value for \"id\" in the input.");
    }

    let mut url = endpoint_url(api_base_url, &["youtube", "channel-videos"])?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("channel_id", channel_id);
        if let Some(sort) = input
            .get("sort")
            .and_then(Value::as_str)
            .filter(|sort| !sort.trim().is_empty())
        {
            query.append_pair("sort", sort);
        }
        if let Some(continuation) = continuation
            .as_str()
            .filter(|continuation| !continuation.trim().is_empty())
        {
            query.append_pair("continuation", continuation);
        }
    }
    Ok(url)
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

fn is_short_video(video: &Value) -> bool {
    if video.get("isShort") == Some(&Value::Bool(true)) {
        return true;
    }

    video
        .get("type")
        .filter(|value| !value.is_null())
        .or_else(|| video.get("videoType"))
        .is_some_and(|kind| js_string(kind).to_lowercase() == "short")
}

fn response_videos(response_data: &Value) -> &[Value] {
    response_data
        .get("videos")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn response_continuation(response_data: &Value) -> Value {
    [
        response_data.get("continuation"),
        response_data.get("continuationToken"),
        response_data
            .get("pagination")
            .and_then(|pagination| pagination.get("continuation")),
        response_data
            .get("pagination")
            .and_then(|pagination| pagination.get("continuationToken")),
    ]
    .into_iter()
    .flatten()
    .find(|value| !value.is_null())
    .cloned()
    .unwrap_or_else(|| Value::String(String::new()))
}

struct FilteredVideos {
    videos: Vec<Value>,
    continuation: Value,
}

async fn collect_filtered_channel_videos<F, Fut>(
    input: &Value,
    channel_id: &str,
    api_base_url: &Url,
    mut fetch_page: F,
) -> Result<FilteredVideos>
where
    F: FnMut(Url) -> Fut,
    Fut: Future<Output = Result<Value>>,
{
    let mut videos = Vec::new();
    let mut continuation = input
        .get("continuation")
        .cloned()
        .unwrap_or_else(|| Value::String(String::new()));
    let mut next_continuation = Value::String(String::new());

    for _ in 0..MAX_FILTER_SCAN_PAGES {
        if videos.len() >= TARGET_SHORT_COUNT {
            break;
        }

        let url = build_channel_shorts_url(input, channel_id, &continuation, api_base_url)?;
        let response_data = fetch_page(url).await?;
        videos.extend(
            response_videos(&response_data)
                .iter()
                .filter(|video| is_short_video(video))
                .cloned(),
        );

        next_continuation = response_continuation(&response_data);
        if !js_truthy(&next_continuation) || next_continuation == continuation {
            break;
        }
        continuation = next_continuation.clone();
    }

    Ok(FilteredVideos {
        videos,
        continuation: next_continuation,
    })
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

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    let message = error.to_string();
    if error.is_timeout() || message.contains("aborted") {
        anyhow!(
            "Scrappa API request timed out after {}s",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        anyhow!(message)
    }
}

async fn fetch_scrappa_page(client: &Client, config: &ActorConfig, url: Url) -> Result<Value> {
    let response = client
        .get(url)
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .header("X-API-Key", &config.scrappa_api_key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(scrappa_request_error)?;
    response_json(response, "Scrappa API request").await
}

async fn push_dataset_items(client: &Client, config: &ActorConfig, items: &[Value]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(items)
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
    let input = if input.is_object() {
        input
    } else {
        Value::Object(serde_json::Map::new())
    };
    let ids = channel_ids(&input);
    if ids.is_empty() {
        bail!("At least one YouTube channel ID must be provided in \"ids\" or \"id\".");
    }
    assert_continuation_matches_batch(&input, &ids)?;

    for channel_id in ids {
        let result = collect_filtered_channel_videos(
            &input,
            &channel_id,
            &config.scrappa_api_base_url,
            |url| async move { fetch_scrappa_page(client, config, url).await },
        )
        .await?;
        push_dataset_items(client, config, &result.videos).await?;
        println!(
            "Successfully fetched {} videos for query: {channel_id}",
            result.videos.len()
        );
        if js_truthy(&result.continuation) {
            println!(
                "Continuation token available for next page: {}",
                js_string(&result.continuation)
            );
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
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
        future,
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
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
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (recorded_requests, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                for response in responses {
                    let (mut stream, _) = loop {
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                if Instant::now() >= deadline {
                                    return;
                                }
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(_) => return,
                        }
                    };
                    let request = read_request(&mut stream).unwrap_or_default();
                    if recorded_requests.send(request).is_err() {
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
                thread: Some(thread),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.try_iter().collect()
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        let mut header_end = None;
        let mut content_length = None;
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if header_end.is_none() {
                header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
                if let Some(header_end) = header_end {
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
            }
            if let (Some(header_end), Some(content_length)) = (header_end, content_length) {
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn config(base_url: Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: base_url,
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
        request.lines().find_map(|line| {
            let (header, value) = line.split_once(':')?;
            header.eq_ignore_ascii_case(name).then_some(value.trim())
        })
    }

    fn request_body(request: &str) -> Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    #[test]
    fn input_schema_keeps_both_channel_id_prefills() {
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
    fn deduplicates_batch_and_legacy_channel_ids_in_input_order() {
        assert_eq!(
            channel_ids(&serde_json::json!({"ids":["UC1, UC2", ["UC3"]], "id":"UC2,UC4"})),
            vec![
                "UC1".to_owned(),
                "UC2".to_owned(),
                "UC3".to_owned(),
                "UC4".to_owned()
            ]
        );
    }

    #[test]
    fn rejects_continuation_for_multiple_distinct_channels() {
        let input = serde_json::json!({"ids":"UC1,UC2", "continuation":"next page"});
        let ids = channel_ids(&input);
        assert_eq!(
            assert_continuation_matches_batch(&input, &ids)
                .unwrap_err()
                .to_string(),
            "The \"continuation\" token can only be used with a single YouTube channel ID."
        );
        assert!(assert_continuation_matches_batch(
            &serde_json::json!({"ids":"UC1,UC1", "continuation":"next"}),
            &channel_ids(&serde_json::json!({"ids":"UC1,UC1", "continuation":"next"}))
        )
        .is_ok());
    }

    #[test]
    fn builds_channel_shorts_url_with_query_encoding_and_optional_fields() {
        let base_url = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let url = build_channel_shorts_url(
            &serde_json::json!({"sort":"popular"}),
            "UC example",
            &serde_json::json!("next page"),
            &base_url,
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://scrappa.co/api/youtube/channel-videos?channel_id=UC+example&sort=popular&continuation=next+page"
        );

        let url = build_channel_shorts_url(
            &serde_json::json!({"sort":"  "}),
            "UC1",
            &Value::String(String::new()),
            &base_url,
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://scrappa.co/api/youtube/channel-videos?channel_id=UC1"
        );
    }

    #[tokio::test]
    async fn stops_scanning_after_ten_pages() {
        let base_url = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let mut calls = 0;
        let result =
            collect_filtered_channel_videos(&serde_json::json!({}), "UC1", &base_url, |_url| {
                calls += 1;
                future::ready(Ok(serde_json::json!({
                    "videos":[{"id":"regular"}],
                    "continuation":format!("page-{calls}")
                })))
            })
            .await
            .unwrap();
        assert_eq!(calls, MAX_FILTER_SCAN_PAGES);
        assert!(result.videos.is_empty());
        assert_eq!(result.continuation.as_str(), Some("page-10"));
    }

    #[tokio::test]
    async fn collects_only_short_rows_and_keeps_page_order() {
        let base_url = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let mut pages = vec![
            serde_json::json!({
                "videos":[
                    {"id":"regular", "type":"video"},
                    {"id":"short-1", "type":"short"},
                    {"id":"short-2", "isShort":true},
                    {"id":"conflict", "type":"video", "videoType":"short"}
                ],
                "pagination":{"continuationToken":"page-2"}
            }),
            serde_json::json!({
                "videos":[{"id":"short-3", "videoType":"SHORT"}],
                "continuation":""
            }),
        ];
        let mut continuations = Vec::new();
        let result =
            collect_filtered_channel_videos(&serde_json::json!({}), "UC1", &base_url, |url| {
                continuations.push(
                    url.query_pairs().find_map(|(key, value)| {
                        (key == "continuation").then(|| value.into_owned())
                    }),
                );
                future::ready(Ok(pages.remove(0)))
            })
            .await
            .unwrap();

        assert_eq!(continuations, vec![None, Some("page-2".to_owned())]);
        assert_eq!(
            result.videos,
            vec![
                serde_json::json!({"id":"short-1", "type":"short"}),
                serde_json::json!({"id":"short-2", "isShort":true}),
                serde_json::json!({"id":"short-3", "videoType":"SHORT"})
            ]
        );
    }

    #[tokio::test]
    async fn writes_only_shorts_to_dataset_with_scrappa_key_and_apify_bearer() {
        let server = MockServer::start(vec![
            response(200, r#"{"ids":"UC1","id":"UC1","sort":"newest"}"#),
            response(
                200,
                r#"{"videos":[{"id":"regular","type":"video"},{"id":"short-1","type":"short"},{"id":"short-2","isShort":true},{"id":"conflict","type":"video","videoType":"short"}],"continuation":"next page"}"#,
            ),
            response(
                200,
                r#"{"videos":[{"id":"short-3","videoType":"SHORT"}],"continuation":null}"#,
            ),
            response(201, "{}"),
        ]);
        let config = config(server.base_url.clone());

        run_actor(&Client::new(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests[0].starts_with("GET /v2/key-value-stores/test-store/records/INPUT "));
        assert_eq!(
            header_value(&requests[0], "authorization"),
            Some("Bearer test-token-not-a-real-credential")
        );
        assert!(requests[1]
            .starts_with("GET /youtube/channel-videos?channel_id=UC1&sort=newest HTTP/1.1"));
        assert_eq!(
            header_value(&requests[1], "x-api-key"),
            Some("test-scrappa-key")
        );
        assert_eq!(
            header_value(&requests[1], "accept"),
            Some("application/json")
        );
        assert!(header_value(&requests[1], "authorization").is_none());
        assert!(requests[2].starts_with(
            "GET /youtube/channel-videos?channel_id=UC1&sort=newest&continuation=next+page HTTP/1.1"
        ));
        assert!(requests[3].starts_with("POST /v2/datasets/test-dataset/items "));
        assert_eq!(
            header_value(&requests[3], "authorization"),
            Some("Bearer test-token-not-a-real-credential")
        );
        assert_eq!(
            request_body(&requests[3]),
            serde_json::json!([
                {"id":"short-1","type":"short"},
                {"id":"short-2","isShort":true},
                {"id":"short-3","videoType":"SHORT"}
            ])
        );
    }

    #[tokio::test]
    async fn reports_scrappa_http_errors() {
        let server = MockServer::start(vec![
            response(200, r#"{"id":"UC1"}"#),
            response(401, "not authorized"),
        ]);
        let config = config(server.base_url.clone());

        let error = run_actor(&Client::new(), &config).await.unwrap_err();

        assert!(error
            .to_string()
            .contains("Scrappa API request failed with 401 Unauthorized"));
        assert!(error.to_string().contains("not authorized"));
        assert_eq!(server.requests().len(), 2);
    }
}
