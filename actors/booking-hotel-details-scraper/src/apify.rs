use crate::apify_retry::ApifyRetryExt;
use std::{env, time::Duration};

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::{json, Value};

use crate::billing::ChargingManager;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);

#[derive(Debug, Clone)]
pub struct ApifyClient {
    http: Client,
    base_url: String,
    token: String,
    actor_run_id: String,
    key_value_store_id: String,
    dataset_id: String,
    input_key: String,
    is_at_home: bool,
}

impl ApifyClient {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .timeout(APIFY_REQUEST_TIMEOUT)
                .build()
                .context("Could not create Apify HTTP client")?,
            base_url: env_or_default(
                "APIFY_API_PUBLIC_BASE_URL",
                &env_or_default("APIFY_API_BASE_URL", APIFY_API_DEFAULT),
            ),
            token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            key_value_store_id: env_or_default("ACTOR_DEFAULT_KEY_VALUE_STORE_ID", "default"),
            dataset_id: env_or_default("ACTOR_DEFAULT_DATASET_ID", "default"),
            input_key: env_or_default("ACTOR_INPUT_KEY", "INPUT"),
            is_at_home: env_bool("APIFY_IS_AT_HOME"),
        })
    }

    pub fn actor_run_id(&self) -> &str {
        &self.actor_run_id
    }

    fn endpoint(&self, resource: &[&str]) -> Result<Url> {
        let mut url =
            Url::parse(&self.base_url).context("Apify API base URL must be a valid URL")?;
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow!("Apify API base URL cannot be a base URL"))?;
        path.pop_if_empty();
        path.extend(["v2"].into_iter().chain(resource.iter().copied()));
        drop(path);
        Ok(url)
    }

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.http
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    async fn send_with_retry<F>(&self, operation: &str, mut build_request: F) -> Result<Response>
    where
        F: FnMut() -> reqwest::RequestBuilder,
    {
        build_request()
            .send_apify_with_retry()
            .await
            .with_context(|| format!("{operation} failed"))
    }

    pub async fn get_input(&self) -> Result<Option<Value>> {
        let url = self.endpoint(&[
            "key-value-stores",
            &self.key_value_store_id,
            "records",
            &self.input_key,
        ])?;
        let response = self
            .send_with_retry("fetch Actor input from the default key-value store", || {
                self.request(Method::GET, url.clone())
            })
            .await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let response = require_success(response, "fetch Actor input").await?;
        Ok(Some(
            response
                .json()
                .await
                .context("Actor input record is not valid JSON")?,
        ))
    }

    pub async fn get_charging_manager(&self) -> Result<ChargingManager> {
        let pricing_info = env::var("APIFY_ACTOR_PRICING_INFO").ok();
        let charged_counts = env::var("APIFY_CHARGED_ACTOR_EVENT_COUNTS").ok();
        if let (Some(pricing_info), Some(charged_counts)) = (pricing_info, charged_counts) {
            let pricing_info: Value = serde_json::from_str(&pricing_info)
                .context("APIFY_ACTOR_PRICING_INFO is not valid JSON")?;
            let charged_counts: Value = serde_json::from_str(&charged_counts)
                .context("APIFY_CHARGED_ACTOR_EVENT_COUNTS is not valid JSON")?;
            let max_total_charge = max_total_charge_from_env()?;
            return ChargingManager::from_values(
                &pricing_info,
                &charged_counts,
                max_total_charge,
                self.is_at_home,
            );
        }

        let url = self.endpoint(&["actor-runs", &self.actor_run_id])?;
        let response = self
            .send_with_retry("fetch Actor run pricing", || {
                self.request(Method::GET, url.clone())
            })
            .await?;
        let run = require_success(response, "fetch Actor run pricing")
            .await?
            .json()
            .await
            .context("Apify run pricing response is not valid JSON")?;
        ChargingManager::from_run(&run, self.is_at_home)
    }

    pub async fn push_dataset_item(&self, item: &Value) -> Result<()> {
        let url = self.endpoint(&["datasets", &self.dataset_id, "items"])?;
        let response = self
            .request(Method::POST, url)
            .json(item)
            .send_apify_with_retry()
            .await
            .context("store a result in the default dataset failed")?;
        require_success(response, "store dataset item").await?;
        Ok(())
    }

    pub async fn charge_event(
        &self,
        event_name: &str,
        count: usize,
        idempotency_key: &str,
    ) -> Result<()> {
        if idempotency_key.trim().is_empty() {
            return Err(anyhow!("charge event idempotency key must not be empty"));
        }

        let url = self.endpoint(&["actor-runs", &self.actor_run_id, "charge"])?;
        let body = json!({"eventName": event_name, "count": count});
        let response = self
            .send_with_retry("charge event", || {
                self.request(Method::POST, url.clone())
                    .header("idempotency-key", idempotency_key)
                    .json(&body)
            })
            .await?;
        require_success(response, "charge event").await?;
        Ok(())
    }

    pub async fn set_terminal_status(&self, message: &str, level: &str) {
        println!("[Status message]: {message}");
        let Ok(http) = Client::builder().timeout(Duration::from_secs(1)).build() else {
            eprintln!("Warning: could not create HTTP client to set Actor status message");
            return;
        };
        let Ok(mut url) = Url::parse(&self.base_url) else {
            eprintln!("Warning: could not parse Apify API base URL to set Actor status message");
            return;
        };
        let Ok(mut path) = url.path_segments_mut() else {
            eprintln!("Warning: could not build Apify API URL to set Actor status message");
            return;
        };
        path.pop_if_empty()
            .extend(["v2", "actor-runs", &self.actor_run_id]);
        drop(path);

        let result = http
            .put(url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&json!({
                "statusMessage": message,
                "isStatusMessageTerminal": true,
                "level": level,
            }))
            .send_apify_with_retry()
            .await;
        match result {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => eprintln!(
                "Warning: setting Actor status message failed with HTTP {}",
                response.status()
            ),
            Err(error) => eprintln!("Warning: setting Actor status message failed: {error}"),
        }
    }

    pub async fn set_terminal_status_from_env(message: &str, level: &str) {
        let (Ok(token), Ok(actor_run_id)) = (env::var("APIFY_TOKEN"), env::var("ACTOR_RUN_ID"))
        else {
            println!("[Status message]: {message}");
            return;
        };
        let base_url = env_or_default(
            "APIFY_API_PUBLIC_BASE_URL",
            &env_or_default("APIFY_API_BASE_URL", APIFY_API_DEFAULT),
        );
        println!("[Status message]: {message}");
        let Ok(http) = Client::builder().timeout(Duration::from_secs(1)).build() else {
            eprintln!("Warning: could not create HTTP client to set Actor status message");
            return;
        };
        let Ok(mut url) = Url::parse(&base_url) else {
            eprintln!("Warning: could not parse Apify API base URL to set Actor status message");
            return;
        };
        let Ok(mut path) = url.path_segments_mut() else {
            eprintln!("Warning: could not build Apify API URL to set Actor status message");
            return;
        };
        path.pop_if_empty()
            .extend(["v2", "actor-runs", &actor_run_id]);
        drop(path);
        let result = http
            .put(url)
            .bearer_auth(token)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&json!({
                "statusMessage": message,
                "isStatusMessageTerminal": true,
                "level": level,
            }))
            .send_apify_with_retry()
            .await;
        match result {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => eprintln!(
                "Warning: setting Actor status message failed with HTTP {}",
                response.status()
            ),
            Err(error) => eprintln!("Warning: setting Actor status message failed: {error}"),
        }
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

fn required_env(name: &str) -> Result<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("Required environment variable {name} is not set"))
}

fn env_bool(name: &str) -> bool {
    env::var(name)
        .ok()
        .is_some_and(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
}

fn max_total_charge_from_env() -> Result<f64> {
    let value = env::var("ACTOR_MAX_TOTAL_CHARGE_USD").ok();
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return Ok(f64::INFINITY);
    };
    let value: f64 = value
        .parse()
        .context("ACTOR_MAX_TOTAL_CHARGE_USD must be a number")?;
    if !value.is_finite() || value < 0.0 {
        return Err(anyhow!(
            "ACTOR_MAX_TOTAL_CHARGE_USD must be a valid non-negative number"
        ));
    }
    Ok(if value == 0.0 { f64::INFINITY } else { value })
}

async fn require_success(response: Response, operation: &str) -> Result<Response> {
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
    Err(anyhow!(
        "Apify API error ({status_code}) while trying to {operation}: {detail}"
    ))
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::{TcpListener, TcpStream},
        sync::mpsc,
        thread,
        time::Duration,
    };

    use super::ApifyClient;
    use anyhow::Result;
    use reqwest::Client;
    use serde_json::{json, Value};

    enum FirstDatasetResponse {
        ServerError,
        Lost,
    }

    struct MockDatasetServer {
        base_url: String,
        rows: mpsc::Receiver<Vec<Value>>,
        stop: mpsc::Sender<()>,
        thread: thread::JoinHandle<()>,
    }

    impl MockDatasetServer {
        fn start(first_response: FirstDatasetResponse) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let (stop, stop_requested) = mpsc::channel();
            let (stored_rows, rows) = mpsc::channel();
            let thread = thread::spawn(move || {
                let mut rows = Vec::new();
                let mut request_count = 0;
                loop {
                    if stop_requested.try_recv().is_ok() {
                        break;
                    }

                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream
                                .set_read_timeout(Some(Duration::from_secs(2)))
                                .unwrap();
                            let (method, offset, limit, item) =
                                read_dataset_request(&mut stream).unwrap();
                            if method == "GET" {
                                let page = rows
                                    .iter()
                                    .skip(offset)
                                    .take(limit)
                                    .cloned()
                                    .collect::<Vec<_>>();
                                write_json_response(&mut stream, 200, Value::Array(page));
                                continue;
                            }

                            rows.push(item.unwrap());
                            request_count += 1;
                            if request_count == 1 {
                                match first_response {
                                    FirstDatasetResponse::ServerError => {
                                        write_response(&mut stream, 500);
                                    }
                                    FirstDatasetResponse::Lost => {
                                        thread::sleep(Duration::from_millis(150));
                                    }
                                }
                            } else {
                                write_response(&mut stream, 201);
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(1));
                        }
                        Err(error) => panic!("mock Apify server failed: {error}"),
                    }
                }
                stored_rows.send(rows).unwrap();
            });

            Self {
                base_url: format!("http://{address}"),
                rows,
                stop,
                thread,
            }
        }

        fn finish(self) -> Vec<Value> {
            self.stop.send(()).unwrap();
            self.thread.join().unwrap();
            self.rows.recv().unwrap()
        }
    }

    fn client(base_url: String, timeout: Duration) -> ApifyClient {
        ApifyClient {
            http: Client::builder().timeout(timeout).build().unwrap(),
            base_url,
            token: "test-token".to_owned(),
            actor_run_id: "test-run".to_owned(),
            key_value_store_id: "default".to_owned(),
            dataset_id: "default".to_owned(),
            input_key: "INPUT".to_owned(),
            is_at_home: true,
        }
    }

    fn read_dataset_request(
        stream: &mut TcpStream,
    ) -> Result<(String, usize, usize, Option<Value>)> {
        let mut request = BufReader::new(stream);
        let mut line = String::new();
        request.read_line(&mut line)?;
        let mut parts = line.split_whitespace();
        let method = parts.next().unwrap_or_default().to_owned();
        let path = parts.next().unwrap_or_default();
        let query = path.split_once('?').map(|(_, query)| query).unwrap_or("");
        let query = url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect::<std::collections::HashMap<_, _>>();
        let offset = query
            .get("offset")
            .and_then(|value| value.parse().ok())
            .unwrap_or_default();
        let limit = query
            .get("limit")
            .and_then(|value| value.parse().ok())
            .unwrap_or(usize::MAX);
        let mut body_length = 0;
        loop {
            line.clear();
            if request.read_line(&mut line)? == 0 {
                anyhow::bail!("request ended before the dataset item was received");
            }
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    body_length = value.trim().parse()?;
                }
            }
        }

        if body_length == 0 {
            return Ok((method, offset, limit, None));
        }
        let mut body = vec![0; body_length];
        request.read_exact(&mut body)?;
        Ok((method, offset, limit, Some(serde_json::from_slice(&body)?)))
    }

    fn write_response(stream: &mut TcpStream, status: u16) {
        let reason = if status == 201 {
            "Created"
        } else {
            "Internal Server Error"
        };
        write!(
            stream,
            "HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
    }

    fn write_json_response(stream: &mut TcpStream, status: u16, body: Value) {
        let body = serde_json::to_vec(&body).unwrap();
        let reason = if status == 200 { "OK" } else { "Mock Response" };
        write!(
            stream,
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(&body).unwrap();
    }

    #[tokio::test]
    async fn confirms_dataset_append_after_server_error_without_reposting() {
        let server = MockDatasetServer::start(FirstDatasetResponse::ServerError);
        let item = json!({"hotel": "ritz-paris"});
        let result = client(server.base_url.clone(), Duration::from_secs(2))
            .push_dataset_item(&item)
            .await;
        let rows = server.finish();

        assert_eq!(rows, vec![item]);
        assert!(
            result.is_ok(),
            "the verification read should confirm the write"
        );
    }

    #[tokio::test]
    async fn confirms_dataset_append_after_lost_response_without_reposting() {
        let server = MockDatasetServer::start(FirstDatasetResponse::Lost);
        let item = json!({"hotel": "ritz-paris"});
        let result = client(server.base_url.clone(), Duration::from_millis(25))
            .push_dataset_item(&item)
            .await;
        let rows = server.finish();

        assert_eq!(rows, vec![item]);
        assert!(
            result.is_ok(),
            "the verification read should confirm the write"
        );
    }
}
