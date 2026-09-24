use std::{future::Future, pin::Pin};

use anyhow::Result;
use serde_json::Value;

use crate::{
    apify::{ApifyClient, LISTING_DETAIL_RESULT_CHARGE_EVENT, PushDataResult},
    error_utils::error_summary,
    request_params::ListingRequest,
    response_utils::{build_dataset_item, select_listing_detail},
};

pub trait ListingDatasetWriter {
    fn is_pay_per_event(&self) -> bool;

    fn listing_event_capacity(&self) -> usize;

    fn push_data<'a>(
        &'a mut self,
        item: &'a Value,
        request_index: usize,
    ) -> Pin<Box<dyn Future<Output = Result<PushDataResult>> + 'a>>;
}

impl ListingDatasetWriter for ApifyClient {
    fn is_pay_per_event(&self) -> bool {
        ApifyClient::is_pay_per_event(self)
    }

    fn listing_event_capacity(&self) -> usize {
        ApifyClient::listing_event_capacity(self, LISTING_DETAIL_RESULT_CHARGE_EVENT)
    }

    fn push_data<'a>(
        &'a mut self,
        item: &'a Value,
        request_index: usize,
    ) -> Pin<Box<dyn Future<Output = Result<PushDataResult>> + 'a>> {
        Box::pin(ApifyClient::push_data(
            self,
            item,
            LISTING_DETAIL_RESULT_CHARGE_EVENT,
            request_index,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingFailure {
    pub ad_id: String,
    pub error: String,
    pub outcome: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingDetailsProcessingResult {
    pub saved_count: usize,
    pub completed_count: usize,
    pub failures: Vec<ListingFailure>,
    pub status_message: Option<String>,
}

pub fn build_output(listings_requested: usize, result: &ListingDetailsProcessingResult) -> Value {
    serde_json::json!({
        "listings_requested": listings_requested,
        "listings_completed": result.completed_count,
        "listings_saved": result.saved_count,
        "listings_failed": result.failures.len(),
        "status_message": result.status_message,
        "failures": result
            .failures
            .iter()
            .map(|failure| serde_json::json!({
                "ad_id": failure.ad_id,
                "error": failure.error,
                "outcome": failure.outcome,
            }))
            .collect::<Vec<_>>(),
    })
}

pub async fn process_listings<W, F, Fut>(
    writer: &mut W,
    listings: &[ListingRequest],
    mut fetch_listing: F,
    max_results: Option<usize>,
) -> ListingDetailsProcessingResult
where
    W: ListingDatasetWriter,
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Value>>,
{
    let mut result = ListingDetailsProcessingResult {
        saved_count: 0,
        completed_count: 0,
        failures: Vec::new(),
        status_message: None,
    };

    for listing in listings {
        if writer.is_pay_per_event() && writer.listing_event_capacity() == 0 {
            let message = format!(
                "Charge limit reached before fetching Kleinanzeigen listing {}.",
                listing.ad_id
            );
            eprintln!("{message}");
            result.status_message = Some(message);
            break;
        }

        result.completed_count += 1;
        let processed = async {
            let response = fetch_listing(listing.ad_id.clone()).await?;
            let detail = select_listing_detail(&response).ok_or_else(|| {
                anyhow::anyhow!(
                    "Scrappa response did not contain a recognizable listing detail payload"
                )
            })?;
            let item = build_dataset_item(&detail, &listing.ad_id, listing.index);
            writer.push_data(&item, listing.index).await
        }
        .await;

        match processed {
            Ok(save_result) => {
                if save_result.saved {
                    result.saved_count += 1;
                }
                if max_results.is_some_and(|limit| result.saved_count >= limit) {
                    break;
                }
                if !save_result.saved || save_result.event_charge_limit_reached {
                    let message = format!(
                        "Charge limit reached after saving {} Kleinanzeigen listing detail result(s).",
                        result.saved_count
                    );
                    eprintln!("{message}");
                    result.status_message = Some(message);
                    break;
                }
            }
            Err(error) => {
                let message = error_summary(&error.to_string());
                eprintln!("Kleinanzeigen listing {} failed: {message}", listing.ad_id);
                result.failures.push(ListingFailure {
                    ad_id: listing.ad_id.clone(),
                    error: message,
                    outcome: "failed",
                });
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, future::Future, pin::Pin, rc::Rc};

    use anyhow::{Result, anyhow};
    use serde_json::{Value, json};

    use crate::{
        apify::PushDataResult,
        listing_processing::{ListingDatasetWriter, build_output, process_listings},
        request_params::ListingRequest,
    };

    #[derive(Default)]
    struct FakeWriter {
        ppe: bool,
        capacity: usize,
        writes: Vec<Value>,
        save_result: Option<PushDataResult>,
    }

    impl ListingDatasetWriter for FakeWriter {
        fn is_pay_per_event(&self) -> bool {
            self.ppe
        }

        fn listing_event_capacity(&self) -> usize {
            self.capacity
        }

        fn push_data<'a>(
            &'a mut self,
            item: &'a Value,
            _request_index: usize,
        ) -> Pin<Box<dyn Future<Output = Result<PushDataResult>> + 'a>> {
            self.writes.push(item.clone());
            let result = self.save_result.clone().unwrap_or(PushDataResult {
                saved: true,
                event_charge_limit_reached: false,
            });
            Box::pin(async move { Ok(result) })
        }
    }

    fn two_listings() -> Vec<ListingRequest> {
        vec![
            ListingRequest {
                ad_id: "1".into(),
                index: 0,
            },
            ListingRequest {
                ad_id: "2".into(),
                index: 1,
            },
        ]
    }

    #[tokio::test]
    async fn continues_after_a_listing_failure_and_writes_successful_rows() {
        let mut writer = FakeWriter {
            ppe: true,
            capacity: 10,
            ..FakeWriter::default()
        };
        let result = process_listings(
            &mut writer,
            &two_listings(),
            |id| async move {
                if id == "1" {
                    return Err(anyhow!("request failed"));
                }
                Ok(json!({ "data": { "id": id, "title": "Listing" } }))
            },
            None,
        )
        .await;

        assert_eq!(result.completed_count, 2);
        assert_eq!(result.saved_count, 1);
        assert_eq!(result.failures[0].ad_id, "1");
        assert_eq!(writer.writes.len(), 1);
        assert_eq!(writer.writes[0]["request_ad_id"], "2");
        assert_eq!(build_output(2, &result)["listings_failed"], 1);
    }

    #[tokio::test]
    async fn checks_ppe_capacity_before_fetching_and_stops_after_a_short_charge() {
        let mut writer = FakeWriter {
            ppe: true,
            capacity: 0,
            ..FakeWriter::default()
        };
        let fetched = Rc::new(Cell::new(false));
        let fetched_in_future = Rc::clone(&fetched);
        let result = process_listings(
            &mut writer,
            &two_listings(),
            move |_| {
                let fetched = Rc::clone(&fetched_in_future);
                async move {
                    fetched.set(true);
                    Ok(json!({ "data": { "id": "1" } }))
                }
            },
            None,
        )
        .await;
        assert!(!fetched.get());
        assert_eq!(result.completed_count, 0);
        assert!(
            result
                .status_message
                .as_deref()
                .unwrap()
                .contains("before fetching")
        );

        writer.capacity = 10;
        writer.save_result = Some(PushDataResult {
            saved: false,
            event_charge_limit_reached: true,
        });
        let result = process_listings(
            &mut writer,
            &two_listings(),
            |id| async move { Ok(json!({ "data": { "id": id } })) },
            None,
        )
        .await;
        assert_eq!(result.saved_count, 0);
        assert!(
            result
                .status_message
                .as_deref()
                .unwrap()
                .contains("after saving 0")
        );
        assert_eq!(writer.writes.len(), 1);
    }

    #[tokio::test]
    async fn malformed_detail_is_not_written_or_counted_as_a_success() {
        let mut writer = FakeWriter {
            ppe: true,
            capacity: 10,
            ..FakeWriter::default()
        };
        let result = process_listings(
            &mut writer,
            &two_listings(),
            |_| async { Ok(json!({ "data": { "success": false, "message": "removed" } })) },
            None,
        )
        .await;
        assert_eq!(result.saved_count, 0);
        assert_eq!(result.failures.len(), 2);
        assert!(writer.writes.is_empty());
    }

    #[tokio::test]
    async fn stops_at_the_first_successful_discovery_result() {
        let mut writer = FakeWriter {
            ppe: true,
            capacity: 10,
            ..FakeWriter::default()
        };
        let result = process_listings(
            &mut writer,
            &two_listings(),
            |id| async move { Ok(json!({ "data": { "id": id } })) },
            Some(1),
        )
        .await;
        assert_eq!(result.saved_count, 1);
        assert_eq!(result.completed_count, 1);
        assert_eq!(writer.writes.len(), 1);
    }
}
