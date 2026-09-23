use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::Value;
use std::{collections::HashSet, env, time::Duration};
use tokio::time::timeout;
use url::{form_urlencoded, Url};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api/youtube/channel-videos";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

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
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key: required_env("SCRAPPA_API_KEY")?,
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

fn parse_ids(value: Option<&Value>, ids: &mut Vec<String>) {
    match value {
        Some(Value::Array(values)) => {
            for value in values {
                parse_ids(Some(value), ids);
            }
        }
        Some(Value::String(value)) => ids.extend(
            value
                .split(',')
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .map(str::to_owned),
        ),
        _ => {}
    }
}

fn channel_ids(input: &Value) -> Vec<String> {
    let mut parsed = Vec::new();
    parse_ids(input.get("ids"), &mut parsed);
    parse_ids(input.get("id"), &mut parsed);

    let mut seen = HashSet::with_capacity(parsed.len());
    parsed.retain(|id| seen.insert(id.clone()));
    parsed
}

fn assert_continuation_matches_batch(input: &Value, ids: &[String]) -> Result<()> {
    if ids.len() > 1
        && input
            .get("continuation")
            .and_then(Value::as_str)
            .is_some_and(|token| !token.trim().is_empty())
    {
        bail!("The \"continuation\" token can only be used with a single YouTube channel ID.");
    }
    Ok(())
}

fn selected_sort(input: &Value) -> Option<&str> {
    match input.get("sort")? {
        Value::Array(values) => values.first().and_then(Value::as_str),
        value => value.as_str(),
    }
}

fn build_channel_podcasts_url(input: &Value, id: &str, api_base_url: &Url) -> Result<Url> {
    if id.is_empty() {
        bail!("Search query \"id\" not provided. Please provide a value for \"id\" in the input.");
    }

    let mut url = api_base_url.clone();
    let mut query = form_urlencoded::Serializer::new(String::new());
    query.append_pair("channel_id", id);
    if let Some(sort) = selected_sort(input).filter(|sort| !sort.trim().is_empty()) {
        query.append_pair("sort", sort);
    }
    if let Some(continuation) = input
        .get("continuation")
        .and_then(Value::as_str)
        .filter(|token| !token.trim().is_empty())
    {
        query.append_pair("continuation", continuation);
    }
    url.set_query(Some(&query.finish()));
    Ok(url)
}

fn contains_podcast_text(value: &Value) -> bool {
    match value {
        Value::String(text) => text.to_lowercase().contains("podcast"),
        Value::Array(values) => values.iter().any(contains_podcast_text),
        Value::Object(values) => values.values().any(contains_podcast_text),
        _ => false,
    }
}

fn is_podcast_video(video: &Value) -> bool {
    if video.get("isPodcast") == Some(&Value::Bool(true)) {
        return true;
    }
    [
        "type",
        "videoType",
        "contentType",
        "category",
        "playlistType",
    ]
    .iter()
    .any(|field| video.get(field).is_some_and(contains_podcast_text))
        || video
            .get("badges")
            .and_then(Value::as_array)
            .is_some_and(|badges| badges.iter().any(contains_podcast_text))
        || video
            .get("metadata")
            .and_then(|metadata| metadata.get("badges"))
            .and_then(Value::as_array)
            .is_some_and(|badges| badges.iter().any(contains_podcast_text))
}

fn podcast_videos(response_data: &Value) -> Result<Vec<Value>> {
    let videos = match response_data.get("videos") {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Array(videos)) => videos,
        _ => bail!("Scrappa API videos must be an array"),
    };
    Ok(videos
        .iter()
        .filter(|video| is_podcast_video(video))
        .cloned()
        .collect())
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

async fn fetch_scrappa_json(client: &Client, config: &ActorConfig, url: &Url) -> Result<Value> {
    let request = async {
        let response = client
            .get(url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .header("X-API-Key", config.scrappa_api_key.as_str())
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
struct DatasetBudget {
    capacity: Option<usize>,
    saved_items: usize,
}

async fn run_dataset_capacity(client: &Client, config: &ActorConfig) -> Result<usize> {
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
    affordable_dataset_items(&run)
}

fn affordable_dataset_items(run: &Value) -> Result<usize> {
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
        return Ok(usize::MAX);
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok(((max_charge - spent + tolerance) / item_price)
        .floor()
        .max(0.0) as usize)
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    items: &[Value],
    budget: &mut DatasetBudget,
) -> Result<usize> {
    if items.is_empty() {
        return Ok(0);
    }
    let capacity = match budget.capacity {
        Some(capacity) => capacity,
        None => {
            let capacity = run_dataset_capacity(client, config).await?;
            budget.capacity = Some(capacity);
            capacity
        }
    };
    let items = &items[..items.len().min(capacity.saturating_sub(budget.saved_items))];
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
    budget.saved_items += items.len();
    Ok(items.len())
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let ids = channel_ids(&input);
    if ids.is_empty() {
        bail!("At least one YouTube channel ID must be provided in \"ids\" or \"id\".");
    }
    assert_continuation_matches_batch(&input, &ids)?;

    let mut budget = DatasetBudget {
        capacity: None,
        saved_items: 0,
    };
    for id in &ids {
        let url = build_channel_podcasts_url(&input, id, &config.scrappa_api_base_url)?;
        println!("Fetching from: {url}");
        let response_data = fetch_scrappa_json(client, config, &url).await?;
        let videos = podcast_videos(&response_data)?;
        let saved = push_dataset_items(client, config, &videos, &mut budget).await?;
        println!(
            "Successfully fetched {} podcast video(s) for channel id: {id}; saved {saved}",
            videos.len()
        );

        if let Some(token) = continuation_token(&response_data).filter(|token| js_truthy(token)) {
            println!(
                "Continuation token available for next page: {}",
                js_string(token)
            );
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube channel podcasts: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    run_actor(&Client::new(), &config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        thread,
    };

    #[test]
    fn prefilled_channels_and_sort_remain_available() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["id"]["prefill"],
            "UCJZv4d5rbIKd4QHMPkcABCw"
        );
        assert_eq!(
            schema["properties"]["ids"]["prefill"],
            "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw"
        );
        assert_eq!(schema["properties"]["sort"]["prefill"], "newest");
        assert_eq!(
            channel_ids(&json!({"ids": schema["properties"]["ids"]["prefill"]})),
            vec![
                "UCJZv4d5rbIKd4QHMPkcABCw".to_owned(),
                "UC_x5XG1OV2P6uZZ5FSM9Ttw".to_owned()
            ]
        );
    }

    #[test]
    fn builds_encoded_channel_url_and_accepts_first_sort_array_value() {
        let base = Url::parse("https://scrappa.co/api/youtube/channel-videos").unwrap();
        let url = build_channel_podcasts_url(
            &json!({"sort": ["popular", "oldest"], "continuation": "next page/+"}),
            "UC example",
            &base,
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://scrappa.co/api/youtube/channel-videos?channel_id=UC+example&sort=popular&continuation=next+page%2F%2B"
        );
    }

    #[test]
    fn batches_ids_in_order_without_duplicates_and_rejects_batch_pagination() {
        let input = json!({"ids": ["UC1, UC2", "UC1"], "id": "UC2,UC3"});
        let ids = channel_ids(&input);
        assert_eq!(
            ids,
            vec!["UC1".to_owned(), "UC2".to_owned(), "UC3".to_owned()]
        );
        assert!(assert_continuation_matches_batch(&input, &ids).is_ok());
        assert!(assert_continuation_matches_batch(
            &json!({"continuation": "next"}),
            &channel_ids(&json!({"ids": "UC1,UC2"}))
        )
        .unwrap_err()
        .to_string()
        .contains("continuation"));
    }

    #[test]
    fn filters_podcast_markers_without_changing_result_rows() {
        let response = json!({"videos": [
            {"id": "flagged", "isPodcast": true},
            {"id": "type", "type": "PODCAST"},
            {"id": "videoType", "videoType": "Podcast episode"},
            {"id": "contentType", "contentType": "podcasts"},
            {"id": "category", "category": "Podcast"},
            {"id": "playlistType", "playlistType": "podcast"},
            {"id": "badge", "badges": ["Podcast"]},
            {"id": "metadataBadge", "metadata": {"badges": [{"label": "Podcast episode"}]}},
            {"id": "regular", "type": "video", "title": "Regular upload"}
        ]});
        let rows = podcast_videos(&response).unwrap();
        assert_eq!(
            rows.iter()
                .map(|video| video["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "flagged",
                "type",
                "videoType",
                "contentType",
                "category",
                "playlistType",
                "badge",
                "metadataBadge"
            ]
        );
        assert_eq!(rows[0], response["videos"][0]);
        assert_eq!(
            rows.last().unwrap().as_object(),
            response["videos"][7].as_object()
        );
    }

    #[test]
    fn malformed_video_payload_fails_instead_of_succeeding_empty() {
        assert!(podcast_videos(&json!({"videos": {"id": "invalid"}})).is_err());
        assert_eq!(
            podcast_videos(&json!({"videos": null})).unwrap(),
            Vec::<Value>::new()
        );
    }

    fn podcast_response() -> String {
        json!({"videos": [
            {"id": "first", "isPodcast": true},
            {"id": "second", "isPodcast": true}
        ]})
        .to_string()
    }

    fn pricing_response(max_charge: f64, item_price: f64, charged_events: Value) -> String {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": item_price},
                        "other-event": {"eventPriceUsd": 0.0003}
                    }}
                },
                "chargedEventCounts": charged_events,
                "options": {"maxTotalChargeUsd": max_charge}
            }
        })
        .to_string()
    }

    fn posted_dataset_rows(requests: &[String]) -> Vec<Value> {
        requests
            .iter()
            .filter(|request| request.starts_with("POST /v2/datasets/dataset/items "))
            .map(|request| serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap())
            .collect()
    }

    #[tokio::test]
    async fn reads_apify_input_calls_scrappa_and_writes_matching_rows() {
        let input = json!({"ids": "UC1, UC2", "id": "UC2,UC3", "sort": "newest"});
        let replies = vec![
            (200, input.to_string()),
            (200, json!({"videos": [{"id": "one", "isPodcast": true}, {"id": "not-podcast"}], "continuation": "next"}).to_string()),
            (200, pricing_response(0.001, 0.0003, json!({}))),
            (201, String::new()),
            (200, json!({"videos": [{"id": "two", "badges": ["Podcast"]}]}).to_string()),
            (201, String::new()),
            (200, json!({"videos": [{"id": "three", "type": "podcast episode"}]}).to_string()),
            (201, String::new()),
        ];
        let (api_base, server) = mock_server(replies);
        let config = test_config(&api_base, "key-store", "dataset");

        run_actor(&Client::new(), &config).await.unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 8);
        assert!(requests[0].starts_with("GET /v2/key-value-stores/key-store/records/INPUT "));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer apify-test"));
        let pricing_request = requests
            .iter()
            .find(|request| request.starts_with("GET /v2/actor-runs/test-run "))
            .unwrap();
        assert!(pricing_request
            .to_ascii_lowercase()
            .contains("authorization: bearer apify-test"));

        let upstream: Vec<_> = requests
            .iter()
            .filter(|request| request.starts_with("GET /api/youtube/channel-videos?"))
            .collect();
        assert_eq!(upstream.len(), 3);
        for (request, id) in upstream.iter().zip(["UC1", "UC2", "UC3"]) {
            assert!(request.contains(format!("channel_id={id}&sort=newest").as_str()));
            assert!(request
                .to_ascii_lowercase()
                .contains("x-api-key: scrappa-test"));
            assert!(request
                .to_ascii_lowercase()
                .contains("accept: application/json"));
        }
        assert_eq!(
            posted_dataset_rows(&requests),
            vec![
                json!([{"id": "one", "isPodcast": true}]),
                json!([{"id": "two", "badges": ["Podcast"]}]),
                json!([{"id": "three", "type": "podcast episode"}])
            ]
        );
    }
    #[tokio::test]
    async fn caps_two_podcast_rows_at_one_dataset_event() {
        let (api_base, server) = mock_server(vec![
            (200, json!({"id": "UC1"}).to_string()),
            (200, podcast_response()),
            (200, pricing_response(0.0003, 0.0003, json!({}))),
            (201, String::new()),
        ]);
        let config = test_config(&api_base, "key-store", "dataset");

        run_actor(&Client::new(), &config).await.unwrap();
        let requests = server.join().unwrap();
        assert!(requests[2].starts_with("GET /v2/actor-runs/test-run "));
        assert!(requests[2]
            .to_ascii_lowercase()
            .contains("authorization: bearer apify-test"));
        assert_eq!(
            posted_dataset_rows(&requests),
            vec![json!([{"id": "first", "isPodcast": true}])]
        );
    }

    #[tokio::test]
    async fn zero_remaining_budget_posts_no_dataset_rows() {
        for pricing in [
            pricing_response(0.0, 0.0003, json!({})),
            pricing_response(0.0003, 0.0003, json!({"other-event": 1})),
        ] {
            let (api_base, server) = mock_server(vec![
                (200, json!({"id": "UC1"}).to_string()),
                (200, podcast_response()),
                (200, pricing),
            ]);
            let config = test_config(&api_base, "key-store", "dataset");

            run_actor(&Client::new(), &config).await.unwrap();
            let requests = server.join().unwrap();
            assert_eq!(requests.len(), 3);
            assert!(posted_dataset_rows(&requests).is_empty());
        }
    }

    #[tokio::test]
    async fn normal_numeric_budget_posts_all_rows() {
        let (api_base, server) = mock_server(vec![
            (200, json!({"id": "UC1"}).to_string()),
            (200, podcast_response()),
            (200, pricing_response(4.506432, 0.0003, json!({}))),
            (201, String::new()),
        ]);
        let config = test_config(&api_base, "key-store", "dataset");

        run_actor(&Client::new(), &config).await.unwrap();
        let requests = server.join().unwrap();
        assert_eq!(
            posted_dataset_rows(&requests),
            vec![json!([
                {"id": "first", "isPodcast": true},
                {"id": "second", "isPodcast": true}
            ])]
        );
    }

    #[tokio::test]
    async fn zero_price_event_has_no_dataset_row_cap() {
        let (api_base, server) = mock_server(vec![
            (200, json!({"id": "UC1"}).to_string()),
            (200, podcast_response()),
            (200, pricing_response(0.0, 0.0, json!({}))),
            (201, String::new()),
        ]);
        let config = test_config(&api_base, "key-store", "dataset");

        run_actor(&Client::new(), &config).await.unwrap();
        let requests = server.join().unwrap();
        assert_eq!(
            posted_dataset_rows(&requests),
            vec![json!([
                {"id": "first", "isPodcast": true},
                {"id": "second", "isPodcast": true}
            ])]
        );
    }

    #[tokio::test]
    async fn multiple_channel_writes_share_the_initial_budget_snapshot() {
        let (api_base, server) = mock_server(vec![
            (200, json!({"ids": "UC1,UC2"}).to_string()),
            (
                200,
                json!({"videos": [{"id": "first", "isPodcast": true}]}).to_string(),
            ),
            (200, pricing_response(0.0003, 0.0003, json!({}))),
            (201, String::new()),
            (
                200,
                json!({"videos": [{"id": "second", "isPodcast": true}]}).to_string(),
            ),
        ]);
        let config = test_config(&api_base, "key-store", "dataset");

        run_actor(&Client::new(), &config).await.unwrap();
        let requests = server.join().unwrap();
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.starts_with("GET /v2/actor-runs/test-run "))
                .count(),
            1
        );
        assert_eq!(
            posted_dataset_rows(&requests),
            vec![json!([{"id": "first", "isPodcast": true}])]
        );
    }

    #[tokio::test]
    async fn missing_dataset_event_price_fails_before_any_dataset_write() {
        let missing_price = json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {}}
            },
            "chargedEventCounts": {},
            "options": {"maxTotalChargeUsd": 1.0}
        }});
        let (api_base, server) = mock_server(vec![
            (200, json!({"id": "UC1"}).to_string()),
            (200, podcast_response()),
            (200, missing_price.to_string()),
        ]);
        let config = test_config(&api_base, "key-store", "dataset");

        let error = run_actor(&Client::new(), &config).await.unwrap_err();
        assert!(error.to_string().contains("dataset item price"));
        let requests = server.join().unwrap();
        assert!(posted_dataset_rows(&requests).is_empty());
    }

    #[tokio::test]
    async fn missing_or_null_spending_limit_fails_before_any_dataset_write() {
        for spending_limit in [None, Some(Value::Null)] {
            let mut invalid_pricing = json!({"data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0003}
                    }}
                },
                "chargedEventCounts": {},
                "options": {}
            }});
            if let Some(spending_limit) = spending_limit {
                invalid_pricing["data"]["options"]["maxTotalChargeUsd"] = spending_limit;
            }
            let (api_base, server) = mock_server(vec![
                (200, json!({"id": "UC1"}).to_string()),
                (200, podcast_response()),
                (200, invalid_pricing.to_string()),
            ]);
            let config = test_config(&api_base, "key-store", "dataset");

            let error = run_actor(&Client::new(), &config).await.unwrap_err();
            assert!(error.to_string().contains("spending limit"));
            let requests = server.join().unwrap();
            assert!(posted_dataset_rows(&requests).is_empty());
        }
    }

    #[test]
    fn missing_charged_counts_fail_closed() {
        let mut run: Value =
            serde_json::from_str(&pricing_response(1.0, 0.0003, json!({}))).unwrap();
        run["data"]
            .as_object_mut()
            .unwrap()
            .remove("chargedEventCounts");
        assert!(affordable_dataset_items(&run).is_err());
    }

    #[tokio::test]
    async fn dataset_write_failure_is_propagated() {
        let (api_base, server) = mock_server(vec![
            (200, json!({"id": "UC1"}).to_string()),
            (200, podcast_response()),
            (200, pricing_response(4.506432, 0.0003, json!({}))),
            (503, "storage unavailable".to_owned()),
        ]);
        let config = test_config(&api_base, "key-store", "dataset");

        let error = run_actor(&Client::new(), &config).await.unwrap_err();
        assert!(error.to_string().contains("Apify dataset write failed"));
        let requests = server.join().unwrap();
        assert_eq!(posted_dataset_rows(&requests).len(), 1);
    }

    #[tokio::test]
    async fn surfaces_scrappa_http_errors() {
        let (api_base, server) = mock_server(vec![
            (200, json!({"id": "UC1"}).to_string()),
            (503, "upstream unavailable".to_owned()),
        ]);
        let config = test_config(&api_base, "key-store", "dataset");
        let error = run_actor(&Client::new(), &config).await.unwrap_err();
        assert!(error
            .to_string()
            .contains("Scrappa API request failed with 503 Service Unavailable"));
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
    }

    fn test_config(api_base: &str, store_id: &str, dataset_id: &str) -> ActorConfig {
        let api_base = Url::parse(api_base).unwrap();
        ActorConfig {
            apify_api_base_url: api_base.clone(),
            scrappa_api_base_url: api_base.join("api/youtube/channel-videos").unwrap(),
            default_key_value_store_id: store_id.to_owned(),
            default_dataset_id: dataset_id.to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "apify-test".to_owned(),
            scrappa_api_key: "scrappa-test".to_owned(),
        }
    }

    fn mock_server(replies: Vec<(u16, String)>) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let mut captured = Vec::new();
            for (status, body) in replies {
                let (mut stream, _) = accept_with_timeout(&listener);
                let request = read_request(&mut stream);
                let reason = match status {
                    200 => "OK",
                    201 => "Created",
                    503 => "Service Unavailable",
                    _ => "Error",
                };
                write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
                stream.flush().unwrap();
                captured.push(request);
            }
            captured
        });
        (format!("http://{address}"), server)
    }

    fn accept_with_timeout(listener: &TcpListener) -> (TcpStream, std::net::SocketAddr) {
        for _ in 0..500 {
            match listener.accept() {
                Ok(connection) => return connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("mock API accept failed: {error}"),
            }
        }
        panic!("timed out waiting for an actor API request");
    }

    fn read_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut chunk = [0; 4096];
        loop {
            let count = stream.read(&mut chunk).unwrap();
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..count]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if bytes.len() >= header_end + 4 + content_length {
                    return String::from_utf8(bytes).unwrap();
                }
            }
        }
        String::from_utf8(bytes).unwrap()
    }
}
