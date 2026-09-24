mod apify;
mod charging;
mod request_params;
mod response_utils;
mod scrappa;

use anyhow::{anyhow, Result};
use apify::ApifyClient;
use charging::{
    charge_limit_status, successful_charge_status, ChargeBudget, DEFAULT_DATASET_ITEM_CHARGE_EVENT,
    VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT,
};
use request_params::{build_vinted_item_details_requests, VintedItemDetailsRequest};
use response_utils::{
    build_vinted_item_details_dataset_item, build_vinted_item_details_error_item,
    get_vinted_item_details,
};
use scrappa::{
    is_actor_level_scrappa_failure, timeout_item_message, ScrappaClient, DEFAULT_API_BASE_URL,
};
use serde_json::{json, Value};
use std::{env, process};

struct PushResult {
    saved: bool,
    charged_count: u64,
    event_charge_limit_reached: bool,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {}", actor_error_message(&error));
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let api_key = env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
    let apify = ApifyClient::from_env()?;

    match run_actor(&apify, api_key).await {
        Ok(()) => Ok(()),
        Err(error) => {
            let message = actor_error_message(&error);
            if let Err(status_error) = apify.set_status_message(&message).await {
                eprintln!("Failed to set Actor error status message: {status_error:#}");
            }
            Err(anyhow!(message))
        }
    }
}

async fn run_actor(apify: &ApifyClient, api_key: String) -> Result<()> {
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let requests = build_vinted_item_details_requests(&input).map_err(anyhow::Error::msg)?;
    println!("Running {} Vinted item detail request(s)", requests.len());

    let run = apify.get_run().await?;
    let mut budget = ChargeBudget::from_run(&run)?;
    let scrappa_base_url = env_or_default("SCRAPPA_API_BASE_URL", DEFAULT_API_BASE_URL);
    let scrappa = ScrappaClient::new(api_key, &scrappa_base_url)?;
    let mut saved_results = 0;
    let mut failed_requests = 0;

    for request in &requests {
        if !budget.has_remaining_event_capacity(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT) {
            let message = charge_limit_status(saved_results, request.index);
            println!(
                "{message} {}",
                json!({
                    "event": VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT,
                    "items_requested": requests.len(),
                    "results_saved": saved_results,
                    "next_request_index": request.index,
                })
            );
            apify.set_status_message(&message).await?;
            return Ok(());
        }

        println!("Fetching Vinted item details for {}", request.describe());
        match process_request(apify, &scrappa, &mut budget, request).await {
            Ok(result) => {
                if result.saved {
                    saved_results += 1;
                    println!("Saved Vinted item detail result {}", request.index + 1);
                }
                if result.event_charge_limit_reached {
                    let message = successful_charge_status(result.saved, request.index);
                    println!(
                        "{message} {}",
                        json!({
                            "event": VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT,
                            "charged_count": result.charged_count,
                            "requested_count": requests.len(),
                            "request_index": request.index,
                        })
                    );
                    apify.set_status_message(&message).await?;
                    return Ok(());
                }
            }
            Err(error) if is_actor_level_scrappa_failure(&error) => return Err(error),
            Err(error) => {
                failed_requests += 1;
                let message = timeout_item_message(&error);
                eprintln!(
                    "Vinted item detail request {} failed: {message}",
                    request.index + 1
                );
                push_error_item(apify, &mut budget, request, &message).await?;
            }
        }
    }

    let output = build_output(&requests, saved_results, failed_requests);
    apify.set_output(&output).await?;
    println!("Vinted item details completed");
    println!("Results summary: {output}");
    Ok(())
}

async fn process_request(
    apify: &ApifyClient,
    scrappa: &ScrappaClient,
    budget: &mut ChargeBudget,
    request: &VintedItemDetailsRequest,
) -> Result<PushResult> {
    let response = scrappa
        .get_item_details(&request.item_id, &request.country)
        .await?;
    let details = get_vinted_item_details(&response).map_err(anyhow::Error::msg)?;
    let item = build_vinted_item_details_dataset_item(&details, request);

    if budget.is_pay_per_event() && !budget.should_push_success_item() {
        return Ok(PushResult {
            saved: false,
            charged_count: 0,
            event_charge_limit_reached: true,
        });
    }
    apify.push_dataset_item(&item).await?;

    if !budget.is_pay_per_event() {
        return Ok(PushResult {
            saved: true,
            charged_count: 1,
            event_charge_limit_reached: false,
        });
    }

    let result = record_successful_dataset_charges(apify, budget, request).await?;
    Ok(result)
}

async fn record_successful_dataset_charges(
    apify: &ApifyClient,
    budget: &mut ChargeBudget,
    request: &VintedItemDetailsRequest,
) -> Result<PushResult> {
    let event_charged_count = budget.record_charge(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT, 1);
    let dataset_charged_count = budget.record_charge(DEFAULT_DATASET_ITEM_CHARGE_EVENT, 1);
    if budget.is_event_configured(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT) {
        let idempotency_key = format!(
            "vinted-item-detail:{}:{}",
            apify.actor_run_id(),
            request.index
        );
        apify
            .charge_event(
                VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT,
                event_charged_count,
                &idempotency_key,
            )
            .await?;
    }

    let charged_count = event_charged_count + dataset_charged_count;
    Ok(PushResult {
        saved: charged_count >= 1,
        charged_count,
        event_charge_limit_reached: budget
            .event_charge_limit_reached(VINTED_ITEM_DETAIL_RESULT_CHARGE_EVENT)
            || budget.event_charge_limit_reached(DEFAULT_DATASET_ITEM_CHARGE_EVENT),
    })
}

async fn push_error_item(
    apify: &ApifyClient,
    budget: &mut ChargeBudget,
    request: &VintedItemDetailsRequest,
    message: &str,
) -> Result<()> {
    if !budget.should_push_error_item() {
        return Ok(());
    }
    let item = build_vinted_item_details_error_item(request, message);
    apify.push_dataset_item(&item).await?;
    if budget.is_pay_per_event() {
        // Apify accounts for the synthetic default dataset event when the row is written.
        budget.record_charge(DEFAULT_DATASET_ITEM_CHARGE_EVENT, 1);
    }
    Ok(())
}

fn build_output(
    requests: &[VintedItemDetailsRequest],
    saved_results: usize,
    failed_requests: usize,
) -> Value {
    json!({
        "requested": requests.len(),
        "succeeded": saved_results,
        "failed": failed_requests,
        "country": requests.first().map(|request| request.country.as_str()),
    })
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error
        .downcast_ref::<scrappa::ScrappaTimeoutError>()
        .is_some()
    {
        timeout_item_message(error)
    } else {
        format!("{error:#}")
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

#[cfg(test)]
mod tests {
    use super::build_output;
    use crate::request_params::build_vinted_item_details_requests;
    use serde_json::json;

    #[test]
    fn output_summary_matches_the_actor_kv_contract() {
        let requests = build_vinted_item_details_requests(&json!({
            "item_ids": ["1", "2"],
            "country": "NL"
        }))
        .unwrap();
        assert_eq!(
            build_output(&requests, 1, 1),
            json!({"requested": 2, "succeeded": 1, "failed": 1, "country": "NL"})
        );
    }

    #[test]
    fn output_uses_null_country_for_empty_request_lists() {
        assert_eq!(
            build_output(&[], 0, 0),
            json!({"requested": 0, "succeeded": 0, "failed": 0, "country": null})
        );
    }

    #[test]
    fn preserves_scrappa_timeout_deadline() {
        assert_eq!(crate::scrappa::REQUEST_TIMEOUT_MS, 90_000);
    }
}
