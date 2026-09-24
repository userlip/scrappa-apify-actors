use std::collections::HashSet;

use serde_json::{Number, Value};

pub const MAX_BATCH_AD_IDS: usize = 100;
pub const DISCOVERY_MAX_CANDIDATES: usize = 3;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingRequest {
    pub ad_id: String,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailsPlan {
    pub listings: Vec<ListingRequest>,
}

fn safe_integer_string(number: &Number) -> Option<String> {
    let value = number.as_f64()?;
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > MAX_SAFE_INTEGER as f64
    {
        return None;
    }
    Some((value as u64).to_string())
}

fn normalize_ad_id(value: &Value, field: &str) -> Result<String, String> {
    match value {
        Value::Null => Err(format!("{field} must be a string or safe integer")),
        Value::Number(number) => safe_integer_string(number)
            .ok_or_else(|| format!("{field} must be a safe integer or string")),
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(format!("{field} cannot be blank"));
            }
            if !trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(format!("{field} must contain only digits"));
            }
            Ok(trimmed.to_owned())
        }
        _ => Err(format!("{field} must be a string or safe integer")),
    }
}

pub fn build_details_plan(input: &Value) -> Result<DetailsPlan, String> {
    let ad_id = input.get("ad_id");
    let ad_ids = input.get("ad_ids");
    if ad_ids.is_some_and(|value| !value.is_array()) {
        return Err("ad_ids must be an array".into());
    }

    let mut raw_ids = Vec::new();
    if let Some(ad_id) = ad_id {
        raw_ids.push((ad_id, "ad_id".to_owned()));
    }
    if let Some(ids) = ad_ids.and_then(Value::as_array) {
        for (index, value) in ids.iter().enumerate() {
            raw_ids.push((value, format!("ad_ids[{index}]")));
        }
    }

    let mut seen = HashSet::new();
    let mut unique_ids = Vec::new();
    for (value, field) in raw_ids {
        let id = normalize_ad_id(value, &field)?;
        if seen.insert(id.clone()) {
            unique_ids.push(id);
        }
    }

    if unique_ids.is_empty() {
        return Err("Provide ad_id or at least one ad_ids entry".into());
    }
    if unique_ids.len() > MAX_BATCH_AD_IDS {
        return Err(format!(
            "A maximum of {MAX_BATCH_AD_IDS} unique ad IDs is allowed"
        ));
    }

    Ok(DetailsPlan {
        listings: unique_ids
            .into_iter()
            .enumerate()
            .map(|(index, ad_id)| ListingRequest { ad_id, index })
            .collect(),
    })
}

pub fn get_discovery_query(input: &Value) -> Result<Option<String>, String> {
    if let Some(query) = input.get("query") {
        match query.as_str() {
            Some(query) if !query.trim().is_empty() => {}
            _ => return Err("query must be a non-empty string".into()),
        }
    }

    if input.get("ad_id").is_some() || input.get("ad_ids").is_some() {
        build_details_plan(input)?;
        return Ok(None);
    }

    Ok(input
        .get("query")
        .and_then(Value::as_str)
        .map(str::trim)
        .map(str::to_owned))
}

fn discovered_ad_id(value: &Value) -> Option<String> {
    match value {
        Value::String(value)
            if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            Some(value.clone())
        }
        Value::Number(number) => safe_integer_string(number),
        _ => None,
    }
}

pub fn plan_discovered_listings(response: &Value) -> Result<DetailsPlan, String> {
    let listings = response
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "Search response did not contain a listing array".to_owned())?;

    let mut seen = HashSet::new();
    let ids = listings
        .iter()
        .filter_map(|listing| listing.get("id"))
        .filter_map(discovered_ad_id)
        .filter(|id| seen.insert(id.clone()))
        .take(DISCOVERY_MAX_CANDIDATES)
        .collect::<Vec<_>>();

    let input = serde_json::json!({ "ad_ids": ids });
    build_details_plan(&input)
}

pub fn describe_request(plan: &DetailsPlan) -> String {
    if plan.listings.len() == 1 {
        format!("listing {}", plan.listings[0].ad_id)
    } else {
        format!("{} Kleinanzeigen listings", plan.listings.len())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_details_plan, describe_request, get_discovery_query, plan_discovered_listings,
    };
    use serde_json::{Value, json};

    #[test]
    fn plans_single_batch_and_combined_ids_in_stable_deduplicated_order() {
        let one = build_details_plan(&json!({ "ad_id": " 3451021120 " })).unwrap();
        assert_eq!(one.listings[0].ad_id, "3451021120");
        assert_eq!(describe_request(&one), "listing 3451021120");

        let plan = build_details_plan(&json!({ "ad_id": "1", "ad_ids": ["2", "1", 3] })).unwrap();
        assert_eq!(
            plan.listings
                .iter()
                .map(|item| (item.ad_id.as_str(), item.index))
                .collect::<Vec<_>>(),
            [("1", 0), ("2", 1), ("3", 2)]
        );
        assert_eq!(describe_request(&plan), "3 Kleinanzeigen listings");
    }

    #[test]
    fn validates_values_and_unique_id_limit() {
        assert_eq!(
            build_details_plan(&json!({})).unwrap_err(),
            "Provide ad_id or at least one ad_ids entry"
        );
        assert!(
            build_details_plan(&json!({ "ad_id": " " }))
                .unwrap_err()
                .contains("cannot be blank")
        );
        assert!(
            build_details_plan(&json!({ "ad_id": "12.5" }))
                .unwrap_err()
                .contains("only digits")
        );
        assert!(
            build_details_plan(&json!({ "ad_id": 1.5 }))
                .unwrap_err()
                .contains("safe integer")
        );
        assert_eq!(
            build_details_plan(&json!({ "ad_ids": "1" })).unwrap_err(),
            "ad_ids must be an array"
        );
        assert!(
            build_details_plan(&json!({ "ad_ids": ["1", null] }))
                .unwrap_err()
                .contains("ad_ids[1] must be a string or safe integer")
        );
        assert_eq!(
            build_details_plan(
                &json!({ "ad_ids": (0..100).map(|i| i.to_string()).collect::<Vec<_>>() })
            )
            .unwrap()
            .listings
            .len(),
            100
        );
        assert!(
            build_details_plan(&json!({
                "ad_id": "100",
                "ad_ids": (0..100).map(|i| i.to_string()).collect::<Vec<_>>()
            }))
            .unwrap_err()
            .contains("maximum of 100")
        );
    }

    #[test]
    fn validates_discovery_and_preserves_prefill_priority() {
        let prefill = json!({ "query": "fahrrad" });
        assert_eq!(
            get_discovery_query(&prefill).unwrap().as_deref(),
            Some("fahrrad")
        );
        assert_eq!(
            get_discovery_query(&json!({ "query": "fahrrad", "ad_id": "123" })).unwrap(),
            None
        );
        assert!(
            get_discovery_query(&json!({ "query": 1 }))
                .unwrap_err()
                .contains("non-empty string")
        );
        assert!(
            get_discovery_query(&json!({ "query": "   " }))
                .unwrap_err()
                .contains("non-empty string")
        );
        assert_eq!(
            plan_discovered_listings(&json!({"data": [null, {}, {"id":"bad"}, {"id":"1"}, {"id":"1"}, {"id":2}, {"id":"3"}, {"id":"4"}]}))
                .unwrap()
                .listings
                .iter()
                .map(|item| item.ad_id.as_str())
                .collect::<Vec<_>>(),
            ["1", "2", "3"]
        );
        assert!(
            plan_discovered_listings(&json!({ "data": {} }))
                .unwrap_err()
                .contains("listing array")
        );
        assert!(
            plan_discovered_listings(&json!({ "data": [] }))
                .unwrap_err()
                .contains("Provide ad_id")
        );
    }

    #[test]
    fn actor_input_schema_keeps_the_default_discovery_prefill() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        let prefill = schema["properties"]["query"]["prefill"].as_str().unwrap();
        assert_eq!(prefill, "fahrrad");
        assert_eq!(
            get_discovery_query(&json!({ "query": prefill }))
                .unwrap()
                .as_deref(),
            Some(prefill)
        );
    }
}
