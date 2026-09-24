use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::{
    apify_client::{ActorClient, SCRAPPA_REQUEST_TIMEOUT},
    challenges::{
        build_search_requests, extract_challenges, format_lookup, normalized_challenge,
        validate_scrappa_response, SearchRequest,
    },
    charge_budget::ChargeBudget,
    CHALLENGE_RESULT_CHARGE_EVENT,
};

fn output_rows(challenges: &[Value], request: &SearchRequest) -> Result<Vec<Value>> {
    challenges
        .iter()
        .map(|challenge| normalized_challenge(challenge, request))
        .collect()
}

pub async fn run_actor(client: &ActorClient) -> Result<Value> {
    let run = client.get_run_pricing().await?;
    let mut budget = ChargeBudget::from_run(&run)?;

    let input = client
        .get_input()
        .await?
        .ok_or_else(|| anyhow!("At least one TikTok challenge search keyword is required"))?;
    let (requests, warnings) = build_search_requests(&input)?;
    for warning in warnings {
        eprintln!("Warning: {warning}");
    }
    println!(
        "Searching TikTok challenges for: {}",
        format_lookup(&requests)
    );

    let mut results = Vec::new();
    let mut saved_challenges = 0_usize;
    let mut status_message = None;

    for request in &requests {
        if budget.chargeable_event_capacity(CHALLENGE_RESULT_CHARGE_EVENT) == 0 {
            let message = format!(
                "Charge limit reached before fetching TikTok challenge keyword {}.",
                request.keyword
            );
            println!("{message}");
            status_message = Some(message);
            break;
        }

        println!(
            "Searching TikTok challenges for keyword: {}",
            request.keyword
        );
        let response = client.search_challenges(request).await?;
        validate_scrappa_response(&response)?;
        let challenges = extract_challenges(response.get("data"));
        let rows = output_rows(&challenges, request)?;

        let (saved_count, charge_limit_reached) = if rows.is_empty() {
            (0, false)
        } else if !budget.is_pay_per_event {
            client.store_dataset_items(&rows).await?;
            (rows.len(), false)
        } else {
            let item_count = budget.item_count_to_push(rows.len(), CHALLENGE_RESULT_CHARGE_EVENT);
            if item_count == 0 {
                (0, true)
            } else {
                client.store_dataset_items(&rows[..item_count]).await?;
                let charged_count = budget.charged_count(item_count, CHALLENGE_RESULT_CHARGE_EVENT);
                if budget.is_configured_event(CHALLENGE_RESULT_CHARGE_EVENT) {
                    client
                        .charge_event(CHALLENGE_RESULT_CHARGE_EVENT, charged_count)
                        .await?;
                }
                budget.apply_charges(CHALLENGE_RESULT_CHARGE_EVENT, charged_count);
                (
                    charged_count.min(rows.len()),
                    budget.event_charge_limit_reached(CHALLENGE_RESULT_CHARGE_EVENT),
                )
            }
        };

        saved_challenges += saved_count;
        let processed_time = response
            .get("processed_time")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null);
        results.push(json!({
            "request_keyword": request.keyword,
            "challenges_returned": challenges.len(),
            "challenges_saved": saved_count,
            "processed_time": processed_time,
            "charge_limit_reached": charge_limit_reached,
        }));

        println!(
            "Found {} challenge(s); saved {} for keyword: {}",
            challenges.len(),
            saved_count,
            request.keyword
        );

        if charge_limit_reached {
            let message = format!(
                "Charge limit reached after saving {saved_count} of {} TikTok challenge result(s) for keyword {}.",
                challenges.len(),
                request.keyword
            );
            println!("{message}");
            status_message = Some(message);
            break;
        }
    }

    let output = json!({
        "keywords_requested": requests.len(),
        "keywords_completed": results.len(),
        "challenges_extracted": saved_challenges,
        "status_message": status_message,
        "results": results,
    });
    client.set_output(&output).await?;
    println!("TikTok challenge search completed successfully");
    println!("Results summary: {}", output);
    Ok(output)
}

pub fn actor_error_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if message.contains("timed out") {
        format!(
            "{message}. The TikTok challenge search request exceeded the {}s Scrappa API timeout. Try a more specific keyword or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_scrappa_deadline_failure_like_the_node_actor() {
        let error = anyhow::anyhow!("Scrappa API request timed out after 60000ms");
        assert_eq!(
            actor_error_message(&error),
            "Scrappa API request timed out after 60000ms. The TikTok challenge search request exceeded the 60s Scrappa API timeout. Try a more specific keyword or run the request again."
        );
    }
}
