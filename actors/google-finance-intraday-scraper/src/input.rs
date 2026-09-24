use anyhow::{anyhow, bail, Result};
use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct IntradayRequest {
    pub(crate) params: Map<String, Value>,
}

fn clean_optional_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(value.to_owned()))
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_optional_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_symbol(value: Option<&Value>, index: usize) -> Result<String> {
    let field = format!("symbols[{index}].symbol");
    let symbol = clean_required_string(value, &field, 20)?.to_uppercase();
    if symbol.chars().any(char::is_whitespace) {
        bail!("{field} cannot contain spaces");
    }
    Ok(symbol)
}

fn clean_exchange(value: Option<&Value>, index: usize) -> Result<Option<String>> {
    Ok(
        clean_optional_string(value, &format!("symbols[{index}].exchange"), 40)?
            .map(|exchange| exchange.to_uppercase()),
    )
}

fn clean_language_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(language) = clean_optional_string(value, "hl", 10)? else {
        return Ok(None);
    };
    let language = language.to_ascii_lowercase();
    let valid = match language.split_once('-') {
        Some((language, country)) => {
            language.len() == 2
                && country.len() == 2
                && language.bytes().all(|byte| byte.is_ascii_lowercase())
                && country.bytes().all(|byte| byte.is_ascii_lowercase())
                && !country.contains('-')
        }
        None => language.len() == 2 && language.bytes().all(|byte| byte.is_ascii_lowercase()),
    };
    if !valid {
        bail!("hl must be a valid language code such as en, de, or zh-cn");
    }
    Ok(Some(language))
}

fn clean_country_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(country) = clean_optional_string(value, "gl", 10)? else {
        return Ok(None);
    };
    let country = country.to_ascii_lowercase();
    if country.len() != 2 || !country.bytes().all(|byte| byte.is_ascii_lowercase()) {
        bail!("gl must be a two-letter country code");
    }
    Ok(Some(country))
}

pub(crate) fn build_intraday_requests(input: &Value) -> Result<Vec<IntradayRequest>> {
    let Some(input) = input.as_object() else {
        bail!("Input must be an object");
    };
    let Some(symbols) = input.get("symbols").and_then(Value::as_array) else {
        bail!("symbols must be an array");
    };
    if symbols.is_empty() {
        bail!("At least one symbol is required");
    }

    let hl = clean_language_code(input.get("hl"))?;
    let gl = clean_country_code(input.get("gl"))?;
    symbols
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let Some(item) = item.as_object() else {
                bail!("symbols[{index}] must be an object");
            };
            let mut params = Map::new();
            params.insert(
                "symbol".to_owned(),
                Value::String(clean_symbol(item.get("symbol"), index)?),
            );
            if let Some(exchange) = clean_exchange(item.get("exchange"), index)? {
                params.insert("exchange".to_owned(), Value::String(exchange));
            }
            if let Some(hl) = &hl {
                params.insert("hl".to_owned(), Value::String(hl.clone()));
            }
            if let Some(gl) = &gl {
                params.insert("gl".to_owned(), Value::String(gl.clone()));
            }
            Ok(IntradayRequest { params })
        })
        .collect()
}

pub(crate) fn describe_intraday_request(params: &Map<String, Value>) -> String {
    let symbol = params
        .get("symbol")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let exchange = params
        .get("exchange")
        .and_then(Value::as_str)
        .filter(|exchange| !exchange.is_empty())
        .map(|exchange| format!(":{exchange}"))
        .unwrap_or_default();
    let details = ["hl", "gl"]
        .iter()
        .filter_map(|field| {
            params
                .get(*field)
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(|value| format!("{field}={value}"))
        })
        .collect::<Vec<_>>();
    if details.is_empty() {
        format!("{symbol}{exchange}")
    } else {
        format!("{symbol}{exchange} ({})", details.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_normalized_requests_for_symbol_batches() {
        let requests = build_intraday_requests(&json!({
            "symbols": [
                { "symbol": " aapl ", "exchange": " nasdaq " },
                { "symbol": "msft" },
            ],
            "hl": "EN",
            "gl": "US",
        }))
        .unwrap();
        let expected = vec![
            json!({ "symbol": "AAPL", "exchange": "NASDAQ", "hl": "en", "gl": "us" })
                .as_object()
                .unwrap()
                .clone(),
            json!({ "symbol": "MSFT", "hl": "en", "gl": "us" })
                .as_object()
                .unwrap()
                .clone(),
        ];
        assert_eq!(
            requests
                .into_iter()
                .map(|request| request.params)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn preserves_input_validation_messages() {
        assert_eq!(
            build_intraday_requests(&json!({})).unwrap_err().to_string(),
            "symbols must be an array"
        );
        assert_eq!(
            build_intraday_requests(&json!({ "symbols": [] }))
                .unwrap_err()
                .to_string(),
            "At least one symbol is required"
        );
        assert!(build_intraday_requests(&json!({ "symbols": ["AAPL"] }))
            .unwrap_err()
            .to_string()
            .contains("symbols[0] must be an object"));
        assert!(
            build_intraday_requests(&json!({ "symbols": [{ "symbol": "BRK B" }] }))
                .unwrap_err()
                .to_string()
                .contains("symbols[0].symbol cannot contain spaces")
        );
        assert!(build_intraday_requests(
            &json!({ "symbols": [{ "symbol": "AAPL" }], "hl": "english" })
        )
        .unwrap_err()
        .to_string()
        .contains("hl must be a valid language code"));
        assert!(build_intraday_requests(
            &json!({ "symbols": [{ "symbol": "AAPL" }], "gl": "usa" })
        )
        .unwrap_err()
        .to_string()
        .contains("gl must be a two-letter country code"));
    }

    #[test]
    fn describes_requests_for_logs() {
        let params = json!({ "symbol": "AAPL", "exchange": "NASDAQ", "hl": "en", "gl": "us" });
        assert_eq!(
            describe_intraday_request(params.as_object().unwrap()),
            "AAPL:NASDAQ (hl=en, gl=us)"
        );
    }
}
