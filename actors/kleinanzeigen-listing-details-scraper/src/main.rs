mod apify;
mod error_utils;
mod listing_processing;
mod request_params;
mod response_utils;
mod scrappa;

use std::{env, process};

use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use apify::ApifyClient;
use error_utils::error_summary;
use listing_processing::{build_output, process_listings};
use request_params::{
    DetailsPlan, build_details_plan, describe_request, get_discovery_query,
    plan_discovered_listings,
};
use scrappa::{MAX_DETAIL_ATTEMPTS, ScrappaClient};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn required_scrappa_api_key() -> Result<String> {
    env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow!(
                "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."
            )
        })
}

async fn build_plan(input: &Value, client: &ScrappaClient) -> Result<(DetailsPlan, bool)> {
    let query = get_discovery_query(input).map_err(anyhow::Error::msg)?;
    if let Some(query) = query {
        let response = client.search(&query).await?;
        return Ok((
            plan_discovered_listings(&response).map_err(anyhow::Error::msg)?,
            true,
        ));
    }
    Ok((
        build_details_plan(input).map_err(anyhow::Error::msg)?,
        false,
    ))
}

async fn run() -> Result<()> {
    let api_key = required_scrappa_api_key()?;
    let api_token = required_env("APIFY_TOKEN")?;
    let actor_run_id = required_env("ACTOR_RUN_ID")?;
    let key_value_store_id = required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?;
    let dataset_id = required_env("ACTOR_DEFAULT_DATASET_ID")?;
    let input_key = env_or_default("ACTOR_INPUT_KEY", "INPUT");
    let apify_api_base = env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT);
    let scrappa_api_base = env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT);

    let mut apify = ApifyClient::new(
        &apify_api_base,
        api_token,
        actor_run_id,
        key_value_store_id,
        dataset_id,
        input_key,
    )?;
    apify.load_charging_state().await?;
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .unwrap_or_else(|| json!({}));
    let scrappa = ScrappaClient::new(api_key, scrappa_api_base)?;
    let (plan, is_discovery) = build_plan(&input, &scrappa).await?;

    println!("Fetching {}", describe_request(&plan));
    let detail_attempts = if is_discovery { 1 } else { MAX_DETAIL_ATTEMPTS };
    let scrappa_ref = &scrappa;
    let result = process_listings(
        &mut apify,
        &plan.listings,
        |ad_id| {
            let client = scrappa_ref;
            async move { client.listing_detail(&ad_id, detail_attempts).await }
        },
        if is_discovery { Some(1) } else { None },
    )
    .await;

    let output = build_output(plan.listings.len(), &result);
    apify.set_output(&output).await?;

    if result.saved_count == 0 && !result.failures.is_empty() {
        return Err(anyhow!(
            "All {} requested Kleinanzeigen listing detail request(s) failed.",
            result.completed_count
        ));
    }
    if let Some(status_message) = result.status_message {
        apify.set_status_message(&status_message).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {}", error_summary(&error.to_string()));
        process::exit(1);
    }
}
