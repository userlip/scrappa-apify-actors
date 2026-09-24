use std::collections::BTreeSet;
use std::fmt;

use anyhow::{anyhow, bail, Result};
use chrono::{SecondsFormat, Utc};
use serde_json::{json, Map, Value};

use crate::{
    apify::{ApifyClient, PpeBudget},
    challenge::{
        build_requests, challenge_id, challenge_name, extract_challenge_detail,
        normalize_challenge_detail, ChallengeRequest, RequestType,
    },
    config::{ActorConfig, CHALLENGE_DETAIL_CHARGE_EVENT, DEFAULT_DATASET_ITEM_EVENT},
    scrappa::{challenge_error, ScrappaClient},
};

#[derive(Debug)]
struct EventChargeFailure(anyhow::Error);

impl fmt::Display for EventChargeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Apify event charge could not be confirmed: {}",
            self.0
        )
    }
}

impl std::error::Error for EventChargeFailure {}

pub(crate) fn request_outcome(
    request: &ChallengeRequest,
    status: &str,
    error: Option<String>,
    canonical_id: Option<String>,
) -> Value {
    let mut outcome = json!({
        "request_type": request.request_type.as_str(),
        "request_value": request.value,
        "status": status,
    });
    if let Some(error) = error {
        outcome["error"] = Value::String(error);
    }
    if let Some(canonical_id) = canonical_id {
        outcome["canonical_challenge_id"] = Value::String(canonical_id);
    }
    outcome
}

fn add_not_attempted_outcomes(
    requests: &[ChallengeRequest],
    start_index: usize,
    outcomes: &mut Vec<Value>,
    error: &str,
) {
    outcomes.extend(
        requests[start_index..]
            .iter()
            .map(|request| request_outcome(request, "not_attempted", Some(error.to_owned()), None)),
    );
}

pub(crate) fn result_error(
    challenge: &Map<String, Value>,
    request: &ChallengeRequest,
) -> Option<String> {
    let canonical_id = challenge_id(challenge);
    if canonical_id.is_none() {
        return Some(
            "Scrappa returned a challenge detail without a canonical challenge ID".to_owned(),
        );
    }
    match request.request_type {
        RequestType::ChallengeName => {
            let returned_name = challenge_name(challenge);
            if returned_name
                .as_deref()
                .is_none_or(|name| !name.eq_ignore_ascii_case(&request.value))
            {
                let returned = returned_name.map(Value::String).unwrap_or(Value::Null);
                Some(format!(
                    "Scrappa returned challenge {} but {} was requested",
                    returned,
                    json!(request.value)
                ))
            } else {
                None
            }
        }
        RequestType::ChallengeId if canonical_id.as_deref() != Some(&request.value) => {
            Some(format!(
                "Scrappa returned challenge ID {} but {} was requested",
                json!(canonical_id.unwrap()),
                json!(request.value)
            ))
        }
        RequestType::ChallengeId => None,
    }
}

async fn save_challenge_detail(
    apify: &ApifyClient,
    config: &ActorConfig,
    budget: &mut PpeBudget,
    request: &ChallengeRequest,
    item: &Value,
    canonical_id: &str,
) -> Result<(bool, bool)> {
    if !budget.can_push_dataset_item() {
        return Ok((false, true));
    }
    apify.push_dataset_item(config, item).await?;
    if !budget.is_pay_per_event {
        return Ok((true, false));
    }

    let dataset_charge_count =
        u64::from(budget.event_prices.contains_key(DEFAULT_DATASET_ITEM_EVENT));
    budget.record_successful_charge(DEFAULT_DATASET_ITEM_EVENT, dataset_charge_count);

    let custom_event_configured = budget
        .event_prices
        .contains_key(CHALLENGE_DETAIL_CHARGE_EVENT);
    if custom_event_configured {
        let idempotency_key = format!(
            "{}:{}:{}",
            config.actor_run_id,
            request.request_type.as_str(),
            canonical_id
        );
        let charged_count = apify
            .charge_event(config, 1, &idempotency_key)
            .await
            .map_err(|error| anyhow::Error::new(EventChargeFailure(error)))?;
        budget.record_successful_charge(CHALLENGE_DETAIL_CHARGE_EVENT, charged_count);
        if charged_count != 1 {
            return Ok((false, true));
        }
    } else {
        eprintln!("Warning: attempt to charge unconfigured event '{CHALLENGE_DETAIL_CHARGE_EVENT}' was ignored");
    }

    let charge_limit_reached = budget.result_capacity() == 0;
    Ok((true, charge_limit_reached))
}

pub(crate) async fn run_batch(
    requests: &[ChallengeRequest],
    scrappa: &ScrappaClient,
    apify: &ApifyClient,
    config: &ActorConfig,
    budget: &mut PpeBudget,
) -> Result<Value> {
    let mut outcomes = Vec::new();
    let mut attempted = 0;
    let mut saved = 0;
    let mut charge_limit_reached = false;
    let mut status_message: Option<String> = None;
    let mut resolved_challenge_ids = BTreeSet::new();

    for (index, request) in requests.iter().enumerate() {
        if budget.result_capacity() == 0 {
            charge_limit_reached = true;
            status_message = Some(
                "Charge limit reached before fetching another TikTok challenge detail.".to_owned(),
            );
            add_not_attempted_outcomes(requests, index, &mut outcomes, "Charge limit reached");
            break;
        }

        attempted += 1;
        let outcome = async {
            let response = scrappa.fetch(request).await?;
            if let Some(error) = challenge_error(&response) {
                bail!("{error}");
            }
            let challenge = extract_challenge_detail(&response)
                .ok_or_else(|| anyhow!("Scrappa returned no challenge detail record"))?;
            if let Some(error) = result_error(challenge, request) {
                bail!("{error}");
            }
            let canonical_id = challenge_id(challenge).expect("result validation requires a canonical ID");
            if resolved_challenge_ids.contains(&canonical_id) {
                return Ok((
                    request_outcome(
                        request,
                        "duplicate",
                        Some(format!(
                            "Duplicate canonical challenge ID {canonical_id}; result was not saved or charged"
                        )),
                        Some(canonical_id),
                    ),
                    false,
                    false,
                ));
            }

            let item = normalize_challenge_detail(
                challenge,
                request,
                Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            );
            let (was_saved, limit_reached) =
                save_challenge_detail(apify, config, budget, request, &item, &canonical_id).await?;
            if !was_saved {
                return Ok((
                    request_outcome(request, "failed", Some("Apify did not save a chargeable challenge detail result".to_owned()), None),
                    false,
                    limit_reached,
                ));
            }
            resolved_challenge_ids.insert(canonical_id);
            Ok((request_outcome(request, "saved", None, None), true, limit_reached))
        }
        .await;

        match outcome {
            Ok((outcome, was_saved, limit_reached)) => {
                if was_saved {
                    saved += 1;
                }
                outcomes.push(outcome);
                if limit_reached {
                    charge_limit_reached = true;
                    status_message = Some(if was_saved {
                        "Charge limit reached after saving a TikTok challenge detail.".to_owned()
                    } else {
                        "Charge limit reached while saving a TikTok challenge detail.".to_owned()
                    });
                    add_not_attempted_outcomes(
                        requests,
                        index + 1,
                        &mut outcomes,
                        "Charge limit reached",
                    );
                    break;
                }
            }
            Err(error) => {
                let charge_failed = error.downcast_ref::<EventChargeFailure>().is_some();
                outcomes.push(request_outcome(
                    request,
                    "failed",
                    Some(error.to_string()),
                    None,
                ));
                if charge_failed {
                    status_message = Some(
                        "Batch stopped because Apify did not confirm a result event charge."
                            .to_owned(),
                    );
                    add_not_attempted_outcomes(
                        requests,
                        index + 1,
                        &mut outcomes,
                        "Apify did not confirm the previous result event charge",
                    );
                    break;
                }
            }
        }
    }

    let failed = outcomes
        .iter()
        .filter(|outcome| outcome["status"] == "failed")
        .count();
    Ok(json!({
        "requested": requests.len(),
        "attempted": attempted,
        "succeeded": saved,
        "failed": failed,
        "saved": saved,
        "charge_limit_reached": charge_limit_reached,
        "status_message": status_message,
        "outcomes": outcomes,
    }))
}

pub async fn run_actor() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let apify = ApifyClient::new(&config)?;
    let input = apify
        .get_input(&config)
        .await?
        .ok_or_else(|| anyhow!("At least one TikTok challenge name or challenge ID is required"))?;
    let mut warnings = Vec::new();
    let requests_result = build_requests(&input, &mut warnings);
    for warning in warnings {
        eprintln!("Warning: {warning}");
    }
    let requests = requests_result?;
    let mut budget = apify.run_budget(&config).await?;
    let scrappa = ScrappaClient::new(&config)?;
    let summary = run_batch(&requests, &scrappa, &apify, &config, &mut budget).await?;
    apify.set_output(&config, &summary).await?;
    println!("TikTok challenge details completed: {summary}");
    Ok(())
}
