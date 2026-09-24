use crate::{
    apify::{ActorPricing, ApifyClient},
    input::PriceInsightsRequest,
    output::build_price_insight_item,
    scrappa::{ScrappaClient, ScrappaFailure},
};
use anyhow::{Context, Result};
use serde_json::Value;
use tokio::task::JoinSet;

const REQUEST_CONCURRENCY: usize = 10;

pub struct BatchFailure {
    pub location: String,
    pub message: String,
    #[allow(dead_code)]
    pub status: Option<u16>,
}

pub struct BatchResult {
    pub succeeded: usize,
    pub failures: Vec<BatchFailure>,
    pub charge_limit_reached: bool,
}

enum FetchOutcome {
    Item(Value),
    Failure(BatchFailure),
}

pub async fn run_price_insights_batch(
    requests: &[PriceInsightsRequest],
    scrappa: &ScrappaClient,
    apify: &ApifyClient,
    pricing: &mut ActorPricing,
) -> Result<BatchResult> {
    let is_pay_per_event = pricing.is_pay_per_event();
    let mut failures = Vec::new();
    let mut succeeded = 0;

    for batch in requests.chunks(REQUEST_CONCURRENCY) {
        let mut pending = JoinSet::new();
        for request in batch.iter().cloned() {
            let scrappa = scrappa.clone();
            pending.spawn(async move {
                let outcome = fetch_price_insight(&scrappa, &request).await;
                (request, outcome)
            });
        }

        let mut outcomes = Vec::with_capacity(batch.len());
        while let Some(outcome) = pending.join_next().await {
            outcomes.push(outcome.context("Price-insights request task failed")?);
        }
        outcomes.sort_by_key(|(request, _)| request.index);

        for (request, outcome) in outcomes {
            let item = match outcome {
                FetchOutcome::Item(item) => item,
                FetchOutcome::Failure(failure) => {
                    failures.push(failure);
                    continue;
                }
            };

            let push_result = apify
                .push_dataset_item(&item, request.index, pricing)
                .await?;
            if is_pay_per_event && push_result.charged_count < 1 {
                if !push_result.event_charge_limit_reached {
                    failures.push(BatchFailure {
                        location: request.location,
                        message: "Apify did not confirm a charged dataset write".to_owned(),
                        status: None,
                    });
                }
                return Ok(BatchResult {
                    succeeded,
                    failures,
                    charge_limit_reached: push_result.event_charge_limit_reached,
                });
            }

            if push_result.charged_count >= 1 || !is_pay_per_event {
                succeeded += 1;
            }
            if push_result.event_charge_limit_reached {
                return Ok(BatchResult {
                    succeeded,
                    failures,
                    charge_limit_reached: true,
                });
            }
        }
    }

    Ok(BatchResult {
        succeeded,
        failures,
        charge_limit_reached: false,
    })
}

async fn fetch_price_insight(
    scrappa: &ScrappaClient,
    request: &PriceInsightsRequest,
) -> FetchOutcome {
    let response = match scrappa.get_price_insights(&request.location).await {
        Ok(response) => response,
        Err(error) => return FetchOutcome::Failure(failure_from_scrappa(request, error)),
    };

    match build_price_insight_item(&response, request) {
        Some(item) => FetchOutcome::Item(item),
        None => FetchOutcome::Failure(BatchFailure {
            location: request.location.clone(),
            message: "Scrappa returned an incomplete price-insights snapshot".to_owned(),
            status: None,
        }),
    }
}

fn failure_from_scrappa(request: &PriceInsightsRequest, error: ScrappaFailure) -> BatchFailure {
    BatchFailure {
        location: request.location.clone(),
        message: error.message,
        status: error.status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrappa::ScrappaFailure;

    #[test]
    fn api_failures_keep_the_requested_location_and_http_status() {
        let request = PriceInsightsRequest {
            location: "Nowhere".to_owned(),
            index: 0,
        };
        let failure = failure_from_scrappa(
            &request,
            ScrappaFailure {
                message: "Location not found".to_owned(),
                status: Some(404),
                retryable: false,
                retry_message: None,
            },
        );

        assert_eq!(failure.location, "Nowhere");
        assert_eq!(failure.message, "Location not found");
        assert_eq!(failure.status, Some(404));
    }
}
