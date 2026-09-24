use anyhow::{bail, Result};
use serde_json::Value;

pub const MAX_INDICES: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndicesParams {
    pub indices: Option<String>,
    pub hl: String,
    pub gl: String,
}

pub fn normalize_indices(value: Option<&Value>) -> Result<Vec<String>> {
    let raw_values: Vec<&str> = match value {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(csv)) => csv.split(',').collect(),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("indices entries must be strings"))
            })
            .collect::<Result<_>>()?,
        Some(_) => bail!("indices must be a comma-separated string or an array of strings"),
    };

    let mut result = Vec::new();
    for candidate in raw_values {
        let symbol = candidate.trim().to_ascii_uppercase();
        if symbol.is_empty() {
            continue;
        }
        if symbol.len() > 64
            || !symbol
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
        {
            bail!("Invalid index symbol: {}", candidate.trim());
        }
        if !result.contains(&symbol) {
            result.push(symbol);
        }
    }

    if result.len() > MAX_INDICES {
        bail!("A maximum of {MAX_INDICES} indices is allowed per run");
    }
    Ok(result)
}

fn locale(value: Option<&Value>, field: &str, fallback: &str) -> Result<String> {
    let normalized = match value {
        None | Some(Value::Null) => fallback.to_owned(),
        Some(Value::String(value)) => value.trim().to_ascii_lowercase(),
        Some(_) => String::new(),
    };
    let valid = match field {
        "hl" => {
            let mut parts = normalized.split('-');
            let language = parts.next().unwrap_or_default();
            let region = parts.next();
            let has_valid_language = (2..=3).contains(&language.len())
                && language.bytes().all(|byte| byte.is_ascii_lowercase());
            has_valid_language
                && region.is_none_or(|region| {
                    (2..=4).contains(&region.len())
                        && region.bytes().all(|byte| byte.is_ascii_lowercase())
                })
                && normalized.matches('-').count() <= 1
        }
        "gl" => normalized.len() == 2 && normalized.bytes().all(|byte| byte.is_ascii_lowercase()),
        _ => false,
    };
    if !valid {
        let label = if field == "gl" {
            "two-letter country"
        } else {
            "language"
        };
        bail!("{field} must be a valid {label} code");
    }
    Ok(normalized)
}

pub fn build_google_finance_indices_params(input: &Value) -> Result<IndicesParams> {
    let input = if input.is_null() {
        None
    } else {
        Some(
            input
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Input must be an object"))?,
        )
    };
    let indices = normalize_indices(input.and_then(|input| input.get("indices")))?;
    Ok(IndicesParams {
        indices: (!indices.is_empty()).then(|| indices.join(",")),
        hl: locale(input.and_then(|input| input.get("hl")), "hl", "en")?,
        gl: locale(input.and_then(|input| input.get("gl")), "gl", "us")?,
    })
}

pub fn describe_google_finance_indices_request(params: &IndicesParams) -> String {
    format!(
        "{} (hl={}, gl={})",
        params.indices.as_deref().unwrap_or("default indices"),
        params.hl,
        params.gl
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn normalizes_csv_and_array_symbols_in_first_seen_order() {
        assert_eq!(
            normalize_indices(Some(&json!(" .inx, .DJI, .inx "))).unwrap(),
            [".INX", ".DJI"]
        );
        assert_eq!(
            normalize_indices(Some(&json!([" .inx ", ".DJI", ".inx"]))).unwrap(),
            [".INX", ".DJI"]
        );
    }

    #[test]
    fn enforces_symbol_count_type_and_locale_rules() {
        assert!(
            normalize_indices(Some(&json!([".INX", ".DJI", ".IXIC", ".RUT"])))
                .unwrap_err()
                .to_string()
                .contains("maximum")
        );
        assert!(normalize_indices(Some(&json!([".INX", 1]))).is_err());
        assert!(normalize_indices(Some(&json!(true))).is_err());
        assert_eq!(
            build_google_finance_indices_params(&json!({"indices":[".INX"],"hl":"EN","gl":"US"}))
                .unwrap(),
            IndicesParams {
                indices: Some(".INX".to_owned()),
                hl: "en".to_owned(),
                gl: "us".to_owned()
            }
        );
        assert!(build_google_finance_indices_params(&json!({"gl":"usa"})).is_err());
        assert!(build_google_finance_indices_params(&json!({"hl":"en-US-extra"})).is_err());
    }

    #[test]
    fn keeps_the_existing_apify_input_schema_and_prefill_defaults() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let schema: Value = serde_json::from_str(
            &fs::read_to_string(format!("{manifest_dir}/.actor/input_schema.json")).unwrap(),
        )
        .unwrap();
        let indices = &schema["properties"]["indices"];

        assert_eq!(indices["editor"], "json");
        assert!(indices["type"]
            .as_array()
            .unwrap()
            .contains(&json!("string")));
        assert!(indices["type"]
            .as_array()
            .unwrap()
            .contains(&json!("array")));
        assert_eq!(schema["properties"]["hl"]["default"], "en");
        assert_eq!(schema["properties"]["gl"]["default"], "us");
        assert_eq!(schema["properties"]["gl"]["minLength"], 2);
        assert_eq!(schema["properties"]["gl"]["maxLength"], 2);
    }
}
