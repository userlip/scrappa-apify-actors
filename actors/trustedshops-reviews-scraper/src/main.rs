use anyhow::{Result, anyhow};
use serde_json::{Map, Value, json};
use std::{env, process};

use trustedshops_reviews_scraper::{
    apify::ApifyClient,
    request_params::{
        RequestPlan, TrustedShopsTarget, build_request_plan, describe_request, page_params,
    },
    review_processing::{collect_reviews, enrich_review, has_next_page},
    scrappa::{REQUEST_TIMEOUT_MS, ScrappaClient, ScrappaError},
};

const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const APIFY_API_PUBLIC_BASE_URL: &str = "https://api.apify.com";
const REVIEW_RESULT_CHARGE_EVENT: &str = "review-result";

struct ActorConfig {
    scrappa_api_key: String,
    scrappa_api_base_url: String,
    apify_api_base_url: String,
    store_id: String,
    input_key: String,
    dataset_id: String,
    run_id: String,
    apify_token: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let api_key = env::var("SCRAPPA_API_KEY").ok();
        let scrappa_api_key = api_key
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."))?;
        Ok(Self {
            scrappa_api_key,
            scrappa_api_base_url: env::var("SCRAPPA_API_BASE_URL")
                .unwrap_or_else(|_| SCRAPPA_API_BASE_URL.into()),
            apify_api_base_url: env::var("APIFY_API_PUBLIC_BASE_URL")
                .unwrap_or_else(|_| APIFY_API_PUBLIC_BASE_URL.into()),
            store_id: required_env(
                "ACTOR_DEFAULT_KEY_VALUE_STORE_ID",
                "ACTOR_DEFAULT_KEY_VALUE_STORE_ID environment variable is not set",
            )?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".into()),
            dataset_id: required_env(
                "ACTOR_DEFAULT_DATASET_ID",
                "ACTOR_DEFAULT_DATASET_ID environment variable is not set",
            )?,
            run_id: required_env(
                "ACTOR_RUN_ID",
                "ACTOR_RUN_ID environment variable is not set",
            )?,
            apify_token: required_env(
                "APIFY_TOKEN",
                "APIFY_TOKEN environment variable is not set",
            )?,
        })
    }

    fn apify_client(&self) -> Result<ApifyClient> {
        ApifyClient::new(
            &self.apify_api_base_url,
            self.apify_token.clone(),
            self.store_id.clone(),
            self.input_key.clone(),
            self.dataset_id.clone(),
            self.run_id.clone(),
        )
    }
}

fn required_env(name: &str, missing_message: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!(missing_message.to_owned()))
}

fn build_output(plan: &RequestPlan, responses: Vec<Value>, reviews_extracted: usize) -> Value {
    let targets = plan.targets.iter().map(target_value).collect::<Vec<_>>();
    let mut request = plan.base_params.clone();
    request.insert("targets".into(), json!(targets));
    request.insert("start_page".into(), json!(plan.start_page));
    request.insert("max_pages".into(), json!(plan.max_pages));
    json!({
        "request": request,
        "pages_fetched": responses.len(),
        "reviews_extracted": reviews_extracted,
        "responses": responses,
    })
}

fn target_value(target: &TrustedShopsTarget) -> Value {
    json!({
        "tsid": target.tsid,
        "input": target.input,
        "sourceUrl": target.source_url,
    })
}

fn page_output(
    target: &TrustedShopsTarget,
    page: u64,
    reviews_count: usize,
    response: &Value,
    include_raw: bool,
) -> Value {
    let mut page = Map::from_iter([
        ("tsid".into(), json!(target.tsid)),
        ("page".into(), json!(page)),
        ("count".into(), json!(reviews_count)),
    ]);
    if include_raw {
        page.insert("response".into(), response.clone());
    }
    Value::Object(page)
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error
        .downcast_ref::<ScrappaError>()
        .is_some_and(ScrappaError::is_timeout)
    {
        return format!(
            "{error}. The Trusted Shops reviews request exceeded the {}s Scrappa API timeout. Try fewer pages, fewer TSIDs, or run the request again.",
            REQUEST_TIMEOUT_MS / 1000
        );
    }
    format!("{error:#}")
}

async fn fetch_page(
    scrappa: &ScrappaClient,
    target: &TrustedShopsTarget,
    params: &Map<String, Value>,
) -> Result<Value> {
    scrappa
        .get(&format!("/trustedshops/reviews/{}", target.tsid), params)
        .await
        .map_err(anyhow::Error::from)
}

async fn run_actor(config: &ActorConfig, apify: &ApifyClient) -> Result<()> {
    let mut pricing = apify.get_run_pricing().await?;
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let plan = build_request_plan(&input).map_err(anyhow::Error::msg)?;
    println!(
        "Fetching Trusted Shops reviews for {}",
        describe_request(&plan)
    );

    let scrappa = ScrappaClient::new(
        config.scrappa_api_key.clone(),
        config.scrappa_api_base_url.clone(),
    )?;
    let mut responses = Vec::new();
    let mut reviews_extracted = 0;
    for target in &plan.targets {
        for offset in 0..plan.max_pages {
            let page = plan.start_page + offset;
            let params = page_params(&plan, page);
            if pricing.is_pay_per_event
                && pricing
                    .chargeable_event_count(REVIEW_RESULT_CHARGE_EVENT)
                    .is_some_and(|capacity| capacity == 0)
            {
                let status_message = format!(
                    "Charge limit reached before fetching Trusted Shops reviews page {page} for {}.",
                    target.tsid
                );
                println!("{status_message}");
                apify.set_terminal_status_message(&status_message).await?;
                return Ok(());
            }

            println!(
                "Fetching Trusted Shops reviews page {page} for {}",
                target.tsid
            );
            let response = fetch_page(&scrappa, target, &params).await?;
            let reviews = collect_reviews(&response);
            responses.push(page_output(
                target,
                page,
                reviews.len(),
                &response,
                plan.include_raw_responses,
            ));

            if reviews.is_empty() {
                println!(
                    "No Trusted Shops reviews found on page {page} for {}",
                    target.tsid
                );
                break;
            }
            let enriched_reviews = reviews
                .iter()
                .map(|review| enrich_review(review, target, &params, &response, false))
                .collect::<Vec<_>>();
            let requested_count = enriched_reviews.len();
            let saved_count = if pricing.is_pay_per_event {
                let count = pricing.dataset_push_limit(requested_count, REVIEW_RESULT_CHARGE_EVENT);
                let rows = &enriched_reviews[..count];
                apify.push_dataset_items(rows).await?;
                if pricing.has_event_price(REVIEW_RESULT_CHARGE_EVENT) {
                    let idempotency_key = format!("{}-{}-{page}", config.run_id, target.tsid);
                    apify
                        .charge_event(REVIEW_RESULT_CHARGE_EVENT, rows.len(), &idempotency_key)
                        .await?;
                }
                let charge_result =
                    pricing.record_dataset_push(REVIEW_RESULT_CHARGE_EVENT, rows.len());
                let saved_count = charge_result.charged_count.min(requested_count);
                if charge_result.event_charge_limit_reached {
                    let status_message = format!(
                        "Charge limit reached after saving {saved_count} of {requested_count} Trusted Shops reviews for {} page {page}.",
                        target.tsid
                    );
                    println!("{status_message}");
                    println!(
                        "{}",
                        json!({
                            "event": REVIEW_RESULT_CHARGE_EVENT,
                            "saved_count": saved_count,
                            "requested_count": requested_count,
                            "tsid": target.tsid,
                            "page": page,
                        })
                    );
                    apify.set_terminal_status_message(&status_message).await?;
                    return Ok(());
                }
                saved_count
            } else {
                apify.push_dataset_items(&enriched_reviews).await?;
                requested_count
            };
            reviews_extracted += saved_count;
            println!(
                "Found {} Trusted Shops review(s) on page {page}; saved {saved_count}",
                reviews.len()
            );

            if has_next_page(&response, page) == Some(false) {
                println!("Stopping after page {page}; Scrappa reported no next page");
                break;
            }
        }
    }

    let output = build_output(&plan, responses, reviews_extracted);
    apify.set_output(&output).await?;
    println!("Trusted Shops reviews extraction completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "targets": plan.targets.iter().map(|target| target.tsid.as_str()).collect::<Vec<_>>(),
            "pages_fetched": output["pages_fetched"],
            "reviews_extracted": reviews_extracted,
        })
    );
    Ok(())
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify = config.apify_client()?;
    if let Err(error) = run_actor(&config, &apify).await {
        let message = actor_error_message(&error);
        if let Err(status_error) = apify.set_terminal_status_message(&message).await {
            eprintln!("Failed to set Actor run failure status: {status_error:#}");
        }
        return Err(anyhow!(message));
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let message = error.to_string();
        if !message.starts_with("Actor failed:") {
            eprintln!("Actor failed: {message}");
        }
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trustedshops_reviews_scraper::request_params::build_request_plan;

    const TSID: &str = "XFB15FFBDE1DEE7A55D292A7D48598A6A";

    #[test]
    fn output_matches_node_actor_request_and_response_shape() {
        let plan =
            build_request_plan(&json!({ "tsids": [TSID], "size": 20, "max_pages": 2 })).unwrap();
        let response = page_output(
            &plan.targets[0],
            1,
            1,
            &json!({"reviews":[{"id":"review-1"}]}),
            true,
        );
        let output = build_output(&plan, vec![response], 1);
        assert_eq!(
            output["request"]["targets"][0]["sourceUrl"],
            format!("https://www.trustedshops.de/bewertung/info_{TSID}.html")
        );
        assert_eq!(output["request"]["start_page"], 1);
        assert_eq!(output["request"]["max_pages"], 2);
        assert_eq!(output["pages_fetched"], 1);
        assert_eq!(output["reviews_extracted"], 1);
        assert_eq!(output["responses"][0]["count"], 1);
        assert_eq!(
            output["responses"][0]["response"]["reviews"][0]["id"],
            "review-1"
        );
    }
}
