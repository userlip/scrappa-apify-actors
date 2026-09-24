use serde_json::{Map, Number, Value, json};

fn record<'a>(value: Option<&'a Value>) -> &'a Map<String, Value> {
    match value.and_then(Value::as_object) {
        Some(value) => value,
        None => empty_record(),
    }
}

fn empty_record() -> &'static Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Map::new)
}

fn array(value: Option<&Value>) -> &[Value] {
    value.and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn first_string<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Option<&'a str> {
    values.into_iter().flatten().find_map(|value| {
        value
            .as_str()
            .filter(|value| !value.trim().is_empty())
    })
}

fn javascript_number(value: &str) -> Option<f64> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let radix_value = if let Some(value) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
        Some((value, 16))
    } else if let Some(value) = value.strip_prefix("0b").or_else(|| value.strip_prefix("0B")) {
        Some((value, 2))
    } else {
        value
            .strip_prefix("0o")
            .or_else(|| value.strip_prefix("0O"))
            .map(|value| (value, 8))
    };
    if let Some((digits, radix)) = radix_value {
        return u64::from_str_radix(digits, radix).ok().map(|number| number as f64);
    }

    value.parse::<f64>().ok()
}

fn first_number<'a>(values: impl IntoIterator<Item = Option<&'a Value>>) -> Option<f64> {
    values.into_iter().flatten().find_map(|value| {
        let number = match value {
            Value::Number(value) => value.as_f64(),
            Value::String(value) => javascript_number(&value.replace(',', "")),
            _ => None,
        }?;
        number.is_finite().then_some(number)
    })
}

fn has_meaningful_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(_) => true,
        Value::Number(value) => value.as_f64().is_some_and(f64::is_finite),
        Value::String(value) => !value.trim().is_empty(),
        Value::Array(values) => values.iter().any(has_meaningful_value),
        Value::Object(values) => values.values().any(has_meaningful_value),
    }
}

fn first_array_string(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_array)
        .and_then(|values| first_string(values.iter().map(Some)))
}

fn extract_related_tickers(discover_more: &[Value]) -> Vec<Value> {
    let mut related = Vec::new();
    for section in discover_more {
        let Some(section) = section.as_object() else {
            continue;
        };
        for key in ["items", "results", "tickers", "quotes"] {
            if let Some(items) = section.get(key).and_then(Value::as_array) {
                related.extend(items.iter().cloned());
            }
        }
    }
    related
}

pub fn has_meaningful_quote_data(response: &Value) -> bool {
    let quote = record(response.get("quote"));
    let summary = record(quote.get("summary"));
    let about = record(quote.get("about"));
    let key_stats = record(quote.get("key_stats"));

    if first_number(
        ["current_price", "price", "last_price"]
            .into_iter()
            .map(|field| summary.get(field)),
    )
    .is_some()
    {
        return true;
    }
    if first_number(
        ["price_change", "change", "percent_change", "change_percent"]
            .into_iter()
            .map(|field| summary.get(field)),
    )
    .is_some()
    {
        return true;
    }
    if first_string(
        ["market_status", "market_state", "market"]
            .into_iter()
            .map(|field| summary.get(field))
            .chain([about.get("description")]),
    )
    .is_some()
    {
        return true;
    }

    if !array(quote.get("financials")).is_empty()
        || !array(quote.get("news")).is_empty()
        || !array(quote.get("discover_more")).is_empty()
    {
        return true;
    }

    key_stats.values().any(has_meaningful_value)
}

pub fn build_quote_dataset_item(response: &Value, params: &Map<String, Value>) -> Value {
    let quote = record(response.get("quote"));
    let summary = record(quote.get("summary"));
    let key_stats = record(quote.get("key_stats"));
    let about = record(quote.get("about"));
    let financials = array(quote.get("financials"));
    let news = array(quote.get("news"));
    let discover_more = array(quote.get("discover_more"));
    let related_tickers = extract_related_tickers(discover_more);

    let current_price = first_number(
        ["current_price", "price", "last_price"]
            .into_iter()
            .map(|field| summary.get(field)),
    );
    let price_change = first_number(
        ["price_change", "change"]
            .into_iter()
            .map(|field| summary.get(field)),
    );
    let percent_change = first_number(
        ["percent_change", "change_percent"]
            .into_iter()
            .map(|field| summary.get(field)),
    );
    let mut item = summary.clone();
    item.insert(
        "symbol".into(),
        first_string([summary.get("symbol"), params.get("symbol")])
            .map(|value| Value::String(value.to_owned()))
            .unwrap_or(Value::Null),
    );
    item.insert(
        "exchange".into(),
        first_string([summary.get("exchange"), params.get("exchange")])
            .map(|value| Value::String(value.to_owned()))
            .unwrap_or(Value::Null),
    );
    item.insert(
        "name".into(),
        first_string([
            summary.get("name"),
            summary.get("title"),
            about.get("name"),
        ])
        .map(|value| Value::String(value.to_owned()))
        .unwrap_or(Value::Null),
    );
    item.insert(
        "current_price".into(),
        current_price
            .and_then(Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    item.insert(
        "currency".into(),
        first_string([
            summary.get("currency"),
            key_stats.get("currency"),
        ])
        .or_else(|| first_array_string(summary.get("extensions")))
        .map(|value| Value::String(value.to_owned()))
        .unwrap_or(Value::Null),
    );
    item.insert(
        "price_change".into(),
        price_change
            .and_then(Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    item.insert(
        "percent_change".into(),
        percent_change
            .and_then(Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    item.insert(
        "market_status".into(),
        first_string(
            ["market_status", "market_state", "market"]
                .into_iter()
                .map(|field| summary.get(field)),
        )
        .map(|value| Value::String(value.to_owned()))
        .unwrap_or(Value::Null),
    );
    item.insert("key_stats".into(), Value::Object(key_stats.clone()));
    item.insert("about".into(), Value::Object(about.clone()));
    item.insert("financials".into(), Value::Array(financials.to_vec()));
    item.insert("news".into(), Value::Array(news.to_vec()));
    item.insert("discover_more".into(), Value::Array(discover_more.to_vec()));
    item.insert("related_tickers".into(), json!(related_tickers));
    if let Some(pagination) = quote
        .get("pagination")
        .or_else(|| response.get("pagination"))
    {
        item.insert("pagination".into(), pagination.clone());
    }
    for field in [
        "symbol",
        "exchange",
        "period_type",
        "hl",
        "gl",
    ] {
        let request_field = format!("request_{field}");
        item.insert(
            request_field,
            params.get(field).cloned().unwrap_or(Value::Null),
        );
    }
    item.insert(
        "result_counts".into(),
        json!({
            "financials": financials.len(),
            "news": news.len(),
            "discover_more": discover_more.len(),
            "related_tickers": related_tickers.len(),
        }),
    );

    Value::Object(item)
}

#[cfg(test)]
mod tests {
    use super::{build_quote_dataset_item, has_meaningful_quote_data};
    use serde_json::{Map, json};

    #[test]
    fn builds_one_flat_dataset_item_with_nested_quote_data() {
        let params: Map<String, serde_json::Value> = json!({
            "symbol": "AAPL",
            "exchange": "NASDAQ",
            "period_type": "quarterly",
            "hl": "en",
            "gl": "us",
        })
        .as_object()
        .unwrap()
        .clone();
        let item = build_quote_dataset_item(
            &json!({
                "quote": {
                    "summary": {
                        "symbol": "AAPL",
                        "exchange": "NASDAQ",
                        "name": "Apple Inc",
                        "current_price": "198.53",
                        "currency": "USD",
                        "price_change": "1.18",
                        "percent_change": "0.6",
                        "market_status": "Closed",
                    },
                    "key_stats": {"Market cap": "2.96T USD"},
                    "about": {"description": "Apple Inc. profile"},
                    "financials": [{"title": "Revenue"}],
                    "news": [{"title": "Apple news"}],
                    "discover_more": [{"title": "Related", "items": [{"symbol": "MSFT"}]}],
                }
            }),
            &params,
        );

        assert_eq!(item["symbol"], "AAPL");
        assert_eq!(item["exchange"], "NASDAQ");
        assert_eq!(item["current_price"], 198.53);
        assert_eq!(item["key_stats"], json!({"Market cap": "2.96T USD"}));
        assert_eq!(item["related_tickers"], json!([{"symbol": "MSFT"}]));
        assert_eq!(
            item["result_counts"],
            json!({"financials": 1, "news": 1, "discover_more": 1, "related_tickers": 1})
        );
    }

    #[test]
    fn keeps_summary_fields_and_falls_back_to_request_values() {
        let item = build_quote_dataset_item(
            &json!({
                "quote": {
                    "summary": {"custom_summary_field": "kept", "price": "125"},
                    "pagination": {"current_page": 1, "has_next_page": false},
                }
            }),
            &json!({"symbol": "VOO"}).as_object().unwrap().clone(),
        );

        assert_eq!(item["symbol"], "VOO");
        assert_eq!(item["current_price"], 125.0);
        assert_eq!(item["custom_summary_field"], "kept");
        assert_eq!(item["pagination"], json!({"current_page": 1, "has_next_page": false}));
        assert_eq!(item["financials"], json!([]));
        assert_eq!(item["news"], json!([]));
        assert_eq!(item["related_tickers"], json!([]));
    }

    #[test]
    fn extracts_only_related_ticker_arrays_from_discovery_sections() {
        let discover_more = json!([
            {"title": "You may be interested in", "description": "Popular market lists"},
            {"title": "Indexes", "groups": [{"symbol": ".INX"}], "quotes": [{"symbol": "SPY"}]}
        ]);
        let item = build_quote_dataset_item(
            &json!({"quote": {"discover_more": discover_more}}),
            &json!({"symbol": "AAPL"}).as_object().unwrap().clone(),
        );

        assert_eq!(item["discover_more"], discover_more);
        assert_eq!(item["related_tickers"], json!([{"symbol": "SPY"}]));
        assert_eq!(item["result_counts"]["discover_more"], 2);
    }

    #[test]
    fn identifies_usable_quote_content_without_accepting_metadata_alone() {
        for response in [
            json!({}),
            json!({"quote": {"summary": {}, "key_stats": {}, "about": {}}}),
            json!({"quote": {"summary": {"name": "Finance", "symbol": "MSFT", "exchange": "NASDAQ"}}}),
            json!({"quote": {"key_stats": {"stats": [], "tags": [], "climate_change": {}}}}),
            json!({"quote": {"key_stats": {"stats": [{}], "tags": [{"text": ""}], "climate_change": {"score": null}}}}),
        ] {
            assert!(!has_meaningful_quote_data(&response), "{response}");
        }

        for response in [
            json!({"quote": {"summary": {"current_price": 0}}}),
            json!({"quote": {"summary": {"current_price": "423.85"}}}),
            json!({"quote": {"summary": {"change": "0"}}}),
            json!({"quote": {"summary": {"change": "-1.27"}}}),
            json!({"quote": {"summary": {"market_state": "Closed"}}}),
            json!({"quote": {"about": {"description": "Microsoft Corporation profile"}}}),
            json!({"quote": {"key_stats": {"market_cap": "3.15T USD"}}}),
            json!({"quote": {"key_stats": {"stats": [{"label": "Avg volume", "value": "21M"}]}}}),
            json!({"quote": {"key_stats": {"is_index": false}}}),
            json!({"quote": {"financials": [{"title": "Income Statement"}]}}),
        ] {
            assert!(has_meaningful_quote_data(&response), "{response}");
        }
    }
}
