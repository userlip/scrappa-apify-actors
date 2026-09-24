mod apify;
mod config;
mod input;
mod output;
mod scrappa;

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::env;

use apify::{get_input, push_dataset_data, store_output};
use config::{scrappa_api_key, ActorConfig};
use input::{build_tiktok_followers_params, js_truthy};
use output::{extract_followers, extract_pagination, follower_item, run_dataset_capacity};
use scrappa::{check_scrappa_code, get_scrappa_json, resolve_tiktok_user_id};

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    if !js_truthy(&input) {
        bail!("TikTok unique_id or user_id is required");
    }

    let params = build_tiktok_followers_params(&input, &mut |warning| eprintln!("{warning}"))?;
    println!(
        "Fetching TikTok followers for: {}",
        params.lookup.log_value()
    );

    let lookup_user_id = resolve_tiktok_user_id(client, config, &params).await?;
    let mut request_params = vec![("user_id", lookup_user_id.clone())];
    if let Some(count) = &params.count {
        request_params.push(("count", count.clone()));
    }
    if let Some(time) = &params.time {
        request_params.push(("time", time.clone()));
    }

    let response = get_scrappa_json(
        client,
        config,
        &["tiktok", "user", "followers"],
        &request_params,
    )
    .await?;
    check_scrappa_code(&response, "Followers")?;

    let data = response.get("data");
    let followers = extract_followers(data);
    let (has_next_page, next_time) = extract_pagination(data);
    if followers.is_empty() {
        println!("No followers found for the given TikTok lookup");
    } else {
        println!("Found {} followers", followers.len());
        let max_saved_rows = run_dataset_capacity(client, config, followers.len()).await?;
        if max_saved_rows < followers.len() {
            println!(
                "PPE spending limit allows saving {max_saved_rows} of {} follower rows",
                followers.len()
            );
        }
        let items = followers
            .iter()
            .take(max_saved_rows)
            .map(|follower| follower_item(follower, params.lookup.unique_id(), &lookup_user_id))
            .collect::<Vec<_>>();
        if !items.is_empty() {
            push_dataset_data(client, config, &items).await?;
        }
    }

    store_output(client, config, &response).await?;
    let summary = json!({
        "followers_extracted": followers.len(),
        "has_next_page": has_next_page,
        "next_time": next_time,
        "processed_time": response.get("processed_time").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
    });
    println!("TikTok followers extraction completed successfully");
    println!("Results summary: {}", summary);
    Ok(())
}

pub async fn run() -> Result<()> {
    let api_key = scrappa_api_key(env::var("SCRAPPA_API_KEY").ok().as_deref())?;
    let config = ActorConfig::from_env(api_key)?;
    let client = Client::builder()
        .build()
        .context("Could not create HTTP client")?;
    run_actor(&client, &config).await
}

#[cfg(test)]
use config::SCRAPPA_REQUEST_TIMEOUT;
#[cfg(test)]
use input::{normalize_tiktok_unique_id, normalize_tiktok_user_id, TikTokLookup};
#[cfg(test)]
use output::affordable_dataset_items;

#[cfg(test)]
mod tests;
