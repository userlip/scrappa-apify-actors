mod apify;
mod tiktok_video;

#[cfg(test)]
mod tests;

use anyhow::{anyhow, Context, Result};
use apify::{ActorConfig, ApifyClient, DatasetBudget};
use reqwest::Client;
use serde_json::{json, Value};
use tiktok_video::{
    assert_successful_response, dataset_item, extract_video, format_video_lookup_for_log, js_trim,
    js_truthy, resolve_video_requests, ScrappaClient,
};

async fn run_actor(http: &Client, config: &ActorConfig) -> Result<()> {
    let apify = ApifyClient { http, config };
    let input = apify
        .get_input()
        .await?
        .filter(js_truthy)
        .ok_or_else(|| anyhow!("At least one TikTok video URL is required"))?;
    let requests = resolve_video_requests(&input)?;
    let hd = input.get("hd") == Some(&Value::Bool(true));
    let scrappa = ScrappaClient {
        http,
        base_url: &config.scrappa_api_base_url,
        api_key: &config.scrappa_api_key,
    };
    let mut dataset_budget = DatasetBudget::new();
    let mut dataset_items = 0;
    let mut videos_found = 0;
    let mut lookups_failed = 0;

    println!(
        "Fetching TikTok video details for {} URL{}",
        requests.len(),
        if requests.len() == 1 { "" } else { "s" }
    );

    for (index, request) in requests.iter().enumerate() {
        if !dataset_budget.can_save_one(&apify).await? {
            eprintln!(
                "Pay-per-event spending limit reached; stopping before lookup {}/{}",
                index + 1,
                requests.len()
            );
            break;
        }

        let request_index = index + 1;
        let formatted_url = format_video_lookup_for_log(&request.url)
            .unwrap_or_else(|_| js_trim(&request.url).to_owned());
        let lookup_result = match &request.validation_error {
            Some(message) => Err(anyhow!(message.clone())),
            None => {
                println!(
                    "Fetching TikTok video {request_index}/{}: {formatted_url}",
                    requests.len()
                );
                scrappa
                    .get_video(&request.url, hd)
                    .await
                    .and_then(|response| {
                        assert_successful_response(&response, &request.url)?;
                        Ok(response)
                    })
            }
        };

        let item = match lookup_result {
            Ok(response) => {
                let video = extract_video(response.get("data"), &request.url);
                if video.is_some() {
                    videos_found += 1;
                    println!("Found 1 TikTok video record");
                } else {
                    println!("No video details found for: {formatted_url}");
                }
                dataset_item(
                    video,
                    &request.url,
                    hd,
                    Some(&response),
                    request_index,
                    None,
                )
            }
            Err(error) => {
                let message = format!("{error:#}");
                eprintln!("TikTok video lookup failed for {formatted_url}: {message}");
                lookups_failed += 1;
                dataset_item(None, &request.url, hd, None, request_index, Some(message))
            }
        };

        apify.push_dataset_item(&item).await?;
        dataset_budget.record_saved_row();
        dataset_items += 1;
    }

    let summary = json!({
        "urls_requested": requests.len(),
        "dataset_items": dataset_items,
        "videos_found": videos_found,
        "lookups_failed": lookups_failed,
        "hd_requested": hd,
    });
    println!("TikTok video details extraction completed successfully");
    println!("Results summary: {summary}");
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let http = Client::builder()
        .build()
        .context("Failed to create HTTP client")?;
    run_actor(&http, &config).await
}
