use anyhow::{bail, Result};
use reqwest::Client;
use serde_json::{json, Value};

use crate::{
    apify::{
        charge_timeline_points, get_actor_run, get_input, ppe_items_result, push_dataset_items,
        put_output, put_terminal_status_message, PpeBudget, TIMELINE_POINT_CHARGE_EVENT,
    },
    config::Config,
    input::build_interest_params,
    response::build_timeline_dataset_items,
    scrappa::{fetch_interest, ScrappaTimeoutError, SCRAPPA_REQUEST_TIMEOUT},
};

pub async fn run_actor(client: &Client, config: &Config) -> Result<()> {
    let run = get_actor_run(client, config).await?;
    let ppe_budget = PpeBudget::from_actor_run(&run)?;
    if config.scrappa_api_key.is_empty() {
        bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
    }
    let input = get_input(client, config).await?;
    if input.is_null() {
        bail!("Input is required");
    }

    let params = build_interest_params(&input)?;
    println!(
        "Fetching Google Trends interest over time for {}",
        params.describe()
    );
    let response = fetch_interest(client, config, &params).await?;
    let dataset_items = build_timeline_dataset_items(&response, &params);

    if dataset_items.is_empty() {
        println!("No Google Trends timeline points found for this request");
    } else if let Some(mut budget) = ppe_budget {
        let (kept, custom_charge, dataset_charge) =
            ppe_items_result(&mut budget, dataset_items.len());
        push_dataset_items(client, config, &dataset_items[..kept]).await?;
        charge_timeline_points(client, config, custom_charge.charged_count).await?;
        let charge_result = custom_charge.merge(dataset_charge);
        if charge_result.event_charge_limit_reached
            && charge_result.charged_count < dataset_items.len()
        {
            let status_message =
                "Charge limit reached before saving all Google Trends timeline points.";
            println!(
                "{status_message} {}",
                json!({
                    "event": TIMELINE_POINT_CHARGE_EVENT,
                    "charged_count": charge_result.charged_count,
                    "requested_count": dataset_items.len(),
                })
            );
            if let Err(error) = put_terminal_status_message(client, config, status_message).await {
                eprintln!("Warning: {error}");
            }
            return Ok(());
        }
        put_output(client, config, &response).await?;
    } else {
        push_dataset_items(client, config, &dataset_items).await?;
        put_output(client, config, &response).await?;
    }

    if dataset_items.is_empty() {
        put_output(client, config, &response).await?;
    }
    println!("Google Trends interest scraping completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "timeline_points": dataset_items.len(),
            "average": response.pointer("/interest_over_time/average").filter(|value| !value.is_null()).unwrap_or(&Value::Null),
            "max_value": response.pointer("/interest_over_time/max_value").filter(|value| !value.is_null()).unwrap_or(&Value::Null),
            "min_value": response.pointer("/interest_over_time/min_value").filter(|value| !value.is_null()).unwrap_or(&Value::Null),
            "response_time_ms": response.get("response_time_ms").filter(|value| !value.is_null()).unwrap_or(&Value::Null),
        })
    );
    Ok(())
}

pub fn actor_error_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{}. The Google Trends interest request exceeded the {}s Scrappa API timeout. Try a shorter time range, a more specific keyword, or run the request again.",
            error,
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        error.to_string()
    }
}
