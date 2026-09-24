use std::time::Duration;

use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use url::Url;

use crate::{
    input::HistoricalPricesRequest,
    output::{build_dataset_items, no_data_output},
    scrappa::{ScrappaClient, ScrappaError},
};

#[test]
fn normalizes_preset_and_custom_range_requests() {
    let preset = HistoricalPricesRequest::build(&json!({
        "symbol": " aapl ",
        "exchange": " nasdaq ",
        "range": "6",
        "interval": "WEEKLY",
        "hl": "EN",
        "gl": "US",
    }))
    .unwrap();
    assert_eq!(
        preset.to_value(),
        json!({
            "symbol": "AAPL",
            "exchange": "NASDAQ",
            "range": 6,
            "interval": "weekly",
            "hl": "en",
            "gl": "us",
        })
    );
    assert_eq!(
        preset.describe(),
        "AAPL:NASDAQ (range=6, interval=weekly, hl=en, gl=us)"
    );

    let custom = HistoricalPricesRequest::build(&json!({
        "symbol": "MSFT",
        "exchange": "NASDAQ",
        "start_date": "2024-01-01",
        "end_date": "2024-01-31",
        "interval": "daily",
    }))
    .unwrap();
    assert!(custom.has_custom_date_range());
    assert_eq!(custom.start_date.as_deref(), Some("2024-01-01"));
    assert_eq!(custom.end_date.as_deref(), Some("2024-01-31"));
}

#[test]
fn rejects_invalid_inputs_with_matching_messages() {
    let cases = [
        (json!({"symbol": "   "}), "symbol is required"),
        (json!({"symbol": "BRK B"}), "symbol cannot contain spaces"),
        (
            json!({"symbol": "AAPL", "range": true}),
            "range must be an integer",
        ),
        (
            json!({"symbol": "AAPL", "range": 9}),
            "range must be between 1 and 8",
        ),
        (
            json!({"symbol": "AAPL", "range": 6, "start_date": "2024-01-01", "end_date": "2024-01-31"}),
            "Cannot use both range and start_date/end_date parameters together",
        ),
        (
            json!({"symbol": "AAPL", "start_date": "2024-01-01"}),
            "start_date and end_date must be provided together",
        ),
        (
            json!({"symbol": "AAPL", "start_date": "2024-02-01", "end_date": "2024-01-31"}),
            "end_date must be on or after start_date",
        ),
        (
            json!({"symbol": "AAPL", "start_date": "2024-02-30", "end_date": "2024-03-01"}),
            "start_date must be a valid date",
        ),
        (
            json!({"symbol": "AAPL", "interval": "hourly"}),
            "interval must be one of: daily, weekly, monthly",
        ),
        (
            json!({"symbol": "AAPL", "hl": "english"}),
            "hl must be a valid language code",
        ),
        (
            json!({"symbol": "AAPL", "gl": "u1"}),
            "gl must be a two-letter country code",
        ),
    ];

    for (input, message) in cases {
        let error = HistoricalPricesRequest::build(&input).unwrap_err();
        assert!(
            error.to_string().contains(message),
            "{error} did not contain {message}"
        );
    }
}

#[test]
fn input_schema_keeps_the_existing_fields_and_symbol_prefill() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    let properties = schema.get("properties").unwrap();
    assert_eq!(schema["required"], json!(["symbol"]));
    assert_eq!(properties["symbol"]["prefill"], "AAPL");
    for field in [
        "symbol",
        "exchange",
        "range",
        "start_date",
        "end_date",
        "interval",
        "hl",
        "gl",
    ] {
        assert!(
            properties.get(field).is_some(),
            "missing input field {field}"
        );
    }
}

#[test]
fn creates_one_ordered_dataset_item_for_each_upstream_price_point() {
    let request = HistoricalPricesRequest::build(&json!({
        "symbol": "AAPL",
        "exchange": "NASDAQ",
        "range": 6,
        "interval": "daily",
        "hl": "en",
        "gl": "us",
    }))
    .unwrap();
    let response = json!({
        "symbol": "AAPL",
        "exchange": "NASDAQ",
        "currency": "USD",
        "previous_close": "275.50",
        "prices": [
            {"date": 1704067200, "close": "241.53", "change": "0", "percent_change": "0.25", "volume": "53,614,054", "source_page": 1},
            {"date": "1704153600", "close": "242.00", "volume": 53600000},
            {"date": 1704240000, "close": "243", "volume": "53,500,000", "source_page": 2}
        ]
    });

    let items = build_dataset_items(&response, &request);
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["position"], 1);
    assert_eq!(items[0]["date"], 1704067200_i64);
    assert_eq!(items[0]["date_iso"], "2024-01-01");
    assert_eq!(items[0]["close"], 241.53);
    assert_eq!(items[0]["volume"], 53_614_054_i64);
    assert_eq!(items[0]["previous_close"], 275.5);
    assert_eq!(items[0]["source_page"], 1);
    assert_eq!(items[0]["result_counts"], json!({"prices": 3}));
    assert_eq!(items[1]["position"], 2);
    assert_eq!(items[1]["date_iso"], "2024-01-02");
    assert_eq!(items[2]["position"], 3);
    assert_eq!(items[2]["date_iso"], "2024-01-03");
    assert_eq!(items[2]["result_counts"], json!({"prices": 3}));
}

#[test]
fn filters_non_object_prices_and_returns_empty_for_missing_prices() {
    let request = HistoricalPricesRequest::build(&json!({"symbol": "VOO", "range": 7})).unwrap();
    let items = build_dataset_items(
        &json!({"prices": [null, 1, [], {"close": "bad"}]}),
        &request,
    );
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["close"], Value::Null);
    assert!(build_dataset_items(&json!({"currency": "USD"}), &request).is_empty());
}

#[test]
fn creates_the_existing_custom_range_not_found_output() {
    let request = HistoricalPricesRequest::build(&json!({
        "symbol": "AAPL",
        "exchange": "NASDAQ",
        "start_date": "2024-01-01",
        "end_date": "2024-01-31",
    }))
    .unwrap();
    let output = no_data_output(&request, "No historical rows");
    assert_eq!(output["symbol"], "AAPL");
    assert_eq!(output["exchange"], "NASDAQ");
    assert_eq!(output["prices"], json!([]));
    assert_eq!(output["request"]["start_date"], "2024-01-01");
    assert_eq!(output["error_code"], "NOT_FOUND");
    assert_eq!(output["status_code"], 404);
}

#[tokio::test]
async fn sends_scrappa_auth_headers_and_the_existing_query() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request(&mut stream).await;
        write_response(&mut stream, 200, r#"{"prices":[]}"#).await;
        request
    });

    let request = HistoricalPricesRequest::build(&json!({
        "symbol": "AAPL",
        "exchange": "NASDAQ",
        "range": 6,
        "interval": "weekly",
        "hl": "en",
        "gl": "us",
    }))
    .unwrap();
    let client = ScrappaClient::new(
        Url::parse(&format!("http://{address}/api")).unwrap(),
        "test-key".to_owned(),
        Duration::from_secs(1),
        3,
    );

    let response = client.get_historical(&request).await.unwrap();
    let captured = server.await.unwrap().to_ascii_lowercase();
    assert_eq!(response, json!({"prices": []}));
    assert!(captured.contains("get /api/google-finance/historical?symbol=aapl&exchange=nasdaq&range=6&interval=weekly&hl=en&gl=us "));
    assert!(captured.contains("x-api-key: test-key"));
    assert!(
        captured.contains("user-agent: thescrappa-google-finance-historical-prices-scraper/1.0")
    );
}

#[tokio::test]
async fn retries_retryable_statuses_and_preserves_json_error_details() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for attempt in 0..2 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let _ = read_request(&mut stream).await;
            if attempt == 0 {
                write_response(
                    &mut stream,
                    503,
                    r#"{"message":"Temporarily unavailable","code":"RETRY","errors":{"symbol":["Try again"]}}"#,
                )
                .await;
            } else {
                write_response(&mut stream, 200, r#"{"symbol":"AAPL"}"#).await;
            }
        }
    });
    let request = HistoricalPricesRequest::build(&json!({"symbol": "AAPL"})).unwrap();
    let client = ScrappaClient::with_retry_base_delay(
        Url::parse(&format!("http://{address}/api")).unwrap(),
        "test-key".to_owned(),
        Duration::from_secs(1),
        3,
        0,
    );

    assert_eq!(
        client.get_historical(&request).await.unwrap(),
        json!({"symbol": "AAPL"})
    );
    server.await.unwrap();

    let error = ScrappaError::Http {
        status: 422,
        message: "Invalid request - symbol: Required".to_owned(),
    };
    assert!(!error.retryable());
    assert_eq!(error.status(), Some(422));
}

#[tokio::test]
async fn classifies_scrappa_deadlines_as_timeouts() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let _ = read_request(&mut stream).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = write_response(&mut stream, 200, r#"{"prices":[]}"#).await;
    });
    let request = HistoricalPricesRequest::build(&json!({"symbol": "AAPL"})).unwrap();
    let client = ScrappaClient::new(
        Url::parse(&format!("http://{address}/api")).unwrap(),
        "test-key".to_owned(),
        Duration::from_millis(20),
        1,
    );

    let error = client.get_historical(&request).await.unwrap_err();
    assert!(matches!(error, ScrappaError::Timeout { .. }));
    assert_eq!(
        error.to_string(),
        "Scrappa API request timed out after 20ms"
    );
    server.await.unwrap();
}

async fn read_request(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let read = stream.read(&mut chunk).await.unwrap();
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

async fn write_response(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
}
