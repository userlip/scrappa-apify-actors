use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://ytapi.scrappa.co";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
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
        .extend(segments);
    Ok(url)
}

fn string_value(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn normalized_country(value: Option<&Value>) -> Option<String> {
    string_value(value).map(|country| country.to_uppercase())
}

#[derive(Debug)]
struct SuggestionsRequest {
    url: Url,
    query: String,
    hl: Option<String>,
    gl: Option<String>,
}

fn build_suggestions_request(input: &Value, api_base_url: &Url) -> Result<SuggestionsRequest> {
    let query =
        string_value(input.get("q")).ok_or_else(|| anyhow!("Search query \"q\" is required."))?;
    let hl = string_value(input.get("hl"));
    let gl = normalized_country(input.get("gl"));
    let mut url = endpoint_url(api_base_url, &["search", "suggestions"])?;
    {
        let mut params = url.query_pairs_mut();
        params.append_pair("q", &query);
        if let Some(hl) = &hl {
            params.append_pair("hl", hl);
        }
        if let Some(gl) = &gl {
            params.append_pair("gl", gl);
        }
    }

    Ok(SuggestionsRequest { url, query, hl, gl })
}

fn suggestions_to_dataset_items(data: &Value, fallback: &SuggestionsRequest) -> Vec<Value> {
    let query = string_value(data.get("query")).unwrap_or_else(|| fallback.query.clone());
    let hl = string_value(data.get("locale").and_then(|locale| locale.get("hl")))
        .or_else(|| fallback.hl.clone());
    let gl = normalized_country(data.get("locale").and_then(|locale| locale.get("gl")))
        .or_else(|| fallback.gl.clone());

    let Some(suggestions) = data.get("suggestions").and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut items = Vec::new();
    for suggestion in suggestions {
        let Some(suggestion) = suggestion.as_str().filter(|value| !value.trim().is_empty()) else {
            continue;
        };
        let mut item = json!({
            "query": query,
            "suggestion": suggestion,
            "position": items.len() + 1,
        });
        if let Some(hl) = &hl {
            item["hl"] = json!(hl);
        }
        if let Some(gl) = &gl {
            item["gl"] = json!(gl);
        }
        items.push(item);
    }
    items
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

async fn fetch_suggestions(client: &Client, request: &SuggestionsRequest) -> Result<Value> {
    let response = client
        .get(request.url.clone())
        .header(reqwest::header::ACCEPT, "application/json")
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() || error.to_string().contains("aborted") {
                anyhow!(
                    "Scrappa API request timed out after {}s",
                    REQUEST_TIMEOUT.as_secs()
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

struct DatasetBudget {
    dataset_item_price_usd: f64,
    charged_usd: f64,
    max_total_charge_usd: f64,
    saved_items: usize,
}

impl DatasetBudget {
    fn from_run(run: &Value) -> Result<Self> {
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
        let dataset_item_price_usd = events
            .get("apify-default-dataset-item")
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Apify run did not provide the spending limit"))?;
        if !dataset_item_price_usd.is_finite()
            || dataset_item_price_usd < 0.0
            || !max_total_charge_usd.is_finite()
            || max_total_charge_usd < 0.0
        {
            bail!("Apify run returned invalid charging values");
        }

        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_usd = 0.0;
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
            charged_usd += price * count as f64;
        }
        if !charged_usd.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        Ok(Self {
            dataset_item_price_usd,
            charged_usd,
            max_total_charge_usd,
            saved_items: 0,
        })
    }

    fn affordable_items(&self, requested: usize) -> usize {
        if self.dataset_item_price_usd == 0.0 {
            return requested;
        }
        let tolerance = f64::EPSILON * self.max_total_charge_usd;
        (1..=requested)
            .take_while(|count| {
                self.saved_items
                    .checked_add(*count)
                    .is_some_and(|total_items| {
                        self.charged_usd + total_items as f64 * self.dataset_item_price_usd
                            <= self.max_total_charge_usd + tolerance
                    })
            })
            .count()
    }

    fn record_saved_items(&mut self, count: usize) {
        self.saved_items = self.saved_items.saturating_add(count);
    }
}

async fn run_dataset_budget(client: &Client, config: &ActorConfig) -> Result<DatasetBudget> {
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
    DatasetBudget::from_run(&run)
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    budget: &mut Option<DatasetBudget>,
    items: &[Value],
) -> Result<usize> {
    if items.is_empty() {
        return Ok(0);
    }
    if budget.is_none() {
        *budget = Some(run_dataset_budget(client, config).await?);
    }
    let Some(dataset_budget) = budget.as_mut() else {
        bail!("Apify dataset pricing budget was not initialized");
    };
    let allowed = dataset_budget.affordable_items(items.len());
    if allowed == 0 {
        return Ok(0);
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(&items[..allowed])
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
    dataset_budget.record_saved_items(allowed);
    Ok(allowed)
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let request = build_suggestions_request(&input, &config.scrappa_api_base_url)?;
    println!("Fetching from: {}", request.url);

    let data = fetch_suggestions(client, &request).await?;
    let items = suggestions_to_dataset_items(&data, &request);
    let mut dataset_budget = None;
    let saved_items = push_dataset_items(client, config, &mut dataset_budget, &items).await?;
    println!(
        "Successfully fetched {} suggestion(s) for query: {}",
        items.len(),
        request.query
    );
    println!(
        "Saved {saved_items} of {} suggestion(s) to the default dataset",
        items.len()
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Failed to fetch YouTube search suggestions: {error:#}");
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
    use std::{
        cell::RefCell,
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc, Arc,
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
        requests: RefCell<mpsc::Receiver<String>>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (request_sender, requests) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                for response in responses {
                    let (mut stream, _) = loop {
                        if stopped.load(Ordering::Relaxed) || Instant::now() >= deadline {
                            return;
                        }
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(_) => return,
                        }
                    };
                    let request = read_request(&mut stream).unwrap_or_default();
                    let _ = request_sender.send(request);
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
                        return;
                    }
                }
            });
            Self {
                base_url: Url::parse(&format!("http://{address}")).unwrap(),
                requests: RefCell::new(requests),
                stop,
                thread: Some(thread),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.borrow_mut().try_iter().collect()
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
        let mut content_length = 0;
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                if content_length == 0 {
                    let headers = String::from_utf8_lossy(&bytes[..header_end]);
                    content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                }
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    fn pricing_response(max_charge: Option<f64>, charged_counts: Value) -> MockResponse {
        let mut options = json!({});
        if let Some(max_charge) = max_charge {
            options["maxTotalChargeUsd"] = json!(max_charge);
        }
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
                    "options": options,
                    "chargedEventCounts": charged_counts
                }
            })
            .to_string(),
        )
    }

    fn config(base_url: &Url) -> ActorConfig {
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: base_url.clone(),
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token-not-a-real-credential".to_owned(),
        }
    }

    fn client() -> Client {
        Client::builder().build().unwrap()
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

    fn query_pairs(url: &Url) -> Vec<(String, String)> {
        url.query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect()
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
                    && value.trim() == "Bearer test-token-not-a-real-credential"
            })
    }

    #[test]
    fn request_url_trims_inputs_encodes_query_and_omits_missing_locales() {
        let base_url = Url::parse(SCRAPPA_API_BASE_URL).unwrap();
        let request = build_suggestions_request(
            &json!({"q":" javascript tutorial /? ","hl":" en ","gl":"us"}),
            &base_url,
        )
        .unwrap();
        assert_eq!(request.query, "javascript tutorial /?");
        assert_eq!(request.hl.as_deref(), Some("en"));
        assert_eq!(request.gl.as_deref(), Some("US"));
        assert_eq!(request.url.scheme(), "https");
        assert_eq!(request.url.host_str(), Some("ytapi.scrappa.co"));
        assert_eq!(request.url.path(), "/search/suggestions");
        let params = query_pairs(&request.url);
        assert_eq!(params.len(), 3);
        assert_eq!(
            params[0],
            ("q".to_owned(), "javascript tutorial /?".to_owned())
        );
        assert_eq!(params[1], ("hl".to_owned(), "en".to_owned()));
        assert_eq!(params[2], ("gl".to_owned(), "US".to_owned()));

        let request = build_suggestions_request(&json!({"q":"news"}), &base_url).unwrap();
        assert_eq!(
            query_pairs(&request.url),
            vec![("q".to_owned(), "news".to_owned())]
        );
        assert!(build_suggestions_request(&json!({"hl":"en"}), &base_url)
            .unwrap_err()
            .to_string()
            .contains("q"));
    }

    #[test]
    fn input_schema_prefills_are_used_without_changing_their_values() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let properties = &schema["properties"];
        let input = json!({
            "q": properties["q"]["prefill"],
            "hl": properties["hl"]["prefill"],
            "gl": properties["gl"]["prefill"],
        });
        let request =
            build_suggestions_request(&input, &Url::parse(SCRAPPA_API_BASE_URL).unwrap()).unwrap();
        assert_eq!(request.query, "javascript");
        assert_eq!(request.hl.as_deref(), Some("en"));
        assert_eq!(request.gl.as_deref(), Some("US"));
        assert_eq!(
            query_pairs(&request.url),
            vec![
                ("q".to_owned(), "javascript".to_owned()),
                ("hl".to_owned(), "en".to_owned()),
                ("gl".to_owned(), "US".to_owned()),
            ]
        );
    }

    #[test]
    fn response_mapping_filters_invalid_suggestions_and_uses_metadata_fallbacks() {
        let fallback = build_suggestions_request(
            &json!({"q":" fallback ","hl":"en","gl":"us"}),
            &Url::parse(SCRAPPA_API_BASE_URL).unwrap(),
        )
        .unwrap();
        let items = suggestions_to_dataset_items(
            &json!({
                "query":" returned query ",
                "locale":{"hl":" fr ","gl":"ca"},
                "suggestions":[" first ","  ",17,"second"]
            }),
            &fallback,
        );
        assert_eq!(
            items,
            vec![
                json!({"query":"returned query","suggestion":" first ","position":1,"hl":"fr","gl":"CA"}),
                json!({"query":"returned query","suggestion":"second","position":2,"hl":"fr","gl":"CA"}),
            ]
        );

        let fallback_items =
            suggestions_to_dataset_items(&json!({"suggestions":["one"]}), &fallback);
        assert_eq!(
            fallback_items,
            vec![json!({
                "query":"fallback","suggestion":"one","position":1,"hl":"en","gl":"US"
            })]
        );

        let no_locales = build_suggestions_request(
            &json!({"q":"news"}),
            &Url::parse(SCRAPPA_API_BASE_URL).unwrap(),
        )
        .unwrap();
        assert_eq!(
            suggestions_to_dataset_items(&json!({"suggestions":["one"]}), &no_locales),
            vec![json!({"query":"news","suggestion":"one","position":1})]
        );
    }

    #[tokio::test]
    async fn local_apify_and_scrappa_mocks_preserve_request_auth_and_dataset_rows() {
        let input = json!({"q":" javascript tutorial ","hl":" en ","gl":"us"});
        let server = MockServer::start(vec![
            response(200, &input.to_string()),
            response(
                200,
                r#"{"query":"","locale":{"hl":null},"suggestions":["javascript","",17,"javascript tutorial"]}"#,
            ),
            pricing_response(Some(1.0), json!({"apify-default-dataset-item":0})),
            response(201, "{}"),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert!(has_test_bearer_token(&requests[0]));
        assert!(request_parts(&requests[1])
            .1
            .starts_with("/search/suggestions?"));
        assert_eq!(request_parts(&requests[2]).1, "/v2/actor-runs/test-run");
        assert!(has_test_bearer_token(&requests[2]));
        let (method, path, body) = request_parts(&requests[3]);
        assert_eq!((method, path), ("POST", "/v2/datasets/test-dataset/items"));
        assert!(has_test_bearer_token(&requests[3]));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([
                {"query":"javascript tutorial","suggestion":"javascript","position":1,"hl":"en","gl":"US"},
                {"query":"javascript tutorial","suggestion":"javascript tutorial","position":2,"hl":"en","gl":"US"}
            ])
        );
        assert!(!has_test_bearer_token(&requests[1]));
        assert!(requests[1]
            .to_ascii_lowercase()
            .contains("accept: application/json"));
    }

    #[tokio::test]
    async fn capped_run_posts_only_the_affordable_prefix() {
        let server = MockServer::start(vec![
            response(200, r#"{"q":"video"}"#),
            response(200, r#"{"suggestions":["first","second"]}"#),
            pricing_response(Some(0.0003), json!({"apify-default-dataset-item":0})),
            response(201, "{}"),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        let (_, _, body) = request_parts(&requests[3]);
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{"query":"video","suggestion":"first","position":1}])
        );
    }

    #[tokio::test]
    async fn zero_spending_limit_skips_dataset_post() {
        let server = MockServer::start(vec![
            response(200, r#"{"q":"video"}"#),
            response(200, r#"{"suggestions":["first","second"]}"#),
            pricing_response(Some(0.0), json!({"apify-default-dataset-item":0})),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));
    }

    #[tokio::test]
    async fn charged_non_dataset_events_reduce_available_capacity() {
        let server = MockServer::start(vec![
            response(200, r#"{"q":"video"}"#),
            response(200, r#"{"suggestions":["first","second"]}"#),
            pricing_response(
                Some(0.00035),
                json!({"apify-default-dataset-item":0,"apify-actor-start":1}),
            ),
            response(201, "{}"),
        ]);
        run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap();
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        let (_, _, body) = request_parts(&requests[3]);
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{"query":"video","suggestion":"first","position":1}])
        );
    }

    #[tokio::test]
    async fn successful_rows_are_counted_locally_across_dataset_writes() {
        let server = MockServer::start(vec![
            pricing_response(Some(0.0003), json!({"apify-default-dataset-item":0})),
            response(201, "{}"),
        ]);
        let config = config(&server.base_url);
        let mut budget = None;
        assert_eq!(
            push_dataset_items(
                &client(),
                &config,
                &mut budget,
                &[json!({"suggestion":"first"})]
            )
            .await
            .unwrap(),
            1
        );
        assert_eq!(
            push_dataset_items(
                &client(),
                &config,
                &mut budget,
                &[json!({"suggestion":"second"})]
            )
            .await
            .unwrap(),
            0
        );
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(request_parts(&requests[0]).1, "/v2/actor-runs/test-run");
        let (_, _, body) = request_parts(&requests[1]);
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{"suggestion":"first"}])
        );
    }

    #[tokio::test]
    async fn missing_spending_limit_fails_closed_without_dataset_rows() {
        let server = MockServer::start(vec![
            response(200, r#"{"q":"video"}"#),
            response(200, r#"{"suggestions":["first","second"]}"#),
            pricing_response(None, json!({"apify-default-dataset-item":0})),
        ]);
        let error = run_actor(&client(), &config(&server.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("spending limit"));
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/datasets/")));
    }
    #[tokio::test]
    async fn cloud_storage_errors_fail_and_upstream_http_errors_are_not_retried() {
        let input_failure = MockServer::start(vec![response(401, "unauthorized")]);
        let error = run_actor(&client(), &config(&input_failure.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("401 Unauthorized"));
        assert_eq!(input_failure.requests().len(), 1);

        let pricing_failure = MockServer::start(vec![
            response(200, r#"{"q":"video"}"#),
            response(200, r#"{"suggestions":["one"]}"#),
            response(500, "run unavailable"),
        ]);
        let error = run_actor(&client(), &config(&pricing_failure.base_url))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Apify run pricing request failed"));
        assert_eq!(pricing_failure.requests().len(), 3);

        let dataset_failure = MockServer::start(vec![
            response(200, r#"{"q":"video"}"#),
            response(200, r#"{"suggestions":["one"]}"#),
            pricing_response(Some(1.0), json!({"apify-default-dataset-item":0})),
            response(500, "dataset unavailable"),
        ]);
        let error = run_actor(&client(), &config(&dataset_failure.base_url))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("500 Internal Server Error"));
        assert_eq!(dataset_failure.requests().len(), 4);

        let upstream_failure = MockServer::start(vec![
            response(200, r#"{"q":"video"}"#),
            response(500, "upstream unavailable"),
        ]);
        let error = run_actor(&client(), &config(&upstream_failure.base_url))
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Scrappa API request failed with 500 Internal Server Error"));
        assert_eq!(upstream_failure.requests().len(), 2);
    }
}
