use std::{env, process::ExitCode};

use anyhow::{anyhow, Result};
use serde_json::json;

use crate::apify::{
    env_or_default, push_charged_item, resume_charged_item, ApifyClient, ApifyConfig,
};
use crate::doctor_details::{
    build_dataset_item, build_doctor_details_params, build_doctor_details_plan,
    build_output_summary, describe_request, InputFailure,
};
use crate::scrappa::{ScrappaClient, SCRAPPA_API_DEFAULT};

struct RunOutcome {
    status_message: Option<String>,
    succeeded: bool,
}

async fn run_actor(apify: &ApifyClient, api_key: &str) -> Result<RunOutcome> {
    let input = apify
        .get_input()
        .await?
        .ok_or_else(|| anyhow!("Input is required"))?;
    let plan = build_doctor_details_plan(&input)?;
    println!(
        "Fetching Jameda doctor details for {}",
        describe_request(&plan.doctor_urls)
    );

    let scrappa = ScrappaClient::new(
        api_key.to_owned(),
        env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
    )?;
    let mut failures = plan.input_failures;
    let mut saved_profiles = 0;
    let mut status_message = None;
    let mut pricing = None;

    for (index, doctor_url) in plan.doctor_urls.iter().enumerate() {
        println!("Fetching Jameda doctor details for {doctor_url}");

        match resume_charged_item(apify, &mut pricing, index + 1, doctor_url).await {
            Ok(Some(result)) => {
                saved_profiles += result.saved_count;
                println!(
                    "Recovered {} Jameda doctor profile result(s) for {doctor_url}",
                    result.saved_count
                );
                if result.status_message.is_some() {
                    status_message = result.status_message;
                    break;
                }
                continue;
            }
            Ok(None) => {}
            Err(error) => {
                let message = error.to_string();
                failures.push(InputFailure {
                    doctor_url: doctor_url.clone(),
                    error: message.clone(),
                });
                eprintln!("Failed to recover Jameda doctor details for {doctor_url}: {message}");
                break;
            }
        }

        let params = build_doctor_details_params(doctor_url);
        let response = match scrappa.get(doctor_url).await {
            Ok(response) => response,
            Err(error) => {
                let message = if error
                    .message
                    .starts_with("Scrappa API request timed out after ")
                {
                    format!(
                        "{}. The Jameda doctor details request exceeded the 90s Scrappa API timeout. Try fewer doctor URLs or run the request again.",
                        error.message
                    )
                } else {
                    error.message
                };
                failures.push(InputFailure {
                    doctor_url: doctor_url.clone(),
                    error: message.clone(),
                });
                eprintln!("Failed to fetch Jameda doctor details for {doctor_url}: {message}");
                continue;
            }
        };
        let item = build_dataset_item(&response, doctor_url, &params);
        match push_charged_item(apify, &mut pricing, &item, index + 1, doctor_url, true).await {
            Ok(result) => {
                saved_profiles += result.saved_count;
                println!(
                    "Saved {} Jameda doctor profile result(s) for {doctor_url}",
                    result.saved_count
                );
                if result.status_message.is_some() {
                    status_message = result.status_message;
                    break;
                }
            }
            Err(error) => {
                let message = error.to_string();
                failures.push(InputFailure {
                    doctor_url: doctor_url.clone(),
                    error: message.clone(),
                });
                eprintln!("Failed to fetch Jameda doctor details for {doctor_url}: {message}");
                break;
            }
        }
    }

    if status_message.is_none() && !failures.is_empty() {
        status_message = Some(format!(
            "{} Jameda doctor detail request(s) failed; {} profile(s) saved.",
            failures.len(),
            saved_profiles
        ));
    }

    let summary = build_output_summary(
        &plan.doctor_urls,
        saved_profiles,
        &failures,
        status_message.as_deref(),
    );
    apify.put_record("OUTPUT", &summary).await?;

    println!("Jameda doctor details extraction completed successfully");
    println!(
        "Results summary: {}",
        json!({
            "doctors_requested": plan.doctor_urls.len(),
            "doctors_saved": saved_profiles,
            "doctors_failed": failures.len(),
        })
    );

    if saved_profiles == 0 && !failures.is_empty() {
        return Ok(RunOutcome {
            status_message: Some(
                status_message
                    .unwrap_or_else(|| "No Jameda doctor profiles were saved.".to_owned()),
            ),
            succeeded: false,
        });
    }

    Ok(RunOutcome {
        status_message,
        succeeded: true,
    })
}

pub(crate) async fn run() -> ExitCode {
    let apify_config = match ApifyConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let apify = match ApifyClient::new(apify_config) {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            return ExitCode::FAILURE;
        }
    };

    let api_key = match env::var("SCRAPPA_API_KEY") {
        Ok(value) if !value.is_empty() => value,
        _ => {
            let error = "SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.";
            eprintln!("Actor failed: {error}");
            if let Err(status_error) = apify.set_terminal_status_message(error).await {
                eprintln!("Failed to set Actor status message: {status_error}");
            }
            return ExitCode::FAILURE;
        }
    };

    match run_actor(&apify, &api_key).await {
        Ok(outcome) => {
            if let Some(status_message) = outcome.status_message {
                if let Err(error) = apify.set_terminal_status_message(&status_message).await {
                    eprintln!("Failed to set Actor status message: {error}");
                    return ExitCode::FAILURE;
                }
            }
            if outcome.succeeded {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            let message = error.to_string();
            eprintln!("Actor failed: {message}");
            if let Err(status_error) = apify.set_terminal_status_message(&message).await {
                eprintln!("Failed to set Actor status message: {status_error}");
            }
            ExitCode::FAILURE
        }
    }
}
