mod apify;
mod charging;
mod input;
mod results;
mod run_translations;
mod scrappa;

use std::env;

use apify::{ApifyClient, ApifyTranslationOutput};
use charging::ChargingManager;
use input::{build_translation_requests, describe_translation_requests};
use run_translations::run_translations;
use scrappa::ScrappaClient;
use serde_json::{Value, json};

const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";

#[tokio::main]
async fn main() {
    if run().await.is_err() {
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let apify = ApifyClient::from_env()?;
    let result = run_actor(&apify).await;
    if let Err(message) = &result {
        eprintln!("Actor failed: {message}");
        if let Err(error) = apify.set_terminal_status_message(message).await {
            eprintln!("Could not set Actor failure status message: {error}");
        }
    }
    result
}

async fn run_actor(apify: &ApifyClient) -> Result<(), String> {
    let run_info = apify.get_run_pricing().await?;
    let charging = ChargingManager::from_run(&run_info)?;

    let api_key = env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."
                .to_owned()
        })?;
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| "Input is required".to_owned())?;
    let requests = build_translation_requests(&input)?;
    println!("Running {}", describe_translation_requests(&requests));

    let base_url =
        env::var("SCRAPPA_API_BASE_URL").unwrap_or_else(|_| SCRAPPA_API_DEFAULT.to_owned());
    let runner = ScrappaClient::new(api_key, base_url).map_err(|error| error.to_string())?;
    let mut output = ApifyTranslationOutput::new(apify, charging);
    let summary = run_translations(&requests, &runner, &mut output).await?;

    let output_value = if requests.len() == 1 {
        match &summary.first_item {
            Some(item) => serde_json::to_value(item)
                .map_err(|error| format!("Could not serialize translation output: {error}"))?,
            None => summary_output(&summary),
        }
    } else {
        summary_output(&summary)
    };
    apify.set_output(&output_value).await?;

    if let Some(status_message) = &summary.status_message {
        if let Err(error) = apify.set_terminal_status_message(status_message).await {
            eprintln!("Could not set Actor status message: {error}");
        }
        println!("Google Translate run completed: {status_message}");
    } else {
        println!("Google Translate run completed successfully");
    }
    println!("Results summary: {}", json!(summary_output(&summary)));
    Ok(())
}

fn summary_output(summary: &run_translations::TranslationRunSummary) -> Value {
    json!({
        "requested": summary.requested,
        "succeeded": summary.succeeded,
        "failed": summary.failed,
        "saved": summary.saved,
        "status_message": summary.status_message,
    })
}
