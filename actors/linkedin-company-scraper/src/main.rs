use std::{collections::HashSet, env, fmt, process, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use url::Url;

const INVALID_URL_MESSAGE: &str =
    "Invalid LinkedIn company URL. Expected format: https://www.linkedin.com/company/company-slug";
const SCRAPPA_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Deserialize, Default)]
#[serde(default)]
struct ActorInput {
    url: Option<Value>,
    urls: Option<Value>,
    use_cache: Option<Value>,
    maximum_cache_age: Option<Value>,
}

#[derive(Debug, PartialEq, Eq)]
struct UrlRequest {
    input_url: String,
    normalized_url: Option<String>,
    validation_error: Option<String>,
}

enum ScrappaError {
    Api { status: u16, message: String },
    Request(anyhow::Error),
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Api { status, message } => {
                write!(formatter, "Scrappa API error ({status}): {message}")
            }
            Self::Request(error) => write!(formatter, "{error}"),
        }
    }
}

struct ActorConfig {
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    scrappa_api_key: String,
    apify_base: String,
    scrappa_base: String,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig {
        apify_token: required_env("APIFY_TOKEN")?,
        key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
        dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
        actor_run_id: required_env("ACTOR_RUN_ID")?,
        input_key: env::var("ACTOR_INPUT_KEY")
            .ok()
            .filter(|key| !key.is_empty())
            .unwrap_or_else(|| "INPUT".to_owned()),
        scrappa_api_key: required_env("SCRAPPA_API_KEY")?,
        apify_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", "https://api.apify.com"),
        scrappa_base: env_or_default("SCRAPPA_API_BASE_URL", "https://scrappa.co/api"),
    };

    let apify_client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("Could not create Apify HTTP client")?;
    let scrappa_client = Client::builder()
        .timeout(SCRAPPA_TIMEOUT)
        .build()
        .context("Could not create Scrappa HTTP client")?;
    run_actor(&apify_client, &scrappa_client, &config).await
}

async fn run_actor(
    apify_client: &Client,
    scrappa_client: &Client,
    config: &ActorConfig,
) -> Result<()> {
    let input_url = apify_url(
        &config.apify_base,
        &[
            "v2",
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let input_value =
        read_json_record(apify_client, &input_url, &config.apify_token, "Apify INPUT").await?;
    let input = serde_json::from_value::<Option<ActorInput>>(input_value)
        .context("Could not parse Apify INPUT")?
        .unwrap_or_default();
    let requests = get_input_urls(&input)?;
    if requests.is_empty() {
        return Err(anyhow!(
            "At least one LinkedIn company URL is required. Provide url or urls."
        ));
    }

    let scrappa_url = endpoint_url(&config.scrappa_base, &["linkedin", "company"])?;
    let dataset_url = apify_url(
        &config.apify_base,
        &["v2", "datasets", &config.dataset_id, "items"],
    )?;
    let output_url = apify_url(
        &config.apify_base,
        &[
            "v2",
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            "OUTPUT",
        ],
    )?;

    let total = requests.len();
    let dataset_capacity = run_dataset_capacity(
        apify_client,
        &config.apify_base,
        &config.actor_run_id,
        &config.apify_token,
        total,
    )
    .await?;
    let mut saved_rows = 0;
    let mut first_result = None;
    let mut succeeded = 0;
    let mut failed = 0;

    println!(
        "Scraping {total} LinkedIn company URL{}",
        if total == 1 { "" } else { "s" }
    );

    for request in requests {
        let result = match request.normalized_url.as_deref() {
            None => {
                println!("Invalid LinkedIn company URL: \"{}\"", request.input_url);
                build_failure_item(
                    request
                        .validation_error
                        .as_deref()
                        .unwrap_or("Invalid LinkedIn company URL"),
                    "error",
                    None,
                    &request.input_url,
                    None,
                )
            }
            Some(normalized_url) => {
                println!("Scraping LinkedIn company: \"{normalized_url}\"");
                match scrape_company(
                    scrappa_client,
                    &scrappa_url,
                    &config.scrappa_api_key,
                    normalized_url,
                    &input,
                )
                .await
                {
                    Ok(response) => {
                        build_dataset_item(response, &request.input_url, normalized_url)?
                    }
                    Err(ScrappaError::Api {
                        status: 404,
                        message,
                    }) => {
                        let error = format!("Scrappa API error (404): {message}");
                        eprintln!("Company scraping returned a per-item failure for {normalized_url}: {error}");
                        build_failure_item(
                            &error,
                            "scrappa_api_error",
                            Some(404),
                            &request.input_url,
                            Some(normalized_url),
                        )
                    }
                    Err(error) => return Err(anyhow!(error.to_string())),
                }
            }
        };

        if !js_truthy(result.get("success").unwrap_or(&Value::Null))
            && result.get("status_code") != Some(&json!(404))
        {
            let message = result
                .get("message")
                .and_then(Value::as_str)
                .map(|message| format!(" ({message})"))
                .unwrap_or_default();
            eprintln!("Company scraping returned success: false{message}");
        } else if js_truthy(result.get("success").unwrap_or(&Value::Null)) {
            let name = result
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .unwrap_or("Unknown");
            println!("Successfully scraped company: {name}");
        }

        push_dataset_item_with_budget(
            apify_client,
            &dataset_url,
            &config.apify_token,
            &result,
            dataset_capacity,
            &mut saved_rows,
        )
        .await?;
        if first_result.is_none() {
            first_result = Some(result.clone());
        }
        if js_truthy(result.get("success").unwrap_or(&Value::Null)) {
            succeeded += 1;
        } else {
            failed += 1;
        }
    }

    let output = build_output(
        total,
        first_result
            .as_ref()
            .expect("non-empty URL list has a result"),
        succeeded,
        failed,
    );
    put_json_record(
        apify_client,
        &output_url,
        &config.apify_token,
        &output,
        "Apify OUTPUT",
    )
    .await?;

    println!("LinkedIn Company scrape completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string_pretty(&json!({
            "requested": total,
            "succeeded": succeeded,
            "failed": failed,
        }))?
    );
    Ok(())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{name} environment variable is not set"))
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

async fn run_dataset_capacity(
    client: &Client,
    apify_base: &str,
    actor_run_id: &str,
    token: &str,
    requested: usize,
) -> Result<usize> {
    let url = apify_url(apify_base, &["v2", "actor-runs", actor_run_id])?;
    let run = read_json_record(client, &url, token, "Apify run pricing").await?;
    affordable_dataset_items(&run, requested)
}

fn affordable_dataset_items(run: &Value, requested: usize) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        bail!("Apify run is not configured for pay-per-event pricing");
    }
    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = events
        .get("apify-default-dataset-item")
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    let max_charge = data
        .pointer("/options/maxTotalChargeUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
    if !item_price.is_finite() || item_price < 0.0 || !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let mut spent = 0.0;
    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    for (event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count"))?;
        if count == 0 {
            continue;
        }
        let price = events
            .get(event_name)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .count())
}

fn get_input_urls(input: &ActorInput) -> Result<Vec<UrlRequest>> {
    let mut raw_urls = Vec::new();
    if let Some(url) = input.url.as_ref().and_then(Value::as_str) {
        raw_urls.push(url);
    }
    if let Some(Value::Array(urls)) = &input.urls {
        for url in urls {
            raw_urls.push(
                url.as_str()
                    .ok_or_else(|| anyhow!("rawUrl.trim is not a function"))?,
            );
        }
    }

    let mut seen = HashSet::new();
    let mut requests = Vec::new();
    for raw_url in raw_urls {
        let input_url = raw_url.trim().to_owned();
        if input_url.is_empty() {
            continue;
        }

        match normalize_linkedin_company_url(&input_url) {
            Ok(normalized_url) => {
                if seen.insert(normalized_url.clone()) {
                    requests.push(UrlRequest {
                        input_url,
                        normalized_url: Some(normalized_url),
                        validation_error: None,
                    });
                }
            }
            Err(message) => {
                if seen.insert(format!("invalid:{input_url}")) {
                    requests.push(UrlRequest {
                        input_url,
                        normalized_url: None,
                        validation_error: Some(message),
                    });
                }
            }
        }
    }
    Ok(requests)
}

fn normalize_linkedin_company_url(raw_url: &str) -> std::result::Result<String, String> {
    let candidate = raw_url.trim();
    let has_scheme = candidate.find("://").is_some_and(|index| {
        index > 0
            && candidate[..index]
                .bytes()
                .all(|byte| byte.is_ascii_alphabetic())
    });
    let with_protocol = if has_scheme {
        candidate.to_owned()
    } else {
        format!("https://{candidate}")
    };
    let parsed = Url::parse(&with_protocol).map_err(|_| "Invalid URL".to_owned())?;

    if !matches!(parsed.scheme(), "http" | "https")
        || !parsed.username().is_empty()
        || parsed
            .password()
            .is_some_and(|password| !password.is_empty())
        || parsed.port().is_some()
    {
        return Err(INVALID_URL_MESSAGE.to_owned());
    }

    let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
    if !is_linkedin_host(&host) {
        return Err(INVALID_URL_MESSAGE.to_owned());
    }

    let path = parsed.path().trim_end_matches('/');
    let path_suffix = path
        .get(..9)
        .filter(|prefix| prefix.eq_ignore_ascii_case("/company/"))
        .and_then(|_| path.get(9..))
        .ok_or_else(|| INVALID_URL_MESSAGE.to_owned())?;
    let slug = path_suffix.split('/').next().unwrap_or_default();
    if slug.is_empty()
        || !slug
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(INVALID_URL_MESSAGE.to_owned());
    }

    Ok(format!(
        "{}://www.linkedin.com/company/{slug}",
        parsed.scheme()
    ))
}

fn is_linkedin_host(host: &str) -> bool {
    matches!(host, "linkedin.com" | "www.linkedin.com" | "m.linkedin.com")
        || host.strip_suffix(".linkedin.com").is_some_and(|subdomain| {
            (2..=3).contains(&subdomain.len())
                && subdomain.bytes().all(|byte| byte.is_ascii_alphabetic())
        })
}

fn cache_age(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(age) = value.as_u64() {
        return (age >= 1).then(|| age.to_string());
    }
    if let Some(age) = value.as_i64() {
        return (age >= 1).then(|| age.to_string());
    }
    let age = value.as_f64()?;
    (age.is_finite() && age >= 1.0 && age.fract() == 0.0).then(|| age.to_string())
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

async fn scrape_company(
    client: &Client,
    endpoint: &Url,
    api_key: &str,
    normalized_url: &str,
    input: &ActorInput,
) -> std::result::Result<Value, ScrappaError> {
    let mut params = vec![("url", normalized_url.to_owned())];
    if input.use_cache.as_ref().is_some_and(js_truthy) {
        params.push(("use_cache", "1".to_owned()));
        if let Some(age) = cache_age(input.maximum_cache_age.as_ref()) {
            params.push(("maximum_cache_age", age));
        }
    }

    let response = client
        .get(endpoint.clone())
        .header("X-API-Key", api_key)
        .header("Accept", "application/json")
        .query(&params)
        .send()
        .await
        .map_err(|error| ScrappaError::Request(error.into()))?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(ScrappaError::Api {
            status,
            message: scrappa_error_message(status, &body),
        });
    }

    response
        .json::<Value>()
        .await
        .map_err(|error| ScrappaError::Request(error.into()))
}

fn scrappa_error_message(status: u16, body: &str) -> String {
    let Ok(data) = serde_json::from_str::<Value>(body) else {
        return if body.is_empty() {
            format!("HTTP {status}")
        } else {
            body.to_owned()
        };
    };

    let mut message = data
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {status}"));
    if let Some(errors) = data.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .map(|(field, messages)| {
                let messages = messages
                    .as_array()
                    .map(|messages| {
                        messages
                            .iter()
                            .map(|message| message.as_str().unwrap_or_default())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                format!("{field}: {messages}")
            })
            .collect::<Vec<_>>()
            .join("; ");
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details);
        }
    }
    message
}

fn build_dataset_item(response: Value, input_url: &str, normalized_url: &str) -> Result<Value> {
    let mut item = response
        .as_object()
        .ok_or_else(|| anyhow!("Scrappa company response must be an object"))?
        .clone();
    if item.get("url").is_none_or(Value::is_null) {
        item.insert("url".to_owned(), json!(normalized_url));
    }
    item.insert("input_url".to_owned(), json!(input_url));
    item.insert("normalized_url".to_owned(), json!(normalized_url));
    Ok(Value::Object(item))
}

fn build_failure_item(
    error: &str,
    error_type: &str,
    status_code: Option<u16>,
    input_url: &str,
    normalized_url: Option<&str>,
) -> Value {
    let message = if status_code == Some(404) {
        "Company not found"
    } else {
        error
    };
    let mut item = Map::from_iter([
        ("success".to_owned(), json!(false)),
        ("input_url".to_owned(), json!(input_url)),
        ("error".to_owned(), json!(error)),
        ("error_type".to_owned(), json!(error_type)),
        ("message".to_owned(), json!(message)),
    ]);
    if let Some(normalized_url) = normalized_url {
        item.insert("normalized_url".to_owned(), json!(normalized_url));
        item.insert("url".to_owned(), json!(normalized_url));
    }
    if let Some(status_code) = status_code {
        item.insert("status_code".to_owned(), json!(status_code));
    }
    Value::Object(item)
}

fn build_output(total: usize, first_result: &Value, succeeded: usize, failed: usize) -> Value {
    if total == 1 {
        first_result.clone()
    } else {
        json!({
            "requested": total,
            "succeeded": succeeded,
            "failed": failed,
        })
    }
}

fn endpoint_url(base: &str, segments: &[&str]) -> Result<Url> {
    let mut url = Url::parse(base.trim_end_matches('/'))
        .with_context(|| format!("Invalid API base URL: {base}"))?;
    url.set_query(None);
    url.set_fragment(None);
    let mut path = url
        .path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot be a base: {base}"))?;
    path.pop_if_empty();
    for segment in segments {
        path.push(segment);
    }
    drop(path);
    Ok(url)
}

fn apify_url(base: &str, segments: &[&str]) -> Result<Url> {
    endpoint_url(base, segments)
}

async fn read_json_record(client: &Client, url: &Url, token: &str, label: &str) -> Result<Value> {
    let response = client.get(url.clone()).bearer_auth(token).send().await?;
    ensure_success(response, label)
        .await?
        .json()
        .await
        .with_context(|| format!("Could not parse {label} JSON"))
}

async fn push_dataset_item(client: &Client, url: &Url, token: &str, item: &Value) -> Result<()> {
    let response = client
        .post(url.clone())
        .bearer_auth(token)
        .json(item)
        .send()
        .await?;
    ensure_success(response, "Apify dataset item publication")
        .await?
        .bytes()
        .await
        .context("Could not finish Apify dataset item publication")?;
    Ok(())
}

async fn push_dataset_item_with_budget(
    client: &Client,
    url: &Url,
    token: &str,
    item: &Value,
    capacity: usize,
    saved_rows: &mut usize,
) -> Result<()> {
    if *saved_rows >= capacity {
        return Ok(());
    }
    push_dataset_item(client, url, token, item).await?;
    *saved_rows += 1;
    Ok(())
}

async fn put_json_record(
    client: &Client,
    url: &Url,
    token: &str,
    value: &Value,
    label: &str,
) -> Result<()> {
    let response = client
        .put(url.clone())
        .bearer_auth(token)
        .json(value)
        .send()
        .await?;
    ensure_success(response, label).await?.bytes().await?;
    Ok(())
}

async fn ensure_success(response: Response, label: &str) -> Result<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    let detail = if body.is_empty() {
        status.to_string()
    } else {
        format!("{status}: {body}")
    };
    Err(anyhow!("{label} failed: {detail}"))
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
            let (request_tx, requests) = mpsc::channel();
            let recorded_requests = request_tx;
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
                    let _ = recorded_requests.send(request);
                    let Some(response) = responses.next() else {
                        break;
                    };
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
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

    fn response(status: u16, body: impl Into<String>) -> MockResponse {
        MockResponse {
            status,
            body: body.into(),
        }
    }

    fn input_response() -> MockResponse {
        response(
            200,
            json!({"urls": [
                "https://www.linkedin.com/company/first",
                "https://www.linkedin.com/company/second"
            ]})
            .to_string(),
        )
    }

    fn pricing_response(max_charge: f64) -> MockResponse {
        response(
            200,
            json!({"data": {
                "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                    "apify-actor-start": {"eventPriceUsd": 0.00005}
                }}},
                "chargedEventCounts": {"apify-default-dataset-item": 0, "apify-actor-start": 0},
                "options": {"maxTotalChargeUsd": max_charge}
            }})
            .to_string(),
        )
    }

    fn company_response(name: &str) -> MockResponse {
        response(200, json!({"success": true, "name": name}).to_string())
    }

    fn config(base_url: &Url) -> ActorConfig {
        ActorConfig {
            apify_token: "test-apify-token".to_owned(),
            key_value_store_id: "test-store".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
            apify_base: base_url.to_string(),
            scrappa_base: base_url.to_string(),
        }
    }

    fn client() -> Client {
        Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap()
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

    fn dataset_items(requests: &[String]) -> Vec<Value> {
        requests
            .iter()
            .filter(|request| request.starts_with("POST /v2/datasets/"))
            .map(|request| serde_json::from_str(request_parts(request).2).unwrap())
            .collect()
    }

    fn input(value: Value) -> ActorInput {
        serde_json::from_value(value).expect("valid actor input")
    }

    #[test]
    fn combines_and_deduplicates_multiple_company_urls() {
        let input = input(json!({
            "url": "https://linkedin.com/company/microsoft",
            "urls": [
                "https://www.linkedin.com/company/microsoft/about/",
                "https://m.linkedin.com/company/openai/?trk=foo"
            ]
        }));

        let requests = get_input_urls(&input).expect("valid URL list");
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[0].input_url,
            "https://linkedin.com/company/microsoft"
        );
        assert_eq!(
            requests[0].normalized_url.as_deref(),
            Some("https://www.linkedin.com/company/microsoft")
        );
        assert_eq!(
            requests[1].input_url,
            "https://m.linkedin.com/company/openai/?trk=foo"
        );
        assert_eq!(
            requests[1].normalized_url.as_deref(),
            Some("https://www.linkedin.com/company/openai")
        );
    }

    #[test]
    fn invalid_urls_remain_per_item_failures() {
        let input = input(json!({"urls": ["https://example.com/company/acme"]}));
        let requests = get_input_urls(&input).expect("invalid URL is recoverable");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].normalized_url, None);
        assert_eq!(
            requests[0].validation_error.as_deref(),
            Some(INVALID_URL_MESSAGE)
        );
        assert_eq!(
            build_failure_item(
                INVALID_URL_MESSAGE,
                "error",
                None,
                "https://example.com/company/acme",
                None,
            ),
            json!({
                "success": false,
                "input_url": "https://example.com/company/acme",
                "error": INVALID_URL_MESSAGE,
                "error_type": "error",
                "message": INVALID_URL_MESSAGE,
            })
        );
    }

    #[test]
    fn output_is_detail_for_one_url_and_summary_for_multiple_urls() {
        let detail = json!({
            "success": true,
            "name": "Microsoft",
            "url": "https://www.linkedin.com/company/microsoft",
            "input_url": "linkedin.com/company/microsoft",
            "normalized_url": "https://www.linkedin.com/company/microsoft"
        });
        assert_eq!(build_output(1, &detail, 1, 0), detail);
        assert_eq!(
            build_output(2, &detail, 1, 1),
            json!({"requested": 2, "succeeded": 1, "failed": 1})
        );
    }

    #[test]
    fn dataset_item_preserves_scrappa_fields_and_adds_url_metadata() {
        let item = build_dataset_item(
            json!({"success": true, "name": "Microsoft", "followers": 1}),
            "linkedin.com/company/microsoft",
            "https://www.linkedin.com/company/microsoft",
        )
        .unwrap();
        assert_eq!(
            item,
            json!({
                "success": true,
                "name": "Microsoft",
                "followers": 1,
                "url": "https://www.linkedin.com/company/microsoft",
                "input_url": "linkedin.com/company/microsoft",
                "normalized_url": "https://www.linkedin.com/company/microsoft",
            })
        );
    }

    #[test]
    fn malformed_company_response_does_not_publish_metadata_only() {
        for response in [Value::Null, json!([{"name": "invalid"}]), json!("invalid")] {
            assert!(build_dataset_item(
                response,
                "input",
                "https://www.linkedin.com/company/example"
            )
            .is_err());
        }
    }

    #[test]
    fn not_found_is_a_failed_dataset_item() {
        assert_eq!(
            build_failure_item(
                "Scrappa API error (404): Not found",
                "scrappa_api_error",
                Some(404),
                "linkedin.com/company/missing",
                Some("https://www.linkedin.com/company/missing"),
            ),
            json!({
                "success": false,
                "input_url": "linkedin.com/company/missing",
                "error": "Scrappa API error (404): Not found",
                "error_type": "scrappa_api_error",
                "message": "Company not found",
                "normalized_url": "https://www.linkedin.com/company/missing",
                "url": "https://www.linkedin.com/company/missing",
                "status_code": 404,
            })
        );
    }

    #[test]
    fn preserves_cache_parameter_rules() {
        assert_eq!(cache_age(Some(&json!(86400))), Some("86400".to_owned()));
        assert_eq!(cache_age(Some(&json!(0))), None);
        assert_eq!(cache_age(Some(&json!(42.5))), None);
        assert_eq!(cache_age(Some(&json!(1.0))), Some("1".to_owned()));
    }

    #[test]
    fn normalizes_linkedin_hosts_and_strips_subpaths() {
        assert_eq!(
            normalize_linkedin_company_url("https://de.linkedin.com/company/j.p.morgan/about/"),
            Ok("https://www.linkedin.com/company/j.p.morgan".to_owned())
        );
        assert_eq!(
            normalize_linkedin_company_url("http://linkedin.com/company/microsoft"),
            Ok("http://www.linkedin.com/company/microsoft".to_owned())
        );
    }

    #[tokio::test]
    async fn one_result_budget_posts_only_the_first_row_and_keeps_output_counts() {
        let server = MockServer::start(vec![
            input_response(),
            pricing_response(0.0003),
            company_response("First"),
            response(201, "{}"),
            company_response("Second"),
            response(200, "{}"),
        ]);
        let apify_client = client();
        let scrappa_client = client();
        let actor_config = config(&server.base_url);
        run_actor(&apify_client, &scrappa_client, &actor_config)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 6);
        assert_eq!(request_parts(&requests[1]).1, "/v2/actor-runs/test-run");
        assert!(requests[1]
            .to_ascii_lowercase()
            .contains("authorization: bearer test-apify-token"));
        let rows = dataset_items(&requests);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "First");
        assert_eq!(
            rows[0]["input_url"],
            "https://www.linkedin.com/company/first"
        );
        let output = requests
            .iter()
            .find(|request| {
                request.starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT")
            })
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(output).2).unwrap(),
            json!({"requested": 2, "succeeded": 2, "failed": 0})
        );
    }

    #[tokio::test]
    async fn zero_result_budget_skips_dataset_posts_but_preserves_output() {
        let server = MockServer::start(vec![
            input_response(),
            pricing_response(0.0),
            company_response("First"),
            company_response("Second"),
            response(200, "{}"),
        ]);
        let apify_client = client();
        let scrappa_client = client();
        let actor_config = config(&server.base_url);
        run_actor(&apify_client, &scrappa_client, &actor_config)
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert!(dataset_items(&requests).is_empty());
        let output = requests
            .iter()
            .find(|request| {
                request.starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT")
            })
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(output).2).unwrap(),
            json!({"requested": 2, "succeeded": 2, "failed": 0})
        );
    }

    #[tokio::test]
    async fn generous_numeric_budget_posts_all_rows_in_input_order() {
        let server = MockServer::start(vec![
            input_response(),
            pricing_response(1.0),
            company_response("First"),
            response(201, "{}"),
            company_response("Second"),
            response(201, "{}"),
            response(200, "{}"),
        ]);
        let apify_client = client();
        let scrappa_client = client();
        let actor_config = config(&server.base_url);
        run_actor(&apify_client, &scrappa_client, &actor_config)
            .await
            .unwrap();

        let rows = dataset_items(&server.requests());
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "First");
        assert_eq!(rows[1]["name"], "Second");
    }

    #[tokio::test]
    async fn missing_spending_limit_fails_before_scraping_or_dataset_writes() {
        let invalid_run = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
            }}},
            "chargedEventCounts": {},
            "options": {"maxTotalChargeUsd": null}
        }});
        let server = MockServer::start(vec![
            input_response(),
            response(200, invalid_run.to_string()),
        ]);
        let apify_client = client();
        let scrappa_client = client();
        let actor_config = config(&server.base_url);
        let error = run_actor(&apify_client, &scrappa_client, &actor_config)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("spending limit"));
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert!(dataset_items(&requests).is_empty());
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("GET /linkedin/")));
    }

    #[tokio::test]
    async fn dataset_storage_failure_aborts_without_later_writes_or_output() {
        let server = MockServer::start(vec![
            input_response(),
            pricing_response(1.0),
            company_response("First"),
            response(500, "dataset unavailable"),
        ]);
        let apify_client = client();
        let scrappa_client = client();
        let actor_config = config(&server.base_url);
        let error = run_actor(&apify_client, &scrappa_client, &actor_config)
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Apify dataset item publication failed"));
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(dataset_items(&requests).len(), 1);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("PUT /v2/key-value-stores/")));
    }

    #[test]
    fn available_capacity_accounts_for_every_charged_event() {
        let run = json!({"data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                "apify-actor-start": {"eventPriceUsd": 0.00005}
            }}},
            "chargedEventCounts": {"apify-default-dataset-item": 1, "apify-actor-start": 1},
            "options": {"maxTotalChargeUsd": 0.00065}
        }});
        assert_eq!(affordable_dataset_items(&run, 2).unwrap(), 1);
        let mut missing_counts = run.clone();
        missing_counts["data"]
            .as_object_mut()
            .unwrap()
            .remove("chargedEventCounts");
        assert!(affordable_dataset_items(&missing_counts, 2).is_err());
    }
}
