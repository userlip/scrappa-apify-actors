use crate::input::ReviewsInput;
use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client, Response};
use serde_json::Value;
use std::{fmt, time::Duration};

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub struct ScrappaClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ScrappaClient {
    pub fn new(base_url: String, api_key: String) -> Result<Self> {
        Self::with_timeout(base_url, api_key, REQUEST_TIMEOUT)
    }

    fn with_timeout(base_url: String, api_key: String, timeout: Duration) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(timeout)
                .build()
                .context("Could not create Scrappa HTTP client")?,
            base_url,
            api_key,
        })
    }

    pub async fn get_reviews(&self, input: &ReviewsInput) -> Result<Value> {
        let url = input.request_url(&self.base_url)?;
        let response = self
            .client
            .get(url)
            .header("X-API-Key", &self.api_key)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    anyhow::Error::new(ScrappaTimeoutError)
                } else {
                    anyhow!("Scrappa API request failed: {error}")
                }
            })?;

        if !response.status().is_success() {
            return Err(read_api_error(response).await.into());
        }

        response
            .json()
            .await
            .context("Scrappa API response was not valid JSON")
    }
}

#[derive(Debug)]
pub struct ScrappaTimeoutError;

impl fmt::Display for ScrappaTimeoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API request timed out after {}ms",
            REQUEST_TIMEOUT.as_millis()
        )
    }
}

impl std::error::Error for ScrappaTimeoutError {}

#[derive(Debug)]
pub struct ScrappaApiError {
    status: u16,
    message: String,
}

impl fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status, self.message
        )
    }
}

impl std::error::Error for ScrappaApiError {}

async fn read_api_error(response: Response) -> ScrappaApiError {
    let status = response.status();
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    let body = response.text().await.unwrap_or_default();
    let message = if body.is_empty() {
        fallback
    } else if let Ok(Value::Object(error)) = serde_json::from_str::<Value>(&body) {
        let mut message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(&fallback)
            .to_owned();
        if let Some(errors) = error.get("errors").and_then(Value::as_object) {
            let details = errors
                .iter()
                .filter_map(|(field, messages)| {
                    let messages = messages.as_array()?;
                    let messages = messages
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ");
                    (!messages.is_empty()).then(|| format!("{field}: {messages}"))
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !details.is_empty() {
                message.push_str(" - ");
                message.push_str(&details);
            }
        }
        message
    } else {
        body.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(500)
            .collect()
    };
    ScrappaApiError {
        status: status.as_u16(),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::{ScrappaApiError, ScrappaClient};
    use crate::{
        input::ReviewsInput,
        test_support::{MockResponse, MockServer},
    };
    use serde_json::json;

    #[tokio::test]
    async fn sends_auth_and_query_and_preserves_the_response_body() {
        let response = json!({"items": [{"review_id": "r1"}], "nextPage": "page-2"});
        let server = MockServer::start(vec![MockResponse::json(200, response.clone())]);
        let client = ScrappaClient::new(server.base_url(), "scrappa-secret".to_owned()).unwrap();
        let input = ReviewsInput::parse(json!({
            "business_id": "place:with space",
            "sort": 2,
            "page": "next token"
        }))
        .unwrap();

        assert_eq!(client.get_reviews(&input).await.unwrap(), response);
        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET /api/maps/reviews?"));
        assert!(requests[0]
            .to_ascii_lowercase()
            .contains("x-api-key: scrappa-secret\r\n"));
        assert!(requests[0].contains("business_id=place%3Awith+space"));
        assert!(requests[0].contains("page=next+token"));
        assert!(requests[0].contains("use_cache=1"));
    }

    #[tokio::test]
    async fn formats_laravel_validation_errors_like_the_typescript_client() {
        let server = MockServer::start(vec![MockResponse::json(
            422,
            json!({"message": "The given data was invalid.", "errors": {"sort": ["The sort field is required."]}}),
        )]);
        let client = ScrappaClient::new(server.base_url(), "test-key".to_owned()).unwrap();
        let input = ReviewsInput::parse(json!({"business_id": "place", "sort": 2})).unwrap();
        let error = client.get_reviews(&input).await.unwrap_err();

        assert_eq!(
            error.downcast_ref::<ScrappaApiError>().unwrap().to_string(),
            "Scrappa API error (422): The given data was invalid. - sort: The sort field is required."
        );
        assert_eq!(server.requests().len(), 1);
    }

    #[tokio::test]
    async fn does_not_retry_scrappa_errors() {
        let server = MockServer::start(vec![MockResponse::json(503, json!({"message": "busy"}))]);
        let client = ScrappaClient::new(server.base_url(), "test-key".to_owned()).unwrap();
        let input = ReviewsInput::parse(json!({"business_id": "place", "sort": 2})).unwrap();

        assert!(client.get_reviews(&input).await.is_err());
        assert_eq!(server.requests().len(), 1);
    }

    #[test]
    fn retains_the_sixty_second_upstream_deadline_and_message() {
        assert_eq!(super::REQUEST_TIMEOUT.as_secs(), 60);
        assert_eq!(
            super::ScrappaTimeoutError.to_string(),
            "Scrappa API request timed out after 60000ms"
        );
    }
}
