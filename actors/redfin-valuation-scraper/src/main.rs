mod apify;
mod availability_errors;
mod charging;
mod request_params;
mod response_utils;
mod scrappa;

use apify::ApifyRuntime;
use availability_errors::{
    get_transient_redfin_valuation_status, is_transient_redfin_valuation_error,
};
use charging::{push_charged_valuation, ChargingManager};
use request_params::{build_redfin_valuation_requests, describe_redfin_valuation_request};
use response_utils::{
    build_redfin_valuation_dataset_item, build_redfin_valuation_failure_item,
    get_redfin_valuation_data, has_meaningful_valuation_data,
};
use scrappa::{ScrappaClient, ScrappaError, SCRAPPA_REQUEST_TIMEOUT_MS};
use serde_json::{json, Value};

async fn run() -> Result<(), String> {
    let runtime = ApifyRuntime::from_env()?;
    match execute(&runtime).await {
        Ok(()) => Ok(()),
        Err(message) => {
            if let Err(status_error) = runtime.set_status_message(&message).await {
                eprintln!("Could not set Actor failure status: {status_error}");
            }
            Err(message)
        }
    }
}

async fn execute(runtime: &ApifyRuntime) -> Result<(), String> {
    let current_run = runtime.get_current_run().await?;
    let mut charging = ChargingManager::from_environment(runtime.is_at_home(), &current_run)?;

    let api_key = std::env::var("SCRAPPA_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.".to_owned());
    }

    let Some(input) = runtime.get_input().await? else {
        return Err("Input is required".to_owned());
    };
    if input.is_null() {
        return Err("Input is required".to_owned());
    }
    let requests = build_redfin_valuation_requests(&input)?;
    println!("Running {} Redfin valuation request(s)", requests.len());

    if std::env::var("ACTOR_PURGE_ON_START")
        .map(|value| matches!(value.as_str(), "1" | "true"))
        .unwrap_or(false)
        && std::env::var("ACTOR_USE_CHARGING_LOG_DATASET")
            .map(|value| matches!(value.as_str(), "1" | "true"))
            .unwrap_or(false)
    {
        runtime.delete_local_dataset("charging_log").await?;
    }

    let base_url = std::env::var("SCRAPPA_API_BASE_URL").ok();
    let client = ScrappaClient::new(api_key, base_url, SCRAPPA_REQUEST_TIMEOUT_MS)
        .map_err(|error| error.to_string())?;
    let mut failures = Vec::new();
    let mut successful_items = Vec::new();
    let mut saved_items = Vec::new();
    let mut status_message: Option<String> = None;

    for request in &requests {
        status_message = charging.get_charge_limit_status(saved_items.len(), request.index);
        if let Some(message) = &status_message {
            println!(
                "{message} {}",
                json!({
                    "event": "valuation-result",
                    "valuations_requested": requests.len(),
                    "results": saved_items.len(),
                    "next_request_index": request.index,
                })
            );
            break;
        }

        println!(
            "Fetching Redfin valuation for {}",
            describe_redfin_valuation_request(request)
        );
        let listing_id = request.listing_id.as_ref().map(ToString::to_string);
        match client
            .get(
                "/redfin/valuation",
                &request.property_id.to_string(),
                listing_id.as_deref(),
            )
            .await
        {
            Ok(response) => {
                let data = get_redfin_valuation_data(&response)?;
                if !has_meaningful_valuation_data(&data) {
                    let failure = build_redfin_valuation_failure_item(
                        request,
                        Value::String("unavailable".to_owned()),
                        "Scrappa returned no usable Redfin valuation fields.".to_owned(),
                    );
                    failures.push(failure.clone());
                    let push_result =
                        push_charged_valuation(runtime, &mut charging, &failure, request.index)
                            .await?;
                    if push_result.saved {
                        saved_items.push(failure);
                    }
                    if let Some(message) = push_result.status_message {
                        status_message = Some(message);
                        break;
                    }
                    eprintln!(
                        "No usable Redfin valuation fields for property {}",
                        request.property_id
                    );
                    continue;
                }

                let item = build_redfin_valuation_dataset_item(&response, request)?;
                let push_result =
                    push_charged_valuation(runtime, &mut charging, &item, request.index).await?;
                if push_result.saved {
                    successful_items.push(item.clone());
                    saved_items.push(item);
                }
                if let Some(message) = push_result.status_message {
                    status_message = Some(message);
                    break;
                }
            }
            Err(error) if is_recoverable_availability_error(&error) => {
                let failure = build_redfin_valuation_failure_item(
                    request,
                    Value::Number(error.status().unwrap_or_default().into()),
                    error
                        .details()
                        .unwrap_or("Scrappa valuation unavailable")
                        .to_owned(),
                );
                failures.push(failure.clone());
                let push_result =
                    push_charged_valuation(runtime, &mut charging, &failure, request.index).await?;
                if push_result.saved {
                    saved_items.push(failure);
                }
                if let Some(message) = push_result.status_message {
                    status_message = Some(message);
                    break;
                }
                eprintln!(
                    "Redfin valuation unavailable for property {}: {}",
                    request.property_id,
                    error.details().unwrap_or_default()
                );
            }
            Err(error) if is_transient_redfin_valuation_error(&error) => {
                let message = get_transient_redfin_valuation_status(&error);
                eprintln!("{message}");
                if let Err(status_error) = runtime.set_status_message(&message).await {
                    eprintln!("Could not set Actor status: {status_error}");
                }
                return Ok(());
            }
            Err(error) => return Err(error.to_string()),
        }
    }

    let output = if requests.len() == 1 && successful_items.len() == 1 && failures.is_empty() {
        successful_items[0].clone()
    } else {
        json!({
            "requested": requests.len(),
            "saved": successful_items.len(),
            "dataset_items": saved_items.len(),
            "failed": failures.len(),
            "failures": failures.clone(),
            "status_message": status_message.clone(),
        })
    };
    runtime.set_output(&output).await?;

    if let Some(message) = &status_message {
        println!("Redfin valuation scraping completed: {message}");
    } else {
        println!("Redfin valuation scraping completed successfully");
    }
    println!(
        "Results summary: {}",
        json!({
            "requested": requests.len(),
            "saved": successful_items.len(),
            "dataset_items": saved_items.len(),
            "failed": failures.len(),
            "status_message": status_message,
        })
    );
    Ok(())
}

fn is_recoverable_availability_error(error: &ScrappaError) -> bool {
    matches!(error.status(), Some(404 | 422))
}

#[tokio::main]
async fn main() {
    if let Err(message) = run().await {
        eprintln!("Actor failed: {message}");
        std::process::exit(1);
    }
}
