use serde_json::{json, Map, Number, Value};

const OVERVIEW_SECTION_KEYS: [&str; 6] =
    ["us", "europe", "asia", "currencies", "crypto", "futures"];

fn record(value: Option<&Value>) -> &Map<String, Value> {
    value
        .and_then(Value::as_object)
        .unwrap_or_else(|| empty_record())
}

fn empty_record() -> &'static Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Map::new)
}

fn array(value: Option<&Value>) -> &[Value] {
    value
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn first_string(record: &Map<String, Value>, fields: &[&str]) -> Value {
    fields
        .iter()
        .filter_map(|field| record.get(*field).and_then(Value::as_str))
        .find(|value| !value.trim().is_empty())
        .map(|value| Value::String(value.to_owned()))
        .unwrap_or(Value::Null)
}

fn js_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(value) => value.as_f64().filter(|value| value.is_finite()),
        Value::String(value) => {
            let cleaned = value.replace(',', "");
            let cleaned = cleaned.trim();
            if cleaned.is_empty() {
                return None;
            }
            let parsed = match cleaned
                .strip_prefix("0x")
                .or_else(|| cleaned.strip_prefix("0X"))
            {
                Some(hex) => u64::from_str_radix(hex, 16)
                    .ok()
                    .map(|number| number as f64),
                None => match cleaned
                    .strip_prefix("0b")
                    .or_else(|| cleaned.strip_prefix("0B"))
                {
                    Some(binary) => u64::from_str_radix(binary, 2)
                        .ok()
                        .map(|number| number as f64),
                    None => match cleaned
                        .strip_prefix("0o")
                        .or_else(|| cleaned.strip_prefix("0O"))
                    {
                        Some(octal) => u64::from_str_radix(octal, 8)
                            .ok()
                            .map(|number| number as f64),
                        None => cleaned.parse::<f64>().ok(),
                    },
                },
            };
            parsed.filter(|value| value.is_finite())
        }
        _ => None,
    }
}

fn first_number(record: &Map<String, Value>, fields: &[&str]) -> Value {
    fields
        .iter()
        .filter_map(|field| record.get(*field).and_then(js_number))
        .find_map(Number::from_f64)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn parameter(params: &Map<String, Value>, field: &str) -> Value {
    params.get(field).cloned().unwrap_or(Value::Null)
}

fn build_market_row(
    item: &Value,
    section: &str,
    position: usize,
    params: &Map<String, Value>,
    trend_group_title: Value,
) -> Value {
    let item = record(Some(item));
    let movement = record(item.get("price_movement"));
    json!({
        "item_type": "market_row",
        "section": section,
        "trend": parameter(params, "trend"),
        "trend_group": trend_group_title,
        "position": position,
        "stock": first_string(item, &["stock"]),
        "link": first_string(item, &["link"]),
        "name": first_string(item, &["name"]),
        "symbol": first_string(item, &["symbol"]),
        "exchange": first_string(item, &["exchange"]),
        "price": first_number(item, &["price", "extracted_price"]),
        "currency": first_string(item, &["currency"]),
        "price_movement_direction": first_string(movement, &["direction"]),
        "price_movement_value": first_number(movement, &["value"]),
        "price_movement_percentage": first_number(movement, &["percentage"]),
        "from_currency": first_string(item, &["from_currency"]),
        "to_currency": first_string(item, &["to_currency"]),
        "request_trend": parameter(params, "trend"),
        "request_index_market": parameter(params, "index_market"),
        "request_hl": parameter(params, "hl"),
        "request_gl": parameter(params, "gl")
    })
}

fn build_news_row(item: &Value, position: usize, params: &Map<String, Value>) -> Value {
    let item = record(Some(item));
    json!({
        "item_type": "news_result",
        "section": "finance-news",
        "trend": parameter(params, "trend"),
        "trend_group": Value::Null,
        "position": position,
        "title": first_string(item, &["title"]),
        "link": first_string(item, &["link"]),
        "source": first_string(item, &["source"]),
        "date": first_string(item, &["date"]),
        "snippet": first_string(item, &["snippet"]),
        "thumbnail": first_string(item, &["thumbnail"]),
        "request_trend": parameter(params, "trend"),
        "request_index_market": parameter(params, "index_market"),
        "request_hl": parameter(params, "hl"),
        "request_gl": parameter(params, "gl")
    })
}

pub fn build_markets_dataset_items(response: &Value, params: &Map<String, Value>) -> Vec<Value> {
    let mut items = Vec::new();
    for (group_index, group) in array(response.get("market_trends")).iter().enumerate() {
        let group = record(Some(group));
        let title = first_string(group, &["title"]);
        let title_string = title.as_str().filter(|value| !value.trim().is_empty());
        let section = title_string
            .map(str::to_owned)
            .or_else(|| {
                params
                    .get("trend")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| format!("trend-{}", group_index + 1));
        for (index, result) in array(group.get("results")).iter().enumerate() {
            items.push(build_market_row(
                result,
                &section,
                index + 1,
                params,
                title.clone(),
            ));
        }
    }

    let markets = record(response.get("markets"));
    for section in OVERVIEW_SECTION_KEYS {
        for (index, item) in array(markets.get(section)).iter().enumerate() {
            items.push(build_market_row(
                item,
                section,
                index + 1,
                params,
                Value::Null,
            ));
        }
    }

    for (index, item) in array(response.get("news_results")).iter().enumerate() {
        items.push(build_news_row(item, index + 1, params));
    }
    items
}

pub fn build_markets_result_counts(response: &Value) -> Map<String, Value> {
    let markets = record(response.get("markets"));
    let trend_groups = array(response.get("market_trends"));
    let mut counts = Map::new();
    let mut market_rows = 0usize;
    let mut trend_rows = 0usize;

    for section in OVERVIEW_SECTION_KEYS {
        let count = array(markets.get(section)).len();
        market_rows += count;
        counts.insert(section.to_owned(), json!(count));
    }
    for group in trend_groups {
        trend_rows += array(record(Some(group)).get("results")).len();
    }
    market_rows += trend_rows;
    counts.insert("market_rows".to_owned(), json!(market_rows));
    counts.insert("trend_groups".to_owned(), json!(trend_groups.len()));
    counts.insert("trend_rows".to_owned(), json!(trend_rows));
    counts.insert(
        "news_results".to_owned(),
        json!(array(response.get("news_results")).len()),
    );
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_overview_rows_in_source_section_order() {
        let response = json!({
            "markets": {
                "us": [{
                    "stock": ".DJI:INDEXDJX",
                    "name": "Dow Jones Industrial Average",
                    "symbol": ".DJI",
                    "exchange": "INDEXDJX",
                    "price": 42515.09,
                    "price_movement": { "direction": "Down", "value": -125.69, "percentage": -0.3 }
                }],
                "currencies": [{
                    "stock": "EUR-USD",
                    "name": "EUR / USD",
                    "price": "1,045.7",
                    "from_currency": "EUR",
                    "to_currency": "USD",
                    "price_movement": { "direction": "Up", "value": "0.0012", "percentage": "0.11" }
                }]
            }
        });
        let items = build_markets_dataset_items(
            &response,
            &json!({ "hl": "en", "gl": "us" })
                .as_object()
                .unwrap()
                .clone(),
        );
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["section"], "us");
        assert_eq!(items[0]["position"], 1);
        assert_eq!(items[0]["stock"], ".DJI:INDEXDJX");
        assert_eq!(items[0]["price_movement_direction"], "Down");
        assert_eq!(items[1]["section"], "currencies");
        assert_eq!(items[1]["price"], 1045.7);
        assert_eq!(items[1]["from_currency"], "EUR");
        assert!(items[0].get("raw_market_row").is_none());
        assert!(items[0].get("result_counts").is_none());
    }

    #[test]
    fn maps_trend_groups_and_news_rows() {
        let response = json!({
            "market_trends": [
                { "title": "Americas", "results": [{
                    "stock": "AAPL:NASDAQ",
                    "link": "https://www.google.com/finance/quote/AAPL:NASDAQ",
                    "name": "Apple Inc",
                    "symbol": "AAPL",
                    "exchange": "NASDAQ",
                    "extracted_price": 189.98,
                    "currency": "USD",
                    "price_movement": { "direction": "Up", "value": 3.25, "percentage": 1.74 }
                }] },
                { "results": [{ "price": "broken" }] }
            ],
            "news_results": [{
                "title": "Markets climb",
                "link": "https://example.com/news",
                "source": "Example Finance",
                "date": "2026-05-15 13:30:00",
                "snippet": "Stocks moved higher.",
                "thumbnail": "https://example.com/thumb.jpg"
            }]
        });
        let params = json!({ "trend": "gainers", "hl": "en", "gl": "us" });
        let items = build_markets_dataset_items(&response, params.as_object().unwrap());
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["section"], "Americas");
        assert_eq!(items[0]["trend_group"], "Americas");
        assert_eq!(items[0]["trend"], "gainers");
        assert_eq!(items[0]["price"], 189.98);
        assert_eq!(items[1]["section"], "gainers");
        assert_eq!(items[1]["trend_group"], Value::Null);
        assert_eq!(items[2]["item_type"], "news_result");
        assert_eq!(items[2]["section"], "finance-news");
        assert_eq!(items[2]["title"], "Markets climb");
        assert_eq!(items[2]["position"], 1);
        assert!(items[2].get("raw_news_result").is_none());
    }

    #[test]
    fn counts_overview_trend_and_news_rows() {
        assert_eq!(
            build_markets_result_counts(&json!({
                "markets": { "us": [{ "stock": "SPY:NYSEARCA" }], "futures": [{ "stock": "YMW00:CBOT" }] },
                "market_trends": [{ "results": [{}, {}] }],
                "news_results": [{ "title": "News" }]
            })),
            json!({
                "market_rows": 4,
                "trend_groups": 1,
                "trend_rows": 2,
                "news_results": 1,
                "us": 1,
                "europe": 0,
                "asia": 0,
                "currencies": 0,
                "crypto": 0,
                "futures": 1
            })
            .as_object()
            .unwrap()
            .clone()
        );
    }

    #[test]
    fn invalid_record_fields_fall_back_to_null_and_missing_arrays_are_empty() {
        let items = build_markets_dataset_items(
            &json!({ "markets": { "us": [null, { "price": "not-a-number", "price_movement": [] }] } }),
            &Map::new(),
        );
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["stock"], Value::Null);
        assert_eq!(items[1]["price"], Value::Null);
        assert_eq!(items[1]["price_movement_value"], Value::Null);
        assert!(build_markets_dataset_items(&json!({}), &Map::new()).is_empty());
    }
}
