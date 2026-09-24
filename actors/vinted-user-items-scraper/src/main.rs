mod apify;
mod input;
mod response;
mod runner;
mod scrappa;

use std::process;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    apify::{set_failure_status_message_from_env, ActorConfig, ApifyClient},
    input::{build_plan, ActorInput},
    runner::run_actor,
    scrappa::ScrappaClient,
};

#[cfg(test)]
const ACTOR_TIMEOUT_SECS: u64 = 420;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let message = runner::timeout_status_message(&error).unwrap_or_else(|| error.to_string());
        eprintln!("Actor failed: {message}");
        if let Err(status_error) = set_failure_status_message_from_env(&message).await {
            eprintln!("Could not set terminal Apify failure status message: {status_error}");
        }
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify = ApifyClient::new(config)?;
    let mut pricing = apify.read_pricing().await?;
    let input_value = apify.read_input().await?;
    let input = deserialize_input(input_value)?;
    let plan = build_plan(&input)?;

    println!("Fetching Vinted user items for {}", plan.describe());
    let scrappa = ScrappaClient::new(apify.scrappa_api_key(), apify.scrappa_base());
    let summary = run_actor(&apify, &scrappa, &plan, &mut pricing).await?;

    println!("Vinted user items completed successfully");
    println!(
        "Results summary: {}",
        serde_json::to_string(&serde_json::json!({
            "users_requested": plan.user_ids.len(),
            "pages_fetched": summary.pages_fetched,
            "items_extracted": summary.saved_items,
            "status_message": summary.status_message,
        }))?
    );
    Ok(())
}

fn deserialize_input(input: Value) -> Result<ActorInput> {
    match input {
        Value::Null => Ok(ActorInput::default()),
        input => serde_json::from_value(input).context("Could not parse Apify INPUT"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_input_uses_defaults_before_validation() {
        let input = deserialize_input(Value::Null).unwrap();
        let error = build_plan(&input).unwrap_err();
        assert!(error
            .to_string()
            .contains("Provide at least one Vinted seller user_id"));
    }

    #[test]
    fn actor_timeout_remains_aligned_with_actor_metadata() {
        let actor: Value = serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();
        assert_eq!(
            actor["defaultRunOptions"]["timeoutSecs"],
            ACTOR_TIMEOUT_SECS
        );
    }

    #[test]
    fn input_prefill_remains_available_to_apify_users() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["properties"]["user_id"]["prefill"], "12345678");
        assert_eq!(schema["properties"]["user_ids"]["maxItems"], 100);
    }

    #[test]
    fn actor_input_ignores_apify_schema_metadata() {
        let input: ActorInput = serde_json::from_value(json!({
            "user_id": "12345678",
            "metadata": {"ignored": true}
        }))
        .unwrap();
        assert!(build_plan(&input).is_ok());
    }
}
