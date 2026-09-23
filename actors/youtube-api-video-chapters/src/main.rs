use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::{collections::HashSet, env, time::Duration};
use tokio::time::timeout;
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/youtube/chapters";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

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
        let scrappa_api_key = get_scrappa_api_key(env::var("SCRAPPA_API_KEY").ok())?;
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

fn get_scrappa_api_key(value: Option<String>) -> Result<String> {
    value
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."))
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
        .extend(segments.iter().copied());
    Ok(url)
}

fn parse_ids(value: &Value, ids: &mut Vec<String>) {
    match value {
        Value::Array(values) => values.iter().for_each(|value| parse_ids(value, ids)),
        Value::String(value) => ids.extend(
            value
                .split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_owned),
        ),
        _ => {}
    }
}

fn get_video_ids(input: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    parse_ids(input.get("ids").unwrap_or(&Value::Null), &mut ids);
    parse_ids(input.get("id").unwrap_or(&Value::Null), &mut ids);

    let mut seen = HashSet::with_capacity(ids.len());
    ids.retain(|id| seen.insert(id.clone()));
    ids
}

fn build_video_chapters_url(id: &str, api_base_url: &Url) -> Result<Url> {
    if id.is_empty() {
        bail!("Video \"id\" not provided in input.");
    }

    let mut url = api_base_url.clone();
    url.query_pairs_mut().append_pair("video_id", id);
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

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() || error.to_string().contains("aborted") {
        anyhow!(
            "Scrappa API request timed out after {}s",
            REQUEST_TIMEOUT.as_secs()
        )
    } else {
        anyhow::Error::new(error)
    }
}

async fn fetch_chapters(client: &Client, config: &ActorConfig, id: &str) -> Result<Value> {
    let url = build_video_chapters_url(id, &config.scrappa_api_base_url)?;
    println!("Fetching from: {url}");

    let request = async {
        let response = client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header("X-API-Key", config.scrappa_api_key.as_str())
            .send()
            .await
            .map_err(scrappa_request_error)?;
        let status = response.status();
        if !status.is_success() {
            bail!("Request failed with status code {}", status.as_u16());
        }
        let body = response.text().await.map_err(scrappa_request_error)?;
        Ok(serde_json::from_str(&body).unwrap_or(Value::String(body)))
    };

    timeout(REQUEST_TIMEOUT, request).await.map_err(|_| {
        anyhow!(
            "Scrappa API request timed out after {}s",
            REQUEST_TIMEOUT.as_secs()
        )
    })?
}

fn dataset_row(id: &str, result: Result<Value>) -> (Value, bool) {
    match result {
        Ok(data) => (data, true),
        Err(error) => (
            json!({ "id": id, "error": error.to_string(), "success": false }),
            false,
        ),
    }
}

async fn push_dataset_row(client: &Client, config: &ActorConfig, row: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(row)
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
    let ids = get_video_ids(&input);
    if ids.is_empty() {
        bail!("At least one YouTube video ID must be provided in \"ids\" or \"id\".");
    }

    let mut success_count = 0;
    let mut failure_count = 0;
    for id in &ids {
        let attempt = async {
            let data = fetch_chapters(client, config, id).await?;
            push_dataset_row(client, config, &data).await
        }
        .await;
        if let Err(error) = attempt {
            failure_count += 1;
            let (row, _) = dataset_row(id, Err(error));
            eprintln!(
                "Failed to fetch YouTube video chapters for id {id}: {}",
                row["error"].as_str().unwrap_or_default()
            );
            push_dataset_row(client, config, &row).await?;
        } else {
            success_count += 1;
        }
    }

    if success_count == 0 {
        bail!("Failed to fetch chapters for all {failure_count} video(s).");
    }

    println!("Successfully fetched chapters for {success_count} video(s); {failure_count} failed.");
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube video chapters: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::builder().build()?;
    run_actor(&client, &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    fn test_config(server_url: Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: server_url.clone(),
            scrappa_api_base_url: server_url.join("/api/youtube/chapters").unwrap(),
            default_key_value_store_id: "store-id".to_owned(),
            default_dataset_id: "dataset-id".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
        }
    }

    fn mock_server(
        responses: Vec<(&'static str, &'static str)>,
    ) -> (Url, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                requests.push(read_request(&mut stream));
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
            requests
        });
        (Url::parse(&format!("http://{address}")).unwrap(), server)
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut reader = BufReader::new(stream);
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            headers.push_str(&line);
        }
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        format!("{headers}\r\n{}", String::from_utf8(body).unwrap())
    }

    #[test]
    fn preserves_input_schema_prefills() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["ids"]["prefill"],
            "dQw4w9WgXcQ,aqz-KE-bpKQ"
        );
        assert_eq!(schema["properties"]["id"]["prefill"], "dQw4w9WgXcQ");
    }

    #[test]
    fn parses_and_deduplicates_singular_and_batch_ids() {
        assert_eq!(
            get_video_ids(&json!({ "ids": ["vid1, vid2", ["vid3"]], "id": "vid2,vid4" })),
            vec![
                "vid1".to_owned(),
                "vid2".to_owned(),
                "vid3".to_owned(),
                "vid4".to_owned()
            ]
        );
        assert!(get_video_ids(&json!({ "ids": 7, "id": null })).is_empty());
    }

    #[test]
    fn builds_encoded_url_from_the_original_chapters_endpoint() {
        let base_url = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        assert_eq!(
            build_video_chapters_url("video id", &base_url)
                .unwrap()
                .as_str(),
            "https://scrappa.co/api/youtube/chapters?video_id=video+id"
        );
        assert_eq!(
            get_scrappa_api_key(Some("secret-key".to_owned())).unwrap(),
            "secret-key"
        );
        assert!(get_scrappa_api_key(None)
            .unwrap_err()
            .to_string()
            .contains("SCRAPPA_API_KEY"));
    }

    #[test]
    fn preserves_success_and_error_dataset_rows() {
        let chapter = json!({ "id": "video-1", "chapters": [{ "title": "Start", "time": 0 }] });
        assert_eq!(dataset_row("video-1", Ok(chapter.clone())), (chapter, true));
        assert_eq!(
            dataset_row(
                "video-2",
                Err(anyhow!("Request failed with status code 503"))
            ),
            (
                json!({ "id": "video-2", "error": "Request failed with status code 503", "success": false }),
                false
            )
        );
    }

    #[tokio::test]
    async fn fetches_chapters_and_writes_each_row_in_input_order() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", r#"{"ids":"vid 1,vid2"}"#),
            ("200 OK", r#"{"chapters":[{"title":"Start","time":0}]}"#),
            ("201 Created", ""),
            ("503 Service Unavailable", "upstream unavailable"),
            ("201 Created", ""),
        ]);
        let client = Client::new();
        run_actor(&client, &test_config(base_url)).await.unwrap();
        let requests = server.join().unwrap();

        let input_request = requests[0].to_ascii_lowercase();
        assert!(
            input_request.starts_with("get /v2/key-value-stores/store-id/records/input http/1.1")
        );
        assert!(input_request.contains("authorization: bearer test-token"));

        let first_scrappa_request = requests[1].to_ascii_lowercase();
        assert!(
            first_scrappa_request.starts_with("get /api/youtube/chapters?video_id=vid+1 http/1.1")
        );
        assert!(first_scrappa_request.contains("x-api-key: scrappa-test-key"));
        assert!(first_scrappa_request.contains("accept: application/json"));
        assert!(!first_scrappa_request.contains("authorization:"));

        let first_dataset_request = &requests[2];
        assert!(first_dataset_request
            .to_ascii_lowercase()
            .starts_with("post /v2/datasets/dataset-id/items http/1.1"));
        assert!(first_dataset_request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token"));
        assert_eq!(
            serde_json::from_str::<Value>(first_dataset_request.split_once("\r\n\r\n").unwrap().1)
                .unwrap(),
            json!({ "chapters": [{ "title": "Start", "time": 0 }] })
        );

        let second_scrappa_request = requests[3].to_ascii_lowercase();
        assert!(
            second_scrappa_request.starts_with("get /api/youtube/chapters?video_id=vid2 http/1.1")
        );
        let second_dataset_request = &requests[4];
        assert_eq!(
            serde_json::from_str::<Value>(second_dataset_request.split_once("\r\n\r\n").unwrap().1)
                .unwrap(),
            json!({ "id": "vid2", "error": "Request failed with status code 503", "success": false })
        );
    }

    #[tokio::test]
    async fn dataset_write_failure_becomes_a_row_and_next_video_continues() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", r#"{"ids":"vid1,vid2"}"#),
            ("200 OK", r#"{"chapters":[{"title":"First"}]}"#),
            ("503 Service Unavailable", "dataset unavailable"),
            ("201 Created", ""),
            ("200 OK", r#"{"chapters":[{"title":"Second"}]}"#),
            ("201 Created", ""),
        ]);
        run_actor(&Client::new(), &test_config(base_url))
            .await
            .unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 6);
        let failure: Value =
            serde_json::from_str(requests[3].split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(failure["id"], "vid1");
        assert_eq!(failure["success"], false);
        let success: Value =
            serde_json::from_str(requests[5].split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(success["chapters"][0]["title"], "Second");
    }

    #[tokio::test]
    async fn writes_failure_rows_then_fails_when_all_requests_fail() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", r#"{"id":"vid1"}"#),
            ("503 Service Unavailable", "upstream unavailable"),
            ("201 Created", ""),
        ]);
        let client = Client::new();
        let error = run_actor(&client, &test_config(base_url))
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Failed to fetch chapters for all 1 video(s)."
        );
        let requests = server.join().unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(requests[2].split_once("\r\n\r\n").unwrap().1).unwrap(),
            json!({ "id": "vid1", "error": "Request failed with status code 503", "success": false })
        );
    }
}
