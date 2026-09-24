use anyhow::{bail, Result};
use serde_json::{Map, Value};

pub const MARKET_TRENDS: [&str; 7] = [
    "indexes",
    "most-active",
    "gainers",
    "losers",
    "climate-leaders",
    "cryptocurrencies",
    "currencies",
];

pub const INDEX_MARKETS: [&str; 3] = ["americas", "europe-middle-east-africa", "asia-pacific"];

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };

    match value {
        Value::Null => Ok(None),
        Value::String(value) if value.is_empty() => Ok(None),
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            if trimmed.chars().count() > max_length {
                bail!("{field} must be {max_length} characters or fewer");
            }
            Ok(Some(trimmed.to_owned()))
        }
        _ => bail!("{field} must be a string"),
    }
}

fn clean_language_code(input: &Value) -> Result<Option<String>> {
    let Some(language) = clean_string(input.get("hl"), "hl", 10)? else {
        return Ok(None);
    };
    let normalized = language.to_ascii_lowercase();
    let valid = match normalized.split_once('-') {
        Some((language, region)) => {
            language.len() == 2
                && region.len() == 2
                && language.bytes().all(|byte| byte.is_ascii_lowercase())
                && region.bytes().all(|byte| byte.is_ascii_lowercase())
        }
        None => normalized.len() == 2 && normalized.bytes().all(|byte| byte.is_ascii_lowercase()),
    };
    if !valid {
        bail!("hl must be a valid language code such as en, de, or zh-cn");
    }
    Ok(Some(normalized))
}

fn clean_country_code(input: &Value) -> Result<Option<String>> {
    let Some(country) = clean_string(input.get("gl"), "gl", 10)? else {
        return Ok(None);
    };
    if country.len() != 2 || !country.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        bail!("gl must be a two-letter country code");
    }
    Ok(Some(country.to_ascii_lowercase()))
}

fn clean_choice(
    input: &Value,
    field: &str,
    max_length: usize,
    choices: &[&str],
) -> Result<Option<String>> {
    let Some(value) = clean_string(input.get(field), field, max_length)? else {
        return Ok(None);
    };
    let normalized = value.to_ascii_lowercase();
    if !choices.contains(&normalized.as_str()) {
        bail!("{field} must be one of: {}", choices.join(", "));
    }
    Ok(Some(normalized))
}

pub fn build_google_finance_markets_params(input: &Value) -> Result<Map<String, Value>> {
    let mut params = Map::new();
    let trend = clean_choice(input, "trend", 40, &MARKET_TRENDS)?;
    let index_market = clean_choice(input, "index_market", 40, &INDEX_MARKETS)?;
    let language = clean_language_code(input)?;
    let country = clean_country_code(input)?;

    if let Some(trend) = trend.as_ref() {
        params.insert("trend".to_owned(), Value::String(trend.clone()));
    }
    if trend.as_deref() == Some("indexes") {
        if let Some(index_market) = index_market {
            params.insert("index_market".to_owned(), Value::String(index_market));
        }
    }
    if let Some(language) = language {
        params.insert("hl".to_owned(), Value::String(language));
    }
    if let Some(country) = country {
        params.insert("gl".to_owned(), Value::String(country));
    }
    Ok(params)
}

pub fn describe_google_finance_markets_request(params: &Map<String, Value>) -> String {
    let request_type = params
        .get("trend")
        .and_then(Value::as_str)
        .map(|trend| format!("trend={trend}"))
        .unwrap_or_else(|| "markets overview".to_owned());
    let filters = ["index_market", "hl", "gl"]
        .iter()
        .filter_map(|field| {
            params
                .get(*field)
                .and_then(Value::as_str)
                .map(|value| format!("{field}={value}"))
        })
        .collect::<Vec<_>>();
    let filter_suffix = if filters.is_empty() {
        String::new()
    } else {
        format!(" ({})", filters.join(", "))
    };
    format!("{request_type}{filter_suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preserves_schema_prefill_and_builds_default_overview_params() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema.pointer("/properties/hl/default"), Some(&json!("en")));
        assert_eq!(schema.pointer("/properties/gl/default"), Some(&json!("us")));
        assert_eq!(
            build_google_finance_markets_params(&json!({ "hl": "EN", "gl": "US" })).unwrap(),
            json!({ "hl": "en", "gl": "us" })
                .as_object()
                .unwrap()
                .clone()
        );
        assert!(build_google_finance_markets_params(&json!({}))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn normalizes_trend_and_regional_index_filter() {
        let params = build_google_finance_markets_params(&json!({
            "trend": " INDEXES ",
            "index_market": " Americas ",
            "hl": "ZH-CN",
            "gl": "US"
        }))
        .unwrap();
        assert_eq!(
            params,
            json!({
                "trend": "indexes",
                "index_market": "americas",
                "hl": "zh-cn",
                "gl": "us"
            })
            .as_object()
            .unwrap()
            .clone()
        );
    }

    #[test]
    fn ignores_index_market_unless_indexes_is_selected() {
        assert_eq!(
            build_google_finance_markets_params(&json!({
                "trend": "gainers",
                "index_market": "americas"
            }))
            .unwrap(),
            json!({ "trend": "gainers" }).as_object().unwrap().clone()
        );
        assert!(
            build_google_finance_markets_params(&json!({ "index_market": "americas" }))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_invalid_values_and_non_string_fields() {
        for (input, message) in [
            (json!({ "trend": "hot-stocks" }), "trend must be one of:"),
            (
                json!({ "index_market": "africa" }),
                "index_market must be one of:",
            ),
            (
                json!({ "hl": "english" }),
                "hl must be a valid language code",
            ),
            (
                json!({ "gl": "usa" }),
                "gl must be a two-letter country code",
            ),
            (json!({ "trend": 3 }), "trend must be a string"),
            (
                json!({ "hl": "longlanguage" }),
                "hl must be 10 characters or fewer",
            ),
        ] {
            assert!(
                build_google_finance_markets_params(&input)
                    .unwrap_err()
                    .to_string()
                    .contains(message),
                "input {input} should mention {message}"
            );
        }
    }

    #[test]
    fn describes_request_type_and_filters_in_stable_order() {
        let params = build_google_finance_markets_params(&json!({
            "trend": "indexes",
            "index_market": "asia-pacific",
            "hl": "en",
            "gl": "us"
        }))
        .unwrap();
        assert_eq!(
            describe_google_finance_markets_request(&params),
            "trend=indexes (index_market=asia-pacific, hl=en, gl=us)"
        );
        assert_eq!(
            describe_google_finance_markets_request(
                &build_google_finance_markets_params(&json!({ "trend": "gainers" })).unwrap()
            ),
            "trend=gainers"
        );
    }
}
