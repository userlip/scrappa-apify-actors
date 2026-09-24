mod apify;
mod charging;
mod config;
mod input;
mod response;
mod scrappa;
mod status;

#[cfg(test)]
mod tests;

use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde_json::json;

use crate::{
    apify::ApifyClient,
    charging::{push_search_items, ChargingManager},
    config::Config,
    input::{build_search_requests, describe_search_request},
    response::{build_dataset_items, count_search_results},
    scrappa::{describe_transient_failure, is_retryable_scrappa_error, ScrappaClient},
    status::build_transient_failure_status_message,
};

async fn run_actor() -> Result<()> {
    let config = Config::from_env()?;
    let http = Client::builder()
        .build()
        .context("Failed to create HTTP client")?;
    let apify = ApifyClient {
        http: &http,
        config: &config,
    };
    let mut total_results = 0;
    let mut total_queries = 0;
    let mut zero_result_queries = 0;
    let mut charge_sequence = 0;

    let execution = async {
        let run = apify.get_run().await?;
        let mut charging = ChargingManager::from_run(&run)?;
        let input = apify.get_input().await?;
        if input.is_null() {
            bail!("Input is required");
        }
        let requests = build_search_requests(&input)?;
        total_queries = requests.len();
        let scrappa = ScrappaClient {
            http: &http,
            config: &config,
        };

        for params in requests {
            eprintln!(
                "Searching Google Finance for {}",
                describe_search_request(&params)
            );
            let response = scrappa.get_search(&params).await?;
            let dataset_items = build_dataset_items(&response, &params);
            let result_count = count_search_results(&response);
            if dataset_items.is_empty() {
                zero_result_queries += 1;
                eprintln!(
                    "No Google Finance search results found for {}",
                    describe_search_request(&params)
                );
                continue;
            }

            charge_sequence += 1;
            let push_result = push_search_items(&apify, &mut charging, &dataset_items, charge_sequence).await?;
            if !push_result.pushed {
                return Ok(push_result.status_message);
            }
            total_results += dataset_items.len();
            eprintln!(
                "Saved {} Google Finance search results for {} (raw count: {})",
                dataset_items.len(),
                describe_search_request(&params),
                result_count
            );
        }

        eprintln!("Google Finance search scraping completed successfully");
        eprintln!(
            "Results summary: {}",
            json!({"queries": total_queries, "total_results": total_results, "zero_result_queries": zero_result_queries})
        );
        Ok(None)
    }
    .await;

    match execution {
        Ok(Some(status_message)) => {
            eprintln!("{status_message}");
            if let Err(error) = apify.update_status(&status_message).await {
                eprintln!("Failed to update Actor status message: {error:#}");
            }
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(error) if is_retryable_scrappa_error(&error) => {
            let status_message = build_transient_failure_status_message(
                &describe_transient_failure(&error),
                total_results,
                total_queries,
            );
            eprintln!("{status_message}");
            if let Err(status_error) = apify.update_status(&status_message).await {
                eprintln!("Failed to update Actor status message: {status_error:#}");
            }
            if total_results > 0 || total_queries > 1 {
                Err(anyhow!(status_message))
            } else {
                Ok(())
            }
        }
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            if let Err(status_error) = apify.update_status(&message).await {
                eprintln!("Failed to update Actor status message: {status_error:#}");
            }
            Err(anyhow!(message))
        }
    }
}
#[tokio::main]
async fn main() {
    if let Err(error) = run_actor().await {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}
