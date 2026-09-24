use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, StatusCode, Url};
use serde_json::Value;
use tokio::time::{sleep, timeout};

use crate::{
    request_params::IndicesParams,
    runtime_config::{request_timeout, RunDeadline, REQUEST_ATTEMPTS, RETRY_BACKOFF_MS},
};

const DEFAULT_SCRAPPA_API_BASE: &str = "https://scrappa.co/api";
const RETRYABLE_STATUSES: [u16; 6] = [408, 429, 500, 502, 503, 504];

#[derive(Clone)]
pub struct ScrappaClient {
    http: Client,
    api_key: String,
    base_url: Url,
    deadline: RunDeadline,
}

impl ScrappaClient {
    #[cfg(test)]
    pub fn new(api_key: String, base_url: Option<&str>) -> Result<Self> {
        Self::new_with_deadline(
            api_key,
            base_url,
            RunDeadline::for_actor_timeout(Duration::from_secs(
                crate::runtime_config::ACTOR_TIMEOUT_SECONDS,
            ))?,
        )
    }

    pub fn new_with_deadline(
        api_key: String,
        base_url: Option<&str>,
        deadline: RunDeadline,
    ) -> Result<Self> {
        let raw_base_url = base_url.unwrap_or(DEFAULT_SCRAPPA_API_BASE);
        let base_url = Url::parse(raw_base_url)
            .with_context(|| format!("Invalid Scrappa API base URL: {raw_base_url}"))?;
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(request_timeout())
            .build()
            .context("Could not create Scrappa HTTP client")?;
        Ok(Self {
            http,
            api_key,
            base_url,
            deadline,
        })
    }

    pub async fn get_indices(&self, params: &IndicesParams, symbol: Option<&str>) -> Result<Value> {
        let url = self.indices_url(params, symbol)?;
        let mut last_error = None;

        for attempt in 1..=REQUEST_ATTEMPTS {
            let request_timeout = match self
                .deadline
                .request_timeout(request_timeout(), "a Scrappa request")
            {
                Ok(request_timeout) => request_timeout,
                Err(error) => return Err(error),
            };
            match timeout(request_timeout, self.get_once(&url, request_timeout)).await {
                Ok(Ok(response)) => return Ok(response),
                Ok(Err(error)) => {
                    let retryable = is_retryable_error(&error);
                    last_error = Some(error);
                    if !retryable || attempt == REQUEST_ATTEMPTS {
                        break;
                    }
                }
                Err(_) => {
                    last_error = Some(anyhow!(
                        "Scrappa API request timed out after {}ms",
                        request_timeout.as_millis()
                    ));
                    if attempt == REQUEST_ATTEMPTS {
                        break;
                    }
                }
            }

            let backoff = Duration::from_millis(attempt as u64 * RETRY_BACKOFF_MS);
            if self.deadline.remaining() <= backoff {
                break;
            }
            sleep(backoff).await;
        }

        Err(last_error.unwrap_or_else(|| anyhow!("Scrappa API request failed")))
    }

    fn indices_url(&self, params: &IndicesParams, symbol: Option<&str>) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("SCRAPPA_API_BASE_URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(["google-finance", "indices"]);
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("hl", &params.hl)
                .append_pair("gl", &params.gl);
            if let Some(symbol) = symbol.or(params.indices.as_deref()) {
                query.append_pair("indices", symbol);
            }
        }
        Ok(url)
    }

    async fn get_once(&self, url: &Url, request_timeout: Duration) -> Result<Value> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", &self.api_key)
            .header(reqwest::header::ACCEPT, "application/json")
            .timeout(request_timeout)
            .send()
            .await
            .context("Scrappa API request failed")?;
        let status = response.status();
        if !status.is_success() {
            return Err(ScrappaStatus(status).into());
        }
        response
            .json()
            .await
            .context("Scrappa API returned invalid JSON")
    }
}

fn is_retryable_error(error: &anyhow::Error) -> bool {
    if let Some(status) = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<ScrappaStatus>())
    {
        return RETRYABLE_STATUSES.contains(&status.0.as_u16());
    }
    error.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|error| error.is_timeout() || error.is_connect() || error.is_body())
    }) || error.to_string().contains("timed out")
}

#[derive(Debug)]
struct ScrappaStatus(StatusCode);

impl std::fmt::Display for ScrappaStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scrappa API error ({})", self.0.as_u16())
    }
}

impl std::error::Error for ScrappaStatus {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        request_params::IndicesParams,
        runtime_config::RunDeadline,
        test_support::{MockResponse, MockServer},
    };

    #[tokio::test]
    async fn retries_rate_limits_with_scrappa_auth_and_stops_on_ordinary_client_errors() {
        let server = MockServer::start(vec![
            MockResponse::json(429, "{}"),
            MockResponse::json(200, r#"{"indices":[{"symbol":".INX"}]}"#),
        ]);
        let client = ScrappaClient::new("secret-key".to_owned(), Some(&server.base_url)).unwrap();
        let params = IndicesParams {
            indices: None,
            hl: "en".to_owned(),
            gl: "us".to_owned(),
        };

        let result = client.get_indices(&params, Some(".INX")).await.unwrap();
        let requests = server.finish();
        assert_eq!(result["indices"][0]["symbol"], ".INX");
        assert_eq!(requests.len(), 2);
        assert!(requests[0].target.starts_with("/google-finance/indices?"));
        assert!(requests[0].target.contains("indices=.INX"));
        assert!(requests[0].target.contains("hl=en"));
        assert!(requests[0].target.contains("gl=us"));
        assert!(requests[0]
            .headers
            .to_ascii_lowercase()
            .contains("x-api-key: secret-key"));
        assert!(requests[0]
            .headers
            .to_ascii_lowercase()
            .contains("accept: application/json"));

        let server = MockServer::start(vec![MockResponse::json(404, "{}")]);
        let client = ScrappaClient::new("secret-key".to_owned(), Some(&server.base_url)).unwrap();
        let error = client.get_indices(&params, Some(".INX")).await.unwrap_err();
        assert!(error.to_string().contains("Scrappa API error (404)"));
        assert_eq!(server.finish().len(), 1);
    }

    #[test]
    fn retries_only_documented_http_errors() {
        for status in [408, 429, 500, 502, 503, 504] {
            assert!(is_retryable_error(&anyhow!(ScrappaStatus(
                StatusCode::from_u16(status).unwrap()
            ))));
        }
        assert!(!is_retryable_error(&anyhow!(ScrappaStatus(
            StatusCode::NOT_FOUND
        ))));
    }

    #[tokio::test]
    async fn does_not_start_a_scrappa_request_after_the_work_deadline() {
        let server = MockServer::start(Vec::new());
        let deadline = RunDeadline::for_work_window(Duration::ZERO).unwrap();
        let client = ScrappaClient::new_with_deadline(
            "secret-key".to_owned(),
            Some(&server.base_url),
            deadline,
        )
        .unwrap();
        let params = IndicesParams {
            indices: None,
            hl: "en".to_owned(),
            gl: "us".to_owned(),
        };

        assert!(client.get_indices(&params, Some(".INX")).await.is_err());
        assert!(server.finish().is_empty());
    }
}
