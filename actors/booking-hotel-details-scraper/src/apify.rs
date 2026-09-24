use std::{env, time::Duration};

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::{json, Value};

use crate::billing::ChargingManager;

const APIFY_API_DEFAULT: &str = "https://api.apify.com";
const APIFY_REQUEST_TIMEOUT: Duration = Duration::from_secs(360);
const APIFY_MAX_RETRIES: usize = 8;
const APIFY_MAX_ATTEMPTS: usize = APIFY_MAX_RETRIES + 1;
const APIFY_RETRY_BASE_DELAY_MS: u64 = 500;

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
        for attempt in 1..=APIFY_MAX_ATTEMPTS {
            match build_request().send().await {
                Ok(response)
                    if is_retryable_status(response.status()) && attempt <= APIFY_MAX_RETRIES =>
                {
                    let status = response.status();
                    drop(response);
                    let delay = apify_retry_delay(attempt);
                    eprintln!(
                        "Apify API request failed with HTTP {status}. Retrying attempt {}/{} in {}ms.",
                        attempt + 1,
                        APIFY_MAX_ATTEMPTS,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Ok(response) => return Ok(response),
                Err(error)
                    if is_retryable_request_error(&error) && attempt <= APIFY_MAX_RETRIES =>
                {
                    let delay = apify_retry_delay(attempt);
                    eprintln!(
                        "Apify API request failed ({error}). Retrying attempt {}/{} in {}ms.",
                        attempt + 1,
                        APIFY_MAX_ATTEMPTS,
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(error) => {
                    return Err(error).with_context(|| format!("{operation} failed"));
                }
            }
        }

        unreachable!("the final Apify API retry returns or fails")
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
            .send()
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
            .send()
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
            .send()
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

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_retryable_request_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

fn apify_retry_delay(failed_attempt: usize) -> Duration {
    let multiplier = 2_u64.saturating_pow(failed_attempt.saturating_sub(1) as u32);
    Duration::from_millis(APIFY_RETRY_BASE_DELAY_MS.saturating_mul(multiplier))
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

    use anyhow::Result;
    use reqwest::Client;
    use reqwest::StatusCode;
    use serde_json::{json, Value};

    use super::{apify_retry_delay, is_retryable_status, ApifyClient};

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
                            let item = read_dataset_item(&mut stream).unwrap();
                            rows.push(item);
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

    fn read_dataset_item(stream: &mut TcpStream) -> Result<Value> {
        let mut request = BufReader::new(stream);
        let mut line = String::new();
        request.read_line(&mut line)?;
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

        let mut body = vec![0; body_length];
        request.read_exact(&mut body)?;
        Ok(serde_json::from_slice(&body)?)
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

    #[test]
    fn retries_rate_limits_and_server_errors_only() {
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(!is_retryable_status(StatusCode::REQUEST_TIMEOUT));
        assert!(!is_retryable_status(StatusCode::NOT_FOUND));
    }

    #[test]
    fn uses_apify_client_exponential_retry_delays() {
        assert_eq!(apify_retry_delay(1).as_millis(), 500);
        assert_eq!(apify_retry_delay(2).as_millis(), 1_000);
        assert_eq!(apify_retry_delay(8).as_millis(), 64_000);
    }

    #[tokio::test]
    async fn does_not_retry_dataset_append_after_server_error() {
        let server = MockDatasetServer::start(FirstDatasetResponse::ServerError);
        let item = json!({"hotel": "ritz-paris"});
        let result = client(server.base_url.clone(), Duration::from_secs(2))
            .push_dataset_item(&item)
            .await;
        let rows = server.finish();

        assert_eq!(rows, vec![item]);
        assert!(result.is_err(), "the first 5xx response should be returned");
    }

    #[tokio::test]
    async fn does_not_retry_dataset_append_after_lost_response() {
        let server = MockDatasetServer::start(FirstDatasetResponse::Lost);
        let item = json!({"hotel": "ritz-paris"});
        let result = client(server.base_url.clone(), Duration::from_millis(25))
            .push_dataset_item(&item)
            .await;
        let rows = server.finish();

        assert_eq!(rows, vec![item]);
        assert!(
            result.is_err(),
            "the timed-out append should be returned as an error"
        );
    }
}
