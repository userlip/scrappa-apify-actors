use chrono::{SecondsFormat, Utc};
use serde_json::{json, Map, Value};

use crate::request_params::IndicesParams;

pub fn canonical_symbol(value: Option<&Value>) -> Option<String> {
    let value = value.and_then(Value::as_str)?.trim();
    (!value.is_empty()).then(|| value.to_ascii_uppercase())
}

fn string(value: Option<&Value>) -> Option<String> {
    let value = value.and_then(Value::as_str)?.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn number(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(value) => value.as_f64().filter(|value| value.is_finite()),
        Value::String(value) if !value.trim().is_empty() => {
            let normalized: String = value
                .chars()
                .filter(|character| !matches!(character, '%' | '$' | ','))
                .collect();
            normalized
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
        }
        _ => None,
    }
}

fn first_non_null<'a>(
    row: &'a Map<String, Value>,
    primary: &str,
    fallback: &str,
) -> Option<&'a Value> {
    row.get(primary)
        .filter(|value| !value.is_null())
        .or_else(|| row.get(fallback))
}

fn object_rows(value: &Value) -> Option<Vec<Value>> {
    value.as_array().map(|items| {
        items
            .iter()
            .filter(|item| item.is_object() || item.is_array())
            .cloned()
            .collect()
    })
}

pub fn extract_index_rows(response: &Value) -> Vec<Value> {
    let candidate = [
        &response["data"],
        &response["indices"],
        &response["results"],
    ]
    .into_iter()
    .find(|value| !value.is_null());
    let Some(candidate) = candidate else {
        return Vec::new();
    };
    if let Some(rows) = object_rows(candidate) {
        return rows;
    }
    let Some(container) = candidate.as_object() else {
        return Vec::new();
    };
    [&container.get("indices"), &container.get("results")]
        .into_iter()
        .flatten()
        .find_map(|value| object_rows(value))
        .unwrap_or_default()
}

pub fn map_index_row(
    row: &Value,
    requested_symbol: &str,
    params: &IndicesParams,
    retrieved_at: Option<&str>,
) -> Option<Value> {
    let row = row.as_object()?;
    let symbol = canonical_symbol(row.get("symbol"))?;
    let exchange = string(row.get("exchange"));
    let id = format!(
        "{}:{symbol}",
        exchange
            .as_deref()
            .map(str::to_ascii_uppercase)
            .unwrap_or_else(|| "UNKNOWN".to_owned())
    );
    Some(json!({
        "id": id,
        "requested_symbol": requested_symbol,
        "symbol": symbol,
        "name": string(row.get("name")),
        "exchange": exchange,
        "current_price": number(first_non_null(row, "current_price", "price")),
        "price_change": number(first_non_null(row, "price_change", "change")),
        "percent_change": number(first_non_null(row, "percent_change", "price_change_percent")),
        "previous_close": number(row.get("previous_close")),
        "movement_direction": string(first_non_null(row, "movement_direction", "price_movement_direction")),
        "request_hl": params.hl,
        "request_gl": params.gl,
        "retrieved_at": retrieved_at.map(str::to_owned).unwrap_or_else(|| {
            Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
        }),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_params::IndicesParams;
    use serde_json::json;

    #[test]
    fn maps_aliases_and_normalizes_currency_numeric_values() {
        let rows = extract_index_rows(&json!({
            "results": [{
                "symbol": ".inx", "name": "S&P 500", "exchange": "indexsp",
                "price": "$6,200.50", "change": "-12.25", "price_change_percent": "-0.20%",
                "previous_close": "6,212.75", "price_movement_direction": "DOWN"
            }]
        }));
        let item = map_index_row(
            &rows[0],
            ".INX",
            &IndicesParams {
                indices: None,
                hl: "en".to_owned(),
                gl: "us".to_owned(),
            },
            Some("2026-07-12T00:00:00.000Z"),
        )
        .unwrap();
        assert_eq!(
            item,
            json!({
                "id":"INDEXSP:.INX", "requested_symbol":".INX", "symbol":".INX",
                "name":"S&P 500", "exchange":"indexsp", "current_price":6200.5,
                "price_change":-12.25, "percent_change":-0.2, "previous_close":6212.75,
                "movement_direction":"DOWN", "request_hl":"en", "request_gl":"us",
                "retrieved_at":"2026-07-12T00:00:00.000Z"
            })
        );
    }

    #[test]
    fn extracts_nested_indices_results_and_supports_current_aliases() {
        let rows = extract_index_rows(&json!({
            "data": {"results": [{
                "symbol": ".dji", "exchange": "indexdjx", "current_price": "44,000",
                "price_change": "100", "percent_change": "0.23", "movement_direction": "UP"
            }]}
        }));
        assert_eq!(rows.len(), 1);
        let item = map_index_row(
            &rows[0],
            ".DJI",
            &IndicesParams {
                indices: None,
                hl: "de".to_owned(),
                gl: "de".to_owned(),
            },
            Some("2026-07-12T01:00:00.000Z"),
        )
        .unwrap();
        assert_eq!(item["current_price"], 44000.0);
        assert!(item["previous_close"].is_null());
    }
}
