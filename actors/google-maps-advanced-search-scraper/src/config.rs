use std::env;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;
use url::Url;

pub(crate) const APIFY_API_DEFAULT: &str = "https://api.apify.com";
pub(crate) const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
pub(crate) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) struct Config {
    pub(crate) apify_api_base_url: Url,
    pub(crate) scrappa_api_base_url: Url,
    pub(crate) key_value_store_id: String,
    pub(crate) dataset_id: String,
    pub(crate) actor_run_id: String,
    pub(crate) input_key: String,
    pub(crate) apify_token: String,
    pub(crate) scrappa_api_key: String,
    pub(crate) scrappa_request_timeout: Duration,
    pub(crate) pricing_info: Option<Value>,
    pub(crate) charged_event_counts: Option<Value>,
    pub(crate) max_total_charge_usd: Option<f64>,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").unwrap_or_default();
        if scrappa_api_key.is_empty() {
            bail!("SCRAPPA_API_KEY environment variable is not set.");
        }

        let max_total_charge_usd = env::var("ACTOR_MAX_TOTAL_CHARGE_USD")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|value| {
                value
                    .parse::<f64>()
                    .with_context(|| "ACTOR_MAX_TOTAL_CHARGE_USD must be a valid number")
            })
            .transpose()?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
            scrappa_request_timeout: SCRAPPA_REQUEST_TIMEOUT,
            pricing_info: optional_json_env("APIFY_ACTOR_PRICING_INFO")?,
            charged_event_counts: optional_json_env("APIFY_CHARGED_ACTOR_EVENT_COUNTS")?,
            max_total_charge_usd,
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

fn optional_json_env(name: &str) -> Result<Option<Value>> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| {
            serde_json::from_str(&value).with_context(|| format!("{name} must contain valid JSON"))
        })
        .transpose()
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

pub(crate) fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments.iter().copied());
    Ok(url)
}
