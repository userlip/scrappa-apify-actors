mod apify;
mod quote_fetch;
mod request_params;
mod response_utils;
mod scrappa;

use anyhow::{Context, Result, anyhow};
use apify::{ActorPricing, ApifyClient, ApifyConfig};
use quote_fetch::fetch_quote_with_fallback;
use request_params::{build_google_finance_quote_params, describe_google_finance_quote_request};
use response_utils::{build_quote_dataset_item, has_meaningful_quote_data};
use scrappa::{REQUEST_TIMEOUT, ScrappaClient, ScrappaHttpError, ScrappaTimeoutError};
use serde_json::Value;
use std::{env, process};

const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const QUOTE_RESULT_CHARGE_EVENT: &str = "quote-result";

fn required_scrappa_api_key() -> Result<String> {
    env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })
}

fn exit_without_quote_result(status_message: &str) {
    eprintln!("{status_message}");
}

fn scrappa_timeout_message(error: &ScrappaTimeoutError) -> String {
    format!(
        "{error}. No Google Finance quote result was written or charged. Try the run again later, or provide an exchange code if the symbol is ambiguous."
    )
}

fn upstream_failure_message(status: u16) -> String {
    format!(
        "Scrappa upstream returned {status} after retries; no Google Finance quote result was written or charged. Try the run again later."
    )
}

fn response_count(item: &Value, key: &str) -> usize {
    item.pointer(&format!("/result_counts/{key}"))
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize
}

async fn run() -> Result<()> {
    let api_key = required_scrappa_api_key()?;
    let apify_config = ApifyConfig::from_env()?;
    let apify = ApifyClient::new(apify_config)?;
    let pricing: ActorPricing = apify.get_pricing().await?;
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let params = build_google_finance_quote_params(&input).map_err(anyhow::Error::msg)?;

    println!(
        "Fetching Google Finance quote for {}",
        describe_google_finance_quote_request(&params)
    );

    let scrappa_api_base =
        env::var("SCRAPPA_API_BASE_URL").unwrap_or_else(|_| SCRAPPA_API_DEFAULT.into());
    let scrappa = ScrappaClient::new(api_key, &scrappa_api_base, REQUEST_TIMEOUT)?;
    let fetch_result =
        match fetch_quote_with_fallback(&scrappa, &params, SCRAPPA_MAX_ATTEMPTS).await {
            Ok(result) => result,
            Err(error) => {
                if let Some(error) = error.downcast_ref::<ScrappaHttpError>()
                    && (500..=599).contains(&error.status)
                {
                    exit_without_quote_result(&upstream_failure_message(error.status));
                    return Ok(());
                }
                if let Some(error) = error.downcast_ref::<ScrappaTimeoutError>() {
                    exit_without_quote_result(&scrappa_timeout_message(error));
                    return Ok(());
                }
                return Err(error);
            }
        };

    if !has_meaningful_quote_data(&fetch_result.response) {
        exit_without_quote_result(&format!(
            "Scrappa returned no usable Google Finance quote data for {}; no dataset item was written or charged.",
            describe_google_finance_quote_request(&params)
        ));
        return Ok(());
    }

    let mut item = build_quote_dataset_item(&fetch_result.response, &params);
    if let Some(fallback) = &fetch_result.fallback {
        item.as_object_mut()
            .context("Quote dataset item is not an object")?
            .insert("upstream_fallback".into(), fallback.to_value());
    } else {
        item.as_object_mut()
            .context("Quote dataset item is not an object")?
            .insert("upstream_fallback".into(), Value::Null);
    }

    let charge_plan = pricing.plan_default_dataset_item(QUOTE_RESULT_CHARGE_EVENT);
    if !charge_plan.keep_item {
        let status_message = "Charge limit reached before saving the Google Finance quote result; OUTPUT was not written.";
        println!(
            "{status_message} {{\"event\":\"{QUOTE_RESULT_CHARGE_EVENT}\",\"charged_count\":0}}"
        );
        return Ok(());
    }

    apify.push_dataset_item(&item).await?;
    if charge_plan.charge_quote_result {
        apify.charge_quote_result().await?;
    }
    apify.set_output(&fetch_result.response).await?;

    println!("Google Finance quote scraping completed successfully");
    println!(
        "Results summary: {}",
        serde_json::json!({
            "symbol": item.get("symbol"),
            "exchange": item.get("exchange"),
            "financials": response_count(&item, "financials"),
            "news": response_count(&item, "news"),
            "related_tickers": response_count(&item, "related_tickers"),
            "fallback": fetch_result.fallback.as_ref().map(|fallback| fallback.reason.as_str()),
        })
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::{response_count, upstream_failure_message};
    use serde_json::json;

    #[test]
    fn builds_the_upstream_failure_status_message() {
        assert_eq!(
            upstream_failure_message(503),
            "Scrappa upstream returned 503 after retries; no Google Finance quote result was written or charged. Try the run again later."
        );
    }

    #[test]
    fn reads_result_counts_for_the_success_log() {
        let item = json!({"result_counts": {"financials": 2, "news": 3}});
        assert_eq!(response_count(&item, "financials"), 2);
        assert_eq!(response_count(&item, "related_tickers"), 0);
    }

    #[test]
    fn preserves_the_actor_input_prefill_and_defaults() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["properties"]["symbol"]["prefill"], "AAPL");
        assert_eq!(schema["properties"]["exchange"]["prefill"], "NASDAQ");
        assert_eq!(schema["properties"]["period_type"]["default"], "quarterly");
        assert_eq!(schema["properties"]["hl"]["default"], "en");
        assert_eq!(schema["properties"]["gl"]["default"], "us");
    }
}
