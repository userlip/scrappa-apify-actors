use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Method, Response, StatusCode};
use serde_json::Value;
use std::time::Duration;
use url::Url;

pub(crate) const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: u32 = 2;
const APIFY_RETRY_DELAY: Duration = Duration::from_millis(250);
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub(crate) fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments.iter().copied());
    Ok(url)
}

pub(crate) struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub(crate) fn new(base_url: Url, token: String) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url,
            token,
        })
    }

    fn resource_url(&self, segments: &[&str]) -> Result<Url> {
        let mut path = vec!["v2"];
        path.extend_from_slice(segments);
        endpoint_url(&self.base_url, &path)
    }

    async fn request(
        &self,
        method: Method,
        url: Url,
        body: Option<&Value>,
        operation: &str,
    ) -> Result<Response> {
        let retryable_method = is_retryable_method(&method);
        for attempt in 0..=APIFY_MAX_RETRIES {
            let mut request = self
                .client
                .request(method.clone(), url.clone())
                .bearer_auth(&self.token)
                .header(reqwest::header::ACCEPT, "application/json");
            if let Some(body) = body {
                request = request.json(body);
            }

            match request.send().await {
                Ok(response)
                    if retryable_method
                        && is_retryable_status(response.status())
                        && attempt < APIFY_MAX_RETRIES =>
                {
                    eprintln!("{operation} returned {}; retrying", response.status());
                }
                Ok(response) => return Ok(response),
                Err(error)
                    if retryable_method
                        && is_retryable_request(&error)
                        && attempt < APIFY_MAX_RETRIES =>
                {
                    eprintln!("{operation} failed; retrying: {error}");
                }
                Err(error) => return Err(anyhow!("{operation} failed: {error}")),
            }

            tokio::time::sleep(APIFY_RETRY_DELAY * 2_u32.pow(attempt)).await;
        }
        unreachable!("the final Apify request attempt always returns or fails")
    }

    pub(crate) async fn get_run(&self, actor_run_id: &str) -> Result<Value> {
        let url = self.resource_url(&["actor-runs", actor_run_id])?;
        let response = self
            .request(Method::GET, url, None, "Apify run pricing request")
            .await?;
        let response = successful_response(response, "fetch Actor run pricing").await?;
        response
            .json()
            .await
            .context("Actor run pricing response is not valid JSON")
    }

    pub(crate) async fn get_input(&self, store_id: &str, input_key: &str) -> Result<Option<Value>> {
        let url = self.resource_url(&["key-value-stores", store_id, "records", input_key])?;
        let response = self
            .request(Method::GET, url, None, "Apify INPUT request")
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "fetch Actor input").await?;
        Ok(Some(
            response
                .json()
                .await
                .context("Actor input record is not valid JSON")?,
        ))
    }

    pub(crate) async fn push_data(&self, dataset_id: &str, items: &[Value]) -> Result<()> {
        let url = self.resource_url(&["datasets", dataset_id, "items"])?;
        let response = self
            .request(
                Method::POST,
                url,
                Some(&Value::Array(items.to_vec())),
                "Apify dataset write",
            )
            .await?;
        successful_response(response, "store post items").await?;
        Ok(())
    }

    pub(crate) async fn set_output(&self, store_id: &str, output: &Value) -> Result<()> {
        let url = self.resource_url(&["key-value-stores", store_id, "records", "OUTPUT"])?;
        let response = self
            .request(Method::PUT, url, Some(output), "Apify OUTPUT write")
            .await?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }
}

fn is_retryable_method(method: &Method) -> bool {
    matches!(method.as_str(), "GET" | "PUT")
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_retryable_request(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request()
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.is_empty() {
        format!("HTTP {}", status.as_u16())
    } else {
        body
    };
    bail!(
        "Apify API error ({}) while trying to {operation}: {detail}",
        status.as_u16()
    );
}

#[derive(Debug, PartialEq)]
pub(crate) enum DatasetBudget {
    Unlimited,
    Limited(usize),
}

impl DatasetBudget {
    pub(crate) fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        if data
            .pointer("/pricingInfo/pricingModel")
            .and_then(Value::as_str)
            != Some("PAY_PER_EVENT")
        {
            return Ok(Self::Unlimited);
        }
        let events = data
            .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
        let Some(item_price) = events
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
        else {
            return Ok(Self::Unlimited);
        };
        if !item_price.is_finite() || item_price < 0.0 {
            bail!("Apify run returned an invalid dataset item price");
        }
        if item_price == 0.0 {
            return Ok(Self::Unlimited);
        }

        let max_charge = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|value| *value != 0.0)
            .unwrap_or(f64::INFINITY);
        if max_charge.is_nan() || max_charge < 0.0 {
            bail!("Apify run returned an invalid spending limit");
        }
        if max_charge.is_infinite() {
            return Ok(Self::Unlimited);
        }

        let counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let mut spent = 0.0;
        for (event_name, count) in counts {
            let Some(price) = events
                .get(&event_name)
                .and_then(|event| event.get("eventPriceUsd"))
                .and_then(Value::as_f64)
            else {
                continue;
            };
            let count = count
                .as_u64()
                .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
            if !price.is_finite() || price < 0.0 {
                bail!("Invalid price for charged event {event_name}");
            }
            spent += price * count as f64;
        }
        if !spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }

        let tolerance = f64::EPSILON * max_charge.max(1.0);
        let mut affordable = (((max_charge - spent + tolerance) / item_price)
            .floor()
            .max(0.0))
        .min(usize::MAX as f64) as usize;
        while affordable > 0 && spent + affordable as f64 * item_price > max_charge + tolerance {
            affordable -= 1;
        }
        Ok(Self::Limited(affordable))
    }

    pub(crate) fn limit(&self, requested: usize) -> usize {
        match self {
            Self::Unlimited => requested,
            Self::Limited(remaining) => requested.min(*remaining),
        }
    }
}
