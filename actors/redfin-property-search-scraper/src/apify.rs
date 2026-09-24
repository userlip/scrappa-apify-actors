use std::{env, time::Duration};

use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde_json::Value;
use url::Url;

pub(crate) const DEFAULT_API_BASE_URL: &str = "https://api.apify.com";
const MAX_RETRIES: usize = 2;

#[derive(Debug, Clone)]
pub(crate) struct ActorConfig {
    pub(crate) apify_api_base_url: String,
    pub(crate) apify_token: String,
    pub(crate) actor_run_id: String,
    pub(crate) key_value_store_id: String,
    pub(crate) dataset_id: String,
    pub(crate) input_key: String,
    pub(crate) scrappa_api_base_url: String,
    pub(crate) scrappa_api_key: String,
}

impl ActorConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let apify_api_base_url = env::var("APIFY_API_PUBLIC_BASE_URL")
            .or_else(|_| env::var("APIFY_API_BASE_URL"))
            .unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_owned());

        Ok(Self {
            apify_api_base_url,
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            scrappa_api_base_url: env_or_default(
                "SCRAPPA_API_BASE_URL",
                crate::scrappa::DEFAULT_API_BASE_URL,
            ),
            scrappa_api_key: env::var("SCRAPPA_API_KEY").unwrap_or_default(),
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

pub(crate) struct ApifyClient {
    http: Client,
    base_url: String,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
}

impl ApifyClient {
    pub(crate) fn new(http: Client, config: &ActorConfig) -> Self {
        Self {
            http,
            base_url: config.apify_api_base_url.clone(),
            token: config.apify_token.clone(),
            actor_run_id: config.actor_run_id.clone(),
            key_value_store_id: config.key_value_store_id.clone(),
            dataset_id: config.dataset_id.clone(),
            input_key: config.input_key.clone(),
        }
    }

    pub(crate) async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "v2",
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self.get_with_retry(url, "actor input retrieval").await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = require_apify_success(response, "actor input retrieval").await?;
        response
            .json::<Value>()
            .await
            .context("Apify actor input record was not valid JSON")
            .map(Some)
    }

    pub(crate) async fn get_run(&self) -> Result<Value> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let response = self.get_with_retry(url, "run pricing request").await?;
        require_apify_success(response, "run pricing request")
            .await?
            .json::<Value>()
            .await
            .context("Apify run pricing response was not valid JSON")
    }

    pub(crate) async fn push_dataset_items(&self, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(items)
            .send()
            .await
            .context("Failed to publish dataset items to Apify API")?;
        require_apify_success(response, "dataset item publication").await?;
        Ok(())
    }

    pub(crate) async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["v2", "datasets", &self.dataset_id, "items"])?;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .json(item)
            .send()
            .await
            .context("Failed to publish dataset item to Apify API")?;
        require_apify_success(response, "dataset item publication").await?;
        Ok(())
    }

    pub(crate) async fn charge_event(&self, event_name: &str, idempotency_key: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id, "charge"])?;
        let body = serde_json::json!({"eventName": event_name, "count": 1});
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .post(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .header("Idempotency-Key", idempotency_key)
                .json(&body)
                .send()
                .await;
            match response {
                Ok(response) => {
                    if apify_retry_delay("POST", response.status(), retry_count).is_some() {
                        drop(response);
                        tokio::time::sleep(Duration::from_secs((retry_count + 1) as u64)).await;
                        retry_count += 1;
                        continue;
                    }
                    require_apify_success(response, "event charge").await?;
                    return Ok(());
                }
                Err(error) if retry_count < MAX_RETRIES => {
                    eprintln!("Apify event charge request failed ({error}). Retrying.");
                    tokio::time::sleep(Duration::from_secs((retry_count + 1) as u64)).await;
                    retry_count += 1;
                }
                Err(error) => return Err(error).context("Apify event charge request failed"),
            }
        }
    }

    pub(crate) async fn set_status_message(&self, message: &str) -> Result<()> {
        let url = self.endpoint(&["v2", "actor-runs", &self.actor_run_id])?;
        let body = serde_json::json!({
            "runId": self.actor_run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true,
        });
        let response = self
            .http
            .put(url)
            .bearer_auth(&self.token)
            .header(header::ACCEPT, "application/json")
            .timeout(Duration::from_secs(1))
            .json(&body)
            .send()
            .await
            .context("Apify terminal status message request failed")?;
        require_apify_success(response, "terminal status message update").await?;
        Ok(())
    }

    async fn get_with_retry(&self, url: Url, operation: &str) -> Result<Response> {
        let mut retry_count = 0;
        loop {
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(header::ACCEPT, "application/json")
                .send()
                .await;
            let response = match response {
                Ok(response) => response,
                Err(error) if retry_count < MAX_RETRIES => {
                    eprintln!("Apify {operation} request failed ({error}). Retrying.");
                    tokio::time::sleep(Duration::from_secs((retry_count + 1) as u64)).await;
                    retry_count += 1;
                    continue;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("Apify {operation} request failed"))
                }
            };
            if apify_retry_delay("GET", response.status(), retry_count).is_some() {
                drop(response);
                tokio::time::sleep(Duration::from_secs((retry_count + 1) as u64)).await;
                retry_count += 1;
                continue;
            }
            return Ok(response);
        }
    }

    fn endpoint(&self, path: &[&str]) -> Result<Url> {
        endpoint_url(&self.base_url, path)
    }
}

async fn require_apify_success(response: Response, operation: &str) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(anyhow!(
        "Apify {operation} failed ({}): {body}",
        status.as_u16()
    ))
}

fn apify_retry_delay(method: &str, status: StatusCode, retry_count: usize) -> Option<Duration> {
    if !matches!(method, "GET" | "POST")
        || retry_count >= MAX_RETRIES
        || (status != StatusCode::TOO_MANY_REQUESTS && !status.is_server_error())
    {
        return None;
    }
    Some(Duration::from_secs((retry_count + 1) as u64))
}

fn endpoint_url(base_url: &str, path: &[&str]) -> Result<Url> {
    let mut url = Url::parse(&format!("{}/", base_url.trim_end_matches('/')))
        .with_context(|| format!("Invalid Apify API base URL: {base_url}"))?;
    let mut segments = url.path_segments_mut().map_err(|_| {
        anyhow!("Apify API base URL cannot contain a query or fragment: {base_url}")
    })?;
    segments.pop_if_empty();
    segments.extend(path.iter().copied());
    drop(segments);
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    async fn read_request(stream: &mut tokio::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut chunk = [0; 2048];
        loop {
            let read = stream.read(&mut chunk).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            let Some(body_start) = request
                .windows(4)
                .position(|part| part == b"\r\n\r\n")
                .map(|position| position + 4)
            else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..body_start]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap_or_default())
                })
                .unwrap_or_default();
            if request.len() >= body_start + content_length {
                break;
            }
        }
        request
    }

    async fn server(
        responses: Vec<(u16, String)>,
    ) -> (String, tokio::task::JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                requests.push(read_request(&mut stream).await);
                let reason = StatusCode::from_u16(status)
                    .unwrap()
                    .canonical_reason()
                    .unwrap_or("Unknown");
                let response = format!("HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        (format!("http://{address}/prefix"), handle)
    }

    fn client(base_url: String) -> ApifyClient {
        let config = ActorConfig {
            apify_api_base_url: base_url,
            apify_token: "test-token".to_owned(),
            actor_run_id: "run-1".to_owned(),
            key_value_store_id: "store-1".to_owned(),
            dataset_id: "dataset-1".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_base_url: "https://scrappa.co/api".to_owned(),
            scrappa_api_key: "test-key".to_owned(),
        };
        ApifyClient::new(Client::new(), &config)
    }

    #[tokio::test]
    async fn reads_input_from_default_key_value_store_with_bearer_auth() {
        let (base_url, server) = server(vec![(200, r#"{"region_id":16163}"#.to_owned())]).await;
        let input = client(base_url).get_input().await.unwrap().unwrap();
        assert_eq!(input["region_id"], 16163);
        let request = String::from_utf8(server.await.unwrap().remove(0)).unwrap();
        assert!(
            request.starts_with("GET /prefix/v2/key-value-stores/store-1/records/INPUT HTTP/1.1")
        );
        assert!(request
            .to_lowercase()
            .contains("authorization: bearer test-token"));
    }

    #[tokio::test]
    async fn publishes_dataset_rows_and_charges_with_idempotency_key() {
        let (base_url, server) = server(vec![(201, String::new()), (201, "{}".to_owned())]).await;
        let client = client(base_url);
        client
            .push_dataset_items(&[json!({"id":1}), json!({"id":2})])
            .await
            .unwrap();
        client
            .charge_event("property-result", "run-1-property-result-0-0")
            .await
            .unwrap();
        let requests = server.await.unwrap();
        let dataset_request = String::from_utf8(requests[0].clone()).unwrap();
        assert!(dataset_request.starts_with("POST /prefix/v2/datasets/dataset-1/items HTTP/1.1"));
        assert!(dataset_request.contains(r#"[{"id":1},{"id":2}]"#));
        let charge_request = String::from_utf8(requests[1].clone()).unwrap();
        assert!(charge_request.starts_with("POST /prefix/v2/actor-runs/run-1/charge HTTP/1.1"));
        assert!(charge_request
            .to_lowercase()
            .contains("idempotency-key: run-1-property-result-0-0"));
        assert!(charge_request.contains(r#""eventName":"property-result""#));
        assert!(charge_request.contains(r#""count":1"#));
    }

    #[tokio::test]
    async fn writes_actor_failure_as_a_terminal_status_message() {
        let (base_url, server) = server(vec![(200, "{}".to_owned())]).await;
        client(base_url)
            .set_status_message("Scrappa API error (422): Invalid request")
            .await
            .unwrap();
        let request = String::from_utf8(server.await.unwrap().remove(0)).unwrap();
        assert!(request.starts_with("PUT /prefix/v2/actor-runs/run-1 HTTP/1.1"));
        assert!(request.contains(r#""isStatusMessageTerminal":true"#));
        assert!(request.contains(r#""statusMessage":"Scrappa API error (422): Invalid request""#));
    }

    #[tokio::test]
    async fn retries_transient_run_metadata_response_and_returns_missing_input() {
        let (base_url, server) = server(vec![
            (503, "{}".to_owned()),
            (200, r#"{"data":{"pricingInfo":{"pricingModel":"FREE"},"chargedEventCounts":{},"options":{}}}"#.to_owned()),
            (404, "{}".to_owned()),
        ]).await;
        let client = client(base_url);
        let run = client.get_run().await.unwrap();
        assert_eq!(
            run.pointer("/data/pricingInfo/pricingModel").unwrap(),
            "FREE"
        );
        let input = client.get_input().await.unwrap();
        assert_eq!(input, None);
        assert_eq!(server.await.unwrap().len(), 3);
    }
}
