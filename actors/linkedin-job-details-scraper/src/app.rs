use anyhow::{anyhow, bail, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::{env, process::ExitCode};

use crate::{
    apify_client::{env_or_default, ApifyClient, ApifyConfig},
    job::{
        build_failure_item, build_job_params, build_output, build_success_item, get_input_urls,
        is_recoverable_job_error, is_success, js_string, should_charge_result, ScrappaApiError,
    },
    pricing::{PricingState, DEFAULT_DATASET_ITEM_EVENT, JOB_RESULT_CHARGE_EVENT},
    scrappa::{ScrappaClient, REQUEST_TIMEOUT},
};

const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";

pub(crate) struct Config {
    pub(crate) apify: ApifyConfig,
    pub(crate) scrappa_api_base: String,
    pub(crate) scrappa_api_key: String,
}

impl Config {
    pub(crate) fn from_env() -> Result<Self> {
        let apify = ApifyConfig::from_env()?;
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify,
            scrappa_api_base: env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
            scrappa_api_key,
        })
    }
}

pub(crate) struct PushChargedItemsResult {
    pub(crate) saved_count: usize,
    pub(crate) status_message: Option<String>,
}

pub(crate) async fn push_charged_item(
    apify: &ApifyClient,
    pricing: &mut PricingState,
    item: &Value,
    charge_event: bool,
    request_index: usize,
) -> Result<PushChargedItemsResult> {
    if !pricing.is_pay_per_event {
        apify.push_dataset_item(item).await?;
        return Ok(PushChargedItemsResult {
            saved_count: 1,
            status_message: None,
        });
    }

    let event_name = charge_event.then_some(JOB_RESULT_CHARGE_EVENT);
    if !pricing.should_push_item(event_name) {
        return Ok(PushChargedItemsResult {
            saved_count: usize::from(!charge_event),
            status_message: charge_event.then(|| charge_limit_message(0, 1)),
        });
    }

    apify.push_dataset_item(item).await?;

    let mut event_names = Vec::with_capacity(2);
    if let Some(event_name) = event_name {
        event_names.push(event_name.to_owned());
    }
    event_names.push(DEFAULT_DATASET_ITEM_EVENT.to_owned());

    let mut charges = Vec::with_capacity(event_names.len());
    for event_name in &event_names {
        charges.push(pricing.register_charge(event_name, 1));
    }
    for charge in &charges {
        if charge.should_call_api {
            let idempotency_key = format!(
                "{}-{}-{}",
                apify.actor_run_id, charge.event_name, request_index
            );
            apify
                .charge_event(&charge.event_name, charge.charged_count, &idempotency_key)
                .await?;
        }
    }

    if !charge_event {
        return Ok(PushChargedItemsResult {
            saved_count: 1,
            status_message: None,
        });
    }

    let charged_count = charges
        .iter()
        .map(|charge| charge.charged_count)
        .sum::<usize>();
    let event_charge_limit_reached = event_names
        .iter()
        .any(|event_name| pricing.event_charge_limit_reached(event_name));
    if !event_charge_limit_reached {
        return Ok(PushChargedItemsResult {
            saved_count: 1,
            status_message: None,
        });
    }

    let saved_count = charged_count.min(1);
    let status_message = charge_limit_message(saved_count, 1);
    println!(
        "{status_message} {}",
        json!({
            "event": JOB_RESULT_CHARGE_EVENT,
            "charged_count": charged_count,
            "requested_count": 1,
            "saved_count": saved_count,
        })
    );
    Ok(PushChargedItemsResult {
        saved_count,
        status_message: Some(status_message),
    })
}

pub(crate) fn charge_limit_message(saved_count: usize, requested_count: usize) -> String {
    format!(
        "Charge limit reached after saving {saved_count} of {requested_count} LinkedIn job detail results."
    )
}

pub(crate) async fn run(
    config: &Config,
    apify: &ApifyClient,
    http: Client,
) -> Result<Option<String>> {
    let run = apify.get_run().await?;
    let mut pricing = PricingState::from_run(&run)?;
    let input = apify.get_input().await?;
    let urls = get_input_urls(input.as_ref())?;
    if urls.is_empty() {
        bail!("At least one LinkedIn job URL is required. Provide either url (single URL) or urls (array of URLs).");
    }

    let scrappa = ScrappaClient::new(
        http,
        config.scrappa_api_base.clone(),
        config.scrappa_api_key.clone(),
    );
    let mut first_result: Option<Value> = None;
    let mut succeeded = 0;
    let mut failed = 0;
    let mut status_message: Option<String> = None;
    println!(
        "Scraping {} LinkedIn job URL{}",
        urls.len(),
        if urls.len() == 1 { "" } else { "s" }
    );

    let use_cache = input.as_ref().and_then(|input| input.get("use_cache"));
    let maximum_cache_age = input
        .as_ref()
        .and_then(|input| input.get("maximum_cache_age"));
    for (request_index, request) in urls.iter().enumerate() {
        let result = if let Some(normalized_url) = request.normalized_url.as_deref() {
            println!("Fetching LinkedIn job details: {normalized_url}");
            let params = build_job_params(normalized_url, use_cache, maximum_cache_age);
            match scrappa.get_job(&params).await {
                Ok(response) => build_success_item(response, &request.input_url, normalized_url)?,
                Err(error) if is_recoverable_job_error(&error) => {
                    let api_error = error
                        .downcast_ref::<ScrappaApiError>()
                        .expect("404 Scrappa error must retain its API error type");
                    eprintln!(
                        "Job detail scraping returned a per-item failure for {normalized_url}: {api_error}"
                    );
                    build_failure_item(
                        &api_error.to_string(),
                        Some(api_error.status),
                        &request.input_url,
                        Some(normalized_url),
                    )
                }
                Err(error) => return Err(error),
            }
        } else {
            eprintln!("Invalid LinkedIn job URL: \"{}\"", request.input_url);
            build_failure_item(
                request
                    .validation_error
                    .as_deref()
                    .unwrap_or("Invalid LinkedIn job URL"),
                None,
                &request.input_url,
                None,
            )
        };

        let push_result = push_charged_item(
            apify,
            &mut pricing,
            &result,
            should_charge_result(&result),
            request_index,
        )
        .await?;
        if let Some(message) = push_result.status_message {
            status_message = Some(message);
        }
        if push_result.saved_count > 0 {
            first_result.get_or_insert_with(|| result.clone());
        }

        if is_success(&result) {
            succeeded += push_result.saved_count;
            let title = result
                .get("title")
                .filter(|title| !title.is_null())
                .map(js_string)
                .or_else(|| request.normalized_url.clone())
                .unwrap_or_else(|| request.input_url.clone());
            println!("Saved LinkedIn job detail: {title}");
        } else {
            failed += push_result.saved_count;
            let message = result
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if message.is_empty() {
                eprintln!("LinkedIn job detail failed");
            } else {
                eprintln!("LinkedIn job detail failed ({message})");
            }
        }

        if status_message.is_some() {
            break;
        }
    }

    let output = match (urls.len(), first_result.as_ref()) {
        (1, Some(result)) => build_output(result),
        _ => json!({
            "requested": urls.len(),
            "succeeded": succeeded,
            "failed": failed,
        }),
    };
    apify.put_record("OUTPUT", &output).await?;

    println!("LinkedIn job detail scraping completed");
    println!(
        "Job detail summary: {}",
        serde_json::to_string_pretty(&json!({
            "requested": urls.len(),
            "succeeded": succeeded,
            "failed": failed,
        }))?
    );

    Ok(status_message)
}

pub(crate) async fn run_actor() -> ExitCode {
    let apify_config = ApifyConfig::from_env().ok();
    let config = Config::from_env();
    let http = match Client::builder().timeout(REQUEST_TIMEOUT).build() {
        Ok(http) => http,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            return ExitCode::FAILURE;
        }
    };

    let config = match config {
        Ok(config) => config,
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            if let Some(apify_config) = apify_config {
                ApifyClient::new(http, &apify_config)
                    .set_terminal_status_message(&message)
                    .await;
            }
            return ExitCode::FAILURE;
        }
    };

    let apify = ApifyClient::new(http.clone(), &config.apify);
    match run(&config, &apify, http).await {
        Ok(status_message) => {
            if let Some(message) = status_message {
                println!("[Status message]: {message}");
                apify.set_terminal_status_message(&message).await;
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            println!("[Status message]: {message}");
            apify.set_terminal_status_message(&message).await;
            ExitCode::FAILURE
        }
    }
}
