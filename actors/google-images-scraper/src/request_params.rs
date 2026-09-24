use serde_json::{json, Map, Number, Value};

const MAX_QUERIES_PER_RUN: usize = 50;
const IMAGE_SIZE_VALUES: &[&str] = &["large", "medium", "icon"];
const IMAGE_TYPE_VALUES: &[&str] = &["photo", "clipart", "lineart", "gif", "face"];
const IMAGE_COLOR_VALUES: &[&str] = &[
    "color", "gray", "trans", "red", "orange", "yellow", "green", "teal", "blue", "purple", "pink",
    "white", "black", "brown",
];
const ASPECT_RATIO_VALUES: &[&str] = &["tall", "square", "wide"];
const SAFE_VALUES: &[&str] = &["active", "off"];

pub fn build_google_images_params(input: &Value) -> Result<Value, String> {
    let mut params = Map::new();
    let q = clean_required_string(input.get("q"), "q", 500)?;
    params.insert("q".into(), json!(q));

    if let Some(page) = clean_integer(input.get("page"), "page", 1)? {
        params.insert("page".into(), Value::Number(page));
    }
    if let Some(hl) = clean_two_letter_code(input.get("hl"), "hl")? {
        params.insert("hl".into(), json!(hl));
    }
    if let Some(gl) = clean_two_letter_code(input.get("gl"), "gl")? {
        params.insert("gl".into(), json!(gl));
    }
    if let Some(imgsz) = clean_enum(input.get("imgsz"), "imgsz", IMAGE_SIZE_VALUES)? {
        params.insert("imgsz".into(), json!(imgsz));
    }
    if let Some(imgtype) = clean_enum(input.get("imgtype"), "imgtype", IMAGE_TYPE_VALUES)? {
        params.insert("imgtype".into(), json!(imgtype));
    }
    if let Some(imgcolor) = clean_enum(input.get("imgcolor"), "imgcolor", IMAGE_COLOR_VALUES)? {
        params.insert("imgcolor".into(), json!(imgcolor));
    }
    if let Some(imgar) = clean_enum(input.get("imgar"), "imgar", ASPECT_RATIO_VALUES)? {
        params.insert("imgar".into(), json!(imgar));
    }
    if let Some(tbs) = clean_string(input.get("tbs"), "tbs", 500)? {
        params.insert("tbs".into(), json!(tbs));
    }
    if let Some(safe) = clean_enum(input.get("safe"), "safe", SAFE_VALUES)? {
        params.insert("safe".into(), json!(safe));
    }

    Ok(Value::Object(params))
}

pub fn build_google_images_param_list(input: &Value) -> Result<Vec<Value>, String> {
    get_google_images_queries(input)?
        .into_iter()
        .map(|query| {
            let mut query_input = input.as_object().cloned().unwrap_or_default();
            query_input.insert("q".into(), json!(query));
            query_input.remove("queries");
            build_google_images_params(&Value::Object(query_input))
        })
        .collect()
}

fn get_google_images_queries(input: &Value) -> Result<Vec<String>, String> {
    let mut raw_queries = Vec::new();
    if let Some(q) = input.get("q") {
        if !q.is_null() && q != "" {
            raw_queries.push(q);
        }
    }

    if let Some(raw_queries_value) = input.get("queries") {
        if !raw_queries_value.is_null() && raw_queries_value != "" {
            let Some(queries) = raw_queries_value.as_array() else {
                return Err("queries must be an array".into());
            };
            if queries.len() > MAX_QUERIES_PER_RUN {
                return Err(format!(
                    "queries must contain {MAX_QUERIES_PER_RUN} items or fewer"
                ));
            }
            raw_queries.extend(queries.iter());
        }
    }

    let mut seen = std::collections::HashSet::new();
    let mut queries = Vec::new();
    for raw_query in raw_queries {
        let Some(query) = clean_string(Some(raw_query), "q", 500)? else {
            continue;
        };
        if seen.insert(query.clone()) {
            queries.push(query);
        }
    }

    if queries.is_empty() {
        return Err("q is required".into());
    }
    if queries.len() > MAX_QUERIES_PER_RUN {
        return Err(format!(
            "queries must contain {MAX_QUERIES_PER_RUN} items or fewer"
        ));
    }
    Ok(queries)
}

fn clean_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value == "" {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        return Err(format!("{field} must be a string"));
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        return Err(format!("{field} must be {max_length} characters or fewer"));
    }
    Ok(Some(value.to_owned()))
}

fn clean_required_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<String, String> {
    clean_string(value, field, max_length)?.ok_or_else(|| format!("{field} is required"))
}

fn clean_two_letter_code(value: Option<&Value>, field: &str) -> Result<Option<String>, String> {
    let Some(value) = clean_string(value, field, 2)? else {
        return Ok(None);
    };
    if value.len() != 2 || !value.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(format!("{field} must be a two-letter code"));
    }
    Ok(Some(value.to_ascii_lowercase()))
}

fn clean_integer(value: Option<&Value>, field: &str, min: i64) -> Result<Option<Number>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value == "" {
        return Ok(None);
    }
    let Some(number) = value.as_f64() else {
        return Err(format!("{field} must be an integer"));
    };
    if !number.is_finite() || number.fract() != 0.0 {
        return Err(format!("{field} must be an integer"));
    }
    if number < min as f64 {
        return Err(format!("{field} must be greater than or equal to {min}"));
    }
    let number = if number <= u64::MAX as f64 {
        Number::from(number as u64)
    } else {
        Number::from_f64(number).ok_or_else(|| format!("{field} must be an integer"))?
    };
    Ok(Some(number))
}

fn clean_enum(
    value: Option<&Value>,
    field: &str,
    allowed_values: &[&str],
) -> Result<Option<String>, String> {
    let Some(value) = clean_string(value, field, 100)? else {
        return Ok(None);
    };
    let normalized = value.to_ascii_lowercase();
    if !allowed_values.contains(&normalized.as_str()) {
        return Err(format!(
            "{field} must be one of: {}",
            allowed_values.join(", ")
        ));
    }
    Ok(Some(normalized))
}

pub fn describe_google_images_request(params: &Value) -> String {
    let filters = ["imgsz", "imgtype", "imgcolor", "imgar", "tbs", "safe"]
        .into_iter()
        .filter_map(|field| {
            params
                .get(field)
                .filter(|value| !value.is_null())
                .map(|value| format!("{field}={}", js_string(value)))
        })
        .collect::<Vec<_>>();
    let page = params
        .get("page")
        .and_then(Value::as_f64)
        .map(|page| format!(" page {}", js_number_string(page)))
        .unwrap_or_default();
    let filter_suffix = if filters.is_empty() {
        String::new()
    } else {
        format!(" ({})", filters.join(", "))
    };
    let query = params
        .get("q")
        .and_then(Value::as_str)
        .unwrap_or("unknown query");
    format!("query \"{query}\"{page}{filter_suffix}")
}

fn js_string(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn js_number_string(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn builds_a_filtered_google_images_request() {
        assert_eq!(
            build_google_images_params(&json!({
                "q": " coffee product photography ",
                "page": 2,
                "hl": "EN",
                "gl": "US",
                "imgsz": "LARGE",
                "imgtype": "photo",
                "imgcolor": "white",
                "imgar": "wide",
                "tbs": " qdr:w ",
                "safe": "ACTIVE"
            }))
            .unwrap(),
            json!({
                "q": "coffee product photography",
                "page": 2,
                "hl": "en",
                "gl": "us",
                "imgsz": "large",
                "imgtype": "photo",
                "imgcolor": "white",
                "imgar": "wide",
                "tbs": "qdr:w",
                "safe": "active"
            })
        );
    }

    #[test]
    fn deduplicates_queries_after_trimming_and_preserves_batch_order() {
        assert_eq!(
            build_google_images_param_list(&json!({
                "q": "coffee product photography",
                "queries": [" coffee product photography ", "espresso machine"],
                "gl": "US",
                "safe": "ACTIVE"
            }))
            .unwrap(),
            vec![
                json!({"q":"coffee product photography", "gl":"us", "safe":"active"}),
                json!({"q":"espresso machine", "gl":"us", "safe":"active"})
            ]
        );
    }

    #[test]
    fn rejects_invalid_query_batch_shapes_and_empty_queries() {
        assert_eq!(
            build_google_images_param_list(&json!({"queries": "coffee"}))
                .unwrap_err()
                .to_string(),
            "queries must be an array"
        );
        assert_eq!(
            build_google_images_param_list(&json!({"queries": ["coffee", 4]}))
                .unwrap_err()
                .to_string(),
            "q must be a string"
        );
        assert_eq!(
            build_google_images_param_list(&json!({"queries": [" "]}))
                .unwrap_err()
                .to_string(),
            "q is required"
        );
        assert_eq!(
            build_google_images_param_list(&json!({
                "q": "extra query",
                "queries": (0..50).map(|index| format!("query {index}")).collect::<Vec<_>>()
            }))
            .unwrap_err()
            .to_string(),
            "queries must contain 50 items or fewer"
        );
    }

    #[test]
    fn validates_page_localization_and_filter_values() {
        for (input, message) in [
            (
                json!({"q":"coffee", "page":0}),
                "page must be greater than or equal to 1",
            ),
            (
                json!({"q":"coffee", "hl":"eng"}),
                "hl must be 2 characters or fewer",
            ),
            (json!({"q":"coffee", "gl":123}), "gl must be a string"),
            (
                json!({"q":"coffee", "imgsz":"huge"}),
                "imgsz must be one of: large, medium, icon",
            ),
            (
                json!({"q":"coffee", "safe":"moderate"}),
                "safe must be one of: active, off",
            ),
        ] {
            assert_eq!(
                build_google_images_params(&input).unwrap_err().to_string(),
                message
            );
        }
    }

    #[test]
    fn describes_query_page_and_filters_for_logs() {
        assert_eq!(
            describe_google_images_request(&json!({
                "q":"coffee", "page":2, "imgsz":"large", "safe":"active"
            })),
            "query \"coffee\" page 2 (imgsz=large, safe=active)"
        );
    }

    #[test]
    fn input_schema_keeps_batch_prefill_and_legacy_query() {
        let source = include_str!("../.actor/input_schema.json");
        let schema: Value = serde_json::from_str(source).unwrap();
        assert_eq!(
            schema["properties"]["queries"]["prefill"],
            json!(["coffee"])
        );
        assert!(schema["properties"]["q"]["prefill"].is_null());
        assert_eq!(schema["properties"]["queries"]["maxItems"], 50);
        assert_eq!(schema["properties"]["page"]["default"], 1);
        assert_eq!(schema["properties"]["hl"]["default"], "en");
        assert_eq!(schema["properties"]["gl"]["default"], "us");
        assert_eq!(schema["properties"]["safe"]["default"], "active");
        assert!(source.find("\"queries\"").unwrap() < source.find("\"q\"").unwrap());
    }
}
