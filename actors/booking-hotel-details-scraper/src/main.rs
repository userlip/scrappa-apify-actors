mod apify;
mod billing;
mod input;
mod response;
mod scrappa;

use std::{env, process::ExitCode};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use apify::ApifyClient;
use billing::{
    merge_charge_results, ChargeResult, ChargingManager, BOOKING_HOTEL_RESULT_CHARGE_EVENT,
};
use input::{build_booking_hotel_requests, describe_booking_hotel_request, BookingHotelRequest};
use response::{
    build_booking_hotel_dataset_item, build_booking_hotel_error_item, get_booking_hotel_details,
};
use scrappa::{ScrappaClient, ScrappaError, SCRAPPA_REQUEST_TIMEOUT_MS};

#[derive(Debug)]
struct PushHotelItemResult {
    saved: bool,
    status_message: Option<String>,
    charged_count: u64,
    event_charge_limit_reached: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let message = actor_error_message(&error);
            eprintln!("Actor failed: {message}");
            ApifyClient::set_terminal_status_from_env(&message, "ERROR").await;
            ExitCode::FAILURE
        }
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
    let input = apify
        .get_input()
        .await?
        .filter(is_truthy_json)
        .ok_or_else(|| anyhow!("Input is required"))?;
    let requests = build_booking_hotel_requests(&input)?;
    println!(
        "Running {} Booking.com hotel detail request(s)",
        requests.len()
    );

    let scrappa = ScrappaClient::new(
        api_key,
        env::var("SCRAPPA_API_BASE_URL").unwrap_or_else(|_| "https://scrappa.co/api".into()),
    )
    .map_err(anyhow::Error::msg)?;
    let mut charging = apify.get_charging_manager().await?;
    let mut saved_results = 0;
    let mut failed_requests = 0;

    for request in &requests {
        if let Some(status_message) =
            charging.hotel_charge_limit_status(saved_results, request.index)
        {
            println!(
                "{status_message} {}",
                json!({
                    "event": BOOKING_HOTEL_RESULT_CHARGE_EVENT,
                    "hotels_requested": requests.len(),
                    "results_saved": saved_results,
                    "next_request_index": request.index,
                })
            );
            apify.set_terminal_status(&status_message, "INFO").await;
            return Ok(());
        }

        println!(
            "Fetching Booking.com hotel details for {}",
            describe_booking_hotel_request(request)
        );

        let result = process_successful_request(&apify, &mut charging, &scrappa, request).await;
        match result {
            Ok(push_result) => {
                if push_result.saved {
                    saved_results += 1;
                    println!("Saved hotel detail result {}", request.index + 1);
                }
                if let Some(status_message) = push_result.status_message {
                    println!(
                        "{status_message} {}",
                        json!({
                            "event": BOOKING_HOTEL_RESULT_CHARGE_EVENT,
                            "charged_count": push_result.charged_count,
                            "requested_count": requests.len(),
                            "request_index": request.index,
                            "event_charge_limit_reached": push_result.event_charge_limit_reached,
                        })
                    );
                    apify.set_terminal_status(&status_message, "INFO").await;
                    return Ok(());
                }
            }
            Err(error) => {
                if error
                    .downcast_ref::<ScrappaError>()
                    .is_some_and(ScrappaError::is_actor_level_failure)
                {
                    return Err(error);
                }

                failed_requests += 1;
                let message = actor_error_message(&error);
                eprintln!(
                    "Hotel detail request {} failed: {message}",
                    request.index + 1
                );
                push_error_hotel_item(&apify, &mut charging, request, &message).await?;
            }
        }
    }

    println!("Booking.com hotel details completed");
    println!(
        "Results summary: {}",
        json!({
            "hotels_requested": requests.len(),
            "results_saved": saved_results,
            "failed_requests": failed_requests,
        })
    );
    Ok(())
}

async fn process_successful_request(
    apify: &ApifyClient,
    charging: &mut ChargingManager,
    scrappa: &ScrappaClient,
    request: &BookingHotelRequest,
) -> Result<PushHotelItemResult> {
    let response = scrappa
        .get("/booking/hotel", &request.params)
        .await
        .map_err(anyhow::Error::new)?;
    let details = get_booking_hotel_details(&response);
    let item = build_booking_hotel_dataset_item(details, request);
    push_successful_hotel_item(apify, charging, item, request).await
}

async fn push_successful_hotel_item(
    apify: &ApifyClient,
    charging: &mut ChargingManager,
    item: Value,
    request: &BookingHotelRequest,
) -> Result<PushHotelItemResult> {
    if !charging.is_pay_per_event() {
        apify.push_dataset_item(&item).await?;
        return Ok(PushHotelItemResult {
            saved: true,
            status_message: None,
            charged_count: 1,
            event_charge_limit_reached: false,
        });
    }

    let plan = charging.plan_dataset_push(1, Some(BOOKING_HOTEL_RESULT_CHARGE_EVENT), true);
    if plan.items_to_keep == 0 {
        return Ok(PushHotelItemResult {
            saved: false,
            status_message: Some(format!(
                "Charge limit reached before saving Booking.com hotel detail result {}.",
                request.index + 1
            )),
            charged_count: 0,
            event_charge_limit_reached: true,
        });
    }

    apify.push_dataset_item(&item).await?;
    let mut charge_result = None;
    for (event_name, count) in plan.events_to_charge {
        let idempotency_key = format!("{}-{event_name}-{}", apify.actor_run_id(), request.index);
        let result = charging
            .charge(apify, &event_name, count, &idempotency_key)
            .await?;
        charge_result = Some(match charge_result {
            Some(previous) => merge_charge_results(previous, result),
            None => result,
        });
    }
    let charge_result = charge_result.unwrap_or(ChargeResult {
        charged_count: 0,
        event_charge_limit_reached: false,
    });
    let saved = charge_result.charged_count >= 1;
    let status_message = charge_result.event_charge_limit_reached.then(|| {
        if saved {
            format!(
                "Charge limit reached after saving Booking.com hotel detail result {}.",
                request.index + 1
            )
        } else {
            format!(
                "Charge limit reached before saving Booking.com hotel detail result {}.",
                request.index + 1
            )
        }
    });
    Ok(PushHotelItemResult {
        saved,
        status_message,
        charged_count: charge_result.charged_count,
        event_charge_limit_reached: charge_result.event_charge_limit_reached,
    })
}

async fn push_error_hotel_item(
    apify: &ApifyClient,
    charging: &mut ChargingManager,
    request: &BookingHotelRequest,
    message: &str,
) -> Result<()> {
    let item = build_booking_hotel_error_item(request, message);
    let plan = charging.plan_dataset_push(1, None, true);
    if plan.items_to_keep == 0 {
        return Ok(());
    }
    apify.push_dataset_item(&item).await?;
    for (event_name, count) in plan.events_to_charge {
        let idempotency_key = format!(
            "{}-{event_name}-error-{}",
            apify.actor_run_id(),
            request.index
        );
        charging
            .charge(apify, &event_name, count, &idempotency_key)
            .await?;
    }
    Ok(())
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error
        .downcast_ref::<ScrappaError>()
        .is_some_and(ScrappaError::is_timeout)
    {
        format!(
            "{error}. The Booking.com hotel detail request exceeded the {}s Scrappa API timeout. Try the Booking.com URL form or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT_MS / 1000
        )
    } else {
        format!("{error:#}")
    }
}

fn is_truthy_json(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}
