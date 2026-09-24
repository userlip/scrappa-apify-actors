use std::{env, process::ExitCode, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{Map, Value};
use url::Url;

use website_content_extractor_scraper::{
    apify::{ApifyClient, ApifyConfig, DatasetWriteResult},
    input::{
        build_web_scraper_params, describe_web_scraper_request, get_input_urls, get_response_type,
        ResponseType,
    },
    response::{
        build_failure_dataset_item, build_json_dataset_item, build_markdown_dataset_item,
        insert_summary_fields,
    },
    web_scraper_client::ScrappaWebScraperClient,
};

const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const SCRAPPA_MAX_ATTEMPTS: usize = 2;
const SCRAPPA_RETRY_DELAY: Duration = Duration::from_secs(1);

struct Config {
    scrappa_api_key: String,
    scrappa_api_base_url: Url,
    apify: ApifyConfig,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;
        let scrappa_api_base_url = base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT)?;

        Ok(Self {
            scrappa_api_key,
            scrappa_api_base_url,
            apify: ApifyConfig::from_env()?,
        })
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let apify = ApifyClient::new(config.apify)?;
    let input = apify.get_input().await?;
    let response_type = get_response_type(input.as_ref())?;
    let requests = get_input_urls(input.as_ref());
    if requests.is_empty() {
        bail!("At least one URL is required. Provide urls or backward-compatible url.");
    }

    let client = ScrappaWebScraperClient::new(
        config.scrappa_api_key,
        config.scrappa_api_base_url,
        SCRAPPA_REQUEST_TIMEOUT,
        SCRAPPA_MAX_ATTEMPTS,
        SCRAPPA_RETRY_DELAY,
    )
    .context("Could not create Scrappa Web Scraper API client")?;

    let mut billing = apify.get_billing_state().await?;
    let mut succeeded = 0;
    let mut failed = 0;
    let mut saved = 0;

    println!(
        "Extracting website content for {} URL{} with response_type={}",
        requests.len(),
        if requests.len() == 1 { "" } else { "s" },
        response_type.as_str()
    );

    for (index, request) in requests.iter().enumerate() {
        let params = build_web_scraper_params(request, input.as_ref(), response_type);
        println!(
            "Calling Scrappa Web Scraper API ({})",
            describe_web_scraper_request(&params)
        );

        let item = match response_type {
            ResponseType::Json => match client.scrape_json(&params).await {
                Ok(response) => build_json_dataset_item(&response, request, &params),
                Err(error) => {
                    eprintln!(
                        "Scrappa Web Scraper API returned a per-URL failure for {}: {}",
                        request.input_url, error
                    );
                    build_failure_dataset_item(&error, request, &params)
                }
            },
            ResponseType::Markdown => match client.scrape_markdown(&params).await {
                Ok(markdown) => build_markdown_dataset_item(&markdown, request, &params),
                Err(error) => {
                    eprintln!(
                        "Scrappa Web Scraper API returned a per-URL failure for {}: {}",
                        request.input_url, error
                    );
                    build_failure_dataset_item(&error, request, &params)
                }
            },
        };

        let result = apify
            .push_dataset_item(&item, &mut billing, &apify.charge_idempotency_key(index))
            .await?;
        if result == DatasetWriteResult::ChargeLimitReached {
            println!(
                "Charge limit reached while saving the website content result: {}",
                request.input_url
            );
            break;
        }

        saved += 1;
        if item.get("success").and_then(Value::as_bool) == Some(true) {
            succeeded += 1;
        } else {
            failed += 1;
        }
    }

    let mut summary = Map::new();
    insert_summary_fields(
        &mut summary,
        requests.len(),
        saved,
        succeeded,
        failed,
        response_type,
    );
    let summary = Value::Object(summary);
    println!("Website content extraction completed");
    println!(
        "Results summary: {}",
        serde_json::to_string(&summary).context("Could not serialize the results summary")?
    );
    apify.put_output(&summary).await?;

    if saved < requests.len() {
        println!(
            "Saved {saved} of {} requested URL result(s).",
            requests.len()
        );
    }
    Ok(())
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}
