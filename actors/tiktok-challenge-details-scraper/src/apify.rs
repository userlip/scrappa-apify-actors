use std::{collections::BTreeMap, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, RequestBuilder, Response, StatusCode};
use serde_json::{json, Value};
use tokio::time::sleep;
use url::Url;

use crate::config::{
    ActorConfig, APIFY_MAX_RETRIES, APIFY_REQUEST_TIMEOUT, CHALLENGE_DETAIL_CHARGE_EVENT,
    DEFAULT_DATASET_ITEM_EVENT, OUTPUT_KEY,
};

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_millis((500_u64 << attempt.min(6)).min(30_000))
}

#[cfg(test)]
pub(crate) fn max_retry_sequence_duration() -> Duration {
    let retry_delays = (0..APIFY_MAX_RETRIES).fold(Duration::ZERO, |elapsed, attempt| {
        elapsed + retry_delay(attempt)
    });
    APIFY_REQUEST_TIMEOUT * (APIFY_MAX_RETRIES as u32 + 1) + retry_delays
}

#[derive(Default)]
pub(crate) struct PpeBudget {
    pub(crate) is_pay_per_event: bool,
    max_total_charge_usd: f64,
    pub(crate) event_prices: BTreeMap<String, f64>,
    pub(crate) charged_event_counts: BTreeMap<String, u64>,
}

impl PpeBudget {
    pub(crate) fn from_run(run: &Value) -> Result<Self> {
        let data = run
            .get("data")
            .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
        let pricing = data.get("pricingInfo");
        let is_pay_per_event = pricing
            .and_then(|pricing| pricing.get("pricingModel"))
            .and_then(Value::as_str)
            == Some("PAY_PER_EVENT");
        if !is_pay_per_event {
            return Ok(Self::default());
        }

        let event_prices = pricing
            .and_then(|pricing| pricing.pointer("/pricingPerEvent/actorChargeEvents"))
            .and_then(Value::as_object)
            .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?
            .iter()
            .filter_map(|(name, event)| {
                event
                    .get("eventPriceUsd")
                    .and_then(Value::as_f64)
                    .map(|price| (name.clone(), price))
            })
            .collect::<BTreeMap<_, _>>();
        if event_prices
            .values()
            .any(|price| !price.is_finite() || *price < 0.0)
        {
            bail!("Apify run returned invalid charging values");
        }

        let max_total_charge_usd = data
            .pointer("/options/maxTotalChargeUsd")
            .and_then(Value::as_f64)
            .filter(|charge| *charge != 0.0)
            .unwrap_or(f64::INFINITY);
        if !max_total_charge_usd.is_finite() && max_total_charge_usd != f64::INFINITY {
            bail!("Apify run returned invalid charging values");
        }
        if max_total_charge_usd < 0.0 {
            bail!("Apify run returned invalid charging values");
        }

        let charged_event_counts = data
            .get("chargedEventCounts")
            .and_then(Value::as_object)
            .map(|events| {
                events
                    .iter()
                    .map(|(name, count)| {
                        count
                            .as_u64()
                            .map(|count| (name.clone(), count))
                            .ok_or_else(|| anyhow!("Invalid charged event count for {name}"))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()
            })
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            is_pay_per_event,
            max_total_charge_usd,
            event_prices,
            charged_event_counts,
        })
    }

    fn total_charged_amount(&self) -> f64 {
        let amount = self
            .charged_event_counts
            .iter()
            .map(|(name, count)| {
                self.event_prices.get(name).copied().unwrap_or(0.0) * *count as f64
            })
            .sum::<f64>();
        (amount * 1_000_000.0).round() / 1_000_000.0
    }

    fn max_charges_by_price(&self, price: f64) -> usize {
        if price == 0.0 || !price.is_finite() {
            return usize::MAX;
        }
        let remaining = self.max_total_charge_usd - self.total_charged_amount();
        if !remaining.is_finite() {
            return if remaining.is_sign_positive() {
                usize::MAX
            } else {
                0
            };
        }
        let max_count = (remaining / price + 1e-12).floor();
        max_count.max(0.0).min(usize::MAX as f64) as usize
    }

    #[cfg(test)]
    pub(crate) fn event_capacity(&self, event_name: &str) -> usize {
        if !self.is_pay_per_event {
            return usize::MAX;
        }
        self.event_prices
            .get(event_name)
            .copied()
            .map(|price| self.max_charges_by_price(price))
            .unwrap_or(usize::MAX)
    }

    pub(crate) fn result_capacity(&self) -> usize {
        if !self.is_pay_per_event {
            return usize::MAX;
        }
        let custom_event_price = self
            .event_prices
            .get(CHALLENGE_DETAIL_CHARGE_EVENT)
            .copied()
            .unwrap_or(0.0);
        let dataset_event_price = self
            .event_prices
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0.0);
        self.max_charges_by_price(custom_event_price + dataset_event_price)
    }

    pub(crate) fn can_push_dataset_item(&self) -> bool {
        self.result_capacity() > 0
    }

    pub(crate) fn record_successful_charge(&mut self, event_name: &str, count: u64) {
        if self.is_pay_per_event && self.event_prices.contains_key(event_name) {
            *self
                .charged_event_counts
                .entry(event_name.to_owned())
                .or_default() += count;
        }
    }
}

pub(crate) struct ApifyClient {
    http: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub(crate) fn new(config: &ActorConfig) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: config.apify_api_base_url.clone(),
            token: config.apify_token.clone(),
        })
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL must be a base URL"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(resource.iter().copied()));
        Ok(url)
    }

    fn request(&self, method: reqwest::Method, url: Url) -> RequestBuilder {
        self.http
            .request(method, url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
    }

    async fn send_with_retries<F>(&self, operation: &str, mut build_request: F) -> Result<Response>
    where
        F: FnMut() -> RequestBuilder,
    {
        for attempt in 0..=APIFY_MAX_RETRIES {
            match build_request().send().await {
                Ok(response)
                    if attempt < APIFY_MAX_RETRIES
                        && (response.status() == StatusCode::TOO_MANY_REQUESTS
                            || response.status().is_server_error()) =>
                {
                    sleep(retry_delay(attempt)).await;
                }
                Ok(response) => return Ok(response),
                Err(error)
                    if attempt < APIFY_MAX_RETRIES
                        && (error.is_connect() || error.is_timeout() || error.is_request()) =>
                {
                    sleep(retry_delay(attempt)).await;
                }
                Err(error) => return Err(error).with_context(|| format!("{operation} failed")),
            }
        }
        unreachable!("the retry loop returns after its final attempt")
    }

    async fn send_once<F>(&self, operation: &str, build_request: F) -> Result<Response>
    where
        F: FnOnce() -> RequestBuilder,
    {
        // Dataset inserts append rows, so retrying after a lost response can duplicate data and PPE charges.
        build_request()
            .send()
            .await
            .with_context(|| format!("{operation} failed"))
    }

    async fn successful_response(&self, response: Response, operation: &str) -> Result<Response> {
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

    pub(crate) async fn get_input(&self, config: &ActorConfig) -> Result<Option<Value>> {
        let url = self.resource_url(&[
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            &config.input_key,
        ])?;
        let response = self
            .send_with_retries("Apify INPUT request", || {
                self.request(reqwest::Method::GET, url.clone())
            })
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        self.successful_response(response, "INPUT request")
            .await?
            .json::<Value>()
            .await
            .context("Apify INPUT record was not valid JSON")
            .map(Some)
    }

    pub(crate) async fn run_budget(&self, config: &ActorConfig) -> Result<PpeBudget> {
        let url = self.resource_url(&["actor-runs", &config.actor_run_id])?;
        let response = self
            .send_with_retries("Apify run pricing request", || {
                self.request(reqwest::Method::GET, url.clone())
            })
            .await?;
        let run = self
            .successful_response(response, "run pricing request")
            .await?
            .json::<Value>()
            .await
            .context("Apify run pricing request returned invalid JSON")?;
        PpeBudget::from_run(&run)
    }

    pub(crate) async fn push_dataset_item(&self, config: &ActorConfig, item: &Value) -> Result<()> {
        let url = self.resource_url(&["datasets", &config.dataset_id, "items"])?;
        let response = self
            .send_once("Apify dataset write", || {
                self.request(reqwest::Method::POST, url.clone()).json(item)
            })
            .await?;
        self.successful_response(response, "dataset write").await?;
        Ok(())
    }

    pub(crate) async fn charge_event(
        &self,
        config: &ActorConfig,
        count: u64,
        idempotency_key: &str,
    ) -> Result<u64> {
        let url = self.resource_url(&["actor-runs", &config.actor_run_id, "charge"])?;
        let response = self
            .send_with_retries("Apify event charge", || {
                self.request(reqwest::Method::POST, url.clone())
                    .header("idempotency-key", idempotency_key)
                    .json(&json!({"eventName": CHALLENGE_DETAIL_CHARGE_EVENT, "count": count}))
            })
            .await?;
        // The raw endpoint returns `{}`; HTTP 201 is its charge result, unlike the SDK ChargeResult.
        if response.status() == StatusCode::CREATED {
            return Ok(count);
        }
        self.successful_response(response, "event charge").await?;
        bail!("Apify event charge returned an unexpected success status")
    }

    pub(crate) async fn set_output(&self, config: &ActorConfig, output: &Value) -> Result<()> {
        let url = self.resource_url(&[
            "key-value-stores",
            &config.key_value_store_id,
            "records",
            OUTPUT_KEY,
        ])?;
        let response = self
            .send_with_retries("Apify OUTPUT write", || {
                self.request(reqwest::Method::PUT, url.clone()).json(output)
            })
            .await?;
        self.successful_response(response, "OUTPUT write").await?;
        Ok(())
    }
}
