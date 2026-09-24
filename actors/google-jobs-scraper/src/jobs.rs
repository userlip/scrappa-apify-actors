use serde_json::Value;

pub fn get_jobs(response: &Value) -> Vec<Value> {
    let jobs = response.get("jobs").and_then(Value::as_array);
    if let Some(jobs) = jobs.filter(|jobs| !jobs.is_empty()) {
        return jobs.clone();
    }

    if let Some(jobs_results) = response.get("jobs_results").and_then(Value::as_array) {
        return jobs_results.clone();
    }

    if jobs.is_none() {
        eprintln!("DEBUG: Unexpected Google Jobs response shape: expected \"jobs\" or \"jobs_results\" array.");
    }
    jobs.cloned().unwrap_or_default()
}

pub fn get_next_page_token(response: &Value) -> Option<&str> {
    response
        .get("next_page_token")
        .and_then(Value::as_str)
        .or_else(|| {
            response
                .pointer("/pagination/next_page_token")
                .and_then(Value::as_str)
        })
}

pub fn filter_count(response: &Value) -> usize {
    response
        .get("filters")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prefers_nonempty_jobs_and_falls_back_to_jobs_results() {
        assert_eq!(
            get_jobs(&json!({
                "jobs": [{"title": "Primary"}],
                "jobs_results": [{"title": "Fallback"}]
            })),
            vec![json!({"title": "Primary"})]
        );
        assert_eq!(
            get_jobs(&json!({
                "jobs": [],
                "jobs_results": [{"title": "Fallback"}]
            })),
            vec![json!({"title": "Fallback"})]
        );
    }

    #[test]
    fn returns_empty_jobs_for_empty_or_unexpected_shapes() {
        assert!(get_jobs(&json!({ "jobs": [] })).is_empty());
        assert!(get_jobs(&json!({})).is_empty());
    }

    #[test]
    fn reads_next_page_token_from_both_supported_locations() {
        assert_eq!(
            get_next_page_token(&json!({ "next_page_token": "top-level" })),
            Some("top-level")
        );
        assert_eq!(
            get_next_page_token(&json!({ "pagination": { "next_page_token": "nested" } })),
            Some("nested")
        );
        assert_eq!(get_next_page_token(&json!({})), None);
    }

    #[test]
    fn counts_only_array_filters() {
        assert_eq!(filter_count(&json!({ "filters": [1, 2] })), 2);
        assert_eq!(filter_count(&json!({ "filters": "invalid" })), 0);
    }
}
