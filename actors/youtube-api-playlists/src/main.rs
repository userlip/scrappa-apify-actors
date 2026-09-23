use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/youtube/search";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_LIMIT: u64 = 20;

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
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|api_key| !api_key.is_empty())
            .ok_or_else(|| anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."))?;
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

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

fn build_playlist_search_url(input: &Value, api_base_url: &Url) -> Result<Url> {
    let query = input
        .get("q")
        .and_then(Value::as_str)
        .filter(|query| !query.is_empty())
        .ok_or_else(|| anyhow!("Search query \"q\" not provided. Please provide a value for \"searchPlaylistQuery\" in the input."))?;

    let mut url = api_base_url.clone();
    let mut params = url.query_pairs_mut();
    params.append_pair("query", query);
    params.append_pair("type", "playlist");

    let sort = match input.get("sort") {
        None => Some("relevance"),
        Some(Value::String(sort)) if !sort.trim().is_empty() => Some(sort.as_str()),
        _ => None,
    };
    if let Some(sort) = sort {
        let order = match sort {
            "upload_date" => "date",
            "view_count" => "viewCount",
            value => value,
        };
        params.append_pair("order", order);
    }

    let limit = match input.get("limit") {
        None => Some(MAX_LIMIT),
        Some(Value::Number(number)) => positive_limit(number),
        _ => None,
    };
    if let Some(limit) = limit {
        params.append_pair("limit", &limit.to_string());
    }

    if let Some(continuation) = input
        .get("continuation")
        .and_then(Value::as_str)
        .filter(|continuation| !continuation.trim().is_empty())
    {
        params.append_pair("continuation", continuation);
    }
    drop(params);
    Ok(url)
}

fn positive_limit(number: &serde_json::Number) -> Option<u64> {
    if let Some(limit) = number.as_u64() {
        return (limit > 0).then_some(limit.min(MAX_LIMIT));
    }
    let limit = number.as_f64()?;
    (limit.is_finite() && limit > 0.0 && limit.fract() == 0.0)
        .then_some(limit.min(MAX_LIMIT as f64) as u64)
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
        .header("X-API-Key", &config.scrappa_api_key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| {
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
    response
        .json()
        .await
        .map_err(|error| anyhow!("Scrappa API response was not valid JSON: {error}"))
}

fn response_results(data: &Value) -> Result<Value> {
    match data.get("results") {
        Some(Value::Array(results)) => Ok(Value::Array(results.clone())),
        Some(Value::Object(results)) => Ok(Value::Object(results.clone())),
        _ => bail!("Scrappa API response is missing a results array or object"),
    }
}

fn result_count(results: &Value) -> usize {
    results.as_array().map_or(1, Vec::len)
}

fn continuation_token(data: &Value) -> Option<&str> {
    let token = data
        .get("continuation")
        .filter(|token| !token.is_null())
        .or_else(|| data.pointer("/pagination/continuationToken"))?;
    token.as_str().filter(|token| !token.is_empty())
}

async fn push_dataset_items(client: &Client, config: &ActorConfig, items: &Value) -> Result<()> {
    if items.as_array().is_some_and(Vec::is_empty) {
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
    let query = input.get("q").and_then(Value::as_str).unwrap_or_default();
    let url = build_playlist_search_url(&input, &config.scrappa_api_base_url)?;
    println!("Fetching playlists for query: {query}");

    let data = fetch_playlists(client, config, &url).await?;
    let results = response_results(&data)?;
    push_dataset_items(client, config, &results).await?;
    println!(
        "Successfully fetched {} playlists for query: {query}",
        result_count(&results)
    );

    if let Some(continuation) = continuation_token(&data) {
        println!("Continuation token available for next page: {continuation}");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch playlists: {error:#}");
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
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver},
            Arc,
        },
        thread::{self, JoinHandle},
        time::Duration,
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
                    let (mut stream, _) = loop {
                        if stopped.load(Ordering::Relaxed) {
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

    fn test_config(base_url: &Url) -> ActorConfig {
        let mut scrappa_api_base_url = base_url.clone();
        scrappa_api_base_url.set_path("/api/youtube/search");
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url,
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    fn has_header(request: &str, name: &str, value: &str) -> bool {
        request.lines().any(|line| {
            line.split_once(':').is_some_and(|(header, actual)| {
                header.eq_ignore_ascii_case(name) && actual.trim().eq_ignore_ascii_case(value)
            })
        })
    }

    #[test]
    fn maps_playlist_type_sort_aliases_limit_and_continuation() {
        let base_url = Url::parse("https://scrappa.co/api/youtube/search").unwrap();
        let input = serde_json::json!({
            "q": "music & mixes",
            "sort": "view_count",
            "limit": 50,
            "continuation": "next page/+"
        });
        let url = build_playlist_search_url(&input, &base_url).unwrap();
        let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(
            params.get("query").map(String::as_str),
            Some("music & mixes")
        );
        assert_eq!(params.get("type").map(String::as_str), Some("playlist"));
        assert_eq!(params.get("order").map(String::as_str), Some("viewCount"));
        assert_eq!(params.get("limit").map(String::as_str), Some("20"));
        assert_eq!(
            params.get("continuation").map(String::as_str),
            Some("next page/+")
        );

        let input = serde_json::json!({"q": "news", "sort": "upload_date", "limit": 3});
        let url = build_playlist_search_url(&input, &base_url).unwrap();
        let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(params.get("order").map(String::as_str), Some("date"));
        assert_eq!(params.get("limit").map(String::as_str), Some("3"));

        let input = serde_json::json!({"q": "news", "sort": "rating"});
        let url = build_playlist_search_url(&input, &base_url).unwrap();
        let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(params.get("order").map(String::as_str), Some("rating"));
    }

    #[test]
    fn defaults_sort_and_limit_but_omits_explicit_empty_filters() {
        let base_url = Url::parse("https://scrappa.co/api/youtube/search").unwrap();
        let url = build_playlist_search_url(&serde_json::json!({"q": "news"}), &base_url).unwrap();
        let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(params.get("order").map(String::as_str), Some("relevance"));
        assert_eq!(params.get("limit").map(String::as_str), Some("20"));

        let input =
            serde_json::json!({"q": "news", "sort": " ", "limit": 2.5, "continuation": " "});
        let url = build_playlist_search_url(&input, &base_url).unwrap();
        let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert!(!params.contains_key("order"));
        assert!(!params.contains_key("limit"));
        assert!(!params.contains_key("continuation"));
    }

    #[test]
    fn rejects_missing_playlist_query_with_legacy_message() {
        let base_url = Url::parse("https://scrappa.co/api/youtube/search").unwrap();
        let error = build_playlist_search_url(&serde_json::json!({}), &base_url).unwrap_err();
        assert_eq!(error.to_string(), "Search query \"q\" not provided. Please provide a value for \"searchPlaylistQuery\" in the input.");
    }

    #[test]
    fn preserves_results_shape_and_continuation_response_variants() {
        let rows = serde_json::json!([{"id": "first"}, {"id": "second"}]);
        assert_eq!(
            response_results(&serde_json::json!({"results": rows})).unwrap(),
            rows
        );
        let one = serde_json::json!({"id": "single"});
        assert_eq!(
            response_results(&serde_json::json!({"results": one})).unwrap(),
            one
        );
        assert!(response_results(&serde_json::json!({"results": null})).is_err());
        assert_eq!(result_count(&serde_json::json!([1, 2])), 2);
        assert_eq!(result_count(&serde_json::json!({"id": "one"})), 1);
        assert_eq!(
            continuation_token(&serde_json::json!({"pagination": {"continuationToken": "next"}})),
            Some("next")
        );
        assert_eq!(
            continuation_token(&serde_json::json!({"continuation": "legacy"})),
            Some("legacy")
        );
    }

    #[test]
    fn schema_keeps_playlist_sort_prefill_and_limit_default() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema.pointer("/properties/sort/prefill"),
            Some(&Value::String("relevance".to_owned()))
        );
        assert_eq!(
            schema.pointer("/properties/limit/default"),
            Some(&Value::Number(20.into()))
        );
        assert_eq!(
            schema
                .pointer("/required")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            schema.pointer("/required/0").and_then(Value::as_str),
            Some("q")
        );
    }

    #[tokio::test]
    async fn fetches_playlist_results_and_writes_rows_to_apify_dataset() {
        let server = MockServer::start(vec![
            response(
                200,
                r#"{"q":"music & mixes","sort":"view_count","limit":50,"continuation":"next page/+"}"#,
            ),
            response(
                200,
                r#"{"results":[{"id":"one","title":"First"},{"id":"two","title":"Second"}],"pagination":{"continuationToken":"next"}}"#,
            ),
            response(200, "{}"),
        ]);
        let config = test_config(&server.base_url);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        run_actor(&client, &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(
            requests[0].starts_with("GET /v2/key-value-stores/test-store/records/INPUT HTTP/1.1")
        );
        assert!(has_header(
            &requests[0],
            "Authorization",
            "Bearer test-token-not-a-real-credential"
        ));
        let search_url = Url::parse(&format!(
            "http://localhost{}",
            requests[1].split_whitespace().nth(1).unwrap()
        ))
        .unwrap();
        let params: std::collections::HashMap<_, _> =
            search_url.query_pairs().into_owned().collect();
        assert_eq!(
            params.get("query").map(String::as_str),
            Some("music & mixes")
        );
        assert_eq!(params.get("type").map(String::as_str), Some("playlist"));
        assert_eq!(params.get("order").map(String::as_str), Some("viewCount"));
        assert_eq!(params.get("limit").map(String::as_str), Some("20"));
        assert_eq!(
            params.get("continuation").map(String::as_str),
            Some("next page/+")
        );
        assert!(has_header(&requests[1], "X-API-Key", "test-scrappa-key"));
        assert!(has_header(&requests[1], "Accept", "application/json"));
        assert!(requests[2].starts_with("POST /v2/datasets/test-dataset/items HTTP/1.1"));
        assert!(has_header(
            &requests[2],
            "Authorization",
            "Bearer test-token-not-a-real-credential"
        ));
        let body = requests[2].split_once("\r\n\r\n").unwrap().1;
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            serde_json::json!([
                {"id":"one","title":"First"},
                {"id":"two","title":"Second"}
            ])
        );
    }

    #[tokio::test]
    async fn upstream_failure_is_returned_without_dataset_write() {
        let server = MockServer::start(vec![
            response(200, r#"{"q":"music"}"#),
            response(429, r#"{"error":"rate limited"}"#),
        ]);
        let config = test_config(&server.base_url);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let error = run_actor(&client, &config).await.unwrap_err();
        assert!(error
            .to_string()
            .contains("Scrappa API request failed with 429 Too Many Requests"));
        assert_eq!(server.requests().len(), 2);
    }
}
