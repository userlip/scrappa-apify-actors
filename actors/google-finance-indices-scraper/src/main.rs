mod apify;
mod batch_runner;
mod request_params;
mod response_utils;
mod runtime_config;
mod scrappa;

#[cfg(test)]
mod test_support;

use anyhow::Result;
use serde_json::{json, Value};

use apify::{ActorConfig, ApifyClient, SaveResult};
use batch_runner::{run_indices_batch, BatchDependencies, BatchSummary};
use request_params::{build_google_finance_indices_params, normalize_indices, IndicesParams};
use runtime_config::RunDeadline;
use scrappa::ScrappaClient;

struct ActorDependencies {
    apify: ApifyClient,
    scrappa: ScrappaClient,
    params: IndicesParams,
}

impl BatchDependencies for ActorDependencies {
    async fn get_capacity(&mut self) -> Result<usize> {
        self.apify.get_capacity().await
    }

    async fn fetch(&self, symbol: Option<String>) -> Result<Value> {
        self.scrappa
            .get_indices(&self.params, symbol.as_deref())
            .await
    }

    async fn save(&mut self, item: &Value, capacity: usize) -> Result<SaveResult> {
        self.apify.save_index(item, capacity).await
    }
}

async fn run_actor() -> Result<BatchSummary> {
    let config = ActorConfig::from_env()?;
    let deadline = RunDeadline::from_env()?;
    let apify = ApifyClient::new_with_deadline(config.clone(), deadline)?;
    let input = apify.get_input().await?;
    let input = if input.is_null() { json!({}) } else { input };
    let params = build_google_finance_indices_params(&input)?;
    let requested = normalize_indices(input.get("indices"))?;

    println!(
        "Fetching Google Finance indices for {}",
        request_params::describe_google_finance_indices_request(&params)
    );

    let scrappa = ScrappaClient::new_with_deadline(
        config.scrappa_api_key,
        config.scrappa_api_base.as_deref(),
        deadline,
    )?;
    let mut dependencies = ActorDependencies {
        apify,
        scrappa,
        params: params.clone(),
    };
    run_indices_batch(&requested, &params, &mut dependencies).await
}

#[tokio::main]
async fn main() {
    match run_actor().await {
        Ok(summary) => {
            println!(
                "Google Finance indices completed: requested={} attempted={} saved={} duplicate={} failed={} charged={} charge_limit_reached={}",
                summary.requested,
                summary.attempted,
                summary.saved,
                summary.duplicate,
                summary.failed,
                summary.charged,
                summary.charge_limit_reached
            );
        }
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            std::process::exit(1);
        }
    }
}
