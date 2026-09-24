mod apify;
mod request_params;
mod response_utils;
mod scrappa_client;

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Result};
use reqwest::Client;
use serde_json::json;

use crate::{
    apify::{ApifyClient, Config},
    request_params::{build_google_flights_request, describe_google_flights_request},
    response_utils::{build_flight_dataset_items, build_unavailable_search_response, get_flights},
    scrappa_client::{
        is_retryable_scrappa_error, ScrappaClient, ScrappaTimeoutError, MAX_ATTEMPTS,
        REQUEST_TIMEOUT,
    },
};

const APIFY_API_TIMEOUT_SECS: u64 = 60;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {}", actor_error_message(&error));
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let http = Client::builder()
        .timeout(std::time::Duration::from_secs(APIFY_API_TIMEOUT_SECS))
        .build()
        .map_err(|error| anyhow!("Failed to create HTTP client: {error}"))?;
    let apify = ApifyClient::new(&http, &config);
    let input = apify.get_input().await?;
    if input.is_null() {
        bail!("Input is required");
    }

    let request = build_google_flights_request(&input, today_utc_days())?;
    let request_description = describe_google_flights_request(&request);
    println!("Searching Google Flights for {request_description}");
    let search_started_at = Instant::now();
    let scrappa = ScrappaClient::new(&http, config.scrappa_api_base(), &config.scrappa_api_key);
    let response = match scrappa.get(request.endpoint, &request.params).await {
        Ok(response) => response,
        Err(error) if is_retryable_scrappa_error(&error) => {
            let unavailable_response = build_unavailable_search_response(
                &request.params,
                request.trip_type,
                &error.to_string(),
                MAX_ATTEMPTS,
                search_started_at.elapsed().as_millis(),
            );
            apify.put_output(&unavailable_response).await?;
            let status_message = format!(
                "Google Flights is temporarily unavailable after {MAX_ATTEMPTS} attempts. No results were saved; retry this run later."
            );
            eprintln!(
                "{status_message} {}",
                serde_json::to_string(&json!({
                    "origin": request.params.get("origin"),
                    "destination": request.params.get("destination"),
                    "departure_date": request.params.get("departure_date"),
                    "retryable": true
                }))?
            );
            set_terminal_status(&apify, &status_message).await;
            return Ok(());
        }
        Err(error) => return Err(error),
    };

    let items = build_flight_dataset_items(&response, &request.params, request.trip_type);
    let flights = get_flights(&response);
    if items.is_empty() {
        println!("No flight results found for the given search criteria");
    } else {
        let pushed = apify.push_dataset_items(&items).await?;
        if pushed.event_charge_limit_reached {
            let status_message = format!(
                "Charge limit reached after saving {}/{} Google Flights result(s).",
                pushed.saved_count,
                items.len()
            );
            println!(
                "{status_message} {}",
                serde_json::to_string(&json!({
                    "event": apify::FLIGHT_RESULT_CHARGE_EVENT,
                    "charged_count": pushed.saved_count,
                    "result_count": items.len()
                }))?
            );
            set_terminal_status(&apify, &status_message).await;
            return Ok(());
        }
        println!(
            "Found {} flight result(s); saved {} dataset item(s)",
            flights.len(),
            pushed.saved_count
        );
    }

    apify.put_output(&response).await?;
    println!("Google Flights search completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string(&json!({
            "trip_type": request.trip_type.as_str(),
            "origin": request.params.get("origin"),
            "destination": request.params.get("destination"),
            "departure_date": request.params.get("departure_date"),
            "return_date": request.params.get("return_date"),
            "flights": flights.len(),
            "has_baggage_info": response.get("baggage_info").is_some()
        }))?
    );
    Ok(())
}

async fn set_terminal_status(apify: &ApifyClient<'_>, status_message: &str) {
    if let Err(error) = apify.set_status_message(status_message).await {
        eprintln!("Could not set terminal run status message: {error}");
    }
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        return format!(
            "{}. The Google Flights request exceeded the {}s Scrappa API timeout. Try a narrower route/date filter or run the request again.",
            error,
            REQUEST_TIMEOUT.as_secs()
        );
    }
    error.to_string()
}

fn today_utc_days() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        / 86_400
}
