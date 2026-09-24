mod apify;
mod input;
mod response;
mod scrappa;

use anyhow::{bail, Context, Result};
use apify::{base_url_from_env, max_total_charge_from_env, ApifyClient, EventBudget, PricingMode};
use reqwest::Client;
use scrappa::{ScrappaClient, MAX_ATTEMPTS, REQUEST_TIMEOUT};
use serde_json::json;
use std::env;
use url::Url;

const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";

struct ActorConfig {
    api_key: String,
    scrappa_api_base_url: Url,
    apify_api_base_url: Url,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            api_key: scrappa_api_key()?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            apify_api_base_url: base_url_from_env(
                "APIFY_API_PUBLIC_BASE_URL",
                apify::APIFY_API_BASE_URL,
            )?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.trim().is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn scrappa_api_key() -> Result<String> {
    env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."
            )
        })
}

struct QueryFailure {
    query: String,
    error: String,
}

async fn run_actor() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let http_client = Client::new();
    let mut apify = ApifyClient::new(
        http_client.clone(),
        config.apify_api_base_url,
        config.apify_token,
        config.key_value_store_id,
        config.dataset_id,
        config.actor_run_id,
        config.input_key,
    );
    let input = apify
        .get_input()
        .await?
        .ok_or_else(|| anyhow::anyhow!("Input is required"))?;
    let request = input::build_request(&input)?;
    let run = apify.get_run().await?;
    let max_total_charge = max_total_charge_from_env()?;
    let mut pricing = EventBudget::from_run(&run, max_total_charge)?;
    let scrappa = ScrappaClient::new(http_client, config.api_key, config.scrappa_api_base_url);

    let mut failures = Vec::new();
    let mut completed_queries = 0;
    let mut suggestion_count = 0;
    let mut saved_suggestion_count = 0;
    let mut charge_limit_reached = false;

    println!(
        "Fetching Google Hotels suggestions for {} unique query or queries",
        request.queries.len()
    );

    for query in &request.queries {
        let chargeable_count = match &pricing {
            PricingMode::PayPerEvent(budget) => budget.affordable_count(usize::MAX),
            PricingMode::NonPayPerEvent => usize::MAX,
        };
        if chargeable_count == 0 {
            charge_limit_reached = true;
            println!(
                "Charge limit reached after saving {saved_suggestion_count} suggestion result(s)"
            );
            break;
        }

        let params = request.params_for_query(query);
        let response = match scrappa
            .get("/google-hotels/autocomplete", &params, MAX_ATTEMPTS)
            .await
        {
            Ok(response) => response,
            Err(error) => {
                let message = if error.is_timeout() {
                    format!(
                        "{error}. The request exceeded the {}s Scrappa API timeout.",
                        REQUEST_TIMEOUT.as_secs()
                    )
                } else {
                    error.to_string()
                };
                record_query_failure(&mut failures, query, &message);
                continue;
            }
        };

        let items = response::build_dataset_items(&response, query, &request.common_params);
        suggestion_count += items.len();
        completed_queries += 1;

        if items.is_empty() {
            println!("No suggestions found for query \"{query}\"");
            continue;
        }

        match &mut pricing {
            PricingMode::PayPerEvent(budget) => {
                let save_count = match apify.push_charged_dataset_items(budget, &items).await {
                    Ok(save_count) => save_count,
                    Err(error) => {
                        record_query_failure(&mut failures, query, &format!("{error:#}"));
                        continue;
                    }
                };
                saved_suggestion_count += save_count;
                if save_count < items.len() || budget.affordable_count(1) == 0 {
                    charge_limit_reached = true;
                    println!("Charge limit reached after saving {saved_suggestion_count} suggestion result(s)");
                    break;
                }
            }
            PricingMode::NonPayPerEvent => {
                if let Err(error) = apify.push_dataset_items(&items).await {
                    record_query_failure(&mut failures, query, &format!("{error:#}"));
                    continue;
                }
                saved_suggestion_count += items.len();
            }
        }
    }

    if completed_queries == 0 && !failures.is_empty() {
        let details = failures
            .iter()
            .map(|failure| format!("{}: {}", failure.query, failure.error))
            .collect::<Vec<_>>()
            .join("; ");
        bail!("All queries failed: {details}");
    }

    let failed_queries = failures
        .iter()
        .map(|failure| json!({"query": failure.query, "error": failure.error}))
        .collect::<Vec<_>>();
    let summary = json!({
        "requested_queries": request.queries.len(),
        "completed_queries": completed_queries,
        "failed_queries": failed_queries,
        "suggestions_found": suggestion_count,
        "suggestions_saved": saved_suggestion_count,
        "charge_event": apify::RESULT_CHARGE_EVENT,
        "charge_limit_reached": charge_limit_reached,
    });
    apify.put_output(&summary).await?;
    println!("Google Hotels autocomplete completed: {summary}");
    let status_message = if charge_limit_reached {
        format!("Charge limit reached after {saved_suggestion_count} suggestion results.")
    } else {
        format!(
            "Saved {saved_suggestion_count} suggestion results from {completed_queries} queries{}.",
            if failures.is_empty() {
                String::new()
            } else {
                format!("; {} failed", failures.len())
            }
        )
    };
    println!("{status_message}");
    apify.set_terminal_status_message(&status_message).await?;
    Ok(())
}

fn record_query_failure(failures: &mut Vec<QueryFailure>, query: &str, message: &str) {
    failures.push(QueryFailure {
        query: query.to_owned(),
        error: message.to_owned(),
    });
    eprintln!("Query \"{query}\" failed: {message}");
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run_actor().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
