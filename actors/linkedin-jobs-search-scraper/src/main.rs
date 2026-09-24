mod apify;
mod request_params;
mod scrappa;

use anyhow::{anyhow, Result};
use apify::{ApifyClient, ApifyConfig};
use request_params::{build_search_params, job_search_results, normalize_input, validate_input};
use scrappa::ScrappaClient;
use serde_json::{json, Value};
use std::{env, process};

const SCRAPPA_TIMEOUT_MS: u64 = 60_000;
const SCRAPPA_REQUEST_ATTEMPTS: usize = 3;
const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {}", actor_error_message(&error));
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let scrappa_api_key = required_env(
        "SCRAPPA_API_KEY",
        "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.",
    )?;
    let apify = ApifyClient::new(ApifyConfig {
        api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
        token: required_env("APIFY_TOKEN", "APIFY_TOKEN environment variable is not set")?,
        run_id: required_env(
            "ACTOR_RUN_ID",
            "ACTOR_RUN_ID environment variable is not set",
        )?,
        store_id: required_env(
            "ACTOR_DEFAULT_KEY_VALUE_STORE_ID",
            "ACTOR_DEFAULT_KEY_VALUE_STORE_ID environment variable is not set",
        )?,
        dataset_id: required_env(
            "ACTOR_DEFAULT_DATASET_ID",
            "ACTOR_DEFAULT_DATASET_ID environment variable is not set",
        )?,
        input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
    })?;
    let scrappa = ScrappaClient::new(
        &env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
        scrappa_api_key,
        std::time::Duration::from_millis(SCRAPPA_TIMEOUT_MS),
        SCRAPPA_REQUEST_ATTEMPTS,
    )?;

    let input = normalize_input(apify.get_input().await?)?;
    validate_input(&input)?;
    let params = build_search_params(&input)?;

    println!(
        "Searching LinkedIn Jobs for: \"{}\"",
        input.query.as_deref().unwrap_or_default()
    );
    let response = scrappa.get(&params).await?;
    let jobs = job_search_results(&response);

    if jobs.is_empty() {
        println!("No LinkedIn job results found for the given search criteria");
    } else {
        let dataset_capacity = apify.dataset_item_capacity(jobs.len()).await?;
        let saved = apify.push_dataset_items(&jobs, dataset_capacity).await?;
        println!("Found {} LinkedIn job result(s)", jobs.len());
        if saved < jobs.len() {
            println!(
                "Saved {saved} of {} LinkedIn job result(s) within the run's charge limit",
                jobs.len()
            );
        }
    }

    apify.set_output(&response).await?;
    println!("LinkedIn Jobs search completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string(&build_summary(&response, jobs.len()))?
    );
    Ok(())
}

fn required_env(name: &str, message: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{message}"))
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error
        .downcast_ref::<scrappa::ScrappaTimeoutError>()
        .is_some()
    {
        format!(
            "{error}. The LinkedIn Jobs Search request exceeded the {}s Scrappa API timeout. Try again or refine the query.",
            SCRAPPA_TIMEOUT_MS / 1000
        )
    } else {
        format!("{error:#}")
    }
}

fn build_summary(response: &Value, jobs: usize) -> Value {
    json!({
        "jobs": jobs,
        "total_results": response
            .get("total_results")
            .filter(|value| !value.is_null())
            .or_else(|| response.pointer("/search_information/total_results"))
            .cloned()
            .unwrap_or(Value::Null),
        "current_page": response
            .pointer("/pagination/current_page")
            .cloned()
            .unwrap_or(Value::Null),
        "pages": response
            .pointer("/pagination/pages")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::{actor_error_message, build_summary};
    use crate::scrappa::ScrappaTimeoutError;
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn summary_preserves_total_result_fallback_and_page_count() {
        let summary = build_summary(
            &json!({
                "search_information": {"total_results": 24},
                "pagination": {"current_page": 2, "pages": [{}, {}]}
            }),
            3,
        );
        assert_eq!(
            summary,
            json!({
                "jobs": 3,
                "total_results": 24,
                "current_page": 2,
                "pages": 2
            })
        );
    }

    #[test]
    fn timeout_message_keeps_the_search_specific_guidance() {
        let error = anyhow::Error::new(ScrappaTimeoutError::new(Duration::from_secs(60)));
        assert_eq!(
            actor_error_message(&error),
            "Scrappa API request timed out after 60000ms. The LinkedIn Jobs Search request exceeded the 60s Scrappa API timeout. Try again or refine the query."
        );
    }
}
