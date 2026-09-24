mod apify;
mod config;
mod input;
mod scrappa;

use std::process::ExitCode;

use anyhow::{anyhow, Context, Result};
use apify::{ApifyClient, DatasetBudget, APIFY_REQUEST_TIMEOUT};
use config::Config;
use input::{get_business_id_requests, BusinessIdRequest};
use reqwest::Client;
use scrappa::{ScrappaApiError, ScrappaClient, REQUEST_TIMEOUT};
use serde_json::{json, Map, Value};

fn business_detail_dataset_items(
    response: &Value,
    request: &BusinessIdRequest,
) -> Result<Option<Vec<Value>>> {
    let Some(details) = response.get("data").and_then(Value::as_array) else {
        return Ok(None);
    };
    if details.is_empty() {
        return Ok(None);
    }

    let items = details
        .iter()
        .map(|detail| {
            let mut item: Map<String, Value> = detail
                .as_object()
                .cloned()
                .ok_or_else(|| anyhow!("Scrappa business details item was not an object"))?;
            let business_id = item
                .get("business_id")
                .filter(|business_id| !business_id.is_null())
                .cloned()
                .unwrap_or_else(|| json!(request.business_id));
            item.insert(
                "input_business_id".to_owned(),
                json!(request.input_business_id),
            );
            item.insert("business_id".to_owned(), business_id);
            Ok(Value::Object(item))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Some(items))
}

fn not_found_dataset_item(request: &BusinessIdRequest) -> Value {
    json!({
        "success": false,
        "input_business_id": request.input_business_id,
        "business_id": request.business_id,
        "error": "Business not found"
    })
}

fn no_details_dataset_item(request: &BusinessIdRequest) -> Value {
    json!({
        "success": false,
        "input_business_id": request.input_business_id,
        "business_id": request.business_id,
        "error": "No business details found"
    })
}

fn build_batch_output(
    requested: usize,
    succeeded: usize,
    failed: usize,
    results: Vec<Value>,
) -> Value {
    json!({
        "requested": requested,
        "succeeded": succeeded,
        "failed": failed,
        "results": results
    })
}

fn build_output(
    requested: usize,
    first_output: Option<Value>,
    succeeded: usize,
    failed: usize,
    results: Vec<Value>,
) -> Result<Value> {
    if requested == 1 {
        return first_output.context("No business details response was received");
    }

    Ok(build_batch_output(requested, succeeded, failed, results))
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let apify_http = Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .context("Failed to create Apify HTTP client")?;
    let scrappa_http = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("Failed to create Scrappa HTTP client")?;
    let apify = ApifyClient::new(&apify_http, &config);
    let input = apify.get_input().await?.filter(|input| !input.is_null());
    let requests = get_business_id_requests(input.as_ref())?;
    if requests.is_empty() {
        return Err(anyhow!(
            "At least one Business ID is required. Provide business_ids or legacy business_id."
        ));
    }

    let input_value = input.as_ref().unwrap_or(&Value::Null);
    let scrappa = ScrappaClient::new(
        scrappa_http,
        config.scrappa_api_base.clone(),
        config.scrappa_api_key.clone(),
        REQUEST_TIMEOUT,
    );
    let mut results = Vec::new();
    let mut first_output = None;
    let mut succeeded = 0;
    let mut failed = 0;
    let mut dataset_budget = DatasetBudget::default();

    println!(
        "Fetching Google Maps business details for {} business{}",
        requests.len(),
        if requests.len() == 1 { "" } else { "es" }
    );

    for request in &requests {
        println!("Fetching details for business ID: {}", request.business_id);
        let response = match scrappa.get_business_details(request, input_value).await {
            Ok(response) => response,
            Err(error) if is_business_not_found(&error) => {
                println!("Business not found (404): {}", request.business_id);
                let item = not_found_dataset_item(request);
                push_result(&apify, &mut dataset_budget, &[item]).await?;
                results.push(json!({
                    "input_business_id": request.input_business_id,
                    "business_id": request.business_id,
                    "found": false,
                    "error": "Business not found"
                }));
                first_output
                    .get_or_insert_with(|| json!({"data": [], "error": "Business not found"}));
                failed += 1;
                continue;
            }
            Err(error) => return Err(error),
        };

        if let Some(dataset_items) = business_detail_dataset_items(&response, request)? {
            let saved_items = push_result(&apify, &mut dataset_budget, &dataset_items).await?;
            if saved_items < dataset_items.len() {
                println!(
                    "Skipped {} dataset item{} because the PAY_PER_EVENT run budget is exhausted",
                    dataset_items.len() - saved_items,
                    if dataset_items.len() - saved_items == 1 {
                        ""
                    } else {
                        "s"
                    }
                );
            }
            println!(
                "Successfully fetched: {}",
                dataset_items
                    .first()
                    .and_then(|item| item.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("Business")
            );
            succeeded += 1;
            results.push(json!({
                "input_business_id": request.input_business_id,
                "business_id": request.business_id,
                "found": true
            }));
        } else {
            println!("No business details found for ID: {}", request.business_id);
            push_result(
                &apify,
                &mut dataset_budget,
                &[no_details_dataset_item(request)],
            )
            .await?;
            results.push(json!({
                "input_business_id": request.input_business_id,
                "business_id": request.business_id,
                "found": false
            }));
            failed += 1;
        }

        first_output.get_or_insert(response);
    }

    let output = build_output(requests.len(), first_output, succeeded, failed, results)?;
    apify.put_output(&output).await?;
    println!("Completed successfully");
    Ok(())
}

async fn push_result(
    apify: &ApifyClient<'_>,
    budget: &mut DatasetBudget,
    items: &[Value],
) -> Result<usize> {
    apify
        .push_dataset_items(items, budget)
        .await
        .context("Failed to save business details to the Apify dataset")
}

fn is_business_not_found(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<ScrappaApiError>()
        .is_some_and(|error| error.status == reqwest::StatusCode::NOT_FOUND)
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        build_batch_output, build_output, business_detail_dataset_items, input::BusinessIdRequest,
        no_details_dataset_item, not_found_dataset_item,
    };

    fn request(input_business_id: &str, business_id: &str) -> BusinessIdRequest {
        BusinessIdRequest {
            input_business_id: input_business_id.to_owned(),
            business_id: business_id.to_owned(),
        }
    }

    #[test]
    fn enriches_each_api_record_and_keeps_all_original_fields() {
        let request = request("input-id", "requested-id");
        let response = json!({
            "status": "OK",
            "data": [
                {"business_id": null, "name": "Coffee", "photos": [{"width": 400}]},
                {"business_id": "canonical-id", "name": "Bakery", "custom": true}
            ]
        });

        assert_eq!(
            business_detail_dataset_items(&response, &request).unwrap(),
            Some(vec![
                json!({
                    "business_id": "requested-id",
                    "input_business_id": "input-id",
                    "name": "Coffee",
                    "photos": [{"width": 400}]
                }),
                json!({
                    "business_id": "canonical-id",
                    "input_business_id": "input-id",
                    "name": "Bakery",
                    "custom": true
                })
            ])
        );
    }

    #[test]
    fn empty_or_missing_data_uses_the_legacy_no_details_row() {
        let request = request("input-id", "business-id");
        assert_eq!(
            business_detail_dataset_items(&json!({"data": []}), &request).unwrap(),
            None
        );
        assert_eq!(
            business_detail_dataset_items(&json!({"status": "OK"}), &request).unwrap(),
            None
        );
        assert_eq!(
            no_details_dataset_item(&request),
            json!({
                "success": false,
                "input_business_id": "input-id",
                "business_id": "business-id",
                "error": "No business details found"
            })
        );
    }

    #[test]
    fn preserves_404_row_and_batch_output_shapes() {
        let request = request("input-id", "business-id");
        assert_eq!(
            not_found_dataset_item(&request),
            json!({
                "success": false,
                "input_business_id": "input-id",
                "business_id": "business-id",
                "error": "Business not found"
            })
        );
        assert_eq!(
            build_batch_output(
                2,
                1,
                1,
                vec![
                    json!({"input_business_id": "found", "business_id": "found", "found": true}),
                    json!({"input_business_id": "missing", "business_id": "missing", "found": false})
                ]
            ),
            json!({
                "requested": 2,
                "succeeded": 1,
                "failed": 1,
                "results": [
                    {"input_business_id": "found", "business_id": "found", "found": true},
                    {"input_business_id": "missing", "business_id": "missing", "found": false}
                ]
            })
        );
    }

    #[test]
    fn keeps_the_full_api_response_for_legacy_single_business_runs() {
        let response = json!({
            "status": "OK",
            "data": [{"business_id": "business-id", "name": "Coffee"}],
            "pagination": {"next_page": null}
        });

        assert_eq!(
            build_output(1, Some(response.clone()), 1, 0, vec![]).unwrap(),
            response
        );
        assert_eq!(
            build_output(
                1,
                Some(json!({"data": [], "error": "Business not found"})),
                0,
                1,
                vec![]
            )
            .unwrap(),
            json!({"data": [], "error": "Business not found"})
        );
    }
}
