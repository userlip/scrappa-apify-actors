use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde_json::{json, Map, Value};

fn first_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().find_map(|value| {
        value
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
    })
}

fn as_number(value: &Value) -> Option<Value> {
    if let Value::Number(number) = value {
        if number.as_i64().is_some() || number.as_u64().is_some() {
            return Some(value.clone());
        }
    }
    let number = match value {
        Value::Number(number) => number.as_f64()?,
        Value::String(value) => value.replace(',', "").trim().parse::<f64>().ok()?,
        _ => return None,
    };
    if !number.is_finite() {
        return None;
    }
    if number.fract() == 0.0 {
        if number >= i64::MIN as f64 && number <= i64::MAX as f64 {
            return Some(json!(number as i64));
        }
        if number >= 0.0 && number <= u64::MAX as f64 {
            return Some(json!(number as u64));
        }
    }
    serde_json::Number::from_f64(number).map(Value::Number)
}

fn first_number(values: &[Option<&Value>]) -> Value {
    values
        .iter()
        .find_map(|value| value.and_then(as_number))
        .unwrap_or(Value::Null)
}

fn date_to_iso(value: Option<&Value>) -> Option<String> {
    let value = value?.as_str()?.trim();
    if value.is_empty() {
        return None;
    }

    let parsed = DateTime::parse_from_str(value, "%b %e %Y, %I:%M %p UTC%:z")
        .map(|date| date.with_timezone(&Utc))
        .or_else(|_| DateTime::parse_from_rfc3339(value).map(|date| date.with_timezone(&Utc)))
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f").map(|date| date.and_utc())
        })
        .or_else(|_| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d").map(|date| {
                date.and_hms_opt(0, 0, 0)
                    .expect("midnight is a valid time")
                    .and_utc()
            })
        })
        .ok()?;
    Some(parsed.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())
}

pub(crate) fn build_intraday_price_point_dataset_items(
    response: &Value,
    params: &Map<String, Value>,
) -> Vec<Value> {
    let Some(graph) = response.get("graph").and_then(Value::as_array) else {
        return Vec::new();
    };
    graph
        .iter()
        .filter_map(Value::as_object)
        .enumerate()
        .map(|(index, point)| {
            let mut item = point.clone();
            let date = first_string(&[point.get("date")])
                .map(Value::String)
                .unwrap_or(Value::Null);
            let date_iso = date_to_iso(point.get("date"))
                .map(Value::String)
                .unwrap_or_else(|| {
                    if let Some(date) = point
                        .get("date")
                        .and_then(Value::as_str)
                        .filter(|date| !date.trim().is_empty())
                    {
                        eprintln!("Could not parse Google Finance intraday date: {date}");
                    }
                    Value::Null
                });
            let symbol = first_string(&[response.get("symbol"), params.get("symbol")])
                .map(Value::String)
                .unwrap_or(Value::Null);
            let exchange = first_string(&[response.get("exchange"), params.get("exchange")])
                .map(Value::String)
                .unwrap_or(Value::Null);
            let currency = first_string(&[point.get("currency"), response.get("currency")])
                .map(Value::String)
                .unwrap_or(Value::Null);

            item.insert("position".to_owned(), json!(index + 1));
            item.insert("date".to_owned(), date);
            item.insert("date_iso".to_owned(), date_iso);
            item.insert("price".to_owned(), first_number(&[point.get("price")]));
            item.insert("change".to_owned(), first_number(&[point.get("change")]));
            item.insert(
                "percent_change".to_owned(),
                first_number(&[point.get("percent_change")]),
            );
            item.insert("volume".to_owned(), first_number(&[point.get("volume")]));
            item.insert("symbol".to_owned(), symbol);
            item.insert("exchange".to_owned(), exchange);
            item.insert("currency".to_owned(), currency);
            for field in ["symbol", "exchange", "hl", "gl"] {
                item.insert(
                    format!("request_{field}"),
                    params.get(field).cloned().unwrap_or(Value::Null),
                );
            }
            Value::Object(item)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_graph_points_to_dataset_items_and_keeps_extra_fields() {
        let params = json!({ "symbol": "AAPL", "exchange": "NASDAQ", "hl": "en", "gl": "us" });
        let items = build_intraday_price_point_dataset_items(
            &json!({
                "symbol": "AAPL",
                "exchange": "NASDAQ",
                "currency": "USD",
                "graph": [{
                    "date": "Jun 16 2025, 09:30 AM UTC-04:00",
                    "price": "198.42",
                    "change": "1.25",
                    "percent_change": "0.63",
                    "volume": "3,482,103",
                    "vendor_field": "kept",
                }],
            }),
            params.as_object().unwrap(),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["position"], 1);
        assert_eq!(items[0]["date"], "Jun 16 2025, 09:30 AM UTC-04:00");
        assert_eq!(items[0]["date_iso"], "2025-06-16T13:30:00.000Z");
        assert_eq!(items[0]["price"], 198.42);
        assert_eq!(items[0]["change"], 1.25);
        assert_eq!(items[0]["percent_change"], 0.63);
        assert_eq!(items[0]["volume"], 3_482_103);
        assert_eq!(items[0]["currency"], "USD");
        assert_eq!(items[0]["vendor_field"], "kept");
        assert_eq!(items[0]["request_symbol"], "AAPL");
        assert_eq!(items[0]["request_exchange"], "NASDAQ");
        assert_eq!(items[0]["request_hl"], "en");
        assert_eq!(items[0]["request_gl"], "us");
        assert!(items[0].get("result_counts").is_none());
    }

    #[test]
    fn ignores_non_object_points_and_handles_missing_graph_and_invalid_dates() {
        let params = json!({ "symbol": "VOO", "exchange": "NYSEARCA" });
        assert!(build_intraday_price_point_dataset_items(
            &json!({ "currency": "USD" }),
            params.as_object().unwrap(),
        )
        .is_empty());
        let items = build_intraday_price_point_dataset_items(
            &json!({ "symbol": "VOO", "graph": [null, "invalid", { "date": "not a date", "price": "bad" }] }),
            params.as_object().unwrap(),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["position"], 1);
        assert_eq!(items[0]["date_iso"], Value::Null);
        assert_eq!(items[0]["price"], Value::Null);
        assert_eq!(items[0]["exchange"], "NYSEARCA");
    }
}
