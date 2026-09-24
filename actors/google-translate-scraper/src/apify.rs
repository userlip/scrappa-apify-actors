use std::{env, time::Duration};

use reqwest::{Client, Method, Response, StatusCode, Url, header};
use serde_json::{Value, json};

use crate::charging::TRANSLATION_RESULT_CHARGE_EVENT;
use crate::{
    charging::{ChargingManager, DEFAULT_DATASET_ITEM_CHARGE_EVENT},
    results::TranslationDatasetItem,
    run_translations::{PushTranslationResult, TranslationOutput},
};

const APIFY_API_DEFAULT: &str = "https://api.apify.com";

pub struct ApifyClient {
    http: Client,
    api_base: Url,
    token: String,
    key_value_store_id: String,
    dataset_id: String,
    actor_run_id: String,
    input_key: String,
}

impl ApifyClient {
    pub fn from_env() -> Result<Self, String> {
        let api_base =
            env::var("APIFY_API_PUBLIC_BASE_URL").unwrap_or_else(|_| APIFY_API_DEFAULT.to_owned());
        let token = required_env("APIFY_TOKEN")?;
        let key_value_store_id = required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?;
        let dataset_id = required_env("ACTOR_DEFAULT_DATASET_ID")?;
        let actor_run_id = required_env("ACTOR_RUN_ID")?;
        let input_key = env::var("ACTOR_INPUT_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "INPUT".to_owned());
        Self::new(
            &api_base,
            token,
            key_value_store_id,
            dataset_id,
            actor_run_id,
            input_key,
        )
    }

    pub fn new(
        api_base: &str,
        token: String,
        key_value_store_id: String,
        dataset_id: String,
        actor_run_id: String,
        input_key: String,
    ) -> Result<Self, String> {
        let api_base = Url::parse(api_base)
            .map_err(|_| "APIFY_API_PUBLIC_BASE_URL must be a valid absolute URL".to_owned())?;
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| format!("Could not create Apify HTTP client: {error}"))?;

        Ok(Self {
            http,
            api_base,
            token,
            key_value_store_id,
            dataset_id,
            actor_run_id,
            input_key,
        })
    }

    pub async fn get_run_pricing(&self) -> Result<Value, String> {
        let url = self.resource_url(&["actor-runs", &self.actor_run_id])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .map_err(|error| format!("Apify run pricing request failed: {error}"))?;
        response_json(response, "Apify run pricing request").await
    }

    pub async fn get_input(&self) -> Result<Option<Value>, String> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .request(Method::GET, url)
            .send()
            .await
            .map_err(|error| format!("Apify INPUT request failed: {error}"))?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = successful_response(response, "fetch Actor input").await?;
        let body = response
            .text()
            .await
            .map_err(|error| format!("Failed to read Actor input response: {error}"))?;
        serde_json::from_str(&body)
            .map(Some)
            .map_err(|_| "Actor input record is not valid JSON".to_owned())
    }

    pub async fn push_dataset_item(&self, item: &TranslationDatasetItem) -> Result<(), String> {
        let url = self.resource_url(&["datasets", &self.dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(item)
            .send()
            .await
            .map_err(|error| format!("Apify dataset write failed: {error}"))?;
        successful_response(response, "store dataset item").await?;
        Ok(())
    }

    pub async fn set_output(&self, output: &Value) -> Result<(), String> {
        let url = self.resource_url(&[
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            "OUTPUT",
        ])?;
        let response = self
            .request(Method::PUT, url)
            .json(output)
            .send()
            .await
            .map_err(|error| {
                format!("Failed to write OUTPUT to the default key-value store: {error}")
            })?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn charge_event(
        &self,
        event_name: &str,
        count: u64,
        idempotency_key: &str,
    ) -> Result<(), String> {
        let url = self.resource_url(&["actor-runs", &self.actor_run_id, "charge"])?;
        let response = self
            .request(Method::POST, url)
            .header("idempotency-key", idempotency_key)
            .json(&json!({"eventName": event_name, "count": count}))
            .send()
            .await
            .map_err(|error| format!("Apify event charge request failed: {error}"))?;
        successful_response(response, "charge Actor event").await?;
        Ok(())
    }

    pub async fn set_terminal_status_message(&self, message: &str) -> Result<(), String> {
        let url = self.resource_url(&["actor-runs", &self.actor_run_id])?;
        let response = self
            .request(Method::PUT, url)
            .timeout(Duration::from_secs(1))
            .json(&json!({
                "statusMessage": message,
                "isStatusMessageTerminal": true
            }))
            .send()
            .await
            .map_err(|error| format!("Apify status message update failed: {error}"))?;
        successful_response(response, "set Actor run status message").await?;
        Ok(())
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.http
            .request(method, url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
    }

    fn resource_url(&self, segments: &[&str]) -> Result<Url, String> {
        let mut url = self.api_base.clone();
        let mut path = url
            .path_segments_mut()
            .map_err(|_| "APIFY_API_PUBLIC_BASE_URL cannot contain path segments".to_owned())?;
        path.pop_if_empty();
        path.extend(std::iter::once("v2").chain(segments.iter().copied()));
        drop(path);
        Ok(url)
    }
}

pub struct ApifyTranslationOutput<'a> {
    client: &'a ApifyClient,
    charging: ChargingManager,
}

impl<'a> ApifyTranslationOutput<'a> {
    pub fn new(client: &'a ApifyClient, charging: ChargingManager) -> Self {
        Self { client, charging }
    }
}

impl TranslationOutput for ApifyTranslationOutput<'_> {
    fn charge_limit_status(&self, processed: usize, requested: usize) -> Option<String> {
        self.charging.charge_limit_status(processed, requested)
    }

    async fn push_translation_result(
        &mut self,
        item: &TranslationDatasetItem,
    ) -> Result<PushTranslationResult, String> {
        if !self.charging.is_pay_per_event() {
            self.client.push_dataset_item(item).await?;
            return Ok(PushTranslationResult {
                saved: true,
                status_message: None,
            });
        }

        let plan = self.charging.plan_dataset_push(item.success);
        if !plan.keep_item {
            if item.success {
                let status_message = format!(
                    "Charge limit reached before saving translation {}; stopping batch without writing uncharged success results.",
                    item.index + 1
                );
                eprintln!(
                    "{status_message} {{\"event\":\"{TRANSLATION_RESULT_CHARGE_EVENT}\",\"charged_count\":0}}"
                );
                return Ok(PushTranslationResult {
                    saved: false,
                    status_message: Some(status_message),
                });
            }
            // The existing TypeScript wrapper ignores pushData's ChargeResult for failure rows.
            return Ok(PushTranslationResult {
                saved: true,
                status_message: None,
            });
        }

        self.client.push_dataset_item(item).await?;
        let mut charged_count = 0;
        for event_name in &plan.events_to_charge {
            self.charging.record_charge(event_name, 1)?;
            charged_count += 1;
            if event_name == DEFAULT_DATASET_ITEM_CHARGE_EVENT {
                continue;
            }
            if !self.charging.has_event_price(event_name) {
                eprintln!("Attempting to charge for an unknown event '{event_name}'");
                continue;
            }

            let idempotency_key = format!(
                "{}-{}-translation-{}",
                self.client.actor_run_id, event_name, item.index
            );
            self.client
                .charge_event(event_name, 1, &idempotency_key)
                .await?;
        }

        let event_charge_limit_reached = self.charging.event_limit_reached(&plan.events_to_charge);
        if item.success && event_charge_limit_reached && charged_count < 1 {
            let status_message = format!(
                "Charge limit reached before saving translation {}; stopping batch without writing uncharged success results.",
                item.index + 1
            );
            eprintln!(
                "{status_message} {{\"event\":\"{TRANSLATION_RESULT_CHARGE_EVENT}\",\"charged_count\":{charged_count}}}"
            );
            return Ok(PushTranslationResult {
                saved: false,
                status_message: Some(status_message),
            });
        }

        Ok(PushTranslationResult {
            saved: true,
            status_message: None,
        })
    }
}

async fn response_json(response: Response, operation: &str) -> Result<Value, String> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("Failed to read {operation} response: {error}"))?;
    if !status.is_success() {
        return Err(api_error(status, &body, operation));
    }
    serde_json::from_str(&body).map_err(|_| format!("{operation} returned invalid JSON"))
}

async fn successful_response(response: Response, operation: &str) -> Result<Response, String> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    Err(api_error(status, &body, operation))
}

fn api_error(status: StatusCode, body: &str, operation: &str) -> String {
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    format!(
        "Apify API error ({}) while trying to {operation}: {reason}{detail}",
        status.as_u16()
    )
}

fn required_env(name: &str) -> Result<String, String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Required environment variable {name} is missing"))
}
