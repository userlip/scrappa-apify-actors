use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use url::Url;

use crate::config::endpoint_url;

pub(crate) fn json_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| match value {
                Value::Null => String::new(),
                _ => json_string(value),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

pub(crate) fn is_javascript_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn append_query_value(url: &mut Url, name: &str, value: Option<&Value>) {
    let Some(value) = value else {
        return;
    };
    if value.is_null() || value.as_str() == Some("") {
        return;
    }
    if let Value::Bool(boolean) = value {
        if *boolean {
            url.query_pairs_mut().append_pair(name, "1");
        }
        return;
    }
    url.query_pairs_mut().append_pair(name, &json_string(value));
}

pub(crate) fn build_search_url(input: &Value, api_base_url: &Url) -> Result<Url> {
    let mut url = endpoint_url(api_base_url, &["maps", "advance-search"])?;
    url.set_query(None);

    let query = input
        .get("query")
        .ok_or_else(|| anyhow!("Search query and zoom level are required"))?;
    let zoom = input
        .get("zoom")
        .ok_or_else(|| anyhow!("Search query and zoom level are required"))?;
    if !is_javascript_truthy(query) {
        bail!("Search query and zoom level are required");
    }

    append_query_value(&mut url, "query", Some(query));
    append_query_value(&mut url, "zoom", Some(zoom));
    append_query_value(&mut url, "lat", input.get("latitude"));
    append_query_value(&mut url, "lon", input.get("longitude"));
    append_query_value(&mut url, "limit", input.get("limit"));

    let language = input
        .get("hl")
        .filter(|value| is_javascript_truthy(value))
        .map(json_string)
        .unwrap_or_else(|| "en".to_owned());
    url.query_pairs_mut().append_pair("hl", &language);
    append_query_value(&mut url, "gl", input.get("gl"));

    Ok(url)
}
