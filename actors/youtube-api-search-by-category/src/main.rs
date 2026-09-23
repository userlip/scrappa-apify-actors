use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://ytapi.scrappa.co";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    input_key: String,
    apify_token: String,
    actor_run_id: String,
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
            actor_run_id: required_env("ACTOR_RUN_ID")?,
        })
    }
}

#[derive(Debug)]
struct DatasetBudget {
    max_saved_items: usize,
    saved_items: usize,
}

impl DatasetBudget {
    fn remaining(&self) -> usize {
        self.max_saved_items.saturating_sub(self.saved_items)
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

fn selected_string(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };

    if let Some(values) = value.as_array() {
        let mut selected = None;
        for value in values.iter().filter_map(Value::as_str) {
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
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

fn append_features(features: &mut Vec<String>, value: &str) {
    features.extend(
        value
            .split(',')
            .map(str::trim)
            .filter(|feature| !feature.is_empty())
            .map(str::to_owned),
    );
}

fn feature_values(value: Option<&Value>) -> Vec<String> {
    let mut features = Vec::new();
    match value {
        Some(Value::Array(values)) => {
            for value in values.iter().filter_map(Value::as_str) {
                append_features(&mut features, value);
            }
        }
        Some(Value::String(value)) => append_features(&mut features, value),
        _ => {}
    }
    features
}

fn positive_limit(value: Option<&Value>) -> Result<Option<u64>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };

    let integer = value.as_u64().or_else(|| {
        value.as_f64().and_then(|value| {
            (value.is_finite() && value > 0.0 && value.fract() == 0.0 && value <= u64::MAX as f64)
                .then_some(value as u64)
        })
    });
    match integer.filter(|value| *value > 0) {
        Some(value) => Ok(Some(value)),
        None => bail!("Limit must be a positive integer."),
    }
}

#[derive(Debug)]
struct CategoryRequest {
    url: Url,
    category: String,
}

fn build_category_request(input: &Value, api_base_url: &Url) -> Result<CategoryRequest> {
    let category = selected_string(input.get("category"), "category")?
        .ok_or_else(|| anyhow!("Search category is required."))?;
    let sort =
        selected_string(input.get("sort"), "sort")?.unwrap_or_else(|| "relevance".to_owned());
    let duration = selected_string(input.get("duration"), "duration")?;
    let upload_date = selected_string(input.get("upload_date"), "upload_date")?;
    let continuation = selected_string(input.get("continuation"), "continuation")?;
    let content_type = selected_string(input.get("contentType"), "contentType")?;
    let features = feature_values(input.get("features"));
    let limit = positive_limit(input.get("limit"))?;

    let mut url = endpoint_url(api_base_url, &["search", "category"])?;
    {
        let mut params = url.query_pairs_mut();
        params.append_pair("category", &category);
        params.append_pair("sort", &sort);
        if let Some(duration) = &duration {
            params.append_pair("duration", duration);
        }
        if let Some(upload_date) = &upload_date {
            params.append_pair("upload_date", upload_date);
        }
        if let Some(continuation) = &continuation {
            params.append_pair("continuation", continuation);
        }
        if let Some(content_type) = &content_type {
            params.append_pair("contentType", content_type);
        }
        if !features.is_empty() {
            params.append_pair("features", &features.join(","));
        }
        if let Some(limit) = limit {
            params.append_pair("limit", &limit.to_string());
        }
    }

    Ok(CategoryRequest { url, category })
}

fn category_videos_to_dataset_items(data: &Value) -> &[Value] {
    data.get("results")
        .and_then(Value::as_array)
        .or_else(|| data.get("videos").and_then(Value::as_array))
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn continuation_token(data: &Value) -> Option<&Value> {
    data.get("pagination")
        .and_then(|pagination| pagination.get("continuationToken"))
        .filter(|token| !token.is_null())
        .or_else(|| data.get("continuation").filter(|token| !token.is_null()))
}

fn javascript_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
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

async fn fetch_category(client: &Client, request: &CategoryRequest) -> Result<Value> {
    let response = client
        .get(request.url.clone())
        .header(reqwest::header::ACCEPT, "application/json")
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() || error.to_string().contains("aborted") {
                anyhow!(
                    "Scrappa API request timed out after {}s",
                    SCRAPPA_REQUEST_TIMEOUT.as_secs()
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
        .context("Scrappa API response was not valid JSON")
}

async fn run_dataset_capacity(
    client: &Client,
    config: &ActorConfig,
    requested: usize,
) -> Result<DatasetBudget> {
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
    Ok(DatasetBudget {
        max_saved_items: affordable_dataset_items(&run, requested)?,
        saved_items: 0,
    })
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

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    items: &[Value],
    budget: &mut DatasetBudget,
) -> Result<()> {
    let item_count = items.len().min(budget.remaining());
    if item_count == 0 {
        return Ok(());
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(&items[..item_count])
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
    budget.saved_items += item_count;
    Ok(())
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let request = build_category_request(&input, &config.scrappa_api_base_url)?;
    println!("Fetching from: {}", request.url);

    let data = fetch_category(client, &request).await?;
    let items = category_videos_to_dataset_items(&data);
    if !items.is_empty() {
        let mut budget = run_dataset_capacity(client, config, items.len()).await?;
        push_dataset_items(client, config, items, &mut budget).await?;
    }
    println!(
        "Successfully fetched {} category video(s) for category: {}",
        items.len(),
        request.category
    );

    if let Some(token) = continuation_token(&data).filter(|token| javascript_truthy(token)) {
        let token = token
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| token.to_string());
        println!("Continuation token available for next page: {token}");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube category search results: {error:#}");
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
    use serde_json::json;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    const INPUT_SCHEMA: &str = include_str!("../.actor/input_schema.json");

    #[derive(Debug)]
    struct MockRequest {
        line: String,
        headers: String,
        body: String,
    }

    fn mock_server(replies: Vec<String>) -> (Url, thread::JoinHandle<Vec<MockRequest>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let base_url = Url::parse(&format!("http://{address}")).unwrap();
        let server = thread::spawn(move || {
            let mut requests = Vec::with_capacity(replies.len());
            for reply in replies {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                requests.push(read_request(&stream));
                stream.write_all(reply.as_bytes()).unwrap();
            }
            requests
        });
        (base_url, server)
    }

    fn read_request(stream: &TcpStream) -> MockRequest {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let line = line.trim_end().to_owned();
        let mut headers = String::new();
        let mut content_length = 0;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).unwrap();
            if header == "\r\n" || header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.trim().parse().unwrap();
                }
            }
            headers.push_str(&header.to_ascii_lowercase());
        }
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).unwrap();
        MockRequest {
            line,
            headers,
            body: String::from_utf8(body).unwrap(),
        }
    }

    fn http_reply(status: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    fn test_config(api_base_url: Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: api_base_url.clone(),
            scrappa_api_base_url: api_base_url,
            default_key_value_store_id: "store-id".to_owned(),
            default_dataset_id: "dataset-id".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token".to_owned(),
            actor_run_id: "run-id".to_owned(),
        }
    }

    fn pricing_metadata(max_total_charge_usd: f64) -> String {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            "scrappa-lookup": {"eventPriceUsd": 0.00015}
                        }
                    }
                },
                "chargedEventCounts": {"scrappa-lookup": 2},
                "options": {"maxTotalChargeUsd": max_total_charge_usd}
            }
        })
        .to_string()
    }

    fn query_value(url: &Url, key: &str) -> Option<String> {
        url.query_pairs()
            .find_map(|(name, value)| (name == key).then(|| value.into_owned()))
    }

    #[test]
    fn schema_prefills_match_request_defaults() {
        let schema: Value = serde_json::from_str(INPUT_SCHEMA).unwrap();
        let properties = schema.get("properties").unwrap();
        let mut input = json!({});
        for field in ["category", "sort", "duration", "upload_date"] {
            let prefill = properties[field].get("prefill").unwrap().clone();
            assert_eq!(prefill.as_array().unwrap().len(), 1);
            input[field] = prefill;
        }

        let api_base_url = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let request = build_category_request(&input, &api_base_url).unwrap();
        assert_eq!(
            query_value(&request.url, "category").as_deref(),
            Some("education")
        );
        assert_eq!(
            query_value(&request.url, "sort").as_deref(),
            Some("relevance")
        );
        assert_eq!(
            query_value(&request.url, "duration").as_deref(),
            Some("short")
        );
        assert_eq!(
            query_value(&request.url, "upload_date").as_deref(),
            Some("hour")
        );
    }

    #[test]
    fn maps_category_filters_and_url_encodes_values() {
        let input = json!({
            "category": [" music & arts "],
            "sort": [null, "view_count"],
            "duration": ["short"],
            "upload_date": ["week"],
            "limit": 1024,
            "continuation": "next/page+token",
            "contentType": ["live"],
            "features": "hd, cc"
        });
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let request = build_category_request(&input, &base).unwrap();

        assert_eq!(request.url.path(), "/search/category");
        assert_eq!(
            query_value(&request.url, "category").as_deref(),
            Some("music & arts")
        );
        assert_eq!(
            query_value(&request.url, "sort").as_deref(),
            Some("view_count")
        );
        assert_eq!(
            query_value(&request.url, "duration").as_deref(),
            Some("short")
        );
        assert_eq!(
            query_value(&request.url, "upload_date").as_deref(),
            Some("week")
        );
        assert_eq!(query_value(&request.url, "limit").as_deref(), Some("1024"));
        assert_eq!(
            query_value(&request.url, "continuation").as_deref(),
            Some("next/page+token")
        );
        assert_eq!(
            query_value(&request.url, "contentType").as_deref(),
            Some("live")
        );
        assert_eq!(
            query_value(&request.url, "features").as_deref(),
            Some("hd,cc")
        );
        assert!(request.url.as_str().contains("music+%26+arts"));
        assert!(request.url.as_str().contains("next%2Fpage%2Btoken"));
    }

    #[test]
    fn normalizes_scalar_empty_optional_and_feature_inputs() {
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let request = build_category_request(
            &json!({
                "category": "music",
                "sort": "rating",
                "duration": [],
                "upload_date": " ",
                "continuation": "",
                "contentType": [null, ""],
                "features": [" hd,cc ", "", " 4k "]
            }),
            &base,
        )
        .unwrap();
        assert_eq!(
            query_value(&request.url, "category").as_deref(),
            Some("music")
        );
        assert_eq!(query_value(&request.url, "sort").as_deref(), Some("rating"));
        assert_eq!(
            query_value(&request.url, "features").as_deref(),
            Some("hd,cc,4k")
        );
        for key in ["duration", "upload_date", "continuation", "contentType"] {
            assert_eq!(query_value(&request.url, key), None);
        }
    }

    #[test]
    fn rejects_missing_or_multi_selected_values_and_invalid_limits() {
        let base = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        assert!(build_category_request(&json!({"category": [""]}), &base)
            .unwrap_err()
            .to_string()
            .contains("Search category is required"));
        assert!(
            build_category_request(&json!({"category": ["music", "gaming"]}), &base)
                .unwrap_err()
                .to_string()
                .contains("category accepts only one selected value")
        );
        assert!(build_category_request(
            &json!({"category": ["music"], "sort": ["rating", "relevance"]}),
            &base
        )
        .unwrap_err()
        .to_string()
        .contains("sort accepts only one selected value"));
        for limit in [json!("5"), json!(0), json!(1.5)] {
            assert!(
                build_category_request(&json!({"category": "music", "limit": limit}), &base)
                    .unwrap_err()
                    .to_string()
                    .contains("Limit must be a positive integer")
            );
        }
    }

    #[test]
    fn maps_all_result_rows_and_legacy_fallback() {
        let rows = vec![json!({"id":"one"}), json!(null), json!({"id":"two"})];
        assert_eq!(
            category_videos_to_dataset_items(&json!({"results": rows})).len(),
            3
        );
        assert_eq!(
            category_videos_to_dataset_items(&json!({"results": false, "videos": [{"id":"old"}]})),
            &[json!({"id":"old"})]
        );
        assert!(category_videos_to_dataset_items(&json!({})).is_empty());
    }

    #[test]
    fn reads_continuation_from_current_and_legacy_locations() {
        assert_eq!(
            continuation_token(&json!({"pagination":{"continuationToken":"next"}}))
                .and_then(Value::as_str),
            Some("next")
        );
        assert_eq!(
            continuation_token(&json!({"continuation":"legacy"})).and_then(Value::as_str),
            Some("legacy")
        );
        assert_eq!(
            continuation_token(
                &json!({"pagination":{"continuationToken":""},"continuation":"legacy"})
            )
            .and_then(Value::as_str),
            Some("")
        );
    }

    #[tokio::test]
    async fn fetches_input_scrappa_results_and_writes_exact_dataset_rows() {
        let input = r#"{"category":["education"],"sort":["view_count"],"limit":5,"continuation":"next-page","features":"hd, cc"}"#;
        let results = r#"{"results":[{"id":"first"},{"id":"second"}],"pagination":{"continuationToken":"next"}}"#;
        let (base_url, server) = mock_server(vec![
            http_reply("200 OK", input),
            http_reply("200 OK", results),
            http_reply("200 OK", &pricing_metadata(1.0)),
            http_reply("201 Created", ""),
        ]);
        let client = Client::builder().build().unwrap();
        run_actor(&client, &test_config(base_url.clone()))
            .await
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 4);
        assert!(requests[0].line.starts_with("GET /v2/key-value-stores/"));
        assert!(requests[0].line.ends_with("/records/INPUT HTTP/1.1"));
        assert!(requests[0]
            .headers
            .contains("authorization: bearer test-token"));
        let scrappa_target = requests[1].line.split_whitespace().nth(1).unwrap();
        let scrappa_url = base_url.join(scrappa_target).unwrap();
        assert_eq!(scrappa_url.path(), "/search/category");
        assert_eq!(
            query_value(&scrappa_url, "category").as_deref(),
            Some("education")
        );
        assert_eq!(
            query_value(&scrappa_url, "sort").as_deref(),
            Some("view_count")
        );
        assert_eq!(query_value(&scrappa_url, "limit").as_deref(), Some("5"));
        assert_eq!(
            query_value(&scrappa_url, "continuation").as_deref(),
            Some("next-page")
        );
        assert_eq!(
            query_value(&scrappa_url, "features").as_deref(),
            Some("hd,cc")
        );
        assert!(requests[1].headers.contains("accept: application/json"));
        assert!(!requests[1].headers.contains("authorization:"));
        assert!(requests[2]
            .line
            .starts_with("GET /v2/actor-runs/run-id HTTP/1.1"));
        assert!(requests[2]
            .headers
            .contains("authorization: bearer test-token"));
        assert!(requests[3]
            .line
            .starts_with("POST /v2/datasets/dataset-id/items HTTP/1.1"));
        assert!(requests[3]
            .headers
            .contains("authorization: bearer test-token"));
        assert_eq!(
            serde_json::from_str::<Value>(&requests[3].body).unwrap(),
            json!([{"id":"first"},{"id":"second"}])
        );
    }

    #[tokio::test]
    async fn trims_two_category_results_to_the_affordable_budget() {
        let input = r#"{"category":["education"]}"#;
        let results = r#"{"results":[{"id":"first"},{"id":"second"}]}"#;
        let (base_url, server) = mock_server(vec![
            http_reply("200 OK", input),
            http_reply("200 OK", results),
            http_reply("200 OK", &pricing_metadata(0.0006)),
            http_reply("201 Created", ""),
        ]);
        let client = Client::builder().build().unwrap();
        run_actor(&client, &test_config(base_url)).await.unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 4);
        assert_eq!(
            serde_json::from_str::<Value>(&requests[3].body).unwrap(),
            json!([{"id":"first"}])
        );
    }

    #[tokio::test]
    async fn zero_total_charge_cap_skips_the_dataset_post() {
        let input = r#"{"category":["education"]}"#;
        let results = r#"{"results":[{"id":"first"},{"id":"second"}]}"#;
        let mut run: Value = serde_json::from_str(&pricing_metadata(0.0)).unwrap();
        run["data"]["chargedEventCounts"] = json!({});
        let (base_url, server) = mock_server(vec![
            http_reply("200 OK", input),
            http_reply("200 OK", results),
            http_reply("200 OK", &run.to_string()),
        ]);
        let client = Client::builder().build().unwrap();
        run_actor(&client, &test_config(base_url)).await.unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 3);
        assert!(requests[2]
            .line
            .starts_with("GET /v2/actor-runs/run-id HTTP/1.1"));
    }

    #[tokio::test]
    async fn missing_spending_limit_fails_closed_before_dataset_post() {
        let input = r#"{"category":["education"]}"#;
        let results = r#"{"results":[{"id":"first"},{"id":"second"}]}"#;
        let mut run: Value = serde_json::from_str(&pricing_metadata(1.0)).unwrap();
        run["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
        let (base_url, server) = mock_server(vec![
            http_reply("200 OK", input),
            http_reply("200 OK", results),
            http_reply("200 OK", &run.to_string()),
        ]);
        let client = Client::builder().build().unwrap();
        let error = run_actor(&client, &test_config(base_url))
            .await
            .unwrap_err();
        let requests = server.join().unwrap();

        assert!(error
            .to_string()
            .contains("did not provide the spending limit"));
        assert_eq!(requests.len(), 3);
    }

    #[tokio::test]
    async fn local_saved_count_prevents_overspend_across_dataset_writes() {
        let (base_url, server) = mock_server(vec![
            http_reply("200 OK", &pricing_metadata(0.0006)),
            http_reply("201 Created", ""),
        ]);
        let client = Client::builder().build().unwrap();
        let config = test_config(base_url);
        let mut budget = run_dataset_capacity(&client, &config, 2).await.unwrap();
        push_dataset_items(&client, &config, &[json!({"id":"first"})], &mut budget)
            .await
            .unwrap();
        push_dataset_items(&client, &config, &[json!({"id":"second"})], &mut budget)
            .await
            .unwrap();
        let requests = server.join().unwrap();

        assert_eq!(requests.len(), 2);
        assert!(requests[1]
            .line
            .starts_with("POST /v2/datasets/dataset-id/items HTTP/1.1"));
        assert_eq!(
            serde_json::from_str::<Value>(&requests[1].body).unwrap(),
            json!([{"id":"first"}])
        );
    }

    #[tokio::test]
    async fn dataset_write_errors_are_returned() {
        let input = r#"{"category":["education"]}"#;
        let results = r#"{"results":[{"id":"first"},{"id":"second"}]}"#;
        let (base_url, server) = mock_server(vec![
            http_reply("200 OK", input),
            http_reply("200 OK", results),
            http_reply("200 OK", &pricing_metadata(1.0)),
            http_reply("503 Service Unavailable", "storage unavailable"),
        ]);
        let client = Client::builder().build().unwrap();
        let error = run_actor(&client, &test_config(base_url))
            .await
            .unwrap_err();
        let requests = server.join().unwrap();

        assert!(error
            .to_string()
            .contains("Apify dataset write failed with 503"));
        assert_eq!(requests.len(), 4);
        assert_eq!(
            serde_json::from_str::<Value>(&requests[3].body).unwrap(),
            json!([{"id":"first"},{"id":"second"}])
        );
    }

    #[tokio::test]
    async fn surfaces_upstream_http_errors_without_changing_the_status_message() {
        let input = r#"{"category":["education"]}"#;
        let (base_url, server) = mock_server(vec![
            http_reply("200 OK", input),
            http_reply("503 Service Unavailable", "upstream detail"),
        ]);
        let client = Client::builder().build().unwrap();
        let error = run_actor(&client, &test_config(base_url))
            .await
            .unwrap_err();
        server.join().unwrap();
        assert_eq!(
            error.to_string(),
            "Scrappa API request failed with 503 Service Unavailable"
        );
    }
}
