mod apify;
mod request_params;
mod response_utils;
mod runtime_config;
mod scrappa;

use std::{env, process};

use anyhow::{Context, Result, anyhow};
use request_params::{build_related_params, describe_related_request, should_include_autocomplete};
use runtime_config::{
    AUTOCOMPLETE_MAX_ATTEMPTS, RELATED_MAX_ATTEMPTS, autocomplete_request_timeout,
    related_max_retry_delay, related_request_timeout,
};
use serde_json::{Map, Value, json};
use url::Url;

const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";

fn scrappa_api_key() -> Result<String> {
    env::var("SCRAPPA_API_KEY")
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
        })
}

fn scrappa_api_base_url() -> Result<Url> {
    let value = env::var("SCRAPPA_API_BASE_URL").unwrap_or_else(|_| SCRAPPA_API_DEFAULT.to_owned());
    Url::parse(&value).context("SCRAPPA_API_BASE_URL must be a valid absolute URL")
}

fn output_record(response: &Value, items: &[Value], autocomplete: &AutocompleteSummary) -> Value {
    let related_query_count = items
        .iter()
        .filter(|item| item.get("result_kind").and_then(Value::as_str) == Some("query"))
        .count();
    let related_topic_count = items
        .iter()
        .filter(|item| item.get("result_kind").and_then(Value::as_str) == Some("topic"))
        .count();
    json!({
        "search_parameters": response.get("search_parameters").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "related_query_count": related_query_count,
        "related_topic_count": related_topic_count,
        "response_time_ms": response.get("response_time_ms").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "autocomplete": autocomplete.response,
        "autocomplete_error": autocomplete.error,
        "raw_response": response,
    })
}

struct AutocompleteSummary {
    response: Value,
    error: Value,
}

async fn fetch_autocomplete_summary(
    client: &scrappa::ScrappaClient,
    params: &Map<String, Value>,
) -> AutocompleteSummary {
    match client
        .get_json(
            "/google-trends/autocomplete",
            &request_params::build_autocomplete_params(params),
            scrappa::RetryPolicy {
                timeout: autocomplete_request_timeout(),
                attempts: AUTOCOMPLETE_MAX_ATTEMPTS,
                max_retry_delay: std::time::Duration::ZERO,
            },
        )
        .await
    {
        Ok(response) => AutocompleteSummary {
            response,
            error: Value::Null,
        },
        Err(error) => {
            let message = error.to_string();
            println!("Google Trends autocomplete summary failed: {message}");
            AutocompleteSummary {
                response: Value::Null,
                error: Value::String(message),
            }
        }
    }
}

async fn run() -> Result<()> {
    let api_key = scrappa_api_key()?;
    let apify = apify::ApifyClient::new(apify::ApifyConfig::from_env()?)?;
    let input = apify
        .get_input()
        .await?
        .filter(|input| !input.is_null())
        .ok_or_else(|| anyhow!("Input is required"))?;
    let params = build_related_params(&input).map_err(anyhow::Error::msg)?;
    let include_autocomplete = should_include_autocomplete(&input).map_err(anyhow::Error::msg)?;

    println!(
        "Fetching Google Trends related queries for {}",
        describe_related_request(&params)
    );
    let scrappa = scrappa::ScrappaClient::new(api_key, scrappa_api_base_url()?)?;
    let response = scrappa
        .get_json(
            "/google-trends/related",
            &params,
            scrappa::RetryPolicy {
                timeout: related_request_timeout(),
                attempts: RELATED_MAX_ATTEMPTS,
                max_retry_delay: related_max_retry_delay(),
            },
        )
        .await?;
    let dataset_items = response_utils::build_dataset_items(&response, &params);

    if dataset_items.is_empty() {
        println!("No Google Trends related queries or topics found for this request");
    } else {
        let charge_result = apify.push_dataset_items(&dataset_items).await?;
        if charge_result.event_charge_limit_reached
            && charge_result.charged_count < dataset_items.len()
        {
            let status_message =
                "Charge limit reached before saving all Google Trends related query results.";
            println!(
                "{status_message} {}",
                json!({
                    "event":"related-result",
                    "charged_count":charge_result.charged_count,
                    "requested_count":dataset_items.len()
                })
            );
            apify.set_terminal_status_message(status_message).await?;
            return Ok(());
        }
    }

    let autocomplete = if include_autocomplete {
        fetch_autocomplete_summary(&scrappa, &params).await
    } else {
        AutocompleteSummary {
            response: Value::Null,
            error: Value::Null,
        }
    };
    let output = output_record(&response, &dataset_items, &autocomplete);
    apify.set_output(&output).await?;

    println!("Google Trends related queries scraping completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "related_results":dataset_items.len(),
            "autocomplete_included":include_autocomplete,
            "autocomplete_succeeded":if include_autocomplete { Some(autocomplete.error.is_null()) } else { None },
            "response_time_ms":response.get("response_time_ms").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null)
        })
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {}", actor_error_message(&error));
        process::exit(1);
    }
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error
        .downcast_ref::<scrappa::ScrappaTimeoutError>()
        .is_some()
    {
        format!(
            "{error}. The Google Trends related queries request exceeded the {}s Scrappa API timeout. Try a shorter time range, a more specific keyword, or run the request again.",
            runtime_config::RELATED_REQUEST_TIMEOUT_MS / 1_000
        )
    } else {
        error.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn output_matches_summary_contract() {
        let response = json!({"search_parameters":{"keyword":"coffee"}, "response_time_ms":623, "related_queries":[]});
        let items = vec![
            json!({"result_kind":"query"}),
            json!({"result_kind":"topic"}),
        ];
        let autocomplete = AutocompleteSummary {
            response: json!({"suggestions":["coffee shop"]}),
            error: Value::Null,
        };
        assert_eq!(
            output_record(&response, &items, &autocomplete),
            json!({
                "search_parameters":{"keyword":"coffee"},
                "related_query_count":1,
                "related_topic_count":1,
                "response_time_ms":623,
                "autocomplete":{"suggestions":["coffee shop"]},
                "autocomplete_error":null,
                "raw_response":response
            })
        );
    }

    #[test]
    fn timeout_error_keeps_related_request_context() {
        let error = anyhow::Error::new(scrappa::ScrappaTimeoutError::new(30_000));
        let message = actor_error_message(&error);
        assert_eq!(
            message,
            "Scrappa API request timed out after 30000ms. The Google Trends related queries request exceeded the 30s Scrappa API timeout. Try a shorter time range, a more specific keyword, or run the request again."
        );
    }
}
