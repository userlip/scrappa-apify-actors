use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

use crate::{
    apify::{ActorConfig, ApifyClient},
    charging::{charge_limit_message, BillingState, PROPERTY_RESULT_EVENT},
    request_params::{build_search_requests, describe_search},
    response_utils::{dataset_item, property_listings, search_count},
    scrappa::{ScrappaClient, ScrappaError},
};

pub(crate) async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let http = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| anyhow!("Failed to configure HTTP client: {error}"))?;
    let apify = ApifyClient::new(http.clone(), &config);
    match run_actor(&config, &apify, http).await {
        Ok(()) => Ok(()),
        Err(error) => {
            let message = actor_error_message(&error);
            if let Err(status_error) = apify.set_status_message(&message).await {
                eprintln!("Failed to set Apify terminal status message: {status_error}");
            }
            Err(anyhow!(message))
        }
    }
}

fn actor_error_message(error: &anyhow::Error) -> String {
    if error
        .downcast_ref::<ScrappaError>()
        .is_some_and(ScrappaError::is_timeout)
    {
        format!("{error}. The Redfin request exceeded the 90s Scrappa API timeout. Try fewer homes, fewer batched searches, or run the request again.")
    } else {
        error.to_string()
    }
}

async fn run_actor(config: &ActorConfig, apify: &ApifyClient, http: Client) -> Result<()> {
    let mut billing = match BillingState::from_environment()? {
        Some(billing) => billing,
        None => BillingState::from_run(&apify.get_run().await?)?,
    };

    if config.scrappa_api_key.is_empty() {
        return Err(anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings."));
    }

    let input = apify
        .get_input()
        .await?
        .ok_or_else(|| anyhow!("Input is required"))?;
    let requests = build_search_requests(&input)?;
    let searches_requested = requests.len();
    println!(
        "Running {} Redfin property search request(s)",
        requests.len()
    );

    let scrappa = ScrappaClient::new(
        http,
        config.scrappa_api_base_url.clone(),
        config.scrappa_api_key.clone(),
    );
    let mut total_results = 0;
    let mut status_message: Option<String> = None;

    for request in requests {
        status_message = billing.charge_limit_status(total_results, request.index);
        if let Some(message) = status_message.as_deref() {
            println!(
                "{message} {}",
                json!({
                    "event": PROPERTY_RESULT_EVENT,
                    "searches_requested": searches_requested,
                    "results": total_results,
                    "next_search_index": request.index,
                })
            );
            break;
        }

        println!("Searching Redfin for {}", describe_search(&request.params));
        let response = scrappa.get("/redfin/search", &request.params).await?;
        let properties = property_listings(&response)
            .iter()
            .map(|property| dataset_item(property, &request.params, request.index))
            .collect::<Vec<_>>();

        if properties.is_empty() {
            println!(
                "Search {} returned no Redfin property results {}",
                request.index + 1,
                json!({
                    "search_index": request.index,
                    "api_count": search_count(&response),
                    "request_region_id": request.params.get("region_id"),
                    "request_market": request.params.get("market"),
                })
            );
            continue;
        }

        if billing.is_pay_per_event() {
            let mut saved_for_search = 0;
            for (property_offset, property) in properties.iter().enumerate() {
                if !billing.can_write_property_result() {
                    let message =
                        charge_limit_message(request.index, saved_for_search, properties.len(), 0);
                    println!(
                        "{message} {}",
                        json!({
                            "event": PROPERTY_RESULT_EVENT,
                            "charged_count": 0,
                            "saved_count": saved_for_search,
                            "requested_count": properties.len(),
                            "search_index": request.index,
                        })
                    );
                    status_message = Some(message);
                    break;
                }

                apify.push_dataset_item(property).await?;
                billing.record_dataset_item();

                if billing.should_charge_property_result() {
                    let idempotency_key = format!(
                        "{}-{}-{}-{}",
                        config.actor_run_id, PROPERTY_RESULT_EVENT, request.index, property_offset,
                    );
                    apify
                        .charge_event(PROPERTY_RESULT_EVENT, &idempotency_key)
                        .await?;
                }
                billing.record_property_result_charge();

                let charged_count = 2;
                if charged_count >= 1 {
                    saved_for_search += 1;
                    total_results += 1;
                }

                if billing.charge_limit_reached() {
                    let message = charge_limit_message(
                        request.index,
                        saved_for_search,
                        properties.len(),
                        charged_count,
                    );
                    println!(
                        "{message} {}",
                        json!({
                            "event": PROPERTY_RESULT_EVENT,
                            "charged_count": charged_count,
                            "saved_count": saved_for_search,
                            "requested_count": properties.len(),
                            "search_index": request.index,
                        })
                    );
                    status_message = Some(message);
                    break;
                }
            }
        } else {
            apify.push_dataset_items(&properties).await?;
            total_results += properties.len();
        }

        println!(
            "Search {} returned {} Redfin property result(s) {}",
            request.index + 1,
            properties.len(),
            json!({
                "search_index": request.index,
                "api_count": search_count(&response),
                "request_region_id": request.params.get("region_id"),
                "request_market": request.params.get("market"),
            })
        );

        if status_message.is_some() {
            break;
        }
    }

    if let Some(message) = status_message.as_deref() {
        println!("Redfin property search completed: {message}");
    } else {
        println!("Redfin property search completed successfully");
    }
    println!(
        "Results summary: {}",
        json!({
            "searches": searches_requested,
            "results": total_results,
            "status_message": status_message,
        })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn timeout_errors_keep_the_actor_guidance_suffix() {
        let error = anyhow!(ScrappaError::Timeout { timeout_ms: 90_000 });
        assert_eq!(
            actor_error_message(&error),
            "Scrappa API request timed out after 90000ms. The Redfin request exceeded the 90s Scrappa API timeout. Try fewer homes, fewer batched searches, or run the request again."
        );
    }
}
