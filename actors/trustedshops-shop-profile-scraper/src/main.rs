mod apify;
mod charging;
mod request_params;
mod response_utils;
mod scrappa;

#[cfg(test)]
mod test_support;

use anyhow::{Context, Result, anyhow};
use apify::ApifyClient;
use charging::ChargingManager;
use request_params::{ShopProfilePlan, ShopProfileRequest, build_plan, describe_request};
use response_utils::{build_dataset_item, build_output_summary, has_shop_profile_data};
use scrappa::{REQUEST_TIMEOUT_MS, ScrappaClient, ScrappaTimeoutError};
use serde_json::{Value, json};
use std::{env, process};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";

#[derive(Clone, Debug)]
struct ActorConfig {
    api_key: String,
    apify_token: String,
    actor_run_id: String,
    default_key_value_store_id: String,
    default_dataset_id: String,
    input_key: String,
    apify_api_base_url: String,
    scrappa_api_base_url: String,
    is_at_home: bool,
    max_total_charge_usd: Option<f64>,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            api_key: env::var("SCRAPPA_API_KEY").unwrap_or_default(),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            default_key_value_store_id: env::var("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")
                .unwrap_or_else(|_| "default".to_owned()),
            default_dataset_id: env::var("ACTOR_DEFAULT_DATASET_ID")
                .unwrap_or_else(|_| "default".to_owned()),
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_api_base_url: env::var("APIFY_API_PUBLIC_BASE_URL")
                .unwrap_or_else(|_| APIFY_API_BASE_URL.to_owned()),
            scrappa_api_base_url: env::var("SCRAPPA_API_BASE_URL")
                .unwrap_or_else(|_| SCRAPPA_API_BASE_URL.to_owned()),
            is_at_home: env::var("APIFY_IS_AT_HOME").as_deref() == Ok("1"),
            max_total_charge_usd: optional_env_float("ACTOR_MAX_TOTAL_CHARGE_USD")?,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.trim().is_empty() {
        return Err(anyhow!("Required environment variable {name} is empty"));
    }
    Ok(value)
}

fn optional_env_float(name: &str) -> Result<Option<f64>> {
    let Ok(value) = env::var(name) else {
        return Ok(None);
    };
    let value = value
        .parse::<f64>()
        .with_context(|| format!("{name} must be a number"))?;
    if !value.is_finite() {
        return Err(anyhow!("{name} must be a finite number"));
    }
    Ok(Some(value))
}

async fn load_charging_manager(
    config: &ActorConfig,
    apify: &ApifyClient,
) -> Result<ChargingManager> {
    if let (Ok(pricing_info), Ok(charged_event_counts)) = (
        env::var("APIFY_ACTOR_PRICING_INFO"),
        env::var("APIFY_CHARGED_ACTOR_EVENT_COUNTS"),
    ) {
        let pricing_info = serde_json::from_str::<Value>(&pricing_info)
            .context("APIFY_ACTOR_PRICING_INFO is not valid JSON")?;
        let charged_event_counts = serde_json::from_str::<Value>(&charged_event_counts)
            .context("APIFY_CHARGED_ACTOR_EVENT_COUNTS is not valid JSON")?;
        return Ok(ChargingManager::from_environment(
            &pricing_info,
            &charged_event_counts,
            config.max_total_charge_usd,
        ));
    }

    if config.is_at_home {
        return Ok(ChargingManager::from_run(
            &apify.get_run(&config.actor_run_id).await?,
        ));
    }

    Ok(ChargingManager::free())
}

fn invalid_request_failure(request: &ShopProfileRequest, message: &str) -> Value {
    json!({
        "source_url": request.source_url,
        "error": message,
    })
}

fn format_actor_error(error: &anyhow::Error) -> String {
    if error
        .chain()
        .any(|cause| cause.downcast_ref::<ScrappaTimeoutError>().is_some())
    {
        return format!(
            "{}. The TrustedShops shop profile request exceeded the {}s Scrappa API timeout. Try fewer TSIDs or run the request again.",
            error,
            REQUEST_TIMEOUT_MS / 1_000
        );
    }

    error.to_string()
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify = ApifyClient::new(&config.apify_api_base_url, config.apify_token.clone())?;
    let mut charging = load_charging_manager(&config, &apify).await?;
    let result = run_actor_with_charging(&config, &apify, &mut charging).await;

    if let Err(error) = result {
        let message = format_actor_error(&error);
        apify
            .set_status_message(&config.actor_run_id, &message)
            .await;
        return Err(error);
    }

    Ok(())
}

async fn run_actor_with_charging(
    config: &ActorConfig,
    apify: &ApifyClient,
    charging: &mut ChargingManager,
) -> Result<()> {
    if config.api_key.is_empty() {
        return Err(anyhow!(
            "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."
        ));
    }

    let input = apify
        .get_input(&config.default_key_value_store_id, &config.input_key)
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let plan = build_plan(&input).map_err(anyhow::Error::msg)?;
    println!(
        "Fetching TrustedShops shop profiles for {}",
        describe_request(&plan)
    );

    let scrappa = ScrappaClient::new(config.api_key.clone(), config.scrappa_api_base_url.clone())?;
    process_plan_with_charging(config, apify, &scrappa, charging, plan).await
}

async fn process_plan_with_charging(
    config: &ActorConfig,
    apify: &ApifyClient,
    scrappa: &ScrappaClient,
    charging: &mut ChargingManager,
    plan: ShopProfilePlan,
) -> Result<()> {
    let mut failures = Vec::new();
    let mut saved_profiles = 0;
    let mut status_message = None;

    for request in &plan.requests {
        let Some(tsid) = request.tsid.as_deref() else {
            let message = request
                .validation_error
                .as_deref()
                .unwrap_or("Invalid TrustedShops profile input");
            failures.push(invalid_request_failure(request, message));
            eprintln!(
                "Skipping invalid TrustedShops profile input: {}",
                request
                    .validation_error
                    .as_deref()
                    .or(request.source_url.as_deref())
                    .unwrap_or("unknown input")
            );
            continue;
        };

        println!("Fetching TrustedShops shop profile for {tsid}");
        match fetch_and_save_profile_with_charging(
            config, apify, scrappa, charging, &plan, request, tsid,
        )
        .await
        {
            Ok(result) => {
                saved_profiles += result.saved_count;
                println!(
                    "Saved {} TrustedShops shop profile result(s) for {tsid}",
                    result.saved_count
                );
                if let Some(message) = result.status_message {
                    status_message = Some(message);
                    break;
                }
            }
            Err(error) => {
                let message = format_actor_error(&error);
                failures.push(json!({
                    "tsid": tsid,
                    "source_url": request.source_url,
                    "error": message,
                }));
                eprintln!("Failed to fetch TrustedShops shop profile for {tsid}: {message}");
            }
        }
    }

    if status_message.is_none() && !failures.is_empty() {
        status_message = Some(format!(
            "{} of {} TrustedShops shop profile request(s) failed.",
            failures.len(),
            plan.requests.len()
        ));
    }

    let output = build_output_summary(
        plan.requests.len(),
        saved_profiles,
        &failures,
        status_message.as_deref(),
    );
    apify
        .set_output(&config.default_key_value_store_id, &output)
        .await?;

    println!("TrustedShops shop profile extraction completed");
    println!(
        "Results summary: {}",
        json!({
            "profiles_requested": plan.requests.len(),
            "profiles_saved": saved_profiles,
            "profiles_failed": failures.len(),
        })
    );

    if saved_profiles == 0 && !failures.is_empty() {
        return Err(anyhow!(status_message.unwrap_or_else(|| {
            "No TrustedShops shop profiles were saved.".to_owned()
        })));
    }

    if let Some(message) = status_message {
        apify
            .set_status_message(&config.actor_run_id, &message)
            .await;
    }

    Ok(())
}

async fn fetch_and_save_profile_with_charging(
    config: &ActorConfig,
    apify: &ApifyClient,
    scrappa: &ScrappaClient,
    charging: &mut ChargingManager,
    plan: &ShopProfilePlan,
    request: &ShopProfileRequest,
    tsid: &str,
) -> Result<apify::PushDataResult> {
    let response = scrappa.get_shop_profile(tsid).await?;
    if !has_shop_profile_data(&response) {
        return Err(anyhow!(
            "Scrappa response did not include a TrustedShops shop profile object"
        ));
    }

    let item = build_dataset_item(
        &response,
        tsid,
        request.source_url.as_deref(),
        plan.include_raw_response,
    )
    .map_err(anyhow::Error::msg)?;
    apify
        .push_charged_item(
            &config.actor_run_id,
            &config.default_dataset_id,
            &item,
            charging,
        )
        .await
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {}", format_actor_error(&error));
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        charging::SHOP_PROFILE_RESULT_CHARGE_EVENT,
        test_support::{MockResponse, MockServer, request_parts},
    };

    fn test_config(base_url: &str) -> ActorConfig {
        ActorConfig {
            api_key: "scrappa-test-key".to_owned(),
            apify_token: "apify-test-token".to_owned(),
            actor_run_id: "test-run".to_owned(),
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_api_base_url: base_url.to_owned(),
            scrappa_api_base_url: format!("{base_url}/api"),
            is_at_home: true,
            max_total_charge_usd: None,
        }
    }

    fn pricing_response() -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "shop-profile-result": {"eventPriceUsd":0.10},
                        "apify-default-dataset-item": {"eventPriceUsd":0.05}
                    }}
                },
                "chargedEventCounts": {},
                "options": {"maxTotalChargeUsd":0.15}
            }
        })
    }

    #[tokio::test]
    async fn runs_one_ppe_profile_from_input_through_output_and_charge_stores() {
        let tsid = "XFB15FFBDE1DEE7A55D292A7D48598A6A";
        let source_url = format!("https://www.trustedshops.de/bewertung/info_{tsid}.html");
        let server = MockServer::start(vec![
            MockResponse::json(200, pricing_response()),
            MockResponse::json(
                200,
                json!({"urls":[source_url],"include_raw_response":true}),
            ),
            MockResponse::json(
                200,
                json!({
                    "response":{"data":{"shop":{"tsId":tsid,"name":"Example Shop","url":"example-shop.de"}}}
                }),
            ),
            MockResponse::json(200, json!({})),
            MockResponse::json(201, json!({})),
            MockResponse::json(200, json!({})),
            MockResponse::json(200, json!({})),
        ]);
        let config = test_config(&server.base_url);
        let apify =
            ApifyClient::new(&config.apify_api_base_url, config.apify_token.clone()).unwrap();
        let mut charging = ChargingManager::from_run(&apify.get_run("test-run").await.unwrap());

        run_actor_with_charging(&config, &apify, &mut charging)
            .await
            .unwrap();

        let requests = server.requests();
        let paths = requests
            .iter()
            .map(|request| request_parts(request).1)
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            [
                "/v2/actor-runs/test-run",
                "/v2/key-value-stores/test-store/records/INPUT",
                "/api/trustedshops/shop/XFB15FFBDE1DEE7A55D292A7D48598A6A",
                "/v2/datasets/test-dataset/items",
                "/v2/actor-runs/test-run/charge",
                "/v2/key-value-stores/test-store/records/OUTPUT",
                "/v2/actor-runs/test-run",
            ]
        );
        let dataset_items: Value = serde_json::from_str(request_parts(&requests[3]).2).unwrap();
        assert_eq!(dataset_items[0]["name"], "Example Shop");
        assert_eq!(dataset_items[0]["source_url"], source_url);
        assert!(dataset_items[0].get("raw_response").is_some());
        let output: Value = serde_json::from_str(request_parts(&requests[5]).2).unwrap();
        assert_eq!(output["profiles_saved"], 1);
        assert_eq!(
            output["status_message"],
            "Charge limit reached after saving 1 of 1 TrustedShops shop profile results."
        );
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[4]).2).unwrap()["eventName"],
            SHOP_PROFILE_RESULT_CHARGE_EVENT
        );
    }

    #[tokio::test]
    async fn reports_invalid_inputs_and_keeps_the_summary_before_failing_the_run() {
        let server = MockServer::start(vec![
            MockResponse::json(
                200,
                json!({"urls":["https://trustedshops.de/no-tsid.html"]}),
            ),
            MockResponse::json(200, json!({})),
        ]);
        let config = test_config(&server.base_url);
        let apify =
            ApifyClient::new(&config.apify_api_base_url, config.apify_token.clone()).unwrap();
        let mut charging = ChargingManager::free();

        let error = run_actor_with_charging(&config, &apify, &mut charging)
            .await
            .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("1 of 1 TrustedShops shop profile request(s) failed.")
        );
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert_eq!(
            request_parts(&requests[1]).1,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
        let output: Value = serde_json::from_str(request_parts(&requests[1]).2).unwrap();
        assert_eq!(output["profiles_saved"], 0);
        assert_eq!(output["profiles_failed"], 1);
        assert_eq!(
            output["failures"][0]["source_url"],
            "https://trustedshops.de/no-tsid.html"
        );
    }

    #[test]
    fn adds_timeout_guidance_to_scrappa_timeout_errors() {
        let error: anyhow::Error = ScrappaTimeoutError::new(REQUEST_TIMEOUT_MS, None).into();
        assert_eq!(
            format_actor_error(&error),
            "Scrappa API request timed out after 90000ms. The TrustedShops shop profile request exceeded the 90s Scrappa API timeout. Try fewer TSIDs or run the request again."
        );
    }
}
