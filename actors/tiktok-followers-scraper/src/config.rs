use anyhow::{anyhow, bail, Context, Result};
use std::{env, time::Duration};
use url::Url;

pub(super) const APIFY_API_BASE_URL: &str = "https://api.apify.com";
pub(super) const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
pub(super) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub(super) const APIFY_DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub(super) struct ActorConfig {
    pub(super) apify_api_base_url: Url,
    pub(super) scrappa_api_base_url: Url,
    pub(super) default_key_value_store_id: String,
    pub(super) default_dataset_id: String,
    pub(super) input_key: String,
    pub(super) actor_run_id: String,
    pub(super) apify_token: String,
    pub(super) scrappa_api_key: String,
    pub(super) scrappa_request_timeout: Duration,
}

impl ActorConfig {
    pub(super) fn from_env(scrappa_api_key: String) -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
            scrappa_request_timeout: SCRAPPA_REQUEST_TIMEOUT,
        })
    }
}

pub(super) fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.trim().is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

pub(super) fn scrappa_api_key(value: Option<&str>) -> Result<String> {
    value
        .filter(|key| !key.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            anyhow!(
                "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."
            )
        })
}

pub(super) fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}
