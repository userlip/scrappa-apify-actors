use std::{collections::HashSet, env, process::ExitCode, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Number, Value};
use url::Url;

const PROFILE_URL_ERROR: &str =
    "Invalid LinkedIn profile URL. Expected format: https://www.linkedin.com/in/profile-slug";
const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_MAX_RETRIES: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
struct UrlRequest {
    input_url: String,
    normalized_url: Option<String>,
    validation_error: Option<String>,
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

fn is_recoverable_linkedin_profile_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| error.status == 404)
}

struct Config {
    apify_api_base: String,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_base: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify_api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: required_env("ACTOR_INPUT_KEY")?,
            scrappa_api_base: env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
            scrappa_api_key,
        })
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

struct ApifyClient {
    http: Client,
    base_url: String,
    token: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    fn new(http: Client, config: &Config) -> Self {
        Self {
            http,
            base_url: config.apify_api_base.clone(),
            token: config.apify_token.clone(),
            key_value_store_id: config.key_value_store_id.clone(),
            dataset_id: config.dataset_id.clone(),
            input_key: config.input_key.clone(),
        }
    }

    fn endpoint(&self, path: &[&str]) -> Result<Url> {
        endpoint_url(&self.base_url, path)
    }

    async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await
                .context("Failed to retrieve actor input from Apify API")?;

            if response.status() == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if let Some(delay) = apify_retry_delay("GET", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            let response = require_apify_success(response, "input retrieval").await?;
            return response
                .json::<Value>()
                .await
                .context("Apify input record was not valid JSON")
                .map(Some);
        }
    }

    async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(item)
            .send()
            .await
            .context("Failed to publish dataset item to Apify API")?;
        require_apify_success(response, "dataset item publication").await?;
        Ok(())
    }

    async fn put_record(&self, key: &str, value: &Value) -> Result<()> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            key,
        ])?;
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .put(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .json(value)
                .send()
                .await
                .with_context(|| format!("Failed to write {key} record to Apify API"))?;

            if let Some(delay) = apify_retry_delay("PUT", response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }

            require_apify_success(response, &format!("{key} record publication")).await?;
            return Ok(());
        }
    }
}

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    bail!("Apify {operation} failed ({}): {body}", status.as_u16());
}

// Retry idempotent GET/PUT on explicit transient statuses only; a dataset POST may have appended before failure.
fn apify_retry_delay(method: &str, status: StatusCode, retry_count: usize) -> Option<Duration> {
    if !matches!(method, "GET" | "PUT")
        || retry_count >= APIFY_MAX_RETRIES
        || (status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error())
    {
        return None;
    }

    Some(Duration::from_secs((retry_count + 1) as u64))
}

struct ScrappaClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    fn new(http: Client, base_url: String, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    async fn get_profile(
        &self,
        normalized_url: &str,
        use_cache: Option<&Value>,
        maximum_cache_age: Option<&Value>,
    ) -> Result<Value> {
        let url = endpoint_url(&self.base_url, &["linkedin", "profile"])?;
        let params = build_profile_params(normalized_url, use_cache, maximum_cache_age);
        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .query(&params)
            .send()
            .await
            .map_err(scrappa_transport_error)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .context("Failed to read Scrappa API error response")?;
            return Err(ScrappaApiError {
                status,
                message: scrappa_error_message(status, &body),
            }
            .into());
        }

        response
            .json::<Value>()
            .await
            .map_err(scrappa_transport_error)
    }
}

fn scrappa_transport_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            SCRAPPA_TIMEOUT.as_millis()
        )
    } else {
        error.into()
    }
}

fn endpoint_url(base: &str, path: &[&str]) -> Result<Url> {
    let mut url = Url::parse(&format!("{}/", base.trim_end_matches('/')))
        .with_context(|| format!("Invalid API base URL: {base}"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow!("API base URL cannot contain query or fragment: {base}"))?;
        segments.pop_if_empty();
        for segment in path {
            segments.push(segment);
        }
    }
    Ok(url)
}

fn scrappa_error_message(status: u16, body: &str) -> String {
    let Ok(error_data) = serde_json::from_str::<Value>(body) else {
        return if body.is_empty() {
            format!("HTTP {status}")
        } else {
            body.to_owned()
        };
    };

    let mut message = error_data
        .get("message")
        .filter(|value| !value.is_null())
        .map(js_string)
        .unwrap_or_else(|| format!("HTTP {status}"));

    if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .map(|(field, messages)| {
                let messages = messages
                    .as_array()
                    .map(|messages| {
                        messages
                            .iter()
                            .map(js_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                format!("{field}: {messages}")
            })
            .collect::<Vec<_>>()
            .join("; ");
        message.push_str(" - ");
        message.push_str(&details);
    }

    message
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

fn build_profile_params(
    normalized_url: &str,
    use_cache: Option<&Value>,
    maximum_cache_age: Option<&Value>,
) -> Vec<(String, String)> {
    let mut params = vec![("url".to_owned(), normalized_url.to_owned())];
    if !use_cache.is_some_and(js_truthy) {
        return params;
    }

    params.push(("use_cache".to_owned(), "1".to_owned()));
    if let Some(age) = maximum_cache_age.and_then(cache_age_string) {
        params.push(("maximum_cache_age".to_owned(), age));
    }
    params
}

fn cache_age_string(value: &Value) -> Option<String> {
    let number = value.as_number()?;
    if let Some(age) = number.as_u64().filter(|age| *age >= 1) {
        return Some(age.to_string());
    }

    let age = number.as_f64()?;
    if !age.is_finite() || age < 1.0 || age.fract() != 0.0 {
        return None;
    }

    Some(format!("{age:.0}"))
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

fn get_input_urls(input: Option<&Value>) -> Result<Vec<UrlRequest>> {
    let mut raw_urls = Vec::new();
    if let Some(url) = input
        .and_then(|input| input.get("url"))
        .and_then(Value::as_str)
    {
        raw_urls.push(url);
    }
    if let Some(urls) = input
        .and_then(|input| input.get("urls"))
        .and_then(Value::as_array)
    {
        for url in urls {
            raw_urls.push(
                url.as_str()
                    .ok_or_else(|| anyhow!("LinkedIn profile URLs must be strings"))?,
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

        match normalize_linkedin_profile_url(&input_url) {
            Ok(normalized_url) if seen.insert(normalized_url.clone()) => {
                requests.push(UrlRequest {
                    input_url,
                    normalized_url: Some(normalized_url),
                    validation_error: None,
                });
            }
            Ok(_) => {}
            Err(validation_error) => {
                if seen.insert(format!("invalid:{input_url}")) {
                    requests.push(UrlRequest {
                        input_url,
                        normalized_url: None,
                        validation_error: Some(validation_error),
                    });
                }
            }
        }
    }

    Ok(requests)
}

fn normalize_linkedin_profile_url(raw_url: &str) -> std::result::Result<String, String> {
    let candidate = raw_url.trim();
    let has_protocol = candidate.split_once("://").is_some_and(|(scheme, _)| {
        !scheme.is_empty() && scheme.bytes().all(|b| b.is_ascii_alphabetic())
    });
    let with_protocol = if has_protocol {
        candidate.to_owned()
    } else {
        format!("https://{candidate}")
    };
    let parsed = Url::parse(&with_protocol).map_err(|_| "Invalid URL".to_owned())?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(PROFILE_URL_ERROR.to_owned());
    }
    if !parsed.username().is_empty()
        || parsed
            .password()
            .is_some_and(|password| !password.is_empty())
        || parsed.port().is_some()
    {
        return Err(PROFILE_URL_ERROR.to_owned());
    }

    let hostname = parsed.host_str().unwrap_or_default();
    if !is_linkedin_hostname(hostname) {
        return Err(PROFILE_URL_ERROR.to_owned());
    }

    let path = parsed.path().trim_end_matches('/');
    let mut segments = path.strip_prefix('/').unwrap_or_default().split('/');
    let section = segments.next().unwrap_or_default();
    let slug = segments.next().unwrap_or_default();
    if !section.eq_ignore_ascii_case("in")
        || slug.is_empty()
        || !slug
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(PROFILE_URL_ERROR.to_owned());
    }

    Ok(format!("{}://www.linkedin.com/in/{slug}", parsed.scheme()))
}

fn is_linkedin_hostname(hostname: &str) -> bool {
    if hostname == "linkedin.com" {
        return true;
    }
    let Some(prefix) = hostname.strip_suffix(".linkedin.com") else {
        return false;
    };
    prefix == "www"
        || prefix == "m"
        || ((2..=3).contains(&prefix.len()) && prefix.bytes().all(|b| b.is_ascii_lowercase()))
}

fn build_success_item(mut response: Value, input_url: &str, normalized_url: &str) -> Result<Value> {
    let fields = response
        .as_object_mut()
        .ok_or_else(|| anyhow!("Scrappa profile response was not a JSON object"))?;
    let response_url = fields
        .get("url")
        .and_then(Value::as_str)
        .filter(|url| !url.trim().is_empty())
        .unwrap_or(normalized_url)
        .to_owned();
    fields.insert("url".to_owned(), Value::String(response_url));
    fields.insert("input_url".to_owned(), Value::String(input_url.to_owned()));
    fields.insert(
        "normalized_url".to_owned(),
        Value::String(normalized_url.to_owned()),
    );
    Ok(response)
}

fn build_failure_item(
    error_message: &str,
    api_status: Option<u16>,
    input_url: &str,
    normalized_url: Option<&str>,
) -> Value {
    let mut result = Map::new();
    result.insert("success".to_owned(), Value::Bool(false));
    result.insert("input_url".to_owned(), Value::String(input_url.to_owned()));
    if let Some(normalized_url) = normalized_url {
        result.insert(
            "normalized_url".to_owned(),
            Value::String(normalized_url.to_owned()),
        );
        result.insert("url".to_owned(), Value::String(normalized_url.to_owned()));
    }
    result.insert("error".to_owned(), Value::String(error_message.to_owned()));
    result.insert(
        "error_type".to_owned(),
        Value::String(
            if api_status.is_some() {
                "scrappa_api_error"
            } else {
                "error"
            }
            .to_owned(),
        ),
    );
    result.insert(
        "message".to_owned(),
        Value::String(if api_status == Some(404) {
            "Profile not found or not publicly accessible".to_owned()
        } else {
            error_message.to_owned()
        }),
    );
    if let Some(status) = api_status {
        result.insert(
            "status_code".to_owned(),
            Value::Number(Number::from(status)),
        );
    }
    Value::Object(result)
}

fn is_success(result: &Value) -> bool {
    result.get("success").is_some_and(js_truthy)
}

fn build_output(result: &Value) -> Value {
    let Some(fields) = result.as_object() else {
        return result.clone();
    };
    let mut output = fields.clone();
    for key in ["input_url", "normalized_url", "url", "error", "error_type"] {
        output.remove(key);
    }
    Value::Object(output)
}

struct PublicationPlan<'a> {
    successful_results: Vec<&'a Value>,
    failures: Vec<&'a Value>,
    output: Value,
}

fn plan_publication(results: &[Value]) -> PublicationPlan<'_> {
    let successful_results: Vec<_> = results.iter().filter(|result| is_success(result)).collect();
    let failures: Vec<_> = results
        .iter()
        .filter(|result| !is_success(result))
        .collect();
    let summary = json!({
        "requested": results.len(),
        "succeeded": successful_results.len(),
        "failed": failures.len(),
    });
    let output = if results.len() == 1 {
        build_output(&results[0])
    } else {
        summary
    };

    PublicationPlan {
        successful_results,
        failures,
        output,
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let http = Client::builder().timeout(SCRAPPA_TIMEOUT).build()?;
    let apify = ApifyClient::new(http.clone(), &config);
    let scrappa = ScrappaClient::new(
        http,
        config.scrappa_api_base.clone(),
        config.scrappa_api_key,
    );

    let input = apify.get_input().await?;
    let requests = get_input_urls(input.as_ref())?;
    if requests.is_empty() {
        bail!("At least one LinkedIn profile URL is required. Provide either url (single URL) or urls (array of URLs).");
    }

    println!(
        "Scraping {} LinkedIn profile URL{}",
        requests.len(),
        if requests.len() == 1 { "" } else { "s" }
    );
    let use_cache = input.as_ref().and_then(|input| input.get("use_cache"));
    let maximum_cache_age = input
        .as_ref()
        .and_then(|input| input.get("maximum_cache_age"));
    let mut results = Vec::with_capacity(requests.len());

    for request in requests {
        let Some(normalized_url) = request.normalized_url.as_deref() else {
            let message = request
                .validation_error
                .as_deref()
                .unwrap_or("Invalid LinkedIn profile URL");
            eprintln!("Invalid LinkedIn profile URL: \"{}\"", request.input_url);
            results.push(build_failure_item(message, None, &request.input_url, None));
            continue;
        };

        println!("Fetching LinkedIn profile: {normalized_url}");
        let result = match scrappa
            .get_profile(normalized_url, use_cache, maximum_cache_age)
            .await
        {
            Ok(response) => build_success_item(response, &request.input_url, normalized_url)?,
            Err(error) => {
                let recoverable = is_recoverable_linkedin_profile_error(&error);
                if !recoverable {
                    return Err(error);
                }
                let api_error = error
                    .downcast_ref::<ScrappaApiError>()
                    .expect("404 Scrappa error must retain its API error type");
                eprintln!(
                    "Profile scraping returned a per-item failure for {normalized_url}: {api_error}"
                );
                build_failure_item(
                    &api_error.to_string(),
                    Some(api_error.status),
                    &request.input_url,
                    Some(normalized_url),
                )
            }
        };

        if is_success(&result) {
            let name = result
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .unwrap_or("Unknown");
            println!("Successfully scraped profile: {name}");
        } else if result.get("status_code").and_then(Value::as_u64) != Some(404) {
            let message = result
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if message.is_empty() {
                eprintln!("Profile scraping returned success: false");
            } else {
                eprintln!("Profile scraping returned success: false ({message})");
            }
        }
        results.push(result);
    }

    let publication = plan_publication(&results);
    for result in &publication.successful_results {
        apify.push_dataset_item(result).await?;
    }
    apify.put_record("OUTPUT", &publication.output).await?;
    if !publication.failures.is_empty() {
        let failures = Value::Array(
            publication
                .failures
                .iter()
                .map(|failure| (*failure).clone())
                .collect(),
        );
        apify.put_record("FAILURES", &failures).await?;
    }

    println!("LinkedIn profile scraping completed");
    println!(
        "Profile summary: {}",
        serde_json::to_string_pretty(&json!({
            "requested": results.len(),
            "succeeded": publication.successful_results.len(),
            "failed": publication.failures.len(),
        }))?
    );
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_profile_urls_and_strips_tracking_and_extra_paths() {
        assert_eq!(
            normalize_linkedin_profile_url(" linkedin.com/in/williamhgates/?trk=foo ").unwrap(),
            "https://www.linkedin.com/in/williamhgates"
        );
        assert_eq!(
            normalize_linkedin_profile_url(
                "https://de.linkedin.com/in/SatyaNadella/details/experience/#about"
            )
            .unwrap(),
            "https://www.linkedin.com/in/SatyaNadella"
        );
        assert_eq!(
            normalize_linkedin_profile_url("http://m.linkedin.com/in/williamhgates").unwrap(),
            "http://www.linkedin.com/in/williamhgates"
        );
    }

    #[test]
    fn rejects_invalid_profile_urls_with_the_actor_validation_message() {
        for url in [
            "https://example.com/in/profile",
            "https://linkedin.com/company/acme",
            "ftp://linkedin.com/in/profile",
            "https://user:pass@linkedin.com/in/profile",
            "https://linkedin.com:444/in/profile",
        ] {
            assert_eq!(
                normalize_linkedin_profile_url(url),
                Err(PROFILE_URL_ERROR.to_owned())
            );
        }
        assert_eq!(
            normalize_linkedin_profile_url("https://%"),
            Err("Invalid URL".to_owned())
        );
    }

    #[test]
    fn combines_and_deduplicates_url_inputs_and_keeps_invalid_values_as_failures() {
        let input = json!({
            "url": "linkedin.com/in/williamhgates",
            "urls": [
                "https://de.linkedin.com/in/satyanadella/?trk=foo",
                "https://www.linkedin.com/in/williamhgates/details/experience/",
                "https://example.com/in/acme",
                "https://example.com/in/acme"
            ]
        });
        let requests = get_input_urls(Some(&input)).unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].input_url, "linkedin.com/in/williamhgates");
        assert_eq!(
            requests[0].normalized_url.as_deref(),
            Some("https://www.linkedin.com/in/williamhgates")
        );
        assert_eq!(
            requests[1].input_url,
            "https://de.linkedin.com/in/satyanadella/?trk=foo"
        );
        assert_eq!(requests[2].normalized_url, None);
        assert_eq!(
            requests[2].validation_error.as_deref(),
            Some(PROFILE_URL_ERROR)
        );
    }

    #[test]
    fn cache_parameters_match_the_existing_request_contract() {
        let url = "https://www.linkedin.com/in/williamhgates";
        assert_eq!(
            build_profile_params(url, Some(&json!(true)), Some(&json!(1))),
            vec![
                ("url".to_owned(), url.to_owned()),
                ("use_cache".to_owned(), "1".to_owned()),
                ("maximum_cache_age".to_owned(), "1".to_owned()),
            ]
        );
        for age in [json!(0), json!(-1), json!(1.5), json!("3600"), Value::Null] {
            assert_eq!(
                build_profile_params(url, Some(&json!(true)), Some(&age)),
                vec![
                    ("url".to_owned(), url.to_owned()),
                    ("use_cache".to_owned(), "1".to_owned()),
                ]
            );
        }
        assert_eq!(
            build_profile_params(url, Some(&json!(false)), Some(&json!(3600))),
            vec![("url".to_owned(), url.to_owned())]
        );
    }

    #[test]
    fn response_and_failure_shapes_match_the_existing_actor_contract() {
        let success = build_success_item(
            json!({"success": true, "name": "Bill Gates", "url": " ", "followers": 1}),
            "linkedin.com/in/williamhgates",
            "https://www.linkedin.com/in/williamhgates",
        )
        .unwrap();
        assert_eq!(success["url"], "https://www.linkedin.com/in/williamhgates");
        assert_eq!(success["input_url"], "linkedin.com/in/williamhgates");
        assert_eq!(
            success["normalized_url"],
            "https://www.linkedin.com/in/williamhgates"
        );
        assert_eq!(success["followers"], 1);

        let failure = build_failure_item(
            "Scrappa API error (404): Not found",
            Some(404),
            "linkedin.com/in/missing-profile",
            Some("https://www.linkedin.com/in/missing-profile"),
        );
        assert_eq!(failure["success"], false);
        assert_eq!(failure["error_type"], "scrappa_api_error");
        assert_eq!(
            failure["message"],
            "Profile not found or not publicly accessible"
        );
        assert_eq!(failure["status_code"], 404);

        assert_eq!(
            build_failure_item(
                PROFILE_URL_ERROR,
                None,
                "https://example.com/in/profile",
                None
            ),
            json!({
                "success": false,
                "input_url": "https://example.com/in/profile",
                "error": PROFILE_URL_ERROR,
                "error_type": "error",
                "message": PROFILE_URL_ERROR
            })
        );
        assert!(is_recoverable_linkedin_profile_error(&anyhow::Error::new(
            ScrappaApiError {
                status: 404,
                message: "Not found".to_owned(),
            }
        )));
        assert!(!is_recoverable_linkedin_profile_error(&anyhow::Error::new(
            ScrappaApiError {
                status: 401,
                message: "Unauthorized".to_owned(),
            }
        )));
        assert!(!is_recoverable_linkedin_profile_error(&anyhow::Error::new(
            ScrappaApiError {
                status: 500,
                message: "Unavailable".to_owned(),
            }
        )));
    }

    #[test]
    fn formats_scrappa_api_error_bodies_like_the_existing_client() {
        assert_eq!(
            scrappa_error_message(404, r#"{"message":"Not found"}"#),
            "Not found"
        );
        assert_eq!(
            scrappa_error_message(
                422,
                r#"{"message":"Invalid","errors":{"url":["bad","required"]}}"#
            ),
            "Invalid - url: bad, required"
        );
        assert_eq!(scrappa_error_message(503, ""), "HTTP 503");
        assert_eq!(scrappa_error_message(503, "Unavailable"), "Unavailable");
    }

    #[test]
    fn retries_only_safe_apify_methods_for_bounded_transient_responses() {
        assert_eq!(
            apify_retry_delay("GET", StatusCode::TOO_MANY_REQUESTS, 0),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            apify_retry_delay("PUT", StatusCode::INTERNAL_SERVER_ERROR, 1),
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            apify_retry_delay("GET", StatusCode::SERVICE_UNAVAILABLE, 2),
            None
        );
        assert_eq!(apify_retry_delay("PUT", StatusCode::BAD_REQUEST, 0), None);
        assert_eq!(
            apify_retry_delay("POST", StatusCode::INTERNAL_SERVER_ERROR, 0),
            None
        );
    }

    #[tokio::test]
    async fn dataset_post_failure_surfaces_without_retrying() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 2048];
            loop {
                let read = stream.read(&mut chunk).await.unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                let Some(body_start) = request
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .map(|position| position + 4)
                else {
                    continue;
                };
                let headers = std::str::from_utf8(&request[..body_start]).unwrap();
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap_or_default())
                    })
                    .unwrap_or_default();
                if request.len() >= body_start + content_length {
                    break;
                }
            }
            stream
                .write_all(
                    b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
            request
        });

        let config = Config {
            apify_api_base: format!("http://{address}"),
            apify_token: "test-token".to_owned(),
            key_value_store_id: "store".to_owned(),
            dataset_id: "dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_base: SCRAPPA_API_DEFAULT.to_owned(),
            scrappa_api_key: "test-key".to_owned(),
        };
        let http = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let apify = ApifyClient::new(http, &config);
        let error = apify
            .push_dataset_item(&json!({"success": true}))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Apify dataset item publication failed (503)"));
        let request = server.await.unwrap();
        assert!(request.starts_with(b"POST /v2/datasets/dataset/items HTTP/1.1\r\n"));
    }

    #[test]
    fn single_output_strips_wrapper_fields_and_batch_writes_only_successes() {
        let single = json!({
            "success": false,
            "input_url": "missing",
            "normalized_url": "https://www.linkedin.com/in/missing",
            "url": "https://www.linkedin.com/in/missing",
            "error": "ignored wrapper error",
            "error_type": "scrappa_api_error",
            "status_code": 404,
            "message": "Profile not found or not publicly accessible"
        });
        assert_eq!(
            build_output(&single),
            json!({
                "success": false,
                "status_code": 404,
                "message": "Profile not found or not publicly accessible"
            })
        );

        let success = json!({"success": true, "name": "Valid", "input_url": "valid"});
        let failed = json!({"success": false, "input_url": "invalid"});
        let results = vec![success.clone(), failed.clone()];
        let plan = plan_publication(&results);
        assert_eq!(plan.successful_results, vec![&success]);
        assert_eq!(plan.failures, vec![&failed]);
        assert_eq!(
            plan.output,
            json!({"requested": 2, "succeeded": 1, "failed": 1})
        );
    }

    #[test]
    fn api_endpoint_builder_keeps_local_mock_base_paths() {
        assert_eq!(
            endpoint_url(
                "http://127.0.0.1:8080/mock/",
                &["v2", "datasets", "dataset", "items"]
            )
            .unwrap()
            .as_str(),
            "http://127.0.0.1:8080/mock/v2/datasets/dataset/items"
        );
    }
}
