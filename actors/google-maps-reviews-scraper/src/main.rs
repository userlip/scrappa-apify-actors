mod apify;
mod input;
mod scrappa;

#[cfg(test)]
mod test_support;

use anyhow::{anyhow, Result};
use input::ReviewsInput;
use serde_json::{json, Map, Value};
use std::{env, process};

const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const APIFY_API_DEFAULT: &str = "https://api.apify.com";

struct Config {
    apify_api_base: String,
    apify_token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_base: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;
        Ok(Self {
            apify_api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_base: env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
            scrappa_api_key,
        })
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn build_summary(input: &ReviewsInput, response: &Value) -> Value {
    let mut summary = Map::new();
    summary.insert(
        "reviews_extracted".to_owned(),
        json!(response
            .get("items")
            .and_then(Value::as_array)
            .map_or(0, Vec::len)),
    );
    summary.insert("business_id".to_owned(), json!(input.business_id));
    if let Some(sort_order) = input.sort_name() {
        summary.insert("sort_order".to_owned(), json!(sort_order));
    }
    summary.insert(
        "has_next_page".to_owned(),
        json!(response.get("nextPage").is_some_and(js_truthy)),
    );
    Value::Object(summary)
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let apify = apify::ApifyClient::new(
        &config.apify_api_base,
        config.apify_token,
        config.actor_run_id,
        config.key_value_store_id,
        config.dataset_id,
        config.input_key,
    )?;
    let input = parse_actor_input(apify.get_input().await?)?;
    println!(
        "Fetching reviews for business ID: \"{}\" sorted by: {}",
        input.business_id,
        input.sort_name().unwrap_or("undefined")
    );

    let scrappa = scrappa::ScrappaClient::new(config.scrappa_api_base, config.scrappa_api_key)?;
    let response = scrappa.get_reviews(&input).await?;
    let items = response
        .get("items")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if items.is_empty() {
        println!("No reviews found for the given search criteria");
    } else {
        let mut remaining_budget = apify.dataset_item_budget().await?;
        let saved = apify.push_data(items, &mut remaining_budget).await?;
        println!("Found {} reviews", items.len());
        if saved < items.len() {
            println!(
                "Saved {saved} reviews; skipped {} dataset rows because the pay-per-event budget was exhausted",
                items.len() - saved
            );
        }
    }

    apify.set_output(&response).await?;
    println!("Google Maps Reviews extraction completed successfully");
    println!("Results summary: {}", build_summary(&input, &response));
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {}", actor_error_message(&error));
        process::exit(1);
    }
}

fn actor_error_message(error: &anyhow::Error) -> String {
    format!("{error:#}")
}

fn parse_actor_input(input: Option<Value>) -> Result<ReviewsInput> {
    ReviewsInput::parse(input.unwrap_or(Value::Null))
}

#[cfg(test)]
mod tests {
    use super::{build_summary, js_truthy};
    use crate::input::ReviewsInput;
    use serde_json::json;

    #[test]
    fn summary_retains_review_count_sort_and_pagination_state() {
        let input = ReviewsInput::parse(json!({"business_id": "place", "sort": 3})).unwrap();
        let summary = build_summary(
            &input,
            &json!({"items": [{"review_id": "r1"}, {"review_id": "r2"}], "nextPage": "token"}),
        );

        assert_eq!(summary["reviews_extracted"], 2);
        assert_eq!(summary["business_id"], "place");
        assert_eq!(summary["sort_order"], "Highest Rating");
        assert_eq!(summary["has_next_page"], true);
    }

    #[test]
    fn pagination_truthiness_matches_the_typescript_summary() {
        assert!(!js_truthy(&json!("")));
        assert!(js_truthy(&json!([])));
        assert!(js_truthy(&json!({})));
    }

    #[test]
    fn missing_input_keeps_the_typescript_business_id_error() {
        assert_eq!(
            super::parse_actor_input(None).unwrap_err().to_string(),
            "Business ID is required"
        );
    }
}
