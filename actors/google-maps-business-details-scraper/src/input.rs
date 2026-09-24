use std::collections::HashSet;

use anyhow::{bail, Result};
use serde_json::Value;

pub const MAX_BUSINESS_IDS_PER_RUN: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusinessIdRequest {
    pub input_business_id: String,
    pub business_id: String,
}

pub fn get_business_id_requests(input: Option<&Value>) -> Result<Vec<BusinessIdRequest>> {
    let mut raw_business_ids = Vec::new();

    if let Some(business_id) = input
        .and_then(|input| input.get("business_id"))
        .and_then(Value::as_str)
    {
        raw_business_ids.push(business_id);
    }

    if let Some(business_ids) = input
        .and_then(|input| input.get("business_ids"))
        .and_then(Value::as_array)
    {
        for business_id in business_ids {
            raw_business_ids.push(
                business_id
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("business_ids items must be strings"))?,
            );
        }
    }

    let mut seen = HashSet::new();
    let mut requests = Vec::new();

    for raw_business_id in raw_business_ids {
        let business_id = raw_business_id.trim();
        if business_id.is_empty() || !seen.insert(business_id.to_owned()) {
            continue;
        }

        requests.push(BusinessIdRequest {
            input_business_id: business_id.to_owned(),
            business_id: business_id.to_owned(),
        });
    }

    if requests.len() > MAX_BUSINESS_IDS_PER_RUN {
        bail!("business_ids must contain {MAX_BUSINESS_IDS_PER_RUN} unique items or fewer");
    }

    Ok(requests)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{get_business_id_requests, BusinessIdRequest, MAX_BUSINESS_IDS_PER_RUN};

    #[test]
    fn supports_the_legacy_single_id_and_trims_it() {
        let requests =
            get_business_id_requests(Some(&json!({"business_id": "  maps-id  "}))).unwrap();

        assert_eq!(
            requests,
            vec![BusinessIdRequest {
                input_business_id: "maps-id".to_owned(),
                business_id: "maps-id".to_owned(),
            }]
        );
    }

    #[test]
    fn combines_both_fields_in_order_and_deduplicates_trimmed_ids() {
        let requests = get_business_id_requests(Some(&json!({
            "business_id": " one ",
            "business_ids": ["one", " two ", "", "two"]
        })))
        .unwrap();

        assert_eq!(
            requests
                .iter()
                .map(|request| request.business_id.as_str())
                .collect::<Vec<_>>(),
            ["one", "two"]
        );
    }

    #[test]
    fn ignores_non_string_legacy_id_and_non_array_batch_field() {
        assert!(get_business_id_requests(Some(&json!({
            "business_id": 4,
            "business_ids": "maps-id"
        })))
        .unwrap()
        .is_empty());
    }

    #[test]
    fn caps_the_unique_ids_after_deduplication() {
        let at_limit = (0..MAX_BUSINESS_IDS_PER_RUN)
            .map(|index| format!("business-{index}"))
            .collect::<Vec<_>>();
        assert_eq!(
            get_business_id_requests(Some(&json!({"business_ids": at_limit})))
                .unwrap()
                .len(),
            MAX_BUSINESS_IDS_PER_RUN
        );

        let over_limit = (0..=MAX_BUSINESS_IDS_PER_RUN)
            .map(|index| format!("business-{index}"))
            .collect::<Vec<_>>();
        let error =
            get_business_id_requests(Some(&json!({"business_ids": over_limit}))).unwrap_err();
        assert_eq!(
            error.to_string(),
            "business_ids must contain 10 unique items or fewer"
        );
    }
}
