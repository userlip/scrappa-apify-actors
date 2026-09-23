use std::{collections::HashSet, env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response};
use serde_json::{json, Map, Value};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const TOKEN_FIELDS: [&str; 5] = [
    "topic_token",
    "kgmid",
    "publication_token",
    "section_token",
    "story_token",
];
const MAX_QUERIES_PER_RUN: usize = 10;
const REQUEST_ENRICHMENT_FIELDS: [&str; 11] = [
    "q",
    "gl",
    "hl",
    "page",
    "start",
    "so",
    "topic_token",
    "publication_token",
    "section_token",
    "story_token",
    "kgmid",
];

struct Config {
    apify_api_base: Url,
    scrappa_api_base: Url,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        Ok(Self {
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
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

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to read API response")?;
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
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
    serde_json::from_str(&body).with_context(|| format!("{operation} returned invalid JSON"))
}

async fn ensure_success(response: Response, operation: &str) -> Result<()> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }
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

struct ApifyClient<'a> {
    http: &'a Client,
    config: &'a Config,
}

impl ApifyClient<'_> {
    fn endpoint(&self, segments: &[&str]) -> Result<Url> {
        endpoint_url(&self.config.apify_api_base, segments)
    }

    async fn get_input(&self) -> Result<Value> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            &self.config.input_key,
        ])?;
        let response = self
            .http
            .get(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .context("Apify INPUT request failed")?;
        response_json(response, "Apify INPUT request").await
    }

    async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "datasets", &self.config.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Apify dataset write failed")?;
        ensure_success(response, "Apify dataset write").await
    }

    async fn put_output(&self, output: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.config.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.config.apify_token)
            .header(header::ACCEPT, "application/json")
            .json(output)
            .send()
            .await
            .context("Apify OUTPUT write failed")?;
        ensure_success(response, "Apify OUTPUT write").await
    }
}

#[derive(Debug)]
struct ScrappaApiError {
    status: u16,
    message: String,
}

impl std::fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.message
        )
    }
}

impl std::error::Error for ScrappaApiError {}

fn is_actor_level_scrappa_failure(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| matches!(error.status, 401 | 403))
}

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            SCRAPPA_REQUEST_TIMEOUT.as_millis()
        )
    } else {
        anyhow!(error.to_string())
    }
}

fn scrappa_error(status: u16, body: &str) -> ScrappaApiError {
    let Some(data) = serde_json::from_str::<Value>(body).ok() else {
        return ScrappaApiError {
            status,
            message: if body.is_empty() {
                format!("HTTP {status}")
            } else {
                body.to_owned()
            },
        };
    };
    let Some(object) = data.as_object() else {
        return ScrappaApiError {
            status,
            message: body.to_owned(),
        };
    };

    let mut message = object
        .get("message")
        .filter(|value| !value.is_null())
        .map(js_string)
        .unwrap_or_else(|| format!("HTTP {status}"));
    if let Some(errors) = object.get("errors").filter(|value| js_truthy(value)) {
        let Some(errors) = errors.as_object() else {
            return ScrappaApiError {
                status,
                message: body.to_owned(),
            };
        };
        let mut details = Vec::with_capacity(errors.len());
        for (field, messages) in errors {
            let Some(messages) = messages.as_array() else {
                return ScrappaApiError {
                    status,
                    message: body.to_owned(),
                };
            };
            details.push(format!(
                "{field}: {}",
                messages
                    .iter()
                    .map(js_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        message.push_str(" - ");
        message.push_str(&details.join("; "));
    }
    ScrappaApiError { status, message }
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(number) => number
            .as_f64()
            .filter(|value| value.fract() == 0.0 && value.abs() < 1e21)
            .map(|value| {
                if value == 0.0 {
                    "0".to_owned()
                } else {
                    format!("{value:.0}")
                }
            })
            .unwrap_or_else(|| number.to_string()),
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

async fn fetch_google_news(
    http: &Client,
    config: &Config,
    params: &Map<String, Value>,
) -> Result<Value> {
    let mut url = endpoint_url(&config.scrappa_api_base, &["google", "news"])?;
    for (key, value) in params {
        url.query_pairs_mut().append_pair(key, &js_string(value));
    }
    let response = http
        .get(url)
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .header("X-API-Key", &config.scrappa_api_key)
        .header(header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(scrappa_request_error)?;
    let status = response.status();
    let body = response.text().await.map_err(scrappa_request_error)?;
    if !status.is_success() {
        return Err(scrappa_error(status.as_u16(), &body).into());
    }
    serde_json::from_str(&body)
        .map_err(|error| anyhow!("Scrappa API response was not valid JSON: {error}"))
}

fn clean_optional_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(value.to_owned()))
}

fn clean_two_letter_code(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() != 2 || !value.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        bail!("{field} must be a two-letter code");
    }
    Ok(Some(value.to_ascii_lowercase()))
}

fn clean_integer(
    value: Option<&Value>,
    field: &str,
    min: i64,
    max: Option<i64>,
) -> Result<Option<Value>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(number) = value.as_number() else {
        bail!("{field} must be an integer");
    };
    let numeric = number
        .as_i64()
        .map(|value| value as f64)
        .or_else(|| number.as_u64().map(|value| value as f64))
        .or_else(|| number.as_f64().filter(|value| value.is_finite()))
        .ok_or_else(|| anyhow!("{field} must be an integer"))?;
    if numeric.fract() != 0.0 {
        bail!("{field} must be an integer");
    }
    if numeric < min as f64 || max.is_some_and(|max| numeric > max as f64) {
        if let Some(max) = max {
            bail!("{field} must be between {min} and {max}");
        }
        bail!("{field} must be greater than or equal to {min}");
    }
    Ok(Some(value.clone()))
}

fn get_google_news_queries(input: &Map<String, Value>) -> Result<Vec<String>> {
    let mut raw_queries = Vec::new();
    if let Some(query) = input
        .get("q")
        .filter(|value| !value.is_null() && value.as_str() != Some(""))
    {
        raw_queries.push(query);
    }
    if let Some(queries) = input
        .get("queries")
        .filter(|value| !value.is_null() && value.as_str() != Some(""))
    {
        let Some(queries) = queries.as_array() else {
            bail!("queries must be an array");
        };
        if queries.len() > MAX_QUERIES_PER_RUN {
            bail!("queries must contain {MAX_QUERIES_PER_RUN} items or fewer");
        }
        raw_queries.extend(queries.iter());
    }

    let mut seen = HashSet::new();
    let mut queries = Vec::new();
    for raw_query in raw_queries {
        let Some(query) = clean_optional_string(Some(raw_query), "q", 500)? else {
            continue;
        };
        if seen.insert(query.clone()) {
            queries.push(query);
        }
    }
    if queries.len() > MAX_QUERIES_PER_RUN {
        bail!("queries must contain {MAX_QUERIES_PER_RUN} items or fewer");
    }
    Ok(queries)
}

fn build_google_news_params(input: &Map<String, Value>) -> Result<Map<String, Value>> {
    let query = clean_optional_string(input.get("q"), "q", 500)?;
    let mut token_params = Vec::new();
    for field in TOKEN_FIELDS {
        let max_length = match field {
            "story_token" => 2000,
            "kgmid" => 100,
            _ => 200,
        };
        if let Some(value) = clean_optional_string(input.get(field), field, max_length)? {
            token_params.push((field, value));
        }
    }
    if query.is_none() && token_params.is_empty() {
        bail!("Provide q or one Google News token parameter");
    }
    if query.is_some() && !token_params.is_empty() {
        bail!("Cannot use q with topic_token, kgmid, publication_token, section_token, or story_token");
    }
    if token_params.iter().any(|(field, _)| *field == "kgmid") && token_params.len() > 1 {
        bail!("kgmid must be used alone and cannot be combined with other token parameters");
    }
    if let Some(("kgmid", value)) = token_params.iter().find(|(field, _)| *field == "kgmid") {
        if !value.starts_with("/m/") && !value.starts_with("/g/") {
            bail!("kgmid must start with /m/ or /g/");
        }
    }

    let page = clean_integer(input.get("page"), "page", 1, None)?;
    let start = clean_integer(input.get("start"), "start", 0, None)?;
    if page.is_some() && start.is_some() {
        bail!("Cannot use both page and start parameters");
    }
    let sort_order = clean_integer(input.get("so"), "so", 0, Some(1))?;

    let mut params = Map::new();
    if let Some(query) = query {
        params.insert("q".to_owned(), Value::String(query));
    }
    if input.contains_key("gl") {
        if let Some(code) = clean_two_letter_code(input.get("gl"), "gl")? {
            params.insert("gl".to_owned(), Value::String(code));
        }
    }
    if input.contains_key("hl") {
        if let Some(code) = clean_two_letter_code(input.get("hl"), "hl")? {
            params.insert("hl".to_owned(), Value::String(code));
        }
    }
    if let Some(page) = page {
        params.insert("page".to_owned(), page);
    }
    if let Some(start) = start {
        params.insert("start".to_owned(), start);
    }
    if let Some(sort_order) = sort_order {
        params.insert("so".to_owned(), sort_order);
    }
    for (field, value) in token_params {
        params.insert(field.to_owned(), Value::String(value));
    }
    Ok(params)
}

fn build_google_news_param_list(input: &Value) -> Result<Vec<Map<String, Value>>> {
    let Some(input) = input.as_object() else {
        bail!("Input must be an object");
    };
    let queries = get_google_news_queries(input)?;
    if queries.is_empty() {
        return Ok(vec![build_google_news_params(input)?]);
    }
    if TOKEN_FIELDS.iter().any(|field| {
        input
            .get(*field)
            .is_some_and(|value| !value.is_null() && value.as_str() != Some(""))
    }) {
        bail!("Cannot use queries with topic_token, kgmid, publication_token, section_token, or story_token");
    }

    queries
        .iter()
        .map(|query| {
            let mut query_input = input.clone();
            query_input.insert("q".to_owned(), Value::String(query.clone()));
            query_input.remove("queries");
            build_google_news_params(&query_input)
        })
        .collect()
}

fn describe_google_news_request(params: &Map<String, Value>) -> String {
    let mut pagination = Vec::new();
    if let Some(page) = params.get("page") {
        pagination.push(format!("page {}", js_string(page)));
    }
    if let Some(start) = params.get("start") {
        pagination.push(format!("start {}", js_string(start)));
    }
    let pagination = if pagination.is_empty() {
        String::new()
    } else {
        format!(" ({})", pagination.join(", "))
    };
    if let Some(query) = params.get("q").and_then(Value::as_str) {
        return format!("query \"{query}\"{pagination}");
    }
    for field in TOKEN_FIELDS {
        if let Some(token) = params.get(field).and_then(Value::as_str) {
            return format!("{field} {token}{pagination}");
        }
    }
    format!("Google News request{pagination}")
}

fn response_items<'a>(response: &'a Value, field: &str) -> Result<&'a [Value]> {
    match response.get(field) {
        None | Some(Value::Null) => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        Some(_) => bail!("Scrappa API response field {field} must be an array"),
    }
}

fn array_or_string_length(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(values)) => values.len(),
        Some(Value::String(value)) => value.encode_utf16().count(),
        _ => 0,
    }
}

fn enrich_result(result: &Value, params: &Map<String, Value>) -> Result<Value> {
    let Some(result) = result.as_object() else {
        bail!("Scrappa API news_results items must be objects");
    };
    let mut enriched = result.clone();
    enriched.remove("source_name");
    let source_name = match result.get("source") {
        Some(Value::String(source)) => Some(Value::String(source.clone())),
        Some(Value::Object(source)) => source
            .get("name")
            .filter(|value| !value.is_null())
            .or_else(|| source.get("title").filter(|value| !value.is_null()))
            .cloned(),
        _ => None,
    };
    if let Some(source_name) = source_name {
        enriched.insert("source_name".to_owned(), source_name);
    }
    for field in REQUEST_ENRICHMENT_FIELDS {
        enriched.insert(
            format!("request_{field}"),
            params.get(field).cloned().unwrap_or(Value::Null),
        );
    }
    Ok(Value::Object(enriched))
}

async fn run_actor(http: &Client, config: &Config) -> Result<()> {
    let apify = ApifyClient { http, config };
    let input = apify.get_input().await?;
    if input.is_null() {
        bail!("Input is required");
    }
    let param_list = build_google_news_param_list(&input)?;
    println!(
        "Running {} Google News request{}",
        param_list.len(),
        if param_list.len() == 1 { "" } else { "s" }
    );
    let keep_raw_response = param_list.len() == 1;
    let mut single_response = None;
    let mut request_summaries = Vec::with_capacity(param_list.len());
    let mut total_news_results = 0;
    let mut failed_requests = 0;

    for params in &param_list {
        let request_description = describe_google_news_request(params);
        println!("Fetching Google News for {request_description}");
        let request_result: Result<(Value, Vec<Value>, usize, usize)> = async {
            let response = fetch_google_news(http, config, params).await?;
            let news_results = response_items(&response, "news_results")?;
            let story_count = array_or_string_length(response.get("stories"));
            let related_search_count = array_or_string_length(response.get("related_searches"));
            let enriched_results = news_results
                .iter()
                .map(|result| enrich_result(result, params))
                .collect::<Result<Vec<_>>>()?;
            Ok((
                response,
                enriched_results,
                story_count,
                related_search_count,
            ))
        }
        .await;

        match request_result {
            Ok((response, enriched_results, story_count, related_search_count)) => {
                apify.push_dataset_items(&enriched_results).await?;
                let news_count = enriched_results.len();
                total_news_results += news_count;
                if keep_raw_response {
                    single_response = Some(response);
                }
                if news_count > 0 {
                    println!("Found {news_count} news results");
                } else {
                    println!("No Google News results found for this request");
                }
                request_summaries.push(json!({
                    "request": params,
                    "success": true,
                    "news_results": news_count,
                    "stories": story_count,
                    "related_searches": related_search_count,
                }));
            }
            Err(error) => {
                if param_list.len() == 1 || is_actor_level_scrappa_failure(&error) {
                    return Err(error);
                }
                failed_requests += 1;
                let message = error.to_string();
                eprintln!("Google News request failed for {request_description}: {message}");
                request_summaries.push(json!({
                    "request": params,
                    "success": false,
                    "news_results": 0,
                    "stories": 0,
                    "related_searches": 0,
                    "error_message": message,
                }));
            }
        }
    }

    let output = if keep_raw_response {
        single_response.ok_or_else(|| anyhow!("Google News response was not available"))?
    } else {
        json!({
            "requests": request_summaries,
            "news_results": total_news_results,
            "failed_requests": failed_requests,
        })
    };
    apify.put_output(&output).await?;
    println!("Google News scraping completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string(&json!({
            "requests": param_list.len(),
            "news_results": total_news_results,
            "failed_requests": failed_requests,
        }))?
    );
    Ok(())
}

fn actor_error_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if message.contains("timed out") {
        format!(
            "{message}. The Google News request exceeded the {}s Scrappa API timeout. Try a more specific query or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

#[tokio::main]
async fn main() {
    let result = async {
        let config = Config::from_env()?;
        let http = Client::new();
        run_actor(&http, &config).await
    }
    .await;
    if let Err(error) = result {
        eprintln!("Actor failed: {}", actor_error_message(&error));
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
        thread::{self, JoinHandle},
    };

    const INPUT_SCHEMA: &str = include_str!("../.actor/input_schema.json");

    struct MockRequest {
        method: String,
        target: String,
        headers: Map<String, Value>,
        body: String,
    }

    struct MockServer {
        base_url: String,
        requests: Receiver<MockRequest>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<(u16, String)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                for (status, body) in responses {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_mock_request(&mut stream);
                    sender.send(request).unwrap();
                    let reason = match status {
                        200 => "OK",
                        403 => "Forbidden",
                        404 => "Not Found",
                        _ => "Error",
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();
                }
            });
            Self {
                base_url: format!("http://{address}"),
                requests,
                thread: Some(thread),
            }
        }

        fn finish(mut self) -> Vec<MockRequest> {
            let requests = self.requests.try_iter().collect();
            self.thread.take().unwrap().join().unwrap();
            requests
        }
    }

    fn mock_response(status: u16, body: Value) -> (u16, String) {
        (status, body.to_string())
    }

    fn read_mock_request(stream: &mut TcpStream) -> MockRequest {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
            if let Some(header_end) = header_end {
                let header_text = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length = header_text
                    .lines()
                    .skip(1)
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0, "mock request ended before its body was read");
            bytes.extend_from_slice(&buffer[..count]);
        }
        let header_end = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let header_text = String::from_utf8_lossy(&bytes[..header_end]);
        let mut lines = header_text.lines();
        let mut request_line = lines.next().unwrap().split_whitespace();
        let method = request_line.next().unwrap().to_owned();
        let target = request_line.next().unwrap().to_owned();
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| {
                (
                    name.to_ascii_lowercase(),
                    Value::String(value.trim().to_owned()),
                )
            })
            .collect();
        let body = String::from_utf8(bytes[header_end + 4..].to_vec()).unwrap();
        MockRequest {
            method,
            target,
            headers,
            body,
        }
    }

    fn test_config(base_url: &str) -> Config {
        Config {
            apify_api_base: Url::parse(base_url).unwrap(),
            scrappa_api_base: Url::parse(base_url).unwrap(),
            apify_token: "apify-test-token".to_owned(),
            key_value_store_id: "store-id".to_owned(),
            dataset_id: "dataset-id".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
        }
    }

    fn input_request() -> String {
        "/v2/key-value-stores/store-id/records/INPUT".to_owned()
    }

    fn output_request() -> String {
        "/v2/key-value-stores/store-id/records/OUTPUT".to_owned()
    }

    #[test]
    fn input_schema_and_query_builder_keep_the_actor_contract() {
        let schema: Value = serde_json::from_str(INPUT_SCHEMA).unwrap();
        assert_eq!(schema["title"], "Google News Scraper");
        assert_eq!(schema["properties"]["queries"]["maxItems"], 10);
        assert_eq!(schema["properties"]["queries"]["items"]["type"], "string");
        for field in TOKEN_FIELDS {
            assert_eq!(schema["properties"][field]["type"], "string");
        }

        let params = build_google_news_param_list(&json!({
            "q": "markets",
            "queries": [" climate ", "markets", "technology"],
            "gl": " US ",
            "hl": "en",
            "page": 2
        }))
        .unwrap();
        assert_eq!(params.len(), 3);
        assert_eq!(params[0]["q"], "markets");
        assert_eq!(params[1]["q"], "climate");
        assert_eq!(params[2]["q"], "technology");
        assert_eq!(params[0]["gl"], "us");
        assert_eq!(
            describe_google_news_request(&params[1]),
            "query \"climate\" (page 2)"
        );
        assert_eq!(js_string(&json!(1.0)), "1");

        assert!(build_google_news_param_list(&json!({ "queries": "markets" })).is_err());
        assert!(
            build_google_news_param_list(&json!({ "q": "news", "topic_token": "token" })).is_err()
        );
        assert!(
            build_google_news_param_list(&json!({ "q": "news", "page": 1, "start": 0 })).is_err()
        );
        assert!(build_google_news_param_list(&json!({ "kgmid": "entity" })).is_err());
        assert!(build_google_news_param_list(&json!({ "kgmid": "/m/entity" })).is_ok());
        assert!(build_google_news_param_list(&json!({
            "queries": ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11"]
        }))
        .is_err());
        assert!(build_google_news_param_list(&json!({
            "queries": ["news"],
            "topic_token": "topic"
        }))
        .is_err());
        assert!(build_google_news_param_list(&json!({
            "kgmid": "/m/entity",
            "topic_token": "topic"
        }))
        .is_err());
        assert!(build_google_news_param_list(&json!({ "q": "news", "page": 1.5 })).is_err());
        assert!(build_google_news_param_list(&json!({ "q": "news", "so": 2 })).is_err());
        assert!(build_google_news_param_list(&json!({ "q": "news", "gl": "usa" })).is_err());
        let topic = build_google_news_param_list(&json!({ "topic_token": " topic " })).unwrap();
        assert_eq!(topic[0]["topic_token"], "topic");
    }

    #[tokio::test]
    async fn multiple_queries_keep_order_enrich_rows_and_recover_after_404() {
        let input = json!({
            "q": "markets",
            "queries": [" climate ", "markets", "science", ""],
            "gl": " US ",
            "hl": "en",
            "start": 20
        });
        let server = MockServer::start(vec![
            mock_response(200, input),
            mock_response(
                200,
                json!({
                    "news_results": [
                        {"position": 1, "title": "Headline", "source": {"name": "Wire"}},
                        {"position": 2, "title": "Second", "source": {"title": "Paper"}}
                    ],
                    "stories": [{}, {}],
                    "related_searches": [{}],
                    "provider_field": "preserved"
                }),
            ),
            mock_response(200, json!({})),
            mock_response(404, json!({"message": "missing page"})),
            mock_response(
                200,
                json!({
                    "news_results": [{"position": 3, "title": "Follow-up", "source": "Daily"}]
                }),
            ),
            mock_response(200, json!({})),
            mock_response(200, json!({})),
        ]);
        let config = test_config(&server.base_url);
        run_actor(&Client::new(), &config).await.unwrap();
        let requests = server.finish();
        assert_eq!(requests.len(), 7);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].target, input_request());
        assert_eq!(requests[1].method, "GET");
        assert!(requests[1].target.starts_with("/google/news?"));
        let query_url = Url::parse(&format!("http://mock{}", requests[1].target)).unwrap();
        let query: Map<String, Value> = query_url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), Value::String(value.into_owned())))
            .collect();
        assert_eq!(query["q"], "markets");
        assert_eq!(query["gl"], "us");
        assert_eq!(query["hl"], "en");
        assert_eq!(query["start"], "20");
        assert_eq!(requests[2].method, "POST");
        assert_eq!(requests[2].target, "/v2/datasets/dataset-id/items");
        let first_rows: Value = serde_json::from_str(&requests[2].body).unwrap();
        assert_eq!(first_rows.as_array().unwrap().len(), 2);
        assert_eq!(first_rows[0]["title"], "Headline");
        assert_eq!(first_rows[0]["position"], 1);
        assert_eq!(first_rows[1]["position"], 2);
        assert_eq!(first_rows[1]["title"], "Second");
        assert_eq!(first_rows[1]["source_name"], "Paper");
        assert_eq!(first_rows[0]["source_name"], "Wire");
        assert_eq!(first_rows[0]["request_q"], "markets");
        assert_eq!(first_rows[0]["request_gl"], "us");
        assert_eq!(first_rows[0]["request_start"], 20);
        assert_eq!(first_rows[0]["request_page"], Value::Null);
        assert_eq!(requests[3].method, "GET");
        assert!(requests[3].target.contains("q=climate"));
        assert_eq!(requests[4].method, "GET");
        assert!(requests[4].target.contains("q=science"));
        assert_eq!(requests[5].method, "POST");
        let later_rows: Value = serde_json::from_str(&requests[5].body).unwrap();
        assert_eq!(later_rows.as_array().unwrap().len(), 1);
        assert_eq!(later_rows[0]["position"], 3);
        assert_eq!(later_rows[0]["source_name"], "Daily");
        assert_eq!(later_rows[0]["request_q"], "science");
        assert_eq!(requests[6].method, "PUT");
        assert_eq!(requests[6].target, output_request());
        let output: Value = serde_json::from_str(&requests[6].body).unwrap();
        assert_eq!(output["news_results"], 3);
        assert_eq!(output["failed_requests"], 1);
        assert_eq!(output["requests"].as_array().unwrap().len(), 3);
        assert_eq!(output["requests"][0]["request"]["q"], "markets");
        assert_eq!(output["requests"][0]["news_results"], 2);
        assert_eq!(output["requests"][0]["stories"], 2);
        assert_eq!(output["requests"][0]["related_searches"], 1);
        assert_eq!(output["requests"][0]["success"], true);
        assert_eq!(output["requests"][1]["request"]["q"], "climate");
        assert_eq!(output["requests"][1]["success"], false);
        assert_eq!(output["requests"][1]["news_results"], 0);
        assert_eq!(
            output["requests"][1]["error_message"],
            "Scrappa API error (404): missing page"
        );
        assert_eq!(output["requests"][2]["request"]["q"], "science");
        assert_eq!(output["requests"][2]["success"], true);
        assert_eq!(output["requests"][2]["news_results"], 1);
        for request in [&requests[0], &requests[2], &requests[5], &requests[6]] {
            assert_eq!(request.headers["authorization"], "Bearer apify-test-token");
        }
        for request in [&requests[1], &requests[3], &requests[4]] {
            assert_eq!(request.headers["x-api-key"], "scrappa-test-key");
            assert!(!request.headers.contains_key("authorization"));
        }
    }

    #[tokio::test]
    async fn multi_query_dataset_failure_stops_without_claiming_success() {
        let server = MockServer::start(vec![
            mock_response(200, json!({"queries": ["one", "two"]})),
            mock_response(200, json!({"news_results": [{"title": "First"}]})),
            mock_response(503, json!({"message": "storage unavailable"})),
        ]);
        let config = test_config(&server.base_url);
        let error = run_actor(&Client::new(), &config).await.unwrap_err();
        assert!(error.to_string().contains("503"));
        let requests = server.finish();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[2].method, "POST");
    }

    #[tokio::test]
    async fn single_query_writes_the_unmodified_raw_response_to_output() {
        let response = json!({
            "news_results": [],
            "stories": [{"title": "story"}],
            "provider_field": {"kept": true}
        });
        let server = MockServer::start(vec![
            mock_response(200, json!({"q": "one"})),
            mock_response(200, response.clone()),
            mock_response(200, json!({})),
        ]);
        let config = test_config(&server.base_url);
        run_actor(&Client::new(), &config).await.unwrap();
        let requests = server.finish();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[2].method, "PUT");
        assert_eq!(requests[2].target, output_request());
        assert_eq!(
            serde_json::from_str::<Value>(&requests[2].body).unwrap(),
            response
        );
    }

    #[tokio::test]
    async fn multi_query_authentication_errors_fail_the_actor_without_output() {
        let server = MockServer::start(vec![
            mock_response(200, json!({"queries": ["one", "two"]})),
            mock_response(403, json!({"message": "forbidden"})),
        ]);
        let config = test_config(&server.base_url);
        let error = run_actor(&Client::new(), &config).await.unwrap_err();
        assert!(is_actor_level_scrappa_failure(&error));
        assert!(error.to_string().contains("Scrappa API error (403)"));
        let requests = server.finish();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].target, input_request());
        assert!(requests[1].target.starts_with("/google/news?"));
    }

    #[test]
    fn only_401_and_403_are_actor_level_scrappa_failures() {
        for status in [401, 403] {
            let error: anyhow::Error = ScrappaApiError {
                status,
                message: "denied".to_owned(),
            }
            .into();
            assert!(is_actor_level_scrappa_failure(&error));
        }
        let error: anyhow::Error = ScrappaApiError {
            status: 404,
            message: "missing".to_owned(),
        }
        .into();
        assert!(!is_actor_level_scrappa_failure(&error));
    }
}
