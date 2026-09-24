use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::{header, Client, StatusCode};
use serde_json::{Map, Value};
use url::Url;

use crate::{config::endpoint_url, input::BusinessIdRequest};

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const DETAILS_ENDPOINT: [&str; 2] = ["maps", "business-details"];

#[derive(Debug)]
pub struct ScrappaApiError {
    pub status: StatusCode,
    pub message: String,
}

impl std::fmt::Display for ScrappaApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Scrappa API error ({}): {}",
            self.status.as_u16(),
            self.message
        )
    }
}

impl std::error::Error for ScrappaApiError {}

pub struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
    timeout: Duration,
}

impl ScrappaClient {
    pub fn new(http: Client, base_url: Url, api_key: String, timeout: Duration) -> Self {
        Self {
            http,
            base_url,
            api_key,
            timeout,
        }
    }

    pub async fn get_business_details(
        &self,
        request: &BusinessIdRequest,
        input: &Value,
    ) -> Result<Value> {
        let url = endpoint_url(&self.base_url, &DETAILS_ENDPOINT)?;
        let params = build_business_details_params(&request.business_id, input);
        let response = self
            .http
            .get(url)
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .query(&params)
            .send()
            .await
            .map_err(|error| transport_error(error, self.timeout))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .map_err(|error| transport_error(error, self.timeout))
                .context("Failed to read Scrappa API error response")?;
            return Err(ScrappaApiError {
                status,
                message: scrappa_error_message(status, &body),
            }
            .into());
        }

        response
            .json::<Value>()
            .await
            .map_err(|error| transport_error(error, self.timeout))
            .context("Scrappa API response was not valid JSON")
    }
}

fn transport_error(error: reqwest::Error, timeout: Duration) -> anyhow::Error {
    if error.is_timeout() {
        anyhow!(
            "Scrappa API request timed out after {}ms",
            timeout.as_millis()
        )
    } else {
        error.into()
    }
}

fn build_business_details_params(business_id: &str, input: &Value) -> Vec<(String, String)> {
    let mut params = vec![("business_id".to_owned(), business_id.to_owned())];

    if input.get("use_cache") != Some(&Value::Bool(false)) {
        params.push(("use_cache".to_owned(), "1".to_owned()));
    }

    if let Some(maximum_cache_age) = input
        .get("maximum_cache_age")
        .filter(|value| !value.is_null() && *value != "")
    {
        params.push((
            "maximum_cache_age".to_owned(),
            javascript_string(maximum_cache_age),
        ));
    }

    params
}

fn javascript_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                integer.to_string()
            } else if let Some(integer) = value.as_u64() {
                integer.to_string()
            } else if let Some(float) = value.as_f64() {
                if float.fract() == 0.0 {
                    format!("{float:.0}")
                } else {
                    float.to_string()
                }
            } else {
                value.to_string()
            }
        }
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    javascript_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn scrappa_error_message(status: StatusCode, body: &str) -> String {
    let fallback = status
        .canonical_reason()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));

    let Ok(error_data) = serde_json::from_str::<Value>(body) else {
        if body.is_empty() {
            return fallback;
        }
        return body
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(500)
            .collect();
    };

    let mut message = error_data
        .get("message")
        .filter(|value| !value.is_null())
        .map(javascript_string)
        .unwrap_or(fallback);

    if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
        let details = error_details(errors);
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details);
        }
    }

    message
}

fn error_details(errors: &Map<String, Value>) -> String {
    errors
        .iter()
        .map(|(field, messages)| {
            let messages = messages
                .as_array()
                .map(|messages| {
                    messages
                        .iter()
                        .map(javascript_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| javascript_string(messages));
            format!("{field}: {messages}")
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, time::Duration};

    use reqwest::Client;
    use serde_json::{json, Value};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        task::JoinHandle,
    };
    use url::Url;

    use crate::input::BusinessIdRequest;

    use super::{
        build_business_details_params, scrappa_error_message, ScrappaApiError, ScrappaClient,
        REQUEST_TIMEOUT,
    };

    fn request() -> BusinessIdRequest {
        BusinessIdRequest {
            input_business_id: "maps-id".to_owned(),
            business_id: "maps-id".to_owned(),
        }
    }

    #[test]
    fn cache_parameters_match_the_existing_actor_rules() {
        assert_eq!(
            build_business_details_params("maps-id", &json!({"maximum_cache_age": 0})),
            [
                ("business_id".to_owned(), "maps-id".to_owned()),
                ("use_cache".to_owned(), "1".to_owned()),
                ("maximum_cache_age".to_owned(), "0".to_owned()),
            ]
        );
        assert_eq!(
            build_business_details_params("maps-id", &json!({"maximum_cache_age": 3600.0}))[2],
            ("maximum_cache_age".to_owned(), "3600".to_owned())
        );
        assert_eq!(
            build_business_details_params(
                "maps-id",
                &json!({"use_cache": false, "maximum_cache_age": null})
            ),
            [("business_id".to_owned(), "maps-id".to_owned())]
        );
    }

    #[test]
    fn formats_json_and_plain_text_api_errors() {
        assert_eq!(
            scrappa_error_message(
                reqwest::StatusCode::UNPROCESSABLE_ENTITY,
                r#"{"message":"Invalid request","errors":{"business_id":["is required","is invalid"]}}"#
            ),
            "Invalid request - business_id: is required, is invalid"
        );
        assert_eq!(
            scrappa_error_message(
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                "  upstream\nfailed  "
            ),
            "upstream failed"
        );
    }

    async fn mock_response(
        status: u16,
        body: &'static str,
        delay: Duration,
    ) -> (Url, JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address: SocketAddr = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 8192];
            let length = socket.read(&mut request).await.unwrap();
            tokio::time::sleep(delay).await;
            let reason = if status == 404 { "Not Found" } else { "OK" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            request.truncate(length);
            request
        });
        (Url::parse(&format!("http://{address}/api")).unwrap(), task)
    }

    #[tokio::test]
    async fn sends_the_expected_endpoint_auth_headers_and_query() {
        let (base_url, server) = mock_response(200, r#"{"data":[]}"#, Duration::ZERO).await;
        let http = Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap();
        let client = ScrappaClient::new(http, base_url, "secret-key".to_owned(), REQUEST_TIMEOUT);

        let response = client
            .get_business_details(
                &request(),
                &json!({"use_cache": false, "maximum_cache_age": 0}),
            )
            .await
            .unwrap();
        let request = String::from_utf8(server.await.unwrap())
            .unwrap()
            .to_ascii_lowercase();

        assert_eq!(response, json!({"data": []}));
        assert!(request.starts_with(
            "get /api/maps/business-details?business_id=maps-id&maximum_cache_age=0 http/1.1\r\n"
        ));
        assert!(request.contains("x-api-key: secret-key\r\n"));
        assert!(request.contains("accept: application/json\r\n"));
    }

    #[tokio::test]
    async fn preserves_404_as_a_typed_api_error_and_does_not_retry_scrappa() {
        let (base_url, server) =
            mock_response(404, r#"{"message":"Business not found"}"#, Duration::ZERO).await;
        let client = ScrappaClient::new(
            Client::builder().timeout(REQUEST_TIMEOUT).build().unwrap(),
            base_url,
            "secret-key".to_owned(),
            REQUEST_TIMEOUT,
        );

        let error = client
            .get_business_details(&request(), &json!({}))
            .await
            .unwrap_err();
        let captured_request = server.await.unwrap();

        assert_eq!(
            error.downcast_ref::<ScrappaApiError>().unwrap().status,
            reqwest::StatusCode::NOT_FOUND
        );
        assert_eq!(
            error.to_string(),
            "Scrappa API error (404): Business not found"
        );
        assert!(String::from_utf8(captured_request)
            .unwrap()
            .starts_with("GET "));
    }

    #[tokio::test]
    async fn applies_a_bounded_request_deadline() {
        let timeout = Duration::from_millis(25);
        let (base_url, server) =
            mock_response(200, r#"{"data":[]}"#, Duration::from_millis(100)).await;
        let client = ScrappaClient::new(
            Client::builder().timeout(timeout).build().unwrap(),
            base_url,
            "secret-key".to_owned(),
            timeout,
        );

        let error = client
            .get_business_details(&request(), &Value::Null)
            .await
            .unwrap_err();
        let _ = server.await;

        assert_eq!(
            error.to_string(),
            "Scrappa API request timed out after 25ms"
        );
    }
}
