use anyhow::{Result, anyhow};
use serde_json::{Map, Number, Value, json};

const LANGUAGES: [&str; 12] = [
    "english",
    "deutsch",
    "french",
    "spanish",
    "italian",
    "portuguese",
    "dutch",
    "russian",
    "chinese",
    "japanese",
    "arabic",
    "all",
];
const DEFAULT_MAX_RESULTS_PER_QUERY: usize = 20;
const MAX_QUERIES_PER_RUN: usize = 100;
const MAX_RESULTS_PER_QUERY: usize = 100;

#[derive(Debug, PartialEq)]
pub struct StartpageSearchRequest {
    pub query: String,
    pub params: Map<String, Value>,
}

#[derive(Debug, PartialEq)]
pub struct StartpageSearchPlan {
    pub requests: Vec<StartpageSearchRequest>,
    pub max_results_per_query: usize,
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        anyhow::bail!("{field} must be a string");
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        anyhow::bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(value.to_owned()))
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_integer(value: Option<&Value>, field: &str, min: i64, max: i64) -> Result<Option<i64>> {
    let Some(value) = value.filter(|value| !value.is_null() && *value != "") else {
        return Ok(None);
    };
    let Some(number) = value.as_f64() else {
        anyhow::bail!("{field} must be an integer");
    };
    if !number.is_finite() || number.fract() != 0.0 {
        anyhow::bail!("{field} must be an integer");
    }
    if number < min as f64 || number > max as f64 {
        anyhow::bail!("{field} must be between {min} and {max}");
    }
    Ok(Some(number as i64))
}

fn clean_boolean(value: Option<&Value>, field: &str) -> Result<Option<bool>> {
    let Some(value) = value.filter(|value| !value.is_null() && *value != "") else {
        return Ok(None);
    };
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| anyhow!("{field} must be a boolean"))
}

fn clean_language(value: Option<&Value>) -> Result<Option<String>> {
    let Some(language) = clean_string(value, "language", 50)? else {
        return Ok(None);
    };
    let language = language.to_lowercase();
    if !LANGUAGES.contains(&language.as_str()) {
        anyhow::bail!("language must be one of: {}", LANGUAGES.join(", "));
    }
    Ok(Some(language))
}

fn clean_queries(value: Option<&Value>) -> Result<Vec<&Map<String, Value>>> {
    let Some(value) = value.and_then(Value::as_array) else {
        anyhow::bail!("queries must be an array");
    };
    if value.is_empty() {
        anyhow::bail!("queries must include at least one search query");
    }
    if value.len() > MAX_QUERIES_PER_RUN {
        anyhow::bail!("queries cannot include more than {MAX_QUERIES_PER_RUN} items");
    }
    value
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_object()
                .ok_or_else(|| anyhow!("queries[{index}] must be an object"))
        })
        .collect()
}

pub fn build_startpage_search_plan(input: &Value) -> Result<StartpageSearchPlan> {
    let queries = clean_queries(input.get("queries"))?;
    let max_results_per_query = clean_integer(
        input.get("max_results_per_query"),
        "max_results_per_query",
        1,
        MAX_RESULTS_PER_QUERY as i64,
    )?
    .map(|value| value as usize)
    .unwrap_or(DEFAULT_MAX_RESULTS_PER_QUERY);

    let requests = queries
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let query =
                clean_required_string(item.get("query"), &format!("queries[{index}].query"), 500)?;
            let language = clean_language(item.get("language"))?;
            let page = clean_integer(item.get("page"), &format!("queries[{index}].page"), 0, 10)?;
            let safe_search = clean_boolean(
                item.get("safe_search"),
                &format!("queries[{index}].safe_search"),
            )?;

            let mut params = Map::new();
            params.insert("query".into(), Value::String(query.clone()));
            if let Some(language) = language {
                params.insert("language".into(), Value::String(language));
            }
            if let Some(page) = page {
                params.insert("page".into(), json!(page));
            }
            if let Some(safe_search) = safe_search {
                params.insert(
                    "safe_search".into(),
                    Value::Number(Number::from(u8::from(safe_search))),
                );
            }
            Ok(StartpageSearchRequest { query, params })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(StartpageSearchPlan {
        requests,
        max_results_per_query,
    })
}

pub fn describe_startpage_search_plan(plan: &StartpageSearchPlan) -> String {
    let sample = plan
        .requests
        .iter()
        .take(3)
        .map(|request| format!("\"{}\"", request.query))
        .collect::<Vec<_>>();
    let suffix = if plan.requests.len() > sample.len() {
        format!(" and {} more", plan.requests.len() - sample.len())
    } else {
        String::new()
    };
    format!(
        "{} query request(s): {}{}",
        plan.requests.len(),
        sample.join(", "),
        suffix
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_batch_startpage_request_params() {
        let plan = build_startpage_search_plan(&json!({
            "queries": [
                {"query": " privacy tools ", "language": "ENGLISH", "page": 0, "safe_search": true},
                {"query": " private search ", "safe_search": false}
            ],
            "max_results_per_query": 10
        }))
        .unwrap();

        assert_eq!(plan.max_results_per_query, 10);
        assert_eq!(plan.requests[0].query, "privacy tools");
        assert_eq!(
            plan.requests[0].params,
            json!({
                "query": "privacy tools", "language": "english", "page": 0, "safe_search": 1
            })
            .as_object()
            .unwrap()
            .clone()
        );
        assert_eq!(
            plan.requests[1].params,
            json!({
                "query": "private search", "safe_search": 0
            })
            .as_object()
            .unwrap()
            .clone()
        );
    }

    #[test]
    fn omits_optional_params_and_uses_default_result_limit() {
        let plan =
            build_startpage_search_plan(&json!({"queries": [{"query": "privacy tools"}]})).unwrap();
        assert_eq!(plan.max_results_per_query, DEFAULT_MAX_RESULTS_PER_QUERY);
        assert_eq!(
            plan.requests[0].params,
            json!({"query": "privacy tools"})
                .as_object()
                .unwrap()
                .clone()
        );
    }

    #[test]
    fn rejects_invalid_queries_and_controls() {
        let invalid_inputs = [
            (json!({}), "queries must be an array"),
            (json!({"queries": []}), "at least one"),
            (
                json!({"queries": ["privacy"]}),
                "queries[0] must be an object",
            ),
            (
                json!({"queries": [{"query": "   "}]}),
                "queries[0].query is required",
            ),
            (
                json!({"queries": [{"query": "privacy", "language": "klingon"}]}),
                "language must be one of",
            ),
            (
                json!({"queries": [{"query": "privacy", "page": 11}]}),
                "page must be between 0 and 10",
            ),
            (
                json!({"queries": [{"query": "privacy", "safe_search": "false"}]}),
                "safe_search must be a boolean",
            ),
            (
                json!({"queries": [{"query": "privacy"}], "max_results_per_query": 0}),
                "max_results_per_query must be between 1 and 100",
            ),
            (
                json!({"queries": [{"query": "privacy", "page": 1.5}]}),
                "page must be an integer",
            ),
        ];
        for (input, expected) in invalid_inputs {
            assert!(
                build_startpage_search_plan(&input)
                    .unwrap_err()
                    .to_string()
                    .contains(expected)
            );
        }
    }

    #[test]
    fn describes_batch_requests() {
        let plan = build_startpage_search_plan(&json!({
            "queries": [{"query": "one"}, {"query": "two"}, {"query": "three"}, {"query": "four"}]
        }))
        .unwrap();
        assert_eq!(
            describe_startpage_search_plan(&plan),
            "4 query request(s): \"one\", \"two\", \"three\" and 1 more"
        );
    }
}
