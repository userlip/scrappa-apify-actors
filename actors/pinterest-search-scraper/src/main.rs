mod api_url;
mod apify_client;
mod charging;
mod pinterest_input;
mod pinterest_response;
mod scrappa_client;

use std::process::ExitCode;
use std::time::Duration;

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::json;

use apify_client::{ApifyClient, Config};
use charging::{push_charged_pins, ChargeBudget, PIN_RESULT_CHARGE_EVENT};
use pinterest_input::{
    build_pinterest_search_plan, cap_pinterest_search_params, describe_pinterest_search_request,
    PinterestSearchParams,
};
use pinterest_response::{
    limit_pinterest_search_response, nullish, pinterest_dataset_item, pinterest_next_bookmark,
    select_pinterest_pins,
};
use scrappa_client::{ScrappaClient, ScrappaTimeoutError, SCRAPPA_REQUEST_TIMEOUT};

async fn execute_actor(
    config: &Config,
    apify: &ApifyClient,
    budget: &mut ChargeBudget,
) -> Result<Option<String>> {
    let api_key = config.scrappa_api_key.as_ref().ok_or_else(|| {
        anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
    })?;
    let input = apify.get_input().await?.unwrap_or_else(|| json!({}));
    let input = input.as_object().cloned().unwrap_or_default();
    let plan = build_pinterest_search_plan(&input)?;
    println!(
        "Searching Pinterest for {}",
        describe_pinterest_search_request(&plan)
    );

    let scrappa = ScrappaClient::new(config.scrappa_api_base.clone(), api_key.clone())?;
    let mut responses = Vec::new();
    let mut query_summaries = Vec::new();
    let mut searches_fetched = 0usize;
    let mut extracted_pins = 0usize;
    let mut saved_pins = 0usize;
    let mut status_message = None;

    for query in &plan.queries {
        let chargeable_pin_capacity = budget.chargeable_pin_capacity();
        if chargeable_pin_capacity == 0 {
            let message =
                format!("Charge limit reached before fetching Pinterest pins for \"{query}\".");
            eprintln!(
                "{message} {}",
                json!({"event": PIN_RESULT_CHARGE_EVENT, "query": query})
            );
            status_message = Some(message);
            break;
        }

        let params = PinterestSearchParams {
            query: query.clone(),
            limit: plan.limit,
            bookmark: plan.bookmark.clone(),
        };
        let fetch_params = cap_pinterest_search_params(&params, chargeable_pin_capacity);
        println!(
            "Fetching Pinterest pins for \"{}\" with limit {}",
            query, fetch_params.fetch_limit
        );
        let response = scrappa.pinterest_search(&fetch_params.params).await?;
        searches_fetched += 1;

        let selection = select_pinterest_pins(&response);
        let pins = selection.pins;
        let source = selection.source;
        extracted_pins += pins.len();
        let items = pins
            .iter()
            .map(|pin| pinterest_dataset_item(pin, &fetch_params.params, &response))
            .collect::<Result<Vec<_>>>()?;
        let push_result = push_charged_pins(apify, budget, &items, query).await?;
        saved_pins += push_result.saved_count;
        responses.push(limit_pinterest_search_response(
            &response,
            push_result.saved_count,
            source,
        ));
        query_summaries.push(json!({
            "query": query,
            "requested_limit": fetch_params.requested_limit,
            "fetch_limit": fetch_params.fetch_limit,
            "request_bookmark": fetch_params.params.bookmark,
            "count": nullish(response.get("count")),
            "results_count": response.get("results_count")
                .filter(|value| !value.is_null())
                .cloned()
                .unwrap_or_else(|| json!(pins.len())),
            "pins_extracted": pins.len(),
            "pins_saved": push_result.saved_count,
            "nextBookmark": pinterest_next_bookmark(&response),
        }));
        println!(
            "Found {} Pinterest pin result(s) for \"{}\"; saved {}",
            pins.len(),
            query,
            push_result.saved_count
        );
        if push_result.status_message.is_some() {
            status_message = push_result.status_message;
            break;
        }
    }

    let output = json!({
        "request": {
            "queries": plan.queries,
            "limit": plan.limit,
            "bookmark": plan.bookmark,
        },
        "searches_fetched": searches_fetched,
        "responses_saved": responses.len(),
        "pins_extracted": extracted_pins,
        "pins_saved": saved_pins,
        "status_message": status_message,
        "query_summaries": query_summaries,
        "responses": responses,
    });
    apify.put_output(&output).await?;

    println!("Pinterest search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "searches_fetched": searches_fetched,
            "responses_saved": responses.len(),
            "pins_extracted": extracted_pins,
            "pins_saved": saved_pins,
            "queries": plan.queries.len(),
        })
    );
    Ok(status_message)
}

fn actor_failure_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if error.downcast_ref::<ScrappaTimeoutError>().is_some() {
        format!(
            "{message}. The Pinterest search request exceeded the {}s Scrappa API timeout. Try fewer queries, a lower limit, or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let apify = ApifyClient::new(Client::new(), &config);
    let run = apify.get_actor_run().await?;
    let mut budget = ChargeBudget::from_actor_run(&run)?;
    let result = execute_actor(&config, &apify, &mut budget).await;

    match result {
        Ok(Some(message)) => {
            match tokio::time::timeout(Duration::from_secs(1), apify.set_status_message(&message))
                .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    eprintln!("Warning: failed to set the final status message: {error}")
                }
                Err(_) => eprintln!("Warning: setting the final status message timed out"),
            }
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(error) => {
            let message = actor_failure_message(&error);
            match tokio::time::timeout(Duration::from_secs(1), apify.set_status_message(&message))
                .await
            {
                Ok(Ok(())) => {}
                Ok(Err(status_error)) => {
                    eprintln!("Warning: failed to set the failure status message: {status_error}")
                }
                Err(_) => eprintln!("Warning: setting the failure status message timed out"),
            }
            Err(anyhow!(message))
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests;
