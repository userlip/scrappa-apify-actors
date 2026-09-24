mod apify;
mod batch;
mod input;
mod output;
mod scrappa;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use anyhow::{anyhow, Context, Result};
use apify::ApifyClient;
use scrappa::ScrappaClient;
use std::{env, time::Duration};
use url::Url;

const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";

pub async fn run_from_env() -> Result<()> {
    let apify = ApifyClient::from_env()?;
    let result = run_actor(&apify).await;

    match result {
        Ok(status_message) => {
            apify.set_terminal_status(&status_message).await?;
            println!("{status_message}");
            Ok(())
        }
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            if let Err(status_error) = apify.set_terminal_status(&message).await {
                eprintln!("Could not set the Actor failure status: {status_error}");
            }
            Err(error)
        }
    }
}

async fn run_actor(apify: &ApifyClient) -> Result<String> {
    let api_key = env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|key| !key.is_empty())
        .ok_or_else(|| anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."))?;
    let scrappa_base_url = base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?;

    let scrappa = ScrappaClient::new(api_key, scrappa_base_url, SCRAPPA_REQUEST_TIMEOUT);
    run_actor_with_clients(apify, &scrappa).await
}

async fn run_actor_with_clients(apify: &ApifyClient, scrappa: &ScrappaClient) -> Result<String> {
    let input = apify.get_input().await?;
    let requests = input::normalize_locations(input.as_ref())?;
    println!(
        "Fetching ImmobilienScout24 price insights for {} unique location(s)",
        requests.len()
    );

    let mut pricing = apify.get_pricing_info().await?;
    let result = batch::run_price_insights_batch(&requests, scrappa, apify, &mut pricing).await?;

    for failure in &result.failures {
        eprintln!(
            "Price insights unavailable for {}: {}",
            failure.location, failure.message
        );
    }

    if !result.charge_limit_reached && result.succeeded == 0 {
        return Err(anyhow!(
            "No price-insights snapshots were resolved for {} requested location(s).",
            requests.len()
        ));
    }

    Ok(if result.charge_limit_reached {
        format!(
            "Charge limit reached after {} successful location snapshot(s).",
            result.succeeded
        )
    } else {
        format!(
            "Saved {} of {} requested location snapshot(s); {} failed.",
            result.succeeded,
            requests.len(),
            result.failures.len()
        )
    })
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let value = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&value).with_context(|| format!("{name} must be a valid absolute URL"))
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn default_scrappa_base_url_and_deadline_match_the_actor_contract() {
        assert_eq!(SCRAPPA_API_BASE_URL, "https://scrappa.co/api");
        assert_eq!(SCRAPPA_REQUEST_TIMEOUT, Duration::from_secs(10));
    }
}
