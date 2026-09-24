mod apify;
mod input;
mod output;
mod scrappa;

use anyhow::{anyhow, Result};
use apify::{ActorApi, ActorConfig};
use input::HistoricalPricesRequest;
use output::{build_dataset_items, no_data_output, result_summary};
use scrappa::{ScrappaClient, ScrappaError};
use std::process;

const SCRAPPA_REQUEST_TIMEOUT_SECS: u64 = 60;
const SCRAPPA_MAX_ATTEMPTS: u32 = 3;
const NO_DATA_STATUS_MESSAGE: &str = "No Google Finance historical price points found for this custom date range. Scrappa returned NOT_FOUND; use a preset range for the most stable historical data.";
const CHARGE_LIMIT_STATUS_MESSAGE: &str = "Charge limit reached before saving all Google Finance historical price points; OUTPUT was not written.";

async fn run_actor(config: &ActorConfig, actor_api: &ActorApi) -> Result<()> {
    let api_key = config.scrappa_api_key.as_deref().filter(|key| !key.is_empty()).ok_or_else(|| {
        anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
    })?;

    let input = actor_api.get_input().await?;
    let request = HistoricalPricesRequest::build(&input)?;
    println!(
        "Fetching Google Finance historical prices for {}",
        request.describe()
    );

    let scrappa_client = ScrappaClient::new(
        config.scrappa_api_base_url.clone(),
        api_key.to_owned(),
        std::time::Duration::from_secs(SCRAPPA_REQUEST_TIMEOUT_SECS),
        SCRAPPA_MAX_ATTEMPTS,
    );
    let response = match scrappa_client.get_historical(&request).await {
        Ok(response) => response,
        Err(error) if request.has_custom_date_range() && error.status() == Some(404) => {
            println!("{NO_DATA_STATUS_MESSAGE}");
            actor_api
                .set_value("OUTPUT", &no_data_output(&request, NO_DATA_STATUS_MESSAGE))
                .await?;
            actor_api.set_status_message(NO_DATA_STATUS_MESSAGE).await?;
            return Ok(());
        }
        Err(error) => return Err(anyhow!("{}", format_scrappa_error(error))),
    };

    let dataset_items = build_dataset_items(&response, &request);
    if dataset_items.is_empty() {
        println!("No Google Finance historical price points found for this request");
    }

    let write_result = actor_api.push_dataset_items(&dataset_items).await?;
    if write_result.event_charge_limit_reached && write_result.charged_count < dataset_items.len() {
        println!(
            "{CHARGE_LIMIT_STATUS_MESSAGE} {}",
            serde_json::json!({
                "event": "price-point",
                "charged_count": write_result.charged_count,
                "requested_count": dataset_items.len(),
            })
        );
        actor_api
            .set_status_message(CHARGE_LIMIT_STATUS_MESSAGE)
            .await?;
        return Ok(());
    }

    actor_api.set_value("OUTPUT", &response).await?;
    println!("Google Finance historical prices scraping completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string(&result_summary(&response, &request, dataset_items.len()))?
    );
    Ok(())
}

fn format_scrappa_error(error: ScrappaError) -> String {
    match error {
        ScrappaError::Timeout { message } => format!(
            "{message}. The Google Finance historical prices request exceeded the {SCRAPPA_REQUEST_TIMEOUT_SECS}s Scrappa API timeout. Provide an exchange code, shorten the date range, or run the request again."
        ),
        other => other.to_string(),
    }
}

#[tokio::main]
async fn main() {
    let config = match ActorConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            process::exit(1);
        }
    };
    let actor_api = match ActorApi::new(config.clone()) {
        Ok(actor_api) => actor_api,
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            process::exit(1);
        }
    };

    if let Err(error) = run_actor(&config, &actor_api).await {
        let message = format!("{error:#}");
        eprintln!("Actor failed: {message}");
        if let Err(status_error) = actor_api.set_status_message(&message).await {
            eprintln!("Failed to set terminal status message: {status_error:#}");
        }
        process::exit(1);
    }
}

#[cfg(test)]
mod tests;
