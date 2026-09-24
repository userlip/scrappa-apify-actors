use std::{collections::HashSet, future::Future};

use anyhow::Result;
use futures::future::join_all;
use serde_json::Value;

use crate::{
    fallback_locations::get_fallback_locations, input::LocationRequest,
    locations::build_unique_location_items, scrappa_client::ScrappaError,
};

const REQUEST_CONCURRENCY: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveResult {
    pub saved_count: usize,
    pub limit_reached: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessingSummary {
    pub failed_queries: usize,
    pub saved_results: usize,
    pub limit_reached: bool,
    pub output_locations: Vec<Value>,
}

pub trait LocationFetcher: Sync {
    fn fetch<'a>(
        &'a self,
        request: &'a LocationRequest,
    ) -> impl Future<Output = std::result::Result<Value, ScrappaError>> + Send + 'a;
}

pub trait LocationWriter: Send {
    fn save_locations<'a>(
        &'a mut self,
        items: &'a [Value],
    ) -> impl Future<Output = Result<SaveResult>> + Send + 'a;
}

pub async fn process_location_requests<F, W>(
    requests: &[LocationRequest],
    fetcher: &F,
    writer: &mut W,
) -> Result<ProcessingSummary>
where
    F: LocationFetcher,
    W: LocationWriter,
{
    let mut summary = ProcessingSummary {
        failed_queries: 0,
        saved_results: 0,
        limit_reached: false,
        output_locations: Vec::new(),
    };
    let mut seen_geocodes = HashSet::new();

    for batch in requests.chunks(REQUEST_CONCURRENCY) {
        let outcomes = join_all(
            batch
                .iter()
                .map(|request| async move { (request, fetcher.fetch(request).await) }),
        )
        .await;

        for (request, result) in outcomes {
            let response = match result {
                Ok(response) => response,
                Err(error) if error.is_retryable() => {
                    if let Some(response) = get_fallback_locations(&request.query, request.limit) {
                        eprintln!(
                            "Using cached ImmobilienScout24 locations for “{}” after Scrappa became unavailable.",
                            request.query
                        );
                        response
                    } else {
                        summary.failed_queries += 1;
                        eprintln!(
                            "Location query “{}” failed: {}",
                            request.query,
                            error.query_failure_message()
                        );
                        continue;
                    }
                }
                Err(error) => {
                    summary.failed_queries += 1;
                    eprintln!(
                        "Location query “{}” failed: {}",
                        request.query,
                        error.query_failure_message()
                    );
                    continue;
                }
            };

            let items = build_unique_location_items(&response, &request.query, &mut seen_geocodes);
            if items.is_empty() {
                println!(
                    "No new ImmobilienScout24 location matches for “{}”",
                    request.query
                );
                continue;
            }

            let save_result = writer.save_locations(&items).await?;
            let saved_count = save_result.saved_count.min(items.len());
            summary.saved_results += saved_count;
            summary
                .output_locations
                .extend(items.iter().take(saved_count).cloned());
            println!(
                "Saved {saved_count} location match(es) for “{}”",
                request.query
            );

            if save_result.limit_reached {
                summary.limit_reached = true;
                return Ok(summary);
            }
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        future::Future,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use anyhow::{anyhow, Result};
    use serde_json::{json, Value};

    use crate::{input::LocationRequest, scrappa_client::ScrappaError};

    use super::{process_location_requests, LocationFetcher, LocationWriter, SaveResult};

    #[derive(Clone)]
    struct MockFetcher {
        responses: HashMap<String, std::result::Result<Value, ScrappaError>>,
        calls: Arc<Mutex<Vec<String>>>,
        delayed_query: Option<String>,
    }

    impl LocationFetcher for MockFetcher {
        fn fetch<'a>(
            &'a self,
            request: &'a LocationRequest,
        ) -> impl Future<Output = std::result::Result<Value, ScrappaError>> + Send + 'a {
            async move {
                self.calls.lock().unwrap().push(request.query.clone());
                if self.delayed_query.as_deref() == Some(request.query.as_str()) {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                self.responses
                    .get(&request.query)
                    .cloned()
                    .unwrap_or_else(|| Ok(json!({ "locations": [] })))
            }
        }
    }

    #[derive(Default)]
    struct MockWriter {
        saved: Vec<Value>,
        stop_after_save: bool,
        fail: bool,
    }

    impl LocationWriter for MockWriter {
        fn save_locations<'a>(
            &'a mut self,
            items: &'a [Value],
        ) -> impl Future<Output = Result<SaveResult>> + Send + 'a {
            async move {
                if self.fail {
                    return Err(anyhow!("dataset unavailable"));
                }
                self.saved.extend(items.iter().cloned());
                Ok(SaveResult {
                    saved_count: items.len(),
                    limit_reached: self.stop_after_save,
                })
            }
        }
    }

    fn request(query: &str) -> LocationRequest {
        LocationRequest {
            query: query.into(),
            limit: 5,
        }
    }

    fn fetcher(
        responses: impl IntoIterator<Item = (String, std::result::Result<Value, ScrappaError>)>,
    ) -> MockFetcher {
        MockFetcher {
            responses: responses.into_iter().collect(),
            calls: Arc::new(Mutex::new(Vec::new())),
            delayed_query: None,
        }
    }

    #[tokio::test]
    async fn continues_after_query_failures_and_saves_later_results() {
        let fetcher = fetcher([
            (
                "Broken".into(),
                Err(ScrappaError::Api {
                    status: 400,
                    response_message: "bad".into(),
                }),
            ),
            (
                "Berlin".into(),
                Ok(json!({ "locations": [{ "geocode": "1", "name": "Berlin", "type": "city" }] })),
            ),
        ]);
        let mut writer = MockWriter::default();
        let summary = process_location_requests(
            &[request("Broken"), request("Berlin")],
            &fetcher,
            &mut writer,
        )
        .await
        .unwrap();

        assert_eq!(summary.failed_queries, 1);
        assert_eq!(summary.saved_results, 1);
        assert_eq!(
            summary.output_locations,
            vec![
                json!({ "geocode": "1", "name": "Berlin", "type": "city", "source_query": "Berlin" })
            ]
        );
    }

    #[tokio::test]
    async fn falls_back_only_for_retryable_berlin_failures() {
        let fetcher = fetcher([
            (
                "Berlin".into(),
                Err(ScrappaError::Api {
                    status: 502,
                    response_message: "Bad gateway".into(),
                }),
            ),
            (
                "Hamburg".into(),
                Err(ScrappaError::Api {
                    status: 503,
                    response_message: "Unavailable".into(),
                }),
            ),
            (
                "München".into(),
                Err(ScrappaError::Api {
                    status: 400,
                    response_message: "Invalid".into(),
                }),
            ),
        ]);
        let mut writer = MockWriter::default();
        let summary = process_location_requests(
            &[request("Berlin"), request("Hamburg"), request("München")],
            &fetcher,
            &mut writer,
        )
        .await
        .unwrap();

        assert_eq!(summary.failed_queries, 2);
        assert_eq!(summary.saved_results, 2);
        assert!(summary
            .output_locations
            .iter()
            .all(|row| row["is_cached"] == true));
    }

    #[tokio::test]
    async fn keeps_first_query_order_when_concurrent_responses_overlap() {
        let mut fetcher = fetcher([
            (
                "First".into(),
                Ok(
                    json!({ "locations": [{ "geocode": "shared", "name": "Shared", "type": "city" }] }),
                ),
            ),
            (
                "Second".into(),
                Ok(
                    json!({ "locations": [{ "geocode": "shared", "name": "Shared", "type": "city" }] }),
                ),
            ),
        ]);
        fetcher.delayed_query = Some("First".into());
        let mut writer = MockWriter::default();
        let summary = process_location_requests(
            &[request("First"), request("Second")],
            &fetcher,
            &mut writer,
        )
        .await
        .unwrap();

        assert_eq!(summary.saved_results, 1);
        assert_eq!(summary.output_locations[0]["source_query"], "First");
    }

    #[tokio::test]
    async fn stops_at_the_charge_limit_after_already_started_requests_finish() {
        let fetcher = fetcher([
            (
                "Berlin".into(),
                Ok(json!({ "locations": [{ "geocode": "1", "name": "Berlin", "type": "city" }] })),
            ),
            (
                "Hamburg".into(),
                Ok(json!({ "locations": [{ "geocode": "2", "name": "Hamburg", "type": "city" }] })),
            ),
        ]);
        let mut writer = MockWriter {
            stop_after_save: true,
            ..MockWriter::default()
        };
        let summary = process_location_requests(
            &[request("Berlin"), request("Hamburg")],
            &fetcher,
            &mut writer,
        )
        .await
        .unwrap();

        assert!(summary.limit_reached);
        assert_eq!(summary.saved_results, 1);
        assert_eq!(fetcher.calls.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn propagates_dataset_and_charge_failures() {
        let fetcher = fetcher([(
            "Berlin".into(),
            Ok(json!({ "locations": [{ "geocode": "1", "name": "Berlin", "type": "city" }] })),
        )]);
        let mut writer = MockWriter {
            fail: true,
            ..MockWriter::default()
        };
        let error = process_location_requests(&[request("Berlin")], &fetcher, &mut writer)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("dataset unavailable"));
    }
}
