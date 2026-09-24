mod apify;
mod fallback;
mod jobs;
mod params;
mod scrappa;
#[cfg(test)]
mod test_support;

use std::{env, process::ExitCode, time::Duration};

use anyhow::{bail, Context, Result};
use apify::ApifyClient;
use fallback::transform_indeed_fallback_response;
use jobs::{filter_count, get_jobs, get_next_page_token};
use params::{build_indeed_fallback_params, build_jobs_params, normalize_jobs_input, GoogleJobsInput};
use reqwest::Client;
use scrappa::{ScrappaClient, ScrappaError};
use serde_json::Value;
use url::Url;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const SCRAPPA_API_DEFAULT: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SCRAPPA_MAX_ATTEMPTS: usize = 3;
const SCRAPPA_FALLBACK_ATTEMPTS: usize = 2;

struct Config {
    apify_api_base: Url,
    apify_token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
    scrappa_api_base: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY").unwrap_or_default();
        if scrappa_api_key.is_empty() {
            bail!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.");
        }
        Ok(Self {
            apify_api_base: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT)?,
            apify_token: required_env("APIFY_TOKEN")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            scrappa_api_base: env::var("SCRAPPA_API_BASE_URL")
                .unwrap_or_else(|_| SCRAPPA_API_DEFAULT.to_owned()),
            scrappa_api_key,
        })
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", display_actor_error(&error));
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let config = Config::from_env()?;
    let apify_http = Client::builder()
        .timeout(SCRAPPA_REQUEST_TIMEOUT)
        .build()
        .context("Could not create Apify HTTP client")?;
    let apify = ApifyClient::new(
        apify_http,
        config.apify_api_base,
        config.apify_token,
        config.key_value_store_id,
        config.dataset_id,
        config.actor_run_id,
        config.input_key,
    );

    let input = normalize_jobs_input(apify.get_input().await?)?;
    input.validate()?;
    let search_label = input
        .q
        .as_ref()
        .map(|query| format!("\"{query}\""))
        .unwrap_or_else(|| "next page token".to_owned());
    println!("Searching Google Jobs for: {search_label}");

    let scrappa = ScrappaClient::new(
        &config.scrappa_api_base,
        config.scrappa_api_key,
        SCRAPPA_REQUEST_TIMEOUT,
    )
    .map_err(anyhow::Error::new)?;
    let response = get_jobs_response(&scrappa, &input).await?;
    let jobs = get_jobs(&response);

    if jobs.is_empty() {
        println!("No job results found for the given search criteria");
    } else {
        let allowed_items = apify.dataset_item_limit(jobs.len()).await?;
        let saved_items = apify.push_dataset_items(&jobs[..allowed_items]).await?;
        if saved_items < jobs.len() {
            println!(
                "Charge limit reached after saving {saved_items}/{} Google Jobs result(s).",
                jobs.len()
            );
        }
        println!("Found {saved_items} job result(s)");
    }

    apify.set_output(&response).await?;
    println!("Google Jobs search completed successfully");
    println!(
        "Results summary: {}",
        serde_json::json!({
            "jobs": jobs.len(),
            "filters": filter_count(&response),
            "has_next_page": get_next_page_token(&response).is_some()
        })
    );
    Ok(())
}

async fn get_jobs_response(scrappa: &ScrappaClient, input: &GoogleJobsInput) -> Result<Value> {
    match scrappa
        .get("/google/jobs", &build_jobs_params(input), SCRAPPA_MAX_ATTEMPTS)
        .await
    {
        Ok(response) => Ok(response),
        Err(google_error)
            if input.next_page_token.is_none() && google_error.is_retryable() =>
        {
            let google_message = google_error.to_string();
            eprintln!(
                "Google Jobs request failed after retries ({google_message}). Falling back to Scrappa Indeed jobs for this search."
            );
            match scrappa
                .get(
                    "/indeed/jobs",
                    &build_indeed_fallback_params(input),
                    SCRAPPA_FALLBACK_ATTEMPTS,
                )
                .await
            {
                Ok(response) => Ok(transform_indeed_fallback_response(
                    &response,
                    input,
                    &google_message,
                )),
                Err(fallback_error) => {
                    let message = format!(
                        "Google Jobs failed ({google_message}), and Indeed fallback also failed: {fallback_error}"
                    );
                    if fallback_error.is_timeout() {
                        Err(anyhow::Error::new(ScrappaError::Timeout(message)))
                    } else {
                        Err(anyhow::Error::new(ScrappaError::Fallback(message)))
                    }
                }
            }
        }
        Err(error) => Err(anyhow::Error::new(error)),
    }
}

fn display_actor_error(error: &anyhow::Error) -> String {
    if let Some(ScrappaError::Timeout(message)) = error.downcast_ref::<ScrappaError>() {
        return format!(
            "{message}. The Google Jobs request exceeded the {}s Scrappa API timeout. Try again or refine the query.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        );
    }
    format!("{error:#}")
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name).with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockResponse, MockServer};

    fn test_client(server: &MockServer) -> ScrappaClient {
        let mut client = ScrappaClient::new(
            &server.base_url(),
            "test-key".to_owned(),
            Duration::from_secs(1),
        )
        .unwrap();
        client.set_retry_delay_override(Duration::ZERO);
        client
    }

    #[tokio::test]
    async fn falls_back_to_indeed_after_retryable_google_failures() {
        let server = MockServer::start(vec![
            MockResponse::json(504, r#"{"message":"Gateway Timeout"}"#),
            MockResponse::json(503, r#"{"message":"Unavailable"}"#),
            MockResponse::json(502, r#"{"message":"Bad Gateway"}"#),
            MockResponse::json(200, r#"{"jobs":[{"id":"fallback-job","title":"Nurse"}]}"#),
        ]);
        let client = test_client(&server);
        let input = normalize_jobs_input(Some(serde_json::json!({
            "q": "nurse jobs in Austin",
            "gl": "us",
            "hl": "en"
        })))
        .unwrap();
        let response = get_jobs_response(&client, &input).await.unwrap();
        assert_eq!(response["service_used"], "indeed");
        assert_eq!(response["jobs_results"][0]["job_id"], "fallback-job");
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests[..3]
            .iter()
            .all(|request| request.starts_with("GET /google/jobs?")));
        assert!(requests[3].starts_with("GET /indeed/jobs?query=nurse&limit=10&location=Austin&country=US&gl=us&hl=en"));
    }

    #[tokio::test]
    async fn does_not_fall_back_for_client_errors_or_pagination_requests() {
        let server = MockServer::start(vec![MockResponse::json(
            400,
            r#"{"message":"Invalid query"}"#,
        )]);
        let client = test_client(&server);
        let input = normalize_jobs_input(Some(serde_json::json!({ "q": "nurse" }))).unwrap();
        let error = get_jobs_response(&client, &input).await.unwrap_err();
        assert_eq!(error.to_string(), "Scrappa API error (400): Invalid query");
        assert_eq!(server.requests().len(), 1);

        let server = MockServer::start(vec![
            MockResponse::json(504, r#"{"message":"Gateway Timeout"}"#),
            MockResponse::json(504, r#"{"message":"Gateway Timeout"}"#),
            MockResponse::json(504, r#"{"message":"Gateway Timeout"}"#),
        ]);
        let client = test_client(&server);
        let input = normalize_jobs_input(Some(serde_json::json!({
            "next_page_token": "next-token"
        })))
        .unwrap();
        assert!(get_jobs_response(&client, &input).await.is_err());
        assert_eq!(server.requests().len(), 3);
    }

    #[test]
    fn adds_timeout_context_to_actor_failure_message() {
        let timeout = ScrappaError::Timeout("Scrappa API request timed out after 60000ms".to_owned());
        let error = anyhow::Error::new(timeout);
        assert_eq!(
            display_actor_error(&error),
            "Scrappa API request timed out after 60000ms. The Google Jobs request exceeded the 60s Scrappa API timeout. Try again or refine the query."
        );
    }
}
