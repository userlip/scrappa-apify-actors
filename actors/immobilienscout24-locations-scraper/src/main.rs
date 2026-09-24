mod apify_client;
mod config;
mod fallback_locations;
mod input;
mod locations;
mod runner;
mod scrappa_client;

#[cfg(test)]
mod test_utils;

use anyhow::{anyhow, Result};

use apify_client::ApifyClient;
use config::Config;
use input::build_location_requests;
use runner::process_location_requests;
use scrappa_client::ScrappaClient;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let scrappa = ScrappaClient::new(
        config.scrappa_api_base.clone(),
        config.scrappa_api_key.clone(),
    )?;
    let mut apify = ApifyClient::new(config)?;
    apify.initialize_charging().await?;

    let input = apify
        .get_input()
        .await?
        .ok_or_else(|| anyhow!("Input is required"))?;
    let requests = build_location_requests(&input)?;
    let summary = process_location_requests(&requests, &scrappa, &mut apify).await?;

    if summary.failed_queries == requests.len() {
        return Err(anyhow!(
            "All {} location queries failed",
            summary.failed_queries
        ));
    }

    apify.write_output(&summary.output_locations).await?;
    if summary.limit_reached {
        let status_message = format!(
            "Charge limit reached after saving {} location result(s).",
            summary.saved_results
        );
        println!("{status_message}");
        apify.set_status_message(&status_message).await?;
        return Ok(());
    }

    println!(
        "ImmobilienScout24 location autocomplete completed {{\"queries\":{},\"failed_queries\":{},\"unique_results\":{}}}",
        requests.len(),
        summary.failed_queries,
        summary.saved_results
    );
    Ok(())
}

impl runner::LocationFetcher for ScrappaClient {
    fn fetch<'a>(
        &'a self,
        request: &'a input::LocationRequest,
    ) -> impl std::future::Future<
        Output = std::result::Result<serde_json::Value, scrappa_client::ScrappaError>,
    > + Send
           + 'a {
        async move { self.get_locations(&request.query, request.limit).await }
    }
}
