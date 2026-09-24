mod apify;
mod config;
mod input;
mod pricing;
mod scrappa;
mod transform;

use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};

use crate::{
    apify::{update_terminal_status_message_from_env, ApifyClient},
    config::Config,
    input::{build_intraday_requests, describe_intraday_request},
    pricing::INTRADAY_PRICE_POINT_CHARGE_EVENT,
    scrappa::{is_no_data_error, ScrappaClient, ScrappaTimeoutError, SCRAPPA_REQUEST_TIMEOUT},
    transform::build_intraday_price_point_dataset_items,
};

#[derive(Default)]
struct RunSummary {
    requested: usize,
    succeeded: usize,
    no_data: usize,
    failed: usize,
    graph_points: usize,
}

impl RunSummary {
    fn to_value(&self) -> Value {
        json!({
            "requested": self.requested,
            "succeeded": self.succeeded,
            "no_data": self.no_data,
            "failed": self.failed,
            "graph_points": self.graph_points,
        })
    }
}

async fn run_actor(config: &Config) -> Result<()> {
    let apify_http = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("Failed to create the Apify API client")?;
    let scrappa_http = Client::new();
    let mut apify = ApifyClient::new(&apify_http, config);

    let Some(input) = apify.get_input().await? else {
        bail!("Input is required");
    };
    let requests = build_intraday_requests(&input)?;
    let scrappa = ScrappaClient::new(
        &scrappa_http,
        &config.scrappa_api_key,
        &config.scrappa_api_base_url,
    );
    let mut summary = RunSummary {
        requested: requests.len(),
        ..RunSummary::default()
    };

    println!(
        "Fetching Google Finance intraday data for {} symbol{}",
        requests.len(),
        if requests.len() == 1 { "" } else { "s" }
    );

    for request in &requests {
        let description = describe_intraday_request(&request.params);
        println!("Fetching Google Finance intraday data for {description}");
        let response = match scrappa.get_intraday(&request.params).await {
            Ok(response) => response,
            Err(error) if is_no_data_error(&error) => {
                println!("No Google Finance intraday graph points found for {description}");
                summary.no_data += 1;
                continue;
            }
            Err(error) => return Err(error),
        };
        let dataset_items = build_intraday_price_point_dataset_items(&response, &request.params);
        if dataset_items.is_empty() {
            println!("No Google Finance intraday graph points found for {description}");
            summary.no_data += 1;
            continue;
        }

        let push_result = apify.push_dataset_items(&dataset_items).await?;
        if push_result.charge_limit_reached {
            let status_message = "Charge limit reached before saving all Google Finance intraday price points; remaining symbols were not processed.";
            println!(
                "{status_message} {}",
                json!({
                    "event": INTRADAY_PRICE_POINT_CHARGE_EVENT,
                    "charged_count": push_result.saved_count,
                    "requested_count": dataset_items.len(),
                })
            );
            apify.set_terminal_status_message(status_message).await?;
            return Ok(());
        }
        summary.succeeded += 1;
        summary.graph_points += push_result.saved_count;
    }

    apify.put_output(&summary.to_value()).await?;
    println!("Google Finance intraday scraping completed successfully");
    println!("Results summary: {}", summary.to_value());
    Ok(())
}

#[tokio::main]
async fn main() {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            if let Err(status_error) = update_terminal_status_message_from_env(&message).await {
                eprintln!("Could not set the Apify run status message: {status_error}");
            }
            std::process::exit(1);
        }
    };

    if let Err(error) = run_actor(&config).await {
        let message = if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
            format!(
                "{error}. The Google Finance intraday request exceeded the {}s Scrappa API timeout. Provide exchange codes, reduce the symbol batch, or run the request again.",
                SCRAPPA_REQUEST_TIMEOUT.as_secs()
            )
        } else {
            error.to_string()
        };
        eprintln!("Actor failed: {message}");
        if let Err(status_error) = update_terminal_status_message_from_env(&message).await {
            eprintln!("Could not set the Apify run status message: {status_error}");
        }
        std::process::exit(1);
    }
}
