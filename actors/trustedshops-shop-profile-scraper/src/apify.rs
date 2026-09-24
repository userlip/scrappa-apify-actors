use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::charging::{
    ChargingManager, DEFAULT_DATASET_ITEM_CHARGE_EVENT, SHOP_PROFILE_RESULT_CHARGE_EVENT,
};

static CHARGE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const MAX_CHARGE_ATTEMPTS: u32 = 3;

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushDataResult {
    pub saved_count: usize,
    pub status_message: Option<String>,
}

impl ApifyClient {
    pub fn new(base_url: &str, token: String) -> Result<Self> {
        let base_url =
            Url::parse(base_url).context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?;
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url,
            token,
        })
    }

    pub async fn get_input(&self, store_id: &str, input_key: &str) -> Result<Option<Value>> {
        let response = self
            .request(
                Method::GET,
                self.resource_url(&["key-value-stores", store_id, "records", input_key])?,
            )
            .send()
            .await
            .context("Failed to fetch Actor input from the default key-value store")?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }

        Ok(Some(
            successful_response(response, "fetch Actor input")
                .await?
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
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn get_run(&self, run_id: &str) -> Result<Value> {
        let response = self
            .request(Method::GET, self.resource_url(&["actor-runs", run_id])?)
            .send()
            .await
            .context("Apify run pricing request failed")?;
        successful_response(response, "fetch Actor run pricing")
            .await?
            .json()
            .await
            .context("Actor run pricing response is not valid JSON")
    }

    pub async fn push_charged_item(
        &self,
        run_id: &str,
        dataset_id: &str,
        item: &Value,
        charging: &mut ChargingManager,
    ) -> Result<PushDataResult> {
        if !charging.is_pay_per_event() {
            self.push_dataset_item(dataset_id, item).await?;
            return Ok(PushDataResult {
                saved_count: 1,
                status_message: None,
            });
        }

        if charging.push_limit(1) == 0 {
            let status_message = charge_limit_message(0, 1);
            return Ok(PushDataResult {
                saved_count: 0,
                status_message: Some(status_message),
            });
        }

        self.push_dataset_item(dataset_id, item).await?;

        let custom_charge = charging.charge_event(SHOP_PROFILE_RESULT_CHARGE_EVENT);
        let dataset_charge = charging.charge_event(DEFAULT_DATASET_ITEM_CHARGE_EVENT);
        let charge_error = if custom_charge.charged_count > 0
            && charging.is_priced_event(SHOP_PROFILE_RESULT_CHARGE_EVENT)
        {
            self.charge_event(
                run_id,
                SHOP_PROFILE_RESULT_CHARGE_EVENT,
                custom_charge.charged_count,
            )
            .await
            .err()
        } else {
            None
        };

        let charged_count = custom_charge.charged_count + dataset_charge.charged_count;
        let limit_reached =
            custom_charge.event_charge_limit_reached || dataset_charge.event_charge_limit_reached;
        let saved_count = if charge_error.is_some() {
            1
        } else if limit_reached {
            charged_count.min(1)
        } else {
            1
        };
        let status_message = if let Some(error) = charge_error {
            Some(format!(
                "TrustedShops shop profile was saved, but its Apify charge could not be confirmed: {error}"
            ))
        } else {
            limit_reached.then(|| charge_limit_message(saved_count, 1))
        };

        Ok(PushDataResult {
            saved_count,
            status_message,
        })
    }

    pub async fn set_status_message(&self, run_id: &str, message: &str) {
        eprintln!("[Status message]: {message}");
        let body = json!({
            "runId": run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true,
        });
        let response = self
            .request(
                Method::PUT,
                self.resource_url(&["actor-runs", run_id])
                    .expect("valid run path"),
            )
            .json(&body)
            .send()
            .await;
        match response {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => eprintln!(
                "Could not set Actor run status message: {}",
                response.status()
            ),
            Err(error) => eprintln!("Could not set Actor run status message: {error}"),
        }
    }

    async fn push_dataset_item(&self, dataset_id: &str, item: &Value) -> Result<()> {
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["datasets", dataset_id, "items"])?,
            )
            .json(&[item])
            .send()
            .await
            .context("Failed to store shop profile in the default dataset")?;
        successful_response(response, "store shop profile in the default dataset").await?;
        Ok(())
    }

    async fn charge_event(&self, run_id: &str, event_name: &str, count: usize) -> Result<()> {
        let idempotency_key = format!(
            "{run_id}-{event_name}-{}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            CHARGE_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        );
        let url = self.resource_url(&["actor-runs", run_id, "charge"])?;
        let body = json!({"eventName": event_name, "count": count});

        for attempt in 1..=MAX_CHARGE_ATTEMPTS {
            let response = self
                .request(Method::POST, url.clone())
                .header("idempotency-key", &idempotency_key)
                .json(&body)
                .send()
                .await;

            match response {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response)
                    if attempt < MAX_CHARGE_ATTEMPTS
                        && retryable_charge_status(response.status()) =>
                {
                    eprintln!(
                        "Apify charge request returned HTTP {}; retrying attempt {}/{} with the same idempotency key.",
                        response.status().as_u16(),
                        attempt + 1,
                        MAX_CHARGE_ATTEMPTS,
                    );
                }
                Ok(response) => {
                    successful_response(response, "charge for a shop profile result").await?;
                    return Ok(());
                }
                Err(error)
                    if attempt < MAX_CHARGE_ATTEMPTS && retryable_charge_transport(&error) =>
                {
                    eprintln!(
                        "Apify charge request failed ({error}); retrying attempt {}/{} with the same idempotency key.",
                        attempt + 1,
                        MAX_CHARGE_ATTEMPTS,
                    );
                }
                Err(error) => {
                    return Err(error).context("Apify event charge request failed");
                }
            }

            tokio::time::sleep(charge_retry_delay(attempt)).await;
        }

        bail!("Apify event charge request exhausted its retry attempts")
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    fn resource_url(&self, resource: &[&str]) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("APIFY_API_PUBLIC_BASE_URL cannot be a base URL"))?
            .pop_if_empty()
            .extend(["v2"].into_iter().chain(resource.iter().copied()));
        Ok(url)
    }
}

fn charge_limit_message(saved_count: usize, requested_count: usize) -> String {
    format!(
        "Charge limit reached after saving {saved_count} of {requested_count} TrustedShops shop profile results."
    )
}

fn retryable_charge_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn retryable_charge_transport(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect() || error.is_request()
}

fn charge_retry_delay(failed_attempt: u32) -> Duration {
    Duration::from_millis(u64::from(failed_attempt) * 250)
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
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
    use crate::{
        charging::ChargingManager,
        test_support::{MockResponse, MockServer, request_parts},
    };
    use serde_json::json;

    fn client(server: &MockServer) -> ApifyClient {
        ApifyClient::new(&server.base_url, "test-token".to_owned()).unwrap()
    }

    fn charging(max_total: f64, charged_event_counts: Value) -> ChargingManager {
        ChargingManager::from_environment(
            &json!({
                "pricingModel":"PAY_PER_EVENT",
                "pricingPerEvent":{"actorChargeEvents":{
                    "shop-profile-result":{"eventPriceUsd":0.10},
                    "apify-default-dataset-item":{"eventPriceUsd":0.05}
                }}
            }),
            &charged_event_counts,
            Some(max_total),
        )
    }

    #[tokio::test]
    async fn reads_input_writes_dataset_and_summary_with_apify_bearer_auth() {
        let server = MockServer::start(vec![
            MockResponse::json(200, json!({"tsid":"example"})),
            MockResponse::json(200, json!({})),
            MockResponse::json(200, json!({})),
        ]);
        let apify = client(&server);

        assert_eq!(
            apify.get_input("test-store", "INPUT").await.unwrap(),
            Some(json!({"tsid":"example"}))
        );
        apify
            .push_charged_item(
                "test-run",
                "test-dataset",
                &json!({"tsid":"example"}),
                &mut ChargingManager::free(),
            )
            .await
            .unwrap();
        apify
            .set_output("test-store", &json!({"profiles_saved":1}))
            .await
            .unwrap();

        let requests = server.requests();
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/key-value-stores/test-store/records/INPUT"
        );
        assert_eq!(
            request_parts(&requests[1]).1,
            "/v2/datasets/test-dataset/items"
        );
        assert_eq!(
            request_parts(&requests[2]).1,
            "/v2/key-value-stores/test-store/records/OUTPUT"
        );
        assert!(requests.iter().all(|request| {
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-token")
        }));
        assert_eq!(
            serde_json::from_str::<Value>(request_parts(&requests[1]).2).unwrap(),
            json!([{"tsid":"example"}])
        );
    }

    #[tokio::test]
    async fn charges_one_result_after_dataset_write_and_signals_budget_limit() {
        let server = MockServer::start(vec![
            MockResponse::json(200, json!({})),
            MockResponse::json(201, json!({})),
        ]);
        let apify = client(&server);
        let mut charging = charging(0.15, json!({}));

        let result = apify
            .push_charged_item(
                "test-run",
                "test-dataset",
                &json!({"tsid":"example"}),
                &mut charging,
            )
            .await
            .unwrap();

        assert_eq!(result.saved_count, 1);
        assert_eq!(
            result.status_message.as_deref(),
            Some("Charge limit reached after saving 1 of 1 TrustedShops shop profile results.")
        );
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/datasets/test-dataset/items"
        );
        let (method, path, body) = request_parts(&requests[1]);
        assert_eq!((method, path), ("POST", "/v2/actor-runs/test-run/charge"));
        assert!(
            requests[1]
                .to_ascii_lowercase()
                .contains("idempotency-key:")
        );
        assert_eq!(
            serde_json::from_str::<Value>(body).unwrap(),
            json!({"eventName":"shop-profile-result","count":1})
        );
    }

    #[tokio::test]
    async fn retries_transient_charge_failures_with_the_same_idempotency_key() {
        let server = MockServer::start(vec![
            MockResponse::json(201, json!({})),
            MockResponse::json(503, json!({"error":"temporary failure"})),
            MockResponse::json(200, json!({})),
        ]);
        let apify = client(&server);
        let mut charging = charging(0.15, json!({}));

        let result = apify
            .push_charged_item(
                "test-run",
                "test-dataset",
                &json!({"tsid":"example"}),
                &mut charging,
            )
            .await
            .unwrap();

        assert_eq!(result.saved_count, 1);
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/datasets/test-dataset/items"
        );
        assert_eq!(
            request_parts(&requests[1]).1,
            "/v2/actor-runs/test-run/charge"
        );
        assert_eq!(
            request_parts(&requests[2]).1,
            "/v2/actor-runs/test-run/charge"
        );
        assert_eq!(
            request_header(&requests[1], "idempotency-key"),
            request_header(&requests[2], "idempotency-key")
        );
        assert_eq!(request_parts(&requests[1]).2, request_parts(&requests[2]).2);
    }

    #[tokio::test]
    async fn persistent_charge_failures_keep_the_dataset_result_counted_as_saved() {
        let server = MockServer::start(vec![
            MockResponse::json(201, json!({})),
            MockResponse::json(503, json!({"error":"temporary failure"})),
            MockResponse::json(503, json!({"error":"temporary failure"})),
            MockResponse::json(503, json!({"error":"temporary failure"})),
        ]);
        let apify = client(&server);
        let mut charging = charging(0.15, json!({}));

        let result = apify
            .push_charged_item(
                "test-run",
                "test-dataset",
                &json!({"tsid":"example"}),
                &mut charging,
            )
            .await
            .unwrap();

        assert_eq!(result.saved_count, 1);
        assert!(result.status_message.as_deref().is_some_and(|message| {
            message.contains("was saved") && message.contains("charge could not be confirmed")
        }));
        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        assert_eq!(
            request_parts(&requests[0]).1,
            "/v2/datasets/test-dataset/items"
        );
        let keys = requests[1..]
            .iter()
            .map(|request| request_header(request, "idempotency-key"))
            .collect::<Vec<_>>();
        assert!(keys.iter().all(|key| key == &keys[0]));
    }

    #[tokio::test]
    async fn exhausted_budget_does_not_write_or_charge_another_result() {
        let server = MockServer::start(Vec::new());
        let apify = client(&server);
        let mut charging = charging(
            0.15,
            json!({"shop-profile-result":2,"apify-default-dataset-item":1}),
        );

        let result = apify
            .push_charged_item(
                "test-run",
                "test-dataset",
                &json!({"tsid":"example"}),
                &mut charging,
            )
            .await
            .unwrap();

        assert_eq!(result.saved_count, 0);
        assert_eq!(
            result.status_message.as_deref(),
            Some("Charge limit reached after saving 0 of 1 TrustedShops shop profile results.")
        );
        assert!(server.requests().is_empty());
    }

    fn request_header(request: &str, name: &str) -> String {
        request
            .lines()
            .find_map(|line| {
                let (header, value) = line.split_once(':')?;
                header
                    .eq_ignore_ascii_case(name)
                    .then(|| value.trim().to_owned())
            })
            .expect("request header is present")
    }
}
