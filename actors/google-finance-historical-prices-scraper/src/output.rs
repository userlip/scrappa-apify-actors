use chrono::{DateTime, Utc};
use serde_json::{json, Map, Number, Value};

use crate::input::HistoricalPricesRequest;

pub fn build_dataset_items(response: &Value, request: &HistoricalPricesRequest) -> Vec<Value> {
    let prices = response
        .get("prices")
        .and_then(Value::as_array)
        .map(|prices| {
            prices
                .iter()
                .filter_map(Value::as_object)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let result_count = prices.len();
    let request_value = request.to_value();
    let response_symbol = first_string(response.get("symbol"));
    let response_exchange = first_string(response.get("exchange"));
    let response_currency = first_string(response.get("currency"));
    let previous_close = first_number(response.get("previous_close"));

    prices
        .into_iter()
        .enumerate()
        .map(|(index, price)| {
            let mut item = price
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<Map<String, Value>>();

            item.insert("position".to_owned(), Value::from(index + 1));
            item.insert("date".to_owned(), first_number(price.get("date")));
            item.insert(
                "date_iso".to_owned(),
                timestamp_to_iso_date(price.get("date")),
            );
            item.insert("close".to_owned(), first_number(price.get("close")));
            item.insert("change".to_owned(), first_number(price.get("change")));
            item.insert(
                "percent_change".to_owned(),
                first_number(price.get("percent_change")),
            );
            item.insert("volume".to_owned(), first_number(price.get("volume")));
            item.insert(
                "symbol".to_owned(),
                response_symbol
                    .clone()
                    .unwrap_or_else(|| request.symbol.clone())
                    .into(),
            );
            item.insert(
                "exchange".to_owned(),
                response_exchange
                    .clone()
                    .or_else(|| request.exchange.clone())
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
            item.insert(
                "currency".to_owned(),
                response_currency
                    .clone()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
            item.insert("previous_close".to_owned(), previous_close.clone());
            for (name, value) in [
                ("request_symbol", Some(request.symbol.as_str())),
                ("request_exchange", request.exchange.as_deref()),
            ] {
                item.insert(
                    name.to_owned(),
                    value.map(Value::from).unwrap_or(Value::Null),
                );
            }
            item.insert(
                "request_range".to_owned(),
                request_value.get("range").cloned().unwrap_or(Value::Null),
            );
            item.insert(
                "request_start_date".to_owned(),
                request_value
                    .get("start_date")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
            item.insert(
                "request_end_date".to_owned(),
                request_value
                    .get("end_date")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
            for (name, value) in [
                ("request_interval", request.interval.as_deref()),
                ("request_hl", request.hl.as_deref()),
                ("request_gl", request.gl.as_deref()),
            ] {
                item.insert(
                    name.to_owned(),
                    value.map(Value::from).unwrap_or(Value::Null),
                );
            }
            item.insert(
                "result_counts".to_owned(),
                json!({ "prices": result_count }),
            );
            Value::Object(item)
        })
        .collect()
}

pub fn no_data_output(request: &HistoricalPricesRequest, message: &str) -> Value {
    json!({
        "symbol": request.symbol,
        "exchange": request.exchange,
        "prices": [],
        "request": request.to_value(),
        "error": message,
        "error_code": "NOT_FOUND",
        "status_code": 404,
    })
}

pub fn result_summary(response: &Value, request: &HistoricalPricesRequest, count: usize) -> Value {
    json!({
        "symbol": first_string(response.get("symbol")).unwrap_or_else(|| request.symbol.clone()),
        "exchange": first_string(response.get("exchange")).or_else(|| request.exchange.clone()),
        "prices": count,
        "range": request.range,
        "start_date": request.start_date,
        "end_date": request.end_date,
        "interval": request.interval,
    })
}

fn first_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn first_number(value: Option<&Value>) -> Value {
    let Some(value) = value else {
        return Value::Null;
    };
    let number = match value {
        Value::Number(number) => number.as_f64(),
        Value::String(value) => {
            let cleaned = value.replace(',', "");
            let cleaned = cleaned.trim();
            if cleaned.is_empty() {
                None
            } else {
                cleaned.parse::<f64>().ok()
            }
        }
        _ => None,
    }
    .filter(|number| number.is_finite());

    number.map(number_value).unwrap_or(Value::Null)
}

fn number_value(number: f64) -> Value {
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

    if number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER {
        return Value::from(number as i64);
    }

    Number::from_f64(number)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn timestamp_to_iso_date(value: Option<&Value>) -> Value {
    let Some(timestamp) = numeric_f64(value) else {
        return Value::Null;
    };
    let milliseconds = if timestamp > 100_000_000_000.0 {
        timestamp
    } else {
        timestamp * 1000.0
    };
    if !milliseconds.is_finite() || milliseconds.abs() > 8_640_000_000_000_000.0 {
        return Value::Null;
    }
    let date = DateTime::<Utc>::from_timestamp_millis(milliseconds.trunc() as i64);
    date.map(|date| Value::String(date.format("%Y-%m-%d").to_string()))
        .unwrap_or(Value::Null)
}

fn numeric_f64(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    match value {
        Value::Number(number) => number.as_f64().filter(|number| number.is_finite()),
        Value::String(value) => {
            let cleaned = value.replace(',', "");
            let cleaned = cleaned.trim();
            if cleaned.is_empty() {
                None
            } else {
                cleaned
                    .parse::<f64>()
                    .ok()
                    .filter(|number| number.is_finite())
            }
        }
        _ => None,
    }
}
