mod apify;
mod input;
mod ports;
mod response;
mod scrape;
mod scrappa;

use anyhow::{anyhow, bail, Result};
use apify::{ActorConfig, ApifyActor, ApifyClient, RunPricing};
use input::parse_input;
use scrape::{is_total_failure, scrape_challenges};
use scrappa::ScrappaClient;
use serde_json::Value;
use std::{env, process::ExitCode};

#[tokio::main]
async fn main() -> ExitCode {
    match run_actor().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run_actor() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let api = ApifyClient::new(config)?;
    let run = api.get_run().await?;
    let pricing = RunPricing::from_run_response(&run)?;

    let api_key = env::var("SCRAPPA_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        bail!("SCRAPPA_API_KEY environment variable is not set");
    }
    let input = api
        .get_input()
        .await?
        .filter(is_js_truthy)
        .ok_or_else(|| anyhow!("Actor input is required"))?;
    let requests = parse_input(&input)?;
    let client = ScrappaClient::from_env(api_key)?;
    let mut actor = ApifyActor::new(api, pricing);
    let summaries = scrape_challenges(&client, &mut actor, &requests).await?;
    for summary in &summaries {
        println!("{}", summary.to_json());
    }

    let saved = summaries
        .iter()
        .map(|summary| summary.videos_saved)
        .sum::<usize>();
    let incomplete = summaries
        .iter()
        .filter(|summary| summary.status != "succeeded")
        .count();
    println!(
        "Completed {}/{} challenges: {saved} videos saved, {incomplete} incomplete challenges.",
        summaries.len(),
        requests.len()
    );
    if is_total_failure(&summaries) {
        bail!("Every challenge failed before producing a video. Check the challenge IDs, Scrappa API key, and upstream availability.");
    }
    Ok(())
}

fn is_js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests;
