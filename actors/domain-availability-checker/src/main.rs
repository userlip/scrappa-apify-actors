mod apify;
mod input;
mod results;
mod scrappa;

use std::{env, process, time::Duration};

use anyhow::{anyhow, Context, Result};
use apify::{ApifyClient, ChargeBudget};
use input::get_domain_requests;
use results::{failure_item, success_item};
use scrappa::ScrappaClient;
use serde_json::{json, Value};

const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_millis(25_000);
const DOMAIN_RESULT_CHARGE_EVENT: &str = "domain-result";

struct ActorConfig {
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    scrappa_api_key: String,
    apify_base_url: String,
    scrappa_base_url: String,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify_client = ApifyClient::new(&config.apify_base_url, config.apify_token.clone())?;
    let result = run_actor(&apify_client, &config).await;
    if let Err(error) = &result {
        if let Err(status_error) = apify_client
            .set_status_message(&config.actor_run_id, &format!("{error:#}"))
            .await
        {
            eprintln!("Could not update Actor status message: {status_error:#}");
        }
    }
    result
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|key| !key.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;
        Ok(Self {
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|key| !key.is_empty())
                .unwrap_or_else(|| "INPUT".to_owned()),
            scrappa_api_key,
            apify_base_url: env_or_default("APIFY_API_PUBLIC_BASE_URL", "https://api.apify.com"),
            scrappa_base_url: env_or_default("SCRAPPA_API_BASE_URL", "https://scrappa.co/api"),
        })
    }
}

async fn run_actor(apify: &ApifyClient, config: &ActorConfig) -> Result<()> {
    let input_value = apify
        .get_input(&config.key_value_store_id, &config.input_key)
        .await?;
    let requests = get_domain_requests(&input_value).context("Could not parse Actor input")?;
    if requests.is_empty() {
        return Err(anyhow!(
            "At least one domain is required. Provide domain or domains."
        ));
    }

    let mut charge_budget = apify.get_charge_budget(&config.actor_run_id).await?;
    charge_budget.require_domain_result_event()?;
    let scrappa = ScrappaClient::new(
        config.scrappa_api_key.clone(),
        config.scrappa_base_url.clone(),
        SCRAPPA_REQUEST_TIMEOUT,
    )?;

    let total = requests.len();
    let mut first_result = None;
    let mut succeeded = 0;
    let mut failed = 0;
    let mut status_message = None;
    let mut service_failure_streak = 0;
    let mut fatal_service_failure_message: Option<String> = None;

    println!(
        "Checking availability for {total} domain{}",
        if total == 1 { "" } else { "s" }
    );

    for request in &requests {
        let domain = request.domain.as_deref();
        let mut result = if domain.is_none() {
            let message = request
                .validation_error
                .as_deref()
                .unwrap_or("Invalid domain");
            failure_item(&message, None, &request.input_domain, None)
        } else if let Some(message) = fatal_service_failure_message.as_deref() {
            failure_item(&message, None, &request.input_domain, domain)
        } else if let Some(message) =
            domain_charge_limit_status(&charge_budget, succeeded + failed, total)
        {
            println!("{message}");
            status_message = Some(message.clone());
            failure_item(&message, None, &request.input_domain, domain)
        } else {
            let domain = domain.expect("the invalid-domain branch handled missing domains");
            println!("Checking domain availability: {domain}");
            match scrappa.get_availability(domain).await {
                Ok(response) => {
                    service_failure_streak = 0;
                    success_item(&response, &request.input_domain, domain)
                        .map_err(|message| anyhow!(message))?
                }
                Err(error) if error.is_service_failure() => {
                    service_failure_streak += 1;
                    let message = format!(
                        "Scrappa domain availability service failed after retries: {error}"
                    );
                    eprintln!("{message}");
                    if service_failure_streak >= 2 {
                        fatal_service_failure_message = Some(format!(
                            "Scrappa domain availability service failed repeatedly; stopping further upstream requests after {} of {total} domain(s).",
                            succeeded + failed + 1
                        ));
                    }
                    failure_item(
                        &error,
                        error.http_status(),
                        &request.input_domain,
                        Some(domain),
                    )
                }
                Err(error) if error.is_per_domain_failure() => {
                    eprintln!(
                        "Domain availability returned a per-domain failure for {domain}: {error}"
                    );
                    service_failure_streak = 0;
                    failure_item(
                        &error,
                        error.http_status(),
                        &request.input_domain,
                        Some(domain),
                    )
                }
                Err(error) => return Err(anyhow!(error.to_string())),
            }
        };

        if !publish_domain_result(apify, config, &mut charge_budget, &result).await? {
            let message = format!(
                "Charge limit reached before saving {}; stopping batch without writing uncharged success results.",
                result
                    .get("domain")
                    .and_then(Value::as_str)
                    .unwrap_or(&request.input_domain)
            );
            eprintln!("{message}");
            status_message = Some(message.clone());
            result = failure_item(&message, None, &request.input_domain, domain);
            if !publish_domain_result(apify, config, &mut charge_budget, &result).await? {
                eprintln!("Charge limit prevented saving a domain failure result.");
            }
        }

        if first_result.is_none() {
            first_result = Some(result.clone());
        }
        if result.get("success") == Some(&Value::Bool(true)) {
            succeeded += 1;
        } else {
            failed += 1;
        }
    }

    let output = if total == 1 {
        first_result.expect("a non-empty request list has one result")
    } else {
        json!({ "requested": total, "succeeded": succeeded, "failed": failed })
    };
    apify
        .set_output(&config.key_value_store_id, &output)
        .await?;

    println!("Domain availability checks completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string_pretty(&json!({
            "requested": total,
            "succeeded": succeeded,
            "failed": failed
        }))?
    );

    if let Some(message) = status_message {
        if let Err(error) = apify
            .set_status_message(&config.actor_run_id, &message)
            .await
        {
            eprintln!("Could not update Actor status message: {error:#}");
        }
        return Ok(());
    }

    if let Some(message) = fatal_service_failure_message {
        return Err(anyhow!(message));
    }

    Ok(())
}

fn domain_charge_limit_status(
    budget: &ChargeBudget,
    processed: usize,
    requested: usize,
) -> Option<String> {
    if !budget.is_pay_per_event() {
        return None;
    }
    if budget.max_event_charge_count_within_limit(DOMAIN_RESULT_CHARGE_EVENT) > 0
        && budget.can_push_item(Some(DOMAIN_RESULT_CHARGE_EVENT))
    {
        return None;
    }
    Some(format!(
        "Charge limit reached before fetching the next domain availability result; {processed} of {requested} domain(s) were processed."
    ))
}

async fn publish_domain_result(
    apify: &ApifyClient,
    config: &ActorConfig,
    budget: &mut ChargeBudget,
    item: &Value,
) -> Result<bool> {
    let is_success = item.get("success") == Some(&Value::Bool(true));
    let event_name =
        (budget.is_pay_per_event() && is_success).then_some(DOMAIN_RESULT_CHARGE_EVENT);
    if !budget.can_push_item(event_name) {
        return Ok(false);
    }

    apify.push_dataset_item(&config.dataset_id, item).await?;
    budget.record_dataset_item();
    if let Some(event_name) = event_name {
        apify.charge_event(&config.actor_run_id, event_name).await?;
        budget.record_charge(event_name);
    }
    Ok(true)
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{name} environment variable is not set"))
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}
