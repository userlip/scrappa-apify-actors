mod apify;
mod request_params;
mod response_utils;
mod scrappa;

use anyhow::{bail, Context, Result};
use apify::{ActorConfig, ApifyClient};
use request_params::build_search_params;
use response_utils::{dataset_item, limited_response, pagination_number, property_listings};
use scrappa::{ScrappaClient, ScrappaError, REQUEST_TIMEOUT_MS};
use serde_json::{json, Value};
use std::process;

const SCRAPPA_ENDPOINT: &str = "/immowelt/search";
const TIMEOUT_GUIDANCE: &str =
    "The Immowelt request exceeded the 90s Scrappa API timeout. Try a smaller page size or run the request again.";

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let message = actor_error_message(&error);
        eprintln!("Actor failed: {message}");
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify = ApifyClient::new(
        config.apify_api_base_url.clone(),
        config.apify_token.clone(),
    )?;
    let result = run_actor(&config, &apify).await;
    if let Err(error) = &result {
        let message = actor_error_message(error);
        if let Err(status_error) = apify
            .set_status_message(&config.actor_run_id, &message)
            .await
        {
            eprintln!("Could not update run status message: {status_error:#}");
        }
    }
    result
}

async fn run_actor(config: &ActorConfig, apify: &ApifyClient) -> Result<()> {
    if config.scrappa_api_key.is_empty() {
        bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
    }

    let input = apify
        .get_input(&config.default_key_value_store_id, &config.input_key)
        .await?;
    let params = build_search_params(input.as_ref())?;
    println!("Searching Immowelt for {}", params.describe());

    let scrappa = ScrappaClient::new(
        config.scrappa_api_key.clone(),
        config.scrappa_api_base_url.clone(),
    )?;
    let response = scrappa
        .get(SCRAPPA_ENDPOINT, &params.query_pairs())
        .await
        .map_err(anyhow::Error::from)?;

    let raw_listings = property_listings(&response);
    let listings = raw_listings
        .iter()
        .take(params.per_page as usize)
        .map(|listing| dataset_item(listing, &params))
        .collect::<Vec<_>>();

    let saved_count = if listings.is_empty() {
        0
    } else {
        apify
            .push_data(&config.actor_run_id, &config.default_dataset_id, &listings)
            .await?
    };

    if !listings.is_empty() && saved_count < listings.len() {
        let status_message = format!(
            "Charge limit reached after saving {saved_count} of {} Immowelt property result(s); OUTPUT was not written.",
            listings.len()
        );
        println!(
            "{status_message} {}",
            json!({
                "event": "property-result",
                "charged_count": saved_count,
                "requested_count": listings.len()
            })
        );
        apify
            .set_status_message(&config.actor_run_id, &status_message)
            .await
            .context("Could not set the charge-limit status message")?;
        return Ok(());
    }

    if listings.is_empty() {
        println!("No Immowelt property results found for this request");
    } else {
        println!("Found {saved_count} Immowelt property result(s)");
        if raw_listings.len() > listings.len() {
            println!(
                "Scrappa returned {} result(s); saved the requested limit of {}.",
                raw_listings.len(),
                listings.len()
            );
        }
    }

    let output = limited_response(&response, saved_count);
    apify
        .set_output(&config.default_key_value_store_id, &output)
        .await?;

    println!("Immowelt property search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "listings": saved_count,
            "total_results": pagination_number(&response, "total_results").unwrap_or(Value::Null),
            "page": pagination_number(&response, "page").unwrap_or(Value::Null),
            "total_pages": pagination_number(&response, "total_pages").unwrap_or(Value::Null),
            "request_location": params.location,
            "request_type": params.property_type
        })
    );
    Ok(())
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if let Some(error) = error.downcast_ref::<ScrappaError>() {
        if error.is_timeout() {
            return format!(
                "{error}. The Immowelt request exceeded the {}s Scrappa API timeout. Try a smaller page size or run the request again.",
                REQUEST_TIMEOUT_MS / 1_000
            );
        }
        return error.to_string();
    }
    let message = error.to_string();
    if message.contains("Scrappa API request timed out after")
        && !message.contains(TIMEOUT_GUIDANCE)
    {
        return format!("{message}. {TIMEOUT_GUIDANCE}");
    }
    message
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_params::SearchParams;

    #[test]
    fn timeout_failure_keeps_the_actor_specific_guidance() {
        let error = anyhow::Error::new(ScrappaError::Timeout {
            timeout_ms: REQUEST_TIMEOUT_MS,
        });
        assert_eq!(
            actor_error_message(&error),
            format!(
                "Scrappa API request timed out after {REQUEST_TIMEOUT_MS}ms. {TIMEOUT_GUIDANCE}"
            )
        );
    }

    #[test]
    fn request_summary_contains_pagination_and_search_context() {
        let params = SearchParams {
            location: "Berlin".into(),
            property_type: "apartment-rent".into(),
            page: 2,
            per_page: 25,
        };
        assert_eq!(
            params.describe(),
            "apartment-rent properties in Berlin (page 2, per_page 25)"
        );
    }
}
