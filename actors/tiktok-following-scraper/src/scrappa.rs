use anyhow::{anyhow, bail, Context, Result};
use reqwest::Client;
use serde_json::{Map, Value};
use tokio::time::timeout;
use url::Url;

use crate::{
    input::FollowingParams,
    url_utils::endpoint_url,
    value::{js_number_string, js_string, js_truthy},
    REQUEST_TIMEOUT,
};

fn extract_string_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) => {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_owned())
        }
        Value::Number(value) if value.as_f64().is_some_and(|number| number.is_finite()) => {
            Some(js_number_string(value))
        }
        _ => None,
    }
}

pub(crate) fn extract_profile_user_id(data: Option<&Value>) -> Option<String> {
    let profile = match data? {
        Value::Array(values) => values.first()?,
        value => value,
    };
    if !profile.is_object() {
        return None;
    }

    extract_string_value(profile.get("user_id"))
        .or_else(|| extract_string_value(profile.get("id")))
        .or_else(|| {
            let user = profile.get("user")?;
            extract_string_value(user.get("user_id"))
                .or_else(|| extract_string_value(user.get("id")))
        })
}

pub(crate) fn following_items(data: Option<&Value>) -> &[Value] {
    match data {
        Some(Value::Array(values)) => values,
        Some(Value::Object(data)) => ["following", "followings", "users", "user_list"]
            .iter()
            .find_map(|field| data.get(*field).and_then(Value::as_array))
            .map(Vec::as_slice)
            .unwrap_or(&[]),
        _ => &[],
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Pagination {
    pub(crate) has_next_page: bool,
    pub(crate) next_time: Option<Value>,
}

pub(crate) fn extract_pagination(data: Option<&Value>) -> Pagination {
    let Some(data) = data.and_then(Value::as_object) else {
        return Pagination {
            has_next_page: false,
            next_time: None,
        };
    };

    let has_more = data
        .get("hasMore")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("has_more").filter(|value| !value.is_null()));
    let next_time = data
        .get("time")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("min_time").filter(|value| !value.is_null()))
        .or_else(|| data.get("max_time").filter(|value| !value.is_null()));

    Pagination {
        has_next_page: has_more.is_some_and(js_truthy),
        next_time: next_time.cloned(),
    }
}

pub(crate) fn dataset_item(user: &Value, unique_id: Option<&str>, user_id: Option<&str>) -> Value {
    let mut item = match user {
        Value::Object(fields) => fields.clone(),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        Value::String(value) => value
            .chars()
            .enumerate()
            .map(|(index, character)| (index.to_string(), Value::String(character.to_string())))
            .collect(),
        _ => Map::new(),
    };
    item.insert(
        "lookup_unique_id".to_owned(),
        unique_id.map_or(Value::Null, |value| Value::String(value.to_owned())),
    );
    item.insert(
        "lookup_user_id".to_owned(),
        user_id.map_or(Value::Null, |value| Value::String(value.to_owned())),
    );
    Value::Object(item)
}

pub(crate) fn following_url(
    base_url: &Url,
    params: &FollowingParams,
    count: usize,
    time: Option<&Value>,
) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["tiktok", "user", "following"])?;
    {
        let mut query = url.query_pairs_mut();
        if let Some(unique_id) = &params.unique_id {
            query.append_pair("unique_id", unique_id);
        }
        if let Some(user_id) = &params.user_id {
            query.append_pair("user_id", user_id);
        }
        query.append_pair("count", &count.to_string());
        if let Some(time) = time.filter(|value| {
            !value.is_null() && value.as_str() != Some("") && !matches!(value, Value::Bool(false))
        }) {
            let time = match time {
                Value::Bool(true) => "1".to_owned(),
                _ => js_string(time),
            };
            query.append_pair("time", &time);
        }
    }
    Ok(url)
}

pub(crate) fn profile_url(base_url: &Url, unique_id: &str) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["tiktok", "user", "profile"])?;
    url.query_pairs_mut().append_pair("unique_id", unique_id);
    Ok(url)
}

fn scrappa_request_error(error: reqwest::Error) -> anyhow::Error {
    if error.is_timeout() || error.to_string().contains("aborted") {
        anyhow!(
            "Scrappa API request timed out after {}s",
            REQUEST_TIMEOUT.as_secs()
        )
    } else {
        anyhow::Error::new(error)
    }
}

fn response_error_message(body: &str, fallback: &str) -> String {
    let Ok(error_data) = serde_json::from_str::<Value>(body) else {
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
        .map(js_string)
        .unwrap_or_else(|| fallback.to_owned());
    if let Some(errors) = error_data.get("errors").and_then(Value::as_object) {
        let details = errors
            .iter()
            .filter_map(|(field, messages)| {
                let messages = messages.as_array()?;
                let messages = messages
                    .iter()
                    .map(|message| {
                        if message.is_null() {
                            String::new()
                        } else {
                            js_string(message)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(format!("{field}: {messages}"))
            })
            .collect::<Vec<_>>()
            .join("; ");
        if !details.is_empty() {
            message.push_str(" - ");
            message.push_str(&details);
        }
    }
    message
}

pub(crate) async fn fetch_scrappa_json(client: &Client, url: &Url, api_key: &str) -> Result<Value> {
    let request = async {
        let response = client
            .get(url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .header("X-API-Key", api_key)
            .send()
            .await
            .map_err(scrappa_request_error)?;
        let status = response.status();
        if !status.is_success() {
            let fallback = status.canonical_reason().unwrap_or("Unknown error");
            let body = response.text().await.map_err(scrappa_request_error)?;
            let message = if body.is_empty() {
                fallback.to_owned()
            } else {
                response_error_message(&body, fallback)
            };
            bail!("Scrappa API error ({}): {message}", status.as_u16());
        }
        response
            .json()
            .await
            .context("Scrappa API response was not valid JSON")
    };

    timeout(REQUEST_TIMEOUT, request).await.map_err(|_| {
        anyhow!(
            "Scrappa API request timed out after {}s",
            REQUEST_TIMEOUT.as_secs()
        )
    })?
}

pub(crate) fn check_scrappa_code(response: &Value, endpoint: &str) -> Result<()> {
    let Some(code) = response.get("code") else {
        return Ok(());
    };
    if code.as_f64() == Some(0.0) {
        return Ok(());
    }
    let message = response
        .get("msg")
        .filter(|message| !message.is_null())
        .map(js_string)
        .unwrap_or_else(|| "Unknown error".to_owned());
    bail!(
        "Scrappa TikTok {endpoint} API returned code {}: {message}",
        js_string(code)
    );
}
