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
                if let Err(error) = actor.charge_user_profile_result(request).await {
                    eprintln!(
                        "Vinted user profile request {} failed while charging its saved result: {error:#}",
                        request.index + 1
                    );
                    failed += 1;
                    continue;
                }
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
}
