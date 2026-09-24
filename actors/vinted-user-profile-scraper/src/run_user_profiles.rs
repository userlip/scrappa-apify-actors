use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::{anyhow, Result};
use serde_json::json;
use tokio::{sync::Semaphore, task::JoinSet};

use crate::{
    apify::{ApifyClient, ChargeBudget},
    request_params::VintedUserProfileRequest,
    response_utils::{build_vinted_user_profile_dataset_item, get_vinted_user_profile},
    runtime_budget::{
        PROFILE_REQUEST_CONCURRENCY, PROFILE_WORKFLOW_CONCURRENCY, SCRAPPA_MAX_ATTEMPTS,
    },
    scrappa::{ScrappaClient, ScrappaError},
};

#[derive(Debug, PartialEq, Eq)]
pub struct VintedUserProfileRunSummary {
    pub requested: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub status_message: Option<String>,
}

enum ProfileResult {
    Saved { request_index: usize },
    Failed,
    ActorFailure(String),
    SkippedAfterActorFailure,
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

        let remaining_requests = requests.len() - offset;
        // Reserve one affordable result event for each workflow in this wave.
        let batch_size = remaining_requests.min(budget.capacity().unwrap_or(remaining_requests));
        let batch = &requests[offset..offset + batch_size];
        offset += batch_size;
        let stop_workers = Arc::new(AtomicBool::new(false));
        let workflow_slots = Arc::new(Semaphore::new(PROFILE_WORKFLOW_CONCURRENCY));
        let scrappa_slots = Arc::new(Semaphore::new(PROFILE_REQUEST_CONCURRENCY));
        let charges_profile_results = budget.charges_profile_results();
        let mut tasks = JoinSet::new();
        for (batch_index, request) in batch.iter().cloned().enumerate() {
            let actor = actor.clone();
            let client = client.clone();
            let stop_workers = stop_workers.clone();
            let workflow_slots = workflow_slots.clone();
            let scrappa_slots = scrappa_slots.clone();
            tasks.spawn(async move {
                let result = process_vinted_user_profile(
                    actor,
                    client,
                    request,
                    charges_profile_results,
                    workflow_slots,
                    scrappa_slots,
                    stop_workers,
                )
                .await;
                (batch_index, result)
            });
        }

        let mut results = (0..batch.len())
            .map(|_| None)
            .collect::<Vec<Option<ProfileResult>>>();
        let mut actor_level_failure = None;
        while let Some(result) = tasks.join_next().await {
            let (batch_index, result) =
                result.map_err(|error| anyhow!("Vinted profile worker failed: {error}"))?;
            if let ProfileResult::ActorFailure(error) = &result {
                actor_level_failure.get_or_insert_with(|| error.clone());
            }
            results[batch_index] = Some(result);
        }

        if let Some(error) = actor_level_failure {
            return Err(anyhow!("{error}"));
        }

        for result in results {
            match result.unwrap_or(ProfileResult::SkippedAfterActorFailure) {
                ProfileResult::Saved { request_index } => {
                    succeeded += 1;
                    budget.event_charge_succeeded();
                    println!("Saved Vinted user profile result {}", request_index + 1);

                    if budget.capacity() == Some(0) {
                        let message = charge_limit_after_result(request_index);
                        println!(
                            "{message} {}",
                            json!({
                                "event": "user-profile-result",
                                "charged_count": 1,
                                "requested_count": 1,
                                "request_index": request_index,
                            })
                        );
                        status_message = Some(message);
                    }
                }
                ProfileResult::Failed => failed += 1,
                ProfileResult::ActorFailure(_) => unreachable!("actor failures return above"),
                ProfileResult::SkippedAfterActorFailure => {}
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

async fn process_vinted_user_profile(
    actor: ApifyClient,
    client: ScrappaClient,
    request: VintedUserProfileRequest,
    charges_profile_results: bool,
    workflow_slots: Arc<Semaphore>,
    scrappa_slots: Arc<Semaphore>,
    stop_workers: Arc<AtomicBool>,
) -> ProfileResult {
    let _workflow_slot = match workflow_slots.acquire_owned().await {
        Ok(permit) => permit,
        Err(error) => {
            stop_workers.store(true, Ordering::SeqCst);
            return ProfileResult::ActorFailure(format!(
                "Could not acquire a Vinted profile worker slot: {error}"
            ));
        }
    };
    if stop_workers.load(Ordering::SeqCst) {
        return ProfileResult::SkippedAfterActorFailure;
    }

    let scrappa_slot = match scrappa_slots.acquire_owned().await {
        Ok(permit) => permit,
        Err(error) => {
            stop_workers.store(true, Ordering::SeqCst);
            return ProfileResult::ActorFailure(format!(
                "Could not acquire a Scrappa request slot: {error}"
            ));
        }
    };
    if stop_workers.load(Ordering::SeqCst) {
        return ProfileResult::SkippedAfterActorFailure;
    }

    println!(
        "Fetching Vinted user profile {} in {}",
        request.user_id, request.country
    );
    let response = client.get(&request, SCRAPPA_MAX_ATTEMPTS).await;
    drop(scrappa_slot);

    let response = match response {
        Ok(response) => response,
        Err(error) if error.is_auth_failure() => {
            stop_workers.store(true, Ordering::SeqCst);
            return ProfileResult::ActorFailure(error.to_string());
        }
        Err(error) => {
            log_profile_failure(&request, &error);
            return ProfileResult::Failed;
        }
    };
    if stop_workers.load(Ordering::SeqCst) {
        eprintln!(
            "Skipping Vinted user profile request {} after an actor-level failure.",
            request.index + 1
        );
        return ProfileResult::SkippedAfterActorFailure;
    }

    let profile = match get_vinted_user_profile(&response) {
        Ok(profile) => profile,
        Err(error) => {
            eprintln!(
                "Vinted user profile request {} failed: {error}",
                request.index + 1
            );
            return ProfileResult::Failed;
        }
    };
    let item = build_vinted_user_profile_dataset_item(profile, &request, &response);

    if let Err(error) = actor.push_dataset_item(&item).await {
        eprintln!(
            "Vinted user profile request {} failed: {error:#}",
            request.index + 1
        );
        return ProfileResult::Failed;
    }

    if charges_profile_results {
        if let Err(error) = actor.charge_user_profile_result(&request).await {
            stop_workers.store(true, Ordering::SeqCst);
            return ProfileResult::ActorFailure(format!(
                "Vinted user profile request {} was saved but charging its result failed: {error:#}",
                request.index + 1
            ));
        }
    }

    ProfileResult::Saved {
        request_index: request.index,
    }
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
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
        thread::{self, JoinHandle},
        time::Duration,
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

    #[tokio::test]
    async fn pipelines_profile_persistence_within_reserved_capacity() {
        let profile_response = r#"{"success":true,"data":{"user":{"id":1,"login":"seller","profile_url":"https://www.vinted.de/member/1"}}}"#;
        let (scrappa_base_url, scrappa_server) = mock_server(vec![
            (200, profile_response.into()),
            (200, profile_response.into()),
        ]);
        let (apify_base_url, apify_server) = mock_concurrent_server(4, Duration::from_millis(40));
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
        let requests = (1..=3)
            .map(|index| VintedUserProfileRequest {
                user_id: index.to_string(),
                country: "DE".into(),
                index: index - 1,
            })
            .collect::<Vec<_>>();
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

        let summary = run_vinted_user_profiles(&actor, &client, &requests, budget)
            .await
            .unwrap();
        let scrappa_requests = scrappa_server.join().unwrap();
        let (apify_requests, max_concurrent_requests) = apify_server.join().unwrap();

        assert_eq!(summary.requested, 3);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.status_message, Some(charge_limit_after_result(1)));
        assert_eq!(scrappa_requests.len(), 2);
        assert_eq!(max_concurrent_requests, 2);
        assert_eq!(
            apify_requests
                .iter()
                .filter(|request| request.starts_with("POST /v2/datasets/test-dataset/items "))
                .count(),
            2
        );
        assert_eq!(
            apify_requests
                .iter()
                .filter(|request| request.starts_with("POST /v2/actor-runs/test-run/charge "))
                .count(),
            2
        );
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

    fn mock_concurrent_server(
        expected_requests: usize,
        delay: Duration,
    ) -> (String, JoinHandle<(Vec<String>, usize)>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let base_url = format!("http://{address}");
        let thread = thread::spawn(move || {
            let requests = Arc::new(Mutex::new(Vec::with_capacity(expected_requests)));
            let active_requests = Arc::new(AtomicUsize::new(0));
            let max_concurrent_requests = Arc::new(AtomicUsize::new(0));
            let mut handlers = Vec::with_capacity(expected_requests);

            for _ in 0..expected_requests {
                let (mut stream, _) = listener.accept().unwrap();
                let requests = Arc::clone(&requests);
                let active_requests = Arc::clone(&active_requests);
                let max_concurrent_requests = Arc::clone(&max_concurrent_requests);
                handlers.push(thread::spawn(move || {
                    let active = active_requests.fetch_add(1, Ordering::SeqCst) + 1;
                    max_concurrent_requests.fetch_max(active, Ordering::SeqCst);
                    let request = read_request(&mut stream);
                    requests.lock().unwrap().push(request.clone());
                    thread::sleep(delay);

                    let status = if request.starts_with("POST /v2/datasets/") {
                        201
                    } else {
                        200
                    };
                    let reason = if status == 201 { "Created" } else { "OK" };
                    let body = "{}";
                    let response = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    stream.write_all(response.as_bytes()).unwrap();
                    active_requests.fetch_sub(1, Ordering::SeqCst);
                }));
            }

            for handler in handlers {
                handler.join().unwrap();
            }

            let requests = Arc::try_unwrap(requests).unwrap().into_inner().unwrap();
            let max_concurrent_requests = max_concurrent_requests.load(Ordering::SeqCst);
            (requests, max_concurrent_requests)
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
