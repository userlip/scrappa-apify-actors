use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::env;
use url::Url;

mod apify_client;
mod params;
mod response;
mod scrappa_client;

#[cfg(test)]
mod tests;

use apify_client::{ApifyClient, DatasetBudget, APIFY_API_BASE_URL};
use params::{build_params, format_lookup_for_log};
use response::{enrich_post, extract_pagination, extract_posts, js_truthy, validate_scrappa_code};
use scrappa_client::{scrappa_response, SCRAPPA_API_BASE_URL};

pub(crate) struct ActorConfig {
    pub(crate) apify_api_base_url: Url,
    pub(crate) scrappa_api_base_url: Url,
    pub(crate) default_key_value_store_id: String,
    pub(crate) default_dataset_id: String,
    pub(crate) actor_run_id: String,
    pub(crate) input_key: String,
    pub(crate) apify_token: String,
    pub(crate) scrappa_api_key: String,
}

impl ActorConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env(
            "SCRAPPA_API_KEY",
            "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.",
        )?;
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env(
                "ACTOR_DEFAULT_KEY_VALUE_STORE_ID",
                "ACTOR_DEFAULT_KEY_VALUE_STORE_ID environment variable is not set",
            )?,
            default_dataset_id: required_env(
                "ACTOR_DEFAULT_DATASET_ID",
                "ACTOR_DEFAULT_DATASET_ID environment variable is not set",
            )?,
            actor_run_id: required_env(
                "ACTOR_RUN_ID",
                "ACTOR_RUN_ID environment variable is not set",
            )?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env(
                "APIFY_TOKEN",
                "APIFY_TOKEN environment variable is not set",
            )?,
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str, missing_message: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{missing_message}"))
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify = ApifyClient::new(
        config.apify_api_base_url.clone(),
        config.apify_token.clone(),
    )?;

    // Actor.init() loads run pricing before the input or scraper request is processed.
    let run_pricing = apify.get_run(&config.actor_run_id).await?;
    let dataset_budget = DatasetBudget::from_run(&run_pricing)?;
    let input = apify
        .get_input(&config.default_key_value_store_id, &config.input_key)
        .await?
        .filter(js_truthy)
        .ok_or_else(|| anyhow!("TikTok unique_id or user_id is required"))?;
    let params = build_params(&input)?;
    println!(
        "Fetching TikTok user posts for: {}",
        format_lookup_for_log(&input)?
    );

    let response = scrappa_response(&Client::new(), &config, &params).await?;
    validate_scrappa_code(&response)?;
    let data = response.get("data");
    let posts = extract_posts(data);
    let (has_next_page, next_cursor) = extract_pagination(data);
    let posts_saved = if posts.is_empty() {
        println!("No posts found for the given TikTok lookup");
        0
    } else {
        let rows = posts
            .iter()
            .map(|post| enrich_post(post, &params))
            .collect::<Vec<_>>();
        let allowed = dataset_budget.limit(rows.len());
        if allowed < rows.len() {
            println!(
                "Apify spend limit permits saving {allowed} of {} posts",
                rows.len()
            );
        }
        if allowed > 0 {
            apify
                .push_data(&config.default_dataset_id, &rows[..allowed])
                .await?;
        }
        println!("Found {} posts; saved {allowed}", rows.len());
        allowed
    };

    apify
        .set_output(&config.default_key_value_store_id, &response)
        .await?;
    let processed_time = response
        .get("processed_time")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);
    println!("TikTok user posts extraction completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "posts_extracted": posts.len(),
            "posts_saved": posts_saved,
            "has_next_page": has_next_page,
            "next_cursor": next_cursor,
            "processed_time": processed_time,
        })
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}
