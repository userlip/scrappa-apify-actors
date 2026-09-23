use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Datelike, SecondsFormat, TimeZone, Timelike, Utc};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{
    env,
    time::{Duration, SystemTime},
};
use tokio::time::timeout;
use url::{form_urlencoded, Url};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/youtube/search";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_MAX_LIMIT: f64 = 20.0;

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
    scrappa_api_key: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = match env::var("SCRAPPA_API_KEY") {
            Ok(value) if !value.is_empty() => value,
            _ => bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."),
        };

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
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
        .extend(segments.iter().copied());
    Ok(url)
}

fn nonblank_string(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn days_in_utc_month(year: i32, month: u32) -> u32 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn subtract_utc_months(date: DateTime<Utc>, months: i32) -> DateTime<Utc> {
    let month_index = i64::from(date.year()) * 12 + i64::from(date.month0()) - i64::from(months);
    let year = month_index.div_euclid(12) as i32;
    let month = month_index.rem_euclid(12) as u32 + 1;
    let day = date.day().min(days_in_utc_month(year, month));
    Utc.with_ymd_and_hms(year, month, day, date.hour(), date.minute(), date.second())
        .single()
        .expect("valid UTC calendar date")
        + chrono::Duration::nanoseconds(i64::from(date.nanosecond()))
}

fn published_after(upload_date: &str, now: DateTime<Utc>) -> Option<String> {
    let date = match upload_date {
        "hour" => now - chrono::Duration::hours(1),
        "today" => now - chrono::Duration::days(1),
        "week" => now - chrono::Duration::days(7),
        "month" => subtract_utc_months(now, 1),
        "year" => subtract_utc_months(now, 12),
        _ => return None,
    };
    Some(date.to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn build_hashtag_search_url(
    input: &Value,
    api_base_url: &Url,
    now: DateTime<Utc>,
) -> Result<(Url, String)> {
    let hashtag = input
        .get("hashtag")
        .and_then(Value::as_str)
        .filter(|hashtag| !hashtag.trim().is_empty())
        .ok_or_else(|| anyhow!("Search query \"hashtag\" not provided. Please provide a value for \"hashtag\" in the input."))?;

    for filter_name in ["contentType", "features"] {
        if nonblank_string(input.get(filter_name)).is_some() {
            bail!("The \"{filter_name}\" filter is not supported by the Scrappa YouTube search endpoint.");
        }
    }

    let normalized_hashtag = if hashtag.starts_with('#') {
        hashtag.to_owned()
    } else {
        format!("#{hashtag}")
    };
    let mut params = form_urlencoded::Serializer::new(String::new());
    params.append_pair("query", &normalized_hashtag);
    params.append_pair("type", "video");

    let sort = match input.get("sort") {
        None => Some("relevance"),
        value => nonblank_string(value),
    };
    if let Some(sort) = sort {
        params.append_pair(
            "order",
            match sort {
                "upload_date" => "date",
                "view_count" => "viewCount",
                other => other,
            },
        );
    }

    if let Some(limit) = input
        .get("limit")
        .and_then(Value::as_f64)
        .filter(|limit| limit.is_finite() && *limit > 0.0 && limit.fract() == 0.0)
    {
        params.append_pair("limit", &(limit.min(SCRAPPA_MAX_LIMIT) as u64).to_string());
    }

    if let Some(duration) = nonblank_string(input.get("duration")) {
        params.append_pair("videoDuration", duration);
    }
    if let Some(upload_date) = nonblank_string(input.get("upload_date")) {
        if let Some(published_after) = published_after(upload_date, now) {
            params.append_pair("publishedAfter", &published_after);
        }
    }
    if let Some(continuation) = nonblank_string(input.get("continuation")) {
        params.append_pair("continuation", continuation);
    }

    let mut url = api_base_url.clone();
    url.set_query(Some(&params.finish()));
    Ok((url, hashtag.to_owned()))
}

fn continuation_token(data: &Value) -> Option<&Value> {
    data.get("continuation")
        .filter(|token| !token.is_null())
        .or_else(|| {
            data.get("pagination")
                .and_then(|pagination| pagination.get("continuationToken"))
                .filter(|token| !token.is_null())
        })
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

async fn fetch_hashtag(client: &Client, url: &Url, api_key: &str) -> Result<Value> {
    let request = async {
        let response = client
            .get(url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .header("X-API-Key", api_key)
            .send()
            .await
            .map_err(scrappa_request_error)?;
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
                "Scrappa API request failed with {} {reason}{detail}",
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

fn hashtag_results(data: &Value) -> Result<&Value> {
    let results = data
        .get("results")
        .ok_or_else(|| anyhow!("Scrappa API response did not contain results"))?;
    if results.is_array() || results.is_object() {
        Ok(results)
    } else {
        bail!("Scrappa API results must be a JSON object or array")
    }
}

fn result_count(results: &Value) -> usize {
    results.as_array().map_or(1, Vec::len)
}

async fn run_dataset_capacity(
    client: &Client,
    config: &ActorConfig,
    requested: usize,
) -> Result<usize> {
    if requested == 0 {
        return Ok(0);
    }
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

struct DatasetWriteBudget {
    capacity: usize,
    saved_rows: usize,
}

impl DatasetWriteBudget {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            saved_rows: 0,
        }
    }

    fn remaining(&self) -> usize {
        self.capacity.saturating_sub(self.saved_rows)
    }
}

async fn push_dataset_results(
    client: &Client,
    config: &ActorConfig,
    results: &Value,
    budget: &mut DatasetWriteBudget,
) -> Result<()> {
    let row_count = result_count(results).min(budget.remaining());
    if row_count == 0 {
        return Ok(());
    }
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let request = if let Some(rows) = results.as_array() {
        client
            .post(url.clone())
            .bearer_auth(&config.apify_token)
            .json(&rows[..row_count])
    } else {
        client
            .post(url)
            .bearer_auth(&config.apify_token)
            .json(results)
    };
    let response = request.send().await.context("Apify dataset write failed")?;
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
    budget.saved_rows += row_count;
    Ok(())
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let (url, hashtag) = build_hashtag_search_url(
        &input,
        &config.scrappa_api_base_url,
        DateTime::<Utc>::from(SystemTime::now()),
    )?;
    println!("Fetching from: {url}");

    let data = fetch_hashtag(client, &url, &config.scrappa_api_key).await?;
    let results = hashtag_results(&data)?;
    let mut budget =
        DatasetWriteBudget::new(run_dataset_capacity(client, config, result_count(results)).await?);
    push_dataset_results(client, config, results, &mut budget).await?;
    println!(
        "Successfully fetched {} hashtag results for query: {hashtag}",
        result_count(results)
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
        eprintln!("Failed to fetch YouTube hashtag results: {error:#}");
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

    const NORMAL_PRICING_RUN: &str = r#"{
        "data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                "apify-actor-start": {"eventPriceUsd": 0.00005}
            }}},
            "options": {"maxTotalChargeUsd": 1},
            "chargedEventCounts": {"apify-actor-start": 1}
        }
    }"#;
    const ONE_ROW_PRICING_RUN: &str = r#"{
        "data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                "apify-actor-start": {"eventPriceUsd": 0.00005}
            }}},
            "options": {"maxTotalChargeUsd": 0.00015},
            "chargedEventCounts": {"apify-actor-start": 1}
        }
    }"#;
    const ZERO_ROW_PRICING_RUN: &str = r#"{
        "data": {
            "pricingInfo": {"pricingModel": "PAY_PER_EVENT", "pricingPerEvent": {"actorChargeEvents": {
                "apify-default-dataset-item": {"eventPriceUsd": 0.0001}
            }}},
            "options": {"maxTotalChargeUsd": 0},
            "chargedEventCounts": {}
        }
    }"#;

    fn base_url() -> Url {
        Url::parse("https://scrappa.co/api/youtube/search").unwrap()
    }

    fn utc(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
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

    fn test_config(server_url: Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: server_url.clone(),
            scrappa_api_base_url: server_url.join("search").unwrap(),
            default_key_value_store_id: "store-id".to_owned(),
            default_dataset_id: "dataset-id".to_owned(),
            actor_run_id: "run-id".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token".to_owned(),
            scrappa_api_key: "scrappa-secret".to_owned(),
        }
    }

    #[test]
    fn preserves_schema_prefills_and_limit_bounds() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let properties = &schema["properties"];
        assert_eq!(properties["sort"]["prefill"], "relevance");
        assert_eq!(properties["duration"]["prefill"], "short");
        assert_eq!(properties["upload_date"]["prefill"], "hour");
        assert_eq!(properties["limit"]["default"], 20);
        assert_eq!(properties["limit"]["minimum"], 1);
        assert_eq!(properties["limit"]["maximum"], 20);
    }

    #[test]
    fn encodes_hashtag_query_and_preserves_parameter_order() {
        let (url, hashtag) = build_hashtag_search_url(
            &json!({
                "hashtag": "rust & systems",
                "sort": "view_count",
                "limit": 50,
                "continuation": "next page"
            }),
            &base_url(),
            utc("2026-05-27T12:00:00.000Z"),
        )
        .unwrap();
        assert_eq!(hashtag, "rust & systems");
        assert_eq!(
            url.as_str(),
            "https://scrappa.co/api/youtube/search?query=%23rust+%26+systems&type=video&order=viewCount&limit=20&continuation=next+page"
        );
    }

    #[test]
    fn blank_hashtag_names_the_actual_input_field() {
        let error = build_hashtag_search_url(
            &json!({"hashtag": "   "}),
            &base_url(),
            utc("2026-05-27T12:00:00.000Z"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("\"hashtag\""));
        assert!(!error.to_string().contains("searchHashtag"));
    }

    #[test]
    fn handles_existing_hash_unicode_and_url_reserved_characters() {
        let (url, _) = build_hashtag_search_url(
            &json!({ "hashtag": "#café/世界?" }),
            &base_url(),
            utc("2026-05-27T12:00:00.000Z"),
        )
        .unwrap();
        assert_eq!(
            url.query(),
            Some("query=%23caf%C3%A9%2F%E4%B8%96%E7%95%8C%3F&type=video&order=relevance")
        );

        let (url, _) = build_hashtag_search_url(
            &json!({ "hashtag": "##double" }),
            &base_url(),
            utc("2026-05-27T12:00:00.000Z"),
        )
        .unwrap();
        assert_eq!(
            url.query_pairs().find(|(key, _)| key == "query").unwrap().1,
            "##double"
        );
    }

    #[test]
    fn maps_sort_aliases_and_passes_through_other_sort_values() {
        for (sort, expected) in [
            ("upload_date", "date"),
            ("view_count", "viewCount"),
            ("rating", "rating"),
        ] {
            let (url, _) = build_hashtag_search_url(
                &json!({ "hashtag": "rust", "sort": sort }),
                &base_url(),
                utc("2026-05-27T12:00:00.000Z"),
            )
            .unwrap();
            assert_eq!(
                url.query_pairs().find(|(key, _)| key == "order").unwrap().1,
                expected
            );
        }
    }

    #[test]
    fn caps_only_positive_integer_limits_at_scrappa_maximum() {
        for (limit, expected) in [
            (json!(50), Some("20")),
            (json!(1.0), Some("1")),
            (json!(1.5), None),
            (json!(0), None),
            (json!(-1), None),
            (json!("50"), None),
        ] {
            let (url, _) = build_hashtag_search_url(
                &json!({ "hashtag": "rust", "limit": limit }),
                &base_url(),
                utc("2026-05-27T12:00:00.000Z"),
            )
            .unwrap();
            assert_eq!(
                url.query_pairs()
                    .find(|(key, _)| key == "limit")
                    .map(|(_, value)| value.to_string())
                    .as_deref(),
                expected
            );
        }
    }

    #[test]
    fn maps_upload_date_filters_using_utc_and_clamps_month_edges() {
        let now = utc("2026-05-27T12:34:56.789Z");
        for (filter, expected) in [
            ("hour", "2026-05-27T11:34:56.789Z"),
            ("today", "2026-05-26T12:34:56.789Z"),
            ("week", "2026-05-20T12:34:56.789Z"),
            ("month", "2026-04-27T12:34:56.789Z"),
            ("year", "2025-05-27T12:34:56.789Z"),
        ] {
            assert_eq!(published_after(filter, now).as_deref(), Some(expected));
        }
        assert_eq!(published_after("unknown", now), None);
        assert_eq!(
            published_after("month", utc("2024-03-31T12:00:00.000Z")).as_deref(),
            Some("2024-02-29T12:00:00.000Z")
        );
        assert_eq!(
            published_after("year", utc("2024-02-29T12:00:00.000Z")).as_deref(),
            Some("2023-02-28T12:00:00.000Z")
        );
    }

    #[test]
    fn ignores_blank_optional_strings_and_rejects_unsupported_filters() {
        let (url, _) = build_hashtag_search_url(
            &json!({ "hashtag": "rust", "sort": "  ", "duration": "  ", "upload_date": "  ", "continuation": "  " }),
            &base_url(),
            utc("2026-05-27T12:00:00.000Z"),
        )
        .unwrap();
        assert_eq!(url.query(), Some("query=%23rust&type=video"));

        for (name, value) in [("contentType", "live"), ("features", "hd,subtitles")] {
            let mut input = json!({ "hashtag": "rust" });
            input[name] = json!(value);
            assert!(
                build_hashtag_search_url(&input, &base_url(), utc("2026-05-27T12:00:00.000Z"))
                    .unwrap_err()
                    .to_string()
                    .contains(name)
            );
        }
        assert!(build_hashtag_search_url(
            &json!({ "hashtag": "rust", "contentType": "  ", "features": 4 }),
            &base_url(),
            utc("2026-05-27T12:00:00.000Z")
        )
        .is_ok());
    }

    #[test]
    fn errors_when_hashtag_is_missing_or_empty() {
        for input in [
            json!({}),
            json!({ "hashtag": "" }),
            json!({ "hashtag": null }),
        ] {
            assert!(
                build_hashtag_search_url(&input, &base_url(), utc("2026-05-27T12:00:00.000Z"))
                    .unwrap_err()
                    .to_string()
                    .contains("Search query \"hashtag\" not provided")
            );
        }
    }

    #[test]
    fn preserves_result_rows_and_empty_dataset_behavior() {
        let data = json!({ "results": [{ "id": "one", "extra": true }, { "id": "two" }] });
        let results = hashtag_results(&data).unwrap();
        assert_eq!(results, &data["results"]);
        assert_eq!(result_count(results), 2);
        assert!(hashtag_results(&json!({ "results": [] }))
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(
            result_count(hashtag_results(&json!({ "results": { "id": "one" } })).unwrap()),
            1
        );
        assert!(hashtag_results(&json!({ "results": null })).is_err());
        assert!(hashtag_results(&json!({})).is_err());
    }

    #[tokio::test]
    async fn fetches_with_scrappa_key_and_writes_exact_dataset_rows_with_apify_auth() {
        let (server_url, server) = mock_server(vec![
            (
                "200 OK",
                r#"{"hashtag":"rust & systems","sort":"view_count","limit":50,"continuation":"next page"}"#,
            ),
            (
                "200 OK",
                r#"{"results":[{"id":"one","extra":{"preserved":true}},{"id":"two"}],"pagination":{"continuationToken":"next"}}"#,
            ),
            ("200 OK", NORMAL_PRICING_RUN),
            ("201 Created", ""),
        ]);
        run_actor(&Client::new(), &test_config(server_url))
            .await
            .unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 4);

        let input_request = requests[0].to_ascii_lowercase();
        assert!(
            input_request.starts_with("get /v2/key-value-stores/store-id/records/input http/1.1")
        );
        assert!(input_request.contains("authorization: bearer test-token"));

        let scrappa_request = requests[1].to_ascii_lowercase();
        assert!(scrappa_request.starts_with(
            "get /search?query=%23rust+%26+systems&type=video&order=viewcount&limit=20&continuation=next+page http/1.1"
        ));
        assert!(scrappa_request.contains("x-api-key: scrappa-secret"));
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
    async fn one_result_budget_posts_only_the_first_of_two_rows() {
        let (server_url, server) = mock_server(vec![
            ("200 OK", r#"{"hashtag":"rust"}"#),
            ("200 OK", r#"{"results":[{"id":"one"},{"id":"two"}]}"#),
            ("200 OK", ONE_ROW_PRICING_RUN),
            ("201 Created", ""),
        ]);
        run_actor(&Client::new(), &test_config(server_url))
            .await
            .unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 4);
        let body = requests[3].split_once("\r\n\r\n").unwrap().1;
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{ "id": "one" }])
        );
    }

    #[tokio::test]
    async fn zero_result_budget_skips_dataset_post() {
        let (server_url, server) = mock_server(vec![
            ("200 OK", r#"{"hashtag":"rust"}"#),
            ("200 OK", r#"{"results":[{"id":"one"},{"id":"two"}]}"#),
            ("200 OK", ZERO_ROW_PRICING_RUN),
        ]);
        run_actor(&Client::new(), &test_config(server_url))
            .await
            .unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests.iter().all(|request| !request
            .to_ascii_lowercase()
            .starts_with("post /v2/datasets/")));
    }

    #[tokio::test]
    async fn pricing_lookup_error_fails_before_any_dataset_write() {
        let (server_url, server) = mock_server(vec![
            ("200 OK", r#"{"hashtag":"rust"}"#),
            ("200 OK", r#"{"results":[{"id":"one"}]}"#),
            ("503 Service Unavailable", "pricing unavailable"),
        ]);
        let error = run_actor(&Client::new(), &test_config(server_url))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Apify run pricing request failed with 503"));
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests.iter().all(|request| !request
            .to_ascii_lowercase()
            .starts_with("post /v2/datasets/")));
    }

    #[tokio::test]
    async fn dataset_write_error_fails_the_actor() {
        let (server_url, server) = mock_server(vec![
            ("200 OK", r#"{"hashtag":"rust"}"#),
            ("200 OK", r#"{"results":[{"id":"one"}]}"#),
            ("200 OK", NORMAL_PRICING_RUN),
            ("503 Service Unavailable", "dataset unavailable"),
        ]);
        let error = run_actor(&Client::new(), &test_config(server_url))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Apify dataset write failed with 503"));
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[3]
            .to_ascii_lowercase()
            .starts_with("post /v2/datasets/dataset-id/items"));
    }

    #[tokio::test]
    async fn local_saved_row_count_prevents_overspending_across_writes() {
        let (server_url, server) = mock_server(vec![("201 Created", "")]);
        let config = test_config(server_url);
        let mut budget = DatasetWriteBudget::new(1);
        let client = Client::new();
        push_dataset_results(&client, &config, &json!([{ "id": "one" }]), &mut budget)
            .await
            .unwrap();
        push_dataset_results(&client, &config, &json!([{ "id": "two" }]), &mut budget)
            .await
            .unwrap();
        assert_eq!(budget.saved_rows, 1);
        assert_eq!(budget.remaining(), 0);
        assert_eq!(server.join().unwrap().len(), 1);
    }

    #[test]
    fn charged_events_reduce_capacity_and_invalid_pricing_fails_closed() {
        let run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                        "apify-actor-start": {"eventPriceUsd": 0.00005}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.0003},
                "chargedEventCounts": {
                    "apify-default-dataset-item": 1,
                    "apify-actor-start": 1
                }
            }
        });
        assert_eq!(affordable_dataset_items(&run, 2).unwrap(), 1);
        assert!(affordable_dataset_items(&json!({"data": {}}), 1).is_err());
        let mut missing_limit = run.clone();
        assert!(missing_limit["data"]["options"]
            .as_object_mut()
            .unwrap()
            .remove("maxTotalChargeUsd")
            .is_some());
        assert!(affordable_dataset_items(&missing_limit, 1).is_err());
        let mut missing_counts = run.clone();
        missing_counts["data"]
            .as_object_mut()
            .unwrap()
            .remove("chargedEventCounts");
        assert!(affordable_dataset_items(&missing_counts, 1).is_err());
        let mut null_limit = run;
        null_limit["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
        assert!(affordable_dataset_items(&null_limit, 1).is_err());
    }

    #[tokio::test]
    async fn propagates_scrappa_http_errors_without_dataset_output() {
        let (server_url, server) = mock_server(vec![
            ("200 OK", r#"{"hashtag":"rust"}"#),
            ("503 Service Unavailable", "upstream unavailable"),
        ]);
        let error = run_actor(&Client::new(), &test_config(server_url))
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API request failed with 503 Service Unavailable: upstream unavailable"
        );
        assert_eq!(server.join().unwrap().len(), 2);
    }
}
