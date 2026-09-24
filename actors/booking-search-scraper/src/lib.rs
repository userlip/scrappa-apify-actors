pub mod apify;
pub mod booking;
pub mod pricing;
pub mod scrappa;

use anyhow::{bail, Result};
use reqwest::Client;
use serde_json::json;

use apify::{
    charge_event, get_input, get_run_info, push_dataset_items, set_status_message, ActorConfig,
};
use booking::{
    build_booking_dataset_item, build_booking_search_requests, describe_booking_search_request,
    get_booking_search_results,
};
use pricing::{charge_plan, is_pay_per_event, ChargeBudget, BOOKING_RESULT_CHARGE_EVENT};
use scrappa::{ScrappaClient, ScrappaError, SCRAPPA_REQUEST_TIMEOUT};

fn timeout_failure_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if error
        .downcast_ref::<ScrappaError>()
        .is_some_and(ScrappaError::is_timeout)
    {
        format!(
            "{message}. The Booking.com request exceeded the {}s Scrappa API timeout. Try a more specific destination, include check-in/check-out dates, or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

pub async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let Some(input) = get_input(client, config).await? else {
        bail!("Input is required");
    };
    if input.is_null() {
        bail!("Input is required");
    }
    let requests = build_booking_search_requests(&input)?;
    println!("Running {} Booking.com search request(s)", requests.len());

    let scrappa = ScrappaClient::new(
        client.clone(),
        config.scrappa_api_base_url.clone(),
        config.scrappa_api_key.clone(),
    );
    let mut total_results = 0;
    let mut charge_budget = ChargeBudget::default();
    for request in &requests {
        println!(
            "Searching Booking.com for {}",
            describe_booking_search_request(&request.params)
        );
        let response = scrappa
            .get(&request.params)
            .await
            .map_err(anyhow::Error::new)?;
        let properties = get_booking_search_results(&response);
        let items = properties
            .iter()
            .map(|property| build_booking_dataset_item(property, &request.params, request.index))
            .collect::<Vec<_>>();

        if !items.is_empty() {
            let run = get_run_info(client, config).await?;
            if is_pay_per_event(&run) {
                let plan = charge_plan(
                    &run,
                    BOOKING_RESULT_CHARGE_EVENT,
                    items.len(),
                    &mut charge_budget,
                )?;
                let charged_items = &items[..plan.charged_count];
                push_dataset_items(client, config, charged_items).await?;
                charge_budget.record_dataset_items_saved(charged_items.len())?;
                charge_event(client, config, request.index, plan.charged_count).await?;
                charge_budget
                    .record_charged_event(BOOKING_RESULT_CHARGE_EVENT, plan.charged_count)?;

                if plan.event_charge_limit_reached {
                    let status_message = format!(
                        "Charge limit reached after saving {} of {} Booking.com results for search {}.",
                        plan.charged_count,
                        items.len(),
                        request.index + 1
                    );
                    println!(
                        "{status_message} {}",
                        json!({
                            "event": BOOKING_RESULT_CHARGE_EVENT,
                            "charged_count": plan.charged_count,
                            "requested_count": items.len(),
                            "search_index": request.index,
                        })
                    );
                    set_status_message(client, config, &status_message).await?;
                    return Ok(());
                }
            } else {
                push_dataset_items(client, config, &items).await?;
            }
        }

        total_results += items.len();
        println!(
            "Search {} returned {} Booking.com result(s)",
            request.index + 1,
            items.len()
        );
    }

    println!("Booking.com search completed successfully");
    println!(
        "Results summary: {}",
        json!({ "searches": requests.len(), "results": total_results })
    );
    Ok(())
}

pub async fn run_from_env() {
    let config = match ActorConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            std::process::exit(1);
        }
    };
    let client = Client::new();
    if let Err(error) = run_actor(&client, &config).await {
        let message = timeout_failure_message(&error);
        eprintln!("Actor failed: {message}");
        if let Err(status_error) = set_status_message(&client, &config, &message).await {
            eprintln!("Could not set Actor run status message: {status_error}");
        }
        std::process::exit(1);
    }
}
