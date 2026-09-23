use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Datelike, Duration as ChronoDuration, TimeZone, Timelike, Utc};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/youtube/search";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_LIMIT: u64 = 20;

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
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|api_key| !api_key.is_empty())
            .ok_or_else(|| anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."))?;
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
        })
    }
}

#[derive(Debug)]
struct SearchRequest {
    url: Url,
    query: String,
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

fn single_value(value: Option<&Value>) -> Option<&str> {
    let value = match value? {
        Value::Array(values) => values.iter().find(|value| !value.is_null())?,
        value => value,
    };
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn positive_limit(value: Option<&Value>) -> Option<u64> {
    let number = value?.as_number()?;
    if let Some(integer) = number.as_u64() {
        return (integer > 0).then_some(integer.min(MAX_LIMIT));
    }

    let number = number.as_f64()?;
    (number.is_finite() && number > 0.0 && number.fract() == 0.0)
        .then_some((number.min(MAX_LIMIT as f64)) as u64)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if year.rem_euclid(400) == 0
            || (year.rem_euclid(4) == 0 && year.rem_euclid(100) != 0) =>
        {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn subtract_utc_months(date: DateTime<Utc>, months: u32) -> DateTime<Utc> {
    let total_months = i64::from(date.year()) * 12 + i64::from(date.month0()) - i64::from(months);
    let year = total_months.div_euclid(12) as i32;
    let month = total_months.rem_euclid(12) as u32 + 1;
    let day = date.day().min(days_in_month(year, month));
    let (hour, minute, second, millisecond) = (
        date.hour(),
        date.minute(),
        date.second(),
        date.timestamp_subsec_millis(),
    );
    let naive = chrono::NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|date| date.and_hms_milli_opt(hour, minute, second, millisecond));
    // The target year and day are derived from a valid current UTC date.
    DateTime::from_naive_utc_and_offset(naive.expect("valid calendar month subtraction"), Utc)
}

fn published_after(upload_date: Option<&str>, now: DateTime<Utc>) -> Option<String> {
    let now = Utc.timestamp_millis_opt(now.timestamp_millis()).single()?;
    let threshold = match upload_date? {
        "hour" => now.clone() - ChronoDuration::hours(1),
        "today" => now.clone() - ChronoDuration::days(1),
        "week" => now.clone() - ChronoDuration::days(7),
        "month" => subtract_utc_months(now.clone(), 1),
        "year" => subtract_utc_months(now, 12),
        _ => return None,
    };
    Some(threshold.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

fn build_search_request(
    input: &Value,
    api_base_url: &Url,
    now: DateTime<Utc>,
) -> Result<SearchRequest> {
    let query = single_value(input.get("q"))
        .ok_or_else(|| anyhow!("Search query \"q\" is required."))?
        .to_owned();
    let mut url = api_base_url.clone();
    {
        let mut params = url.query_pairs_mut();
        params.append_pair("query", &query);

        let sort = single_value(input.get("sort"));
        let order = match sort {
            Some("upload_date") => "date",
            Some("view_count") => "viewCount",
            Some(value) => value,
            None => "relevance",
        };
        params.append_pair("order", order);

        if let Some(duration) = single_value(input.get("duration")) {
            params.append_pair("videoDuration", duration);
        }
        if let Some(date) = published_after(single_value(input.get("upload_date")), now) {
            params.append_pair("publishedAfter", &date);
        }
        if let Some(continuation) = single_value(input.get("continuation")) {
            params.append_pair("continuation", continuation);
        }
        if let Some(result_type) = single_value(input.get("type")) {
            params.append_pair("type", result_type);
        }
        if let Some(limit) = positive_limit(input.get("limit")) {
            params.append_pair("limit", &limit.to_string());
        }
    }
    Ok(SearchRequest { url, query })
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

async fn fetch_search_results(client: &Client, config: &ActorConfig, url: &Url) -> Result<Value> {
    let response = client
        .get(url.clone())
        .header("X-API-Key", &config.scrappa_api_key)
        .header("Accept", "application/json")
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
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
    response.json().await.map_err(|error| {
        if error.is_timeout() {
            anyhow!(
                "Scrappa API request timed out after {}s",
                SCRAPPA_REQUEST_TIMEOUT.as_secs()
            )
        } else {
            anyhow!("Scrappa API response was not valid JSON: {error}")
        }
    })
}

fn response_results(data: &Value) -> Result<Value> {
    let results = data
        .get("results")
        .filter(|results| !results.is_null())
        .cloned()
        .unwrap_or_else(|| json!([]));
    if results.is_array() || results.is_object() {
        Ok(results)
    } else {
        bail!("Scrappa API response results must be an array or object")
    }
}

fn result_count(results: &Value) -> usize {
    results.as_array().map_or(1, Vec::len)
}

struct DatasetBudget {
    capacity: usize,
    saved_rows: usize,
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

fn continuation_token(data: &Value) -> Option<&str> {
    let token = data
        .get("continuation")
        .filter(|token| !token.is_null())
        .or_else(|| data.pointer("/pagination/continuationToken"))?;
    token.as_str().filter(|token| !token.is_empty())
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    items: &Value,
    budget: &mut DatasetBudget,
) -> Result<usize> {
    let remaining = budget.capacity.saturating_sub(budget.saved_rows);
    let saved_rows = result_count(items).min(remaining);
    if saved_rows == 0 {
        return Ok(0);
    }
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let request = client.post(url).bearer_auth(&config.apify_token);
    let request = if let Some(rows) = items.as_array() {
        request.json(&rows[..saved_rows])
    } else {
        request.json(items)
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
    budget.saved_rows += saved_rows;
    Ok(saved_rows)
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let request = build_search_request(
        &input,
        &config.scrappa_api_base_url,
        DateTime::<Utc>::from(std::time::SystemTime::now()),
    )?;
    println!("Fetching from: {}", request.url);

    let data = fetch_search_results(client, config, &request.url).await?;
    let results = response_results(&data)?;
    let requested = result_count(&results);
    let capacity = run_dataset_capacity(client, config, requested).await?;
    let mut budget = DatasetBudget {
        capacity,
        saved_rows: 0,
    };
    push_dataset_items(client, config, &results, &mut budget).await?;
    println!(
        "Successfully fetched {} results for query: {}",
        result_count(&results),
        request.query
    );

    if let Some(continuation) = continuation_token(&data) {
        println!("Continuation token available for next page: {continuation}");
    }
    Ok(())
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::builder()
        .build()
        .context("Could not create HTTP client")?;
    run_actor(&client, &config).await
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube search data: {error:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::{
        collections::HashMap,
        io::Write,
        net::TcpListener,
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver},
            Arc,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    fn base_url() -> Url {
        Url::parse("https://scrappa.co/api/youtube/search").unwrap()
    }

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 2, 12, 0, 0).unwrap()
    }

    fn params(url: &Url) -> HashMap<String, String> {
        url.query_pairs().into_owned().collect()
    }

    #[test]
    fn maps_prefilled_search_input_and_caps_limit() {
        let input = json!({
            "q": " javascript tutorial ",
            "sort": ["view_count"],
            "duration": ["medium"],
            "upload_date": ["week"],
            "limit": 25,
            "continuation": "next page",
            "type": ["video"],
            "contentType": ["live"],
            "features": "4k,hd"
        });
        let request = build_search_request(&input, &base_url(), fixed_now()).unwrap();
        let query = params(&request.url);

        assert_eq!(request.query, "javascript tutorial");
        assert_eq!(
            request.url.as_str().split('?').next().unwrap(),
            "https://scrappa.co/api/youtube/search"
        );
        assert_eq!(query.get("query").unwrap(), "javascript tutorial");
        assert_eq!(query.get("order").unwrap(), "viewCount");
        assert_eq!(query.get("videoDuration").unwrap(), "medium");
        assert_eq!(
            query.get("publishedAfter").unwrap(),
            "2026-04-25T12:00:00.000Z"
        );
        assert_eq!(query.get("limit").unwrap(), "20");
        assert_eq!(query.get("continuation").unwrap(), "next page");
        assert_eq!(query.get("type").unwrap(), "video");
        assert!(!query.contains_key("contentType"));
        assert!(!query.contains_key("features"));
        assert!(!query.contains_key("upload_date"));
    }

    #[test]
    fn applies_defaults_without_inventing_a_limit() {
        let input = json!({
            "q": "Javascript tutorial",
            "sort": ["relevance"],
            "duration": ["short"],
            "upload_date": ["hour"],
            "type": ["all"]
        });
        let query = params(
            &build_search_request(&input, &base_url(), fixed_now())
                .unwrap()
                .url,
        );

        assert_eq!(query.get("query").unwrap(), "Javascript tutorial");
        assert_eq!(query.get("order").unwrap(), "relevance");
        assert_eq!(query.get("videoDuration").unwrap(), "short");
        assert_eq!(
            query.get("publishedAfter").unwrap(),
            "2026-05-02T11:00:00.000Z"
        );
        assert_eq!(query.get("type").unwrap(), "all");
        assert!(!query.contains_key("limit"));
    }

    #[test]
    fn defaults_order_and_omits_empty_filters() {
        let query = params(
            &build_search_request(
                &json!({"q": "news", "sort": [null, "upload_date"], "limit": 0}),
                &base_url(),
                fixed_now(),
            )
            .unwrap()
            .url,
        );

        assert_eq!(query.get("order").unwrap(), "date");
        assert!(!query.contains_key("limit"));
        assert!(!query.contains_key("videoDuration"));
        assert!(!query.contains_key("publishedAfter"));
    }

    #[test]
    fn maps_upload_date_buckets_to_published_after() {
        let input = |date| json!({"q": "news", "upload_date": [date]});
        for (date, expected) in [
            ("hour", "2026-05-02T11:00:00.000Z"),
            ("today", "2026-05-01T12:00:00.000Z"),
            ("week", "2026-04-25T12:00:00.000Z"),
            ("month", "2026-04-02T12:00:00.000Z"),
            ("year", "2025-05-02T12:00:00.000Z"),
        ] {
            let query = params(
                &build_search_request(&input(date), &base_url(), fixed_now())
                    .unwrap()
                    .url,
            );
            assert_eq!(query.get("publishedAfter").unwrap(), expected);
        }
    }

    #[test]
    fn clips_calendar_month_and_year_boundaries() {
        let month_end = Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap();
        let leap_day = Utc.with_ymd_and_hms(2024, 2, 29, 12, 0, 0).unwrap();

        assert_eq!(
            published_after(Some("month"), month_end).unwrap(),
            "2026-02-28T12:00:00.000Z"
        );
        assert_eq!(
            published_after(Some("year"), leap_day).unwrap(),
            "2023-02-28T12:00:00.000Z"
        );
    }

    #[test]
    fn maps_and_passes_through_sort_values() {
        for (sort, expected) in [("upload_date", "date"), ("rating", "rating")] {
            let input = json!({"q": "videos", "sort": [sort]});
            let query = params(
                &build_search_request(&input, &base_url(), fixed_now())
                    .unwrap()
                    .url,
            );
            assert_eq!(query.get("order").unwrap(), expected);
        }
    }

    #[test]
    fn requires_a_non_empty_query() {
        for input in [
            json!({"sort": ["relevance"]}),
            Value::Null,
            json!({"q": " "}),
        ] {
            assert!(build_search_request(&input, &base_url(), fixed_now())
                .unwrap_err()
                .to_string()
                .contains("q"));
        }
    }

    #[test]
    fn preserves_array_and_object_result_cardinality_for_dataset_rows() {
        let rows = response_results(&json!({
            "results": [
                {"id": "video-1", "title": "First"},
                {"id": "video-2", "title": "Second"}
            ]
        }))
        .unwrap();
        assert_eq!(result_count(&rows), 2);
        assert_eq!(rows.as_array().unwrap()[0]["id"], "video-1");

        let row = response_results(&json!({"results": {"id": "channel-1"}})).unwrap();
        assert_eq!(result_count(&row), 1);
        assert_eq!(row["id"], "channel-1");
    }

    #[test]
    fn missing_or_null_results_produce_no_dataset_rows() {
        for data in [json!({}), json!({"results": null})] {
            let results = response_results(&data).unwrap();
            assert_eq!(results, json!([]));
            assert_eq!(result_count(&results), 0);
        }
    }

    #[test]
    fn reads_continuation_from_top_level_then_pagination() {
        assert_eq!(
            continuation_token(&json!({"continuation": "next"})),
            Some("next")
        );
        assert_eq!(
            continuation_token(&json!({"pagination": {"continuationToken": "nested-next"}})),
            Some("nested-next")
        );
        assert_eq!(
            continuation_token(
                &json!({"continuation": "", "pagination": {"continuationToken": "ignored"}})
            ),
            None
        );
    }

    fn read_mock_request(stream: &mut std::net::TcpStream) -> String {
        use std::io::Read;

        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        let mut content_length = None;
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                if content_length.is_none() {
                    let headers = String::from_utf8_lossy(&bytes[..header_end]);
                    content_length = Some(
                        headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0),
                    );
                }
                if bytes.len() >= header_end + 4 + content_length.unwrap() {
                    break;
                }
            }
        }
        String::from_utf8(bytes).unwrap()
    }

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
            let (recorded_requests, requests) = mpsc::channel();
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
                    let request = read_mock_request(&mut stream);
                    let _ = recorded_requests.send(request);
                    let Some(response) = responses.next() else {
                        break;
                    };
                    let reason = match response.status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        401 => "Unauthorized",
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

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    fn pricing_response(max_charge: f64) -> MockResponse {
        response(
            200,
            &json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            "apify-actor-start": {"eventPriceUsd": 0.00005}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": max_charge},
                    "chargedEventCounts": {
                        "apify-default-dataset-item": 0,
                        "apify-actor-start": 0
                    }
                }
            })
            .to_string(),
        )
    }

    fn mock_config(base_url: &Url) -> ActorConfig {
        let mut scrappa_api_base_url = base_url.clone();
        scrappa_api_base_url.set_path("/api/youtube/search");
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url,
            default_key_value_store_id: "store".to_owned(),
            default_dataset_id: "dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "apify-test-token".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
        }
    }

    fn run_responses(
        results: &Value,
        pricing: MockResponse,
        dataset_status: Option<u16>,
    ) -> Vec<MockResponse> {
        let mut responses = vec![
            response(200, r#"{"q":"test query"}"#),
            response(200, &json!({"results": results}).to_string()),
            pricing,
        ];
        if let Some(status) = dataset_status {
            responses.push(response(status, "{}"));
        }
        responses
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

    fn client() -> Client {
        Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap()
    }

    fn has_test_bearer_token(request: &str) -> bool {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .any(|line| {
                let Some((name, value)) = line.split_once(':') else {
                    return false;
                };
                name.eq_ignore_ascii_case("authorization")
                    && value.trim() == "Bearer apify-test-token"
            })
    }
    fn two_search_rows() -> Value {
        json!([
            {"id": "video-1", "title": "First"},
            {"id": "video-2", "title": "Second"}
        ])
    }

    #[tokio::test]
    async fn capped_run_posts_only_the_affordable_prefix() {
        let rows = two_search_rows();
        let server = MockServer::start(run_responses(&rows, pricing_response(0.0003), Some(201)));
        run_actor(&client(), &mock_config(&server.base_url))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/store/records/INPUT"
        );
        assert!(request_parts(&requests[1])
            .1
            .starts_with("/api/youtube/search?"));
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
        assert!(has_test_bearer_token(&requests[2]));
        assert_eq!(request_parts(&requests[3]).1, "/v2/datasets/dataset/items");
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[3]).2).unwrap(),
            json!([{"id": "video-1", "title": "First"}])
        );
        assert!(requests
            .iter()
            .filter(|request| request.contains("/v2/"))
            .all(|request| has_test_bearer_token(request)));
        assert!(!has_test_bearer_token(&requests[1]));
    }

    #[tokio::test]
    async fn zero_budget_skips_dataset_post() {
        let rows = two_search_rows();
        let server = MockServer::start(run_responses(&rows, pricing_response(0.0), None));
        run_actor(&client(), &mock_config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));
    }

    #[tokio::test]
    async fn default_run_budget_posts_all_search_results_without_user_cap() {
        let rows = two_search_rows();
        let server = MockServer::start(run_responses(&rows, pricing_response(4.506432), Some(201)));
        run_actor(&client(), &mock_config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[3]).2).unwrap(),
            rows
        );
    }

    #[tokio::test]
    async fn missing_or_unavailable_pricing_fails_before_dataset_post() {
        let rows = two_search_rows();
        for pricing in [
            response(500, "pricing unavailable"),
            response(200, r#"{"data":{}}"#),
        ] {
            let server = MockServer::start(vec![
                response(200, r#"{"q":"test query"}"#),
                response(200, &json!({"results": rows.clone()}).to_string()),
                pricing,
            ]);
            assert!(run_actor(&client(), &mock_config(&server.base_url))
                .await
                .is_err());
            let requests = server.requests();
            assert_eq!(requests.len(), 3);
            assert!(requests
                .iter()
                .all(|request| !request.starts_with("POST /v2/datasets/")));
        }
    }

    #[tokio::test]
    async fn dataset_write_errors_propagate() {
        let rows = two_search_rows();
        let server = MockServer::start(run_responses(&rows, pricing_response(1.0), Some(500)));
        let error = run_actor(&client(), &mock_config(&server.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("Apify dataset write failed"));
        assert_eq!(server.requests().len(), 4);
    }

    #[tokio::test]
    async fn multiple_dataset_posts_share_the_local_saved_row_budget() {
        let server = MockServer::start(vec![pricing_response(0.0003), response(201, "{}")]);
        let config = mock_config(&server.base_url);
        let client = client();
        let capacity = run_dataset_capacity(&client, &config, 3).await.unwrap();
        assert_eq!(capacity, 1);
        let mut budget = DatasetBudget {
            capacity,
            saved_rows: 0,
        };
        assert_eq!(
            push_dataset_items(
                &client,
                &config,
                &json!([{"id": "first"}, {"id": "second"}]),
                &mut budget,
            )
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            push_dataset_items(&client, &config, &json!([{"id": "third"}]), &mut budget,)
                .await
                .unwrap(),
            0
        );
        assert_eq!(budget.saved_rows, 1);
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_parts(&requests[0]).1, "/v2/actor-runs/test-run");
        assert_eq!(request_parts(&requests[1]).1, "/v2/datasets/dataset/items");
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[1]).2).unwrap(),
            json!([{"id": "first"}])
        );
    }

    #[test]
    fn capacity_counts_previously_charged_priced_events() {
        let run = json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.0002},
                    "apify-actor-start": {"eventPriceUsd": 0.00005}
                }}
            },
            "options": {"maxTotalChargeUsd": 0.0005},
            "chargedEventCounts": {
                "apify-default-dataset-item": 1,
                "apify-actor-start": 2
            }
        }});
        assert_eq!(affordable_dataset_items(&run, 2).unwrap(), 1);
        assert!(affordable_dataset_items(&json!({"data": {}}), 1).is_err());
        let mut missing_limit = run.clone();
        missing_limit["data"]["options"] = json!({});
        assert!(affordable_dataset_items(&missing_limit, 1).is_err());
        let mut null_limit = run.clone();
        null_limit["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
        assert!(affordable_dataset_items(&null_limit, 1).is_err());
    }
}
