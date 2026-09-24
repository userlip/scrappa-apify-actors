use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::{
    apify::ApifyClient,
    charging::{PpeBudget, REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT},
    request_params::{build_redfin_property_details_requests, describe_request},
    response_utils::{
        build_redfin_property_details_dataset_item, build_redfin_property_error_dataset_item,
        get_redfin_property_details,
    },
    scrappa_client::{
        is_per_property_scrappa_error, ScrappaApiError, ScrappaApiErrorKind, ScrappaClient,
        SCRAPPA_REQUEST_TIMEOUT,
    },
    ActorConfig,
};

pub(crate) async fn run_actor(config: ActorConfig, apify: &ApifyClient, run: &Value) -> Result<()> {
    let mut budget = PpeBudget::from_run(run)?;
    let api_key = config.scrappa_api_key.ok_or_else(|| {
        anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
    })?;

    let input = apify
        .get_input()
        .await?
        .ok_or_else(|| anyhow!("Input is required"))?;
    let requests = build_redfin_property_details_requests(&input)?;
    println!(
        "Fetching {} Redfin property detail request(s)",
        requests.len()
    );

    let scrappa = ScrappaClient::new(config.scrappa_api_base, api_key)?;
    let mut total_results = 0;
    let mut total_errors = 0;
    let mut status_message = None;
    let mut single_output_item = None;

    for request in &requests {
        status_message = budget.charge_limit_status(total_results, request.index);
        if let Some(message) = &status_message {
            println!(
                "{} {}",
                message,
                json!({
                    "event": REDFIN_PROPERTY_DETAILS_RESULT_CHARGE_EVENT,
                    "properties_requested": requests.len(),
                    "results": total_results,
                    "next_property_index": request.index,
                })
            );
            break;
        }

        println!("Fetching Redfin details for {}", describe_request(request));
        match scrappa.get_property(request.params.property_id).await {
            Ok(response) => {
                let property = get_redfin_property_details(&response);
                if let Some(property) = property {
                    let item = build_redfin_property_details_dataset_item(property, request);
                    let push_result = budget.push_property(apify, &item, request.index).await?;
                    if push_result.saved {
                        total_results += 1;
                        if requests.len() == 1 {
                            single_output_item = Some(item);
                        }
                    }
                    if push_result.status_message.is_some() {
                        status_message = push_result.status_message;
                        break;
                    }
                } else {
                    total_errors += 1;
                    let item = build_redfin_property_error_dataset_item(
                        request,
                        "Scrappa returned no property details",
                        None,
                    );
                    budget.push_error_item(apify, &item).await?;
                    if requests.len() == 1 {
                        single_output_item = Some(item);
                    }
                    println!(
                        "No Redfin property details found for property_id {}",
                        request.params.property_id
                    );
                }
            }
            Err(error) if is_per_property_error(&error) => {
                total_errors += 1;
                let scrappa_error = error
                    .downcast_ref::<ScrappaApiError>()
                    .expect("per-property errors are Scrappa HTTP errors");
                let item = build_redfin_property_error_dataset_item(
                    request,
                    &scrappa_error.details,
                    scrappa_error.status,
                );
                budget.push_error_item(apify, &item).await?;
                if requests.len() == 1 {
                    single_output_item = Some(item);
                }
                println!(
                    "Redfin property error for property_id {}: {}",
                    request.params.property_id, scrappa_error.details
                );
            }
            Err(error) => return Err(error),
        }
    }

    let output = if requests.len() == 1 {
        single_output_item.unwrap_or_else(|| {
            json!({
                "properties_requested": requests.len(),
                "results": total_results,
                "errors": total_errors,
                "status_message": status_message,
            })
        })
    } else {
        json!({
            "properties_requested": requests.len(),
            "results": total_results,
            "errors": total_errors,
            "status_message": status_message,
        })
    };
    apify.set_output(&output).await?;

    println!(
        "{}",
        status_message
            .as_ref()
            .map(|message| format!("Redfin property details completed: {message}"))
            .unwrap_or_else(|| "Redfin property details completed successfully".to_owned())
    );
    println!(
        "Results summary: {}",
        json!({
            "properties_requested": requests.len(),
            "results": total_results,
            "errors": total_errors,
            "status_message": status_message,
        })
    );
    Ok(())
}

fn is_per_property_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(is_per_property_scrappa_error)
}

pub(crate) fn actor_error_message(error: &anyhow::Error) -> String {
    if let Some(scrappa_error) = error.downcast_ref::<ScrappaApiError>() {
        if scrappa_error.kind == ScrappaApiErrorKind::Timeout {
            return format!(
                "{}. The Redfin property details request exceeded the {}s Scrappa API timeout. Try fewer batched properties or run the request again.",
                scrappa_error,
                SCRAPPA_REQUEST_TIMEOUT.as_secs()
            );
        }
    }
    format!("{error:#}")
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;
