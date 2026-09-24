mod apify;
mod normalize;
mod request_params;
mod scrappa;
mod urls;

use anyhow::{anyhow, bail, Context, Result};
use apify::ApifyClient;
use normalize::{extract_single_tiktok_ad_record, normalize_tiktok_ad_record};
use request_params::{
    extract_tiktok_ad_id, resolve_tiktok_ad_requests, safe_format_tiktok_ad_lookup_for_log,
    TikTokAdLookup,
};
use reqwest::Client;
use scrappa::ScrappaClient;
use serde_json::{json, Map, Value};
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const ACTOR_TIMEOUT: Duration = Duration::from_secs(300);

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    key_value_store_id: String,
    dataset_id: String,
    run_id: String,
    input_key: String,
    apify_token: String,
    scrappa_api_key: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").unwrap_or_default();
        if scrappa_api_key.is_empty() {
            bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
        }

        Ok(Self {
            apify_api_base_url: urls::base_url_from_env(
                "APIFY_API_PUBLIC_BASE_URL",
                APIFY_API_BASE_URL,
            )?,
            scrappa_api_base_url: urls::base_url_from_env(
                "SCRAPPA_API_BASE_URL",
                SCRAPPA_API_BASE_URL,
            )?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is missing"))
}

fn processed_time(response: &Value) -> Value {
    response
        .get("processed_time")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null)
}

fn cached(response: &Value) -> Value {
    match response.get("cached") {
        Some(Value::Bool(value)) => Value::Bool(*value),
        _ => Value::Null,
    }
}

fn request_ad_id(url: &str) -> Value {
    extract_tiktok_ad_id(url)
        .map(Value::String)
        .unwrap_or(Value::Null)
}

fn success_item(
    ad: Option<&Value>,
    request: &TikTokAdLookup,
    response: &Value,
    request_index: usize,
) -> Value {
    let mut row = match ad {
        Some(ad) => match normalize_tiktok_ad_record(ad) {
            Value::Object(fields) => fields,
            _ => Map::new(),
        },
        None => Map::new(),
    };
    row.insert("request_url".to_owned(), Value::String(request.url.clone()));
    row.insert("request_ad_id".to_owned(), request_ad_id(&request.url));
    row.insert("request_index".to_owned(), Value::from(request_index));
    row.insert("result_found".to_owned(), Value::Bool(ad.is_some()));
    row.insert("processed_time".to_owned(), processed_time(response));
    row.insert("cached".to_owned(), cached(response));
    Value::Object(row)
}

fn failure_item(request: &TikTokAdLookup, request_index: usize, message: String) -> Value {
    json!({
        "request_url": request.url,
        "request_ad_id": request_ad_id(&request.url),
        "request_index": request_index,
        "result_found": false,
        "processed_time": null,
        "cached": null,
        "error_message": message
    })
}

fn assert_successful_response(response: &Value, url: &str) -> Result<()> {
    if let Some(code) = response.get("code") {
        let success = code.as_f64() == Some(0.0);
        if !success {
            let code = js_string(code);
            let message = response
                .get("msg")
                .filter(|value| !value.is_null())
                .map(js_string)
                .unwrap_or_else(|| "Unknown error".to_owned());
            bail!("Scrappa TikTok Ads API returned code {code} for {url}: {message}");
        }
    }
    Ok(())
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::Null => String::new(),
                _ => js_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

async fn run_actor() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let http = Client::builder()
        .timeout(scrappa::REQUEST_TIMEOUT)
        .build()
        .context("Failed to create HTTP client")?;
    let mut apify = ApifyClient::new(
        http.clone(),
        config.apify_api_base_url,
        config.apify_token,
        config.run_id,
        config.key_value_store_id,
        config.dataset_id,
        config.input_key,
    );
    let scrappa = ScrappaClient::new(http, config.scrappa_api_base_url, config.scrappa_api_key);

    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("At least one TikTok Creative Center ad URL is required"))?;
    let requests = resolve_tiktok_ad_requests(&input)?;
    println!(
        "Fetching TikTok ad details for {} URL{}",
        requests.len(),
        if requests.len() == 1 { "" } else { "s" }
    );

    let mut dataset_items = 0usize;
    let mut ads_found = 0usize;
    let mut lookups_failed = 0usize;
    let mut charge_limit_reached = false;

    for (index, request) in requests.iter().enumerate() {
        let request_index = index + 1;
        let remaining_requests = requests.len() - index;
        if apify.dataset_capacity(remaining_requests).await? == 0 {
            charge_limit_reached = true;
            println!(
                "Apify event charge limit reached. Stopping before fetching additional ad URLs."
            );
            break;
        }

        let lookup_result = async {
            if let Some(message) = &request.validation_error {
                bail!("{message}");
            }
            let display_url = safe_format_tiktok_ad_lookup_for_log(&request.url);
            println!(
                "Fetching TikTok ad {request_index}/{}: {display_url}",
                requests.len()
            );
            let response = scrappa.get_ad(&request.url).await?;
            assert_successful_response(&response, &request.url)?;
            let ad =
                extract_single_tiktok_ad_record(response.get("data"), &request.url, |warning| {
                    eprintln!("Warning: {warning}");
                })?;
            let row = success_item(ad.as_ref(), request, &response, request_index);
            apify.push_dataset_item(&row).await?;
            Ok::<bool, anyhow::Error>(ad.is_some())
        }
        .await;

        match lookup_result {
            Ok(true) => {
                dataset_items += 1;
                ads_found += 1;
                println!("Found 1 TikTok ad record");
            }
            Ok(false) => {
                dataset_items += 1;
                println!(
                    "No ad details found for: {}",
                    safe_format_tiktok_ad_lookup_for_log(&request.url)
                );
            }
            Err(error) => {
                let message = format!("{error:#}");
                eprintln!(
                    "TikTok ad lookup failed for {}: {message}",
                    safe_format_tiktok_ad_lookup_for_log(&request.url)
                );
                apify
                    .push_dataset_item(&failure_item(request, request_index, message))
                    .await
                    .context("Failed to save TikTok Ads lookup failure item")?;
                dataset_items += 1;
                lookups_failed += 1;
            }
        }
    }

    let summary = json!({
        "urls_requested": requests.len(),
        "dataset_items": dataset_items,
        "ads_found": ads_found,
        "lookups_failed": lookups_failed,
        "charge_limit_reached": charge_limit_reached
    });
    println!("TikTok ad details extraction completed successfully");
    println!("Results summary: {summary}");
    Ok(())
}

#[tokio::main]
async fn main() {
    match tokio::time::timeout(ACTOR_TIMEOUT, run_actor()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            eprintln!("Actor failed: {error:#}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!(
                "Actor failed: actor deadline exceeded after {}s",
                ACTOR_TIMEOUT.as_secs()
            );
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{assert_successful_response, failure_item, success_item};
    use crate::request_params::TikTokAdLookup;
    use serde_json::json;

    #[test]
    fn success_row_keeps_normalized_ad_and_request_metadata() {
        let request = TikTokAdLookup {
            url: "https://ads.tiktok.com/business/creativecenter/topads/123/pc/en".to_owned(),
            validation_error: None,
        };
        let response = json!({
            "processed_time": 0.25,
            "cached": true,
            "data": { "id": "123", "title": "Ad title" }
        });
        let row = success_item(response.get("data"), &request, &response, 2);
        assert_eq!(row["ad_id"], "123");
        assert_eq!(row["creative_text"], "Ad title");
        assert_eq!(row["request_url"], request.url);
        assert_eq!(row["request_ad_id"], "123");
        assert_eq!(row["request_index"], 2);
        assert_eq!(row["result_found"], true);
        assert_eq!(row["processed_time"], 0.25);
        assert_eq!(row["cached"], true);
    }

    #[test]
    fn not_found_row_keeps_null_response_metadata() {
        let request = TikTokAdLookup {
            url: "https://ads.tiktok.com/business/creativecenter/topads/123".to_owned(),
            validation_error: None,
        };
        let row = success_item(None, &request, &json!({}), 1);
        assert_eq!(row["result_found"], false);
        assert_eq!(row["processed_time"], serde_json::Value::Null);
        assert_eq!(row["cached"], serde_json::Value::Null);
        assert!(row.get("error_message").is_none());
    }

    #[test]
    fn failed_lookup_row_contains_error_and_null_response_metadata() {
        let request = TikTokAdLookup {
            url: "not-a-url".to_owned(),
            validation_error: Some("A valid TikTok Creative Center ad URL is required".to_owned()),
        };
        let row = failure_item(&request, 1, "lookup failed".to_owned());
        assert_eq!(row["result_found"], false);
        assert_eq!(row["error_message"], "lookup failed");
        assert_eq!(row["request_ad_id"], serde_json::Value::Null);
    }

    #[test]
    fn checks_scrappa_code_and_error_message() {
        assert!(assert_successful_response(&json!({ "code": 0 }), "url").is_ok());
        let error =
            assert_successful_response(&json!({ "code": 1001, "msg": "Unavailable" }), "url")
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("Scrappa TikTok Ads API returned code 1001 for url: Unavailable"));
    }
}
