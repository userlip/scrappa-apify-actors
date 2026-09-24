mod apify;
mod request_params;
mod response_utils;
mod scrappa;

use anyhow::{anyhow, Result};
use apify::{ApifyClient, ChargeBudget, PushResult};
use request_params::{build_request_plan, company_details_params, describe_request};
use response_utils::{build_dataset_item, build_output_summary};
use serde_json::{json, Value};
use std::{env, process};

const ENDPOINT: &str = "/trustpilot/company-details";

fn required_api_key() -> Result<String> {
    env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })
}

fn is_pay_per_event(run: &Value) -> bool {
    run.pointer("/data/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        == Some("PAY_PER_EVENT")
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error
        .downcast_ref::<scrappa::ScrappaTimeoutError>()
        .is_some()
    {
        format!(
            "{error}. The Trustpilot company details request exceeded the {}s Scrappa API timeout. Try fewer domains or run the request again.",
            scrappa::REQUEST_TIMEOUT_MS / 1_000
        )
    } else {
        format!("{error:#}")
    }
}

async fn run_actor() -> Result<()> {
    let api_key = required_api_key()?;
    let apify = ApifyClient::from_env()?;
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let plan = build_request_plan(&input).map_err(anyhow::Error::msg)?;

    let run_pricing = apify.get_run_pricing().await?;
    let mut charge_budget = if is_pay_per_event(&run_pricing) {
        Some(ChargeBudget::from_run(&run_pricing)?)
    } else {
        None
    };

    println!(
        "Fetching Trustpilot company details for {}",
        describe_request(&plan)
    );
    let scrappa_base_url = env::var("SCRAPPA_API_BASE_URL").ok();
    let scrappa = scrappa::ScrappaClient::new(api_key, scrappa_base_url)?;
    let mut failures = Vec::new();
    let mut saved_companies = 0;
    let mut status_message = None;
    let mut fatal_error = None;

    for (index, company_domain) in plan.domains.iter().enumerate() {
        let params = company_details_params(&plan, company_domain);
        println!("Fetching Trustpilot company details for {company_domain}");

        let result = async {
            let response = scrappa
                .get(ENDPOINT, &params, scrappa::MAX_ATTEMPTS)
                .await?;
            let item = build_dataset_item(&response, company_domain, &params);
            match charge_budget.as_mut() {
                Some(budget) => apify.push_charged_item(&item, budget, index).await,
                None => {
                    apify.push_dataset_item(&item).await?;
                    Ok(PushResult {
                        saved_count: 1,
                        status_message: None,
                    })
                }
            }
        }
        .await;

        match result {
            Ok(push_result) => {
                saved_companies += push_result.saved_count;
                println!(
                    "Saved {} Trustpilot company detail result(s) for {company_domain}",
                    push_result.saved_count
                );
                if push_result.status_message.is_some() {
                    status_message = push_result.status_message;
                    break;
                }
            }
            Err(error) => {
                let charge_unconfirmed = error
                    .downcast_ref::<apify::CompanyDetailChargeUnconfirmed>()
                    .is_some();
                let message = actor_error_message(&error);
                failures.push(json!({
                    "company_domain": company_domain,
                    "error": message,
                }));
                if charge_unconfirmed {
                    saved_companies += 1;
                    status_message = Some(format!(
                        "The company-detail-result charge for {company_domain} could not be confirmed after saving its dataset row. The run stopped to avoid saving additional uncharged results."
                    ));
                    eprintln!(
                        "Stored Trustpilot company details for {company_domain}, but could not confirm the company-detail-result charge: {message}"
                    );
                    fatal_error = Some(error);
                    break;
                }
                eprintln!(
                    "Failed to fetch Trustpilot company details for {company_domain}: {message}"
                );
            }
        }
    }

    if status_message.is_none() && !failures.is_empty() {
        status_message = Some(format!(
            "{} of {} Trustpilot company detail request(s) failed.",
            failures.len(),
            plan.domains.len()
        ));
    }

    let output = build_output_summary(
        &plan.domains,
        &plan.base_params,
        saved_companies,
        &failures,
        status_message.as_deref(),
    );
    apify.set_output(&output).await?;
    println!(
        "Results summary: {}",
        json!({
            "companies_requested": plan.domains.len(),
            "companies_saved": saved_companies,
            "companies_failed": failures.len(),
        })
    );

    if let Some(error) = fatal_error {
        return Err(error);
    }

    if saved_companies == 0 && !failures.is_empty() {
        let message = status_message
            .clone()
            .unwrap_or_else(|| "No Trustpilot company details were saved.".into());
        return Err(anyhow!(message));
    }

    if let Some(status_message) = status_message {
        apify.set_status_message(&status_message).await?;
    }
    println!("Trustpilot company details extraction completed successfully");
    Ok(())
}

async fn set_failure_status(error: &anyhow::Error) {
    if let Ok(apify) = ApifyClient::from_env() {
        let message = actor_error_message(error);
        let _ = apify.set_status_message(&message).await;
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run_actor().await {
        let message = actor_error_message(&error);
        eprintln!("Actor failed: {message}");
        set_failure_status(&error).await;
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
        thread::{self, JoinHandle},
        time::Duration,
    };

    struct EnvironmentGuard(Vec<(String, Option<std::ffi::OsString>)>);

    impl EnvironmentGuard {
        fn set(values: &[(&str, &str)]) -> Self {
            let previous = values
                .iter()
                .map(|(name, value)| {
                    let previous = env::var_os(name);
                    env::set_var(name, value);
                    ((*name).to_owned(), previous)
                })
                .collect();
            Self(previous)
        }
    }

    impl Drop for EnvironmentGuard {
        fn drop(&mut self) {
            for (name, value) in self.0.drain(..) {
                if let Some(value) = value {
                    env::set_var(name, value);
                } else {
                    env::remove_var(name);
                }
            }
        }
    }

    struct ActorMockServer {
        base_url: String,
        thread: JoinHandle<Vec<String>>,
    }

    impl ActorMockServer {
        fn start(max_requests: usize) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let thread = thread::spawn(move || {
                let mut requests = Vec::new();
                while requests.len() < max_requests {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let request = read_request(&mut stream);
                            respond_to_request(&mut stream, &request);
                            requests.push(request);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(error) => panic!("mock server accept failed: {error}"),
                    }
                }
                requests
            });
            Self {
                base_url: format!("http://{address}"),
                thread,
            }
        }

        fn finish(self) -> Vec<String> {
            self.thread.join().unwrap()
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let bytes_read = stream.read(&mut buffer).unwrap();
            if bytes_read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..bytes_read]);
            let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n")
            else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or_default();
            if request.len() >= header_end + 4 + content_length {
                break;
            }
        }
        String::from_utf8(request).unwrap()
    }

    fn respond_to_request(stream: &mut TcpStream, request: &str) {
        let request_line = request.lines().next().unwrap_or_default();
        let (status, body) = if request_line
            .starts_with("GET /v2/key-value-stores/test-store/records/INPUT ")
        {
            (
                200,
                json!({"company_domains":["example.com", "another.com"]}),
            )
        } else if request_line.starts_with("GET /v2/actor-runs/test-run ") {
            (
                200,
                json!({
                    "data": {
                        "pricingInfo": {
                            "pricingModel": "PAY_PER_EVENT",
                            "pricingPerEvent": {"actorChargeEvents": {
                                "company-detail-result": {"eventPriceUsd": 0.10},
                                "apify-default-dataset-item": {"eventPriceUsd": 0.05}
                            }}
                        },
                        "chargedEventCounts": {},
                        "options": {"maxTotalChargeUsd": 1.0}
                    }
                }),
            )
        } else if request_line.starts_with("GET /trustpilot/company-details?") {
            (
                200,
                json!({"data":{"basic_info":{"name":"Smoke Company","domain":"example.com"}}}),
            )
        } else if request_line.starts_with("POST /v2/datasets/test-dataset/items ") {
            (201, json!({}))
        } else if request_line.starts_with("POST /v2/actor-runs/test-run/charge ") {
            (503, json!({"error":"charge endpoint unavailable"}))
        } else if request_line.starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT ") {
            (201, json!({}))
        } else {
            (404, json!({"error":"unexpected request"}))
        };
        let body = body.to_string();
        let reason = match status {
            200 => "OK",
            201 => "Created",
            503 => "Service Unavailable",
            _ => "Not Found",
        };
        write!(
            stream,
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    }

    fn request_body(request: &str) -> Value {
        serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap()
    }

    #[test]
    fn detects_pay_per_event_pricing_without_changing_legacy_pricing() {
        assert!(is_pay_per_event(&json!({
            "data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"}}
        })));
        assert!(!is_pay_per_event(&json!({
            "data":{"pricingInfo":{"pricingModel":"PRICE_PER_DATASET_ITEM"}}
        })));
    }

    #[test]
    fn formats_timeout_errors_with_the_existing_actor_guidance() {
        let message = actor_error_message(&anyhow::Error::new(scrappa::ScrappaTimeoutError));
        assert!(message.contains("timed out after 90000ms"));
        assert!(message.contains("Try fewer domains or run the request again."));
    }

    #[tokio::test]
    async fn stops_after_a_saved_row_cannot_be_confirmed_as_charged() {
        let server = ActorMockServer::start(8);
        let _environment = EnvironmentGuard::set(&[
            ("SCRAPPA_API_KEY", "integration-test-key"),
            ("SCRAPPA_API_BASE_URL", &server.base_url),
            ("APIFY_API_PUBLIC_BASE_URL", &server.base_url),
            ("APIFY_TOKEN", "integration-test-token"),
            ("ACTOR_DEFAULT_KEY_VALUE_STORE_ID", "test-store"),
            ("ACTOR_DEFAULT_DATASET_ID", "test-dataset"),
            ("ACTOR_RUN_ID", "test-run"),
            ("ACTOR_INPUT_KEY", "INPUT"),
        ]);

        let result = run_actor().await;
        let requests = server.finish();

        assert!(result.is_err());
        let company_requests = requests
            .iter()
            .filter(|request| request.starts_with("GET /trustpilot/company-details?"))
            .count();
        assert_eq!(company_requests, 1, "later domains must not be fetched");

        let dataset_requests = requests
            .iter()
            .filter(|request| request.starts_with("POST /v2/datasets/test-dataset/items "))
            .count();
        assert_eq!(dataset_requests, 1);

        let charge_requests = requests
            .iter()
            .filter(|request| request.starts_with("POST /v2/actor-runs/test-run/charge "))
            .collect::<Vec<_>>();
        assert_eq!(charge_requests.len(), 3, "charge retries should be bounded");
        let idempotency_keys = charge_requests
            .iter()
            .map(|request| {
                request
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("idempotency-key")
                            .then(|| value.trim())
                    })
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert!(idempotency_keys
            .iter()
            .all(|key| *key == "test-run-company-detail-result-0"));

        let output_request = requests
            .iter()
            .find(|request| {
                request.starts_with("PUT /v2/key-value-stores/test-store/records/OUTPUT ")
            })
            .expect("OUTPUT should report the row that was already stored");
        assert_eq!(request_body(output_request)["companies_saved"], 1);
        assert_eq!(request_body(output_request)["companies_failed"], 1);
        assert!(format!("{:#}", result.unwrap_err()).contains("could not be confirmed"));
    }
}
