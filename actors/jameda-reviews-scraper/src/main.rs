mod apify;
mod request_params;
mod response_utils;
mod scrappa;

use anyhow::{anyhow, Context, Result};
use apify::ApifyClient;
use request_params::{
    build_request_params, build_request_plan, describe_request, DoctorUrlFailure,
};
use response_utils::{build_output_summary, build_review_dataset_item, get_reviews};
use serde_json::{json, Map, Value};
use std::{env, process};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const JAMEDA_REVIEW_CHARGE_EVENT: &str = "jameda-review-result";

struct ActorConfig {
    apify_api_base: String,
    apify_token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_base: String,
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
            apify_api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_base: env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
            scrappa_api_key,
        })
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

async fn run_actor(config: ActorConfig) -> Result<()> {
    let apify = ApifyClient::new(&config.apify_api_base, config.apify_token.clone())?;
    let input = apify
        .get_input(&config.key_value_store_id, &config.input_key)
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let plan = build_request_plan(&apply_input_defaults(input)).map_err(anyhow::Error::msg)?;
    println!("Fetching Jameda reviews for {}", describe_request(&plan));

    let scrappa = scrappa::ScrappaClient::new(
        config.scrappa_api_key.clone(),
        config.scrappa_api_base.clone(),
    )?;
    let mut failures = plan.input_failures.clone();
    let mut saved_reviews = 0;
    let mut status_message = None;
    let mut remaining_event_capacity = apify
        .event_capacity(&config.actor_run_id, JAMEDA_REVIEW_CHARGE_EVENT)
        .await?;

    for (doctor_index, doctor_url) in plan.doctor_urls.iter().enumerate() {
        if remaining_event_capacity == Some(0) {
            status_message = Some(format!(
                "Charge limit reached before fetching Jameda reviews for {doctor_url}; {saved_reviews} review(s) were saved."
            ));
            println!("{}", status_message.as_deref().unwrap_or_default());
            break;
        }

        let params = build_request_params(&plan, doctor_url);
        println!("Fetching Jameda reviews for {doctor_url}");

        let result = process_doctor(
            &apify,
            &scrappa,
            &config,
            doctor_url,
            doctor_index,
            &params,
            &mut remaining_event_capacity,
        )
        .await;

        match result {
            Ok((found_count, saved_count, limit_message)) => {
                saved_reviews += saved_count;
                println!("Found {found_count} Jameda review result(s) for {doctor_url}; saved {saved_count}");
                if limit_message.is_some() {
                    status_message = limit_message;
                    break;
                }
            }
            Err(error) => {
                let message = scrappa::actor_error_message(&error);
                failures.push(DoctorUrlFailure {
                    doctor_url: doctor_url.clone(),
                    error: message.clone(),
                });
                eprintln!("Failed to fetch Jameda reviews for {doctor_url}: {message}");
            }
        }
    }

    if status_message.is_none() && !failures.is_empty() {
        status_message = Some(format!(
            "{} Jameda review request(s) failed; {saved_reviews} review(s) saved.",
            failures.len()
        ));
    }

    let output = build_output_summary(&plan, saved_reviews, &failures, status_message.as_deref());
    apify
        .set_output(&config.key_value_store_id, &output)
        .await?;

    println!(
        "Results summary: {}",
        json!({
            "doctors_requested": plan.doctor_urls.len(),
            "reviews_saved": saved_reviews,
            "requests_failed": failures.len(),
        })
    );

    if saved_reviews == 0 && !failures.is_empty() {
        let message = status_message.unwrap_or_else(|| "No Jameda reviews were saved.".to_owned());
        let _ = apify
            .set_status_message(&config.actor_run_id, &message)
            .await;
        anyhow::bail!(message);
    }

    if let Some(message) = status_message {
        apify
            .set_status_message(&config.actor_run_id, &message)
            .await
            .context("Could not set the Jameda run status message")?;
        return Ok(());
    }

    println!("Jameda reviews extraction completed successfully");
    Ok(())
}

async fn process_doctor(
    apify: &ApifyClient,
    scrappa: &scrappa::ScrappaClient,
    config: &ActorConfig,
    doctor_url: &str,
    doctor_index: usize,
    params: &Map<String, Value>,
    remaining_event_capacity: &mut Option<usize>,
) -> Result<(usize, usize, Option<String>)> {
    let response = scrappa.get("/jameda/reviews", params).await?;
    let reviews = get_reviews(&response).map_err(anyhow::Error::msg)?;
    let items = reviews
        .iter()
        .map(|review| build_review_dataset_item(review, doctor_url, params, &response))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(anyhow::Error::msg)?;

    let Some(remaining_capacity) = remaining_event_capacity.as_mut() else {
        apify.push_dataset_items(&config.dataset_id, &items).await?;
        return Ok((items.len(), items.len(), None));
    };

    let saved_count = items.len().min(*remaining_capacity);
    if saved_count > 0 {
        apify
            .push_dataset_items(&config.dataset_id, &items[..saved_count])
            .await?;
        *remaining_capacity = (*remaining_capacity).saturating_sub(saved_count);
        let idempotency_key = format!(
            "{}-{}-{}-{}",
            config.actor_run_id, JAMEDA_REVIEW_CHARGE_EVENT, doctor_index, saved_count
        );
        apify
            .charge_event(
                &config.actor_run_id,
                JAMEDA_REVIEW_CHARGE_EVENT,
                saved_count,
                &idempotency_key,
            )
            .await?;
    }

    let limit_reached = saved_count < items.len() || (saved_count > 0 && *remaining_capacity == 0);
    let limit_message = limit_reached.then(|| {
        format!(
            "Charge limit reached after saving {saved_count} of {} Jameda review results.",
            items.len()
        )
    });
    Ok((items.len(), saved_count, limit_message))
}

fn apply_input_defaults(mut input: Value) -> Value {
    if let Some(input) = input.as_object_mut() {
        if !input.contains_key("sort") {
            input.insert("sort".to_owned(), Value::String("newest".to_owned()));
        }
    }
    input
}

#[tokio::main]
async fn main() {
    if let Err(error) = run_actor_from_env().await {
        eprintln!("Actor failed: {}", scrappa::actor_error_message(&error));
        process::exit(1);
    }
}

async fn run_actor_from_env() -> Result<()> {
    run_actor(ActorConfig::from_env()?).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn applies_schema_sort_default_only_when_omitted() {
        assert_eq!(
            apply_input_defaults(json!({"doctor_url": "x"}))["sort"],
            "newest"
        );
        assert_eq!(
            apply_input_defaults(json!({"doctor_url": "x", "sort": null}))["sort"],
            Value::Null
        );
        assert_eq!(
            apply_input_defaults(json!({"doctor_url": "x", "sort": "oldest"}))["sort"],
            "oldest"
        );
    }
}
