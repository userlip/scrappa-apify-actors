use serde::Serialize;
use serde_json::Value;

use crate::{input::TranslationRequest, scrappa::ScrappaError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TranslationDatasetItem {
    pub success: bool,
    pub index: usize,
    pub text: String,
    pub translated_text: Option<String>,
    pub source: String,
    pub target: String,
    pub error: Option<String>,
    pub status_code: Option<u16>,
}

pub fn extract_translated_text(response: &Value) -> Result<String, String> {
    if let Some(value) = response.as_str().and_then(clean_translation_field) {
        return Ok(value);
    }

    if let Some(payload) = response.as_object() {
        for field in ["translated_text", "translation", "result"] {
            if let Some(value) = payload
                .get(field)
                .and_then(Value::as_str)
                .and_then(clean_translation_field)
            {
                return Ok(value);
            }
        }

        if let Some(data) = payload.get("data").filter(|value| !value.is_null()) {
            return extract_translated_text(data);
        }
    }

    Err("Scrappa Google Translate response did not include translated_text".to_owned())
}

pub fn build_translation_dataset_item(
    request: &TranslationRequest,
    response: &Value,
) -> Result<TranslationDatasetItem, String> {
    Ok(TranslationDatasetItem {
        success: true,
        index: request.index,
        text: request.text.clone(),
        translated_text: Some(extract_translated_text(response)?),
        source: request.source.clone(),
        target: request.target.clone(),
        error: None,
        status_code: None,
    })
}

pub fn build_translation_failure_item(
    request: &TranslationRequest,
    error: &ScrappaError,
) -> TranslationDatasetItem {
    TranslationDatasetItem {
        success: false,
        index: request.index,
        text: request.text.clone(),
        translated_text: None,
        source: request.source.clone(),
        target: request.target.clone(),
        error: Some(error.to_string()),
        status_code: error.status_code(),
    }
}

pub fn build_translation_failure_item_with_message(
    request: &TranslationRequest,
    message: String,
) -> TranslationDatasetItem {
    TranslationDatasetItem {
        success: false,
        index: request.index,
        text: request.text.clone(),
        translated_text: None,
        source: request.source.clone(),
        target: request.target.clone(),
        error: Some(message),
        status_code: None,
    }
}

fn clean_translation_field(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request() -> TranslationRequest {
        TranslationRequest {
            index: 1,
            text: "Good morning".to_owned(),
            source: "en".to_owned(),
            target: "de".to_owned(),
        }
    }

    #[test]
    fn extracts_translation_fields_in_precedence_order() {
        assert_eq!(
            extract_translated_text(&json!({"translated_text":"Guten Morgen"})).unwrap(),
            "Guten Morgen"
        );
        assert_eq!(
            extract_translated_text(&json!({"data":{"translated_text":"Hola"}})).unwrap(),
            "Hola"
        );
        assert_eq!(
            extract_translated_text(&json!({"data":"Hola"})).unwrap(),
            "Hola"
        );
        assert_eq!(
            extract_translated_text(&json!({"translation":"Bonjour"})).unwrap(),
            "Bonjour"
        );
        assert_eq!(
            extract_translated_text(&json!({"result":"Ciao"})).unwrap(),
            "Ciao"
        );
        assert_eq!(extract_translated_text(&json!("Hallo")).unwrap(), "Hallo");
        assert_eq!(
            extract_translated_text(&json!({
                "translated_text":"Guten Morgen",
                "translation":"Bonjour",
                "result":"Ciao",
                "data":{"translated_text":"Hola"}
            }))
            .unwrap(),
            "Guten Morgen"
        );
        assert_eq!(
            extract_translated_text(&json!({
                "translation":"Bonjour",
                "result":"Ciao",
                "data":{"translated_text":"Hola"}
            }))
            .unwrap(),
            "Bonjour"
        );
    }

    #[test]
    fn reports_missing_or_blank_translation_text() {
        for response in [
            json!({"data":{}}),
            json!({"data":null}),
            json!(""),
            json!({"translated_text":"   "}),
        ] {
            assert_eq!(
                extract_translated_text(&response).unwrap_err(),
                "Scrappa Google Translate response did not include translated_text"
            );
        }
    }

    #[test]
    fn builds_success_and_failure_dataset_rows() {
        assert_eq!(
            build_translation_dataset_item(&request(), &json!({"translated_text":"Guten Morgen"}))
                .unwrap(),
            TranslationDatasetItem {
                success: true,
                index: 1,
                text: "Good morning".to_owned(),
                translated_text: Some("Guten Morgen".to_owned()),
                source: "en".to_owned(),
                target: "de".to_owned(),
                error: None,
                status_code: None,
            }
        );
        let failure = build_translation_failure_item(
            &request(),
            &ScrappaError::Http {
                status: 503,
                details: "Translation service temporarily unavailable. Please retry.".to_owned(),
            },
        );
        assert_eq!(failure.success, false);
        assert_eq!(failure.translated_text, None);
        assert_eq!(
            failure.error.as_deref(),
            Some(
                "Scrappa API error (503): Translation service temporarily unavailable. Please retry."
            )
        );
        assert_eq!(failure.status_code, Some(503));
    }
}
