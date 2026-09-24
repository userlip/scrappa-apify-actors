mod apify;
mod request_params;
mod response;
mod scrappa;

use anyhow::{anyhow, Result};
use apify::{ActorConfig, ApifyClient};
use request_params::{build_google_patents_search_params, describe_google_patents_search_request};
use reqwest::Client;
use response::{
    build_summary, enrich_patent_result, extract_patent_results, extract_patent_search_data,
    limit_patent_search_response,
};
use serde_json::{json, Value};

async fn run_actor(http: &Client, config: &ActorConfig) -> Result<()> {
    let apify = ApifyClient::new(http, config);
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let params = build_google_patents_search_params(&input)?;
    println!(
        "Fetching Google Patents for {}",
        describe_google_patents_search_request(&params)
    );

    let response = scrappa::search(
        http,
        &config.scrappa_api_base_url,
        &config.scrappa_api_key,
        &params,
    )
    .await?;
    let data = extract_patent_search_data(&response).clone();
    let patents = extract_patent_results(&response).to_vec();
    let requested_result_count = params
        .get("num")
        .and_then(Value::as_u64)
        .map(|count| count as usize)
        .unwrap_or(patents.len());
    let patents_to_save = patents
        .iter()
        .take(requested_result_count)
        .map(|result| enrich_patent_result(result, &params))
        .collect::<Vec<_>>();

    let save = apify.save_dataset_items(&patents_to_save).await?;
    if !patents_to_save.is_empty() {
        println!(
            "Found {} patent results; saved {}",
            patents.len(),
            save.saved_count
        );
    } else {
        println!("No Google Patents results found for this request");
    }

    let status_message = if save.charge_limit_reached && save.saved_count < patents_to_save.len() {
        let message = format!(
            "Charge limit reached after saving {}/{} Google Patents result(s).",
            save.saved_count,
            patents_to_save.len()
        );
        println!(
            "{message} {}",
            json!({
                "charged_count": save.saved_count,
                "requested_count": patents_to_save.len(),
            })
        );
        Some(message)
    } else {
        None
    };

    let output = if save.saved_count == patents.len() {
        response
    } else {
        limit_patent_search_response(&response, save.saved_count)
    };
    apify.put_output(&output).await?;

    if let Some(message) = status_message {
        if let Err(error) = apify.set_status_message(&message).await {
            eprintln!("Failed to set Apify status message: {error}");
        }
        return Ok(());
    }

    let saved_patents = &patents_to_save[..save.saved_count];
    println!("Google Patents scraping completed successfully");
    println!(
        "Results summary: {}",
        build_summary(&data, patents.len(), saved_patents)
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    let result = async {
        let config = ActorConfig::from_env()?;
        run_actor(&Client::new(), &config).await
    }
    .await;

    if let Err(error) = result {
        let message = scrappa::actor_error_message(&error.to_string());
        eprintln!("Actor failed: {message}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc::{self, Receiver},
        thread::{self, JoinHandle},
    };
    use url::Url;

    const INPUT_SCHEMA: &str = include_str!("../.actor/input_schema.json");

    struct MockRequest {
        method: String,
        target: String,
        headers: HashMap<String, String>,
        body: String,
    }

    struct MockServer {
        base_url: String,
        requests: Receiver<MockRequest>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<(u16, String)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let (sender, requests) = mpsc::channel();
            let thread = thread::spawn(move || {
                for (status, body) in responses {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_mock_request(&mut stream);
                    sender.send(request).unwrap();
                    let reason = match status {
                        200 => "OK",
                        201 => "Created",
                        204 => "No Content",
                        400 => "Bad Request",
                        404 => "Not Found",
                        422 => "Unprocessable Entity",
                        500 => "Internal Server Error",
                        503 => "Service Unavailable",
                        _ => "Error",
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();
                }
            });
            Self {
                base_url: format!("http://{address}"),
                requests,
                thread: Some(thread),
            }
        }

        fn finish(mut self) -> Vec<MockRequest> {
            self.thread.take().unwrap().join().unwrap();
            self.requests.iter().collect()
        }
    }

    fn read_mock_request(stream: &mut TcpStream) -> MockRequest {
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        let (header_end, content_length) = loop {
            let read = stream.read(&mut buffer).unwrap();
            assert_ne!(read, 0, "mock client closed before sending a full request");
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = find_header_end(&bytes) {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length = headers
                    .lines()
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, value)| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if bytes.len() >= header_end + 4 + content_length {
                    break (header_end, content_length);
                }
            }
        };
        let head = String::from_utf8_lossy(&bytes[..header_end]);
        let mut lines = head.lines();
        let mut request_line = lines.next().unwrap().split_whitespace();
        let method = request_line.next().unwrap().to_owned();
        let target = request_line.next().unwrap().to_owned();
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        let body_start = header_end + 4;
        let body_end = body_start + content_length;
        let body = String::from_utf8_lossy(&bytes[body_start..body_end]).into_owned();
        MockRequest {
            method,
            target,
            headers,
            body,
        }
    }

    fn find_header_end(bytes: &[u8]) -> Option<usize> {
        bytes.windows(4).position(|window| window == b"\r\n\r\n")
    }

    fn actor_config(apify_url: &str, scrappa_url: &str) -> ActorConfig {
        ActorConfig::for_test(
            Url::parse(apify_url).unwrap(),
            Url::parse(scrappa_url).unwrap(),
        )
    }

    fn input_body() -> String {
        json!({
            "q":"wireless charging",
            "page":1,
            "num":2,
            "status":"GRANT"
        })
        .to_string()
    }

    fn patent_response() -> String {
        json!({
            "success": true,
            "trace_id": "trace-test",
            "data": {
                "total_results": 45,
                "total_pages": 5,
                "current_page": 1,
                "many_results": true,
                "cached": true,
                "stale": false,
                "patents": [
                    {
                        "patent_id": "patent/US123B1/en",
                        "publication_number": "US123B1",
                        "rank": 1,
                        "title": "Wireless charging",
                        "dates": {"priority":"2020-01-01","filing":"2021-01-01"},
                        "family_status": [{"country":"US","status":"ACTIVE"}],
                        "pdf": "https://example.test/patent.pdf",
                        "upstream_score": 9
                    },
                    {"patent_id":"patent/EP123A1/en","rank":2,"title":"Wireless transfer"},
                    {"patent_id":"patent/WO123A1/en","rank":3,"title":"Wireless power"}
                ]
            }
        })
        .to_string()
    }

    fn priced_run(max_charge: f64) -> String {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                        "apify-actor-start": {"eventPriceUsd": 0.05}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_charge},
                "chargedEventCounts": {"apify-actor-start": 1}
            }
        })
        .to_string()
    }

    fn run_responses(max_charge: f64) -> Vec<(u16, String)> {
        vec![
            (200, input_body()),
            (200, priced_run(max_charge)),
            (201, "{}".to_owned()),
            (201, "{}".to_owned()),
            (200, "{}".to_owned()),
        ]
    }

    #[test]
    fn input_schema_keeps_query_prefill_and_pagination_defaults() {
        let schema: Value = serde_json::from_str(INPUT_SCHEMA).unwrap();
        assert_eq!(
            schema["properties"]["q"]["prefill"],
            "wireless charging vehicle battery"
        );
        assert_eq!(schema["properties"]["page"]["default"], 1);
        assert_eq!(schema["properties"]["num"]["default"], 10);
        assert_eq!(schema["required"], json!(["q"]));
    }

    #[tokio::test]
    async fn retries_transient_upstream_error_and_saves_only_affordable_prefix() {
        let apify = MockServer::start(run_responses(0.15));
        let scrappa = MockServer::start(vec![
            (503, r#"{"message":"Upstream unavailable"}"#.to_owned()),
            (200, patent_response()),
        ]);
        let config = actor_config(&apify.base_url, &format!("{}/api", scrappa.base_url));
        run_actor(&Client::new(), &config).await.unwrap();

        let apify_requests = apify.finish();
        let scrappa_requests = scrappa.finish();
        assert_eq!(apify_requests.len(), 5);
        assert_eq!(scrappa_requests.len(), 2);
        assert_eq!(scrappa_requests[0].method, "GET");
        assert_eq!(scrappa_requests[1].headers["x-api-key"], "scrappa-test-key");
        assert_eq!(
            scrappa_requests[1].headers["user-agent"],
            "thescrappa-google-patents-search-scraper/1.0"
        );
        assert!(scrappa_requests[1]
            .target
            .starts_with("/api/google-patents/search?"));
        let query = scrappa_requests[1].target.split_once('?').unwrap().1;
        let query: HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes())
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        assert_eq!(query["q"], "wireless charging");
        assert_eq!(query["page"], "1");
        assert_eq!(query["num"], "2");
        assert_eq!(query["status"], "GRANT");

        assert_eq!(apify_requests[0].method, "GET");
        assert_eq!(
            apify_requests[0].target,
            "/v2/key-value-stores/store-test/records/INPUT"
        );
        assert_eq!(
            apify_requests[0].headers["authorization"],
            "Bearer apify-test-token"
        );
        assert_eq!(apify_requests[1].method, "GET");
        assert_eq!(apify_requests[1].target, "/v2/actor-runs/run-test");
        assert_eq!(apify_requests[2].method, "POST");
        let dataset_items: Value = serde_json::from_str(&apify_requests[2].body).unwrap();
        assert_eq!(dataset_items.as_array().unwrap().len(), 1);
        assert_eq!(dataset_items[0]["title"], "Wireless charging");
        assert_eq!(dataset_items[0]["upstream_score"], 9);
        assert_eq!(
            dataset_items[0]["patent_page"],
            "https://patents.google.com/patent/US123B1"
        );
        assert_eq!(dataset_items[0]["request_q"], "wireless charging");
        assert_eq!(apify_requests[3].method, "PUT");
        assert_eq!(
            apify_requests[3].target,
            "/v2/key-value-stores/store-test/records/OUTPUT"
        );
        let output: Value = serde_json::from_str(&apify_requests[3].body).unwrap();
        assert_eq!(output["trace_id"], "trace-test");
        assert_eq!(output["data"]["patents"].as_array().unwrap().len(), 1);
        assert_eq!(output["data"]["total_results"], 45);
        assert_eq!(apify_requests[4].method, "PUT");
        assert_eq!(apify_requests[4].target, "/v2/actor-runs/run-test");
        let status: Value = serde_json::from_str(&apify_requests[4].body).unwrap();
        assert_eq!(status["runId"], "run-test");
        assert_eq!(
            status["statusMessage"],
            "Charge limit reached after saving 1/2 Google Patents result(s)."
        );
        assert_eq!(status["isStatusMessageTerminal"], true);
    }

    #[tokio::test]
    async fn saves_empty_budget_limited_output_without_posting_dataset_items() {
        let apify = MockServer::start(vec![
            (200, input_body()),
            (200, priced_run(0.04)),
            (201, "{}".to_owned()),
            (200, "{}".to_owned()),
        ]);
        let scrappa = MockServer::start(vec![(200, patent_response())]);
        let config = actor_config(&apify.base_url, &format!("{}/api", scrappa.base_url));
        run_actor(&Client::new(), &config).await.unwrap();

        let apify_requests = apify.finish();
        assert_eq!(apify_requests.len(), 4);
        assert_eq!(apify_requests[2].method, "PUT");
        assert_eq!(
            apify_requests[2].target,
            "/v2/key-value-stores/store-test/records/OUTPUT"
        );
        let output: Value = serde_json::from_str(&apify_requests[2].body).unwrap();
        assert!(output["data"]["patents"].as_array().unwrap().is_empty());
        assert_eq!(apify_requests[3].method, "PUT");
        let status: Value = serde_json::from_str(&apify_requests[3].body).unwrap();
        assert_eq!(status["runId"], "run-test");
        assert_eq!(
            status["statusMessage"],
            "Charge limit reached after saving 0/2 Google Patents result(s)."
        );
        assert_eq!(status["isStatusMessageTerminal"], true);
    }
}
