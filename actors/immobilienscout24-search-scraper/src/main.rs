mod apify;
mod billing;
mod config;
mod endpoint;
mod input;
mod response;
mod scrappa;

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use std::{process::ExitCode, time::Duration};
use tokio::time::timeout;

use apify::ApifyClient;
use billing::{ChargePricing, PROPERTY_RESULT_CHARGE_EVENT};
use config::{Config, APIFY_REQUEST_TIMEOUT, SCRAPPA_REQUEST_TIMEOUT};
use input::normalize_search_input;
use response::{
    dataset_item, get_listings, get_response_page, get_total_pages, get_total_results,
    limited_response,
};
use scrappa::{search_immobilienscout24, ScrappaClient};

async fn run_actor() -> Result<()> {
    let config = Config::from_env()?;
    let apify_http = Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .context("Could not create Apify HTTP client")?;
    let scrappa_http = Client::builder()
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .build()
        .context("Could not create Scrappa HTTP client")?;
    let apify = ApifyClient::new(apify_http, &config);
    let scrappa = ScrappaClient::new(
        scrappa_http,
        config.scrappa_api_base.clone(),
        config.scrappa_api_key.clone(),
    );

    let run = apify.get_run().await?;
    let pricing = ChargePricing::from_run(&run)?;
    let input = apify.get_input().await?;
    let params = normalize_search_input(input.as_ref())?;
    println!("Searching ImmobilienScout24 for {}", params.describe());

    let response = search_immobilienscout24(&scrappa, &params).await?;
    let raw_listings = get_listings(&response);
    let requested_limit = params.per_page as usize;
    let listings = raw_listings
        .iter()
        .take(requested_limit)
        .map(|listing| dataset_item(listing, &params))
        .collect::<Vec<_>>();

    if !listings.is_empty() {
        if pricing.is_pay_per_event {
            let plan = pricing.plan_dataset_push(listings.len());
            apify
                .push_dataset_items(&listings[..plan.items_to_push])
                .await?;
            if plan.should_charge_custom_event {
                apify
                    .charge_event(PROPERTY_RESULT_CHARGE_EVENT, plan.custom_event_charge_count)
                    .await?;
            }
            if plan.items_to_push < listings.len() {
                let message = format!(
                    "Charge limit reached after saving {} of {} ImmobilienScout24 property result(s); OUTPUT was not written.",
                    plan.items_to_push,
                    listings.len()
                );
                println!("[Status message]: {message}");
                match timeout(
                    Duration::from_secs(1),
                    apify.set_terminal_status_message(&message),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        eprintln!("Could not set final Apify status message: {error}")
                    }
                    Err(_) => eprintln!("Setting status message timed out after 1s"),
                }
                println!(
                    "{message} {}",
                    json!({
                        "event": PROPERTY_RESULT_CHARGE_EVENT,
                        "saved_count": plan.items_to_push,
                        "charged_event_count": plan.charged_count,
                        "requested_count": listings.len(),
                        "limit_reached": plan.limit_reached,
                    })
                );
                return Ok(());
            }
        } else {
            apify.push_dataset_items(&listings).await?;
        }

        println!(
            "Found {} ImmobilienScout24 property result(s)",
            listings.len()
        );
        if raw_listings.len() > listings.len() {
            println!(
                "Scrappa returned {} result(s); saved the requested limit of {}.",
                raw_listings.len(),
                listings.len()
            );
        }
    } else {
        println!("No ImmobilienScout24 property results found for this request");
    }

    apify
        .put_record("OUTPUT", &limited_response(&response, listings.len()))
        .await?;
    println!("ImmobilienScout24 property search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "listings": listings.len(),
            "total_results": get_total_results(&response),
            "page": get_response_page(&response),
            "total_pages": get_total_pages(&response),
            "request_location": params.location,
            "request_type": params.property_type,
        })
    );
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run_actor().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
