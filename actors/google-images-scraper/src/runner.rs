use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use tokio::task::JoinSet;

use crate::{
    apify::ApifyClient,
    request_params::{build_google_images_param_list, describe_google_images_request},
    response_utils::{enrich_result, extract_image_results, limit_image_results},
    scrappa::ScrappaClient,
};

const BATCH_CONCURRENCY: usize = 5;

struct GoogleImagesRequestSummary {
    request: Value,
    image_results: usize,
    products: usize,
    with_original: usize,
    with_dimensions: usize,
}

struct GoogleImagesRequestResult {
    summary: GoogleImagesRequestSummary,
    dataset_items: Vec<Value>,
    response: Value,
}

pub async fn run_actor(
    apify: &ApifyClient,
    scrappa: Arc<ScrappaClient>,
    key_value_store_id: &str,
    dataset_id: &str,
    actor_run_id: &str,
    input_key: &str,
) -> Result<()> {
    let input = apify
        .get_input(key_value_store_id, input_key)
        .await?
        .ok_or_else(|| anyhow!("Input is required"))?;
    let param_list = build_google_images_param_list(&input).map_err(anyhow::Error::msg)?;
    println!(
        "Running {} Google Images request{}",
        param_list.len(),
        if param_list.len() == 1 { "" } else { "s" }
    );

    let mut remaining_budget = apify.dataset_item_budget(actor_run_id).await?;
    let results = run_requests(
        apify,
        scrappa,
        dataset_id,
        &mut remaining_budget,
        param_list,
    )
    .await?;
    let request_summaries = results
        .iter()
        .map(|result| summary_value(&result.summary))
        .collect::<Vec<_>>();
    let total_image_results = results
        .iter()
        .map(|result| result.summary.image_results)
        .sum::<usize>();
    let total_products = results
        .iter()
        .map(|result| result.summary.products)
        .sum::<usize>();
    let total_with_original = results
        .iter()
        .map(|result| result.summary.with_original)
        .sum::<usize>();
    let total_with_dimensions = results
        .iter()
        .map(|result| result.summary.with_dimensions)
        .sum::<usize>();

    let output = if results.len() == 1 {
        results[0].response.clone()
    } else {
        json!({
            "requests": request_summaries,
            "image_results": total_image_results,
        })
    };
    apify.set_output(key_value_store_id, &output).await?;

    let summary = json!({
        "requests": results.len(),
        "image_results": total_image_results,
        "products": total_products,
        "with_original": total_with_original,
        "with_dimensions": total_with_dimensions,
    });
    println!("Google Images scraping completed successfully");
    println!("Results summary: {summary}");
    Ok(())
}

async fn run_requests(
    apify: &ApifyClient,
    scrappa: Arc<ScrappaClient>,
    dataset_id: &str,
    remaining_budget: &mut usize,
    param_list: Vec<Value>,
) -> Result<Vec<GoogleImagesRequestResult>> {
    let mut pending = JoinSet::new();
    let mut next_index = 0;
    let mut results = (0..param_list.len()).map(|_| None).collect::<Vec<_>>();

    while next_index < param_list.len() || !pending.is_empty() {
        while next_index < param_list.len() && pending.len() < BATCH_CONCURRENCY {
            let index = next_index;
            let params = param_list[index].clone();
            let client = Arc::clone(&scrappa);
            pending.spawn(async move {
                let result = run_google_images_request(&client, params).await;
                (index, result)
            });
            next_index += 1;
        }

        let (index, result) = pending
            .join_next()
            .await
            .context("Google Images request task ended unexpectedly")?
            .map_err(|error| anyhow!("Google Images request task failed: {error}"))?;
        let mut result = result?;
        if !result.dataset_items.is_empty() {
            let saved = apify
                .push_data(dataset_id, &result.dataset_items, remaining_budget)
                .await?;
            if saved < result.dataset_items.len() {
                println!(
                    "Saved {saved} of {} image results within the PAY_PER_EVENT budget",
                    result.dataset_items.len()
                );
                result.response = limit_image_results(&result.response, saved);
            }
        }
        results[index] = Some(result);
    }

    results
        .into_iter()
        .map(|result| result.ok_or_else(|| anyhow!("Google Images request result is missing")))
        .collect()
}

async fn run_google_images_request(
    client: &ScrappaClient,
    params: Value,
) -> Result<GoogleImagesRequestResult> {
    println!(
        "Fetching Google Images for {}",
        describe_google_images_request(&params)
    );
    let response = client.get("/images", &params).await?;
    let image_results = extract_image_results(&response);
    let dataset_items = image_results
        .iter()
        .map(|result| enrich_result(result, &params))
        .collect::<Vec<_>>();

    if dataset_items.is_empty() {
        println!("No Google Images results found for this request");
    } else {
        println!("Found {} image results", image_results.len());
    }

    let summary = GoogleImagesRequestSummary {
        request: params,
        image_results: image_results.len(),
        products: image_results
            .iter()
            .filter(|result| is_truthy(result.get("is_product")))
            .count(),
        with_original: image_results
            .iter()
            .filter(|result| {
                result
                    .get("original")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.is_empty())
            })
            .count(),
        with_dimensions: image_results
            .iter()
            .filter(|result| {
                result.get("original_width").is_some_and(Value::is_number)
                    && result.get("original_height").is_some_and(Value::is_number)
            })
            .count(),
    };

    Ok(GoogleImagesRequestResult {
        summary,
        dataset_items,
        response,
    })
}

fn summary_value(summary: &GoogleImagesRequestSummary) -> Value {
    json!({
        "request": summary.request,
        "image_results": summary.image_results,
        "products": summary.products,
        "with_original": summary.with_original,
        "with_dimensions": summary.with_dimensions,
    })
}

fn is_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Number(number)) => number.as_f64().is_some_and(|value| value != 0.0),
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Bool(true) | Value::Array(_) | Value::Object(_)) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockResponse, MockServer};
    use serde_json::json;

    const RUN_INFO: &str = r#"{
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                    "apify-actor-start": {"eventPriceUsd": 0.01}
                }}
            },
            "options": {"maxTotalChargeUsd": 0.21},
            "chargedEventCounts": {"apify-actor-start": 1}
        }
    }"#;

    #[tokio::test]
    async fn runs_a_query_saves_only_affordable_items_and_preserves_single_query_output() {
        let input = r#"{"q":"coffee","page":2,"hl":"en","gl":"us"}"#;
        let response = r#"[{"position":1,"title":"Coffee","original":"https://example.com/coffee.jpg","is_product":true},{"position":2,"title":"Beans"},{"position":3,"title":"Cup"}]"#;
        let apify_server = MockServer::start(vec![
            MockResponse {
                status: 200,
                body: input.into(),
            },
            MockResponse {
                status: 200,
                body: RUN_INFO.into(),
            },
            MockResponse {
                status: 201,
                body: String::new(),
            },
            MockResponse {
                status: 200,
                body: String::new(),
            },
        ]);
        let scrappa_server = MockServer::start(vec![MockResponse {
            status: 200,
            body: response.into(),
        }]);
        let apify = ApifyClient::new("test-token", &apify_server.base_url).unwrap();
        let scrappa = Arc::new(
            ScrappaClient::with_timeout(
                "test-key",
                format!("{}/api", scrappa_server.base_url),
                1_000,
            )
            .unwrap(),
        );

        run_actor(&apify, scrappa, "store", "dataset", "run", "INPUT")
            .await
            .unwrap();

        let apify_requests = apify_server.requests();
        assert_eq!(apify_requests.len(), 4);
        let dataset_write = &apify_requests[2];
        let (_, path, body) = request_parts(dataset_write);
        assert_eq!(path, "/v2/datasets/dataset/items");
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([
                {
                    "position": 1,
                    "title": "Coffee",
                    "original": "https://example.com/coffee.jpg",
                    "is_product": true,
                    "source": null,
                    "image_url": "https://example.com/coffee.jpg",
                    "thumbnail_url": null,
                    "source_url": null,
                    "width": null,
                    "height": null,
                    "request_q": "coffee",
                    "request_page": 2,
                    "request_hl": "en",
                    "request_gl": "us",
                    "request_imgsz": null,
                    "request_imgtype": null,
                    "request_imgcolor": null,
                    "request_imgar": null,
                    "request_tbs": null,
                    "request_safe": null
                },
                {
                    "position": 2,
                    "title": "Beans",
                    "source": null,
                    "image_url": null,
                    "thumbnail_url": null,
                    "source_url": null,
                    "width": null,
                    "height": null,
                    "is_product": false,
                    "request_q": "coffee",
                    "request_page": 2,
                    "request_hl": "en",
                    "request_gl": "us",
                    "request_imgsz": null,
                    "request_imgtype": null,
                    "request_imgcolor": null,
                    "request_imgar": null,
                    "request_tbs": null,
                    "request_safe": null
                }
            ])
        );
        let output_write = &apify_requests[3];
        let (_, path, body) = request_parts(output_write);
        assert_eq!(path, "/v2/key-value-stores/store/records/OUTPUT");
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([
                {"position":1,"title":"Coffee","original":"https://example.com/coffee.jpg","is_product":true},
                {"position":2,"title":"Beans"}
            ])
        );

        let scrappa_requests = scrappa_server.requests();
        assert_eq!(scrappa_requests.len(), 1);
        assert!(scrappa_requests[0].starts_with("GET /api/images?"));
        assert!(scrappa_requests[0].contains("q=coffee"));
        assert!(scrappa_requests[0].contains("page=2"));
        assert!(scrappa_requests[0]
            .lines()
            .any(|line| line.eq_ignore_ascii_case("X-API-Key: test-key")));
    }

    #[tokio::test]
    async fn non_pay_per_event_runs_save_all_results_and_preserve_output() {
        let input = r#"{"q":"coffee"}"#;
        let response = r#"[
            {"position":1,"title":"Coffee"},
            {"position":2,"title":"Beans"},
            {"position":3,"title":"Cup"}
        ]"#;
        let apify_server = MockServer::start(vec![
            MockResponse {
                status: 200,
                body: input.into(),
            },
            MockResponse {
                status: 200,
                body: r#"{"data":{"pricingInfo":{"pricingModel":"FREE"}}}"#.into(),
            },
            MockResponse {
                status: 201,
                body: String::new(),
            },
            MockResponse {
                status: 200,
                body: String::new(),
            },
        ]);
        let scrappa_server = MockServer::start(vec![MockResponse {
            status: 200,
            body: response.into(),
        }]);
        let apify = ApifyClient::new("test-token", &apify_server.base_url).unwrap();
        let scrappa = Arc::new(
            ScrappaClient::with_timeout(
                "test-key",
                format!("{}/api", scrappa_server.base_url),
                1_000,
            )
            .unwrap(),
        );

        run_actor(&apify, scrappa, "store", "dataset", "run", "INPUT")
            .await
            .unwrap();

        let apify_requests = apify_server.requests();
        assert_eq!(apify_requests.len(), 4);
        let (method, path, body) = request_parts(&apify_requests[2]);
        assert_eq!((method, path), ("POST", "/v2/datasets/dataset/items"));
        assert_eq!(serde_json::from_str::<Value>(body).unwrap().as_array().unwrap().len(), 3);
        let (_, path, body) = request_parts(&apify_requests[3]);
        assert_eq!(path, "/v2/key-value-stores/store/records/OUTPUT");
        assert_eq!(serde_json::from_str::<Value>(body).unwrap(), json!([
            {"position":1,"title":"Coffee"},
            {"position":2,"title":"Beans"},
            {"position":3,"title":"Cup"}
        ]));
    }

    #[tokio::test]
    async fn batch_runs_write_each_result_and_store_ordered_request_summaries() {
        let apify_server = MockServer::start(vec![
            MockResponse {
                status: 200,
                body: r#"{"queries":["coffee","tea"]}"#.into(),
            },
            MockResponse {
                status: 200,
                body: RUN_INFO.into(),
            },
            MockResponse {
                status: 201,
                body: String::new(),
            },
            MockResponse {
                status: 201,
                body: String::new(),
            },
            MockResponse {
                status: 200,
                body: String::new(),
            },
        ]);
        let scrappa_server = MockServer::start(vec![
            MockResponse {
                status: 200,
                body: r#"[{"position":1,"title":"Image"}]"#.into(),
            },
            MockResponse {
                status: 200,
                body: r#"[{"position":1,"title":"Image"}]"#.into(),
            },
        ]);
        let apify = ApifyClient::new("test-token", &apify_server.base_url).unwrap();
        let scrappa = Arc::new(
            ScrappaClient::with_timeout(
                "test-key",
                format!("{}/api", scrappa_server.base_url),
                1_000,
            )
            .unwrap(),
        );

        run_actor(&apify, scrappa, "store", "dataset", "run", "INPUT")
            .await
            .unwrap();

        let apify_requests = apify_server.requests();
        assert_eq!(apify_requests.len(), 5);
        let dataset_items = apify_requests[2..4]
            .iter()
            .flat_map(|request| {
                let (_, path, body) = request_parts(request);
                assert_eq!(path, "/v2/datasets/dataset/items");
                serde_json::from_str::<Vec<Value>>(body).unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(dataset_items.len(), 2);
        let mut request_queries = dataset_items
            .iter()
            .filter_map(|item| item["request_q"].as_str())
            .collect::<Vec<_>>();
        request_queries.sort_unstable();
        assert_eq!(request_queries, vec!["coffee", "tea"]);

        let (_, path, body) = request_parts(&apify_requests[4]);
        assert_eq!(path, "/v2/key-value-stores/store/records/OUTPUT");
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!({
                "requests": [
                    {
                        "request": {"q":"coffee"},
                        "image_results": 1,
                        "products": 0,
                        "with_original": 0,
                        "with_dimensions": 0
                    },
                    {
                        "request": {"q":"tea"},
                        "image_results": 1,
                        "products": 0,
                        "with_original": 0,
                        "with_dimensions": 0
                    }
                ],
                "image_results": 2
            })
        );
        assert_eq!(scrappa_server.requests().len(), 2);
    }

    #[test]
    fn counts_products_using_javascript_truthiness() {
        assert!(!is_truthy(None));
        assert!(!is_truthy(Some(&Value::Null)));
        assert!(!is_truthy(Some(&json!(false))));
        assert!(!is_truthy(Some(&json!(0))));
        assert!(!is_truthy(Some(&json!(""))));
        assert!(is_truthy(Some(&json!("false"))));
        assert!(is_truthy(Some(&json!([]))));
        assert!(is_truthy(Some(&json!({}))));
    }

    fn request_parts(request: &str) -> (&str, &str, &str) {
        let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
        let mut parts = headers
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace();
        (
            parts.next().unwrap_or_default(),
            parts.next().unwrap_or_default(),
            body,
        )
    }
}
