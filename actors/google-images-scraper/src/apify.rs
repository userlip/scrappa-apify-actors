use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::Value;

const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const PAY_PER_EVENT_PRICING_MODEL: &str = "PAY_PER_EVENT";

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub fn new(token: impl Into<String>, base_url: &str) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: Url::parse(base_url).context("APIFY_API_BASE_URL must be a valid URL")?,
            token: token.into(),
        })
    }

    pub async fn get_input(&self, store_id: &str, record_key: &str) -> Result<Option<Value>> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&["key-value-stores", store_id, "records", record_key])?,
            )
            .send()
            .await
            .context("Failed to fetch Actor input from the default key-value store")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(
            response,
            "fetch Actor input from the default key-value store",
        )
        .await?;
        Ok(Some(
            response
                .json()
                .await
                .context("Actor input record is not valid JSON")?,
        ))
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
        successful_response(response, "write OUTPUT to the default key-value store").await?;
        Ok(())
    }

    pub async fn dataset_item_budget(&self, actor_run_id: &str) -> Result<usize> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&["actor-runs", actor_run_id])?,
            )
            .send()
            .await
            .context("Apify run pricing request failed")?;
        let run = successful_response(response, "fetch Actor run pricing")
            .await?
            .json()
            .await
            .context("Actor run pricing response is not valid JSON")?;
        affordable_dataset_items(&run)
    }

    pub async fn push_data(
        &self,
        dataset_id: &str,
        items: &[Value],
        remaining_budget: &mut usize,
    ) -> Result<usize> {
        let count = items.len().min(*remaining_budget);
        if count == 0 {
            return Ok(0);
        }
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["datasets", dataset_id, "items"])?,
            )
            .json(&items[..count])
            .send()
            .await
            .context("Failed to store image results in the default dataset")?;
        successful_response(response, "store image results in the default dataset").await?;
        *remaining_budget -= count;
        Ok(count)
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_BASE_URL cannot be a base URL"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(resource.iter().copied()));
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }
}

fn affordable_dataset_items(run: &Value) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    let pricing_model = data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if pricing_model != PAY_PER_EVENT_PRICING_MODEL {
        return Ok(usize::MAX);
    }

    let Some(max_charge) = data
        .pointer("/options/maxTotalChargeUsd")
        .filter(|value| !value.is_null())
    else {
        return Ok(usize::MAX);
    };
    let max_charge = max_charge
        .as_f64()
        .ok_or_else(|| anyhow!("Apify run returned invalid charging values"))?;
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = events
        .get(DEFAULT_DATASET_ITEM_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    if !item_price.is_finite() || item_price < 0.0 {
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
            .ok_or_else(|| anyhow!("Invalid charged event count"))?;
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
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }

    if item_price == 0.0 {
        return Ok(usize::MAX);
    }
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let available_charge = (max_charge - spent + tolerance).max(0.0);
    let affordable_items = (available_charge / item_price).floor();
    Ok(if affordable_items.is_finite() {
        affordable_items.min(usize::MAX as f64) as usize
    } else {
        usize::MAX
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockResponse, MockServer};
    use serde_json::{json, Value};

    const RUN_INFO: &str = r#"{
        "data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "apify-default-dataset-item": {"eventPriceUsd": 0.1},
                    "apify-actor-start": {"eventPriceUsd": 0.01}
                }}
            },
            "options": {"maxTotalChargeUsd": 0.21},
            "chargedEventCounts": {"apify-actor-start": 1}
        }
    }"#;

    #[tokio::test]
    async fn reads_input_writes_output_and_caps_dataset_items_to_event_budget() {
        let server = MockServer::start(vec![
            MockResponse {
                status: 200,
                body: r#"{"queries":["coffee"]}"#.into(),
            },
            MockResponse {
                status: 200,
                body: RUN_INFO.into(),
            },
            MockResponse {
                status: 200,
                body: String::new(),
            },
            MockResponse {
                status: 200,
                body: String::new(),
            },
        ]);
        let client = ApifyClient::new("test-token", &server.base_url).unwrap();

        let input = client.get_input("store", "INPUT").await.unwrap();
        assert_eq!(input, Some(json!({"queries":["coffee"]})));
        let mut budget = client.dataset_item_budget("run").await.unwrap();
        assert_eq!(budget, 2);
        let saved = client
            .push_data(
                "dataset",
                &[json!({"id":1}), json!({"id":2}), json!({"id":3})],
                &mut budget,
            )
            .await
            .unwrap();
        assert_eq!(saved, 2);
        assert_eq!(budget, 0);
        client
            .set_output("store", &json!({"image_results":2}))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert!(requests
            .iter()
            .all(|request| has_test_bearer_token(request)));
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/store/records/INPUT"
        );
        assert_eq!(request_parts(&requests[1]).1, "/v2/actor-runs/run");
        let (method, path, body) = request_parts(&requests[2]);
        assert_eq!((method, path), ("POST", "/v2/datasets/dataset/items"));
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!([{"id":1},{"id":2}])
        );
        let (method, path, body) = request_parts(&requests[3]);
        assert_eq!(
            (method, path),
            ("PUT", "/v2/key-value-stores/store/records/OUTPUT")
        );
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!({"image_results":2})
        );
    }

    #[tokio::test]
    async fn skips_dataset_write_when_the_pay_per_event_budget_is_empty() {
        let run_info = RUN_INFO.replace("0.21", "0.01");
        let server = MockServer::start(vec![MockResponse {
            status: 200,
            body: run_info,
        }]);
        let client = ApifyClient::new("test-token", &server.base_url).unwrap();
        let mut budget = client.dataset_item_budget("run").await.unwrap();
        assert_eq!(budget, 0);
        assert_eq!(
            client
                .push_data("dataset", &[json!({"id":1})], &mut budget)
                .await
                .unwrap(),
            0
        );
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn non_pay_per_event_runs_have_no_dataset_item_budget_limit() {
        for pricing_model in ["FREE", "FLAT_PRICE_PER_MONTH", "PRICE_PER_DATASET_ITEM"] {
            let run = json!({"data":{"pricingInfo":{"pricingModel":pricing_model}}});
            assert_eq!(affordable_dataset_items(&run).unwrap(), usize::MAX);
        }
    }

    #[test]
    fn pay_per_event_dataset_budget_handles_absent_null_zero_and_positive_caps() {
        let mut run: Value = serde_json::from_str(RUN_INFO).unwrap();

        run.pointer_mut("/data/options")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("maxTotalChargeUsd");
        assert_eq!(affordable_dataset_items(&run).unwrap(), usize::MAX);

        run["data"]["options"]["maxTotalChargeUsd"] = Value::Null;
        assert_eq!(affordable_dataset_items(&run).unwrap(), usize::MAX);

        run["data"]["options"]["maxTotalChargeUsd"] = json!(0.0);
        assert_eq!(affordable_dataset_items(&run).unwrap(), 0);

        run["data"]["options"]["maxTotalChargeUsd"] = json!(0.21);
        assert_eq!(affordable_dataset_items(&run).unwrap(), 2);
    }

    #[tokio::test]
    async fn writes_unbounded_non_pay_per_event_rows_without_a_custom_charge_request() {
        let server = MockServer::start(vec![
            MockResponse {
                status: 200,
                body: r#"{"data":{"pricingInfo":{"pricingModel":"FREE"}}}"#.into(),
            },
            MockResponse {
                status: 201,
                body: String::new(),
            },
        ]);
        let client = ApifyClient::new("test-token", &server.base_url).unwrap();
        let mut budget = client.dataset_item_budget("run").await.unwrap();
        assert_eq!(budget, usize::MAX);

        let items = [json!({"id":1}), json!({"id":2}), json!({"id":3})];
        assert_eq!(
            client
                .push_data("dataset", &items, &mut budget)
                .await
                .unwrap(),
            items.len()
        );

        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        let (method, path, body) = request_parts(&requests[1]);
        assert_eq!((method, path), ("POST", "/v2/datasets/dataset/items"));
        assert_eq!(serde_json::from_str::<Value>(body).unwrap(), json!(items));
    }

    fn request_parts(request: &str) -> (&str, &str, &str) {
        let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
        let mut parts = headers
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace();
        (
            parts.next().unwrap_or_default(),
            parts.next().unwrap_or_default(),
            body,
        )
    }

    fn has_test_bearer_token(request: &str) -> bool {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .any(|line| line.eq_ignore_ascii_case("authorization: Bearer test-token"))
    }
}
