use anyhow::{anyhow, Context, Result};
use std::{env, time::Duration};

pub(crate) const APIFY_API_DEFAULT: &str = "https://api.apify.com";
pub(crate) const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
pub(crate) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
pub(crate) const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub(crate) const SCRAPPA_MAX_ATTEMPTS: usize = 3;
pub(crate) const APIFY_MAX_RETRIES: usize = 2;

#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) apify_api_base: String,
    pub(crate) apify_token: String,
    pub(crate) actor_run_id: String,
    pub(crate) key_value_store_id: String,
    pub(crate) dataset_id: String,
    pub(crate) input_key: String,
    pub(crate) scrappa_api_base: String,
    pub(crate) scrappa_api_key: String,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env("SCRAPPA_API_KEY").context(
            "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.",
        )?;
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
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}
