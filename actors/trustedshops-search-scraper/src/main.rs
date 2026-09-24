mod apify;
mod request_params;
mod response_utils;
mod scrappa;

#[cfg(test)]
mod test_support;

use anyhow::{Context, Result, anyhow};
use apify::{
    ApifyClient, ChargeManager, ChargeResult, DATASET_ITEM_CHARGE_EVENT, SHOP_RESULT_CHARGE_EVENT,
};
use request_params::{SearchPlan, build_search_plan, describe_request, page_params};
use response_utils::{build_dataset_item, trusted_shops};
use scrappa::{ScrappaClient, ScrappaTimeoutError};
use serde_json::{Map, Value, json};
use std::{env, process};

const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const APIFY_API_PUBLIC_BASE_URL: &str = "https://api.apify.com";
const SEARCH_ENDPOINT: &str = "/trustedshops/search";

struct Config {
    scrappa_api_key: String,
    scrappa_api_base_url: String,
    apify_api_base_url: String,
    apify_token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    is_at_home: bool,
    test_pay_per_event: bool,
    max_total_charge_usd: Option<f64>,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = required_env("SCRAPPA_API_KEY").map_err(|_| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })?;
        let max_total_charge_usd = env::var("ACTOR_MAX_TOTAL_CHARGE_USD")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|value| {
                value
                    .parse::<f64>()
                    .context("ACTOR_MAX_TOTAL_CHARGE_USD must be a number")
            })
            .transpose()?;

        Ok(Self {
            scrappa_api_key,
            scrappa_api_base_url: env::var("SCRAPPA_API_BASE_URL")
                .unwrap_or_else(|_| SCRAPPA_API_BASE_URL.to_owned()),
            apify_api_base_url: env::var("APIFY_API_PUBLIC_BASE_URL")
                .unwrap_or_else(|_| APIFY_API_PUBLIC_BASE_URL.to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            is_at_home: env_flag("APIFY_IS_AT_HOME"),
            test_pay_per_event: env_flag("ACTOR_TEST_PAY_PER_EVENT"),
            max_total_charge_usd,
        })
    }

    fn apify_client(&self) -> Result<ApifyClient> {
        ApifyClient::new(
            &self.apify_api_base_url,
            self.apify_token.clone(),
            self.actor_run_id.clone(),
            self.key_value_store_id.clone(),
            self.dataset_id.clone(),
            self.input_key.clone(),
            self.is_at_home,
        )
    }
}

struct PushResult {
    saved_count: usize,
    charge_result: Option<ChargeResult>,
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is missing"))
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE"))
        .unwrap_or(false)
}

fn build_output(
    plan: &SearchPlan,
    pages_fetched: usize,
    responses: Vec<Value>,
    shops_extracted: usize,
    status_message: Option<&str>,
    latest_metadata: Option<&Value>,
) -> Value {
    let mut request: Map<String, Value> = plan.base_params.clone();
    request.insert("start_page".to_owned(), json!(plan.start_page));
    request.insert("max_pages".to_owned(), json!(plan.max_pages));
    json!({
        "request": request,
        "pages_fetched": pages_fetched,
        "responses_saved": responses.len(),
        "shops_extracted": shops_extracted,
        "status_message": status_message,
        "total_shop_count": latest_metadata
            .and_then(|metadata| metadata.get("totalShopCount"))
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
        "total_page_count": latest_metadata
            .and_then(|metadata| metadata.get("totalPageCount"))
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
        "responses": responses,
    })
}

async fn push_charged_items(
    apify: &ApifyClient,
    charge_manager: &mut ChargeManager,
    items: &[Value],
    actor_run_id: &str,
    page: i64,
) -> Result<PushResult> {
    if !charge_manager.is_pay_per_event() {
        apify.push_data(items).await?;
        return Ok(PushResult {
            saved_count: items.len(),
            charge_result: None,
        });
    }

    let items_to_keep = charge_manager.items_to_keep(items.len());
    if items_to_keep == 0 {
        return Ok(PushResult {
            saved_count: 0,
            charge_result: Some(ChargeResult {
                charged_count: 0,
                event_charge_limit_reached: true,
            }),
        });
    }

    apify.push_data(&items[..items_to_keep]).await?;
    let custom_event_result = charge_manager
        .charge(
            apify,
            SHOP_RESULT_CHARGE_EVENT,
            items_to_keep,
            &format!("{actor_run_id}-trustedshops-{page}-shop-result"),
        )
        .await?;
    let dataset_event_result = charge_manager
        .charge(
            apify,
            DATASET_ITEM_CHARGE_EVENT,
            items_to_keep,
            &format!("{actor_run_id}-trustedshops-{page}-dataset-item"),
        )
        .await?;
    let charge_result = ChargeResult {
        charged_count: custom_event_result
            .charged_count
            .saturating_add(dataset_event_result.charged_count),
        event_charge_limit_reached: custom_event_result.event_charge_limit_reached
            || dataset_event_result.event_charge_limit_reached,
    };

    Ok(PushResult {
        saved_count: charge_result.charged_count.min(items.len()),
        charge_result: Some(charge_result),
    })
}

async fn search(
    plan: &SearchPlan,
    apify: &ApifyClient,
    scrappa: &ScrappaClient,
    charge_manager: &mut ChargeManager,
    actor_run_id: &str,
) -> Result<Value> {
    println!("Searching Trusted Shops for {}", describe_request(plan));

    let mut responses = Vec::new();
    let mut pages_fetched = 0;
    let mut shops_extracted = 0;
    let mut status_message = None;
    let mut latest_metadata = None;

    for offset in 0..plan.max_pages {
        let page = plan.start_page + offset;
        let params = page_params(plan, page);
        println!(
            "Fetching Trusted Shops page {page} for {} in {}",
            params.get("q").and_then(Value::as_str).unwrap_or_default(),
            params
                .get("market")
                .and_then(Value::as_str)
                .unwrap_or_default()
        );

        let response = scrappa.get(SEARCH_ENDPOINT, &params).await?;
        pages_fetched += 1;
        latest_metadata = response.get("metaData").cloned();

        let shops = trusted_shops(&response);
        let dataset_items = shops
            .iter()
            .map(|shop| build_dataset_item(shop, &params, &response))
            .collect::<Vec<_>>();

        if !dataset_items.is_empty() {
            let result =
                push_charged_items(apify, charge_manager, &dataset_items, actor_run_id, page)
                    .await?;
            shops_extracted += result.saved_count;
            println!(
                "Found {} shop result(s) on page {page}; saved {}",
                dataset_items.len(),
                result.saved_count
            );

            if result
                .charge_result
                .is_some_and(|charge_result| charge_result.event_charge_limit_reached)
            {
                let message = format!(
                    "Charge limit reached after saving {} of {} Trusted Shops results on the current page.",
                    result.saved_count,
                    dataset_items.len()
                );
                println!(
                    "{message} {}",
                    json!({
                        "event": SHOP_RESULT_CHARGE_EVENT,
                        "charged_count": result.charge_result.map(|charge| charge.charged_count),
                        "requested_count": dataset_items.len(),
                        "saved_count": result.saved_count
                    })
                );
                status_message = Some(message);
                break;
            }
            responses.push(response);
        } else {
            responses.push(response);
            println!("No Trusted Shops results found on page {page}");
            break;
        }

        let total_page_count = latest_metadata
            .as_ref()
            .and_then(|metadata| metadata.get("totalPageCount"))
            .and_then(Value::as_f64);
        if total_page_count.is_some_and(|total| (page + 1) as f64 >= total) {
            println!(
                "Stopping after page {page}; Scrappa reported {total_page_count:?} total page(s)"
            );
            break;
        }
    }

    let output = build_output(
        plan,
        pages_fetched,
        responses,
        shops_extracted,
        status_message.as_deref(),
        latest_metadata.as_ref(),
    );
    apify.set_output(&output).await?;

    println!("Trusted Shops search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "pages_fetched": output["pages_fetched"],
            "responses_saved": output["responses_saved"],
            "shops_extracted": output["shops_extracted"],
            "total_shop_count": output["total_shop_count"],
            "total_page_count": output["total_page_count"]
        })
    );
    if let Some(status_message) = status_message.as_deref() {
        apify.set_status_message(status_message).await?;
    }
    Ok(output)
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let apify = config.apify_client()?;
    let mut charge_manager = apify
        .load_charge_manager(config.max_total_charge_usd, config.test_pay_per_event)
        .await?;
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let plan = build_search_plan(&input).map_err(anyhow::Error::msg)?;
    let scrappa = ScrappaClient::new(config.scrappa_api_key, config.scrappa_api_base_url)?;
    search(
        &plan,
        &apify,
        &scrappa,
        &mut charge_manager,
        &config.actor_run_id,
    )
    .await?;
    Ok(())
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{error}. The Trusted Shops search request exceeded the {}s Scrappa API timeout. Try fewer pages or run the request again.",
            scrappa::REQUEST_TIMEOUT_MS / 1000
        )
    } else {
        format!("{error:#}")
    }
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
    use super::*;
    use crate::test_support::{MockServer, has_bearer_token, request_parts, response};
    use serde_json::json;

    fn test_apify(base_url: &str) -> ApifyClient {
        ApifyClient::new(
            base_url,
            "test-apify-token".to_owned(),
            "test-run".to_owned(),
            "test-store".to_owned(),
            "test-dataset".to_owned(),
            "INPUT".to_owned(),
            true,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn pay_per_event_page_limits_saved_rows_and_persists_actor_outputs() {
        let pricing = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "shop-result": {"eventPriceUsd": 0.04},
                        "apify-default-dataset-item": {"eventPriceUsd": 0.01},
                        "other-event": {"eventPriceUsd": 0.01}
                    }}
                },
                "options": {"maxTotalChargeUsd": 0.08},
                "chargedEventCounts": {"other-event": 1}
            }
        });
        let apify_server = MockServer::start(vec![
            response(200, pricing.to_string()),
            response(
                200,
                r#"{"q":" H&M partner ","market":"fra","page":2,"max_pages":2}"#,
            ),
            response(200, "{}"),
            response(201, "{}"),
            response(200, "{}"),
            response(200, "{}"),
        ]);
        let scrappa_server = MockServer::start(vec![response(
            200,
            json!({
                "metaData": {"totalShopCount": 77, "totalPageCount": 6},
                "shops": [
                    {
                        "tsID": "shop-1",
                        "shopName": "First Shop",
                        "shopUrl": "first.example",
                        "profileUrl": "www.trustedshops.de/shop-1",
                        "shopCategories": [{"name": "Fashion", "id": 23, "urlPath": "fashion"}],
                        "upstreamExtra": true
                    },
                    {"tsID": "shop-2", "shopName": "Second Shop"}
                ]
            })
            .to_string(),
        )]);
        let apify = test_apify(&apify_server.base_url);
        let mut charge_manager = apify.load_charge_manager(None, false).await.unwrap();
        let input = apify.get_input().await.unwrap().unwrap();
        let plan = build_search_plan(&input).unwrap();
        let scrappa = ScrappaClient::new(
            "test-scrappa-key".to_owned(),
            format!("{}/api", scrappa_server.base_url),
        )
        .unwrap();

        let output = search(&plan, &apify, &scrappa, &mut charge_manager, "test-run")
            .await
            .unwrap();

        assert_eq!(
            output,
            json!({
                "request": {"q":"H&M partner","market":"FRA","start_page":2,"max_pages":2},
                "pages_fetched": 1,
                "responses_saved": 0,
                "shops_extracted": 2,
                "status_message": "Charge limit reached after saving 2 of 2 Trusted Shops results on the current page.",
                "total_shop_count": 77,
                "total_page_count": 6,
                "responses": []
            })
        );

        let scrappa_requests = scrappa_server.requests();
        assert_eq!(scrappa_requests.len(), 1);
        let (method, path, _) = request_parts(&scrappa_requests[0]);
        assert_eq!(method, "GET");
        assert!(path.starts_with("/api/trustedshops/search?"));
        assert!(path.contains("q=H%26M+partner"));
        assert!(path.contains("market=FRA"));
        assert!(path.contains("page=2"));
        assert!(
            scrappa_requests[0]
                .to_ascii_lowercase()
                .contains("x-api-key: test-scrappa-key")
        );
        assert!(scrappa_requests[0].contains("thescrappa-trustedshops-search-scraper/1.0"));

        let apify_requests = apify_server.requests();
        assert_eq!(apify_requests.len(), 6);
        assert_eq!(
            request_parts(&apify_requests[0]).1,
            "/v2/actor-runs/test-run"
        );
        assert_eq!(
            request_parts(&apify_requests[1]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert_eq!(
            request_parts(&apify_requests[2]).1,
            "/v2/datasets/test-dataset/items"
        );
        assert_eq!(
            request_parts(&apify_requests[3]).1,
            "/v2/actor-runs/test-run/charge"
        );
        assert_eq!(
            request_parts(&apify_requests[4]).1,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
        assert_eq!(
            request_parts(&apify_requests[5]).1,
            "/v2/actor-runs/test-run"
        );
        for request in &apify_requests {
            assert!(has_bearer_token(request, "test-apify-token"));
        }

        let saved_items: Value = serde_json::from_str(request_parts(&apify_requests[2]).2).unwrap();
        assert_eq!(saved_items.as_array().unwrap().len(), 1);
        assert_eq!(saved_items[0]["tsID"], "shop-1");
        assert_eq!(saved_items[0]["shop_url"], "https://first.example");
        assert_eq!(
            saved_items[0]["profile_url"],
            "https://www.trustedshops.de/shop-1"
        );
        assert_eq!(saved_items[0]["category_names"], "Fashion");
        assert_eq!(saved_items[0]["request_q"], "H&M partner");
        assert_eq!(saved_items[0]["request_page"], 2);
        assert_eq!(saved_items[0]["total_shop_count"], 77);
        assert_eq!(saved_items[0]["upstreamExtra"], true);

        let charged_event: Value =
            serde_json::from_str(request_parts(&apify_requests[3]).2).unwrap();
        assert_eq!(charged_event, json!({"eventName":"shop-result","count":1}));
        assert!(
            apify_requests[3]
                .to_ascii_lowercase()
                .contains("idempotency-key: test-run-trustedshops-2-shop-result")
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&apify_requests[4]).2).unwrap(),
            output
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&apify_requests[5]).2).unwrap(),
            json!({
                "runId":"test-run",
                "statusMessage":"Charge limit reached after saving 2 of 2 Trusted Shops results on the current page.",
                "isStatusMessageTerminal":true
            })
        );
    }

    #[test]
    fn output_keeps_response_count_distinct_from_pages_fetched() {
        let plan = build_search_plan(&json!({"q":"zalando","page":3,"max_pages":2})).unwrap();
        let output = build_output(
            &plan,
            2,
            vec![json!({"shops": []})],
            1,
            None,
            Some(&json!({"totalPageCount": 9})),
        );
        assert_eq!(output["pages_fetched"], 2);
        assert_eq!(output["responses_saved"], 1);
        assert_eq!(output["shops_extracted"], 1);
        assert_eq!(output["request"]["start_page"], 3);
        assert_eq!(output["request"]["max_pages"], 2);
        assert_eq!(output["total_page_count"], 9);
    }
}
