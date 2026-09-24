mod apify;
mod input;
mod response;
mod scrappa;

#[cfg(test)]
mod test_support;

use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use apify::ApifyClient;
use reqwest::Client;
use serde_json::{json, Value};
use url::Url;

use crate::{
    input::{build_google_finance_markets_params, describe_google_finance_markets_request},
    response::{build_markets_dataset_items, build_markets_result_counts},
    scrappa::{
        ScrappaClient, ScrappaError, SCRAPPA_API_DEFAULT, SCRAPPA_MAX_ATTEMPTS,
        SCRAPPA_REQUEST_TIMEOUT,
    },
};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const MARKET_ITEM_CHARGE_EVENT: &str = "market-item";

struct Config {
    apify_api_base: Url,
    scrappa_api_base: Url,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        if scrappa_api_key.is_empty() {
            bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
        }
        Ok(Self {
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

async fn run_actor() -> Result<()> {
    let config = Config::from_env()?;
    let apify_http = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to initialize Apify API client")?;
    let actor = ApifyClient::new(
        apify_http,
        config.apify_api_base,
        config.apify_token,
        config.actor_run_id,
        config.key_value_store_id,
        config.dataset_id,
        config.input_key,
    );

    let input = actor.get_input().await?;
    let params = build_google_finance_markets_params(&input)?;
    println!(
        "Fetching Google Finance markets data for {}",
        describe_google_finance_markets_request(&params)
    );

    let scrappa = ScrappaClient::new(
        config.scrappa_api_base,
        config.scrappa_api_key,
        SCRAPPA_REQUEST_TIMEOUT,
    )
    .context("Failed to initialize Scrappa API client")?;
    let response: Value = scrappa
        .get("/google-finance/markets", &params, SCRAPPA_MAX_ATTEMPTS)
        .await?;
    let dataset_items = build_markets_dataset_items(&response, &params);

    if !dataset_items.is_empty() {
        let charge_result = actor
            .push_data(&dataset_items, MARKET_ITEM_CHARGE_EVENT)
            .await?;
        if charge_result.event_charge_limit_reached
            && charge_result.charged_count < dataset_items.len()
        {
            let status_message = "Charge limit reached before saving all Google Finance market items; OUTPUT was not written.";
            println!(
                "{status_message} {}",
                json!({
                    "event": MARKET_ITEM_CHARGE_EVENT,
                    "charged_count": charge_result.charged_count,
                    "requested_count": dataset_items.len()
                })
            );
            if let Err(error) = actor.set_terminal_status_message(status_message).await {
                eprintln!("Unable to set terminal status message: {error:#}");
            }
            return Ok(());
        }
    } else {
        println!("No Google Finance market items found for this request");
    }

    actor.put_output(&response).await?;
    let mut summary = serde_json::Map::new();
    summary.insert(
        "trend".to_owned(),
        params.get("trend").cloned().unwrap_or(Value::Null),
    );
    summary.insert(
        "index_market".to_owned(),
        params.get("index_market").cloned().unwrap_or(Value::Null),
    );
    summary.extend(build_markets_result_counts(&response));
    println!("Google Finance markets scraping completed successfully");
    println!("Results summary: {}", Value::Object(summary));
    Ok(())
}

fn actor_error_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if error
        .downcast_ref::<ScrappaError>()
        .is_some_and(ScrappaError::is_timeout)
    {
        format!(
            "{message}. The Google Finance markets request exceeded the {}s Scrappa API timeout. Narrow the request with trend/index_market, or run it again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run_actor().await {
        eprintln!("Actor failed: {}", actor_error_message(&error));
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn adds_the_existing_timeout_guidance_to_scrappa_deadlines() {
        let error = anyhow::Error::new(ScrappaError::Timeout {
            timeout: Duration::from_secs(60),
        });
        assert_eq!(
            actor_error_message(&error),
            "Scrappa API request timed out after 60000ms. The Google Finance markets request exceeded the 60s Scrappa API timeout. Narrow the request with trend/index_market, or run it again."
        );
    }
}
