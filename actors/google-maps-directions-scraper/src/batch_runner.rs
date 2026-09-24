use std::time::{Duration, Instant};

use anyhow::Result;
use serde_json::{json, Value};

use crate::{request_params::DirectionsRequest, response_utils::build_directions_dataset_rows};

pub const MAX_BATCH_DURATION: Duration = Duration::from_secs(240);

pub trait DirectionsClient {
    async fn get_directions(&self, request: &DirectionsRequest, deadline: Instant)
        -> Result<Value>;
}

pub trait RouteWriter {
    fn can_save(&self) -> bool;
    async fn save(&mut self, item: &Value) -> Result<RouteSaveResult>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchFailure {
    pub request_index: usize,
    pub origin: String,
    pub destination: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BatchResult {
    pub requested: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub alternatives_saved: usize,
    pub charged: usize,
    pub failures: Vec<BatchFailure>,
    pub charge_limit_reached: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RouteSaveResult {
    pub saved: bool,
    pub charged_count: usize,
    pub charge_limit_reached: bool,
}

pub async fn run_directions_batch<C, W>(
    requests: &[DirectionsRequest],
    client: &C,
    writer: &mut W,
    max_duration: Duration,
) -> BatchResult
where
    C: DirectionsClient,
    W: RouteWriter,
{
    let mut result = BatchResult {
        requested: requests.len(),
        ..BatchResult::default()
    };
    let deadline = Instant::now() + max_duration;

    for (request_offset, request) in requests.iter().enumerate() {
        if !writer.can_save() {
            result.charge_limit_reached = true;
            break;
        }
        if Instant::now() >= deadline {
            result.failures.extend(
                requests[request_offset..]
                    .iter()
                    .map(|request| BatchFailure {
                        request_index: request.index,
                        origin: request.origin.clone(),
                        destination: request.destination.clone(),
                        message: "Batch deadline reached before this route could be processed"
                            .to_owned(),
                    }),
            );
            break;
        }

        let response = match client.get_directions(request, deadline).await {
            Ok(response) => response,
            Err(error) => {
                result.failures.push(failure(request, error.to_string()));
                continue;
            }
        };
        let rows = match build_directions_dataset_rows(&response, request) {
            Ok(rows) => rows,
            Err(error) => {
                result.failures.push(failure(request, error.to_string()));
                continue;
            }
        };

        let mut request_saved = 0;
        let mut save_error = None;
        for row in rows {
            let saved = match writer.save(&row).await {
                Ok(saved) => saved,
                Err(error) => {
                    save_error = Some(error.to_string());
                    break;
                }
            };

            if saved.saved {
                request_saved += 1;
                result.alternatives_saved += 1;
                result.charged += saved.charged_count;
            }
            if saved.charge_limit_reached {
                result.succeeded += usize::from(request_saved > 0);
                result.charge_limit_reached = true;
                result.failed = result.failures.len();
                return result;
            }
        }

        if let Some(message) = save_error {
            result.failures.push(failure(request, message));
            continue;
        }
        if request_saved > 0 {
            result.succeeded += 1;
        }
    }

    result.failed = result.failures.len();
    result
}

fn failure(request: &DirectionsRequest, message: String) -> BatchFailure {
    BatchFailure {
        request_index: request.index,
        origin: request.origin.clone(),
        destination: request.destination.clone(),
        message,
    }
}

impl BatchResult {
    pub fn to_json(&self) -> Value {
        json!({
            "requested": self.requested,
            "succeeded": self.succeeded,
            "failed": self.failed,
            "alternativesSaved": self.alternatives_saved,
            "charged": self.charged,
            "failures": self.failures.iter().map(|failure| json!({
                "requestIndex": failure.request_index,
                "origin": failure.origin,
                "destination": failure.destination,
                "message": failure.message,
            })).collect::<Vec<_>>(),
            "chargeLimitReached": self.charge_limit_reached,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::{Duration, Instant},
    };

    use anyhow::Result;
    use serde_json::{json, Value};

    use super::*;
    use crate::request_params::build_directions_requests;

    struct MockClient {
        failed_origins: HashSet<String>,
        delay: Duration,
        calls: Arc<AtomicUsize>,
    }

    impl DirectionsClient for MockClient {
        async fn get_directions(
            &self,
            request: &DirectionsRequest,
            _deadline: Instant,
        ) -> Result<Value> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            if self.failed_origins.contains(&request.origin) {
                anyhow::bail!("temporary route failure");
            }
            Ok(json!({ "status": "OK", "directions": [{ "distance": 50 }, { "distance": 60 }] }))
        }
    }

    #[derive(Default)]
    struct MockWriter {
        rows: Vec<Value>,
        charge_limit_reached: bool,
        can_save: bool,
    }

    impl RouteWriter for MockWriter {
        fn can_save(&self) -> bool {
            self.can_save
        }

        async fn save(&mut self, item: &Value) -> Result<RouteSaveResult> {
            self.rows.push(item.clone());
            Ok(RouteSaveResult {
                saved: !self.charge_limit_reached,
                charged_count: usize::from(!self.charge_limit_reached),
                charge_limit_reached: self.charge_limit_reached,
            })
        }
    }

    fn requests() -> Vec<DirectionsRequest> {
        build_directions_requests(Some(&json!({
            "routes": [
                { "origin": "A", "destination": "B", "mode": "walking" },
                { "origin": "C", "destination": "D" }
            ]
        })))
        .unwrap()
    }

    #[tokio::test]
    async fn continues_after_a_route_failure_and_counts_alternatives() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = MockClient {
            failed_origins: HashSet::from(["A".to_owned()]),
            delay: Duration::ZERO,
            calls: calls.clone(),
        };
        let mut writer = MockWriter {
            can_save: true,
            ..MockWriter::default()
        };

        let result =
            run_directions_batch(&requests(), &client, &mut writer, Duration::from_secs(1)).await;

        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(result.requested, 2);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 1);
        assert_eq!(result.alternatives_saved, 2);
        assert_eq!(result.charged, 2);
        assert_eq!(writer.rows.len(), 2);
        assert_eq!(result.failures[0].request_index, 0);
        assert_eq!(result.to_json()["chargeLimitReached"], false);
    }

    #[tokio::test]
    async fn records_a_batch_deadline_for_routes_not_yet_started() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = MockClient {
            failed_origins: HashSet::new(),
            delay: Duration::from_millis(20),
            calls: calls.clone(),
        };
        let mut writer = MockWriter {
            can_save: true,
            ..MockWriter::default()
        };

        let result =
            run_directions_batch(&requests(), &client, &mut writer, Duration::from_millis(5)).await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.failed, 1);
        assert!(result.failures[0]
            .message
            .contains("Batch deadline reached"));
    }

    #[tokio::test]
    async fn stops_after_a_charge_limit_refuses_a_route_row() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = MockClient {
            failed_origins: HashSet::new(),
            delay: Duration::ZERO,
            calls: calls.clone(),
        };
        let mut writer = MockWriter {
            can_save: true,
            charge_limit_reached: true,
            ..MockWriter::default()
        };

        let result =
            run_directions_batch(&requests(), &client, &mut writer, Duration::from_secs(1)).await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(result.charge_limit_reached);
        assert_eq!(result.alternatives_saved, 0);
        assert_eq!(writer.rows.len(), 1);
    }

    #[tokio::test]
    async fn skips_upstream_requests_when_no_event_capacity_remains() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = MockClient {
            failed_origins: HashSet::new(),
            delay: Duration::ZERO,
            calls: calls.clone(),
        };
        let mut writer = MockWriter::default();

        let result =
            run_directions_batch(&requests(), &client, &mut writer, Duration::from_secs(1)).await;

        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(result.charge_limit_reached);
        assert_eq!(result.requested, 2);
    }
}
