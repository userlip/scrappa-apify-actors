use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::{env, time::Duration};
use url::Url;

const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
pub const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostsParams {
    pub challenge_id: String,
    pub count: usize,
    pub region: Option<String>,
    pub cursor: Option<String>,
}

pub trait ScrappaApi {
    async fn get_posts(&self, params: &PostsParams) -> Result<Value>;
}

pub struct ScrappaClient {
    client: Client,
    base_url: Url,
    api_key: String,
}

impl ScrappaClient {
    pub fn from_env(api_key: String) -> Result<Self> {
        let raw_base_url =
            env::var("SCRAPPA_API_BASE_URL").unwrap_or_else(|_| SCRAPPA_API_BASE_URL.to_owned());
        let base_url = Url::parse(&raw_base_url)
            .context("SCRAPPA_API_BASE_URL must be a valid absolute URL")?;
        let client = Client::builder()
            .timeout(SCRAPPA_REQUEST_TIMEOUT)
            .build()
            .context("Failed to create Scrappa HTTP client")?;
        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_base_url(
        api_key: String,
        base_url: Url,
        request_timeout: Duration,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(request_timeout)
            .build()
            .context("Failed to create Scrappa HTTP client")?;
        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }

    fn request_url(&self, params: &PostsParams) -> Result<Url> {
        let mut url = self.base_url.clone();
        url.path_segments_mut()
            .map_err(|_| anyhow!("Scrappa API base URL cannot contain path segments"))?
            .pop_if_empty()
            .extend(["tiktok", "challenges", "posts"]);
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("challenge_id", &params.challenge_id);
            query.append_pair("count", &params.count.to_string());
            if let Some(region) = &params.region {
                query.append_pair("region", region);
            }
            if let Some(cursor) = &params.cursor {
                query.append_pair("cursor", cursor);
            }
        }
        Ok(url)
    }
}

impl ScrappaApi for ScrappaClient {
    async fn get_posts(&self, params: &PostsParams) -> Result<Value> {
        let url = self.request_url(params)?;
        let response = match self
            .client
            .get(url)
            .header("Accept", "application/json")
            .header("X-API-Key", &self.api_key)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) if error.is_timeout() => {
                return Err(anyhow!(
                    "Scrappa API request timed out after {}ms",
                    SCRAPPA_REQUEST_TIMEOUT.as_millis()
                ));
            }
            Err(error) => return Err(anyhow!("Scrappa API request failed: {error}")),
        };

        if !response.status().is_success() {
            return Err(anyhow!(
                "Scrappa API request failed with HTTP {}",
                response.status().as_u16()
            ));
        }
        response
            .json()
            .await
            .context("Scrappa API response returned invalid JSON")
    }
}
