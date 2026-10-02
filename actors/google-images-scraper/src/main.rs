use std::{env, sync::Arc};

use anyhow::{anyhow, bail, Context, Result};
use google_images_scraper::{apify::ApifyClient, runner::run_actor, scrappa::ScrappaClient};
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";

struct Config {
    apify_api_base: Url,
    scrappa_api_base: Url,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        if scrappa_api_key.is_empty() {
            bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
        }

        Ok(Self {
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            scrappa_api_base: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env_alias(
                "ACTOR_DEFAULT_KEY_VALUE_STORE_ID",
                "APIFY_DEFAULT_KEY_VALUE_STORE_ID",
            )?,
            dataset_id: required_env_alias("ACTOR_DEFAULT_DATASET_ID", "APIFY_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env_alias("ACTOR_RUN_ID", "APIFY_ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY")
                .or_else(|_| env::var("APIFY_INPUT_KEY"))
                .unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn required_env_alias(primary: &str, fallback: &str) -> Result<String> {
    match env::var(primary) {
        Ok(value) if !value.is_empty() => Ok(value),
        Ok(_) => bail!("Required environment variable {primary} is empty"),
        Err(_) => required_env(fallback),
    }
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn actor_error_message(error: &anyhow::Error) -> String {
    let raw_message = error.to_string();
    if raw_message.contains("timed out") {
        format!(
            "{raw_message}. The Google Images request exceeded the {}s Scrappa API timeout. Try a more specific query or run the request again.",
            google_images_scraper::scrappa::REQUEST_TIMEOUT_MS / 1000
        )
    } else {
        raw_message
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let apify = ApifyClient::new(&config.apify_token, config.apify_api_base.as_str())?;
    let scrappa = Arc::new(ScrappaClient::new(
        &config.scrappa_api_key,
        config.scrappa_api_base.as_str(),
    )?);

    run_actor(
        &apify,
        scrappa,
        &config.key_value_store_id,
        &config.dataset_id,
        &config.actor_run_id,
        &config.input_key,
    )
    .await
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {}", actor_error_message(&error));
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn adds_the_original_timeout_guidance_to_scrappa_timeouts() {
        assert_eq!(
            actor_error_message(&anyhow!("Scrappa API request timed out after 120000ms")),
            "Scrappa API request timed out after 120000ms. The Google Images request exceeded the 120s Scrappa API timeout. Try a more specific query or run the request again."
        );
    }
}
