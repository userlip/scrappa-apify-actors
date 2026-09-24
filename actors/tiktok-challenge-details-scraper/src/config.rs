use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use url::Url;

pub(crate) const APIFY_API_DEFAULT: &str = "https://api.apify.com";
pub(crate) const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
pub(crate) const INPUT_KEY: &str = "INPUT";
pub(crate) const OUTPUT_KEY: &str = "OUTPUT";
pub(crate) const CHALLENGE_DETAIL_CHARGE_EVENT: &str = "challenge-detail-result";
pub(crate) const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
pub(crate) const MAX_ENTITIES: usize = 100;
pub(crate) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub(crate) const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
pub(crate) const APIFY_MAX_RETRIES: usize = 2;

pub(crate) struct ActorConfig {
    pub(crate) apify_api_base_url: Url,
    pub(crate) scrappa_api_base_url: Url,
    pub(crate) key_value_store_id: String,
    pub(crate) input_key: String,
    pub(crate) dataset_id: String,
    pub(crate) actor_run_id: String,
    pub(crate) apify_token: String,
    pub(crate) scrappa_api_key: String,
}

impl ActorConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|key| !key.is_empty())
                .unwrap_or_else(|| INPUT_KEY.to_owned()),
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
        })
    }
}

pub(crate) fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.trim().is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

pub(crate) fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

pub(crate) fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}
