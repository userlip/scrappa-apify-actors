use std::{env, process::ExitCode};

use anyhow::{anyhow, Result};
mod actor;
mod apify;
mod charging;
mod http_utils;
mod request_params;
mod response_utils;
mod scrappa_client;

use actor::{actor_error_message, run_actor};
use apify::ApifyClient;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";

#[derive(Clone)]
pub(crate) struct ActorConfig {
    pub(crate) apify_api_base: String,
    pub(crate) apify_token: String,
    pub(crate) actor_run_id: String,
    pub(crate) key_value_store_id: String,
    pub(crate) dataset_id: String,
    pub(crate) input_key: String,
    pub(crate) scrappa_api_base: String,
    pub(crate) scrappa_api_key: Option<String>,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_base: env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
            scrappa_api_key: env::var("SCRAPPA_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
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
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

#[tokio::main]
async fn main() -> ExitCode {
    let config = match ActorConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {}", actor_error_message(&error));
            return ExitCode::FAILURE;
        }
    };
    let apify = match ApifyClient::new(&config) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Actor failed: {}", actor_error_message(&error));
            return ExitCode::FAILURE;
        }
    };
    let run = match apify.get_run().await {
        Ok(run) => run,
        Err(error) => {
            eprintln!("Actor failed: {}", actor_error_message(&error));
            return ExitCode::FAILURE;
        }
    };

    match run_actor(config, &apify, &run).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let message = actor_error_message(&error);
            eprintln!("Actor failed: {message}");
            let _ = apify.set_status_message(&message).await;
            ExitCode::FAILURE
        }
    }
}
