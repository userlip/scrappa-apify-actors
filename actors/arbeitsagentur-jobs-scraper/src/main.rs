mod apify;
mod scrappa;
#[cfg(test)]
mod tests;

use std::{env, process::ExitCode};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
#[cfg(test)]
use reqwest::StatusCode;
use serde_json::{json, Map, Value};
use url::Url;

#[cfg(test)]
use apify::affordable_dataset_items;
use apify::{ApifyClient, APIFY_API_DEFAULT, APIFY_TIMEOUT};
#[cfg(test)]
use scrappa::{
    build_jobs_url, duration_millis, get_retry_delay_ms, parse_retry_after_ms,
    scrappa_error_message, ScrappaRetryPolicy, SCRAPPA_REQUEST_DEADLINE, SCRAPPA_USER_AGENT,
};
use scrappa::{
    query_value, ScrappaClient, ScrappaFailure, SCRAPPA_API_DEFAULT, SCRAPPA_REQUEST_TIMEOUT,
};

#[cfg(test)]
const ACTOR_TIMEOUT_MS: u64 = 240_000;
#[cfg(test)]
const ACTOR_COMPLETION_RESERVE_MS: u64 = 30_000;

const INPUT_KEYS: &[&str] = &[
    "was",
    "wo",
    "berufsfeld",
    "arbeitgeber",
    "angebotsart",
    "arbeitszeit",
    "befristung",
    "veroeffentlichtseit",
    "umkreis",
    "zeitarbeit",
    "pav",
    "page",
    "size",
];

#[derive(Debug)]
struct Config {
    apify_api_base: String,
    apify_token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    scrappa_api_base: String,
    scrappa_api_key: String,
}

impl Config {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify_api_base: env_or_default("APIFY_API_PUBLIC_BASE_URL", APIFY_API_DEFAULT),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_base: env_or_default("SCRAPPA_API_BASE_URL", SCRAPPA_API_DEFAULT),
            scrappa_api_key,
        })
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn endpoint_url(base_url: &str, path: &[&str]) -> Result<Url> {
    let mut url = Url::parse(&format!("{}/", base_url.trim_end_matches('/')))
        .with_context(|| format!("Invalid API base URL: {base_url}"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow!("API base URL cannot contain query or fragment: {base_url}"))?;
        segments.pop_if_empty();
        for segment in path {
            segments.push(segment);
        }
    }
    Ok(url)
}

fn normalize_input(input: Option<&Value>) -> Map<String, Value> {
    let mut normalized = Map::new();
    if let Some(fields) = input.and_then(Value::as_object) {
        for (key, value) in fields {
            if !INPUT_KEYS.contains(&key.as_str()) || value.is_null() {
                continue;
            }
            if let Some(value) = value.as_str() {
                let value = value.trim();
                if !value.is_empty() {
                    let value = if key == "arbeitszeit" {
                        value
                            .chars()
                            .filter(|character| !character.is_whitespace())
                            .flat_map(char::to_lowercase)
                            .collect()
                    } else {
                        value.to_owned()
                    };
                    normalized.insert(key.clone(), Value::String(value));
                }
            } else {
                normalized.insert(key.clone(), value.clone());
            }
        }
    }

    let has_known_input = normalized
        .iter()
        .any(|(key, value)| INPUT_KEYS.contains(&key.as_str()) && value.as_str() != Some(""));
    if !has_known_input {
        return default_input();
    }

    let mut input = default_input();
    input.extend(normalized);
    if input.get("was").is_none_or(Value::is_null) {
        input.insert(
            "was".to_owned(),
            Value::String("Software Entwickler".to_owned()),
        );
    }
    input
}

fn default_input() -> Map<String, Value> {
    let mut input = Map::new();
    input.insert("was".to_owned(), json!("Software Entwickler"));
    input.insert("wo".to_owned(), json!("Berlin"));
    input.insert("umkreis".to_owned(), json!(25));
    input.insert("page".to_owned(), json!(1));
    input.insert("size".to_owned(), json!(25));
    input
}

fn build_jobs_params(input: &Map<String, Value>) -> Map<String, Value> {
    let mut params = Map::new();
    for key in INPUT_KEYS {
        if let Some(value) = input.get(*key) {
            if !value.is_null() && value.as_str() != Some("") {
                params.insert((*key).to_owned(), value.clone());
            }
        }
    }
    params
}

fn js_truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Number(value)) => value.as_f64().is_some_and(|value| value != 0.0),
        Some(Value::String(value)) => !value.is_empty(),
        Some(_) => true,
    }
}

fn get_jobs(response: &Value) -> &[Value] {
    if let Some(jobs) = response
        .pointer("/data/stellenangebote")
        .and_then(Value::as_array)
    {
        return jobs;
    }
    if let Some(jobs) = response.get("stellenangebote").and_then(Value::as_array) {
        return jobs;
    }
    eprintln!("Unexpected Arbeitsagentur Jobs response shape: expected \"data.stellenangebote\" or \"stellenangebote\" array.");
    &[]
}

fn get_metadata(response: &Value) -> &Value {
    response
        .get("data")
        .filter(|data| !data.is_null())
        .unwrap_or(response)
}

fn to_dataset_job(job: &Value) -> Value {
    let mut dataset_job = job.as_object().cloned().unwrap_or_default();
    let location = job.get("arbeitsort");
    let location_fields = location.and_then(Value::as_object);
    let coordinates = location_fields
        .and_then(|fields| fields.get("koordinaten"))
        .and_then(Value::as_object);

    dataset_job.insert("title".to_owned(), value_or_null(job.get("titel")));
    dataset_job.insert("occupation".to_owned(), value_or_null(job.get("beruf")));
    dataset_job.insert(
        "company_name".to_owned(),
        value_or_null(job.get("arbeitgeber")),
    );
    dataset_job.insert(
        "location_formatted".to_owned(),
        get_formatted_location(location)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    dataset_job.insert(
        "location_city".to_owned(),
        value_or_null(location_fields.and_then(|fields| fields.get("ort"))),
    );
    dataset_job.insert(
        "postal_code".to_owned(),
        value_or_null(location_fields.and_then(|fields| fields.get("plz"))),
    );
    dataset_job.insert(
        "region".to_owned(),
        value_or_null(location_fields.and_then(|fields| fields.get("region"))),
    );
    dataset_job.insert(
        "country".to_owned(),
        value_or_null(location_fields.and_then(|fields| fields.get("land"))),
    );
    dataset_job.insert(
        "published_date".to_owned(),
        value_or_null(job.get("aktuelleVeroeffentlichungsdatum")),
    );
    dataset_job.insert(
        "start_date".to_owned(),
        value_or_null(job.get("eintrittsdatum")),
    );
    dataset_job.insert("job_url".to_owned(), value_or_null(job.get("externeUrl")));
    dataset_job.insert(
        "reference_number".to_owned(),
        value_or_null(job.get("refnr")),
    );
    dataset_job.insert(
        "distance_km".to_owned(),
        value_or_null(location_fields.and_then(|fields| fields.get("entfernung"))),
    );
    dataset_job.insert(
        "latitude".to_owned(),
        coordinates
            .and_then(|coordinates| coordinates.get("lat"))
            .filter(|value| value.is_number())
            .cloned()
            .unwrap_or(Value::Null),
    );
    dataset_job.insert(
        "longitude".to_owned(),
        coordinates
            .and_then(|coordinates| coordinates.get("lon"))
            .filter(|value| value.is_number())
            .cloned()
            .unwrap_or(Value::Null),
    );
    Value::Object(dataset_job)
}

fn get_formatted_location(location: Option<&Value>) -> Option<String> {
    let location = location?;
    if let Some(location) = location.as_str() {
        return Some(location.to_owned());
    }
    let fields = location.as_object()?;
    let parts = ["plz", "ort", "region", "land"]
        .iter()
        .filter_map(|key| fields.get(*key).and_then(Value::as_str))
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn value_or_null(value: Option<&Value>) -> Value {
    value.cloned().unwrap_or(Value::Null)
}

async fn run_actor(config: Config) -> Result<()> {
    let apify_http = Client::builder().timeout(APIFY_TIMEOUT).build()?;
    let scrappa_http = Client::builder().build()?;
    let apify = ApifyClient::new(apify_http, &config);
    let scrappa = ScrappaClient::new(
        scrappa_http,
        config.scrappa_api_base.clone(),
        config.scrappa_api_key.clone(),
    );

    let input = normalize_input(apify.get_input().await?.as_ref());
    if !js_truthy(input.get("was")) {
        bail!("Arbeitsagentur jobs search keyword is required.");
    }
    let query = query_value(
        input
            .get("was")
            .expect("normalized input always contains was"),
    );
    println!("Searching Arbeitsagentur Jobs for: \"{query}\"");

    let response = scrappa.get_jobs(&build_jobs_params(&input)).await.map_err(|error| {
        if error.downcast_ref::<ScrappaFailure>().is_some_and(|error| matches!(error, ScrappaFailure::Timeout)) {
            anyhow!("{error}. The Arbeitsagentur Jobs request exceeded the {}s Scrappa API timeout. Try again or refine the query.", SCRAPPA_REQUEST_TIMEOUT.as_secs())
        } else {
            error
        }
    })?;
    let jobs = get_jobs(&response);
    let dataset_jobs = jobs.iter().map(to_dataset_job).collect::<Vec<_>>();

    let saved = if dataset_jobs.is_empty() {
        println!("No Arbeitsagentur job results found for the given search criteria");
        0
    } else {
        let saved = apify.push_dataset_items(&dataset_jobs).await?;
        println!(
            "Saved {saved} of {} Arbeitsagentur job result(s)",
            dataset_jobs.len()
        );
        saved
    };

    apify.put_record("OUTPUT", &response).await?;
    println!("Arbeitsagentur Jobs search completed successfully");

    let metadata = get_metadata(&response);
    let first_job = jobs.first().map(|job| {
        json!({
            "title": job.get("titel").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
            "company": job.get("arbeitgeber").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
            "location": get_formatted_location(job.get("arbeitsort")).map(Value::String).unwrap_or(Value::Null),
            "reference_number": job.get("refnr").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        })
    });
    let summary = json!({
        "jobs": jobs.len(),
        "saved": saved,
        "total_jobs": metadata.get("maxErgebnisse").filter(|value| !value.is_null()).cloned().unwrap_or(json!(jobs.len())),
        "page": metadata.get("page").filter(|value| !value.is_null()).cloned().unwrap_or_else(|| input.get("page").cloned().unwrap_or(Value::Null)),
        "size": metadata.get("size").filter(|value| !value.is_null()).cloned().unwrap_or_else(|| input.get("size").cloned().unwrap_or(Value::Null)),
        "query": input.get("was"),
        "location": input.get("wo").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "first_job": first_job,
    });
    println!("Results summary: {}", serde_json::to_string(&summary)?);
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match Config::from_env() {
        Ok(config) => match run_actor(config).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Actor failed: {error:#}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("Actor failed: {error:#}");
            ExitCode::FAILURE
        }
    }
}
