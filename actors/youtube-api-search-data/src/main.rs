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

fn continuation_token(data: &Value) -> Option<&str> {
    let token = data
        .get("continuation")
        .filter(|token| !token.is_null())
        .or_else(|| data.pointer("/pagination/continuationToken"))?;
    token.as_str().filter(|token| !token.is_empty())
}

async fn push_dataset_items(client: &Client, config: &ActorConfig, items: &Value) -> Result<()> {
    if items.as_array().is_some_and(Vec::is_empty) {
        return Ok(());
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
    Ok(())
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
    push_dataset_items(client, config, &results).await?;
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
    use std::collections::HashMap;

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

    #[tokio::test]
    async fn posts_result_rows_to_the_default_dataset() {
        use std::{io::Write, net::TcpListener, thread};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_mock_request(&mut stream);
            stream
                .write_all(
                    b"HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
            request
        });
        let config = ActorConfig {
            apify_api_base_url: Url::parse(&format!("http://{address}")).unwrap(),
            scrappa_api_base_url: base_url(),
            default_key_value_store_id: "store".to_owned(),
            default_dataset_id: "dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "apify-test-token".to_owned(),
            scrappa_api_key: "scrappa-test-key".to_owned(),
        };
        let expected = json!([
            {"id": "video-1", "title": "First"},
            {"id": "video-2", "title": "Second"}
        ]);
        let results = response_results(&json!({"results": expected.clone()})).unwrap();
        let client = Client::builder().build().unwrap();

        push_dataset_items(&client, &config, &results)
            .await
            .unwrap();

        let request = server.join().unwrap();
        let (headers, body) = request.split_once("\r\n\r\n").unwrap();
        assert_eq!(
            request.lines().next().unwrap(),
            "POST /v2/datasets/dataset/items HTTP/1.1"
        );
        assert!(headers
            .lines()
            .any(|line| line.eq_ignore_ascii_case("authorization: bearer apify-test-token")));
        assert_eq!(serde_json::from_str::<Value>(body).unwrap(), expected);
    }
}
