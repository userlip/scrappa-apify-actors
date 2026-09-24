use serde_json::{json, Map, Value};

use crate::{
    input::{ResponseType, UrlRequest, WebScraperParams},
    web_scraper_client::ScrappaError,
};

pub fn build_json_dataset_item(
    response: &Value,
    request: &UrlRequest,
    params: &WebScraperParams,
) -> Value {
    let mut item = response.as_object().cloned().unwrap_or_default();
    let data = response.get("data").and_then(Value::as_object);
    let links_count = array_len(data.and_then(|data| data.get("links")));
    let emails_count = array_len(data.and_then(|data| data.get("emails")));
    let phone_numbers_count = array_len(data.and_then(|data| data.get("phone_numbers")));
    let images_count = array_len(data.and_then(|data| data.get("images")));

    item.insert(
        "success".to_owned(),
        Value::Bool(response.get("success").and_then(Value::as_bool) == Some(true)),
    );
    item.insert("input_url".to_owned(), json!(request.input_url));
    item.insert("request_url".to_owned(), json!(params.url));
    item.insert(
        "response_type".to_owned(),
        json!(ResponseType::Json.as_str()),
    );
    item.insert(
        "include_html".to_owned(),
        Value::Bool(params.include_html == Some(true)),
    );
    item.insert(
        "url".to_owned(),
        response
            .get("url")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or_else(|| json!(params.url)),
    );
    item.insert(
        "final_url".to_owned(),
        response
            .get("final_url")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    );
    item.insert(
        "site_status_code".to_owned(),
        response
            .get("site_status_code")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    );
    for field in ["title", "description", "body_text"] {
        item.insert(
            field.to_owned(),
            data.and_then(|data| data.get(field))
                .filter(|value| !value.is_null())
                .cloned()
                .unwrap_or(Value::Null),
        );
    }
    item.insert("links_count".to_owned(), json!(links_count));
    item.insert("emails_count".to_owned(), json!(emails_count));
    item.insert("phone_numbers_count".to_owned(), json!(phone_numbers_count));
    item.insert("images_count".to_owned(), json!(images_count));
    item.insert(
        "languages_detected".to_owned(),
        data.and_then(|data| data.get("languages_detected"))
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or_else(|| json!([])),
    );

    Value::Object(item)
}

pub fn build_markdown_dataset_item(
    markdown: &str,
    request: &UrlRequest,
    params: &WebScraperParams,
) -> Value {
    json!({
        "success": true,
        "input_url": request.input_url,
        "request_url": params.url,
        "response_type": ResponseType::Markdown.as_str(),
        "include_html": false,
        "url": params.url,
        "final_url": null,
        "site_status_code": null,
        "markdown": markdown,
        "markdown_length": markdown.chars().count(),
    })
}

pub fn build_failure_dataset_item(
    error: &ScrappaError,
    request: &UrlRequest,
    params: &WebScraperParams,
) -> Value {
    let body = error.body();
    json!({
        "success": false,
        "input_url": request.input_url,
        "request_url": params.url,
        "response_type": params.response_type.as_str(),
        "include_html": params.include_html == Some(true),
        "url": params.url,
        "final_url": null,
        "site_status_code": null,
        "status_code": error.status_code(),
        "error": error.to_string(),
        "error_type": error.error_type(),
        "error_code": body.and_then(|body| body.get("error_code")).cloned().unwrap_or(Value::Null),
        "diagnostics": body.and_then(|body| body.get("diagnostics")).cloned().unwrap_or(Value::Null),
    })
}

fn array_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, Vec::len)
}

pub fn insert_summary_fields(
    output: &mut Map<String, Value>,
    requested: usize,
    saved: usize,
    succeeded: usize,
    failed: usize,
    response_type: ResponseType,
) {
    output.insert("requested".to_owned(), json!(requested));
    output.insert("saved".to_owned(), json!(saved));
    output.insert("succeeded".to_owned(), json!(succeeded));
    output.insert("failed".to_owned(), json!(failed));
    output.insert("response_type".to_owned(), json!(response_type.as_str()));
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::{
        input::{ResponseType, UrlRequest, WebScraperParams},
        web_scraper_client::ScrappaError,
    };

    use super::{
        build_failure_dataset_item, build_json_dataset_item, build_markdown_dataset_item,
        insert_summary_fields,
    };

    fn request() -> UrlRequest {
        UrlRequest {
            input_url: "https://example.com".to_owned(),
            request_url: "https://example.com".to_owned(),
        }
    }

    #[test]
    fn normalizes_json_fields_and_keeps_the_upstream_response() {
        let params = WebScraperParams {
            url: "https://example.com".to_owned(),
            include_html: Some(true),
            response_type: ResponseType::Json,
        };
        let item = build_json_dataset_item(
            &json!({
                "success": true,
                "site_status_code": 200,
                "url": "https://example.com",
                "final_url": "https://example.com/",
                "data": {
                    "title": "Example Domain",
                    "description": "Example description",
                    "body_text": "Example body",
                    "links": ["https://www.iana.org/domains/example"],
                    "emails": [],
                    "phone_numbers": ["+1 555 1000"],
                    "images": [{"src": "https://example.com/image.png"}],
                    "languages_detected": ["en"]
                }
            }),
            &request(),
            &params,
        );

        assert_eq!(item["success"], true);
        assert_eq!(item["input_url"], "https://example.com");
        assert_eq!(item["response_type"], "json");
        assert_eq!(item["include_html"], true);
        assert_eq!(item["site_status_code"], 200);
        assert_eq!(item["title"], "Example Domain");
        assert_eq!(item["links_count"], 1);
        assert_eq!(item["emails_count"], 0);
        assert_eq!(item["phone_numbers_count"], 1);
        assert_eq!(item["images_count"], 1);
        assert_eq!(item["languages_detected"], json!(["en"]));
        assert!(item.get("data").is_some());
    }

    #[test]
    fn requires_explicit_json_success_true() {
        let params = WebScraperParams {
            url: "https://example.com".to_owned(),
            include_html: Some(false),
            response_type: ResponseType::Json,
        };
        let item = build_json_dataset_item(
            &json!({"data": {"title": "Partial page"}}),
            &request(),
            &params,
        );
        assert_eq!(item["success"], false);
        assert_eq!(item["title"], "Partial page");
    }

    #[test]
    fn wraps_markdown_and_counts_unicode_code_points() {
        let params = WebScraperParams {
            url: "https://example.com".to_owned(),
            include_html: None,
            response_type: ResponseType::Markdown,
        };
        let item = build_markdown_dataset_item("A😀B", &request(), &params);
        assert_eq!(item["success"], true);
        assert_eq!(item["response_type"], "markdown");
        assert_eq!(item["include_html"], false);
        assert_eq!(item["markdown_length"], 3);
    }

    #[test]
    fn builds_failure_items_with_http_status_and_scrappa_error_fields() {
        let params = WebScraperParams {
            url: "bad-url".to_owned(),
            include_html: Some(false),
            response_type: ResponseType::Json,
        };
        let error = ScrappaError::Http {
            status: 400,
            details: "Invalid URL format.".to_owned(),
            body: Some(json!({
                "error_code": "INVALID_URL",
                "diagnostics": {"field": "url"}
            })),
        };
        let item = build_failure_dataset_item(&error, &request(), &params);

        assert_eq!(item["success"], false);
        assert_eq!(item["status_code"], 400);
        assert_eq!(item["error_type"], "scrappa_api_error");
        assert_eq!(item["error_code"], "INVALID_URL");
        assert_eq!(item["diagnostics"]["field"], "url");
        assert!(item["error"]
            .as_str()
            .unwrap()
            .contains("Invalid URL format"));
    }

    #[test]
    fn records_the_run_summary_for_the_key_value_store() {
        let mut summary = serde_json::Map::new();
        insert_summary_fields(&mut summary, 3, 2, 1, 1, ResponseType::Json);
        let value = Value::Object(summary);
        assert_eq!(
            value,
            json!({
                "requested": 3,
                "saved": 2,
                "succeeded": 1,
                "failed": 1,
                "response_type": "json"
            })
        );
    }
}
