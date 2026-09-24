mod apify;
mod runtime_config;
mod scrappa;
mod stepstone_input;
mod stepstone_response;

#[cfg(test)]
mod test_support;

use anyhow::{Context, Result, anyhow};
use apify::ApifyClient;
use runtime_config::{JOBS_MAX_ATTEMPTS, JOBS_MAX_RETRY_DELAY_MS, JOBS_REQUEST_TIMEOUT_MS};
use scrappa::{ScrappaClient, ScrappaTimeoutError};
use std::{env, process::ExitCode, time::Duration};
use stepstone_input::StepstoneJobsInput;
use stepstone_response::{dataset_job, jobs, summary};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const JOBS_ENDPOINT: &str = "/stepstone/jobs";

struct ActorConfig {
    apify_api_base_url: String,
    scrappa_api_base_url: String,
    apify_token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_key: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify_api_base_url: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            scrappa_api_base_url: env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{error}. The Stepstone Jobs request exceeded the {}s Scrappa API timeout. Try again or refine the query.",
            JOBS_REQUEST_TIMEOUT_MS / 1000
        )
    } else {
        format!("{error:#}")
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify = ApifyClient::new(
        &config.apify_api_base_url,
        config.apify_token,
        config.actor_run_id,
        config.key_value_store_id,
        config.input_key,
        config.dataset_id,
    )?;
    let actor_input = apify.get_input().await?;
    let input = StepstoneJobsInput::normalize(actor_input.as_ref());
    if !input.query_is_truthy() {
        return Err(anyhow!("Stepstone jobs search query is required."));
    }

    println!(
        "Searching Stepstone Jobs for: \"{}\"",
        input.query_for_log()
    );
    let scrappa = ScrappaClient::new(
        config.scrappa_api_key,
        &config.scrappa_api_base_url,
        Duration::from_millis(JOBS_REQUEST_TIMEOUT_MS),
        JOBS_MAX_ATTEMPTS,
        JOBS_MAX_RETRY_DELAY_MS,
    )?;
    let params = input.request_params();
    let response = scrappa.get(JOBS_ENDPOINT, &params).await?;
    let jobs = jobs(&response);
    let dataset_jobs = jobs.iter().map(dataset_job).collect::<Vec<_>>();

    if dataset_jobs.is_empty() {
        println!("No Stepstone job results found for the given search criteria");
    } else {
        let mut dataset_budget = apify.dataset_item_budget().await?;
        let saved = apify
            .push_data(&dataset_jobs, &mut dataset_budget)
            .await
            .context("Could not publish Stepstone results to the default dataset")?;
        println!("Found {saved} Stepstone job result(s)");
        if saved < dataset_jobs.len() {
            println!(
                "Stopped at the pay-per-event budget after saving {saved} of {} result(s)",
                dataset_jobs.len()
            );
        }
    }

    apify
        .set_output(&response)
        .await
        .context("Could not save the raw Scrappa response to OUTPUT")?;
    println!("Stepstone Jobs search completed successfully");

    let result_summary = summary(&response, &input, &jobs);
    println!("Results summary: {result_summary}");
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {}", actor_error_message(&error));
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn adds_the_stepstone_timeout_guidance_only_for_scrappa_timeouts() {
        let timeout = anyhow::Error::new(ScrappaTimeoutError {
            timeout: Duration::from_secs(30),
        });
        assert_eq!(
            actor_error_message(&timeout),
            "Scrappa API request timed out after 30000ms. The Stepstone Jobs request exceeded the 30s Scrappa API timeout. Try again or refine the query."
        );
        assert_eq!(
            actor_error_message(&anyhow!("invalid input")),
            "invalid input"
        );
    }

    #[test]
    fn input_prefill_and_defaults_remain_in_the_actor_schema() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["properties"]["query"]["prefill"],
            json!("software engineer")
        );
        assert_eq!(schema["properties"]["location"]["prefill"], json!("Berlin"));
        assert_eq!(schema["properties"]["country"]["default"], json!("de"));
        assert_eq!(schema["properties"]["page"]["default"], json!(1));
        assert_eq!(schema["properties"]["limit"]["default"], json!(25));
    }
}
