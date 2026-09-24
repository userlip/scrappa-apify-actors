use serde_json::{Number, Value};
use std::collections::HashSet;

pub const MAX_ITEM_IDS_PER_RUN: usize = 50;
pub const VALID_COUNTRIES: &[&str] = &[
    "FR", "DE", "ES", "IT", "NL", "BE", "AT", "PL", "CZ", "LT", "LU", "SK", "HU", "RO", "PT", "SE",
    "DK", "FI", "US",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VintedItemDetailsRequest {
    pub item_id: String,
    pub country: String,
    pub index: usize,
}

impl VintedItemDetailsRequest {
    pub fn describe(&self) -> String {
        format!("{} in {}", self.item_id, self.country)
    }
}

pub fn build_vinted_item_details_requests(
    input: &Value,
) -> Result<Vec<VintedItemDetailsRequest>, String> {
    let Some(input) = input.as_object() else {
        return Err("Provide at least one Vinted item ID using item_id or item_ids".to_owned());
    };
    let country = clean_country(input.get("country"))?;
    let mut item_ids = Vec::new();
    let mut seen = HashSet::new();

    if let Some(item_id) = input.get("item_id") {
        add_item_id(&mut item_ids, &mut seen, item_id, "item_id")?;
    }

    if let Some(item_ids_input) = input.get("item_ids") {
        let ids = item_ids_input
            .as_array()
            .ok_or_else(|| "item_ids must be an array of numeric Vinted item IDs".to_owned())?;
        for (index, item_id) in ids.iter().enumerate() {
            add_item_id(
                &mut item_ids,
                &mut seen,
                item_id,
                &format!("item_ids[{index}]"),
            )?;
        }
    }

    if item_ids.is_empty() {
        return Err("Provide at least one Vinted item ID using item_id or item_ids".to_owned());
    }
    if item_ids.len() > MAX_ITEM_IDS_PER_RUN {
        return Err(format!(
            "item_ids cannot include more than {MAX_ITEM_IDS_PER_RUN} IDs per run"
        ));
    }

    Ok(item_ids
        .into_iter()
        .enumerate()
        .map(|(index, item_id)| VintedItemDetailsRequest {
            item_id,
            country: country.clone(),
            index,
        })
        .collect())
}

fn clean_country(value: Option<&Value>) -> Result<String, String> {
    let Some(country) = clean_string(value, "country", 2)? else {
        return Ok("FR".to_owned());
    };
    let country = country.to_ascii_uppercase();
    if !VALID_COUNTRIES.contains(&country.as_str()) {
        return Err(format!(
            "country must be one of: {}",
            VALID_COUNTRIES.join(", ")
        ));
    }
    Ok(country)
}

fn clean_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_length {
        return Err(format!("{field} must be {max_length} characters or fewer"));
    }
    Ok(Some(value.to_owned()))
}

fn add_item_id(
    item_ids: &mut Vec<String>,
    seen: &mut HashSet<String>,
    value: &Value,
    field: &str,
) -> Result<(), String> {
    let Some(item_id) = clean_item_id(value, field)? else {
        return Ok(());
    };
    if seen.insert(item_id.clone()) {
        item_ids.push(item_id);
    }
    Ok(())
}

fn clean_item_id(value: &Value, field: &str) -> Result<Option<String>, String> {
    if value.is_null() || value.as_str().is_some_and(|value| value.is_empty()) {
        return Ok(None);
    }

    let item_id = match value {
        Value::Number(number) => clean_integer_item_id(number, field)?,
        _ => clean_string(Some(value), field, 32)?.unwrap_or_default(),
    };
    if item_id.is_empty() {
        return Ok(None);
    }
    if !item_id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{field} must be a numeric Vinted item ID"));
    }
    Ok(Some(item_id))
}

fn clean_integer_item_id(value: &Number, field: &str) -> Result<String, String> {
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

    if let Some(integer) = value.as_u64() {
        if integer > MAX_SAFE_INTEGER as u64 {
            return Err(format!(
                "{field} must be a numeric Vinted item ID string or a safe integer"
            ));
        }
        return Ok(integer.to_string());
    }
    if let Some(integer) = value.as_i64() {
        if integer < 0 {
            return Err(format!(
                "{field} must be a numeric Vinted item ID string or a safe integer"
            ));
        }
        return Ok(integer.to_string());
    }
    let Some(number) = value.as_f64() else {
        return Err(format!("{field} must be a numeric Vinted item ID"));
    };
    if !number.is_finite() || number.fract() != 0.0 || number < 0.0 || number > MAX_SAFE_INTEGER {
        return Err(format!(
            "{field} must be a numeric Vinted item ID string or a safe integer"
        ));
    }
    if number == 0.0 {
        return Ok("0".to_owned());
    }
    Ok(format!("{number:.0}"))
}

#[cfg(test)]
mod tests {
    use super::{build_vinted_item_details_requests, MAX_ITEM_IDS_PER_RUN, VALID_COUNTRIES};
    use serde_json::json;

    #[test]
    fn builds_single_item_request_and_uses_country_default() {
        let requests = build_vinted_item_details_requests(&json!({
            "item_id": " 1234567890 ",
            "country": "de"
        }))
        .unwrap();

        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].item_id, "1234567890");
        assert_eq!(requests[0].country, "DE");
        assert_eq!(requests[0].index, 0);
        assert_eq!(requests[0].describe(), "1234567890 in DE");
        let requests = build_vinted_item_details_requests(&json!({"item_id": "1"})).unwrap();
        assert_eq!(requests[0].country, "FR");
    }

    #[test]
    fn deduplicates_ids_and_accepts_safe_integer_values() {
        let requests = build_vinted_item_details_requests(&json!({
            "item_id": 123,
            "item_ids": ["123", "456", 789.0]
        }))
        .unwrap();

        assert_eq!(
            requests
                .iter()
                .map(|request| request.item_id.as_str())
                .collect::<Vec<_>>(),
            vec!["123", "456", "789"]
        );
        assert_eq!(
            requests
                .iter()
                .map(|request| request.index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(requests.iter().all(|request| request.country == "FR"));
    }

    #[test]
    fn validates_ids_countries_and_batch_size() {
        assert!(build_vinted_item_details_requests(&json!({}))
            .unwrap_err()
            .contains("Provide at least one"));
        assert!(
            build_vinted_item_details_requests(&json!({"item_id": "abc"}))
                .unwrap_err()
                .contains("numeric Vinted item ID")
        );
        assert!(
            build_vinted_item_details_requests(&json!({"item_id": 9007199254740992_u64}))
                .unwrap_err()
                .contains("safe integer")
        );
        assert_eq!(
            build_vinted_item_details_requests(&json!({"item_id": -0.0})).unwrap()[0].item_id,
            "0"
        );
        assert!(
            build_vinted_item_details_requests(&json!({"item_ids": "123"}))
                .unwrap_err()
                .contains("item_ids must be an array")
        );
        assert!(
            build_vinted_item_details_requests(&json!({"item_id": "1", "country": "GB"}))
                .unwrap_err()
                .contains("country must be one of")
        );

        let ids = (1..=MAX_ITEM_IDS_PER_RUN + 1)
            .map(|id| id.to_string())
            .collect::<Vec<_>>();
        assert!(
            build_vinted_item_details_requests(&json!({ "item_ids": ids }))
                .unwrap_err()
                .contains("more than 50")
        );
        assert_eq!(VALID_COUNTRIES.len(), 19);
    }

    #[test]
    fn input_schema_keeps_prefill_and_marketplace_fields() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let actor: serde_json::Value =
            serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();

        assert_eq!(schema["properties"]["item_id"]["prefill"], "1234567890");
        assert_eq!(schema["properties"]["item_id"]["pattern"], "^\\d+$");
        assert_eq!(schema["properties"]["item_ids"]["maxItems"], 50);
        assert_eq!(
            schema["properties"]["item_ids"]["items"]["pattern"],
            "^\\d+$"
        );
        assert_eq!(
            schema["properties"]["country"]["enum"],
            serde_json::json!([
                "FR", "DE", "ES", "IT", "NL", "BE", "AT", "PL", "CZ", "LT", "LU", "SK", "HU", "RO",
                "PT", "SE", "DK", "FI", "US"
            ])
        );
        assert!(
            actor["storages"]["dataset"]["views"]["items"]["transformation"]["fields"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field == "description")
        );
        assert_eq!(
            actor["storages"]["dataset"]["views"]["items"]["display"]["properties"]["description"]
                ["format"],
            "text"
        );
    }
}
