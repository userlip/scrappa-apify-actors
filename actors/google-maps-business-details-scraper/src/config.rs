use std::env;

use anyhow::{anyhow, Context, Result};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";

pub struct Config {
    pub apify_api_base: Url,
    pub scrappa_api_base: Url,
    pub apify_token: String,
    pub actor_run_id: String,
    pub key_value_store_id: String,
    pub dataset_id: String,
    pub input_key: String,
    pub scrappa_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("SCRAPPA_API_KEY environment variable is not set."))?;

        Ok(Self {
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .ok()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "INPUT".to_owned()),
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let value = env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned());
    Url::parse(&value).with_context(|| format!("{name} must be a valid absolute URL"))
}

pub fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments: {base_url}"))?
        .pop_if_empty()
        .extend(segments.iter().copied());
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::endpoint_url;
    use url::Url;

    #[test]
    fn endpoint_url_appends_encoded_path_segments_to_base_path() {
        let base = Url::parse("https://scrappa.example/api/").unwrap();
        let endpoint = endpoint_url(&base, &["maps", "business-details"]).unwrap();

        assert_eq!(
            endpoint.as_str(),
            "https://scrappa.example/api/maps/business-details"
        );
    }
}
