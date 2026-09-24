use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Method, Response, StatusCode};
use serde_json::{json, Value};
use std::{
    env,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::sleep;
use url::Url;

use crate::challenges::{js_string, SearchRequest};

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
pub(crate) const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: usize = 8;
const APIFY_MIN_RETRY_DELAY: Duration = Duration::from_millis(500);
const APIFY_CHARGE_IDEMPOTENCY_HEADER: &str = "idempotency-key";
static CHARGE_IDEMPOTENCY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum ApifyRetryPolicy<'a> {
    Safe,
    WithIdempotencyKey(&'a str),
    Never,
}

fn charge_idempotency_key(run_id: &str, event_name: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = CHARGE_IDEMPOTENCY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{run_id}-{event_name}-{timestamp}-{sequence}")
}

#[derive(Clone, Debug)]
pub struct ActorConfig {
    pub apify_api_base_url: Url,
    pub scrappa_api_base_url: Url,
    pub default_key_value_store_id: String,
    pub default_dataset_id: String,
    pub actor_run_id: String,
    pub input_key: String,
    pub apify_token: String,
    pub scrappa_api_key: String,
}

impl ActorConfig {
    pub fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|key| !key.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            scrappa_api_key,
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

pub struct ActorClient {
    config: ActorConfig,
    apify_http: reqwest::Client,
    scrappa_http: reqwest::Client,
}

impl ActorClient {
    pub fn new(config: ActorConfig) -> Result<Self> {
        let apify_http = reqwest::Client::builder()
            .build()
            .context("Could not create Apify HTTP client")?;
        let scrappa_http = reqwest::Client::builder()
            .build()
            .context("Could not create Scrappa HTTP client")?;
        Ok(Self {
            config,
            apify_http,
            scrappa_http,
        })
    }

    async fn send_apify_json(
        &self,
        method: Method,
        url: Url,
        body: Option<&Value>,
        operation: &str,
        retry_policy: ApifyRetryPolicy<'_>,
    ) -> Result<Response> {
        let max_retries = match retry_policy {
            ApifyRetryPolicy::Never => 0,
            ApifyRetryPolicy::Safe | ApifyRetryPolicy::WithIdempotencyKey(_) => APIFY_MAX_RETRIES,
        };
        for attempt in 0..=max_retries {
            let mut request = self
                .apify_http
                .request(method.clone(), url.clone())
                .bearer_auth(&self.config.apify_token)
                .header(reqwest::header::ACCEPT, "application/json")
                .timeout(APIFY_REQUEST_TIMEOUT);
            if let ApifyRetryPolicy::WithIdempotencyKey(key) = retry_policy {
                request = request.header(APIFY_CHARGE_IDEMPOTENCY_HEADER, key);
            }
            if let Some(body) = body {
                request = request.json(body);
            }

            match request.send().await {
                Ok(response) if response.status().is_success() => {
                    return Ok(response);
                }
                Ok(response) if is_retryable_status(response.status()) && attempt < max_retries => {
                    eprintln!(
                        "Apify API request for {operation} failed with {}; retrying ({}/{max_retries})",
                        response.status(),
                        attempt + 1
                    );
                }
                Ok(response) => return Err(apify_response_error(response, operation).await),
                Err(error) if attempt < max_retries => {
                    eprintln!(
                        "Apify API request for {operation} failed: {error}; retrying ({}/{max_retries})",
                        attempt + 1
                    );
                }
                Err(error) => {
                    return Err(anyhow!(
                        "Apify API request failed while trying to {operation}: {error}"
                    ));
                }
            }

            sleep(apify_retry_delay(attempt)).await;
        }

        unreachable!("the retry loop returns after its final attempt")
    }

    async fn apify_json(
        &self,
        method: Method,
        url: Url,
        body: Option<&Value>,
        operation: &str,
        retry_policy: ApifyRetryPolicy<'_>,
    ) -> Result<Value> {
        let response = self
            .send_apify_json(method, url, body, operation, retry_policy)
            .await?;
        response.json().await.with_context(|| {
            format!("Apify API response while trying to {operation} is not valid JSON")
        })
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &[
                "v2",
                "key-value-stores",
                &self.config.default_key_value_store_id,
                "records",
                &self.config.input_key,
            ],
        )?;
        let response = self
            .send_apify_json(
                Method::GET,
                url,
                None,
                "fetch Actor input",
                ApifyRetryPolicy::Safe,
            )
            .await;
        let response = match response {
            Ok(response) => response,
            Err(error) if error.to_string().contains("(404)") => return Ok(None),
            Err(error) => return Err(error),
        };
        response
            .json()
            .await
            .map(Some)
            .context("Actor input record is not valid JSON")
    }

    pub(crate) async fn get_run_pricing(&self) -> Result<Value> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "actor-runs", &self.config.actor_run_id],
        )?;
        self.apify_json(
            Method::GET,
            url,
            None,
            "fetch Actor run pricing",
            ApifyRetryPolicy::Safe,
        )
        .await
    }

    pub(crate) async fn store_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "datasets", &self.config.default_dataset_id, "items"],
        )?;
        self.send_apify_json(
            Method::POST,
            url,
            Some(&json!(items)),
            "store dataset items",
            ApifyRetryPolicy::Never,
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn charge_event(&self, event_name: &str, count: usize) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &["v2", "actor-runs", &self.config.actor_run_id, "charge"],
        )?;
        let idempotency_key = charge_idempotency_key(&self.config.actor_run_id, event_name);
        self.send_apify_json(
            Method::POST,
            url,
            Some(&json!({ "eventName": event_name, "count": count })),
            "charge Actor run events",
            ApifyRetryPolicy::WithIdempotencyKey(&idempotency_key),
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn set_output(&self, output: &Value) -> Result<()> {
        let url = endpoint_url(
            &self.config.apify_api_base_url,
            &[
                "v2",
                "key-value-stores",
                &self.config.default_key_value_store_id,
                "records",
                "OUTPUT",
            ],
        )?;
        self.send_apify_json(
            Method::PUT,
            url,
            Some(output),
            "write OUTPUT",
            ApifyRetryPolicy::Safe,
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn search_challenges(&self, request: &SearchRequest) -> Result<Value> {
        let mut url = endpoint_url(
            &self.config.scrappa_api_base_url,
            &["tiktok", "challenges", "search"],
        )?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("keywords", &request.keyword);
            if let Some(count) = request.count {
                query.append_pair("count", &count.to_string());
            }
        }

        let response = self
            .scrappa_http
            .get(url)
            .header("X-API-Key", &self.config.scrappa_api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    anyhow!(
                        "Scrappa API request timed out after {}ms",
                        SCRAPPA_REQUEST_TIMEOUT.as_millis()
                    )
                } else {
                    anyhow!("Scrappa API request failed: {error}")
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let fallback = status
                .canonical_reason()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            let body = response.text().await.map_err(|error| {
                if error.is_timeout() {
                    anyhow!(
                        "Scrappa API request timed out after {}ms",
                        SCRAPPA_REQUEST_TIMEOUT.as_millis()
                    )
                } else {
                    anyhow!("Scrappa API error response could not be read: {error}")
                }
            })?;
            let message = scrappa_error_message(&body, &fallback);
            bail!("Scrappa API error ({}): {message}", status.as_u16());
        }

        response.json().await.map_err(|error| {
            if error.is_timeout() {
                anyhow!(
                    "Scrappa API request timed out after {}ms",
                    SCRAPPA_REQUEST_TIMEOUT.as_millis()
                )
            } else {
                anyhow!("Scrappa API response was not valid JSON: {error}")
            }
        })
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn apify_retry_delay(attempt: usize) -> Duration {
    APIFY_MIN_RETRY_DELAY.saturating_mul(2_u32.saturating_pow(attempt.min(16) as u32))
}

async fn apify_response_error(response: Response, operation: &str) -> anyhow::Error {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        format!("HTTP {}", status.as_u16())
    } else {
        body
    };
    anyhow!(
        "Apify API error ({}) while trying to {operation}: {detail}",
        status.as_u16()
    )
}

fn scrappa_error_message(body: &str, fallback: &str) -> String {
    if body.is_empty() {
        return fallback.to_owned();
    }

    if let Ok(error_data) = serde_json::from_str::<Value>(body) {
        let mut message = error_data
            .get("message")
            .filter(|value| !value.is_null())
            .map(js_string)
            .unwrap_or_else(|| fallback.to_owned());
        if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .map(|(field, messages)| {
                    let messages = messages
                        .as_array()
                        .map(|messages| {
                            messages
                                .iter()
                                .map(js_string)
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_else(|| js_string(messages));
                    format!("{field}: {messages}")
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

    body.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}
