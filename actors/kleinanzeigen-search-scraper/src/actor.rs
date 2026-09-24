use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};

use crate::{
    apify::{
        affordable_listing_count, is_pay_per_event, push_charged_listings, ApifyClient,
        DatasetBudget, LISTING_RESULT_CHARGE_EVENT,
    },
    config::Config,
    input::{build_search_plan, describe_search_request, js_string},
    response::{build_dataset_item, limit_search_response, select_listings},
    scrappa::{ScrappaClient, ScrappaTimeoutError, SCRAPPA_REQUEST_TIMEOUT},
};

#[derive(Debug)]
pub struct ActorOutput {
    pub value: Value,
    pub status_message: Option<String>,
}

pub async fn run_actor(http: &Client, config: &Config) -> Result<ActorOutput> {
    let api_key = config.scrappa_api_key.as_deref().ok_or_else(|| {
        anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
    })?;
    let apify = ApifyClient::new(http, config);
    let input = apify.get_input().await?;
    let searches = build_search_plan(&input)?;
    println!(
        "Searching Kleinanzeigen for {}",
        describe_search_request(&searches)
    );

    let scrappa = ScrappaClient::new(http, config, api_key);
    let mut responses = Vec::new();
    let mut saved_listings = 0;
    let mut status_message = None;
    let mut dataset_budget = DatasetBudget::default();

    for search in &searches {
        let run = apify.get_run().await?;
        if is_pay_per_event(&run)? && affordable_listing_count(&run, 1, &mut dataset_budget)? == 0 {
            let query = search
                .params
                .get("query")
                .map(js_string)
                .unwrap_or_default();
            let page = search.params.get("page").map(js_string).unwrap_or_default();
            let message = format!(
                "Charge limit reached before fetching Kleinanzeigen query {query} on page {page}."
            );
            println!(
                "{message} {}",
                json!({
                    "event": LISTING_RESULT_CHARGE_EVENT,
                    "query": search.params.get("query"),
                    "page": search.params.get("page"),
                })
            );
            status_message = Some(message);
            break;
        }

        let query = search
            .params
            .get("query")
            .map(js_string)
            .unwrap_or_default();
        let page = search.params.get("page").map(js_string).unwrap_or_default();
        println!("Fetching Kleinanzeigen query {query} on page {page}");

        let response = scrappa.get(&search.params).await?;
        let (listings, source) = select_listings(&response);
        let items = listings
            .iter()
            .map(|listing| build_dataset_item(listing, &search.params, &response))
            .collect::<Vec<_>>();
        let push_result = push_charged_listings(&apify, &items, &mut dataset_budget).await?;
        saved_listings += push_result.saved_count;
        responses.push(json!({
            "index": search.index,
            "request": search.params,
            "listings_saved": push_result.saved_count,
            "response": limit_search_response(&response, push_result.saved_count, source),
        }));

        println!(
            "Found {} listing(s); saved {}",
            items.len(),
            push_result.saved_count
        );
        if push_result.charge_limit_reached || push_result.saved_count < items.len() {
            let message = format!(
                "Charge limit reached after saving {} of {} Kleinanzeigen listing result(s) for query {}.",
                push_result.saved_count,
                items.len(),
                search
                    .params
                    .get("query")
                    .map(js_string)
                    .unwrap_or_default()
            );
            println!(
                "{message} {}",
                json!({
                    "event": LISTING_RESULT_CHARGE_EVENT,
                    "charged_count": push_result.saved_count,
                    "requested_count": items.len(),
                    "saved_count": push_result.saved_count,
                    "query": search.params.get("query"),
                    "page": search.params.get("page"),
                })
            );
            status_message = Some(message);
            break;
        }
    }

    let output = json!({
        "searches_requested": searches.len(),
        "searches_completed": responses.len(),
        "listings_extracted": saved_listings,
        "status_message": status_message,
        "responses": responses,
    });
    apify.put_output(&output).await?;
    println!("Kleinanzeigen search completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "searches_requested": output["searches_requested"],
            "searches_completed": output["searches_completed"],
            "listings_extracted": output["listings_extracted"],
        })
    );
    Ok(ActorOutput {
        value: output,
        status_message,
    })
}

pub fn actor_failure_message(error: &anyhow::Error) -> String {
    let raw_message = error.to_string();
    if error
        .chain()
        .any(|cause| cause.downcast_ref::<ScrappaTimeoutError>().is_some())
    {
        format!(
            "{raw_message}. The Kleinanzeigen search request exceeded the {}s Scrappa API timeout. Try fewer searches, narrower filters, or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        raw_message
    }
}
