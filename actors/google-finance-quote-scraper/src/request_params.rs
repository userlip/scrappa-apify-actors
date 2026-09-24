use serde_json::{Map, Value};

pub type RequestParams = Map<String, Value>;

const PERIOD_TYPE_VALUES: &[&str] = &["quarterly", "annual"];

fn clean_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.encode_utf16().count() > max_length {
        return Err(format!(
            "{field} must be {max_length} characters or fewer"
        ));
    }
    Ok(Some(trimmed.to_owned()))
}

fn required_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<String, String> {
    clean_string(value, field, max_length)?.ok_or_else(|| format!("{field} is required"))
}

pub fn build_google_finance_quote_params(input: &Value) -> Result<RequestParams, String> {
    let fields = input.as_object();
    let symbol = required_string(fields.and_then(|fields| fields.get("symbol")), "symbol", 20)?
        .to_uppercase();
    if symbol
        .chars()
        .any(|character| character.is_whitespace() || character == '\u{feff}')
    {
        return Err("symbol cannot contain spaces".into());
    }

    let mut params = Map::new();
    params.insert("symbol".into(), Value::String(symbol));

    if let Some(exchange) = clean_string(
        fields.and_then(|fields| fields.get("exchange")),
        "exchange",
        40,
    )? {
        params.insert("exchange".into(), Value::String(exchange.to_uppercase()));
    }

    if let Some(period_type) = clean_string(
        fields.and_then(|fields| fields.get("period_type")),
        "period_type",
        20,
    )? {
        let normalized = period_type.to_lowercase();
        if !PERIOD_TYPE_VALUES.contains(&normalized.as_str()) {
            return Err(format!(
                "period_type must be one of: {}",
                PERIOD_TYPE_VALUES.join(", ")
            ));
        }
        params.insert("period_type".into(), Value::String(normalized));
    }

    if let Some(language) = clean_string(fields.and_then(|fields| fields.get("hl")), "hl", 10)? {
        let normalized = language.to_lowercase();
        let valid = match normalized.split_once('-') {
            Some((language, country)) => {
                language.len() == 2
                    && country.len() == 2
                    && language.bytes().all(|byte| byte.is_ascii_lowercase())
                    && country.bytes().all(|byte| byte.is_ascii_lowercase())
            }
            None => {
                normalized.len() == 2
                    && normalized.bytes().all(|byte| byte.is_ascii_lowercase())
            }
        };
        if !valid {
            return Err("hl must be a valid language code such as en, de, or zh-cn".into());
        }
        params.insert("hl".into(), Value::String(normalized));
    }

    if let Some(country) = clean_string(fields.and_then(|fields| fields.get("gl")), "gl", 2)? {
        if country.len() != 2 || !country.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            return Err("gl must be a two-letter country code".into());
        }
        params.insert("gl".into(), Value::String(country.to_lowercase()));
    }

    Ok(params)
}

pub fn describe_google_finance_quote_request(params: &RequestParams) -> String {
    let symbol = params
        .get("symbol")
        .and_then(Value::as_str)
        .unwrap_or("unknown symbol");
    let exchange = params
        .get("exchange")
        .and_then(Value::as_str)
        .map(|exchange| format!(":{exchange}"))
        .unwrap_or_default();
    let filters = ["period_type", "hl", "gl"]
        .into_iter()
        .filter_map(|field| {
            params
                .get(field)
                .filter(|value| !value.is_null())
                .map(|value| format!("{field}={}", js_string(value)))
        })
        .collect::<Vec<_>>();
    let filter_suffix = if filters.is_empty() {
        String::new()
    } else {
        format!(" ({})", filters.join(", "))
    };

    format!("{symbol}{exchange}{filter_suffix}")
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => "[object Object]".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_google_finance_quote_params, describe_google_finance_quote_request};
    use serde_json::json;

    #[test]
    fn normalizes_a_complete_quote_request() {
        assert_eq!(
            serde_json::Value::Object(build_google_finance_quote_params(&json!({
                "symbol": " aapl ",
                "exchange": " nasdaq ",
                "period_type": "ANNUAL",
                "hl": "EN",
                "gl": "US",
            })).unwrap()),
            json!({
                "symbol": "AAPL",
                "exchange": "NASDAQ",
                "period_type": "annual",
                "hl": "en",
                "gl": "us",
            })
        );
    }

    #[test]
    fn requires_a_non_empty_symbol_and_rejects_invalid_values() {
        assert_eq!(
            build_google_finance_quote_params(&json!({"symbol": "   "})).unwrap_err(),
            "symbol is required"
        );
        assert_eq!(
            build_google_finance_quote_params(&json!({"symbol": "BRK B"})).unwrap_err(),
            "symbol cannot contain spaces"
        );
        assert_eq!(
            build_google_finance_quote_params(&json!({"symbol": 123})).unwrap_err(),
            "symbol must be a string"
        );
        assert_eq!(
            build_google_finance_quote_params(&json!({"symbol": "AAPL", "hl": "english"}))
                .unwrap_err(),
            "hl must be a valid language code such as en, de, or zh-cn"
        );
        assert_eq!(
            build_google_finance_quote_params(&json!({"symbol": "AAPL", "gl": "usa"}))
                .unwrap_err(),
            "gl must be 2 characters or fewer"
        );
    }

    #[test]
    fn rejects_invalid_period_types_and_accepts_regional_languages() {
        assert_eq!(
            build_google_finance_quote_params(&json!({
                "symbol": "AAPL",
                "period_type": "monthly"
            }))
            .unwrap_err(),
            "period_type must be one of: quarterly, annual"
        );
        assert_eq!(
            build_google_finance_quote_params(&json!({
                "symbol": "AAPL",
                "hl": " ZH-CN ",
            }))
            .unwrap()["hl"],
            "zh-cn"
        );
    }

    #[test]
    fn describes_the_request_in_stable_field_order() {
        let params = build_google_finance_quote_params(&json!({
            "symbol": "AAPL",
            "exchange": "NASDAQ",
            "period_type": "quarterly",
            "hl": "en",
            "gl": "us",
        }))
        .unwrap();
        assert_eq!(
            describe_google_finance_quote_request(&params),
            "AAPL:NASDAQ (period_type=quarterly, hl=en, gl=us)"
        );
    }
}
