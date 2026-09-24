mod apify;
mod input;
mod response;
mod scrappa;

use anyhow::{bail, Context, Result};
use apify::{base_url_from_env, max_total_charge_from_env, ApifyClient, EventBudget, PricingMode};
use reqwest::Client;
use scrappa::{ScrappaClient, MAX_ATTEMPTS, REQUEST_TIMEOUT};
use serde_json::json;
use std::env;
use url::Url;

const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";

struct ActorConfig {
    api_key: String,
    scrappa_api_base_url: Url,
    apify_api_base_url: Url,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            api_key: scrappa_api_key()?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            apify_api_base_url: base_url_from_env(
                "APIFY_API_PUBLIC_BASE_URL",
                apify::APIFY_API_BASE_URL,
            )?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
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

fn scrappa_api_key() -> Result<String> {
    env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."
            )
        })
}

struct QueryFailure {
    query: String,
    error: String,
}

async fn run_actor() -> Result<()> {
    let config = ActorConfig::from_env()?;
    run_actor_with_config(config).await
}

async fn run_actor_with_config(config: ActorConfig) -> Result<()> {
    let http_client = Client::new();
    let mut apify = ApifyClient::new(
        http_client.clone(),
        config.apify_api_base_url,
        config.apify_token,
        config.key_value_store_id,
        config.dataset_id,
        config.actor_run_id,
        config.input_key,
    );
    let input = apify
        .get_input()
        .await?
        .ok_or_else(|| anyhow::anyhow!("Input is required"))?;
    let request = input::build_request(&input)?;
    let run = apify.get_run().await?;
    let max_total_charge = max_total_charge_from_env()?;
    let mut pricing = EventBudget::from_run(&run, max_total_charge)?;
    let scrappa = ScrappaClient::new(http_client, config.api_key, config.scrappa_api_base_url);

    let mut failures = Vec::new();
    let mut completed_queries = 0;
    let mut suggestion_count = 0;
    let mut saved_suggestion_count = 0;
    let mut charge_limit_reached = false;

    println!(
        "Fetching Google Hotels suggestions for {} unique query or queries",
        request.queries.len()
    );

    for query in &request.queries {
        let chargeable_count = match &pricing {
            PricingMode::PayPerEvent(budget) => budget.affordable_count(usize::MAX),
            PricingMode::NonPayPerEvent => usize::MAX,
        };
        if chargeable_count == 0 {
            charge_limit_reached = true;
            println!(
                "Charge limit reached after saving {saved_suggestion_count} suggestion result(s)"
            );
            break;
        }

        let params = request.params_for_query(query);
        let response = match scrappa
            .get("/google-hotels/autocomplete", &params, MAX_ATTEMPTS)
            .await
        {
            Ok(response) => response,
            Err(error) => {
                let message = if error.is_timeout() {
                    format!(
                        "{error}. The request exceeded the {}s Scrappa API timeout.",
                        REQUEST_TIMEOUT.as_secs()
                    )
                } else {
                    error.to_string()
                };
                record_query_failure(&mut failures, query, &message);
                continue;
            }
        };

        let items = response::build_dataset_items(&response, query, &request.common_params);
        suggestion_count += items.len();
        completed_queries += 1;

        if items.is_empty() {
            println!("No suggestions found for query \"{query}\"");
            continue;
        }

        match &mut pricing {
            PricingMode::PayPerEvent(budget) => {
                let save_count = match apify.push_charged_dataset_items(budget, &items).await {
                    Ok(save_count) => save_count,
                    Err(apify::ChargedDatasetError::DatasetWrite(error)) => {
                        record_query_failure(&mut failures, query, &format!("{error:#}"));
                        continue;
                    }
                    Err(apify::ChargedDatasetError::EventCharge(error)) => {
                        return Err(error.context(format!(
                            "Apify event charge failed after publishing suggestion rows for query \"{query}\""
                        )));
                    }
                };
                saved_suggestion_count += save_count;
                if save_count < items.len() || budget.affordable_count(1) == 0 {
                    charge_limit_reached = true;
                    println!("Charge limit reached after saving {saved_suggestion_count} suggestion result(s)");
                    break;
                }
            }
            PricingMode::NonPayPerEvent => {
                if let Err(error) = apify.push_dataset_items(&items).await {
                    record_query_failure(&mut failures, query, &format!("{error:#}"));
                    continue;
                }
                saved_suggestion_count += items.len();
            }
        }
    }

    if completed_queries == 0 && !failures.is_empty() {
        let details = failures
            .iter()
            .map(|failure| format!("{}: {}", failure.query, failure.error))
            .collect::<Vec<_>>()
            .join("; ");
        bail!("All queries failed: {details}");
    }

    let failed_queries = failures
        .iter()
        .map(|failure| json!({"query": failure.query, "error": failure.error}))
        .collect::<Vec<_>>();
    let summary = json!({
        "requested_queries": request.queries.len(),
        "completed_queries": completed_queries,
        "failed_queries": failed_queries,
        "suggestions_found": suggestion_count,
        "suggestions_saved": saved_suggestion_count,
        "charge_event": apify::RESULT_CHARGE_EVENT,
        "charge_limit_reached": charge_limit_reached,
    });
    apify.put_output(&summary).await?;
    println!("Google Hotels autocomplete completed: {summary}");
    let status_message = if charge_limit_reached {
        format!("Charge limit reached after {saved_suggestion_count} suggestion results.")
    } else {
        format!(
            "Saved {saved_suggestion_count} suggestion results from {completed_queries} queries{}.",
            if failures.is_empty() {
                String::new()
            } else {
                format!("; {} failed", failures.len())
            }
        )
    };
    println!("{status_message}");
    apify.set_terminal_status_message(&status_message).await?;
    Ok(())
}

fn record_query_failure(failures: &mut Vec<QueryFailure>, query: &str, message: &str) {
    failures.push(QueryFailure {
        query: query.to_owned(),
        error: message.to_owned(),
    });
    eprintln!("Query \"{query}\" failed: {message}");
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run_actor().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    struct MockServer {
        base_url: Url,
        requests: Arc<Mutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<(u16, String)>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let server_requests = Arc::clone(&requests);
            let stop = Arc::new(AtomicBool::new(false));
            let server_stop = Arc::clone(&stop);
            let thread = thread::spawn(move || {
                for (status, body) in responses {
                    let deadline = Instant::now() + Duration::from_secs(10);
                    let (mut stream, _) = loop {
                        if server_stop.load(Ordering::Relaxed) {
                            return;
                        }
                        match listener.accept() {
                            Ok(connection) => break connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                if Instant::now() >= deadline {
                                    return;
                                }
                                thread::sleep(Duration::from_millis(5));
                            }
                            Err(_) => return,
                        }
                    };
                    let request = read_request(&mut stream).unwrap_or_default();
                    server_requests.lock().unwrap().push(request);
                    let reason = match status {
                        200 => "OK",
                        201 => "Created",
                        400 => "Bad Request",
                        500 => "Internal Server Error",
                        _ => "Mock Response",
                    };
                    let response = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
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
            self.requests.lock().unwrap().clone()
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
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        loop {
            let count = stream.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n")
            else {
                continue;
            };
            let header_end = header_end + 4;
            let headers = String::from_utf8_lossy(&request[..header_end]).to_ascii_lowercase();
            let content_length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if request.len() >= header_end + content_length {
                break;
            }
        }
        Ok(String::from_utf8_lossy(&request).into_owned())
    }

    fn config(api_base_url: &Url) -> ActorConfig {
        let mut scrappa_api_base_url = api_base_url.clone();
        scrappa_api_base_url.set_path("/api");
        ActorConfig {
            api_key: "test-scrappa-key".to_owned(),
            scrappa_api_base_url,
            apify_api_base_url: api_base_url.clone(),
            key_value_store_id: "store-test".to_owned(),
            dataset_id: "dataset-test".to_owned(),
            actor_run_id: "run-test".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-apify-token".to_owned(),
        }
    }

    #[tokio::test]
    async fn charge_failure_after_dataset_write_fails_the_run() {
        let pricing = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "hotel-suggestion-result": {"eventPriceUsd": 0.00025}
                    }}
                },
                "options": {"maxTotalChargeUsd": 1.0},
                "chargedEventCounts": {}
            }
        })
        .to_string();
        let server = MockServer::start(vec![
            (200, r#"{"q":"Berlin"}"#.to_owned()),
            (200, pricing),
            (
                200,
                r#"{"suggestions":[{"position":1,"value":"Berlin hotel","type":"location"}]}"#
                    .to_owned(),
            ),
            (201, "{}".to_owned()),
            (400, r#"{"error":"charge unavailable"}"#.to_owned()),
            (200, "{}".to_owned()),
            (200, r#"{"data":{}}"#.to_owned()),
        ]);

        let result = run_actor_with_config(config(&server.base_url)).await;
        let requests = server.requests();
        assert_eq!(requests.len(), 5);
        assert!(requests[3].starts_with("POST /v2/datasets/dataset-test/items HTTP/1.1"));
        assert!(requests[4].starts_with("POST /v2/actor-runs/run-test/charge HTTP/1.1"));

        let error = result.expect_err("a failed charge after publishing rows must fail the run");
        let message = format!("{error:#}");
        assert!(
            message.contains("Apify event charge failed with 400"),
            "{message}"
        );
    }
}
