mod apify;
mod indeed;
mod scrappa;

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::{env, time::Duration};
use url::Url;

const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let message = if scrappa::is_timeout(&error) {
            scrappa::timeout_message()
        } else {
            format!("{error:#}")
        };
        eprintln!("Actor failed: {message}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let api_key = env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."))?;
    let config = apify::ApifyConfig::from_env()?;
    let scrappa_base_url = base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?;
    let http = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .context("Could not create HTTP client")?;
    let apify = apify::ApifyClient::new(&http, &config);

    let input = indeed::normalize_input(&apify.get_input().await?);
    let query = input.query().filter(|value| is_truthy(value));
    if query.is_none() {
        return Err(anyhow!("Indeed jobs search query is required."));
    }

    println!("Searching Indeed Jobs for: \"{}\"", input.query_for_log());
    let response =
        scrappa::get_jobs_response(&http, &scrappa_base_url, &api_key, &input.params()).await?;
    let jobs = indeed::get_jobs(&response);
    let dataset_jobs = jobs.iter().map(indeed::to_dataset_job).collect::<Vec<_>>();

    if !dataset_jobs.is_empty() {
        let saved = apify.push_dataset_items(&dataset_jobs).await?;
        println!("Found {} Indeed job result(s)", dataset_jobs.len());
        if saved < dataset_jobs.len() {
            println!(
                "Pay-per-event charge limit saved {saved} of {} Indeed job result(s)",
                dataset_jobs.len()
            );
        }
    } else {
        println!("No Indeed job results found for the given search criteria");
    }

    apify.put_output(&response).await?;
    println!("Indeed Jobs search completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string(&indeed::results_summary(&response, &jobs))?
    );
    Ok(())
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_none_or(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_falsey_search_queries() {
        for query in [Value::Null, json!(""), json!(false), json!(0)] {
            assert!(!is_truthy(&query));
        }
        assert!(is_truthy(&json!("engineer")));
    }
}
