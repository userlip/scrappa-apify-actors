use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";

pub const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
pub const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub const INPUT_KEY: &str = "INPUT";
pub const OUTPUT_KEY: &str = "OUTPUT";

pub struct ActorConfig {
    pub apify_api_base_url: Url,
    pub scrappa_api_base_url: Url,
    pub key_value_store_id: String,
    pub input_key: String,
    pub dataset_id: String,
    pub actor_run_id: String,
    pub apify_token: String,
    pub scrappa_api_key: Option<String>,
}

impl ActorConfig {
    pub fn from_env() -> Result<Self> {
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
            scrappa_api_key: env::var("SCRAPPA_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
        })
    }

    pub fn require_scrappa_api_key(&self) -> Result<&str> {
        self.scrappa_api_key.as_deref().ok_or_else(|| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
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

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_has_production_request_deadlines() {
        assert_eq!(SCRAPPA_REQUEST_TIMEOUT.as_secs(), 90);
        assert_eq!(APIFY_REQUEST_TIMEOUT.as_secs(), 60);
        assert_eq!(INPUT_KEY, "INPUT");
        assert_eq!(OUTPUT_KEY, "OUTPUT");
    }
}
