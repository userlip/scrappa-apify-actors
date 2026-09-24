mod apify;
mod input;
mod scrappa;
#[cfg(test)]
mod tests;
mod tiktok_response;

use anyhow::{bail, Result};
use apify::{endpoint_url, get_input, push_dataset_items, put_output, ActorConfig, DatasetBudget};
use input::{build_hashtag_posts_params, TikTokHashtagPostsParams};
use scrappa::{
    fetch_scrappa_response, resolve_challenge, validate_scrappa_code, SCRAPPA_REQUEST_TIMEOUT,
};
use serde_json::{json, Value};
use tiktok_response::{enrich_post, extract_pagination, extract_posts, js_truthy};

async fn run_actor(client: &reqwest::Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let Some(input) = input.filter(js_truthy) else {
        bail!("TikTok challenge_id or challenge_name is required");
    };
    let params = build_hashtag_posts_params(&input)?;
    println!("Fetching TikTok hashtag posts for: {}", params.lookup_label);

    let mut posts_params = TikTokHashtagPostsParams {
        challenge_name: params.challenge_name.clone(),
        challenge_id: params.challenge_id.clone(),
        region: params.region.clone(),
        count: params.count,
        cursor: params.cursor.clone(),
        lookup_label: params.lookup_label.clone(),
    };
    let mut resolved_challenge_name = posts_params.challenge_name.clone();
    if let Some(challenge_name) = posts_params.challenge_name.clone() {
        if posts_params.challenge_id.is_none() {
            let (challenge_id, resolved_name) =
                resolve_challenge(client, config, &challenge_name).await?;
            posts_params.challenge_id = Some(challenge_id.clone());
            posts_params.challenge_name = None;
            resolved_challenge_name = resolved_name;
            println!(
                "Resolved hashtag to challenge_id:{challenge_id}{}",
                resolved_challenge_name
                    .as_ref()
                    .map(|name| format!(" ({name})"))
                    .unwrap_or_default()
            );
        }
    }

    let mut url = endpoint_url(
        &config.scrappa_api_base_url,
        &["tiktok", "challenges", "posts"],
    )?;
    posts_params.append_to_url(&mut url);
    let query = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    let query = query
        .iter()
        .map(|(key, value)| (key.as_str(), value.clone()))
        .collect::<Vec<_>>();
    let response =
        fetch_scrappa_response(client, config, &["tiktok", "challenges", "posts"], &query).await?;
    validate_scrappa_code(&response, "TikTok Hashtag Posts")?;

    let data = response.get("data");
    let posts = extract_posts(data);
    let (has_next_page, next_cursor) = extract_pagination(data);
    let saved_posts = if posts.is_empty() {
        println!("No posts found for the given TikTok hashtag lookup");
        0
    } else {
        let rows = posts
            .iter()
            .map(|post| {
                enrich_post(
                    post,
                    &params,
                    resolved_challenge_name.as_deref(),
                    posts_params.challenge_id.as_deref(),
                )
            })
            .collect::<Vec<_>>();
        let saved =
            push_dataset_items(client, config, &mut DatasetBudget::default(), &rows).await?;
        println!("Found {} posts; saved {saved}", posts.len());
        saved
    };

    put_output(client, config, &response).await?;
    let processed_time = response
        .get("processed_time")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);
    println!("TikTok hashtag posts extraction completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "posts_extracted": posts.len(),
            "posts_saved": saved_posts,
            "has_next_page": has_next_page,
            "next_cursor": next_cursor,
            "processed_time": processed_time,
        })
    );
    Ok(())
}
fn failure_message(error: &anyhow::Error) -> String {
    let message = format!("{error:#}");
    if message.contains("timed out") {
        format!(
            "{message}. The TikTok hashtag posts request exceeded the {}s Scrappa API timeout. Try a more specific hashtag or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

#[tokio::main]
async fn main() {
    let result = async {
        let config = ActorConfig::from_env()?;
        run_actor(&reqwest::Client::new(), &config).await
    }
    .await;
    if let Err(error) = result {
        eprintln!("Actor failed: {}", failure_message(&error));
        std::process::exit(1);
    }
}
