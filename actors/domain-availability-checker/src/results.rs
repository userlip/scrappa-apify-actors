use serde_json::{json, Map, Value};

pub fn success_item(
    response: &Value,
    input_domain: &str,
    requested_domain: &str,
) -> Result<Value, String> {
    let response = response
        .as_object()
        .ok_or_else(|| "Scrappa domain availability response was not a JSON object".to_owned())?;
    let mut result = json!({
        "success": true,
        "input_domain": input_domain,
        "domain": response.get("domain").and_then(Value::as_str).unwrap_or(requested_domain),
        "available": response.get("available").and_then(Value::as_bool),
        "registered": response.get("registered").and_then(Value::as_bool),
        "status": response.get("status").and_then(Value::as_str),
        "confidence": response.get("confidence").and_then(Value::as_str),
        "source": response.get("source").and_then(Value::as_str),
        "rdap_url": response.get("rdap_url").and_then(Value::as_str),
        "rdap_status_code": response
            .get("rdap_status_code")
            .filter(|value| value.is_number())
            .cloned(),
        "rdap_events": response.get("rdap_events").filter(|value| value.is_array()).cloned().unwrap_or_else(|| json!([])),
        "nameservers": response.get("nameservers").filter(|value| value.is_array()).cloned().unwrap_or_else(|| json!([])),
        "status_code": null,
    });
    if let Some(message) = response.get("message").and_then(Value::as_str) {
        result
            .as_object_mut()
            .expect("a JSON object was just created")
            .insert("message".to_owned(), Value::String(message.to_owned()));
    }
    Ok(result)
}

pub fn failure_item(
    error: &dyn std::fmt::Display,
    status_code: Option<u16>,
    input_domain: &str,
    domain: Option<&str>,
) -> Value {
    let mut result = Map::new();
    result.insert("success".to_owned(), Value::Bool(false));
    result.insert(
        "input_domain".to_owned(),
        Value::String(input_domain.to_owned()),
    );
    result.insert(
        "domain".to_owned(),
        domain.map_or(Value::Null, |domain| Value::String(domain.to_owned())),
    );
    result.insert("available".to_owned(), Value::Null);
    result.insert("registered".to_owned(), Value::Null);
    result.insert("status".to_owned(), Value::String("error".to_owned()));
    result.insert("confidence".to_owned(), Value::Null);
    result.insert("source".to_owned(), Value::String("scrappa".to_owned()));
    result.insert("rdap_url".to_owned(), Value::Null);
    result.insert("rdap_status_code".to_owned(), Value::Null);
    result.insert("rdap_events".to_owned(), json!([]));
    result.insert("nameservers".to_owned(), json!([]));
    result.insert("error".to_owned(), Value::String(error.to_string()));
    result.insert(
        "status_code".to_owned(),
        match status_code {
            Some(status) => json!(status),
            None => Value::Null,
        },
    );
    Value::Object(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrappa::ScrappaError;
    use serde_json::json;

    #[test]
    fn maps_success_responses_and_omits_non_string_messages() {
        let response = json!({
            "domain": "example.com",
            "available": false,
            "registered": true,
            "status": "registered",
            "confidence": "high",
            "source": "rdap",
            "rdap_url": "https://rdap.example/domain/example.com",
            "rdap_status_code": 200,
            "rdap_events": [{"action":"registration"}],
            "nameservers": ["NS1.EXAMPLE.COM"],
            "message": "Checked",
            "unknown": "ignored"
        });
        assert_eq!(
            success_item(&response, "https://example.com/path", "example.com").unwrap(),
            json!({
                "success": true,
                "input_domain": "https://example.com/path",
                "domain": "example.com",
                "available": false,
                "registered": true,
                "status": "registered",
                "confidence": "high",
                "source": "rdap",
                "rdap_url": "https://rdap.example/domain/example.com",
                "rdap_status_code": 200,
                "rdap_events": [{"action":"registration"}],
                "nameservers": ["NS1.EXAMPLE.COM"],
                "message": "Checked",
                "status_code": null
            })
        );

        let response = json!({ "message": 12, "nameservers": "not-an-array" });
        let result = success_item(&response, "example.com", "example.com").unwrap();
        assert!(result.get("message").is_none());
        assert_eq!(result["nameservers"], json!([]));
    }

    #[test]
    fn maps_http_and_validation_errors() {
        let error = ScrappaError::Http {
            status: 422,
            message: "Invalid request".to_owned(),
        };
        assert_eq!(
            failure_item(
                &error,
                error.http_status(),
                "bad_domain.com",
                Some("bad_domain.com")
            ),
            json!({
                "success": false,
                "input_domain": "bad_domain.com",
                "domain": "bad_domain.com",
                "available": null,
                "registered": null,
                "status": "error",
                "confidence": null,
                "source": "scrappa",
                "rdap_url": null,
                "rdap_status_code": null,
                "rdap_events": [],
                "nameservers": [],
                "error": "Scrappa API error (422): Invalid request",
                "status_code": 422
            })
        );
        assert_eq!(
            failure_item(&"Invalid domain", None, "localhost", None)["status_code"],
            Value::Null
        );
    }

    #[test]
    fn rejects_non_object_success_responses() {
        assert!(success_item(&Value::Null, "example.com", "example.com").is_err());
    }
}
