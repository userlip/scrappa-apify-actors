use anyhow::Result;
use serde_json::{Map, Value, json};

use crate::scrappa::{ScrappaClient, ScrappaHttpError};

const QUOTE_ENDPOINT: &str = "/google-finance/quote";

#[derive(Debug, PartialEq)]
pub struct QuoteFallback {
    pub reason: String,
    pub omitted_params: Vec<String>,
    pub primary_error: String,
}

pub struct QuoteFetchResult {
    pub response: Value,
    pub fallback: Option<QuoteFallback>,
}

impl QuoteFallback {
    pub fn to_value(&self) -> Value {
        json!({
            "reason": self.reason,
            "omitted_params": self.omitted_params,
            "primary_error": self.primary_error,
        })
    }
}

pub fn should_retry_base_quote(error: &anyhow::Error, params: &Map<String, Value>) -> bool {
    error
        .downcast_ref::<ScrappaHttpError>()
        .is_some_and(|error| (500..=599).contains(&error.status))
        && params.contains_key("period_type")
}

pub async fn fetch_quote_with_fallback(
    client: &ScrappaClient,
    params: &Map<String, Value>,
    attempts: usize,
) -> Result<QuoteFetchResult> {
    match client.get(QUOTE_ENDPOINT, params, attempts).await {
        Ok(response) => Ok(QuoteFetchResult {
            response,
            fallback: None,
        }),
        Err(error) if should_retry_base_quote(&error, params) => {
            let primary_error = error.to_string();
            let mut fallback_params = params.clone();
            fallback_params.remove("period_type");
            eprintln!(
                "Scrappa quote request failed ({primary_error}). Retrying base quote without period_type so the actor can still return quote data."
            );
            let response = client
                .get(QUOTE_ENDPOINT, &fallback_params, attempts)
                .await?;

            Ok(QuoteFetchResult {
                response,
                fallback: Some(QuoteFallback {
                    reason: "scrappa_5xx_after_financial_period_request".into(),
                    omitted_params: vec!["period_type".into()],
                    primary_error,
                }),
            })
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::should_retry_base_quote;
    use crate::scrappa::ScrappaHttpError;
    use serde_json::json;

    #[test]
    fn only_scrappa_5xx_with_a_period_filter_qualifies_for_fallback() {
        let params = json!({"symbol": "MSFT", "period_type": "quarterly"});
        let params = params.as_object().unwrap();
        assert!(should_retry_base_quote(
            &ScrappaHttpError::new(500, "Internal Server Error".into()).into(),
            params
        ));
        assert!(should_retry_base_quote(
            &ScrappaHttpError::new(599, "Unknown status".into()).into(),
            params
        ));
        assert!(!should_retry_base_quote(
            &ScrappaHttpError::new(429, "Rate limited".into()).into(),
            params
        ));
        assert!(!should_retry_base_quote(
            &ScrappaHttpError::new(500, "Internal Server Error".into()).into(),
            &json!({"symbol": "MSFT"}).as_object().unwrap().clone()
        ));
        assert!(!should_retry_base_quote(
            &anyhow::anyhow!("Scrappa API error (500): Internal Server Error"),
            params
        ));
    }
}
