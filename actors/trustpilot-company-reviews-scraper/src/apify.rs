use anyhow::{Context, Result, anyhow, bail};
use reqwest::{Client, Method, Response, StatusCode, Url};
use serde_json::Value;

pub struct ApifyClient {
    client: Client,
    base_url: Url,
    token: String,
}

impl ApifyClient {
    pub fn new(base_url: &str, token: String) -> Result<Self> {
        Ok(Self {
            client: Client::new(),
            base_url: Url::parse(base_url)
                .context("APIFY_API_PUBLIC_BASE_URL must be a valid URL")?,
            token,
        })
    }

    fn record_url(&self, store_id: &str, record_key: &str) -> Result<Url> {
        self.resource_url(&["key-value-stores", store_id, "records", record_key])
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

    fn request(&self, method: Method, url: Url) -> reqwest::RequestBuilder {
        self.client
            .request(method, url)
            .bearer_auth(&self.token)
            .header(reqwest::header::ACCEPT, "application/json")
    }

    pub async fn get_input(&self, store_id: &str, record_key: &str) -> Result<Option<Value>> {
        let response = self
            .request(Method::GET, self.record_url(store_id, record_key)?)
            .send()
            .await
            .context("Failed to fetch Actor input from the default key-value store")?;
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

    pub async fn set_output(&self, store_id: &str, output: &Value) -> Result<()> {
        let response = self
            .request(Method::PUT, self.record_url(store_id, "OUTPUT")?)
            .json(output)
            .send()
            .await
            .context("Failed to write OUTPUT to the default key-value store")?;
        successful_response(response, "write OUTPUT").await?;
        Ok(())
    }

    pub async fn push_data(&self, dataset_id: &str, items: &[Value]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let response = self
            .request(
                Method::POST,
                self.resource_url(&["datasets", dataset_id, "items"])?,
            )
            .json(items)
            .send()
            .await
            .context("Failed to store review items in the default dataset")?;
        successful_response(response, "store dataset items").await?;
        Ok(())
    }
}

async fn successful_response(response: Response, operation: &str) -> Result<Response> {
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
    bail!("Apify API error ({status_code}) while trying to {operation}: {detail}");
}
