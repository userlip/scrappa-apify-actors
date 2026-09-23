mod apify;
mod request_params;
mod review_processing;
mod scrappa;

use anyhow::{Context, Result, anyhow};
use request_params::{RequestPlan, build_request_plan, describe_request, page_params};
use review_processing::{collect_reviews, enrich_review};
use serde_json::{Map, Value, json};
use std::{env, process};

const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const APIFY_API_PUBLIC_BASE_URL: &str = "https://api.apify.com";
const REVIEWS_ENDPOINT: &str = "/trustpilot/company-reviews";

fn required_env(name: &str, missing_message: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{missing_message}"))
}

fn build_output(plan: &RequestPlan, responses: Vec<Value>, reviews_extracted: usize) -> Value {
    let mut request: Map<String, Value> = plan.base_params.clone();
    request.insert("start_page".into(), json!(plan.start_page));
    request.insert("max_pages".into(), json!(plan.max_pages));
    json!({
        "request": request,
        "pages_fetched": responses.len(),
        "reviews_extracted": reviews_extracted,
        "responses": responses,
    })
}

fn optional_page_value(response: Option<&Value>, key: &str) -> Value {
    response
        .and_then(|response| response.get("pagination"))
        .and_then(|pagination| pagination.get(key))
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error
        .downcast_ref::<scrappa::ScrappaTimeoutError>()
        .is_some()
    {
        format!(
            "{error}. The Trustpilot reviews request exceeded the {}s Scrappa API timeout. Try fewer pages or run the request again.",
            scrappa::REQUEST_TIMEOUT_MS / 1000
        )
    } else {
        format!("{error:#}")
    }
}

async fn run() -> Result<()> {
    let api_key = required_env(
        "SCRAPPA_API_KEY",
        "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.",
    )?;
    let api_token = required_env("APIFY_TOKEN", "APIFY_TOKEN environment variable is not set")?;
    let store_id = required_env(
        "ACTOR_DEFAULT_KEY_VALUE_STORE_ID",
        "ACTOR_DEFAULT_KEY_VALUE_STORE_ID environment variable is not set",
    )?;
    let input_key = required_env(
        "ACTOR_INPUT_KEY",
        "ACTOR_INPUT_KEY environment variable is not set",
    )?;
    let dataset_id = required_env(
        "ACTOR_DEFAULT_DATASET_ID",
        "ACTOR_DEFAULT_DATASET_ID environment variable is not set",
    )?;
    let apify_api_base =
        env::var("APIFY_API_PUBLIC_BASE_URL").unwrap_or_else(|_| APIFY_API_PUBLIC_BASE_URL.into());
    let scrappa_api_base =
        env::var("SCRAPPA_API_BASE_URL").unwrap_or_else(|_| SCRAPPA_API_BASE_URL.into());

    let apify = apify::ApifyClient::new(&apify_api_base, api_token)?;
    let input = apify
        .get_input(&store_id, &input_key)
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let plan = build_request_plan(&input).map_err(anyhow::Error::msg)?;
    println!(
        "Fetching Trustpilot company reviews for {}",
        describe_request(&plan)
    );

    let scrappa = scrappa::ScrappaClient::new(api_key, scrappa_api_base)?;
    let mut responses = Vec::new();
    let mut reviews_extracted = 0;
    for offset in 0..plan.max_pages {
        let page = plan.start_page + offset;
        let params = page_params(&plan, page);
        println!(
            "Fetching Trustpilot reviews page {page} for {}",
            params["company_domain"].as_str().unwrap_or_default()
        );
        let response = scrappa.get(REVIEWS_ENDPOINT, &params).await?;
        let reviews = collect_reviews(&response)?;
        if reviews.is_empty() {
            println!("No reviews found on page {page}");
        } else {
            let rows = reviews
                .iter()
                .map(|(review, source)| enrich_review(review, &params, &response, source))
                .collect::<Result<Vec<_>>>()?;
            apify.push_data(&dataset_id, &rows).await?;
            reviews_extracted += rows.len();
            println!("Found {} reviews on page {page}", rows.len());
        }
        let no_next_page = response
            .get("pagination")
            .and_then(|pagination| pagination.get("has_next_page"))
            .and_then(Value::as_bool)
            == Some(false);
        responses.push(response);
        if no_next_page {
            println!("Stopping after page {page}; Scrappa reported no next page");
            break;
        }
    }

    let output = build_output(&plan, responses, reviews_extracted);
    apify.set_output(&store_id, &output).await?;
    let responses = output["responses"]
        .as_array()
        .context("Output responses are missing")?;
    let last_response = responses.last();
    let summary = json!({
        "company_domain": plan.base_params.get("company_domain"),
        "pages_fetched": responses.len(),
        "reviews_extracted": reviews_extracted,
        "total_pages": optional_page_value(last_response, "total_pages"),
        "total_count": optional_page_value(last_response, "total_count"),
        "has_next_page": last_response
            .and_then(|response| response.get("pagination"))
            .and_then(|pagination| pagination.get("has_next_page"))
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Bool(false)),
    });
    println!("Trustpilot company reviews extraction completed successfully");
    println!("Results summary: {summary}");
    Ok(())
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
    use super::build_output;
    use crate::request_params::build_request_plan;
    use serde_json::json;

    #[test]
    fn output_contains_request_counts_and_full_responses() {
        let plan =
            build_request_plan(&json!({"company_domain": "example.com", "max_pages": 2})).unwrap();
        let response = json!({
            "reviews": [{"id": "review-1"}],
            "pagination": {"total_count": 15, "total_pages": 2, "has_next_page": true},
            "businessUnit": {"displayName": "Example"}
        });
        let output = build_output(&plan, vec![response.clone()], 1);
        assert_eq!(output["request"]["start_page"], 1);
        assert_eq!(output["request"]["max_pages"], 2);
        assert_eq!(output["pages_fetched"], 1);
        assert_eq!(output["reviews_extracted"], 1);
        assert_eq!(output["responses"][0], response);
    }
}
