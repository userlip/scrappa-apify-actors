use serde_json::{json, Map, Value};

use crate::input::GoogleFinanceSearchRequest;

fn as_record(value: &Value) -> &Map<String, Value> {
    value.as_object().unwrap_or_else(|| empty_record())
}

fn empty_record() -> &'static Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Map::new)
}

fn first_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().flatten().find_map(|value| {
        value
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
    })
}

fn first_number(values: &[Option<&Value>]) -> Option<f64> {
    values.iter().flatten().find_map(|value| match value {
        Value::Number(number) => number.as_f64().filter(|number| number.is_finite()),
        Value::String(value) => {
            let cleaned = value.replace(',', "");
            let cleaned = cleaned.trim();
            if cleaned.is_empty() {
                return None;
            }
            parse_js_number(cleaned).filter(|number| number.is_finite())
        }
        _ => None,
    })
}

fn parse_js_number(value: &str) -> Option<f64> {
    let (radix, digits) = if let Some(value) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        (16, value)
    } else if let Some(value) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        (2, value)
    } else if let Some(value) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        (8, value)
    } else {
        return value.parse().ok();
    };
    u64::from_str_radix(digits, radix)
        .ok()
        .map(|number| number as f64)
}

fn build_google_finance_url(record: &Map<String, Value>) -> Option<String> {
    if let Some(link) = first_string(&[
        record.get("link"),
        record.get("url"),
        record.get("google_finance_url"),
    ]) {
        return Some(link);
    }
    if let Some(stock) = first_string(&[record.get("stock")]) {
        return Some(format!(
            "https://www.google.com/finance/quote/{}",
            encode_uri_component(&stock)
        ));
    }
    let symbol = first_string(&[record.get("symbol")])?;
    let exchange = first_string(&[record.get("exchange")])?;
    Some(format!(
        "https://www.google.com/finance/quote/{}",
        encode_uri_component(&format!("{symbol}:{exchange}"))
    ))
}

fn encode_uri_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
        {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn extract_search_results(response: &Value) -> &[Value] {
    for key in ["results", "search_results", "items"] {
        if let Some(results) = response
            .get(key)
            .and_then(Value::as_array)
            .filter(|results| !results.is_empty())
        {
            return results;
        }
    }
    if let Some(data) = response.get("data").and_then(Value::as_object) {
        for key in ["results", "search_results", "items"] {
            if let Some(results) = data
                .get(key)
                .and_then(Value::as_array)
                .filter(|results| !results.is_empty())
            {
                return results;
            }
        }
    }
    &[]
}

pub(crate) fn build_dataset_items(
    response: &Value,
    params: &GoogleFinanceSearchRequest,
) -> Vec<Value> {
    extract_search_results(response)
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let record = as_record(result);
            let price_movement = record.get("price_movement").and_then(Value::as_object);
            let google_finance_url = build_google_finance_url(record);
            json!({
                "query": params.q,
                "position": index + 1,
                "name": first_string(&[record.get("name"), record.get("title")]),
                "symbol": first_string(&[record.get("symbol")]),
                "exchange": first_string(&[record.get("exchange")]),
                "stock": first_string(&[record.get("stock")]),
                "type": first_string(&[record.get("type"), record.get("instrument_type"), record.get("asset_type")]),
                "currency": first_string(&[record.get("currency")]),
                "price": first_number(&[record.get("price"), record.get("extracted_price"), record.get("current_price")]),
                "price_change": first_number(&[record.get("price_change"), record.get("change"), price_movement.and_then(|movement| movement.get("value"))]),
                "percent_change": first_number(&[record.get("percent_change"), record.get("change_percent"), price_movement.and_then(|movement| movement.get("percentage"))]),
                "link": first_string(&[record.get("link"), record.get("url")]).or_else(|| google_finance_url.clone()),
                "google_finance_url": google_finance_url,
                "market": first_string(&[record.get("market"), record.get("region")]),
                "request_hl": params.hl,
                "request_gl": params.gl,
                "raw_result": Value::Object(record.clone()),
            })
        })
        .collect()
}

pub(crate) fn count_search_results(response: &Value) -> usize {
    extract_search_results(response).len()
}
