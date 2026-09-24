use serde_json::Value;

const MAX_ITEMS_PER_RUN: usize = 100;
const MAX_TEXT_LENGTH: usize = 5_000;
const MAX_LANGUAGE_CODE_LENGTH: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationRequest {
    pub index: usize,
    pub text: String,
    pub source: String,
    pub target: String,
}

pub fn build_translation_requests(input: &Value) -> Result<Vec<TranslationRequest>, String> {
    let uses_batch = input.get("items").is_some_and(|items| !items.is_null());
    let raw_items = match input.get("items") {
        Some(Value::Null) | None => vec![input],
        Some(Value::Array(items)) => {
            if items.is_empty() {
                return Err("items must include at least one translation item".to_owned());
            }
            if items.len() > MAX_ITEMS_PER_RUN {
                return Err(format!(
                    "items cannot include more than {MAX_ITEMS_PER_RUN} translations"
                ));
            }
            items.iter().collect()
        }
        Some(_) => return Err("items must be an array".to_owned()),
    };

    raw_items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let prefix = if uses_batch {
                format!("items[{index}].")
            } else {
                String::new()
            };
            if uses_batch && !item.is_object() {
                return Err(format!("items[{index}] must be an object"));
            }

            let text =
                clean_required_string(item.get("text"), &format!("{prefix}text"), MAX_TEXT_LENGTH)?;
            let source = clean_language(item.get("source"), &format!("{prefix}source"))?;
            let target = clean_language(item.get("target"), &format!("{prefix}target"))?;

            if source == target {
                return Err(format!(
                    "{prefix}target must be different from {prefix}source"
                ));
            }

            Ok(TranslationRequest {
                index,
                text,
                source,
                target,
            })
        })
        .collect()
}

pub fn describe_translation_requests(requests: &[TranslationRequest]) -> String {
    let sample = requests
        .iter()
        .take(3)
        .map(|request| format!("{}->{}", request.source, request.target))
        .collect::<Vec<_>>();
    let suffix = if requests.len() > sample.len() {
        format!(" and {} more", requests.len() - sample.len())
    } else {
        String::new()
    };

    format!(
        "{} translation request(s): {}{}",
        requests.len(),
        sample.join(", "),
        suffix
    )
}

fn clean_required_string(
    value: Option<&Value>,
    field: &str,
    max_length: usize,
) -> Result<String, String> {
    let Some(Value::String(value)) = value else {
        return Err(format!("{field} must be a string"));
    };
    let cleaned = value.trim_matches(is_javascript_whitespace).to_owned();

    if cleaned.is_empty() {
        return Err(format!("{field} is required"));
    }
    if cleaned.encode_utf16().count() > max_length {
        return Err(format!("{field} must be {max_length} characters or fewer"));
    }

    Ok(cleaned)
}

fn clean_language(value: Option<&Value>, field: &str) -> Result<String, String> {
    let language = normalize_language_code(clean_required_string(
        value,
        field,
        MAX_LANGUAGE_CODE_LENGTH,
    )?);

    if !valid_language_code(&language) {
        return Err(format!(
            "{field} must be a language code like en, de, fr-CA, zh-CN, ms-Arab, or mni-Mtei"
        ));
    }

    Ok(language)
}

fn normalize_language_code(value: String) -> String {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() != 2 {
        return value.to_lowercase();
    }

    let region = parts[1];
    let normalized_region = if region.encode_utf16().count() == 2 {
        region.to_uppercase()
    } else {
        let mut characters = region.chars();
        match characters.next() {
            Some(first) => format!(
                "{}{}",
                first.to_uppercase(),
                characters.as_str().to_lowercase()
            ),
            None => String::new(),
        }
    };

    format!("{}-{normalized_region}", parts[0].to_lowercase())
}

fn valid_language_code(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    if !(parts[0].len() == 2 || parts[0].len() == 3)
        || !parts[0].bytes().all(|byte| byte.is_ascii_lowercase())
    {
        return false;
    }
    if parts.len() == 1 {
        return true;
    }
    if parts.len() != 2 {
        return false;
    }

    let region = parts[1];
    (region.len() == 2 && region.bytes().all(|byte| byte.is_ascii_uppercase()))
        || (region.len() == 3 && region.bytes().all(|byte| byte.is_ascii_digit()))
        || (region.len() == 4
            && region.as_bytes()[0].is_ascii_uppercase()
            && region.as_bytes()[1..]
                .iter()
                .all(|byte| byte.is_ascii_lowercase()))
}

fn is_javascript_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_one_request_from_top_level_fields() {
        let requests = build_translation_requests(&json!({
            "text": " Good morning ",
            "source": "EN",
            "target": "de"
        }))
        .unwrap();

        assert_eq!(
            requests,
            [TranslationRequest {
                index: 0,
                text: "Good morning".to_owned(),
                source: "en".to_owned(),
                target: "de".to_owned(),
            }]
        );
    }

    #[test]
    fn batch_items_override_top_level_fields_and_normalize_regions() {
        let requests = build_translation_requests(&json!({
            "text": "Ignored",
            "source": "en",
            "target": "de",
            "items": [
                {"text": "Hello", "source": "en", "target": "es"},
                {"text": "Welcome", "source": "pt-br", "target": "ZH-cn"},
                {"text": "Script code", "source": "MS-arab", "target": "MNI-mtei"},
                {"text": "Latin America", "source": "en", "target": "ES-419"}
            ]
        }))
        .unwrap();

        assert_eq!(requests[0].source, "en");
        assert_eq!(requests[0].target, "es");
        assert_eq!(requests[1].source, "pt-BR");
        assert_eq!(requests[1].target, "zh-CN");
        assert_eq!(requests[2].source, "ms-Arab");
        assert_eq!(requests[2].target, "mni-Mtei");
        assert_eq!(requests[3].target, "es-419");
    }

    #[test]
    fn null_items_keep_single_item_compatibility() {
        let requests = build_translation_requests(&json!({
            "items": null,
            "text": " Hello ",
            "source": "en",
            "target": "de"
        }))
        .unwrap();

        assert_eq!(requests[0].text, "Hello");
        assert_eq!(
            build_translation_requests(&json!("not an input object")).unwrap_err(),
            "text must be a string"
        );
    }

    #[test]
    fn validates_input_types_lengths_and_language_codes() {
        assert_eq!(
            build_translation_requests(&json!({})).unwrap_err(),
            "text must be a string"
        );
        assert_eq!(
            build_translation_requests(&json!({"text":"", "source":"en", "target":"de"}))
                .unwrap_err(),
            "text is required"
        );
        assert_eq!(
            build_translation_requests(&json!({"text":"Hello", "source":"en", "target":"en"}))
                .unwrap_err(),
            "target must be different from source"
        );
        assert_eq!(
            build_translation_requests(&json!({"items": []})).unwrap_err(),
            "items must include at least one translation item"
        );
        assert_eq!(
            build_translation_requests(&json!({"items": ["Hello"]})).unwrap_err(),
            "items[0] must be an object"
        );
        assert_eq!(
            build_translation_requests(&json!({
                "items": [{"text":"Hello", "source":"english", "target":"de"}]
            }))
            .unwrap_err(),
            "items[0].source must be a language code like en, de, fr-CA, zh-CN, ms-Arab, or mni-Mtei"
        );
        assert_eq!(
            build_translation_requests(&json!({
                "text":"Hello", "source":"en", "target":"es-1a2"
            }))
            .unwrap_err(),
            "target must be a language code like en, de, fr-CA, zh-CN, ms-Arab, or mni-Mtei"
        );
        assert_eq!(
            build_translation_requests(&json!({
                "text":"Hello", "source":"en", "target":"zh-CN-Hans"
            }))
            .unwrap_err(),
            "target must be a language code like en, de, fr-CA, zh-CN, ms-Arab, or mni-Mtei"
        );
        assert!(
            build_translation_requests(&json!({
                "text": "😀".repeat(2_501),
                "source": "en",
                "target": "de"
            }))
            .unwrap_err()
            .contains("5000 characters or fewer")
        );
    }

    #[test]
    fn describes_request_batch_and_keeps_existing_prefill_schema() {
        let requests = build_translation_requests(&json!({
            "items": [
                {"text":"Hello", "source":"en", "target":"de"},
                {"text":"Hola", "source":"es", "target":"fr"},
                {"text":"Bonjour", "source":"fr", "target":"it"},
                {"text":"Ciao", "source":"it", "target":"en"}
            ]
        }))
        .unwrap();
        assert_eq!(
            describe_translation_requests(&requests),
            "4 translation request(s): en->de, es->fr, fr->it and 1 more"
        );

        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["properties"]["items"]["type"], "array");
        assert_eq!(schema["properties"]["items"]["maxItems"], 100);
        assert_eq!(
            schema["properties"]["items"]["items"]["required"],
            json!(["text", "source", "target"])
        );
        assert_eq!(
            schema["properties"]["items"]["prefill"][0]["text"],
            "Good morning"
        );
        assert_eq!(schema["properties"]["text"]["prefill"], "Good morning");
        assert_eq!(schema["properties"]["source"]["default"], "en");
        assert_eq!(schema["properties"]["target"]["default"], "de");
    }
}
