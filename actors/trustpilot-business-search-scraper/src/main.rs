mod apify;
mod charging;
mod request_params;
mod response_utils;
mod scrappa;

use anyhow::{Result, anyhow};
use apify::ApifyClient;
use charging::{BUSINESS_RESULT_CHARGE_EVENT, PpeBudget};
use request_params::{RequestPlan, build_request_plan, describe_request, page_params};
use response_utils::{build_dataset_item, get_businesses, has_next_page};
use scrappa::{REQUEST_TIMEOUT_MS, ScrappaClient, ScrappaTimeoutError};
use serde_json::{Map, Value, json};
use std::{env, process};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";

struct ActorConfig {
    apify_api_base: String,
    scrappa_api_base: String,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base: env::var("APIFY_API_PUBLIC_BASE_URL")
                .unwrap_or_else(|_| APIFY_API_DEFAULT.into()),
            scrappa_api_base: env::var("SCRAPPA_API_BASE_URL")
                .unwrap_or_else(|_| SCRAPPA_API_DEFAULT.into()),
            apify_token: required_env(
                "APIFY_TOKEN",
                "APIFY_TOKEN environment variable is not set",
            )?,
            key_value_store_id: required_env(
                "ACTOR_DEFAULT_KEY_VALUE_STORE_ID",
                "ACTOR_DEFAULT_KEY_VALUE_STORE_ID environment variable is not set",
            )?,
            dataset_id: required_env(
                "ACTOR_DEFAULT_DATASET_ID",
                "ACTOR_DEFAULT_DATASET_ID environment variable is not set",
            )?,
            actor_run_id: required_env(
                "ACTOR_RUN_ID",
                "ACTOR_RUN_ID environment variable is not set",
            )?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".into()),
        })
    }
}

fn required_env(name: &str, message: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!(message.to_owned()))
}

async fn run(config: &ActorConfig) -> Result<()> {
    let apify = ApifyClient::new(&config.apify_api_base, config.apify_token.clone())?;
    let run = apify.get_run(&config.actor_run_id).await?;
    let mut budget = PpeBudget::from_run(&run)?;

    let scrappa_api_key = required_env(
        "SCRAPPA_API_KEY",
        "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.",
    )?;
    let input = apify
        .get_input(&config.key_value_store_id, &config.input_key)
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let plan = build_request_plan(&input).map_err(anyhow::Error::msg)?;
    println!(
        "Searching Trustpilot businesses for {}",
        describe_request(&plan)
    );

    let scrappa = ScrappaClient::new(scrappa_api_key, &config.scrappa_api_base)?;
    let mut responses = Vec::new();
    let mut saved_businesses = 0;
    let mut status_message = None;

    for offset in 0..plan.max_pages {
        let page = plan.start_page + offset;
        let params = page_params(&plan, page);
        println!(
            "Fetching Trustpilot {} page {page}",
            plan.search_type.as_str()
        );

        let response = scrappa.get(plan.endpoint, &params).await?;
        let businesses = get_businesses(&response)
            .iter()
            .map(|business| build_dataset_item(business, plan.search_type, &params, &response))
            .collect::<Vec<_>>();
        responses.push(response);

        if businesses.is_empty() {
            println!("No Trustpilot business results found on page {page}");
            break;
        }

        let item_limit = budget.dataset_item_limit(businesses.len());
        apify
            .push_dataset_items(&config.dataset_id, &businesses[..item_limit])
            .await?;
        if budget.is_pay_per_event && item_limit > 0 && budget.should_charge_business_result() {
            apify
                .charge_event(
                    &config.actor_run_id,
                    BUSINESS_RESULT_CHARGE_EVENT,
                    item_limit,
                )
                .await?;
        }
        let push_result = budget.finish_dataset_push(item_limit, businesses.len());
        saved_businesses += push_result.saved_count;
        println!(
            "Found {} business result(s) on page {page}; saved {}",
            businesses.len(),
            push_result.saved_count
        );
        if push_result.status_message.is_some() {
            status_message = push_result.status_message;
            break;
        }
        if !has_next_page(
            responses.last().expect("the response was just stored"),
            page,
        ) {
            println!("Stopping after page {page}; Scrappa reported no next page");
            break;
        }
    }

    let last_response = responses.last();
    let total_results = first_non_null([
        last_response.and_then(|response| response.pointer("/pagination/totalResults")),
        last_response.and_then(|response| response.pointer("/pageProps/pagination/total_count")),
        last_response.and_then(|response| response.pointer("/pageProps/businessUnits/totalHits")),
    ]);
    let total_pages = first_non_null([
        last_response.and_then(|response| response.pointer("/pagination/totalPages")),
        last_response.and_then(|response| response.pointer("/pageProps/pagination/total_pages")),
        last_response.and_then(|response| response.pointer("/pageProps/businessUnits/totalPages")),
    ]);
    let output = build_output(
        &plan,
        responses,
        saved_businesses,
        status_message.as_deref(),
        total_results,
        total_pages,
    );
    apify
        .set_output(&config.key_value_store_id, &output)
        .await?;

    println!("Trustpilot business search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "search_type": plan.search_type.as_str(),
            "pages_fetched": output["pages_fetched"],
            "businesses_extracted": saved_businesses,
            "total_results": output["total_results"],
            "total_pages": output["total_pages"]
        })
    );
    if let Some(message) = status_message {
        apify
            .set_terminal_status_message(&config.actor_run_id, &message)
            .await?;
    }
    Ok(())
}

fn build_output(
    plan: &RequestPlan,
    responses: Vec<Value>,
    businesses_extracted: usize,
    status_message: Option<&str>,
    total_results: Value,
    total_pages: Value,
) -> Value {
    let mut request = Map::new();
    request.insert(
        "search_type".into(),
        Value::String(plan.search_type.as_str().into()),
    );
    request.insert("endpoint".into(), Value::String(plan.endpoint.into()));
    request.extend(plan.base_params.clone());
    request.insert("start_page".into(), json!(plan.start_page));
    request.insert("max_pages".into(), json!(plan.max_pages));
    json!({
        "request": request,
        "pages_fetched": responses.len(),
        "responses_saved": responses.len(),
        "businesses_extracted": businesses_extracted,
        "status_message": status_message,
        "total_results": total_results,
        "total_pages": total_pages,
        "responses": responses
    })
}

fn first_non_null<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Value {
    values
        .into_iter()
        .flatten()
        .find(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{error}. The Trustpilot business search request exceeded the {}s Scrappa API timeout. Try fewer pages or run the request again.",
            REQUEST_TIMEOUT_MS / 1_000
        )
    } else {
        format!("{error:#}")
    }
}

#[tokio::main]
async fn main() {
    let config = match ActorConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {}", actor_error_message(&error));
            process::exit(1);
        }
    };
    if let Err(error) = run(&config).await {
        let message = actor_error_message(&error);
        eprintln!("Actor failed: {message}");
        if let Ok(apify) = ApifyClient::new(&config.apify_api_base, config.apify_token.clone())
            && let Err(status_error) = apify
                .set_terminal_status_message(&config.actor_run_id, &message)
                .await
        {
            eprintln!("Could not set Actor failure status message: {status_error}");
        }
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_params::{SearchType, build_request_plan};
    use serde_json::json;

    #[test]
    fn output_keeps_request_counts_raw_responses_and_reported_totals() {
        let plan = build_request_plan(&json!({"query":"amazon","max_pages":2})).unwrap();
        let response = json!({
            "businesses":[{"id":"1"}],
            "pagination":{"totalResults":15,"totalPages":2,"has_next_page":true},
            "pageProps":{"pagination":{"total_count":12,"total_pages":4}}
        });
        let output = build_output(&plan, vec![response.clone()], 1, None, json!(15), json!(2));
        assert_eq!(output["request"]["search_type"], "company_search");
        assert_eq!(output["request"]["endpoint"], "/trustpilot/company-search");
        assert_eq!(output["request"]["start_page"], 1);
        assert_eq!(output["request"]["max_pages"], 2);
        assert_eq!(output["pages_fetched"], 1);
        assert_eq!(output["responses_saved"], 1);
        assert_eq!(output["businesses_extracted"], 1);
        assert_eq!(output["status_message"], Value::Null);
        assert_eq!(output["total_results"], 15);
        assert_eq!(output["total_pages"], 2);
        assert_eq!(output["responses"][0], response);
    }

    #[test]
    fn terminal_status_and_summary_fallbacks_are_retained() {
        let plan = RequestPlan {
            search_type: SearchType::Category,
            endpoint: "/trustpilot/businesses",
            base_params: Map::new(),
            start_page: 3,
            max_pages: 1,
        };
        let output = build_output(
            &plan,
            vec![],
            0,
            Some("Charge limit reached"),
            Value::Null,
            Value::Null,
        );
        assert_eq!(output["status_message"], "Charge limit reached");
        assert_eq!(output["responses_saved"], 0);
        assert_eq!(output["request"]["search_type"], "category");
    }

    #[test]
    fn wraps_scrappa_timeout_with_actor_guidance() {
        let error = anyhow::Error::new(ScrappaTimeoutError::new(REQUEST_TIMEOUT_MS));
        assert_eq!(
            actor_error_message(&error),
            "Scrappa API request timed out after 90000ms. The Trustpilot business search request exceeded the 90s Scrappa API timeout. Try fewer pages or run the request again."
        );
    }
}
