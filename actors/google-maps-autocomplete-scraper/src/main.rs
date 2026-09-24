use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Value};
use std::{env, time::Duration};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: usize = 8;
const APIFY_RETRY_BASE_DELAY: Duration = Duration::from_millis(500);
const DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    scrappa_api_key: String,
    default_key_value_store_id: String,
    default_dataset_id: String,
    actor_run_id: String,
    input_key: String,
    apify_token: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("SCRAPPA_API_KEY environment variable is not set."))?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            scrappa_api_key,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.trim().is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

fn autocomplete_url(input: &Value, api_base_url: &Url) -> Result<(String, Url)> {
    let query = input
        .get("query")
        .and_then(Value::as_str)
        .filter(|query| !query.is_empty())
        .ok_or_else(|| anyhow!("Search query is required"))?
        .to_owned();
    let mut url = endpoint_url(api_base_url, &["maps", "autocomplete"])?;
    url.query_pairs_mut().append_pair("query", &query);
    Ok((query, url))
}

fn status_reason(status: StatusCode) -> String {
    status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()))
}

fn scrappa_error_message(status: StatusCode, body: &str) -> String {
    let fallback = status_reason(status);
    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let mut message = error_data
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(&fallback)
            .to_owned();
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    let messages = messages.as_array()?;
                    let messages = messages
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>();
                    (!messages.is_empty()).then(|| format!("{field}: {}", messages.join(", ")))
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details);
            }
        }
        return message;
    }

    if body.is_empty() {
        return fallback;
    }

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = response.text().await.unwrap_or_default();
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "{operation} failed with {} {reason}{detail}",
            status.as_u16(),
        );
    }
    response
        .json()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

async fn send_apify_request(
    build_request: impl FnMut() -> RequestBuilder,
) -> std::result::Result<Response, reqwest::Error> {
    send_request_with_retries(
        build_request,
        APIFY_REQUEST_TIMEOUT,
        APIFY_MAX_RETRIES,
        APIFY_RETRY_BASE_DELAY,
    )
    .await
}

async fn send_apify_request_once(
    build_request: impl FnOnce() -> RequestBuilder,
) -> std::result::Result<Response, reqwest::Error> {
    build_request().timeout(APIFY_REQUEST_TIMEOUT).send().await
}

async fn send_request_with_retries(
    mut build_request: impl FnMut() -> RequestBuilder,
    timeout: Duration,
    max_retries: usize,
    initial_retry_delay: Duration,
) -> std::result::Result<Response, reqwest::Error> {
    let mut retries = 0;
    loop {
        let response = match build_request().timeout(timeout).send().await {
            Ok(response) => response,
            Err(_) if retries < max_retries => {
                tokio::time::sleep(retry_delay(initial_retry_delay, retries)).await;
                retries += 1;
                continue;
            }
            Err(error) => return Err(error),
        };

        if retries < max_retries
            && (response.status() == StatusCode::TOO_MANY_REQUESTS
                || response.status().is_server_error())
        {
            drop(response);
            tokio::time::sleep(retry_delay(initial_retry_delay, retries)).await;
            retries += 1;
            continue;
        }

        return Ok(response);
    }
}

fn retry_delay(initial_delay: Duration, retries: usize) -> Duration {
    initial_delay.saturating_mul(2_u32.saturating_pow(retries as u32))
}

async fn get_input(client: &Client, config: &ActorConfig) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let response = send_apify_request(|| client.get(url.clone()).bearer_auth(&config.apify_token))
        .await
        .context("Apify INPUT request failed")?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(Value::Null);
    }
    response_json(response, "Apify INPUT request").await
}

async fn fetch_suggestions(client: &Client, config: &ActorConfig, url: &Url) -> Result<Value> {
    fetch_suggestions_with_timeout(client, config, url, SCRAPPA_REQUEST_TIMEOUT).await
}

async fn fetch_suggestions_with_timeout(
    client: &Client,
    config: &ActorConfig,
    url: &Url,
    timeout: Duration,
) -> Result<Value> {
    let response = client
        .get(url.clone())
        .header(reqwest::header::ACCEPT, "application/json")
        .header("X-API-Key", &config.scrappa_api_key)
        .timeout(timeout)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() || error.to_string().contains("aborted") {
                anyhow!(
                    "Scrappa API request timed out after {}ms",
                    timeout.as_millis()
                )
            } else {
                anyhow!("Scrappa API request failed: {error}")
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let message = scrappa_error_message(status, &body);
        bail!("Scrappa API error ({}): {message}", status.as_u16());
    }
    response
        .json()
        .await
        .context("Scrappa API response was not valid JSON")
}

struct DatasetBudget {
    dataset_item_price_usd: f64,
    charged_usd: f64,
    max_total_charge_usd: Option<f64>,
}

impl DatasetBudget {
    fn from_run(run: &Value) -> Result<Option<Self>> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing_model = data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Apify run pricing model is missing"))?;
        if pricing_model != "PAY_PER_EVENT" {
            return Ok(None);
        }

        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let dataset_item_price_usd = events
            .get(DATASET_ITEM_EVENT)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
        let max_total_charge_usd = match data.pointer("/options/maxTotalChargeUsd") {
            None | Some(Value::Null) => None,
            Some(value) => {
                let max_total_charge_usd = value
                    .as_f64()
                    .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
                if !max_total_charge_usd.is_finite() || max_total_charge_usd < 0.0 {
                    bail!("Apify run returned an invalid spending limit");
                }
                (max_total_charge_usd > 0.0).then_some(max_total_charge_usd)
            }
        };
        if !dataset_item_price_usd.is_finite() || dataset_item_price_usd < 0.0 {
            bail!("Apify run returned invalid charging values");
        }

        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
        let mut charged_usd = 0.0;
        for (event_name, count) in charged_event_counts {
            let count = count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            if count == 0 {
                continue;
            }
            let price = events
                .get(event_name)
                .and_then(|event| event.get("eventPriceUsd"))
                .and_then(Value::as_f64)
                .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
            if !price.is_finite() || price < 0.0 {
                bail!("Invalid price for charged event {event_name}");
            }
            charged_usd += price * count as f64;
        }
        if !charged_usd.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        Ok(Some(Self {
            dataset_item_price_usd,
            charged_usd,
            max_total_charge_usd,
        }))
    }

    fn affordable_items(&self, requested: usize) -> usize {
        let Some(max_total_charge_usd) = self.max_total_charge_usd else {
            return requested;
        };
        if self.dataset_item_price_usd == 0.0 {
            return requested;
        }
        let tolerance = f64::EPSILON * max_total_charge_usd.max(1.0);
        (1..=requested)
            .take_while(|count| {
                self.charged_usd + *count as f64 * self.dataset_item_price_usd
                    <= max_total_charge_usd + tolerance
            })
            .count()
    }
}

async fn get_dataset_budget(
    client: &Client,
    config: &ActorConfig,
) -> Result<Option<DatasetBudget>> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = send_apify_request(|| client.get(url.clone()).bearer_auth(&config.apify_token))
        .await
        .context("Apify run pricing request failed")?;
    let run = response_json(response, "Apify run pricing request").await?;
    DatasetBudget::from_run(&run)
}

async fn push_dataset_items(
    client: &Client,
    config: &ActorConfig,
    items: &[Value],
) -> Result<usize> {
    if items.is_empty() {
        return Ok(0);
    }

    let budget = get_dataset_budget(client, config).await?;
    let saved_items = budget
        .as_ref()
        .map_or(items.len(), |budget| budget.affordable_items(items.len()));
    if saved_items == 0 {
        return Ok(0);
    }

    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = send_apify_request_once(|| {
        client
            .post(url.clone())
            .bearer_auth(&config.apify_token)
            .json(&items[..saved_items])
    })
    .await
    .context("Apify dataset write failed")?;
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = response.text().await.unwrap_or_default();
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "Apify dataset write failed with {} {reason}{detail}",
            status.as_u16()
        );
    }
    Ok(saved_items)
}

async fn set_output(client: &Client, config: &ActorConfig, output: &Value) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            "OUTPUT",
        ],
    )?;
    let response = send_apify_request(|| {
        client
            .put(url.clone())
            .bearer_auth(&config.apify_token)
            .json(output)
    })
    .await
    .context("Apify OUTPUT write failed")?;
    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    bail!(
        "Apify OUTPUT write failed with {} {reason}{detail}",
        status.as_u16()
    );
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let input = get_input(client, config).await?;
    let (query, url) = autocomplete_url(&input, &config.scrappa_api_base_url)?;
    println!("Getting autocomplete suggestions for: \"{query}\"");

    let response = fetch_suggestions(client, config, &url).await?;
    let suggestions = response
        .get("suggestions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let saved_count = if suggestions.is_empty() {
        println!("No autocomplete suggestions found for the given query");
        0
    } else {
        let saved_count = push_dataset_items(client, config, &suggestions).await?;
        println!("Found {} suggestions", suggestions.len());
        if saved_count < suggestions.len() {
            println!(
                "PAY_PER_EVENT spending limit allowed {saved_count} of {} suggestion dataset item(s)",
                suggestions.len()
            );
        }
        saved_count
    };

    set_output(client, config, &response).await?;

    let summary = json!({
        "query": query,
        "suggestions_found": suggestions.len(),
    });
    println!("Autocomplete completed: {}", summary);
    if saved_count == 0 && !suggestions.is_empty() {
        println!("No suggestions were saved because the PAY_PER_EVENT spending limit was reached");
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        let message = format!("{error:#}");
        eprintln!("Failed: {message}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let config = ActorConfig::from_env()?;
    let client = Client::builder()
        .build()
        .context("Could not create HTTP client")?;
    run_actor(&client, &config).await
}

#[cfg(test)]
mod tests;
