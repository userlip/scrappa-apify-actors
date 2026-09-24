mod apify;
mod request_params;
mod response_utils;
mod scrappa;

use anyhow::{Context, Result, anyhow};
use apify::{ApifyClient, DatasetBudget};
use request_params::{
    StartpageSearchPlan, build_startpage_search_plan, describe_startpage_search_plan,
};
use reqwest::Url;
use response_utils::{build_startpage_dataset_item, extract_startpage_organic_results};
use scrappa::ScrappaClient;
use serde_json::{Value, json};
use std::{env, process};

const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const ENDPOINT: &str = "/startpage/search";

struct Config {
    apify_api_base_url: Url,
    scrappa_api_base_url: String,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
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
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: env::var("SCRAPPA_API_BASE_URL")
                .unwrap_or_else(|_| SCRAPPA_API_BASE_URL.to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        anyhow::bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn maximum_output_rows(plan: &StartpageSearchPlan) -> usize {
    plan.requests
        .len()
        .saturating_mul(plan.max_results_per_query)
}

fn build_output(
    plan: &StartpageSearchPlan,
    queries_fetched: usize,
    results_extracted: usize,
    results_saved: usize,
    charge_limit_reached: bool,
) -> Value {
    json!({
        "requests": plan.requests.iter().map(|request| &request.params).collect::<Vec<_>>(),
        "queries_requested": plan.requests.len(),
        "queries_fetched": queries_fetched,
        "results_extracted": results_extracted,
        "results_saved": results_saved,
        "max_results_per_query": plan.max_results_per_query,
        "charge_limit_reached": charge_limit_reached
    })
}

async fn run_actor(config: &Config) -> Result<()> {
    let apify = ApifyClient::new(
        config.apify_api_base_url.as_str(),
        config.apify_token.clone(),
    )?;
    let input = apify
        .get_input(&config.key_value_store_id, &config.input_key)
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let plan = build_startpage_search_plan(&input).map_err(anyhow::Error::msg)?;
    println!(
        "Fetching Startpage search results for {}",
        describe_startpage_search_plan(&plan)
    );

    let scrappa = ScrappaClient::new(
        config.scrappa_api_key.clone(),
        config.scrappa_api_base_url.clone(),
    )?;
    let mut fetched_queries = 0;
    let mut extracted_results = 0;
    let mut saved_results = 0;
    let mut dataset_budget: Option<DatasetBudget> = None;
    let mut charge_limit_reached = false;

    for (index, request) in plan.requests.iter().enumerate() {
        if dataset_budget
            .as_ref()
            .is_some_and(|budget| budget.remaining_items() == 0)
        {
            charge_limit_reached = true;
            break;
        }

        println!("Fetching Startpage results for \"{}\"", request.query);
        let response = scrappa.get(ENDPOINT, &request.params).await?;
        fetched_queries += 1;

        let organic_results = extract_startpage_organic_results(&response);
        let capped_results = organic_results
            .iter()
            .take(plan.max_results_per_query)
            .map(|result| build_startpage_dataset_item(result, &request.params, &response))
            .collect::<Vec<_>>();
        extracted_results += organic_results.len();

        if !capped_results.is_empty() {
            if dataset_budget.is_none() {
                dataset_budget = Some(
                    apify
                        .dataset_item_budget(&config.actor_run_id, maximum_output_rows(&plan))
                        .await?,
                );
            }
            let budget = dataset_budget
                .as_mut()
                .context("Dataset item budget was not initialized")?;
            let saved = apify
                .push_data(&config.dataset_id, &capped_results, budget)
                .await?;
            saved_results += saved;
            println!(
                "Found {} Startpage result(s) for \"{}\"; saved {saved}",
                organic_results.len(),
                request.query
            );

            if saved < capped_results.len()
                || (budget.remaining_items() == 0 && index + 1 < plan.requests.len())
            {
                charge_limit_reached = true;
                break;
            }
        } else {
            println!("Found no Startpage results for \"{}\"", request.query);
        }
    }

    let output = build_output(
        &plan,
        fetched_queries,
        extracted_results,
        saved_results,
        charge_limit_reached,
    );
    apify
        .set_output(&config.key_value_store_id, &output)
        .await?;
    println!("Startpage search completed successfully");
    println!("Results summary: {output}");
    if charge_limit_reached {
        println!("Pay-per-event charge limit reached; remaining queries were not fetched");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            process::exit(1);
        }
    };
    if let Err(error) = run_actor(&config).await {
        let message = if error.to_string().contains("timed out") {
            format!(
                "{error}. The Startpage request exceeded the {}s Scrappa API timeout. Try fewer queries or run the request again.",
                scrappa::REQUEST_TIMEOUT.as_secs()
            )
        } else {
            format!("{error:#}")
        };
        eprintln!("Actor failed: {message}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        thread,
        time::{Duration, Instant},
    };

    fn test_config(base_url: &str) -> Config {
        let base_url = Url::parse(base_url).unwrap();
        Config {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url: base_url.to_string(),
            apify_token: "test-apify-token".to_owned(),
            key_value_store_id: "test-store".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "test-scrappa-key".to_owned(),
        }
    }

    fn mock_server(responses: Vec<(u16, String)>) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let thread = thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let deadline = Instant::now() + Duration::from_secs(10);
                let (mut stream, _) = loop {
                    match listener.accept() {
                        Ok(connection) => break connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if Instant::now() >= deadline {
                                return requests;
                            }
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => return requests,
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                requests.push(read_request(&mut stream));
                let reason = match status {
                    200 => "OK",
                    201 => "Created",
                    429 => "Too Many Requests",
                    500 => "Internal Server Error",
                    503 => "Service Unavailable",
                    _ => "Mock Response",
                };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                if stream.write_all(response.as_bytes()).is_err() {
                    return requests;
                }
            }
            requests
        });
        (format!("http://{address}"), thread)
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let Ok(read) = stream.read(&mut buffer) else {
                break;
            };
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn json_response(status: u16, body: Value) -> (u16, String) {
        (status, body.to_string())
    }

    fn request_body(request: &str) -> Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    fn input_response(input: Value) -> (u16, String) {
        json_response(200, input)
    }

    fn normal_ppe_run(max_charge: f64) -> (u16, String) {
        json_response(
            200,
            json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "apify-default-dataset-item": {"eventPriceUsd": 0.1}
                        }}
                    },
                    "chargedEventCounts": {},
                    "options": {"maxTotalChargeUsd": max_charge}
                }
            }),
        )
    }

    #[test]
    fn output_summarizes_batch_progress_and_budget_state() {
        let plan = build_startpage_search_plan(&json!({
            "queries": [{"query": "privacy tools"}, {"query": "private search", "page": 1}],
            "max_results_per_query": 5
        }))
        .unwrap();
        assert_eq!(
            build_output(&plan, 1, 8, 5, true),
            json!({
                "requests": [
                    {"query": "privacy tools"},
                    {"query": "private search", "page": 1}
                ],
                "queries_requested": 2,
                "queries_fetched": 1,
                "results_extracted": 8,
                "results_saved": 5,
                "max_results_per_query": 5,
                "charge_limit_reached": true
            })
        );
    }

    #[test]
    fn caps_total_budget_request_to_the_plan_maximum() {
        let plan = build_startpage_search_plan(&json!({
            "queries": [{"query": "one"}, {"query": "two"}],
            "max_results_per_query": 10
        }))
        .unwrap();
        assert_eq!(maximum_output_rows(&plan), 20);
    }

    #[tokio::test]
    async fn calls_scrappa_and_writes_dataset_and_output_with_auth_and_pagination() {
        let input = json!({
            "queries": [
                {"query": "privacy tools", "language": "english", "page": 0, "safe_search": true},
                {"query": "private search"}
            ],
            "max_results_per_query": 1
        });
        let first_response = json!({
            "data": [
                {"position": 1, "title": "Privacy one", "url": "https://one.example"},
                {"position": 2, "title": "Privacy two", "url": "https://two.example"}
            ],
            "source": "startpage",
            "total_results": 20,
            "pagination": {"current": 0},
            "scrappa_pagination": {"page": 0}
        });
        let second_response = json!({
            "organic_results": [{"position": 1, "title": "Private search"}],
            "total_results": 1
        });
        let (base_url, server) = mock_server(vec![
            input_response(input),
            json_response(200, first_response),
            normal_ppe_run(1.0),
            (201, String::new()),
            json_response(200, second_response),
            (201, String::new()),
            (201, String::new()),
        ]);

        run_actor(&test_config(&base_url)).await.unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 7);

        let input_request = requests[0].to_ascii_lowercase();
        assert!(
            input_request.starts_with("get /v2/key-value-stores/test-store/records/input http/1.1")
        );
        assert!(input_request.contains("authorization: bearer test-apify-token"));

        let first_scrappa_request = requests[1].to_ascii_lowercase();
        assert!(first_scrappa_request.starts_with("get /startpage/search?"));
        assert!(first_scrappa_request.contains("query=privacy+tools"));
        assert!(first_scrappa_request.contains("language=english"));
        assert!(first_scrappa_request.contains("page=0"));
        assert!(first_scrappa_request.contains("safe_search=1"));
        assert!(first_scrappa_request.contains("x-api-key: test-scrappa-key"));
        assert!(first_scrappa_request.contains("accept: application/json"));
        assert!(!first_scrappa_request.contains("authorization:"));

        assert!(
            requests[2]
                .to_ascii_lowercase()
                .starts_with("get /v2/actor-runs/test-run http/1.1")
        );
        let first_dataset_request = requests[3].to_ascii_lowercase();
        assert!(first_dataset_request.starts_with("post /v2/datasets/test-dataset/items http/1.1"));
        assert!(first_dataset_request.contains("authorization: bearer test-apify-token"));
        assert_eq!(
            request_body(&requests[3]),
            json!([{
                "position": 1,
                "title": "Privacy one",
                "url": "https://one.example",
                "query": "privacy tools",
                "description": null,
                "domain": null,
                "source": "startpage",
                "request_query": "privacy tools",
                "request_language": "english",
                "request_page": 0,
                "request_safe_search": 1,
                "total_results": 20,
                "pagination": {"current": 0},
                "scrappa_pagination": {"page": 0}
            }])
        );

        let second_dataset_request = requests[5].to_ascii_lowercase();
        assert!(
            second_dataset_request.starts_with("post /v2/datasets/test-dataset/items http/1.1")
        );
        assert!(
            requests[6]
                .to_ascii_lowercase()
                .starts_with("put /v2/key-value-stores/test-store/records/output http/1.1")
        );
        assert_eq!(
            request_body(&requests[6]),
            json!({
                "requests": [
                    {"query": "privacy tools", "language": "english", "page": 0, "safe_search": 1},
                    {"query": "private search"}
                ],
                "queries_requested": 2,
                "queries_fetched": 2,
                "results_extracted": 3,
                "results_saved": 2,
                "max_results_per_query": 1,
                "charge_limit_reached": false
            })
        );
    }

    #[tokio::test]
    async fn limits_dataset_rows_and_stops_remaining_queries_at_the_ppe_cap() {
        let input = json!({
            "queries": [{"query": "first"}, {"query": "second"}],
            "max_results_per_query": 2
        });
        let response = json!({
            "data": [
                {"position": 1, "title": "First result"},
                {"position": 2, "title": "Second result"}
            ]
        });
        let (base_url, server) = mock_server(vec![
            input_response(input),
            json_response(200, response),
            normal_ppe_run(0.1),
            (201, String::new()),
            (201, String::new()),
        ]);

        run_actor(&test_config(&base_url)).await.unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 5);
        assert_eq!(request_body(&requests[3]).as_array().unwrap().len(), 1);
        assert_eq!(request_body(&requests[3])[0]["title"], "First result");
        assert_eq!(request_body(&requests[4])["queries_fetched"], 1);
        assert_eq!(request_body(&requests[4])["results_extracted"], 2);
        assert_eq!(request_body(&requests[4])["results_saved"], 1);
        assert_eq!(request_body(&requests[4])["charge_limit_reached"], true);
    }

    #[tokio::test]
    async fn does_not_retry_dataset_post_when_the_response_is_transient() {
        let (base_url, server) = mock_server(vec![
            normal_ppe_run(0.1),
            (503, "temporarily unavailable".to_owned()),
        ]);
        let client = ApifyClient::new(&base_url, "test-apify-token".to_owned()).unwrap();
        let mut budget = client.dataset_item_budget("test-run", 1).await.unwrap();

        let result = tokio::time::timeout(
            Duration::from_secs(1),
            client.push_data("test-dataset", &[json!({"title": "First result"})], &mut budget),
        )
        .await
        .expect("dataset POST should return without a retry");

        assert!(result.is_err());
        assert_eq!(budget.remaining_items(), 1);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].to_ascii_lowercase().starts_with(
            "get /v2/actor-runs/test-run http/1.1"
        ));
        assert!(requests[1]
            .to_ascii_lowercase()
            .starts_with("post /v2/datasets/test-dataset/items http/1.1"));
    }

    #[tokio::test]
    async fn retries_transient_apify_storage_errors_and_does_not_retry_scrappa_errors() {
        let input = json!({"queries": [{"query": "privacy tools"}]});
        let (base_url, server) = mock_server(vec![
            (503, "temporarily unavailable".to_owned()),
            input_response(input),
            json_response(200, json!({})),
            (201, String::new()),
        ]);
        run_actor(&test_config(&base_url)).await.unwrap();
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 4);
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .starts_with("get /v2/key-value-stores/test-store/records/input")
        );
        assert!(
            requests[1]
                .to_ascii_lowercase()
                .starts_with("get /v2/key-value-stores/test-store/records/input")
        );

        let (base_url, server) = mock_server(vec![
            input_response(json!({"queries": [{"query": "privacy tools"}]})),
            (503, "upstream unavailable".to_owned()),
        ]);
        let error = run_actor(&test_config(&base_url)).await.unwrap_err();
        assert!(error.to_string().contains("Scrappa API error (503)"));
        assert_eq!(server.join().unwrap().len(), 2);
    }
}
