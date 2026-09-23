use anyhow::{anyhow, bail, Context, Result};
use rand::random;
use reqwest::{header::RETRY_AFTER, Client, Response, StatusCode};
use serde_json::{Map, Value};
use std::{
    env, process,
    time::{Duration, SystemTime},
};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/youtube/transcript";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_ATTEMPTS: u8 = 4;
const RETRY_BASE_DELAY_MS: u64 = 1_000;
const RETRY_MAX_DELAY_MS: u64 = 10_000;
const RETRY_JITTER_RATIO: f64 = 0.2;
const MAX_TIMER_DELAY_MS: u64 = 2_147_483_647;
const RETRYABLE_STATUS_CODES: [u16; 7] = [408, 429, 500, 502, 503, 504, 522];

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
            .filter(|key| !key.is_empty())
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
    let mut path = url
        .path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?;
    path.pop_if_empty();
    path.extend(segments.iter().copied());
    drop(path);
    Ok(url)
}

fn single_value(value: Option<&Value>) -> Option<&str> {
    let value = match value? {
        Value::Array(values) => values.first()?,
        value => value,
    };
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn bool_value(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Array(values)) => bool_value(values.first()),
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" => true,
            _ => false,
        },
        _ => false,
    }
}

fn transcript_url(input: &Value, api_base_url: &Url) -> Result<Url> {
    let id = single_value(input.get("id"))
        .ok_or_else(|| anyhow!("YouTube video ID \"id\" is required."))?;
    let mut url = api_base_url.clone();
    if url.path().is_empty() || url.path() == "/" {
        url.set_path("/api/youtube/transcript");
    }
    url.set_query(None);
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("video_id", id);
        if let Some(language) = single_value(input.get("language")) {
            query.append_pair("language", language);
        }
        if let Some(language) = single_value(input.get("lang")) {
            query.append_pair("lang", language);
        }
        if let Some(language) = single_value(input.get("hl")) {
            query.append_pair("hl", &language.to_lowercase());
        }
        if let Some(country) = single_value(input.get("gl")) {
            query.append_pair("gl", &country.to_uppercase());
        }
        if bool_value(input.get("debug")) {
            query.append_pair("debug", "1");
        }
    }
    Ok(url)
}

async fn checked_response(response: Response, operation: &str) -> Result<Response> {
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
    Ok(response)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    checked_response(response, operation)
        .await?
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
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(Value::Null);
    }
    response_json(response, "Apify INPUT request").await
}

fn retryable_status(status: StatusCode) -> bool {
    RETRYABLE_STATUS_CODES.contains(&status.as_u16())
}

fn retry_delay_ms_with_random(attempt: u8, random_value: f64) -> u64 {
    let exponent = u32::from(attempt.saturating_sub(1));
    let base_delay = RETRY_BASE_DELAY_MS
        .saturating_mul(2_u64.saturating_pow(exponent))
        .min(RETRY_MAX_DELAY_MS);
    let jitter = (base_delay as f64 * RETRY_JITTER_RATIO * random_value) as u64;
    base_delay
        .saturating_add(jitter)
        .min(RETRY_MAX_DELAY_MS)
        .min(MAX_TIMER_DELAY_MS)
}

fn retry_delay_ms(attempt: u8) -> u64 {
    retry_delay_ms_with_random(attempt, random::<f64>())
}

fn retry_after_delay_ms(value: &str, now: SystemTime) -> Option<u64> {
    let value = value.trim();
    let seconds = if value.is_empty() {
        Some(0.0)
    } else {
        value
            .parse::<f64>()
            .ok()
            .filter(|seconds| seconds.is_finite())
    };
    if let Some(seconds) = seconds.filter(|seconds| *seconds >= 0.0) {
        return Some((seconds * 1_000.0).min(MAX_TIMER_DELAY_MS as f64) as u64);
    }

    let retry_at = httpdate::parse_http_date(value).ok()?;
    let delay = retry_at
        .duration_since(now)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(MAX_TIMER_DELAY_MS));
    Some(delay as u64)
}

fn retry_after_from_response(response: &Response) -> Option<u64> {
    let value = response.headers().get(RETRY_AFTER)?.to_str().ok()?;
    retry_after_delay_ms(value, SystemTime::now())
}

async fn fetch_transcript(client: &Client, config: &ActorConfig, input: &Value) -> Result<Value> {
    let url = transcript_url(input, &config.scrappa_api_base_url)?;
    for attempt in 1..=MAX_ATTEMPTS {
        println!("Fetching from: {url}");
        let response = client
            .get(url.clone())
            .header("X-API-Key", config.scrappa_api_key.as_str())
            .header("Accept", "application/json")
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await;

        let response = match response {
            Ok(response) => response,
            Err(error) => {
                let message = if error.is_timeout() {
                    format!("Scrappa API request failed before receiving a response: operation aborted: {error}")
                } else {
                    format!("Scrappa API request failed before receiving a response: {error}")
                };
                if attempt == MAX_ATTEMPTS {
                    bail!(message);
                }
                tokio::time::sleep(Duration::from_millis(retry_delay_ms(attempt))).await;
                continue;
            }
        };

        let status = response.status();
        if status.is_success() {
            return match response.json().await {
                Ok(data) => Ok(data),
                Err(error) if error.is_timeout() => Err(anyhow!(
                    "Scrappa API returned an invalid JSON response for {url}: operation aborted"
                )),
                Err(error) => Err(anyhow!(
                    "Scrappa API returned an invalid JSON response for {url}: {error}"
                )),
            };
        }
        let reason = status.canonical_reason().unwrap_or("<none>");
        let message = format!(
            "Scrappa API request failed with {} {reason}",
            status.as_u16()
        );
        if !retryable_status(status) || attempt == MAX_ATTEMPTS {
            bail!(message);
        }

        let delay = retry_after_from_response(&response)
            .map(|header_delay| retry_delay_ms(attempt).max(header_delay))
            .unwrap_or_else(|| retry_delay_ms(attempt));
        drop(response);
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }

    bail!("Scrappa API request failed without a response")
}

fn spread_response(data: &Value) -> Map<String, Value> {
    match data {
        Value::Object(fields) => fields.clone(),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        Value::String(value) => value
            .chars()
            .enumerate()
            .map(|(index, character)| (index.to_string(), Value::String(character.to_string())))
            .collect(),
        _ => Map::new(),
    }
}

fn dataset_row(data: &Value, input: &Value) -> Value {
    let transcript = data
        .get("transcript")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let video_id = data
        .get("videoId")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("id").filter(|value| !value.is_null()))
        .or_else(|| input.get("id").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null);
    let mut row = spread_response(data);
    row.insert("videoId".to_owned(), video_id);
    row.insert("transcript".to_owned(), Value::Array(transcript.clone()));
    row.insert("segmentCount".to_owned(), Value::from(transcript.len()));
    Value::Object(row)
}

async fn push_dataset_row(client: &Client, config: &ActorConfig, row: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(&[row])
        .send()
        .await
        .context("Apify dataset write failed")?;
    checked_response(response, "Apify dataset write").await?;
    Ok(())
}

async fn set_output(client: &Client, config: &ActorConfig, data: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            "OUTPUT",
        ],
    )?;
    let response = client
        .put(url)
        .bearer_auth(&config.apify_token)
        .json(data)
        .send()
        .await
        .context("Apify OUTPUT write failed")?;
    checked_response(response, "Apify OUTPUT write").await?;
    Ok(())
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let input = if input.is_null() {
        Value::Object(Map::new())
    } else {
        input
    };
    let data = fetch_transcript(client, config, &input).await?;
    let row = dataset_row(&data, &input);
    push_dataset_row(client, config, &row).await?;
    set_output(client, config, &data).await?;

    let transcript_count = row["segmentCount"].as_u64().unwrap_or_default();
    let video_id = input
        .get("id")
        .map(js_string)
        .unwrap_or_else(|| "undefined".to_owned());
    println!(
        "Successfully fetched transcript with {transcript_count} segment(s) for video id: {video_id}"
    );
    Ok(())
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

fn actor_error_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if message.contains("aborted") {
        format!(
            "Scrappa API request timed out after {}s",
            REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let message = actor_error_message(&error);
        eprintln!("Failed to fetch YouTube transcript: {message}");
        process::exit(1);
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
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
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
        requests: Arc<Mutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let recorded_requests = Arc::clone(&requests);
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                for response in responses {
                    let connection = loop {
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
                    let (mut stream, _) = connection;
                    let request = read_request(&mut stream).unwrap_or_default();
                    recorded_requests
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .push(request);
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
                        404 => "Not Found",
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
            self.requests
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .clone()
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

    fn response(body: &str) -> MockResponse {
        MockResponse {
            status: 200,
            body: body.to_owned(),
        }
    }

    fn config(base_url: &Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: base_url.clone(),
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            input_key: "CUSTOM INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .skip(1)
            .find_map(|line| {
                let (header_name, value) = line.split_once(':')?;
                header_name.eq_ignore_ascii_case(name).then(|| value.trim())
            })
    }

    fn body(request: &str) -> &str {
        request.split_once("\r\n\r\n").map_or("", |(_, body)| body)
    }

    #[test]
    fn transcript_url_preserves_params_and_input_normalization() {
        let base_url = Url::parse("https://scrappa.co").unwrap();
        let url = transcript_url(
            &json!({
                "id": [" dQw4w9WgXcQ "],
                "language": ["es"],
                "lang": "fr",
                "hl": "EN",
                "gl": "us",
                "debug": "yes"
            }),
            &base_url,
        )
        .unwrap();

        assert_eq!(url.path(), "/api/youtube/transcript");
        assert_eq!(
            url.query(),
            Some("video_id=dQw4w9WgXcQ&language=es&lang=fr&hl=en&gl=US&debug=1")
        );
        assert!(transcript_url(&json!({"id": "  "}), &base_url).is_err());
    }

    #[test]
    fn dataset_normalization_keeps_raw_fields_and_uses_transcript_length() {
        let input = json!({"id": "input-id"});
        let data = json!({
            "id": "response-id",
            "transcript": [{"text": "one"}, {"text": "two"}],
            "segmentCount": 99,
            "custom": {"preserved": true}
        });
        let row = dataset_row(&data, &input);

        assert_eq!(row["videoId"], "response-id");
        assert_eq!(row["segmentCount"], 2);
        assert_eq!(row["transcript"], data["transcript"]);
        assert_eq!(row["custom"], data["custom"]);
        assert_eq!(
            dataset_row(&json!({"transcript": null}), &input)["transcript"],
            json!([])
        );
    }

    #[test]
    fn retry_policy_matches_transient_statuses_and_retry_after_delays() {
        for status in [408, 429, 500, 502, 503, 504, 522] {
            assert!(retryable_status(StatusCode::from_u16(status).unwrap()));
        }
        for status in [400, 401, 422, 501, 505] {
            assert!(!retryable_status(StatusCode::from_u16(status).unwrap()));
        }
        assert_eq!(retry_delay_ms_with_random(1, 0.5), 1_100);
        assert_eq!(retry_delay_ms_with_random(5, 0.5), 10_000);
        assert_eq!(retry_after_delay_ms("3", SystemTime::now()), Some(3_000));
        assert_eq!(
            retry_after_delay_ms("Thu, 01 Jan 1970 00:00:03 GMT", SystemTime::UNIX_EPOCH),
            Some(3_000)
        );
        assert_eq!(
            retry_after_delay_ms("9999999999", SystemTime::now()),
            Some(MAX_TIMER_DELAY_MS)
        );
        assert_eq!(retry_after_delay_ms("not a date", SystemTime::now()), None);
    }

    #[tokio::test]
    async fn reads_input_and_writes_one_normalized_row_and_raw_output() {
        let data = json!({
            "videoId": "response-id",
            "transcript": [{"text": "first", "startMs": 0}, {"text": "second", "startMs": 250}],
            "segmentCount": 50,
            "language": "en",
            "rawMetadata": {"keep": true}
        });
        let server = MockServer::start(vec![
            response(
                r#"{"id":"dQw4w9WgXcQ","language":["es"],"lang":"fr","hl":"EN","gl":"us","debug":true}"#,
            ),
            response(&data.to_string()),
            response("{}"),
            response(&data.to_string()),
        ]);
        let config = config(&server.base_url);
        let client = Client::builder().build().unwrap();

        run_actor(&client, &config).await.unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(
            requests[0].starts_with("GET /v2/key-value-stores/test-store/records/CUSTOM%20INPUT ")
        );
        assert!(requests[1].starts_with("GET /api/youtube/transcript?video_id=dQw4w9WgXcQ&language=es&lang=fr&hl=en&gl=US&debug=1 "));
        assert_eq!(
            header_value(&requests[0], "authorization"),
            Some("Bearer test-token-not-a-real-credential")
        );
        assert_eq!(
            header_value(&requests[2], "authorization"),
            Some("Bearer test-token-not-a-real-credential")
        );
        assert_eq!(
            header_value(&requests[3], "authorization"),
            Some("Bearer test-token-not-a-real-credential")
        );
        assert!(requests[3].starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT "));
        assert_eq!(
            header_value(&requests[1], "x-api-key"),
            Some("test-scrappa-key")
        );
        assert_eq!(
            header_value(&requests[1], "accept"),
            Some("application/json")
        );

        let rows: Value = serde_json::from_str(body(&requests[2])).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 1);
        assert_eq!(rows[0]["videoId"], "response-id");
        assert_eq!(rows[0]["segmentCount"], 2);
        assert_eq!(rows[0]["transcript"], data["transcript"]);
        assert_eq!(rows[0]["rawMetadata"], data["rawMetadata"]);
        assert_eq!(
            serde_json::from_str::<Value>(body(&requests[3])).unwrap(),
            data
        );
    }

    #[test]
    fn timeout_errors_keep_the_actor_failure_message() {
        let error =
            anyhow!("Scrappa API request failed before receiving a response: operation aborted");
        assert_eq!(
            actor_error_message(&error),
            "Scrappa API request timed out after 60s"
        );
    }
}
