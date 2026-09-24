use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug)]
struct RuntimeConfig {
    is_at_home: bool,
    api_base_url: String,
    api_token: Option<String>,
    actor_run_id: Option<String>,
    default_dataset_id: String,
    default_key_value_store_id: String,
    input_key: String,
    local_storage_dir: PathBuf,
}

#[derive(Clone)]
pub struct ApifyRuntime {
    client: Client,
    config: RuntimeConfig,
}

impl ApifyRuntime {
    pub fn from_env() -> Result<Self, String> {
        let is_at_home = env_bool("APIFY_IS_AT_HOME");
        let config = RuntimeConfig {
            is_at_home,
            api_base_url: std::env::var("APIFY_API_BASE_URL")
                .unwrap_or_else(|_| "https://api.apify.com".to_owned()),
            api_token: std::env::var("APIFY_TOKEN").ok(),
            actor_run_id: std::env::var("APIFY_ACTOR_RUN_ID").ok(),
            default_dataset_id: std::env::var("APIFY_DEFAULT_DATASET_ID")
                .unwrap_or_else(|_| "default".to_owned()),
            default_key_value_store_id: std::env::var("APIFY_DEFAULT_KEY_VALUE_STORE_ID")
                .unwrap_or_else(|_| "default".to_owned()),
            input_key: std::env::var("APIFY_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            local_storage_dir: std::env::var("APIFY_LOCAL_STORAGE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./storage")),
        };
        Self::new(config)
    }

    fn new(config: RuntimeConfig) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|error| format!("Could not initialize Apify API client: {error}"))?;
        Ok(Self { client, config })
    }

    pub fn is_at_home(&self) -> bool {
        self.config.is_at_home
    }

    pub(crate) fn actor_run_id(&self) -> Option<&str> {
        self.config.actor_run_id.as_deref()
    }

    pub async fn get_current_run(&self) -> Result<Value, String> {
        if !self.config.is_at_home {
            return Ok(Value::Null);
        }
        let run_id = self.config.actor_run_id.as_deref().ok_or_else(|| {
            "Actor run ID not found even though the Actor is running on Apify".to_owned()
        })?;
        let response = self
            .api_request(Method::GET, &format!("actor-runs/{run_id}"))?
            .send()
            .await
            .map_err(|error| format!("Could not read Apify Actor run: {error}"))?;
        let body = successful_response(response, "Could not read Apify Actor run").await?;
        let body: Value = serde_json::from_slice(&body)
            .map_err(|error| format!("Apify returned invalid Actor run JSON: {error}"))?;
        Ok(body.get("data").cloned().unwrap_or(body))
    }

    pub async fn get_input(&self) -> Result<Option<Value>, String> {
        if self.config.is_at_home {
            let key = url_path_component(&self.config.input_key);
            let response = self
                .api_request(
                    Method::GET,
                    &format!(
                        "key-value-stores/{}/records/{key}",
                        self.config.default_key_value_store_id
                    ),
                )?
                .send()
                .await
                .map_err(|error| format!("Could not read Actor input: {error}"))?;
            if response.status() == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            let body = successful_response(response, "Could not read Actor input").await?;
            let value = serde_json::from_slice(&body)
                .map_err(|error| format!("Actor input is not valid JSON: {error}"))?;
            return Ok(Some(value));
        }

        let store = self
            .config
            .local_storage_dir
            .join("key_value_stores")
            .join(&self.config.default_key_value_store_id);
        let candidates = [
            store.join(format!("{}.json", self.config.input_key)),
            store.join(&self.config.input_key),
        ];
        for path in candidates {
            match tokio::fs::read(&path).await {
                Ok(body) => {
                    let value = serde_json::from_slice(&body)
                        .map_err(|error| format!("Actor input is not valid JSON: {error}"))?;
                    return Ok(Some(value));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("Could not read Actor input: {error}")),
            }
        }
        Ok(None)
    }

    pub async fn push_dataset_item(&self, item: &Value) -> Result<(), String> {
        if self.config.is_at_home {
            let response = self
                .api_request(
                    Method::POST,
                    &format!("datasets/{}/items", self.config.default_dataset_id),
                )?
                .json(item)
                .send()
                .await
                .map_err(|error| format!("Could not write Actor dataset item: {error}"))?;
            successful_response(response, "Could not write Actor dataset item").await?;
            return Ok(());
        }
        self.push_local_dataset_item(&self.config.default_dataset_id, item)
            .await
    }

    pub async fn set_output(&self, output: &Value) -> Result<(), String> {
        if self.config.is_at_home {
            let response = self
                .api_request(
                    Method::PUT,
                    &format!(
                        "key-value-stores/{}/records/OUTPUT",
                        self.config.default_key_value_store_id
                    ),
                )?
                .json(output)
                .send()
                .await
                .map_err(|error| format!("Could not write Actor OUTPUT: {error}"))?;
            successful_response(response, "Could not write Actor OUTPUT").await?;
            return Ok(());
        }
        let store = self
            .config
            .local_storage_dir
            .join("key_value_stores")
            .join(&self.config.default_key_value_store_id);
        tokio::fs::create_dir_all(&store)
            .await
            .map_err(|error| format!("Could not create Actor key-value store: {error}"))?;
        let body = serde_json::to_vec_pretty(output)
            .map_err(|error| format!("Could not serialize Actor OUTPUT: {error}"))?;
        tokio::fs::write(store.join("OUTPUT.json"), body)
            .await
            .map_err(|error| format!("Could not write Actor OUTPUT: {error}"))
    }

    pub async fn charge_event(
        &self,
        event_name: &str,
        count: u64,
        idempotency_key: &str,
    ) -> Result<(), String> {
        let run_id = self
            .config
            .actor_run_id
            .as_deref()
            .ok_or_else(|| "Actor run ID is required to charge an event".to_owned())?;
        let response = self
            .api_request(Method::POST, &format!("actor-runs/{run_id}/charge"))?
            .header("idempotency-key", idempotency_key)
            .json(&serde_json::json!({ "eventName": event_name, "count": count }))
            .send()
            .await
            .map_err(|error| format!("Could not charge Actor event '{event_name}': {error}"))?;
        successful_response(
            response,
            &format!("Could not charge Actor event '{event_name}'"),
        )
        .await?;
        Ok(())
    }

    pub async fn set_status_message(&self, message: &str) -> Result<(), String> {
        eprintln!("[Status message]: {message}");
        if !self.config.is_at_home {
            return Ok(());
        }
        let Some(run_id) = self.config.actor_run_id.as_deref() else {
            return Ok(());
        };
        let response = self
            .api_request(Method::PUT, &format!("actor-runs/{run_id}"))?
            .timeout(Duration::from_secs(1))
            .json(&serde_json::json!({ "statusMessage": message, "isStatusMessageTerminal": true }))
            .send()
            .await
            .map_err(|error| format!("Could not set Actor status message: {error}"))?;
        successful_response(response, "Could not set Actor status message").await?;
        Ok(())
    }

    pub async fn delete_local_dataset(&self, dataset_id: &str) -> Result<(), String> {
        if self.config.is_at_home {
            return Ok(());
        }
        let path = self
            .config
            .local_storage_dir
            .join("datasets")
            .join(dataset_id);
        match tokio::fs::remove_dir_all(path).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("Could not purge local dataset: {error}")),
        }
    }

    pub async fn push_named_dataset_item(
        &self,
        dataset_id: &str,
        item: &Value,
    ) -> Result<(), String> {
        if self.config.is_at_home {
            return Err("Local charging logs are not supported while running on Apify".to_owned());
        }
        self.push_local_dataset_item(dataset_id, item).await
    }

    async fn push_local_dataset_item(&self, dataset_id: &str, item: &Value) -> Result<(), String> {
        let directory = self
            .config
            .local_storage_dir
            .join("datasets")
            .join(dataset_id);
        tokio::fs::create_dir_all(&directory)
            .await
            .map_err(|error| format!("Could not create Actor dataset: {error}"))?;
        let mut entries = tokio::fs::read_dir(&directory)
            .await
            .map_err(|error| format!("Could not read Actor dataset: {error}"))?;
        let mut last_index = 0_u64;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|error| format!("Could not read Actor dataset: {error}"))?
        {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if stem.len() == 9 && stem.chars().all(|character| character.is_ascii_digit()) {
                if let Ok(index) = stem.parse::<u64>() {
                    last_index = last_index.max(index);
                }
            }
        }
        let next_index = last_index + 1;
        let file_name = format!("{next_index:09}.json");
        let body = serde_json::to_vec_pretty(item)
            .map_err(|error| format!("Could not serialize Actor dataset item: {error}"))?;
        tokio::fs::write(directory.join(file_name), body)
            .await
            .map_err(|error| format!("Could not write Actor dataset item: {error}"))
    }

    fn api_request(&self, method: Method, path: &str) -> Result<RequestBuilder, String> {
        let token = self
            .config
            .api_token
            .as_deref()
            .ok_or_else(|| "APIFY_TOKEN is required for Apify platform storage".to_owned())?;
        let url = format!(
            "{}/v2/{}",
            self.config.api_base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        Ok(self.client.request(method, url).bearer_auth(token))
    }
}

async fn successful_response(
    response: reqwest::Response,
    operation: &str,
) -> Result<Vec<u8>, String> {
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|error| format!("{operation}: {error}"))?;
    if status.is_success() {
        return Ok(body.to_vec());
    }
    let details = String::from_utf8_lossy(&body).trim().to_owned();
    Err(if details.is_empty() {
        format!("{operation} (HTTP {status})")
    } else {
        format!("{operation} (HTTP {status}): {details}")
    })
}

fn env_bool(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true"))
        .unwrap_or(false)
}

fn url_path_component(value: &str) -> String {
    let mut url = url::Url::parse("http://localhost/").expect("static URL is valid");
    url.path_segments_mut()
        .expect("URL can be a base")
        .push(value);
    url.path().trim_start_matches('/').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn local_runtime(path: PathBuf) -> ApifyRuntime {
        ApifyRuntime::new(RuntimeConfig {
            is_at_home: false,
            api_base_url: "http://127.0.0.1:1".to_owned(),
            api_token: None,
            actor_run_id: None,
            default_dataset_id: "default".to_owned(),
            default_key_value_store_id: "default".to_owned(),
            input_key: "INPUT".to_owned(),
            local_storage_dir: path,
        })
        .unwrap()
    }

    fn temporary_directory() -> PathBuf {
        std::env::temp_dir().join(format!(
            "redfin-actor-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[tokio::test]
    async fn reads_local_input_and_writes_dataset_and_output_using_apify_paths() {
        let path = temporary_directory();
        let runtime = local_runtime(path.clone());
        let input_directory = path.join("key_value_stores/default");
        tokio::fs::create_dir_all(&input_directory).await.unwrap();
        tokio::fs::write(
            input_directory.join("INPUT.json"),
            br#"{"property_id":194191988}"#,
        )
        .await
        .unwrap();
        assert_eq!(
            runtime.get_input().await.unwrap().unwrap()["property_id"],
            194191988
        );

        runtime
            .push_dataset_item(&serde_json::json!({"success":true}))
            .await
            .unwrap();
        runtime
            .push_dataset_item(&serde_json::json!({"success":false}))
            .await
            .unwrap();
        runtime
            .set_output(&serde_json::json!({"saved":1}))
            .await
            .unwrap();

        let dataset = path.join("datasets/default");
        assert!(dataset.join("000000001.json").exists());
        assert!(dataset.join("000000002.json").exists());
        let output: Value = serde_json::from_slice(
            &tokio::fs::read(input_directory.join("OUTPUT.json"))
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(output["saved"], 1);
        tokio::fs::remove_dir_all(path).await.unwrap();
    }

    #[tokio::test]
    async fn stores_dataset_item_then_charges_the_configured_ppe_event() {
        use crate::charging::{push_charged_valuation, ChargingManager};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        async fn read_request(stream: &mut tokio::net::TcpStream) -> String {
            let mut request = Vec::new();
            loop {
                let mut chunk = [0; 1024];
                let length = stream.read(&mut chunk).await.unwrap();
                request.extend_from_slice(&chunk[..length]);
                let Some(headers_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..headers_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or_default();
                if request.len() >= headers_end + 4 + content_length {
                    break;
                }
            }
            String::from_utf8(request).unwrap()
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                requests.push(read_request(&mut stream).await);
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                    )
                    .await
                    .unwrap();
            }
            requests
        });

        let runtime = ApifyRuntime::new(RuntimeConfig {
            is_at_home: true,
            api_base_url: format!("http://{address}"),
            api_token: Some("test-token".to_owned()),
            actor_run_id: Some("run-test".to_owned()),
            default_dataset_id: "dataset-test".to_owned(),
            default_key_value_store_id: "store-test".to_owned(),
            input_key: "INPUT".to_owned(),
            local_storage_dir: PathBuf::from("storage"),
        })
        .unwrap();
        let pricing = serde_json::json!({
            "pricingModel":"PAY_PER_EVENT",
            "pricingPerEvent":{"actorChargeEvents":{
                "valuation-result":{"eventPriceUsd":0.0005,"eventTitle":"Valuation result"}
            }}
        });
        let mut manager =
            ChargingManager::from_values(true, false, false, &pricing, &Value::Null, 0.001)
                .unwrap();
        let item = serde_json::json!({"success":true,"property_id":194191988});
        let result = push_charged_valuation(&runtime, &mut manager, &item, 0)
            .await
            .unwrap();
        assert_eq!(result.saved, true);
        assert_eq!(
            manager.calculate_max_event_charge_count_within_limit("valuation-result"),
            1.0
        );

        let requests = server.await.unwrap();
        assert!(requests[0].starts_with("POST /v2/datasets/dataset-test/items "));
        assert!(requests[0].contains("\"property_id\":194191988"));
        assert!(requests[1].starts_with("POST /v2/actor-runs/run-test/charge "));
        assert!(requests[1]
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token"));
        assert!(requests[1]
            .to_ascii_lowercase()
            .contains("idempotency-key: redfin-valuation-run-test-0-valuation-result"));
        assert!(requests[1].contains("\"eventName\":\"valuation-result\""));
    }

    #[tokio::test]
    async fn reads_hosted_input_and_writes_hosted_output_records() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (body, content_type) in [
                (r#"{"property_id":194191988}"#, "application/json"),
                ("{}", "application/json"),
            ] {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = [0; 4096];
                let length = stream.read(&mut buffer).await.unwrap();
                requests.push(String::from_utf8(buffer[..length].to_vec()).unwrap());
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });

        let runtime = ApifyRuntime::new(RuntimeConfig {
            is_at_home: true,
            api_base_url: format!("http://{address}"),
            api_token: Some("test-token".to_owned()),
            actor_run_id: Some("run-test".to_owned()),
            default_dataset_id: "dataset-test".to_owned(),
            default_key_value_store_id: "store-test".to_owned(),
            input_key: "INPUT".to_owned(),
            local_storage_dir: PathBuf::from("storage"),
        })
        .unwrap();
        assert_eq!(
            runtime.get_input().await.unwrap().unwrap()["property_id"],
            194191988
        );
        runtime
            .set_output(&serde_json::json!({"saved":1}))
            .await
            .unwrap();

        let requests = server.await.unwrap();
        assert!(requests[0].starts_with("GET /v2/key-value-stores/store-test/records/INPUT "));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer test-token"));
        assert!(requests[1].starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT "));
        assert!(requests[1].contains("\"saved\":1"));
    }
}
