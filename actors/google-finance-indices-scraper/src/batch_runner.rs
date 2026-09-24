use std::collections::HashSet;

use anyhow::{anyhow, Result};
use futures_util::future::join_all;
use serde_json::Value;

use crate::{
    apify::SaveResult,
    request_params::IndicesParams,
    response_utils::{canonical_symbol, extract_index_rows, map_index_row},
};

pub trait BatchDependencies {
    async fn get_capacity(&mut self) -> Result<usize>;
    async fn fetch(&self, symbol: Option<String>) -> Result<Value>;
    async fn save(&mut self, item: &Value, capacity: usize) -> Result<SaveResult>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub symbol: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchSummary {
    pub requested: usize,
    pub attempted: usize,
    pub saved: usize,
    pub duplicate: usize,
    pub failed: usize,
    pub charged: usize,
    pub charge_limit_reached: bool,
    pub outcomes: Vec<Outcome>,
}

pub async fn run_indices_batch<D: BatchDependencies>(
    requested: &[String],
    params: &IndicesParams,
    dependencies: &mut D,
) -> Result<BatchSummary> {
    let mut outcomes = Vec::new();
    let mut saved = 0;
    let mut duplicate = 0;
    let mut failed = 0;
    let mut charge_limit_reached = false;
    let mut seen = HashSet::new();

    let capacity = dependencies.get_capacity().await?;
    if capacity == 0 {
        return Ok(BatchSummary {
            requested: requested.len(),
            attempted: 0,
            saved,
            duplicate,
            failed,
            charged: saved,
            charge_limit_reached: true,
            outcomes: requested
                .iter()
                .map(|symbol| Outcome {
                    symbol: symbol.clone(),
                    status: "not_attempted".to_owned(),
                    error: Some("Charge limit reached".to_owned()),
                })
                .collect(),
        });
    }

    // This endpoint returns a single row per symbol. An empty input fetches the
    // API's defaults once; explicit batches run all affordable requests together.
    let symbols_to_fetch: Vec<Option<String>> = if requested.is_empty() {
        vec![None]
    } else {
        requested.iter().cloned().map(Some).collect()
    };
    let fetch_count = if capacity == usize::MAX {
        symbols_to_fetch.len()
    } else {
        symbols_to_fetch.len().min(capacity)
    };
    let fetch_limited = fetch_count < symbols_to_fetch.len();
    let fetched_symbols = &symbols_to_fetch[..fetch_count];
    let responses: Vec<_> = {
        let futures = fetched_symbols
            .iter()
            .map(|symbol| dependencies.fetch(symbol.clone()));
        join_all(futures).await
    };
    let attempted = responses.len();

    if !responses.is_empty() && responses.iter().all(Result::is_err) {
        return Err(responses[0]
            .as_ref()
            .err()
            .map(|error| anyhow!("{error:#}"))
            .unwrap_or_else(|| anyhow!("Scrappa request failed")));
    }

    for (requested_symbol, response) in fetched_symbols.iter().zip(responses) {
        let requested_symbol = requested_symbol.as_deref();
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                failed += 1;
                outcomes.push(Outcome {
                    symbol: requested_symbol.unwrap_or("default").to_owned(),
                    status: "failed".to_owned(),
                    error: Some(format!("{error:#}")),
                });
                continue;
            }
        };

        for row in extract_index_rows(&response) {
            let source_symbol = canonical_symbol(row.get("symbol"));
            if source_symbol.is_none()
                || requested_symbol
                    .is_some_and(|requested| Some(requested) != source_symbol.as_deref())
            {
                failed += 1;
                outcomes.push(Outcome {
                    symbol: source_symbol.unwrap_or_else(|| "unknown".to_owned()),
                    status: "failed".to_owned(),
                    error: Some(
                        "Scrappa returned an index outside the requested symbol".to_owned(),
                    ),
                });
                continue;
            }

            let source_symbol = source_symbol.unwrap();
            let Some(item) = map_index_row(&row, &source_symbol, params, None) else {
                failed += 1;
                outcomes.push(Outcome {
                    symbol: source_symbol,
                    status: "failed".to_owned(),
                    error: Some("Scrappa returned an index without a canonical symbol".to_owned()),
                });
                continue;
            };
            let item_id = item["id"].as_str().unwrap_or_default().to_owned();
            if seen.contains(&item_id) {
                duplicate += 1;
                outcomes.push(Outcome {
                    symbol: source_symbol,
                    status: "duplicate".to_owned(),
                    error: Some(format!(
                        "Duplicate index {item_id}; result was not saved or charged"
                    )),
                });
                continue;
            }

            let capacity = dependencies.get_capacity().await?;
            if capacity == 0 {
                charge_limit_reached = true;
                outcomes.push(Outcome {
                    symbol: source_symbol,
                    status: "not_attempted".to_owned(),
                    error: Some("Charge limit reached".to_owned()),
                });
                break;
            }

            let result = dependencies.save(&item, capacity).await?;
            if result.saved_count == 1 {
                seen.insert(item_id);
                saved += 1;
                outcomes.push(Outcome {
                    symbol: source_symbol,
                    status: "saved".to_owned(),
                    error: None,
                });
            } else {
                failed += 1;
                outcomes.push(Outcome {
                    symbol: source_symbol,
                    status: "failed".to_owned(),
                    error: Some("Apify did not save a chargeable index result".to_owned()),
                });
            }
            if result.charge_limit_reached {
                charge_limit_reached = true;
                break;
            }
        }

        if charge_limit_reached {
            break;
        }
    }

    for symbol in requested {
        if outcomes.iter().any(|outcome| outcome.symbol == *symbol) {
            continue;
        }
        let not_attempted = charge_limit_reached || fetch_limited;
        outcomes.push(Outcome {
            symbol: symbol.clone(),
            status: if not_attempted {
                "not_attempted"
            } else {
                "failed"
            }
            .to_owned(),
            error: Some(if not_attempted {
                "Charge limit reached".to_owned()
            } else {
                failed += 1;
                "Scrappa returned no matching index result".to_owned()
            }),
        });
    }

    Ok(BatchSummary {
        requested: requested.len(),
        attempted,
        saved,
        duplicate,
        failed,
        charged: saved,
        charge_limit_reached: charge_limit_reached || fetch_limited,
        outcomes,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::request_params::IndicesParams;
    use serde_json::json;

    struct FakeDependencies {
        capacity: usize,
        fetches: Arc<Mutex<Vec<Option<String>>>>,
        responses: Vec<Result<Value>>,
        writes: Vec<Value>,
        stop_after_save: bool,
    }

    impl BatchDependencies for FakeDependencies {
        async fn get_capacity(&mut self) -> Result<usize> {
            Ok(self.capacity)
        }

        async fn fetch(&self, symbol: Option<String>) -> Result<Value> {
            self.fetches.lock().unwrap().push(symbol.clone());
            let index = self.fetches.lock().unwrap().len() - 1;
            self.responses[index]
                .as_ref()
                .map(Value::clone)
                .map_err(|error| anyhow!("{error:#}"))
        }

        async fn save(&mut self, item: &Value, _capacity: usize) -> Result<SaveResult> {
            self.writes.push(item.clone());
            if self.stop_after_save {
                self.capacity = 0;
            } else if self.capacity != usize::MAX {
                self.capacity = self.capacity.saturating_sub(1);
            }
            Ok(SaveResult {
                saved_count: 1,
                charge_limit_reached: self.capacity == 0,
            })
        }
    }

    fn params() -> IndicesParams {
        IndicesParams {
            indices: None,
            hl: "en".to_owned(),
            gl: "us".to_owned(),
        }
    }

    #[tokio::test]
    async fn does_not_fetch_when_pay_per_event_capacity_is_exhausted() {
        let mut dependencies = FakeDependencies {
            capacity: 0,
            fetches: Arc::default(),
            responses: vec![],
            writes: vec![],
            stop_after_save: false,
        };
        let summary = run_indices_batch(&[".INX".to_owned()], &params(), &mut dependencies)
            .await
            .unwrap();
        assert!(dependencies.fetches.lock().unwrap().is_empty());
        assert!(dependencies.writes.is_empty());
        assert_eq!(summary.attempted, 0);
        assert_eq!(summary.charged, 0);
        assert!(summary.charge_limit_reached);
        assert_eq!(summary.outcomes[0].status, "not_attempted");
    }

    #[tokio::test]
    async fn deduplicates_rows_and_stops_after_the_last_affordable_event() {
        let mut dependencies = FakeDependencies {
            capacity: 2,
            fetches: Arc::default(),
            responses: vec![
                Ok(json!({"data":[
                    {"symbol":".INX","exchange":"INDEXSP"},
                    {"symbol":".INX","exchange":"INDEXSP"}
                ]})),
                Ok(json!({"data":[{"symbol":".DJI","exchange":"INDEXDJX"}]})),
            ],
            writes: vec![],
            stop_after_save: false,
        };
        let requested = [".INX".to_owned(), ".DJI".to_owned(), ".IXIC".to_owned()];
        let summary = run_indices_batch(&requested, &params(), &mut dependencies)
            .await
            .unwrap();
        assert_eq!(dependencies.writes.len(), 2);
        assert_eq!(summary.saved, 2);
        assert_eq!(summary.charged, 2);
        assert_eq!(summary.duplicate, 1);
        assert!(summary.charge_limit_reached);
        assert_eq!(
            summary
                .outcomes
                .iter()
                .find(|outcome| outcome.symbol == ".IXIC")
                .unwrap()
                .status,
            "not_attempted"
        );
    }

    #[tokio::test]
    async fn fetches_each_requested_symbol_and_continues_after_one_failure() {
        let fetches = Arc::new(Mutex::new(Vec::new()));
        let mut dependencies = FakeDependencies {
            capacity: usize::MAX,
            fetches: fetches.clone(),
            responses: vec![
                Ok(json!({"data":[{"symbol":".INX","exchange":"INDEXSP"}]})),
                Err(anyhow!("Scrappa request failed after retries")),
                Ok(json!({"data":[{"symbol":".IXIC","exchange":"INDEXNASDAQ"},{"symbol":".RUT"}]})),
            ],
            writes: vec![],
            stop_after_save: false,
        };
        let requested = [".INX".to_owned(), ".DJI".to_owned(), ".IXIC".to_owned()];
        let summary = run_indices_batch(&requested, &params(), &mut dependencies)
            .await
            .unwrap();
        assert_eq!(
            fetches.lock().unwrap().as_slice(),
            [
                Some(".INX".to_owned()),
                Some(".DJI".to_owned()),
                Some(".IXIC".to_owned())
            ]
        );
        assert_eq!(summary.attempted, 3);
        assert_eq!(summary.saved, 2);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.charged, 2);
        assert_eq!(
            dependencies
                .writes
                .iter()
                .map(|item| item["symbol"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [".INX", ".IXIC"]
        );
    }

    #[tokio::test]
    async fn fails_the_actor_when_every_upstream_request_is_exhausted() {
        let mut dependencies = FakeDependencies {
            capacity: usize::MAX,
            fetches: Arc::default(),
            responses: vec![Err(anyhow!("Scrappa request failed after retries"))],
            writes: vec![],
            stop_after_save: false,
        };
        let error = run_indices_batch(&[".INX".to_owned()], &params(), &mut dependencies)
            .await
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("Scrappa request failed after retries"));
    }
}
