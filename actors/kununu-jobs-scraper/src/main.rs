mod apify;
mod params;
mod response;
mod scrappa;

use anyhow::{anyhow, Context, Result};
use apify::{ApifyClient, RESULT_CHARGE_EVENT};
use params::{build_search_plan, describe_request, SearchPlan};
use response::{
    company_name, formatted_location, get_jobs, get_pagination, last_page, to_dataset_job,
};
use scrappa::{
    ScrappaClient, ScrappaTimeoutError, DEFAULT_BASE_URL as SCRAPPA_API_BASE_URL,
    REQUEST_TIMEOUT_MS,
};
use serde_json::{json, Map, Value};
use std::{env, process};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";

fn required_env(name: &str, missing_message: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("{missing_message}"))
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn build_output(
    plan: &SearchPlan,
    pages: Vec<Value>,
    jobs_extracted: usize,
    jobs_saved: usize,
) -> Value {
    let mut request = plan.params.clone();
    request.insert("start_page".into(), json!(plan.start_page));
    request.insert("max_pages".into(), json!(plan.max_pages));
    json!({
        "request": request,
        "pages_fetched": pages.len(),
        "jobs_extracted": jobs_extracted,
        "jobs_saved": jobs_saved,
        "pages": pages
    })
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{error}. The Kununu Jobs request exceeded the {}s Scrappa API timeout. Try again or refine the query.",
            REQUEST_TIMEOUT_MS / 1000
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
    let apify = ApifyClient::new(
        &env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL),
        required_env("APIFY_TOKEN", "APIFY_TOKEN environment variable is not set")?,
        required_env(
            "ACTOR_RUN_ID",
            "ACTOR_RUN_ID environment variable is not set",
        )?,
        required_env(
            "ACTOR_DEFAULT_KEY_VALUE_STORE_ID",
            "ACTOR_DEFAULT_KEY_VALUE_STORE_ID environment variable is not set",
        )?,
        required_env(
            "ACTOR_DEFAULT_DATASET_ID",
            "ACTOR_DEFAULT_DATASET_ID environment variable is not set",
        )?,
        env_or_default("ACTOR_INPUT_KEY", "INPUT"),
    )?;

    let result = run_actor(&apify, api_key).await;
    if let Err(error) = result {
        let message = actor_error_message(&error);
        if let Err(status_error) = apify.set_terminal_status_message(&message).await {
            eprintln!("Could not set terminal Actor status message: {status_error:#}");
        }
        return Err(anyhow!(message));
    }
    Ok(())
}

async fn run_actor(apify: &ApifyClient, api_key: String) -> Result<()> {
    let mut charging = apify.charging_manager().await?;
    let input = apify.get_input().await?;
    let plan = build_search_plan(input.as_ref()).map_err(anyhow::Error::msg)?;
    println!(
        "Searching Kununu Jobs for: {}",
        describe_request(&plan.params)
    );

    let scrappa = ScrappaClient::new(
        api_key,
        &env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL),
    )
    .context("Could not create Scrappa API client")?;
    let mut pages = Vec::new();
    let mut jobs_extracted = 0;
    let mut jobs_saved = 0;
    let mut first_job_summary = None;

    for offset in 0..plan.max_pages {
        let page = plan.start_page + offset as i64;
        let mut params = plan.params.clone();
        params.insert("page".into(), json!(page));
        println!("Fetching Kununu Jobs page {page}");

        let response = scrappa.get("/kununu/jobs", &params).await?;
        let jobs = get_jobs(&response).map_err(anyhow::Error::msg)?;
        let pagination = get_pagination(&response);
        let dataset_jobs = jobs
            .iter()
            .map(|job| to_dataset_job(job, plan.include_raw_job))
            .collect::<Vec<_>>();

        pages.push(page_summary(page, jobs.len(), pagination.as_ref()));
        jobs_extracted += jobs.len();

        if first_job_summary.is_none() {
            if let Some(job) = jobs.first() {
                first_job_summary = Some(json!({
                    "title": job.get("title").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
                    "company": company_name(job.get("company"), job),
                    "location": formatted_location(job.get("location"), job)
                }));
            }
        }

        if !dataset_jobs.is_empty() {
            let charge_result = apify.push_data(&dataset_jobs, &mut charging, page).await?;
            let page_saved_jobs =
                if charge_result.charged_count > 0 || charge_result.event_charge_limit_reached {
                    charge_result.saved_count.min(dataset_jobs.len())
                } else {
                    dataset_jobs.len()
                };
            jobs_saved += page_saved_jobs;

            if charge_result.charged_count == 0 && !charge_result.event_charge_limit_reached {
                eprintln!(
                    "Saved {} Kununu job result(s) on page {page}, but no {RESULT_CHARGE_EVENT} events were charged. Check actor pricing if this was a paid run.",
                    dataset_jobs.len()
                );
            }

            if charge_result.event_charge_limit_reached {
                let status_message = if page_saved_jobs < dataset_jobs.len() {
                    format!("Charge limit reached after saving {page_saved_jobs} of {} Kununu job result(s) on page {page}.", dataset_jobs.len())
                } else {
                    format!("Charge limit reached after saving all {page_saved_jobs} Kununu job result(s) on page {page}.")
                };
                println!(
                    "{status_message} {}",
                    json!({
                        "event": RESULT_CHARGE_EVENT,
                        "charged_count": charge_result.charged_count,
                        "requested_count": dataset_jobs.len(),
                        "page": page
                    })
                );
                if let Err(error) = apify.set_terminal_status_message(&status_message).await {
                    eprintln!("Could not set terminal Actor status message: {error:#}");
                }
                return Ok(());
            }

            println!(
                "Found {} Kununu job result(s) on page {page}",
                dataset_jobs.len()
            );
        } else {
            println!("No Kununu job results found on page {page}");
            break;
        }

        if let Some(last_page) = last_page(pagination.as_ref()) {
            if page >= last_page {
                println!("Stopping after page {page}; Scrappa reported {last_page} total page(s)");
                break;
            }
        }
    }

    let output = build_output(&plan, pages, jobs_extracted, jobs_saved);
    apify.set_output(&output).await?;
    println!("Kununu Jobs search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "jobs": jobs_extracted,
            "saved_jobs": jobs_saved,
            "pages_fetched": output["pages_fetched"],
            "start_page": plan.start_page,
            "first_job": first_job_summary
        })
    );
    Ok(())
}

fn page_summary(page: i64, count: usize, pagination: Option<&Value>) -> Value {
    let mut summary = Map::new();
    summary.insert("page".into(), json!(page));
    summary.insert("count".into(), json!(count));
    if let Some(pagination) = pagination {
        summary.insert("pagination".into(), pagination.clone());
    }
    Value::Object(summary)
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
    use super::{build_output, page_summary};
    use crate::params::build_search_plan;
    use serde_json::json;

    #[test]
    fn output_preserves_request_and_page_counts() {
        let plan = build_search_plan(Some(
            &json!({"query":"Data Analyst","page":2,"max_pages":3}),
        ))
        .unwrap();
        let output = build_output(
            &plan,
            vec![page_summary(2, 1, Some(&json!({"last_page":3})))],
            1,
            1,
        );
        assert_eq!(output["request"]["page"], 2);
        assert_eq!(output["request"]["start_page"], 2);
        assert_eq!(output["request"]["max_pages"], 3);
        assert_eq!(output["pages_fetched"], 1);
        assert_eq!(output["jobs_extracted"], 1);
        assert_eq!(output["jobs_saved"], 1);
        assert_eq!(output["pages"][0]["pagination"]["last_page"], 3);
    }

    #[test]
    fn page_summary_omits_missing_pagination() {
        assert_eq!(page_summary(1, 0, None), json!({"page":1,"count":0}));
    }
}
