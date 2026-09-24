mod apify;
mod request_params;
mod response_utils;
mod scrappa;

use anyhow::{anyhow, Result};
use apify::{ApifyClient, ChargeBudget, PushResult};
use request_params::{build_request_plan, company_details_params, describe_request};
use response_utils::{build_dataset_item, build_output_summary};
use serde_json::{json, Value};
use std::{env, process};

const ENDPOINT: &str = "/trustpilot/company-details";

fn required_api_key() -> Result<String> {
    env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })
}

fn is_pay_per_event(run: &Value) -> bool {
    run.pointer("/data/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        == Some("PAY_PER_EVENT")
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error
        .downcast_ref::<scrappa::ScrappaTimeoutError>()
        .is_some()
    {
        format!(
            "{error}. The Trustpilot company details request exceeded the {}s Scrappa API timeout. Try fewer domains or run the request again.",
            scrappa::REQUEST_TIMEOUT_MS / 1_000
        )
    } else {
        format!("{error:#}")
    }
}

async fn run_actor() -> Result<()> {
    let api_key = required_api_key()?;
    let apify = ApifyClient::from_env()?;
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let plan = build_request_plan(&input).map_err(anyhow::Error::msg)?;

    let run_pricing = apify.get_run_pricing().await?;
    let mut charge_budget = if is_pay_per_event(&run_pricing) {
        Some(ChargeBudget::from_run(&run_pricing)?)
    } else {
        None
    };

    println!(
        "Fetching Trustpilot company details for {}",
        describe_request(&plan)
    );
    let scrappa_base_url = env::var("SCRAPPA_API_BASE_URL").ok();
    let scrappa = scrappa::ScrappaClient::new(api_key, scrappa_base_url)?;
    let mut failures = Vec::new();
    let mut saved_companies = 0;
    let mut status_message = None;

    for (index, company_domain) in plan.domains.iter().enumerate() {
        let params = company_details_params(&plan, company_domain);
        println!("Fetching Trustpilot company details for {company_domain}");

        let result = async {
            let response = scrappa
                .get(ENDPOINT, &params, scrappa::MAX_ATTEMPTS)
                .await?;
            let item = build_dataset_item(&response, company_domain, &params);
            match charge_budget.as_mut() {
                Some(budget) => apify.push_charged_item(&item, budget, index).await,
                None => {
                    apify.push_dataset_item(&item).await?;
                    Ok(PushResult {
                        saved_count: 1,
                        status_message: None,
                    })
                }
            }
        }
        .await;

        match result {
            Ok(push_result) => {
                saved_companies += push_result.saved_count;
                println!(
                    "Saved {} Trustpilot company detail result(s) for {company_domain}",
                    push_result.saved_count
                );
                if push_result.status_message.is_some() {
                    status_message = push_result.status_message;
                    break;
                }
            }
            Err(error) => {
                let message = actor_error_message(&error);
                failures.push(json!({
                    "company_domain": company_domain,
                    "error": message,
                }));
                eprintln!(
                    "Failed to fetch Trustpilot company details for {company_domain}: {message}"
                );
            }
        }
    }

    if status_message.is_none() && !failures.is_empty() {
        status_message = Some(format!(
            "{} of {} Trustpilot company detail request(s) failed.",
            failures.len(),
            plan.domains.len()
        ));
    }

    let output = build_output_summary(
        &plan.domains,
        &plan.base_params,
        saved_companies,
        &failures,
        status_message.as_deref(),
    );
    apify.set_output(&output).await?;
    println!("Trustpilot company details extraction completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "companies_requested": plan.domains.len(),
            "companies_saved": saved_companies,
            "companies_failed": failures.len(),
        })
    );

    if saved_companies == 0 && !failures.is_empty() {
        let message = status_message
            .clone()
            .unwrap_or_else(|| "No Trustpilot company details were saved.".into());
        return Err(anyhow!(message));
    }

    if let Some(status_message) = status_message {
        apify.set_status_message(&status_message).await?;
    }
    Ok(())
}

async fn set_failure_status(error: &anyhow::Error) {
    if let Ok(apify) = ApifyClient::from_env() {
        let message = actor_error_message(error);
        let _ = apify.set_status_message(&message).await;
    }
}

#[tokio::main]
async fn main() {
    if let Err(error) = run_actor().await {
        let message = actor_error_message(&error);
        eprintln!("Actor failed: {message}");
        set_failure_status(&error).await;
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_pay_per_event_pricing_without_changing_legacy_pricing() {
        assert!(is_pay_per_event(&json!({
            "data":{"pricingInfo":{"pricingModel":"PAY_PER_EVENT"}}
        })));
        assert!(!is_pay_per_event(&json!({
            "data":{"pricingInfo":{"pricingModel":"PRICE_PER_DATASET_ITEM"}}
        })));
    }

    #[test]
    fn formats_timeout_errors_with_the_existing_actor_guidance() {
        let message = actor_error_message(&anyhow::Error::new(scrappa::ScrappaTimeoutError));
        assert!(message.contains("timed out after 90000ms"));
        assert!(message.contains("Try fewer domains or run the request again."));
    }
}
