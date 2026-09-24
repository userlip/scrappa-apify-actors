use anyhow::{anyhow, bail, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{Map, Value};
use std::time::Duration;
use url::Url;

const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const APIFY_MAX_RETRIES: usize = 2;

pub struct ApifyConfig {
    pub api_base: String,
    pub token: String,
    pub run_id: String,
    pub store_id: String,
    pub dataset_id: String,
    pub input_key: String,
}

pub struct ApifyClient {
    http: Client,
    base_url: Url,
    token: String,
    run_id: String,
    store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    pub fn new(config: ApifyConfig) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(&config.api_base)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token: config.token,
            run_id: config.run_id,
            store_id: config.store_id,
            dataset_id: config.dataset_id,
            input_key: config.input_key,
        })
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .request_with_retry("GET", url, None, "fetch Actor input")
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

    pub async fn dataset_item_capacity(&self, requested: usize) -> Result<usize> {
        let url = self.resource_url(&["actor-runs", &self.run_id])?;
        let response = self
            .request_with_retry("GET", url, None, "fetch Actor run pricing")
            .await?;
        let response = successful_response(response, "fetch Actor run pricing").await?;
        let run = response
            .json::<Value>()
            .await
            .context("Actor run pricing response is not valid JSON")?;
        affordable_dataset_items(&run, requested)
    }

    pub async fn push_dataset_items(&self, items: &[Value], capacity: usize) -> Result<usize> {
        let items = &items[..items.len().min(capacity)];
        if items.is_empty() {
            return Ok(0);
        }
        let url = self.resource_url(&["datasets", &self.dataset_id, "items"])?;
        let response = self
            .request(
                "POST",
                url,
                Some(Value::Array(items.to_vec())),
                "store dataset items",
            )
            .await?;
        successful_response(response, "store dataset items").await?;
        Ok(items.len())
    }

    pub async fn set_output(&self, output: &Value) -> Result<()> {
        let url = self.resource_url(&["key-value-stores", &self.store_id, "records", "OUTPUT"])?;
        let response = self
            .request_with_retry("PUT", url, Some(output.clone()), "write OUTPUT")
            .await?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
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

    async fn request(
        &self,
        method: &'static str,
        url: Url,
        body: Option<Value>,
        operation: &str,
    ) -> Result<Response> {
        let mut request = match method {
            "GET" => self.http.get(url),
            "POST" => self.http.post(url),
            "PUT" => self.http.put(url),
            _ => bail!("Unsupported Apify request method {method}"),
        }
        .bearer_auth(&self.token)
        .header(header::ACCEPT, "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        request
            .send()
            .await
            .with_context(|| format!("Failed to {operation} through Apify API"))
    }

    async fn request_with_retry(
        &self,
        method: &'static str,
        url: Url,
        body: Option<Value>,
        operation: &str,
    ) -> Result<Response> {
        let mut retry_count = 0;
        loop {
            let response = self
                .request(method, url.clone(), body.clone(), operation)
                .await?;
            if let Some(delay) = apify_retry_delay(method, response.status(), retry_count) {
                drop(response);
                tokio::time::sleep(delay).await;
                retry_count += 1;
                continue;
            }
            return Ok(response);
        }
    }
}

fn affordable_dataset_items(run: &Value, requested: usize) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    match data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
    {
        Some("PAY_PER_EVENT") => {}
        Some(_) => return Ok(requested),
        None => bail!("Apify run is not configured for pay-per-event pricing"),
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = event_price(events, "apify-default-dataset-item")?;
    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        None | Some(Value::Null) => return Ok(requested),
        Some(value) => value
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?,
    };
    if !item_price.is_finite() || item_price < 0.0 || !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    for (event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if count == 0 {
            continue;
        }
        let price = event_price(events, event_name)?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let affordable = ((max_charge - spent + tolerance) / item_price)
        .floor()
        .max(0.0);
    Ok(affordable.min(requested as f64) as usize)
}

fn event_price(events: &Map<String, Value>, event_name: &str) -> Result<f64> {
    events
        .get(event_name)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))
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

fn apify_retry_delay(method: &str, status: StatusCode, retry_count: usize) -> Option<Duration> {
    if !matches!(method, "GET" | "PUT")
        || retry_count >= APIFY_MAX_RETRIES
        || (status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error())
    {
        return None;
    }
    Some(Duration::from_secs((retry_count + 1) as u64))
}

#[cfg(test)]
mod tests {
    use super::affordable_dataset_items;
    use serde_json::{json, Value};

    fn run(max_charge: Option<Value>, charged_counts: Value) -> Value {
        let mut run = json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {"actorChargeEvents": {
                        "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                        "apify-actor-start": {"eventPriceUsd": 0.0001}
                    }}
                },
                "chargedEventCounts": charged_counts,
                "options": {}
            }
        });
        if let Some(max_charge) = max_charge {
            run["data"]["options"]["maxTotalChargeUsd"] = max_charge;
        }
        run
    }

    #[test]
    fn ppe_capacity_counts_spend_from_all_events() {
        let run = run(
            Some(json!(0.001)),
            json!({"apify-default-dataset-item": 1, "apify-actor-start": 1}),
        );
        assert_eq!(affordable_dataset_items(&run, 10).unwrap(), 2);
    }

    #[test]
    fn ppe_capacity_respects_requested_count_and_exhausted_positive_limit() {
        assert_eq!(
            affordable_dataset_items(&run(Some(json!(1.0)), json!({})), 2).unwrap(),
            2
        );
        assert_eq!(
            affordable_dataset_items(
                &run(Some(json!(0.0001)), json!({"apify-actor-start": 1})),
                2
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn ppe_capacity_allows_free_default_items() {
        let mut run = run(Some(json!(0.0001)), json!({}));
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            ["apify-default-dataset-item"]["eventPriceUsd"] = json!(0.0);
        assert_eq!(affordable_dataset_items(&run, 7).unwrap(), 7);
    }

    #[test]
    fn ppe_capacity_is_unbounded_when_the_spending_limit_is_missing() {
        let run = run(None, json!({"apify-actor-start": 1}));
        assert_eq!(affordable_dataset_items(&run, 7).unwrap(), 7);
    }

    #[test]
    fn ppe_capacity_is_unbounded_when_the_spending_limit_is_null() {
        let run = run(Some(Value::Null), json!({"apify-actor-start": 1}));
        assert_eq!(affordable_dataset_items(&run, 7).unwrap(), 7);
    }

    #[test]
    fn ppe_capacity_is_zero_when_the_spending_limit_is_zero() {
        let run = run(Some(json!(0)), json!({"apify-actor-start": 1}));
        assert_eq!(affordable_dataset_items(&run, 7).unwrap(), 0);
    }

    #[test]
    fn missing_apify_pricing_fails() {
        assert!(affordable_dataset_items(&json!({}), 1).is_err());
        assert!(affordable_dataset_items(&json!({"data": {"pricingInfo": {}}}), 1).is_err());
    }

    #[test]
    fn non_ppe_runs_keep_all_requested_dataset_items() {
        for pricing_model in ["PRICE_PER_DATASET_ITEM", "FLAT_PRICE_PER_MONTH", "FREE"] {
            let run = json!({"data": {"pricingInfo": {"pricingModel": pricing_model}}});
            assert_eq!(affordable_dataset_items(&run, 7).unwrap(), 7);
        }
    }
}
