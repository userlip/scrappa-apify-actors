mod apify;
mod request_params;
mod response_utils;
mod scrappa;

use anyhow::{anyhow, Context, Result};
use apify::{ApifyClient, ApifyConfig, DatasetBudget};
use chrono::Utc;
use request_params::{build_google_hotels_search_params, describe_google_hotels_search_request};
use response_utils::{build_hotel_dataset_item, get_hotel_properties};
use scrappa::{ScrappaClient, ScrappaError, REQUEST_ATTEMPTS, REQUEST_TIMEOUT};
use serde_json::{json, Value};
use std::{env, process};

const SCRAPPA_API_KEY_MISSING: &str =
    "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.";
const SCRAPPA_ENDPOINT: &str = "/google-hotels/search";

async fn run() -> Result<()> {
    let scrappa_api_key = env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!(SCRAPPA_API_KEY_MISSING))?;
    let config = ApifyConfig::from_env()?;
    let apify = ApifyClient::new(&config.apify_api_base, &config.apify_token)?;
    let input = apify
        .get_input(&config.key_value_store_id, &config.input_key)
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let params = build_google_hotels_search_params(&input, Utc::now().date_naive())
        .map_err(anyhow::Error::msg)?;

    println!(
        "Searching Google Hotels for {}",
        describe_google_hotels_search_request(&params)
    );

    let scrappa = ScrappaClient::new(scrappa_api_key, &config.scrappa_api_base)?;
    let response = scrappa
        .get(SCRAPPA_ENDPOINT, &params, REQUEST_ATTEMPTS)
        .await
        .map_err(anyhow::Error::new)?;
    let hotels = get_hotel_properties(&response)
        .iter()
        .map(|hotel| build_hotel_dataset_item(hotel, &params))
        .collect::<Result<Vec<_>>>()?;

    if hotels.is_empty() {
        println!("No Google Hotels results found for this request");
    } else {
        let mut budget = DatasetBudget::default();
        let saved = apify
            .push_dataset_items(
                &config.dataset_id,
                &config.actor_run_id,
                &hotels,
                &mut budget,
            )
            .await?;
        println!(
            "Found {} hotel result(s); saved {saved} within the run's PAY_PER_EVENT budget",
            hotels.len()
        );
    }

    apify
        .put_output(&config.key_value_store_id, &response)
        .await
        .context("Failed to write OUTPUT to the default key-value store")?;

    let summary = json!({
        "hotels": hotels.len(),
        "brands": response
            .get("brands")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        "has_next_page": has_next_page(&response),
        "response_time_ms": response.get("response_time_ms").cloned().unwrap_or(Value::Null),
    });
    println!("Google Hotels search completed successfully");
    println!("Results summary: {summary}");
    Ok(())
}

fn has_next_page(response: &Value) -> bool {
    let Some(pagination) = response.get("pagination") else {
        return false;
    };
    js_truthy(pagination.get("next_page_token").unwrap_or(&Value::Null))
        || js_truthy(pagination.get("next").unwrap_or(&Value::Null))
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error
        .downcast_ref::<ScrappaError>()
        .is_some_and(ScrappaError::is_timeout)
    {
        format!(
            "{error}. The Google Hotels request exceeded the {}s Scrappa API timeout. Try a narrower destination, fewer filters, or run the request again.",
            REQUEST_TIMEOUT.as_secs()
        )
    } else {
        format!("{error:#}")
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {}", actor_error_message(&error));
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_pagination_using_google_hotels_response_fields() {
        assert!(has_next_page(
            &json!({"pagination": {"next_page_token": "token"}})
        ));
        assert!(has_next_page(&json!({"pagination": {"next": "url"}})));
        assert!(!has_next_page(
            &json!({"pagination": {"next_page_token": ""}})
        ));
        assert!(!has_next_page(&json!({})));
    }
}
