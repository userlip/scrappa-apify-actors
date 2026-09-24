mod apify;
mod batch_runner;
mod request_params;
mod response_utils;
mod scrappa_client;

use std::{env, process::ExitCode};

use anyhow::{anyhow, Context, Result};
use apify::{ApifyClient, ApifyConfig, ApifyRouteWriter, PpeBudget};
use batch_runner::{run_directions_batch, MAX_BATCH_DURATION};
use request_params::build_directions_requests;
use scrappa_client::{ScrappaClient, ScrappaTimeoutError};

#[tokio::main]
async fn main() -> ExitCode {
    match run_actor().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let raw_message = error.to_string();
            let message = if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
                format!("{raw_message}. Try a smaller batch or run the request again.")
            } else {
                raw_message
            };
            eprintln!("Actor failed: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn run_actor() -> Result<()> {
    let apify_config = ApifyConfig::from_env()?;
    let run_id = apify_config.run_id.clone();
    let apify = ApifyClient::new(apify_config)?;
    let run = apify.get_run().await?;
    let mut budget = PpeBudget::from_run(&run)?;

    let api_key = required_env("SCRAPPA_API_KEY").map_err(|_| {
        anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
    })?;
    let input = apify.get_input().await?;
    let requests = build_directions_requests(input.as_ref())?;
    println!(
        "Running {} Google Maps directions request(s)",
        requests.len()
    );

    let scrappa = ScrappaClient::new(
        api_key,
        env::var("SCRAPPA_API_BASE_URL")
            .ok()
            .filter(|value| !value.is_empty())
            .as_deref(),
    )?;
    let mut writer = ApifyRouteWriter::new(&apify, &mut budget, &run_id);
    let result = run_directions_batch(&requests, &scrappa, &mut writer, MAX_BATCH_DURATION).await;

    println!("Google Maps directions summary: {}", result.to_json());
    if !result.failures.is_empty() {
        let failures = result
            .to_json()
            .get("failures")
            .cloned()
            .unwrap_or_default();
        eprintln!("Google Maps directions request failures: {failures}");
    }

    if result.charge_limit_reached {
        let message = format!(
            "Charge limit reached after saving {} route alternative(s).",
            result.alternatives_saved
        );
        if let Err(error) = apify.set_status_message(&message).await {
            eprintln!("Could not set the terminal run status message: {error:#}");
        }
    }

    Ok(())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
        .with_context(|| format!("{name} configuration is required"))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    #[test]
    fn preserves_actor_defaults_and_prefill_values() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let properties = schema.get("properties").unwrap();
        let origin = properties.get("origin").unwrap();
        let destination = properties.get("destination").unwrap();

        assert_eq!(origin.get("default"), origin.get("prefill"));
        assert_eq!(destination.get("default"), destination.get("prefill"));
        assert_eq!(
            origin.get("default").and_then(Value::as_str),
            Some("Times Square, New York, NY")
        );
        assert_eq!(
            destination.get("default").and_then(Value::as_str),
            Some("Central Park, New York, NY")
        );
    }
}
