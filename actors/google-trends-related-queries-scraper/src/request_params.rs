use serde_json::{Map, Value};

const TIME_RANGE_VALUES: &[&str] = &["1h", "4h", "1d", "7d", "30d", "90d", "1y", "5y", "all"];
const SEARCH_TYPE_VALUES: &[&str] = &["web", "images", "news", "youtube", "shopping"];

fn clean_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };

    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        return Err(format!("{field} must be {max_length} characters or fewer"));
    }
    Ok(Some(value.to_owned()))
}

fn clean_required_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<String, String> {
    clean_string(value, field, max_length)?.ok_or_else(|| format!("{field} is required"))
}

fn clean_geo(value: Option<&Value>) -> Result<Option<String>, String> {
    let Some(geo) = clean_string(value, "geo", 10)? else {
        return Ok(None);
    };
    if geo.eq_ignore_ascii_case("worldwide") {
        return Ok(Some("Worldwide".to_owned()));
    }
    Ok(Some(geo.to_uppercase()))
}

fn clean_language(value: Option<&Value>) -> Result<Option<String>, String> {
    let Some(language) = clean_string(value, "hl", 2)? else {
        return Ok(None);
    };
    if language.len() != 2 || !language.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err("hl must be a two-letter language code".to_owned());
    }
    Ok(Some(language.to_ascii_lowercase()))
}

fn clean_enum(
    value: Option<&Value>,
    field: &str,
    values: &[&str],
) -> Result<Option<String>, String> {
    let Some(value) = clean_string(value, field, 20)? else {
        return Ok(None);
    };
    let normalized = value.to_ascii_lowercase();
    if !values.contains(&normalized.as_str()) {
        return Err(format!("{field} must be one of: {}", values.join(", ")));
    }
    Ok(Some(normalized))
}

fn clean_query_input(input: &Value) -> Result<String, String> {
    if let Some(query) = clean_string(input.get("query"), "query", 100)? {
        return Ok(query);
    }
    clean_required_string(input.get("q"), "query", 100)
}

pub fn build_related_params(input: &Value) -> Result<Map<String, Value>, String> {
    let mut params = Map::new();
    params.insert("q".to_owned(), Value::String(clean_query_input(input)?));

    if let Some(geo) = clean_geo(input.get("geo"))? {
        params.insert("geo".to_owned(), Value::String(geo));
    }
    if let Some(time_range) = clean_enum(input.get("time_range"), "time_range", TIME_RANGE_VALUES)?
    {
        params.insert("time_range".to_owned(), Value::String(time_range));
    }
    if let Some(language) = clean_language(input.get("hl"))? {
        params.insert("hl".to_owned(), Value::String(language));
    }
    if let Some(search_type) =
        clean_enum(input.get("search_type"), "search_type", SEARCH_TYPE_VALUES)?
    {
        params.insert("search_type".to_owned(), Value::String(search_type));
    }

    Ok(params)
}

pub fn build_autocomplete_params(params: &Map<String, Value>) -> Map<String, Value> {
    let mut autocomplete = Map::new();
    for key in ["q", "geo", "hl"] {
        if let Some(value) = params.get(key) {
            autocomplete.insert(key.to_owned(), value.clone());
        }
    }
    autocomplete
}

pub fn should_include_autocomplete(input: &Value) -> Result<bool, String> {
    match input.get("include_autocomplete") {
        None | Some(Value::Null) => Ok(false),
        Some(Value::String(value)) if value.is_empty() => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err("include_autocomplete must be a boolean".to_owned()),
    }
}

pub fn describe_related_request(params: &Map<String, Value>) -> String {
    let query = params
        .get("q")
        .and_then(Value::as_str)
        .unwrap_or("unknown query");
    let filters = ["geo", "time_range", "hl", "search_type"]
        .into_iter()
        .filter_map(|field| {
            params
                .get(field)
                .map(|value| format!("{field}={}", value.as_str().unwrap_or("")))
        })
        .collect::<Vec<_>>();
    let suffix = if filters.is_empty() {
        String::new()
    } else {
        format!(" ({})", filters.join(", "))
    };
    format!("\"{query}\"{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_normalized_related_query_params() {
        let params = build_related_params(&json!({
            "query": " coffee ",
            "geo": " us ",
            "time_range": "1Y",
            "hl": "EN",
            "search_type": "YouTube"
        }))
        .unwrap();

        assert_eq!(params["q"], "coffee");
        assert_eq!(params["geo"], "US");
        assert_eq!(params["time_range"], "1y");
        assert_eq!(params["hl"], "en");
        assert_eq!(params["search_type"], "youtube");
    }

    #[test]
    fn supports_q_alias_and_empty_query_fallback() {
        assert_eq!(
            build_related_params(&json!({"q":"bitcoin", "geo":"worldwide"})).unwrap()["geo"],
            "Worldwide"
        );
        assert_eq!(
            build_related_params(&json!({"query":"  ", "q":"coffee"})).unwrap()["q"],
            "coffee"
        );
        assert!(
            build_related_params(&json!({"query":"  "}))
                .unwrap_err()
                .contains("query is required")
        );
    }

    #[test]
    fn rejects_invalid_inputs_with_actor_messages() {
        assert_eq!(
            build_related_params(&json!({"query": 3})).unwrap_err(),
            "query must be a string"
        );
        assert!(
            build_related_params(&json!({"query":"x", "time_range":"2y"}))
                .unwrap_err()
                .contains("time_range must be one of")
        );
        assert_eq!(
            build_related_params(&json!({"query":"x", "hl":"eng"})).unwrap_err(),
            "hl must be 2 characters or fewer"
        );
        assert!(
            build_related_params(&json!({"query":"x", "search_type":"podcasts"}))
                .unwrap_err()
                .contains("search_type must be one of")
        );
        assert_eq!(
            should_include_autocomplete(&json!({"include_autocomplete":"true"})).unwrap_err(),
            "include_autocomplete must be a boolean"
        );
    }

    #[test]
    fn autocomplete_and_logs_use_only_supported_fields() {
        let params = build_related_params(&json!({
            "query":"tesla", "geo":"US", "time_range":"1y", "hl":"en", "search_type":"web"
        }))
        .unwrap();
        assert_eq!(
            build_autocomplete_params(&params),
            serde_json::from_value(json!({"q":"tesla", "geo":"US", "hl":"en"})).unwrap()
        );
        assert_eq!(
            describe_related_request(&params),
            "\"tesla\" (geo=US, time_range=1y, hl=en, search_type=web)"
        );
        assert!(!should_include_autocomplete(&json!({})).unwrap());
        assert!(should_include_autocomplete(&json!({"include_autocomplete":true})).unwrap());
    }

    #[test]
    fn preserves_query_prefill_and_backwards_compatible_alias_schema() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["required"], json!(["query"]));
        assert_eq!(schema["properties"]["query"]["maxLength"], 100);
        assert_eq!(schema["properties"]["query"]["prefill"], "coffee");
        assert_eq!(schema["properties"]["q"]["maxLength"], 100);
        assert!(
            schema["properties"]["q"]["description"]
                .as_str()
                .unwrap()
                .contains("Alias for query")
        );
    }
}
