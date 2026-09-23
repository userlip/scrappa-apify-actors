use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{env, time::Duration};
use tokio::time::timeout;
use url::{form_urlencoded, Url};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://ytapi.scrappa.co";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    input_key: String,
    actor_run_id: String,
    apify_token: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            apify_token: required_env("APIFY_TOKEN")?,
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
        .extend(segments.iter().copied());
    Ok(url)
}

fn string_input(input: &Value, field: &str) -> Result<Option<String>> {
    let Some(value) = input.get(field) else {
        return Ok(None);
    };

    if let Some(values) = value.as_array() {
        let mut selected = None;
        for value in values {
            let Some(value) = value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            if selected.is_some() {
                bail!("{field} accepts only one selected value.");
            }
            selected = Some(value.to_owned());
        }
        return Ok(selected);
    }

    Ok(value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned))
}

fn build_trending_url(input: &Value, api_base_url: &Url) -> Result<(Url, Option<String>)> {
    let category = string_input(input, "category")?;
    let trending_type = string_input(input, "type")?;
    let mut url = endpoint_url(api_base_url, &["trending"])?;
    let mut query = form_urlencoded::Serializer::new(String::new());
    if let Some(category) = &category {
        query.append_pair("category", category);
    }
    if let Some(trending_type) = &trending_type {
        query.append_pair("type", trending_type);
    }
    let query = query.finish();
    if !query.is_empty() {
        url.set_query(Some(&query));
    }
    Ok((url, category))
}

fn trending_videos_to_dataset_items(data: &Value) -> &[Value] {
    data.get("results")
        .and_then(Value::as_array)
        .or_else(|| data.get("videos").and_then(Value::as_array))
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn continuation_token(data: &Value) -> Option<&Value> {
    data.get("pagination")
        .and_then(|pagination| pagination.get("continuationToken"))
        .filter(|token| !token.is_null())
        .or_else(|| data.get("continuation").filter(|token| !token.is_null()))
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

async fn fetch_trending(client: &Client, url: &Url) -> Result<Value> {
    let request = async {
        let response = client
            .get(url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(scrappa_request_error)?;
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
            .context("Scrappa API response was not valid JSON")
    };

    timeout(REQUEST_TIMEOUT, request).await.map_err(|_| {
        anyhow!(
            "Scrappa API request timed out after {}s",
            REQUEST_TIMEOUT.as_secs()
        )
    })?
}

async fn run_dataset_capacity(
    client: &Client,
    config: &ActorConfig,
    requested: usize,
) -> Result<usize> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify run pricing request failed")?;
    let run = response_json(response, "Apify run pricing request").await?;
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

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    for (event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
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

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    items: &[Value],
) -> Result<usize> {
    if items.is_empty() {
        return Ok(0);
    }

    let capacity = run_dataset_capacity(client, config, items.len()).await?;
    let items = &items[..items.len().min(capacity)];
    if items.is_empty() {
        return Ok(0);
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
    Ok(items.len())
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let (url, category) = build_trending_url(&input, &config.scrappa_api_base_url)?;
    println!("Fetching from: {url}");

    let data = fetch_trending(client, &url).await?;
    let videos = trending_videos_to_dataset_items(&data);
    let saved = push_dataset_items(client, config, videos).await?;
    println!(
        "Successfully fetched {} trending video(s) and saved {} for category: {}",
        videos.len(),
        saved,
        category.as_deref().unwrap_or("default")
    );

    if let Some(token) = continuation_token(&data).filter(|token| js_truthy(token)) {
        println!(
            "Continuation token available for next page: {}",
            js_string(token)
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube trending videos: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::new();
    run_actor(&client, &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    fn base_url() -> Url {
        Url::parse("https://ytapi.scrappa.co").unwrap()
    }

    fn test_config(server_url: Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: server_url.clone(),
            scrappa_api_base_url: server_url,
            default_key_value_store_id: "store-id".to_owned(),
            default_dataset_id: "dataset-id".to_owned(),
            input_key: "INPUT".to_owned(),
            actor_run_id: "run-id".to_owned(),
            apify_token: "test-token".to_owned(),
        }
    }
    const INPUT_RESPONSE: &str = r#"{"category":["music"],"type":["now"]}"#;
    const TWO_VIDEOS_RESPONSE: &str = r#"{"results":[{"id":"one","extra":{"preserved":true}},{"id":"two"}],"pagination":{"continuationToken":"next"}}"#;
    const RUN_ONE_ROW_CAP: &str = r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001},"other-event":{"eventPriceUsd":0.0001}}}},"chargedEventCounts":{"other-event":1},"options":{"maxTotalChargeUsd":0.00025}}}"#;
    const RUN_ZERO_ROW_CAP: &str = r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001},"other-event":{"eventPriceUsd":0.0001}}}},"chargedEventCounts":{"other-event":1},"options":{"maxTotalChargeUsd":0.00015}}}"#;
    const RUN_NORMAL_BUDGET: &str = r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001},"other-event":{"eventPriceUsd":0.0001}}}},"chargedEventCounts":{"other-event":1},"options":{"maxTotalChargeUsd":1.0}}}"#;

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
    fn builds_scrappa_url_from_select_values_and_omits_empty_values() {
        let (url, category) = build_trending_url(
            &json!({ "category": [" music "], "type": ["now"] }),
            &base_url(),
        )
        .unwrap();
        assert_eq!(category.as_deref(), Some("music"));
        assert_eq!(
            url.as_str(),
            "https://ytapi.scrappa.co/trending?category=music&type=now"
        );

        let (url, category) = build_trending_url(
            &json!({ "category": [null, 123, "", "  ", "gaming"], "type": [] }),
            &base_url(),
        )
        .unwrap();
        assert_eq!(category.as_deref(), Some("gaming"));
        assert_eq!(
            url.as_str(),
            "https://ytapi.scrappa.co/trending?category=gaming"
        );

        let (url, _) =
            build_trending_url(&json!({ "category": 123, "type": false }), &base_url()).unwrap();
        assert_eq!(url.as_str(), "https://ytapi.scrappa.co/trending");
    }

    #[test]
    fn accepts_scalar_strings_and_url_encodes_query_values() {
        let (url, category) = build_trending_url(
            &json!({ "category": " news & culture ", "type": "now" }),
            &base_url(),
        )
        .unwrap();
        assert_eq!(category.as_deref(), Some("news & culture"));
        assert_eq!(
            url.as_str(),
            "https://ytapi.scrappa.co/trending?category=news+%26+culture&type=now"
        );
    }

    #[test]
    fn rejects_multiple_selected_values() {
        let error = build_trending_url(&json!({ "category": ["music", "gaming"] }), &base_url())
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "category accepts only one selected value."
        );
    }

    #[test]
    fn maps_result_arrays_without_changing_items_or_cardinality() {
        let data = json!({ "results": [{ "id": "one", "extra": true }, { "id": "two" }] });
        assert_eq!(
            trending_videos_to_dataset_items(&data),
            data["results"].as_array().unwrap()
        );

        let legacy = json!({ "results": {}, "videos": [{ "id": "legacy" }] });
        assert_eq!(
            trending_videos_to_dataset_items(&legacy),
            legacy["videos"].as_array().unwrap()
        );
        assert!(
            trending_videos_to_dataset_items(&json!({ "results": {}, "videos": {} })).is_empty()
        );
    }

    #[test]
    fn preserves_nullish_continuation_fallback_semantics() {
        let data = json!({ "continuation": "legacy", "pagination": { "continuationToken": "" } });
        assert_eq!(continuation_token(&data).and_then(Value::as_str), Some(""));

        let data = json!({ "continuation": "legacy", "pagination": { "continuationToken": null } });
        assert_eq!(
            continuation_token(&data).and_then(Value::as_str),
            Some("legacy")
        );
    }

    #[tokio::test]
    async fn fetches_and_stores_exact_items_with_expected_auth_and_headers() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", INPUT_RESPONSE),
            ("200 OK", TWO_VIDEOS_RESPONSE),
            ("200 OK", RUN_NORMAL_BUDGET),
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

        let scrappa_request = requests[1].to_ascii_lowercase();
        assert!(scrappa_request.starts_with("get /trending?category=music&type=now http/1.1"));
        assert!(scrappa_request.contains("accept: application/json"));
        assert!(!scrappa_request.contains("authorization:"));

        let pricing_request = requests[2].to_ascii_lowercase();
        assert!(pricing_request.starts_with("get /v2/actor-runs/run-id http/1.1"));
        assert!(pricing_request.contains("authorization: bearer test-token"));

        let dataset_request = requests[3].to_ascii_lowercase();
        assert!(dataset_request.starts_with("post /v2/datasets/dataset-id/items http/1.1"));
        assert!(dataset_request.contains("authorization: bearer test-token"));
        let body = requests[3].split_once("\r\n\r\n").unwrap().1;
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([
                { "id": "one", "extra": { "preserved": true } },
                { "id": "two" }
            ])
        );
    }

    #[tokio::test]
    async fn one_result_budget_posts_only_the_first_video() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", INPUT_RESPONSE),
            ("200 OK", TWO_VIDEOS_RESPONSE),
            ("200 OK", RUN_ONE_ROW_CAP),
            ("201 Created", ""),
        ]);
        run_actor(&Client::new(), &test_config(base_url))
            .await
            .unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 4);
        let body = requests[3].split_once("\r\n\r\n").unwrap().1;
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{ "id": "one", "extra": { "preserved": true } }])
        );
    }

    #[tokio::test]
    async fn zero_result_budget_skips_the_dataset_post() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", INPUT_RESPONSE),
            ("200 OK", TWO_VIDEOS_RESPONSE),
            ("200 OK", RUN_ZERO_ROW_CAP),
        ]);
        run_actor(&Client::new(), &test_config(base_url))
            .await
            .unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[2]
            .to_ascii_lowercase()
            .starts_with("get /v2/actor-runs/run-id http/1.1"));
    }

    #[tokio::test]
    async fn pricing_lookup_failure_never_posts_dataset_rows() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", INPUT_RESPONSE),
            ("200 OK", TWO_VIDEOS_RESPONSE),
            ("503 Service Unavailable", "pricing unavailable"),
        ]);
        let error = run_actor(&Client::new(), &test_config(base_url))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Apify run pricing request failed"));
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[2]
            .to_ascii_lowercase()
            .starts_with("get /v2/actor-runs/run-id http/1.1"));
    }

    #[tokio::test]
    async fn invalid_pricing_metadata_never_posts_dataset_rows() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", INPUT_RESPONSE),
            ("200 OK", TWO_VIDEOS_RESPONSE),
            (
                "200 OK",
                r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"},"chargedEventCounts":{},"options":{"maxTotalChargeUsd":1.0}}}"#,
            ),
        ]);
        let error = run_actor(&Client::new(), &test_config(base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("did not provide event prices"));
        assert_eq!(server.join().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn missing_spending_limit_never_posts_dataset_rows() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", INPUT_RESPONSE),
            ("200 OK", TWO_VIDEOS_RESPONSE),
            (
                "200 OK",
                r#"{"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{"apify-default-dataset-item":{"eventPriceUsd":0.0001}}}},"chargedEventCounts":{},"options":{}}}"#,
            ),
        ]);
        let error = run_actor(&Client::new(), &test_config(base_url))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("did not provide the spending limit"));
        assert_eq!(server.join().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn dataset_write_failure_is_returned() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", INPUT_RESPONSE),
            ("200 OK", TWO_VIDEOS_RESPONSE),
            ("200 OK", RUN_NORMAL_BUDGET),
            ("503 Service Unavailable", "dataset unavailable"),
        ]);
        let error = run_actor(&Client::new(), &test_config(base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("Apify dataset write failed"));
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[3]
            .to_ascii_lowercase()
            .starts_with("post /v2/datasets/dataset-id/items http/1.1"));
    }

    #[tokio::test]
    async fn returns_upstream_http_failures_without_retrying_or_suppressing_them() {
        let (base_url, server) = mock_server(vec![
            ("200 OK", "{}"),
            ("503 Service Unavailable", "upstream unavailable"),
        ]);
        let client = Client::new();
        let error = run_actor(&client, &test_config(base_url))
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API request failed with 503 Service Unavailable"
        );
        assert_eq!(server.join().unwrap().len(), 2);
    }
}
