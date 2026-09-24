mod apify;
mod request_params;
mod review_processing;
mod scrappa;
#[cfg(test)]
mod test_support;

use anyhow::{Result, anyhow};
use apify::{ApifyClient, ApifyConfig, EventBudget};
use request_params::{RequestPlan, build_request_plan, describe_request, page_params};
use review_processing::{build_page_summary, collect_reviews, enrich_review, reported_total_pages};
use scrappa::{REQUEST_TIMEOUT_MS, ScrappaClient, ScrappaTimeoutError};
use serde_json::{Map, Value, json};
use std::{env, process, time::Duration};

const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const REVIEW_RESULT_EVENT: &str = "review-result";

#[derive(Debug, PartialEq)]
struct ExecutionResult {
    pages_fetched: usize,
    reviews_extracted: usize,
    output_written: bool,
    status_message: Option<String>,
}

fn build_output(plan: &RequestPlan, responses: Vec<Value>, reviews_extracted: usize) -> Value {
    let mut request = Map::new();
    request.insert("targets".into(), plan.targets_value());
    for (key, value) in &plan.base_params {
        request.insert(key.clone(), value.clone());
    }
    request.insert("start_page".into(), json!(plan.start_page));
    request.insert("max_pages".into(), json!(plan.max_pages));
    json!({
        "request": request,
        "pages_fetched": responses.len(),
        "reviews_extracted": reviews_extracted,
        "responses": responses,
    })
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{error}. The Kununu reviews request exceeded the {}s Scrappa API timeout. Try fewer pages or run the request again.",
            REQUEST_TIMEOUT_MS / 1000
        )
    } else {
        format!("{error:#}")
    }
}

async fn execute(
    apify: &ApifyClient,
    scrappa: &ScrappaClient,
    event_budget: &mut EventBudget,
    input: &Value,
) -> Result<ExecutionResult> {
    if input.is_null() {
        return Err(anyhow!("Input is required"));
    }
    let plan = build_request_plan(input).map_err(anyhow::Error::msg)?;
    println!("Fetching Kununu reviews for {}", describe_request(&plan));
    let mut responses = Vec::new();
    let mut reviews_extracted = 0;

    for target in &plan.targets {
        for offset in 0..plan.max_pages {
            let page = plan.start_page + offset;
            let params = page_params(&plan, target, page);
            let chargeable_review_capacity =
                event_budget.max_event_charge_count(REVIEW_RESULT_EVENT);
            if chargeable_review_capacity == 0 {
                let status_message = format!(
                    "Charge limit reached before fetching Kununu reviews page {page} for {}/{}.",
                    target.country, target.company_slug
                );
                println!(
                    "{status_message} {}",
                    json!({
                        "event": REVIEW_RESULT_EVENT,
                        "saved_count": reviews_extracted,
                        "target": format!("{}/{}", target.country, target.company_slug),
                        "page": page,
                    })
                );
                return Ok(ExecutionResult {
                    pages_fetched: responses.len(),
                    reviews_extracted,
                    output_written: false,
                    status_message: Some(status_message),
                });
            }

            println!(
                "Fetching Kununu reviews page {page} for {}/{}",
                target.country, target.company_slug
            );
            let response = scrappa.get(&params).await?;
            let reviews = collect_reviews(&response);
            responses.push(build_page_summary(
                target,
                page,
                reviews.len(),
                &response,
                plan.include_raw_responses,
            ));

            if reviews.is_empty() {
                println!("No reviews found on page {page}");
            } else {
                let enriched_reviews = reviews
                    .iter()
                    .map(|review| {
                        enrich_review(review, target, &params, &response, plan.include_raw_review)
                    })
                    .collect::<Result<Vec<_>>>()?;
                let saved_count = if event_budget.is_pay_per_event() {
                    let charge_result = apify
                        .push_charged_dataset_items(event_budget, &enriched_reviews)
                        .await?;
                    let saved_count = charge_result.charged_count.min(enriched_reviews.len());
                    if charge_result.event_charge_limit_reached {
                        let status_message = format!(
                            "Charge limit reached after saving {saved_count} of {} Kununu reviews for {}/{} page {page}.",
                            enriched_reviews.len(),
                            target.country,
                            target.company_slug
                        );
                        println!(
                            "{status_message} {}",
                            json!({
                                "event": REVIEW_RESULT_EVENT,
                                "saved_count": saved_count,
                                "requested_count": enriched_reviews.len(),
                                "target": format!("{}/{}", target.country, target.company_slug),
                                "page": page,
                            })
                        );
                        return Ok(ExecutionResult {
                            pages_fetched: responses.len(),
                            reviews_extracted,
                            output_written: false,
                            status_message: Some(status_message),
                        });
                    }
                    saved_count
                } else {
                    apify.push_dataset_items(&enriched_reviews).await?;
                    enriched_reviews.len()
                };
                reviews_extracted += saved_count;
                println!("Found {} reviews on page {page}", reviews.len());
            }

            if reported_total_pages(&response)
                .is_some_and(|total_pages| f64::from(page) >= total_pages)
            {
                println!(
                    "Stopping after page {page}; Scrappa reported {} total page(s)",
                    response["meta"]["pagination"]["totalPages"]
                );
                break;
            }
        }
    }

    let output = build_output(&plan, responses, reviews_extracted);
    apify.set_output(&output).await?;
    let summary = json!({
        "targets": plan.targets.iter().map(|target| format!("{}/{}", target.country, target.company_slug)).collect::<Vec<_>>(),
        "pages_fetched": output["pages_fetched"],
        "reviews_extracted": reviews_extracted,
    });
    println!("Kununu reviews extraction completed successfully");
    println!("Results summary: {summary}");
    Ok(ExecutionResult {
        pages_fetched: output["pages_fetched"].as_u64().unwrap_or(0) as usize,
        reviews_extracted,
        output_written: true,
        status_message: None,
    })
}

fn scrappa_api_key() -> Result<String> {
    env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })
}

async fn run() -> Result<()> {
    let apify = ApifyClient::new(ApifyConfig::from_env()?)?;
    // Actor.init() initializes the charging manager before validating the input or Scrappa key.
    let mut event_budget = apify.get_event_budget().await?;
    let api_key = scrappa_api_key()?;
    let scrappa_api_base =
        env::var("SCRAPPA_API_BASE_URL").unwrap_or_else(|_| SCRAPPA_API_DEFAULT.to_owned());
    let scrappa = ScrappaClient::new(api_key, scrappa_api_base)?;
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let result = execute(&apify, &scrappa, &mut event_budget, &input).await?;
    if let Some(status_message) = result.status_message {
        eprintln!("[Status message]: {status_message}");
        if let Err(error) = tokio::time::timeout(
            Duration::from_secs(1),
            apify.set_status_message(&status_message),
        )
        .await
        .unwrap_or_else(|_| Err(anyhow!("Setting status message timed out after 1s")))
        {
            eprintln!("Warning: {error:#}");
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let message = actor_error_message(&error);
        eprintln!("Actor failed: {message}");
        if let Ok(config) = ApifyConfig::from_env()
            && let Ok(apify) = ApifyClient::new(config)
        {
            let _ =
                tokio::time::timeout(Duration::from_secs(1), apify.set_status_message(&message))
                    .await;
        }
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{ExecutionResult, actor_error_message, build_output, execute};
    use crate::{
        apify::{ApifyClient, ApifyConfig, EventBudget},
        request_params::build_request_plan,
        scrappa::{ScrappaClient, ScrappaTimeoutError},
        test_support::*,
    };
    use serde_json::{Value, json};

    fn apify_config(base_url: String) -> ApifyConfig {
        ApifyConfig {
            base_url,
            token: "test-token".into(),
            actor_run_id: "test-run".into(),
            key_value_store_id: "test-store".into(),
            dataset_id: "test-dataset".into(),
            input_key: "INPUT".into(),
        }
    }

    fn ppe_budget(max_total_charge_usd: f64, event_price: f64) -> EventBudget {
        EventBudget::from_run(&json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "review-result": {"eventPriceUsd": event_price}
                    }}
                },
                "options": {"maxTotalChargeUsd": max_total_charge_usd},
                "chargedEventCounts": {}
            }
        }))
        .unwrap()
    }

    #[test]
    fn output_preserves_the_input_request_and_page_summaries() {
        let plan = build_request_plan(&json!({
            "targets": ["de/bmwgroup"],
            "sort": "newest",
            "max_pages": 2
        }))
        .unwrap();
        let output = build_output(
            &plan,
            vec![json!({
                "target":"de/bmwgroup",
                "page":1,
                "count":1,
                "pagination":{"totalPages":1}
            })],
            1,
        );
        assert_eq!(output["request"]["targets"][0]["company_slug"], "bmwgroup");
        assert_eq!(output["request"]["sort"], "newest");
        assert_eq!(output["request"]["start_page"], 1);
        assert_eq!(output["request"]["max_pages"], 2);
        assert_eq!(output["pages_fetched"], 1);
        assert_eq!(output["reviews_extracted"], 1);
    }

    #[tokio::test]
    async fn full_ppe_run_preserves_dataset_charge_and_output_order() {
        let apify_server = MockServer::start(vec![
            response(200, json!({})),
            response(201, json!({})),
            response(200, json!({})),
        ]);
        let scrappa_server = MockServer::start(vec![response(
            200,
            json!({
                "success": true,
                "data": [{
                    "uuid":"review-1",
                    "score":4.2,
                    "texts":[{"text":"Nice team"}],
                    "company":{"name":"Example GmbH"}
                }],
                "meta":{"pagination":{"totalPages":1,"totalResults":1}}
            }),
        )]);
        let apify = ApifyClient::new(apify_config(apify_server.base_url.clone())).unwrap();
        let scrappa =
            ScrappaClient::new("test-api-key".into(), scrappa_server.base_url.clone()).unwrap();
        let mut budget = ppe_budget(1.0, 0.25);
        let result = execute(
            &apify,
            &scrappa,
            &mut budget,
            &json!({"targets":["de/example-gmbh"], "sort":"newest", "include_raw_responses":false}),
        )
        .await
        .unwrap();
        assert_eq!(
            result,
            ExecutionResult {
                pages_fetched: 1,
                reviews_extracted: 1,
                output_written: true,
                status_message: None,
            }
        );

        let upstream_requests = scrappa_server.requests();
        assert_eq!(upstream_requests.len(), 1);
        assert_eq!(
            header(&upstream_requests[0], "X-API-Key").as_deref(),
            Some("test-api-key")
        );
        let upstream_path = request_parts(&upstream_requests[0]).1;
        assert!(upstream_path.starts_with("/kununu/reviews?"));
        let api_requests = apify_server.requests();
        assert_eq!(api_requests.len(), 3);
        assert_eq!(
            request_parts(&api_requests[0]).1,
            "/v2/datasets/test-dataset/items"
        );
        let item: Value = serde_json::from_str(request_parts(&api_requests[0]).2).unwrap();
        assert_eq!(item[0]["review_id"], "review-1");
        assert_eq!(item[0]["company_target"], "de/example-gmbh");
        assert_eq!(item[0]["request_sort"], "newest");
        assert_eq!(
            request_parts(&api_requests[1]).1,
            "/v2/actor-runs/test-run/charge"
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&api_requests[1]).2).unwrap(),
            json!({"eventName":"review-result","count":1})
        );
        assert_eq!(
            request_parts(&api_requests[2]).1,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
        let output: Value = serde_json::from_str(request_parts(&api_requests[2]).2).unwrap();
        assert_eq!(output["pages_fetched"], 1);
        assert_eq!(output["reviews_extracted"], 1);
        assert_eq!(output["responses"][0]["pagination"]["totalPages"], 1);
        assert!(output["responses"][0].get("response").is_none());
    }

    #[tokio::test]
    async fn zero_event_capacity_exits_before_fetching_and_skips_output() {
        let apify_server = MockServer::start(vec![]);
        let scrappa_server = MockServer::start(vec![]);
        let apify = ApifyClient::new(apify_config(apify_server.base_url.clone())).unwrap();
        let scrappa =
            ScrappaClient::new("test-api-key".into(), scrappa_server.base_url.clone()).unwrap();
        let mut budget = ppe_budget(0.1, 0.25);
        let result = execute(
            &apify,
            &scrappa,
            &mut budget,
            &json!({"targets":["de/bmwgroup"]}),
        )
        .await
        .unwrap();
        assert_eq!(result.pages_fetched, 0);
        assert_eq!(result.reviews_extracted, 0);
        assert!(!result.output_written);
        assert_eq!(
            result.status_message.as_deref(),
            Some("Charge limit reached before fetching Kununu reviews page 1 for de/bmwgroup.")
        );
        assert!(apify_server.requests().is_empty());
        assert!(scrappa_server.requests().is_empty());
    }

    #[tokio::test]
    async fn empty_upstream_page_still_writes_kv_output_without_dataset_or_charge() {
        let apify_server = MockServer::start(vec![response(200, json!({}))]);
        let scrappa_server = MockServer::start(vec![response(
            200,
            json!({"success":true,"data":[],"meta":{"pagination":{"totalPages":1}}}),
        )]);
        let apify = ApifyClient::new(apify_config(apify_server.base_url.clone())).unwrap();
        let scrappa =
            ScrappaClient::new("test-api-key".into(), scrappa_server.base_url.clone()).unwrap();
        let mut budget = ppe_budget(1.0, 0.25);
        let result = execute(
            &apify,
            &scrappa,
            &mut budget,
            &json!({"targets":["de/bmwgroup"]}),
        )
        .await
        .unwrap();
        assert!(result.output_written);
        assert_eq!(result.pages_fetched, 1);
        assert_eq!(result.reviews_extracted, 0);
        let apify_requests = apify_server.requests();
        assert_eq!(apify_requests.len(), 1);
        assert_eq!(
            request_parts(&apify_requests[0]).1,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
    }

    #[test]
    fn timeout_error_keeps_the_existing_actor_guidance() {
        let error = anyhow::Error::new(ScrappaTimeoutError);
        assert_eq!(
            actor_error_message(&error),
            "Scrappa API request timed out after 90000ms. The Kununu reviews request exceeded the 90s Scrappa API timeout. Try fewer pages or run the request again."
        );
    }
}
