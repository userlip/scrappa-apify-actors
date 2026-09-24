use std::{cmp::min, collections::HashMap};

use anyhow::{anyhow, Result};
use serde_json::json;
use tokio::task::JoinSet;

use crate::{
    apify::{ApifyClient, ChargeBudget},
    request_params::VintedUserProfileRequest,
    response_utils::{build_vinted_user_profile_dataset_item, get_vinted_user_profile},
    runtime_budget::{PROFILE_REQUEST_CONCURRENCY, SCRAPPA_MAX_ATTEMPTS},
    scrappa::{ScrappaClient, ScrappaError},
};

#[derive(Debug, PartialEq, Eq)]
pub struct VintedUserProfileRunSummary {
    pub requested: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub status_message: Option<String>,
}

pub async fn run_vinted_user_profiles(
    actor: &ApifyClient,
    client: &ScrappaClient,
    requests: &[VintedUserProfileRequest],
    mut budget: ChargeBudget,
) -> Result<VintedUserProfileRunSummary> {
    let mut succeeded = 0;
    let mut failed = 0;
    let mut status_message = None;
    let mut offset = 0;

    while offset < requests.len() && status_message.is_none() {
        let first = &requests[offset];
        if budget.capacity() == Some(0) {
            let message = charge_limit_before_request(succeeded, first.index);
            println!(
                "{message} {}",
                json!({
                    "event": "user-profile-result",
                    "profiles_requested": requests.len(),
                    "profiles_saved": succeeded,
                    "next_request_index": first.index,
                })
            );
            status_message = Some(message);
            break;
        }

        let batch_size = min(
            PROFILE_REQUEST_CONCURRENCY,
            min(
                requests.len() - offset,
                budget.capacity().unwrap_or(PROFILE_REQUEST_CONCURRENCY),
            ),
        );
        let batch = &requests[offset..offset + batch_size];
        offset += batch_size;
        let mut tasks = JoinSet::new();
        for (batch_index, request) in batch.iter().cloned().enumerate() {
            let client = client.clone();
            tasks.spawn(async move {
                println!(
                    "Fetching Vinted user profile {} in {}",
                    request.user_id, request.country
                );
                let response = client.get(&request, SCRAPPA_MAX_ATTEMPTS).await;
                (batch_index, response)
            });
        }

        let mut responses = HashMap::with_capacity(batch.len());
        let mut actor_level_failure = None;
        while let Some(result) = tasks.join_next().await {
            let (batch_index, response) =
                result.map_err(|error| anyhow!("Vinted profile worker failed: {error}"))?;
            if let Err(error) = &response {
                if error.is_auth_failure() {
                    actor_level_failure.get_or_insert_with(|| error.to_string());
                }
            }
            responses.insert(batch_index, response);
        }

        if let Some(error) = actor_level_failure {
            return Err(anyhow!("{error}"));
        }

        for (batch_index, request) in batch.iter().enumerate() {
            let response = responses
                .remove(&batch_index)
                .ok_or_else(|| anyhow!("Vinted profile worker returned no response"))?;

            let response = match response {
                Ok(response) => response,
                Err(error) => {
                    log_profile_failure(request, &error);
                    failed += 1;
                    continue;
                }
            };
            let profile = match get_vinted_user_profile(&response) {
                Ok(profile) => profile,
                Err(error) => {
                    eprintln!(
                        "Vinted user profile request {} failed: {error}",
                        request.index + 1
                    );
                    failed += 1;
                    continue;
                }
            };
            let item = build_vinted_user_profile_dataset_item(profile, request, &response);

            if let Err(error) = actor.push_dataset_item(&item).await {
                eprintln!(
                    "Vinted user profile request {} failed: {error:#}",
                    request.index + 1
                );
                failed += 1;
                continue;
            }

            if budget.charges_profile_results() {
                actor
                    .charge_user_profile_result(request)
                    .await
                    .map_err(|error| {
                        anyhow!(
                            "Vinted user profile request {} was saved but charging its result failed: {error:#}",
                            request.index + 1
                        )
                    })?;
                budget.event_charge_succeeded();
            }

            succeeded += 1;
            println!("Saved Vinted user profile result {}", request.index + 1);

            if budget.capacity() == Some(0) {
                let message = charge_limit_after_result(request.index);
                println!(
                    "{message} {}",
                    json!({
                        "event": "user-profile-result",
                        "charged_count": 1,
                        "requested_count": 1,
                        "request_index": request.index,
                    })
                );
                status_message = Some(message);
                break;
            }
        }
    }

    Ok(VintedUserProfileRunSummary {
        requested: requests.len(),
        succeeded,
        failed,
        status_message,
    })
}

fn log_profile_failure(request: &VintedUserProfileRequest, error: &ScrappaError) {
    let message = match error {
        ScrappaError::Timeout { .. } => {
            format!("{error}. Run the request again or check Scrappa availability.")
        }
        _ => error.to_string(),
    };
    eprintln!(
        "Vinted user profile request {} failed: {message}",
        request.index + 1
    );
}

fn charge_limit_before_request(saved_profiles: usize, request_index: usize) -> String {
    format!(
        "Charge limit reached before fetching Vinted user profile request {}; {saved_profiles} profile result(s) were saved.",
        request_index + 1
    )
}

fn charge_limit_after_result(request_index: usize) -> String {
    format!(
        "Charge limit reached after saving Vinted user profile result {}.",
        request_index + 1
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        apify::{charge_budget_from_run, ActorConfig},
        request_params::VintedUserProfileRequest,
    };
    use serde_json::Value;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        thread::{self, JoinHandle},
    };

    #[test]
    fn formats_charge_limit_messages_with_one_based_request_numbers() {
        assert_eq!(
            charge_limit_before_request(3, 3),
            "Charge limit reached before fetching Vinted user profile request 4; 3 profile result(s) were saved."
        );
        assert_eq!(
            charge_limit_after_result(0),
            "Charge limit reached after saving Vinted user profile result 1."
        );
    }

    #[tokio::test]
    async fn fails_actor_when_charge_fails_after_storing_the_dataset_row() {
        let (scrappa_base_url, scrappa_server) = mock_server(vec![(
            200,
            r#"{"success":true,"data":{"user":{"id":1,"login":"seller","profile_url":"https://www.vinted.de/member/1"}}}"#.into(),
        )]);
        let (apify_base_url, apify_server) = mock_server(vec![
            (201, "{}".into()),
            (400, r#"{"message":"Charge rejected"}"#.into()),
        ]);
        let config = ActorConfig {
            apify_api_base: url::Url::parse(&apify_base_url).unwrap(),
            scrappa_api_base: None,
            apify_token: "test-apify-token".into(),
            actor_run_id: "test-run".into(),
            key_value_store_id: "test-store".into(),
            dataset_id: "test-dataset".into(),
            input_key: "INPUT".into(),
            scrappa_api_key: "test-scrappa-key".into(),
        };
        let actor = ApifyClient::new(&config).unwrap();
        let client = ScrappaClient::new(
            config.scrappa_api_key.clone(),
            Some(&format!("{scrappa_base_url}/api")),
        )
        .unwrap();
        let requests = [VintedUserProfileRequest {
            user_id: "1".into(),
            country: "DE".into(),
            index: 0,
        }];
        let budget = charge_budget_from_run(&serde_json::json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "user-profile-result": {"eventPriceUsd": 0.0005}
                        }
                    }
                },
                "options": {"maxTotalChargeUsd": 0.001},
                "chargedEventCounts": {}
            }
        }))
        .unwrap();

        let error = run_vinted_user_profiles(&actor, &client, &requests, budget)
            .await
            .expect_err("a saved but uncharged PPE row must fail the actor run");
        let apify_requests = apify_server.join().unwrap();
        let scrappa_requests = scrappa_server.join().unwrap();

        assert!(format!("{error:#}").contains("was saved but charging its result failed"));
        assert_eq!(scrappa_requests.len(), 1);
        assert_eq!(apify_requests.len(), 2);
        assert!(apify_requests[0].starts_with("POST /v2/datasets/test-dataset/items "));
        assert!(apify_requests[1].starts_with("POST /v2/actor-runs/test-run/charge "));
        assert!(apify_requests[1]
            .to_ascii_lowercase()
            .contains("idempotency-key: test-run-user-profile-result-0"));

        let stored_item: Value = serde_json::from_str(
            apify_requests[0]
                .split_once("\r\n\r\n")
                .expect("dataset request should include its JSON body")
                .1,
        )
        .unwrap();
        assert_eq!(stored_item["request_user_id"], "1");
    }

    fn mock_server(responses: Vec<(u16, String)>) -> (String, JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let base_url = format!("http://{address}");
        let thread = thread::spawn(move || {
            let mut requests = Vec::with_capacity(responses.len());
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                requests.push(read_request(&mut stream));
                let reason = match status {
                    200 => "OK",
                    201 => "Created",
                    _ => "Bad Request",
                };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
            requests
        });
        (base_url, thread)
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1_024];
        let header_end = loop {
            let count = stream.read(&mut buffer).unwrap();
            assert_ne!(count, 0, "request ended before its headers were complete");
            request.extend_from_slice(&buffer[..count]);
            if let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                break header_end + 4;
            }
        };
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        while request.len() < header_end + content_length {
            let count = stream.read(&mut buffer).unwrap();
            assert_ne!(count, 0, "request ended before its body was complete");
            request.extend_from_slice(&buffer[..count]);
        }
        String::from_utf8(request).unwrap()
    }
}
