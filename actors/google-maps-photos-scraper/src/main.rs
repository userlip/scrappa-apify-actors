mod apify;
mod business_id;
mod scrappa;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use anyhow::{anyhow, bail, Context, Result};
use apify::ApifyClient;
use business_id::{get_business_id_requests, BusinessIdRequest};
use scrappa::{photo_results, ScrappaClient, ScrappaError};
use serde_json::{json, Map, Value};
use std::{env, time::Duration};
use tokio::time::timeout;
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const ACTOR_TIMEOUT: Duration = Duration::from_secs(720);

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
    scrappa_api_key: String,
    max_total_charge_usd: Option<f64>,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key: required_env("SCRAPPA_API_KEY")?,
            max_total_charge_usd: optional_f64_env("ACTOR_MAX_TOTAL_CHARGE_USD")?,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value =
        env::var(name).with_context(|| format!("{name} environment variable is not set."))?;
    if value.trim().is_empty() {
        bail!("{name} environment variable is not set.");
    }
    Ok(value)
}

fn optional_f64_env(name: &str) -> Result<Option<f64>> {
    let Ok(value) = env::var(name) else {
        return Ok(None);
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    let parsed = value
        .parse::<f64>()
        .with_context(|| format!("{name} must be a valid number"))?;
    if !parsed.is_finite() || parsed < 0.0 {
        bail!("{name} must be a non-negative finite number");
    }
    Ok(Some(parsed))
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

pub(crate) fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

fn dataset_photo(photo: &Value, input_business_id: &str, business_id: &str) -> Value {
    let mut fields = match photo {
        Value::Object(fields) => fields.clone(),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        Value::String(value) => value
            .chars()
            .enumerate()
            .map(|(index, character)| (index.to_string(), Value::String(character.to_string())))
            .collect(),
        _ => Map::new(),
    };
    fields.insert(
        "input_business_id".to_owned(),
        Value::String(input_business_id.to_owned()),
    );
    fields.insert(
        "business_id".to_owned(),
        Value::String(business_id.to_owned()),
    );
    Value::Object(fields)
}

fn input_error_output(input_business_id: &str, business_id: &str, error: &str) -> Value {
    json!({
        "success": false,
        "input_business_id": input_business_id,
        "business_id": business_id,
        "error": error
    })
}

fn api_input_error(status_code: u16) -> Option<&'static str> {
    match status_code {
        404 => Some("Business not found"),
        422 => Some("Invalid input"),
        _ => None,
    }
}

fn business_summary(
    input_business_id: &str,
    business_id: &str,
    total: usize,
    next_page: Value,
    error: Option<&str>,
) -> Value {
    let mut summary = json!({
        "input_business_id": input_business_id,
        "business_id": business_id,
        "total": total,
        "nextPage": next_page,
    });
    if let Some(error) = error {
        summary["error"] = Value::String(error.to_owned());
    }
    summary
}

async fn record_business_error(
    apify: &mut ApifyClient<'_>,
    request: &BusinessIdRequest,
    business_id: &str,
    error: &str,
    run_results: &mut Vec<Value>,
    first_output: &mut Option<Value>,
) -> Result<()> {
    let output = input_error_output(&request.input_business_id, business_id, error);
    apify.push_dataset_items(&[output]).await?;
    run_results.push(business_summary(
        &request.input_business_id,
        business_id,
        0,
        Value::Null,
        Some(error),
    ));
    if first_output.is_none() {
        *first_output = Some(json!({ "photos": [], "total": 0, "nextPage": null, "error": error }));
    }
    Ok(())
}

async fn run_actor(client: &reqwest::Client, config: &ActorConfig) -> Result<()> {
    let mut apify = ApifyClient::new(client, config);
    let input = apify.get_input().await?;
    let requests = get_business_id_requests(input.as_ref())?;
    if requests.is_empty() {
        bail!("At least one Business ID is required. Provide business_ids or legacy business_id.");
    }

    let scrappa = ScrappaClient::new(
        client,
        &config.scrappa_api_base_url,
        &config.scrappa_api_key,
    );
    let mut run_results = Vec::new();
    let mut succeeded = 0;
    let mut failed = 0;
    let mut total_photos = 0;
    let mut first_output = None;

    println!(
        "Fetching Google Maps photos for {} business{}",
        requests.len(),
        if requests.len() == 1 { "" } else { "es" }
    );

    for request in &requests {
        let Some(business_id) = request.business_id.as_deref() else {
            let error = request
                .validation_error
                .as_deref()
                .unwrap_or("Invalid business input");
            println!("Invalid business input: {error}");
            record_business_error(
                &mut apify,
                request,
                &request.input_business_id,
                error,
                &mut run_results,
                &mut first_output,
            )
            .await?;
            failed += 1;
            continue;
        };

        if request.source == Some("url") {
            println!("Extracted Google Maps business identifier from URL: {business_id}");
        }
        println!("Fetching photos for business: {business_id}");

        let response = match scrappa.get_photos(business_id, input.as_ref()).await {
            Ok(response) => response,
            Err(error) => {
                let status_code = error
                    .downcast_ref::<ScrappaError>()
                    .and_then(|error| error.status_code);
                if let Some(error) = status_code.and_then(api_input_error) {
                    println!("Photos request returned {status_code:?}: {business_id}");
                    record_business_error(
                        &mut apify,
                        request,
                        business_id,
                        error,
                        &mut run_results,
                        &mut first_output,
                    )
                    .await?;
                    failed += 1;
                    continue;
                }
                return Err(error);
            }
        };

        let (photos, next_page) = photo_results(&response)?;
        let dataset_photos = photos
            .iter()
            .map(|photo| dataset_photo(photo, &request.input_business_id, business_id))
            .collect::<Vec<_>>();
        let mut saved_photo_count = dataset_photos.len();
        if !dataset_photos.is_empty() {
            let saved = apify.push_dataset_items(&dataset_photos).await?;
            saved_photo_count = saved.saved_count;
            if saved.charge_limit_reached {
                println!(
                    "Apify PPE charge limit reached after saving {} photo item(s)",
                    saved.saved_count
                );
            }
            println!("Found {} photos for {business_id}", dataset_photos.len());
        } else {
            println!("No photos found for business: {business_id}");
        }

        run_results.push(business_summary(
            &request.input_business_id,
            business_id,
            dataset_photos.len(),
            next_page.clone(),
            None,
        ));
        if first_output.is_none() {
            first_output = Some(json!({
                "photos": &dataset_photos[..saved_photo_count],
                "total": photos.len(),
                "nextPage": next_page,
            }));
        }
        succeeded += 1;
        total_photos += dataset_photos.len();
    }

    let output = if requests.len() == 1 {
        first_output.unwrap_or_else(|| json!({ "photos": [], "total": 0, "nextPage": null }))
    } else {
        json!({
            "requested": requests.len(),
            "succeeded": succeeded,
            "failed": failed,
            "total_photos": total_photos,
            "results": run_results,
        })
    };
    apify.set_output_value(&output).await?;
    println!("Photos extraction completed");
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Actor failed: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = reqwest::Client::builder()
        .timeout(APIFY_REQUEST_TIMEOUT)
        .build()
        .context("Could not create HTTP client")?;
    timeout(ACTOR_TIMEOUT, run_actor(&client, &config))
        .await
        .map_err(|_| anyhow!("Actor timed out after {}s", ACTOR_TIMEOUT.as_secs()))??;
    Ok(())
}
