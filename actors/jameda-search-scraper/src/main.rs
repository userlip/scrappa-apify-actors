mod apify;
mod request_params;
mod response_utils;
mod scrappa_client;

use anyhow::{anyhow, Result};
use apify::{is_pay_per_event, ApifyClient, ApifyConfig, PpeBudget};
use request_params::{
    build_jameda_search_plans, build_page_params, describe_jameda_search_request, JamedaSearchPlan,
};
use response_utils::{build_jameda_doctor_dataset_item, get_jameda_doctors};
use scrappa_client::{ScrappaClient, ScrappaTimeoutError, REQUEST_TIMEOUT_MS};
use serde_json::{json, Value};
use std::{env, process};

fn required_api_key() -> Result<String> {
    env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })
}

fn optional_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

fn actor_error_message(error: &anyhow::Error) -> String {
    let raw_message = error.to_string();
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{raw_message}. The Jameda search request exceeded the {}s Scrappa API timeout. Try fewer pages or run the request again.",
            REQUEST_TIMEOUT_MS / 1000
        )
    } else {
        format!("{error:#}")
    }
}

fn response_meta_value(response: &Value, key: &str) -> Value {
    response
        .get("meta")
        .filter(|meta| !meta.is_null())
        .and_then(|meta| meta.get(key))
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn response_has_no_next_page(response: &Value) -> bool {
    response
        .get("meta")
        .and_then(|meta| meta.get("has_next_page"))
        .and_then(Value::as_bool)
        == Some(false)
}

fn response_total_pages_reached(response: &Value, page: u64) -> bool {
    response
        .get("meta")
        .and_then(|meta| meta.get("total_pages"))
        .and_then(Value::as_f64)
        .is_some_and(|total_pages| page as f64 >= total_pages)
}

fn search_summary(
    plan: &JamedaSearchPlan,
    pages_fetched: usize,
    doctors_extracted: usize,
    last_response: Option<&Value>,
) -> Value {
    let mut request = plan.base_params.clone();
    request.insert("start_page".to_owned(), json!(plan.start_page));
    request.insert("max_pages".to_owned(), json!(plan.max_pages));
    json!({
        "request": request,
        "pages_fetched": pages_fetched,
        "doctors_extracted": doctors_extracted,
        "total_results": last_response.map(|response| response_meta_value(response, "total_results")).unwrap_or(Value::Null),
        "total_pages": last_response.map(|response| response_meta_value(response, "total_pages")).unwrap_or(Value::Null),
    })
}

async fn save_doctor_results(
    apify: &ApifyClient,
    items: &[Value],
    ppe_budget: &mut Option<PpeBudget>,
    pay_per_event: &mut Option<bool>,
) -> Result<(usize, bool)> {
    if items.is_empty() {
        return Ok((0, false));
    }
    if pay_per_event.is_none() {
        let run = apify.get_run().await?;
        let is_ppe = is_pay_per_event(&run)?;
        *pay_per_event = Some(is_ppe);
        if is_ppe {
            *ppe_budget = Some(PpeBudget::from_run(&run)?);
        }
    }
    if pay_per_event.as_ref() == Some(&true) {
        let budget = ppe_budget
            .as_mut()
            .ok_or_else(|| anyhow!("Apify pay-per-event budget was not initialized"))?;
        let result = apify.push_charged_items(items, budget).await?;
        Ok((result.saved_count, result.event_charge_limit_reached))
    } else {
        apify.push_dataset_items(items).await?;
        Ok((items.len(), false))
    }
}

async fn execute(apify: &ApifyClient) -> Result<Option<String>> {
    let api_key = required_api_key()?;
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let client = ScrappaClient::new(api_key, optional_env("SCRAPPA_API_BASE_URL").as_deref())?;
    execute_with_input(apify, &client, input).await
}

async fn execute_with_input(
    apify: &ApifyClient,
    client: &ScrappaClient,
    input: Value,
) -> Result<Option<String>> {
    let plans = build_jameda_search_plans(&input)?;
    println!(
        "Running {} Jameda search{}",
        plans.len(),
        if plans.len() == 1 { "" } else { "es" }
    );

    let should_store_raw_responses = plans.len() == 1;
    let mut responses = Vec::new();
    let mut pages_fetched = 0;
    let mut saved_doctors = 0;
    let mut status_message = None;
    let mut latest_response = None;
    let mut ppe_budget = None;
    let mut pay_per_event = None;
    let mut search_summaries = Vec::with_capacity(plans.len());

    for plan in &plans {
        println!(
            "Searching Jameda for {}",
            describe_jameda_search_request(plan)
        );
        let mut search_pages_fetched = 0;
        let mut search_doctors_extracted = 0;
        let mut search_last_response = None;

        for offset in 0..plan.max_pages {
            let page = plan.start_page + offset;
            let params = build_page_params(plan, page);
            let query = params.get("q").and_then(Value::as_str).unwrap_or_default();
            let location = params
                .get("loc")
                .and_then(Value::as_str)
                .map(|location| format!(" in {location}"))
                .unwrap_or_default();
            println!("Fetching Jameda page {page} for {query}{location}");

            let response = client.get(&params).await?;
            pages_fetched += 1;
            search_pages_fetched += 1;
            latest_response = Some(response.clone());
            search_last_response = Some(response.clone());
            let doctors = get_jameda_doctors(&response)
                .iter()
                .map(|doctor| build_jameda_doctor_dataset_item(doctor, &params, &response))
                .collect::<Result<Vec<_>>>()?;
            if should_store_raw_responses {
                responses.push(response.clone());
            }

            if doctors.is_empty() {
                println!("No Jameda results found on page {page}");
                break;
            }
            let (saved_count, charge_limit_reached) =
                save_doctor_results(apify, &doctors, &mut ppe_budget, &mut pay_per_event).await?;
            saved_doctors += saved_count;
            search_doctors_extracted += saved_count;
            println!(
                "Found {} doctor result(s) on page {page}; saved {saved_count}",
                doctors.len()
            );

            if charge_limit_reached {
                let message = format!(
                    "Charge limit reached after saving {saved_count} of {} Jameda doctor results on page {page}.",
                    doctors.len()
                );
                println!(
                    "{message} {}",
                    json!({
                        "event": apify::DOCTOR_RESULT_CHARGE_EVENT,
                        "charged_count": saved_count,
                        "requested_count": doctors.len(),
                        "saved_count": saved_count,
                    })
                );
                status_message = Some(message);
                break;
            }
            if response_has_no_next_page(&response) {
                println!("Stopping after page {page}; Scrappa reported no next page");
                break;
            }
            if response_total_pages_reached(&response, page) {
                println!(
                    "Stopping after page {page}; Scrappa reported {} total page(s)",
                    response_meta_value(&response, "total_pages")
                );
                break;
            }
        }

        search_summaries.push(search_summary(
            plan,
            search_pages_fetched,
            search_doctors_extracted,
            search_last_response.as_ref(),
        ));
        if status_message.is_some() {
            break;
        }
    }

    let responses_saved = if should_store_raw_responses {
        responses.len()
    } else {
        0
    };
    let output = json!({
        "searches_requested": plans.len(),
        "searches": search_summaries,
        "pages_fetched": pages_fetched,
        "responses_saved": responses_saved,
        "doctors_extracted": saved_doctors,
        "status_message": status_message.clone(),
        "total_results": latest_response.as_ref().map(|response| response_meta_value(response, "total_results")).unwrap_or(Value::Null),
        "total_pages": latest_response.as_ref().map(|response| response_meta_value(response, "total_pages")).unwrap_or(Value::Null),
        "responses": if should_store_raw_responses { Value::Array(responses.clone()) } else { Value::Null },
    });
    let output = if should_store_raw_responses {
        output
    } else {
        let mut output = output;
        output.as_object_mut().unwrap().remove("responses");
        output
    };
    apify.put_output(&output).await?;

    println!("Jameda search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "pages_fetched": pages_fetched,
            "responses_saved": responses_saved,
            "doctors_extracted": saved_doctors,
            "total_results": output["total_results"],
            "total_pages": output["total_pages"],
        })
    );
    Ok(status_message)
}

async fn run() -> Result<()> {
    let config = ApifyConfig::from_env()?;
    let apify = ApifyClient::new(config)?;
    match execute(&apify).await {
        Ok(Some(status_message)) => {
            apify.set_terminal_status_message(&status_message).await?;
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(error) => {
            let message = actor_error_message(&error);
            eprintln!("Actor failed: {message}");
            if let Err(status_error) = apify.set_terminal_status_message(&message).await {
                eprintln!("Could not update the Apify run status message: {status_error:#}");
            }
            Err(anyhow!(message))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: String,
        requests: Receiver<String>,
        expected_requests: usize,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let expected_requests = responses.len();
            let thread = thread::spawn(move || {
                for response in responses {
                    let deadline = Instant::now() + Duration::from_secs(10);
                    let (mut stream, _) = loop {
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                assert!(
                                    Instant::now() < deadline,
                                    "actor did not make the expected API request"
                                );
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(error) => panic!("mock API accept failed: {error}"),
                        }
                    };
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let request = read_request(&mut stream);
                    sender
                        .send(String::from_utf8_lossy(&request).into_owned())
                        .unwrap();
                    let reason = reqwest::StatusCode::from_u16(response.status)
                        .unwrap()
                        .canonical_reason()
                        .unwrap_or("OK");
                    let reply = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                    stream.write_all(reply.as_bytes()).unwrap();
                }
            });
            Self {
                base_url: format!("http://{address}"),
                requests,
                expected_requests,
                thread: Some(thread),
            }
        }

        fn finish(mut self) -> Vec<String> {
            let mut requests = Vec::with_capacity(self.expected_requests);
            for _ in 0..self.expected_requests {
                requests.push(
                    self.requests
                        .recv_timeout(Duration::from_secs(2))
                        .expect("expected API request was not captured"),
                );
            }
            self.thread.take().unwrap().join().unwrap();
            requests
        }
    }

    fn read_request(stream: &mut TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let count = stream.read(&mut chunk).unwrap_or(0);
            if count == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..count]);
            let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            if request.len() >= header_end + 4 + content_length {
                break;
            }
        }
        request
    }

    fn mock_response(status: u16, body: Value) -> MockResponse {
        MockResponse {
            status,
            body: body.to_string(),
        }
    }

    fn request_line(request: &str) -> (&str, &str) {
        let mut pieces = request.lines().next().unwrap().split_whitespace();
        (pieces.next().unwrap(), pieces.next().unwrap())
    }

    fn request_url(request: &str) -> url::Url {
        url::Url::parse(&format!("http://localhost{}", request_line(request).1)).unwrap()
    }

    fn request_body(request: &str) -> &str {
        request
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .unwrap_or_default()
    }

    fn test_apify_client(base_url: &str) -> ApifyClient {
        ApifyClient::new(ApifyConfig {
            apify_api_base: url::Url::parse(base_url).unwrap(),
            apify_token: "unit-test-token".to_owned(),
            key_value_store_id: "kvs-test".to_owned(),
            dataset_id: "dataset-test".to_owned(),
            actor_run_id: "run-test".to_owned(),
            input_key: "INPUT".to_owned(),
        })
        .unwrap()
    }

    #[test]
    fn stops_pages_on_empty_results_or_scrappa_pagination_metadata() {
        assert!(get_jameda_doctors(&json!({"data":[]})).is_empty());
        assert!(response_has_no_next_page(
            &json!({"meta":{"has_next_page":false}})
        ));
        assert!(response_total_pages_reached(
            &json!({"meta":{"total_pages":2}}),
            2
        ));
        assert!(!response_total_pages_reached(
            &json!({"meta":{"total_pages":"2"}}),
            2
        ));
    }

    #[test]
    fn search_summary_keeps_request_and_pagination_output_shape() {
        let plan = build_jameda_search_plans(&json!({
            "q":"Zahnarzt", "loc":"Berlin", "page":2, "max_pages":2
        }))
        .unwrap()
        .remove(0);
        let summary = search_summary(
            &plan,
            1,
            3,
            Some(&json!({"meta":{"total_results":80,"total_pages":4}})),
        );
        assert_eq!(summary["request"]["q"], "Zahnarzt");
        assert_eq!(summary["request"]["loc"], "Berlin");
        assert_eq!(summary["request"]["per_page"], 28);
        assert_eq!(summary["request"]["start_page"], 2);
        assert_eq!(summary["request"]["max_pages"], 2);
        assert_eq!(summary["pages_fetched"], 1);
        assert_eq!(summary["doctors_extracted"], 3);
        assert_eq!(summary["total_results"], 80);
    }

    #[tokio::test]
    async fn run_saves_only_affordable_ppe_rows_and_preserves_single_search_output() {
        let apify_server = MockServer::start(vec![
            mock_response(
                200,
                json!({"data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{
                "doctor-result":{"eventPriceUsd":0.10},
                "apify-default-dataset-item":{"eventPriceUsd":0.03},
                "apify-actor-start":{"eventPriceUsd":0.05}
            }}},"options":{"maxTotalChargeUsd":0.25},"chargedEventCounts":{"apify-actor-start":1}}}),
            ),
            mock_response(201, json!({})),
            mock_response(201, json!({})),
            mock_response(200, json!({})),
        ]);
        let apify = test_apify_client(&apify_server.base_url);
        let scrappa_server = MockServer::start(vec![mock_response(
            200,
            json!({
                "data":[
                    {"name":"Dr. A","url":"/a/berlin","rating":"1,0"},
                    {"name":"Dr. B","url":"/b/berlin","rating":"1,3"}
                ],
                "meta":{"page":1,"total_results":30,"total_pages":2,"has_next_page":true}
            }),
        )]);
        let scrappa_base = format!("{}/api", scrappa_server.base_url);
        let client = ScrappaClient::new("test-scrappa-key".into(), Some(&scrappa_base)).unwrap();
        let status_message = execute_with_input(
            &apify,
            &client,
            json!({"q":"Zahnarzt","loc":"Berlin","max_pages":2}),
        )
        .await
        .unwrap();
        let expected_status =
            "Charge limit reached after saving 1 of 2 Jameda doctor results on page 1.";
        assert_eq!(status_message.as_deref(), Some(expected_status));

        let scrappa_requests = scrappa_server.finish();
        assert_eq!(scrappa_requests.len(), 1);
        let request_url = request_url(&scrappa_requests[0]);
        assert_eq!(request_url.path(), "/api/jameda/search");
        let query = request_url
            .query_pairs()
            .into_owned()
            .collect::<std::collections::HashMap<_, _>>();
        assert_eq!(query.get("q").map(String::as_str), Some("Zahnarzt"));
        assert_eq!(query.get("loc").map(String::as_str), Some("Berlin"));
        assert_eq!(query.get("per_page").map(String::as_str), Some("28"));
        assert_eq!(query.get("page").map(String::as_str), Some("1"));
        assert!(scrappa_requests[0]
            .to_ascii_lowercase()
            .contains("x-api-key: test-scrappa-key"));

        let apify_requests = apify_server.finish();
        assert_eq!(apify_requests.len(), 4);
        assert_eq!(
            request_line(&apify_requests[0]),
            ("GET", "/v2/actor-runs/run-test")
        );
        assert_eq!(
            request_line(&apify_requests[1]),
            ("POST", "/v2/datasets/dataset-test/items")
        );
        let saved_rows: Value = serde_json::from_str(request_body(&apify_requests[1])).unwrap();
        assert_eq!(saved_rows.as_array().unwrap().len(), 1);
        assert_eq!(saved_rows[0]["name"], "Dr. A");
        assert_eq!(
            request_line(&apify_requests[2]),
            ("POST", "/v2/actor-runs/run-test/charge")
        );
        assert!(apify_requests[2]
            .to_ascii_lowercase()
            .contains("idempotency-key:"));
        assert_eq!(
            serde_json::from_str::<Value>(request_body(&apify_requests[2])).unwrap(),
            json!({"eventName":"doctor-result","count":1})
        );
        assert_eq!(
            request_line(&apify_requests[3]),
            ("PUT", "/v2/key-value-stores/kvs-test/records/OUTPUT")
        );
        let output: Value = serde_json::from_str(request_body(&apify_requests[3])).unwrap();
        assert_eq!(output["pages_fetched"], 1);
        assert_eq!(output["doctors_extracted"], 1);
        assert_eq!(output["responses_saved"], 1);
        assert_eq!(output["status_message"], expected_status);
        assert_eq!(output["responses"][0]["data"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn failed_dataset_write_does_not_submit_custom_charge() {
        let apify_server = MockServer::start(vec![
            mock_response(
                200,
                json!({
                    "data": {
                        "pricingInfo": {
                            "pricingModel": "PAY_PER_EVENT",
                            "pricingPerEvent": {"actorChargeEvents": {
                                "doctor-result": {"eventPriceUsd": 0.10},
                                "apify-default-dataset-item": {"eventPriceUsd": 0.03}
                            }}
                        },
                        "options": {"maxTotalChargeUsd": 1.0},
                        "chargedEventCounts": {}
                    }
                }),
            ),
            mock_response(500, json!({"error":"dataset unavailable"})),
        ]);
        let apify = test_apify_client(&apify_server.base_url);
        let scrappa_server = MockServer::start(vec![mock_response(
            200,
            json!({
                "data":[{"name":"Dr. A","url":"/a"}],
                "meta":{"page":1,"total_results":1,"total_pages":1,"has_next_page":false}
            }),
        )]);
        let scrappa_base = format!("{}/api", scrappa_server.base_url);
        let client = ScrappaClient::new("test-scrappa-key".into(), Some(&scrappa_base)).unwrap();

        let error = execute_with_input(&apify, &client, json!({"q":"Zahnarzt"}))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("Apify dataset write failed"));

        let apify_requests = apify_server.finish();
        assert_eq!(apify_requests.len(), 2);
        assert_eq!(
            request_line(&apify_requests[0]),
            ("GET", "/v2/actor-runs/run-test")
        );
        assert_eq!(
            request_line(&apify_requests[1]),
            ("POST", "/v2/datasets/dataset-test/items")
        );
        assert!(apify_requests
            .iter()
            .all(|request| !request.contains("/charge")));
        assert_eq!(scrappa_server.finish().len(), 1);
    }

    #[tokio::test]
    async fn non_ppe_run_writes_all_dataset_rows_without_custom_charges() {
        let apify_server = MockServer::start(vec![
            mock_response(
                200,
                json!({"data":{"pricingInfo":{"pricingModel":"PRICE_PER_DATASET_ITEM"}}}),
            ),
            mock_response(201, json!({})),
            mock_response(200, json!({})),
        ]);
        let apify = test_apify_client(&apify_server.base_url);
        let scrappa_server = MockServer::start(vec![mock_response(
            200,
            json!({
                "data":[
                    {"name":"Dr. A","url":"/a"},
                    {"name":"Dr. B","url":"/b"}
                ],
                "meta":{"page":1,"total_results":2,"total_pages":1,"has_next_page":false}
            }),
        )]);
        let scrappa_base = format!("{}/api", scrappa_server.base_url);
        let client = ScrappaClient::new("test-scrappa-key".into(), Some(&scrappa_base)).unwrap();
        assert_eq!(
            execute_with_input(&apify, &client, json!({"q":"Zahnarzt"}))
                .await
                .unwrap(),
            None
        );

        let apify_requests = apify_server.finish();
        assert_eq!(apify_requests.len(), 3);
        assert_eq!(
            request_line(&apify_requests[0]),
            ("GET", "/v2/actor-runs/run-test")
        );
        assert_eq!(
            request_line(&apify_requests[1]),
            ("POST", "/v2/datasets/dataset-test/items")
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_body(&apify_requests[1]))
                .unwrap()
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            request_line(&apify_requests[2]),
            ("PUT", "/v2/key-value-stores/kvs-test/records/OUTPUT")
        );
        assert!(apify_requests
            .iter()
            .all(|request| !request.contains("/charge")));
        let scrappa_requests = scrappa_server.finish();
        assert_eq!(scrappa_requests.len(), 1);
    }
}
