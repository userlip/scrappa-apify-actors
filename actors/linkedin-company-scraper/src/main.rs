use std::{collections::HashSet, env, fmt, process, time::Duration};

use anyhow::{anyhow, Context, Result};
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

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let token = required_env("APIFY_TOKEN")?;
    let key_value_store_id = required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?;
    let dataset_id = required_env("ACTOR_DEFAULT_DATASET_ID")?;
    let input_key = env::var("ACTOR_INPUT_KEY")
        .ok()
        .filter(|key| !key.is_empty())
        .unwrap_or_else(|| "INPUT".to_owned());
    let scrappa_api_key = required_env("SCRAPPA_API_KEY")?;
    let apify_base = env_or_default("APIFY_API_PUBLIC_BASE_URL", "https://api.apify.com");
    let scrappa_base = env_or_default("SCRAPPA_API_BASE_URL", "https://scrappa.co/api");

    let apify_client = Client::new();
    let scrappa_client = Client::builder()
        .timeout(SCRAPPA_TIMEOUT)
        .build()
        .context("Could not create Scrappa HTTP client")?;
    let input_url = apify_url(
        &apify_base,
        &[
            "v2",
            "key-value-stores",
            &key_value_store_id,
            "records",
            &input_key,
        ],
    )?;
    let input_value = read_json_record(&apify_client, &input_url, &token, "Apify INPUT").await?;
    let input = serde_json::from_value::<Option<ActorInput>>(input_value)
        .context("Could not parse Apify INPUT")?
        .unwrap_or_default();
    let requests = get_input_urls(&input)?;
    if requests.is_empty() {
        return Err(anyhow!(
            "At least one LinkedIn company URL is required. Provide url or urls."
        ));
    }

    let scrappa_url = endpoint_url(&scrappa_base, &["linkedin", "company"])?;
    let dataset_url = apify_url(&apify_base, &["v2", "datasets", &dataset_id, "items"])?;
    let output_url = apify_url(
        &apify_base,
        &[
            "v2",
            "key-value-stores",
            &key_value_store_id,
            "records",
            "OUTPUT",
        ],
    )?;

    let total = requests.len();
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
                    &scrappa_client,
                    &scrappa_url,
                    &scrappa_api_key,
                    normalized_url,
                    &input,
                )
                .await
                {
                    Ok(response) => {
                        build_dataset_item(response, &request.input_url, normalized_url)
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

        push_dataset_item(&apify_client, &dataset_url, &token, &result).await?;
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
    put_json_record(&apify_client, &output_url, &token, &output, "Apify OUTPUT").await?;

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

fn build_dataset_item(response: Value, input_url: &str, normalized_url: &str) -> Value {
    let mut item = response.as_object().cloned().unwrap_or_else(Map::new);
    if item.get("url").is_none_or(Value::is_null) {
        item.insert("url".to_owned(), json!(normalized_url));
    }
    item.insert("input_url".to_owned(), json!(input_url));
    item.insert("normalized_url".to_owned(), json!(normalized_url));
    Value::Object(item)
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
        );
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
}
