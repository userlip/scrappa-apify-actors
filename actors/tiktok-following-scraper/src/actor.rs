use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};

use crate::{
    apify::{self, ActorConfig, DatasetBudget},
    input::{build_params, format_lookup},
    scrappa::{
        self, dataset_item, extract_pagination, extract_profile_user_id, following_items,
        following_url, profile_url, Pagination,
    },
    value::{js_strict_equal, requested_count_json},
    PAGE_SIZE,
};

pub(crate) async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = apify::get_input(client, config)
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("TikTok unique_id or user_id is required"))?;
    let mut params = build_params(&input, |warning| eprintln!("Warning: {warning}"))?;
    let requested_count = params.requested_count();
    let original_lookup_unique_id = params.unique_id.clone();

    println!("Fetching TikTok following for: {}", format_lookup(&input));
    if let Some(unique_id) = params
        .unique_id
        .clone()
        .filter(|_| params.user_id.is_none())
    {
        println!("Resolving TikTok username {unique_id} to numeric user_id");
        let response = scrappa::fetch_scrappa_json(
            client,
            &profile_url(&config.scrappa_api_base_url, &unique_id)?,
            &config.scrappa_api_key,
        )
        .await?;
        scrappa::check_scrappa_code(&response, "Profile")?;
        let user_id = extract_profile_user_id(response.get("data"))
            .ok_or_else(|| anyhow!("Could not resolve {unique_id} to a TikTok user_id"))?;
        params.user_id = Some(user_id);
        params.unique_id = None;
    }

    let mut latest_response: Option<Value> = None;
    let mut latest_pagination = Pagination {
        has_next_page: false,
        next_time: None,
    };
    let mut next_time = params.time.clone();
    let mut following_extracted = 0usize;
    let mut pages_fetched = 0usize;
    let mut dataset_budget = DatasetBudget::default();

    while (following_extracted as f64) < requested_count {
        let current_time = next_time.clone();
        let remaining_count =
            (requested_count - following_extracted as f64).min(usize::MAX as f64) as usize;
        let page_count = PAGE_SIZE.min(remaining_count);
        let url = following_url(
            &config.scrappa_api_base_url,
            &params,
            page_count,
            current_time.as_ref(),
        )?;

        println!(
            "Fetching TikTok following page {} ({page_count} requested)",
            pages_fetched + 1
        );
        let response = scrappa::fetch_scrappa_json(client, &url, &config.scrappa_api_key).await?;
        latest_response = Some(response.clone());
        pages_fetched += 1;

        scrappa::check_scrappa_code(&response, "Following")?;
        let data = response.get("data");
        let following = following_items(data);
        latest_pagination = extract_pagination(data);

        if !following.is_empty() {
            let items = following
                .iter()
                .take(remaining_count)
                .map(|user| {
                    dataset_item(
                        user,
                        original_lookup_unique_id.as_deref(),
                        params.user_id.as_deref(),
                    )
                })
                .collect::<Vec<_>>();
            let saved =
                apify::push_dataset_items(client, config, &items, &mut dataset_budget).await?;
            following_extracted += saved;
            println!("Found {saved} followed accounts on page {pages_fetched}");

            if saved < items.len() {
                println!(
                    "PPE budget allowed {saved} of {} dataset items on page {pages_fetched}",
                    items.len()
                );
                break;
            }
        }

        if following.is_empty()
            || !latest_pagination.has_next_page
            || latest_pagination.next_time.is_none()
            || current_time.as_ref().is_some_and(|current| {
                js_strict_equal(latest_pagination.next_time.as_ref().unwrap(), current)
            })
        {
            break;
        }

        next_time = latest_pagination.next_time.clone();
    }

    if following_extracted == 0 {
        println!("No followed accounts found for the given TikTok lookup");
    } else {
        println!("Found {following_extracted} followed accounts");
    }

    let summary = json!({
        "following_extracted": following_extracted,
        "requested_count": requested_count_json(requested_count),
        "pages_fetched": pages_fetched,
        "has_next_page": latest_pagination.has_next_page,
        "next_time": latest_pagination.next_time,
        "processed_time": latest_response
            .as_ref()
            .and_then(|response| response.get("processed_time"))
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    });
    let output = if requested_count <= PAGE_SIZE as f64 {
        latest_response.as_ref().unwrap_or(&summary)
    } else {
        &summary
    };
    apify::set_output(client, config, output).await?;

    println!("TikTok following extraction completed successfully");
    println!("Results summary: {}", summary);
    Ok(())
}
