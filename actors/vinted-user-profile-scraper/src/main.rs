mod apify;
mod request_params;
mod response_utils;
mod run_user_profiles;
mod runtime_budget;
mod scrappa;

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::process;

use apify::{ActorConfig, ApifyClient};
use request_params::{build_vinted_user_profile_requests, VintedUserProfileRequest};
use run_user_profiles::run_vinted_user_profiles;
use scrappa::ScrappaClient;

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let actor = ApifyClient::new(&config)?;
    let (input, budget) = tokio::try_join!(actor.get_input(), actor.get_charge_budget())?;
    let input: Value = input
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let requests = build_vinted_user_profile_requests(&input)?;

    println!("Running {} Vinted user profile request(s)", requests.len());
    println!("First request: {}", describe_request(&requests[0]));

    let client = ScrappaClient::new(config.scrappa_api_key, config.scrappa_api_base.as_deref())?;
    let summary = run_vinted_user_profiles(&actor, &client, &requests, budget).await?;

    println!("Vinted user profiles completed");
    println!(
        "Results summary: {}",
        serde_json::to_string(&serde_json::json!({
            "requested": summary.requested,
            "succeeded": summary.succeeded,
            "failed": summary.failed,
            "statusMessage": summary.status_message
        }))?
    );

    if let Some(status_message) = &summary.status_message {
        actor.set_status_message(status_message).await?;
    }

    Ok(())
}

fn describe_request(request: &VintedUserProfileRequest) -> String {
    request.describe()
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use request_params::build_vinted_user_profile_requests;
    use serde_json::json;

    #[test]
    fn first_request_summary_uses_normalized_country_and_id() {
        let requests = build_vinted_user_profile_requests(&json!({
            "user_ids": "255914028,123456789",
            "country": "de"
        }))
        .unwrap();
        assert_eq!(describe_request(&requests[0]), "255914028 in DE");
    }

    #[test]
    fn actor_input_prefill_and_output_storage_config_remain_compatible() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let actor: Value = serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();

        assert_eq!(
            schema.pointer("/properties/user_id/prefill"),
            Some(&json!("255914028"))
        );
        assert_eq!(
            schema.pointer("/properties/country/default"),
            Some(&json!("FR"))
        );
        assert_eq!(
            schema.pointer("/properties/user_ids/type"),
            Some(&json!(["array", "string"]))
        );
        assert_eq!(
            actor.pointer("/defaultRunOptions/timeoutSecs"),
            Some(&json!(runtime_budget::ACTOR_TIMEOUT_SECS))
        );
        assert_eq!(
            actor.pointer("/storages/dataset/views/profiles/title"),
            Some(&json!("Vinted User Profiles"))
        );
    }
}
