use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Method, Response, StatusCode, Url};
use serde_json::{json, Value};
use std::{env, time::Duration};

const API_TIMEOUT: Duration = Duration::from_secs(60);
const PAY_PER_EVENT_MODEL: &str = "PAY_PER_EVENT";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub fn new(base_url: &str, token: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(API_TIMEOUT)
            .build()
            .context("Could not create Apify HTTP client")?;
        Ok(Self {
            client,
            base_url: Url::parse(base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token,
        })
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot be a base URL"))?;
        path.pop_if_empty();
        path.extend(["v2"].into_iter().chain(resource.iter().copied()));
        drop(path);
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
    }

    pub async fn get_input(&self, store_id: &str, input_key: &str) -> Result<Option<Value>> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&["key-value-stores", store_id, "records", input_key])?,
            )
            .send()
            .await
            .context("Failed to retrieve actor input from Apify API")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "input retrieval").await?;
        response
            .json::<Value>()
            .await
            .context("Apify input record was not valid JSON")
            .map(Some)
    }

    pub async fn event_capacity(&self, run_id: &str, event_name: &str) -> Result<Option<usize>> {
        let response = self
            .request(Method::GET, self.resource_url(&["actor-runs", run_id])?)
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = successful_response(response, "run pricing request")
            .await?
            .json::<Value>()
            .await
            .context("Apify run pricing response was not valid JSON")?;
        chargeable_event_capacity(
            &run,
            event_name,
            env::var("ACTOR_MAX_TOTAL_CHARGE_USD").ok().as_deref(),
        )
    }

    pub async fn charge_event(
        &self,
        run_id: &str,
        event_name: &str,
        count: usize,
        idempotency_key: &str,
    ) -> Result<()> {
        if count == 0 {
            return Ok(());
        }
        let url = self.resource_url(&["actor-runs", run_id, "charge"])?;
        for attempt in 0..3 {
            let response = self
                .request(Method::POST, url.clone())
                .header("idempotency-key", idempotency_key)
                .json(&json!({ "eventName": event_name, "count": count }))
                .send()
                .await;

            match response {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) if is_retryable_apify_status(response.status()) && attempt < 2 => {
                    tokio::time::sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
                }
                Ok(response) => {
                    successful_response(response, "charge Jameda review results").await?;
                    return Ok(());
                }
                Err(error) if (error.is_timeout() || error.is_connect()) && attempt < 2 => {
                    tokio::time::sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
                }
                Err(error) => return Err(error).context("Apify charge request failed"),
            }
        }
        unreachable!("the charge retry loop either succeeds or returns an error")
    }

    pub async fn push_dataset_items(&self, dataset_id: &str, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["datasets", dataset_id, "items"])?,
            )
            .json(items)
            .send()
            .await
            .context("Failed to publish dataset items to Apify API")?;
        successful_response(response, "dataset item publication").await?;
        Ok(())
    }

    pub async fn set_output(&self, store_id: &str, output: &Value) -> Result<()> {
        let response = self
            .request(
                Method::PUT,
                self.resource_url(&["key-value-stores", store_id, "records", "OUTPUT"])?,
            )
            .json(output)
            .send()
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "OUTPUT record publication").await?;
        Ok(())
    }

    pub async fn set_status_message(&self, run_id: &str, status_message: &str) -> Result<()> {
        let response = self
            .request(Method::PUT, self.resource_url(&["actor-runs", run_id])?)
            .json(&json!({ "runId": run_id, "statusMessage": status_message }))
            .send()
            .await
            .context("Failed to set Actor run status message")?;
        successful_response(response, "set Actor run status message").await?;
        Ok(())
    }
}

fn is_retryable_apify_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
}

pub fn chargeable_event_capacity(
    run: &Value,
    event_name: &str,
    max_total_charge_from_env: Option<&str>,
) -> Result<Option<usize>> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some(PAY_PER_EVENT_MODEL)
    {
        return Ok(None);
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let unit_price = event_price(events.get(event_name), event_name)?
        + optional_event_price(events.get(DEFAULT_DATASET_ITEM_EVENT))?;
    if !unit_price.is_finite() {
        bail!("Apify run returned an invalid dataset item price");
    }
    if unit_price == 0.0 {
        return Ok(Some(usize::MAX));
    }

    let Some(max_charge) = max_total_charge_from_env
        .map(parse_max_total_charge)
        .transpose()?
        .flatten()
        .or_else(|| {
            data.pointer("/options/maxTotalChargeUsd")
                .and_then(Value::as_f64)
        })
    else {
        return Ok(Some(usize::MAX));
    };
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned an invalid maximum total charge");
    }
    if max_charge == 0.0 {
        return Ok(Some(usize::MAX));
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0_f64;
    for (charged_event, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {charged_event}"))?;
        if count == 0 {
            continue;
        }
        let price = optional_event_price(events.get(charged_event))?;
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }

    let available_charge = (max_charge - spent).max(0.0);
    let affordable_count = ((available_charge / unit_price) * 10_000.0).round() / 10_000.0;
    let affordable_count = affordable_count.floor();
    Ok(Some(if affordable_count.is_finite() {
        affordable_count as usize
    } else {
        usize::MAX
    }))
}

fn parse_max_total_charge(value: &str) -> Result<Option<f64>> {
    if value.trim().is_empty() || value.trim().eq_ignore_ascii_case("infinity") {
        return Ok(None);
    }
    let charge = value
        .trim()
        .parse::<f64>()
        .context("ACTOR_MAX_TOTAL_CHARGE_USD must be a non-negative number")?;
    if !charge.is_finite() || charge < 0.0 {
        bail!("ACTOR_MAX_TOTAL_CHARGE_USD must be a non-negative number");
    }
    Ok(Some(charge))
}

fn event_price(event: Option<&Value>, event_name: &str) -> Result<f64> {
    let price = event
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the price for event {event_name}"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned an invalid price for event {event_name}");
    }
    Ok(price)
}

fn optional_event_price(event: Option<&Value>) -> Result<f64> {
    let Some(event) = event else {
        return Ok(0.0);
    };
    let price = event
        .get("eventPriceUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run returned an invalid event price"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned an invalid event price");
    }
    Ok(price)
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let status_code = status.as_u16();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.is_empty() {
        format!("HTTP {status_code}")
    } else {
        body
    };
    bail!("Apify API error ({status_code}) while trying to {operation}: {detail}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const EVENT: &str = "jameda-review-result";

    fn pricing_run(max_charge: Value, counts: Value) -> Value {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "apify-actor-start": { "eventPriceUsd": 0.0002 },
                        "jameda-review-result": { "eventPriceUsd": 0.01 },
                        "other-result": { "eventPriceUsd": 0.02 }
                    }}
                },
                "options": { "maxTotalChargeUsd": max_charge },
                "chargedEventCounts": counts
            }
        })
    }

    #[test]
    fn computes_review_capacity_after_all_existing_event_charges() {
        let run = pricing_run(
            json!(0.0722),
            json!({ "apify-actor-start": 1, "other-result": 2 }),
        );
        assert_eq!(
            chargeable_event_capacity(&run, EVENT, None).unwrap(),
            Some(3)
        );
    }

    #[test]
    fn includes_default_dataset_item_price_in_review_capacity() {
        let mut run = pricing_run(json!(0.0502), json!({ "apify-actor-start": 1 }));
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            [DEFAULT_DATASET_ITEM_EVENT] = json!({ "eventPriceUsd": 0.015 });
        assert_eq!(
            chargeable_event_capacity(&run, EVENT, None).unwrap(),
            Some(2)
        );
    }

    #[test]
    fn returns_unlimited_capacity_for_uncapped_or_zero_price_events() {
        let run = pricing_run(Value::Null, json!({ "apify-actor-start": 1 }));
        assert_eq!(
            chargeable_event_capacity(&run, EVENT, None).unwrap(),
            Some(usize::MAX)
        );
        let mut free_event = run.clone();
        free_event["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"][EVENT]
            ["eventPriceUsd"] = json!(0);
        assert_eq!(
            chargeable_event_capacity(&free_event, EVENT, Some("0")).unwrap(),
            Some(usize::MAX)
        );
        let non_ppe = json!({ "data": { "pricingInfo": { "pricingModel": "PRICE_PER_RESULT" } } });
        assert_eq!(
            chargeable_event_capacity(&non_ppe, EVENT, None).unwrap(),
            None
        );
    }

    #[test]
    fn fails_closed_on_missing_or_invalid_pricing_data() {
        let mut missing = pricing_run(json!(1), json!({}));
        missing["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"] = json!({});
        assert!(chargeable_event_capacity(&missing, EVENT, None).is_err());

        let invalid_count = pricing_run(json!(1), json!({ "apify-actor-start": -1 }));
        assert!(chargeable_event_capacity(&invalid_count, EVENT, None).is_err());

        let invalid_limit = pricing_run(json!(1), json!({}));
        assert!(chargeable_event_capacity(&invalid_limit, EVENT, Some("-1")).is_err());
    }

    #[test]
    fn prefers_runtime_limit_and_stops_when_budget_is_exhausted() {
        let run = pricing_run(
            json!(1),
            json!({ "apify-actor-start": 1, "jameda-review-result": 4 }),
        );
        assert_eq!(
            chargeable_event_capacity(&run, EVENT, Some("0.0422")).unwrap(),
            Some(0)
        );
        assert_eq!(
            chargeable_event_capacity(&run, EVENT, Some("0.0722")).unwrap(),
            Some(3)
        );
    }
}
