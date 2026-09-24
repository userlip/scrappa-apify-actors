mod apify;
mod config;
mod input;
mod response;
mod scrappa;

use std::process::ExitCode;

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};

use crate::{
    apify::{ApifyClient, ChargingBudget, ITEM_RESULT_CHARGE_EVENT},
    config::{ActorConfig, SCRAPPA_REQUEST_TIMEOUT},
    input::{build_page_params, build_search_plan, describe_search_request},
    response::{
        build_dataset_item, get_items, get_pagination, has_no_next_page, request_summary,
        total_entries, total_pages,
    },
    scrappa::{ScrappaClient, ScrappaTimeoutError},
};

const SCRAPPA_MAX_ATTEMPTS: usize = 3;

struct PushChargedItemsResult {
    saved_count: usize,
    status_message: Option<String>,
}

async fn push_charged_items(
    apify: &ApifyClient,
    config: &ActorConfig,
    budget: &mut Option<ChargingBudget>,
    items: &[Value],
    page: u64,
) -> Result<PushChargedItemsResult> {
    if items.is_empty() {
        return Ok(PushChargedItemsResult {
            saved_count: 0,
            status_message: None,
        });
    }

    if budget.is_none() {
        *budget = Some(apify.get_charging_budget(&config.actor_run_id).await?);
    }

    match budget.as_mut().expect("charging budget initialized") {
        ChargingBudget::NonPayPerEvent => {
            apify.push_data(&config.dataset_id, items).await?;
            Ok(PushChargedItemsResult {
                saved_count: items.len(),
                status_message: None,
            })
        }
        ChargingBudget::PayPerEvent(ppe_budget) => {
            let saved_count = ppe_budget.affordable_items(items.len())?;
            if saved_count > 0 {
                apify
                    .push_data(&config.dataset_id, &items[..saved_count])
                    .await?;
                apify
                    .charge_event(&config.actor_run_id, ITEM_RESULT_CHARGE_EVENT, saved_count)
                    .await?;
                ppe_budget.record_saved_items(saved_count)?;
            }

            let status_message = (saved_count < items.len()).then(|| {
                format!(
                    "Charge limit reached after saving {saved_count} of {} Vinted result(s) on page {page}.",
                    items.len()
                )
            });
            if let Some(status_message) = &status_message {
                println!(
                    "{status_message} {}",
                    json!({
                        "event": ITEM_RESULT_CHARGE_EVENT,
                        "charged_count": saved_count,
                        "requested_count": items.len(),
                        "saved_count": saved_count,
                        "page": page
                    })
                );
            }
            Ok(PushChargedItemsResult {
                saved_count,
                status_message,
            })
        }
    }
}

async fn run_actor(
    apify: &ApifyClient,
    scrappa: &ScrappaClient,
    config: &ActorConfig,
) -> Result<()> {
    let input = apify
        .get_input(&config.key_value_store_id, &config.input_key)
        .await?
        .unwrap_or_else(|| Value::Object(Map::new()));
    let plan = build_search_plan(&input)?;
    println!("Searching Vinted for {}", describe_search_request(&plan));

    let mut responses = Vec::new();
    let mut pages_fetched = 0_u64;
    let mut saved_items = 0_usize;
    let mut status_message = None;
    let mut latest_pagination: Option<Value> = None;
    let mut charging_budget = None;

    for offset in 0..plan.max_pages {
        let page = plan.start_page + offset;
        let params = build_page_params(&plan, page);
        let country = params
            .get("country")
            .and_then(Value::as_str)
            .unwrap_or("FR");
        let query = params
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("none");
        println!("Fetching Vinted page {page} in {country} for query {query}");

        let response = scrappa
            .get("/vinted/search", &params, SCRAPPA_MAX_ATTEMPTS)
            .await?;
        pages_fetched += 1;
        latest_pagination = get_pagination(&response).cloned();
        let items = get_items(&response)
            .iter()
            .map(|item| build_dataset_item(item, &params, &response))
            .collect::<Vec<_>>();
        responses.push(response);

        if items.is_empty() {
            println!("No Vinted listings found on page {page}");
            break;
        }

        let result = push_charged_items(apify, config, &mut charging_budget, &items, page).await?;
        saved_items += result.saved_count;
        println!(
            "Found {} listing(s) on page {page}; saved {}",
            items.len(),
            result.saved_count
        );
        if result.status_message.is_some() {
            status_message = result.status_message;
            break;
        }

        if has_no_next_page(latest_pagination.as_ref(), page) {
            println!("Stopping after page {page}; Scrappa reported no additional Vinted pages");
            break;
        }
    }

    let output = json!({
        "request": request_summary(&plan),
        "pages_fetched": pages_fetched,
        "responses_saved": responses.len(),
        "items_extracted": saved_items,
        "status_message": status_message,
        "total_pages": total_pages(latest_pagination.as_ref()),
        "total_entries": total_entries(latest_pagination.as_ref()),
        "responses": responses
    });
    apify
        .set_output(&config.key_value_store_id, &output)
        .await?;

    println!("Vinted search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "pages_fetched": pages_fetched,
            "responses_saved": responses.len(),
            "items_extracted": saved_items,
            "total_pages": output["total_pages"],
            "total_entries": output["total_entries"]
        })
    );

    if let Some(status_message) = status_message {
        apify
            .set_status_message(&config.actor_run_id, &status_message, true)
            .await?;
    }
    Ok(())
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "Scrappa API request timed out after {}ms. The Vinted search request exceeded the {}s Scrappa API timeout. Try fewer pages, a narrower filter, or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_millis(),
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        error.to_string()
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify = ApifyClient::new(
        config.apify_api_base_url.clone(),
        config.apify_token.clone(),
    )?;
    let result = async {
        let api_key = config.require_scrappa_api_key()?;
        let scrappa = ScrappaClient::new(config.scrappa_api_base_url.clone(), api_key.to_owned())?;
        run_actor(&apify, &scrappa, &config).await
    }
    .await;

    if let Err(error) = result {
        let message = actor_error_message(&error);
        let _ = apify
            .set_status_message(&config.actor_run_id, &message, true)
            .await;
        return Err(anyhow!("{message}"));
    }
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
    use reqwest::Url;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver},
            Arc,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

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
            let mut responses = responses.into_iter();
            Self::start_with_handler(move |_| {
                responses.next().expect("unexpected HTTP request")
            })
        }

        fn start_with_handler(
            mut response_for_request: impl FnMut(&str) -> MockResponse + Send + 'static,
        ) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (request_sender, requests) = mpsc::channel();
            let stop = Arc::new(AtomicBool::new(false));
            let stopped = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                loop {
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
                    let response = response_for_request(&request);
                    if request_sender.send(request).is_err() {
                        return;
                    }
                    let reason = match response.status {
                        201 => "Created",
                        404 => "Not Found",
                        503 => "Service Unavailable",
                        _ => "OK",
                    };
                    let reply = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    let _ = stream.write_all(reply.as_bytes());
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

    fn mock_response(status: u16, body: Value) -> MockResponse {
        MockResponse {
            status,
            body: body.to_string(),
        }
    }

    fn request_parts(request: &str) -> (&str, &str) {
        let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
        let path = headers
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or_default();
        (path, body)
    }

    fn actor_config(base_url: &Url) -> ActorConfig {
        let mut scrappa_api_base_url = base_url.clone();
        scrappa_api_base_url.set_path("/api");
        ActorConfig {
            apify_api_base_url: base_url.clone(),
            scrappa_api_base_url,
            key_value_store_id: "test-store".to_owned(),
            input_key: "INPUT".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            apify_token: "test-token".to_owned(),
            scrappa_api_key: Some("test-api-key".to_owned()),
        }
    }

    fn ppe_run_response(max_total_charge_usd: f64, charged_counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "item-result": {"eventPriceUsd": 0.0002},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0001},
                        "apify-actor-start": {"eventPriceUsd": 0.005}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_total_charge_usd},
                "chargedEventCounts": charged_counts
            }
        })
    }

    #[test]
    fn timeout_failure_keeps_the_helpful_actor_message() {
        let error = anyhow::Error::new(ScrappaTimeoutError::new(SCRAPPA_REQUEST_TIMEOUT));
        assert_eq!(
            actor_error_message(&error),
            "Scrappa API request timed out after 90000ms. The Vinted search request exceeded the 90s Scrappa API timeout. Try fewer pages, a narrower filter, or run the request again."
        );
    }

    #[tokio::test]
    async fn paginates_until_scrappa_reports_no_next_page() {
        let server = MockServer::start(vec![
            mock_response(
                200,
                json!({"query":"nike shoes","country":"DE","max_pages":3}),
            ),
            mock_response(
                200,
                json!({
                    "items": [{"id": "first", "title": "First listing"}],
                    "pagination": {"has_next_page": true, "total_pages": 4, "total_entries": 30}
                }),
            ),
            mock_response(
                200,
                json!({"data":{"pricingInfo":{"pricingModel":"PRICE_PER_DATASET_ITEM"}}}),
            ),
            mock_response(201, json!({})),
            mock_response(
                200,
                json!({
                    "items": [{"id": "second", "title": "Second listing"}],
                    "pagination": {"has_next_page": false, "total_pages": 4, "total_entries": 30}
                }),
            ),
            mock_response(201, json!({})),
            mock_response(200, json!({})),
        ]);
        let config = actor_config(&server.base_url);
        let apify =
            ApifyClient::new(config.apify_api_base_url.clone(), "test-token".to_owned()).unwrap();
        let scrappa = ScrappaClient::new(
            config.scrappa_api_base_url.clone(),
            "test-api-key".to_owned(),
        )
        .unwrap();

        run_actor(&apify, &scrappa, &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 7);
        assert!(
            requests[1].starts_with("GET /api/vinted/search?"),
            "{:?}",
            requests[1]
        );
        assert!(requests[1].contains("page=1"));
        assert!(requests[4].starts_with("GET /api/vinted/search?"));
        assert!(requests[4].contains("page=2"));
        assert!(requests[3].starts_with("POST /v2/datasets/test-dataset/items "));
        assert!(requests[5].starts_with("POST /v2/datasets/test-dataset/items "));
        let (output_path, output_body) = request_parts(&requests[6]);
        assert_eq!(
            output_path,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
        let output: Value = serde_json::from_str(output_body).unwrap();
        assert_eq!(output["pages_fetched"], json!(2));
        assert_eq!(output["responses_saved"], json!(2));
        assert_eq!(output["items_extracted"], json!(2));
        assert_eq!(output["total_entries"], json!(30));
        assert_eq!(output["status_message"], Value::Null);
        assert_eq!(output["responses"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn partial_pay_per_event_charge_saves_only_affordable_rows_and_stops() {
        let server = MockServer::start(vec![
            mock_response(
                200,
                json!({
                    "query": "nike shoes",
                    "country": "DE",
                    "max_pages": 2,
                    "order": "newest_first"
                }),
            ),
            mock_response(
                200,
                json!({
                    "items": [
                        {"id": "first", "title": "First listing"},
                        {"id": "second", "title": "Second listing"}
                    ],
                    "pagination": {"has_next_page": true, "total_pages": 4, "total_entries": 30}
                }),
            ),
            mock_response(
                200,
                ppe_run_response(0.0053, json!({"apify-actor-start": 1})),
            ),
            mock_response(201, json!({})),
            mock_response(201, json!({})),
            mock_response(200, json!({})),
            mock_response(200, json!({})),
        ]);
        let config = actor_config(&server.base_url);
        let apify =
            ApifyClient::new(config.apify_api_base_url.clone(), "test-token".to_owned()).unwrap();
        let scrappa = ScrappaClient::new(
            config.scrappa_api_base_url.clone(),
            "test-api-key".to_owned(),
        )
        .unwrap();

        run_actor(&apify, &scrappa, &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 7);
        assert_eq!(request_parts(&requests[2]).0, "/v2/actor-runs/test-run");
        assert_eq!(
            request_parts(&requests[3]).0,
            "/v2/datasets/test-dataset/items"
        );
        let saved_rows: Value = serde_json::from_str(request_parts(&requests[3]).1).unwrap();
        assert_eq!(saved_rows.as_array().unwrap().len(), 1);
        assert_eq!(saved_rows[0]["id"], json!("first"));
        assert_eq!(saved_rows[0]["request_page"], json!(1));
        assert_eq!(
            request_parts(&requests[4]).0,
            "/v2/actor-runs/test-run/charge"
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[4]).1).unwrap(),
            json!({"eventName": "item-result", "count": 1})
        );
        assert_eq!(
            request_parts(&requests[5]).0,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
        let output: Value = serde_json::from_str(request_parts(&requests[5]).1).unwrap();
        assert_eq!(output["pages_fetched"], json!(1));
        assert_eq!(output["items_extracted"], json!(1));
        assert_eq!(
            output["status_message"],
            json!("Charge limit reached after saving 1 of 2 Vinted result(s) on page 1.")
        );
        assert_eq!(output["total_pages"], json!(4));
        assert_eq!(output["total_entries"], json!(30));
        assert_eq!(output["responses"].as_array().unwrap().len(), 1);
        assert_eq!(request_parts(&requests[6]).0, "/v2/actor-runs/test-run");
        let status: Value = serde_json::from_str(request_parts(&requests[6]).1).unwrap();
        assert_eq!(
            status["statusMessage"],
            json!("Charge limit reached after saving 1 of 2 Vinted result(s) on page 1.")
        );
        assert_eq!(status["isStatusMessageTerminal"], json!(true));
        assert!(
            requests.iter().all(|request| !request.contains("&page=2")),
            "{requests:#?}"
        );
    }

    #[tokio::test]
    async fn failed_dataset_write_does_not_charge_or_retry_the_append() {
        let server = MockServer::start_with_handler(|request| {
            if request.starts_with("GET /v2/actor-runs/test-run ") {
                mock_response(
                    200,
                    ppe_run_response(1.0, json!({"apify-actor-start": 1})),
                )
            } else if request.starts_with("POST /v2/datasets/test-dataset/items ") {
                MockResponse {
                    status: 503,
                    body: json!({"error": "dataset unavailable"}).to_string(),
                }
            } else if request.starts_with("POST /v2/actor-runs/test-run/charge ") {
                mock_response(201, json!({}))
            } else {
                panic!("unexpected request: {request}");
            }
        });
        let config = actor_config(&server.base_url);
        let apify =
            ApifyClient::new(config.apify_api_base_url.clone(), "test-token".to_owned()).unwrap();
        let items = vec![json!({"id": "listing-1"})];
        let mut budget = None;

        let result = push_charged_items(&apify, &config, &mut budget, &items, 1).await;

        let error = result.err().unwrap().to_string();
        assert!(error.contains("store dataset items"), "{error}");
        let requests = server.requests();
        assert!(requests
            .iter()
            .all(|request| !request.starts_with("POST /v2/actor-runs/test-run/charge ")));
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.starts_with("POST /v2/datasets/test-dataset/items "))
                .count(),
            1
        );
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("GET /v2/actor-runs/test-run "));
        assert!(requests[1].starts_with("POST /v2/datasets/test-dataset/items "));
    }

    #[test]
    fn partial_charge_message_matches_dataset_output_contract() {
        let message = format!(
            "Charge limit reached after saving {} of {} Vinted result(s) on page {}.",
            2, 4, 3
        );
        assert_eq!(
            message,
            "Charge limit reached after saving 2 of 4 Vinted result(s) on page 3."
        );
        assert_eq!(crate::config::OUTPUT_KEY, "OUTPUT");
    }

    #[test]
    fn schema_contract_keeps_existing_default_and_prefill_values() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["query"]["prefill"],
            json!("nike shoes")
        );
        assert_eq!(schema["properties"]["country"]["default"], json!("FR"));
        assert_eq!(schema["properties"]["country"]["prefill"], json!("DE"));
        assert_eq!(schema["properties"]["order"]["default"], json!("relevance"));
        assert_eq!(
            schema["properties"]["order"]["prefill"],
            json!("newest_first")
        );
        assert_eq!(schema["properties"]["max_pages"]["maximum"], json!(20));

        let qa_input = json!({
            "query": schema["properties"]["query"]["prefill"],
            "country": schema["properties"]["country"]["prefill"],
            "page": schema["properties"]["page"]["default"],
            "per_page": schema["properties"]["per_page"]["default"],
            "max_pages": schema["properties"]["max_pages"]["default"],
            "order": schema["properties"]["order"]["prefill"]
        });
        let plan = build_search_plan(&qa_input).unwrap();
        assert_eq!(plan.base_params["country"], json!("DE"));
        assert_eq!(plan.base_params["query"], json!("nike shoes"));
        assert_eq!(plan.base_params["order"], json!("newest_first"));
        assert_eq!(plan.per_page, 24);
        assert_eq!(plan.max_pages, 1);
    }
}
