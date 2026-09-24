mod apify;
mod budget;
mod config;
mod input;
mod scrappa;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};

use apify::{ApifyClient, APIFY_REQUEST_TIMEOUT};
use budget::{load_charge_budget, RESULT_EVENT, SEARCH_EVENT};
use config::Config;
use input::{is_javascript_truthy, json_string};
use scrappa::fetch_search;

async fn run_actor(http: &Client, config: &Config) -> Result<()> {
    if config.scrappa_api_key.is_empty() {
        bail!("SCRAPPA_API_KEY environment variable is not set.");
    }

    let apify = ApifyClient::new(http, config);
    let input = apify.get_input().await?;
    if !input.get("query").is_some_and(is_javascript_truthy) || input.get("zoom").is_none() {
        bail!("Search query and zoom level are required");
    }

    let mut budget = load_charge_budget(http, config).await?;
    let search_charge = budget.charge_event(&apify, SEARCH_EVENT, 1).await?;
    if search_charge.event_charge_limit_reached {
        println!("User budget limit reached, stopping.");
        return Ok(());
    }

    let query = input.get("query").map(json_string).unwrap_or_default();
    let zoom = input.get("zoom").map(json_string).unwrap_or_default();
    let location_info = if input.get("latitude").is_some_and(is_javascript_truthy)
        && input.get("longitude").is_some_and(is_javascript_truthy)
    {
        format!(
            "at lat {}, lon {}",
            input.get("latitude").map(json_string).unwrap_or_default(),
            input.get("longitude").map(json_string).unwrap_or_default()
        )
    } else {
        "(auto-resolved location)".to_owned()
    };
    println!("Advanced search: \"{query}\" at zoom {zoom} {location_info}");

    let response = fetch_search(http, config, &input).await?;
    let items = response
        .get("items")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if !items.is_empty() {
        let allowed_items = budget.affordable_dataset_item_count(RESULT_EVENT, items.len())?;
        let result_charge = budget
            .charge_event(&apify, RESULT_EVENT, allowed_items)
            .await?;
        if result_charge.charged > 0 {
            apify
                .push_dataset_items(&items[..result_charge.charged])
                .await?;
            budget.record_default_dataset_items(result_charge.charged)?;
            println!("Found {} results", result_charge.charged);
        }
    } else {
        println!("No results found for the given search criteria");
    }

    apify.put_output(&response).await?;

    let language = input
        .get("hl")
        .filter(|value| is_javascript_truthy(value))
        .map(json_string)
        .unwrap_or_else(|| "en".to_owned());
    let region = input
        .get("gl")
        .filter(|value| is_javascript_truthy(value))
        .map(json_string)
        .unwrap_or_else(|| "worldwide".to_owned());
    let results_found = response
        .get("items")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    println!(
        "Advanced search completed: {}",
        json!({
            "query": query,
            "results_found": results_found,
            "zoom_level": input.get("zoom"),
            "language": language,
            "region": region
        })
    );
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Actor failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let config = Config::from_env()?;
    let http = Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .context("Could not create HTTP client")?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Could not start Tokio runtime")?;
    runtime.block_on(run_actor(&http, &config))
}
