use super::run_actor;
use crate::{apify::ApifyClient, scrappa_client::SCRAPPA_USER_AGENT, ActorConfig};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

struct MockResponse {
    status: u16,
    body: String,
}

impl MockResponse {
    fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            body: body.to_string(),
        }
    }
}

#[derive(Debug)]
struct CapturedRequest {
    line: String,
    headers: String,
    body: String,
}

async fn start_mock_server(
    responses: Vec<MockResponse>,
) -> (String, JoinHandle<Vec<CapturedRequest>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut requests = Vec::with_capacity(responses.len());
        for response in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_http_request(&mut stream).await;
            let reason = match response.status {
                201 => "Created",
                503 => "Service Unavailable",
                _ => "OK",
            };
            let body = response.body.as_bytes();
            let headers = format!(
                "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n",
                response.status,
                reason,
                body.len()
            );
            stream.write_all(headers.as_bytes()).await.unwrap();
            stream.write_all(body).await.unwrap();
            requests.push(request);
        }
        requests
    });
    (format!("http://{address}"), server)
}

async fn read_http_request(stream: &mut TcpStream) -> CapturedRequest {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 2048];
    let header_end = loop {
        let read = stream.read(&mut chunk).await.unwrap();
        assert!(read > 0, "request ended before its headers were complete");
        bytes.extend_from_slice(&chunk[..read]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    while bytes.len() - header_end < content_length {
        let read = stream.read(&mut chunk).await.unwrap();
        assert!(read > 0, "request ended before its body was complete");
        bytes.extend_from_slice(&chunk[..read]);
    }
    let body = String::from_utf8_lossy(&bytes[header_end..header_end + content_length]).to_string();
    let line = headers.lines().next().unwrap_or_default().to_owned();
    CapturedRequest {
        line,
        headers,
        body,
    }
}

#[tokio::test]
async fn runs_batch_with_scrappa_auth_retry_dataset_output_and_ppe_budget() {
    let responses = vec![
        MockResponse::json(
            200,
            json!({
                "data": {
                    "pricingInfo": {
                        "pricingModel": "PAY_PER_EVENT",
                        "pricingPerEvent": {"actorChargeEvents": {
                            "property-result": {"eventPriceUsd": 0.0005}
                        }}
                    },
                    "options": {"maxTotalChargeUsd": 0.0005},
                    "chargedEventCounts": {}
                }
            }),
        ),
        MockResponse::json(200, json!({"property_ids": [123, 456]})),
        MockResponse::json(503, json!({"message": "Temporary Scrappa issue"})),
        MockResponse::json(
            200,
            json!({"data": {"property_id": 123, "address": "1 Main St"}}),
        ),
        MockResponse::json(201, Value::Null),
        MockResponse::json(201, Value::Null),
        MockResponse::json(200, Value::Null),
    ];
    let (base_url, server) = start_mock_server(responses).await;
    let config = ActorConfig {
        apify_api_base: base_url.clone(),
        apify_token: "apify-test-token".to_owned(),
        actor_run_id: "run-test".to_owned(),
        key_value_store_id: "store-test".to_owned(),
        dataset_id: "dataset-test".to_owned(),
        input_key: "INPUT".to_owned(),
        scrappa_api_base: format!("{base_url}/api"),
        scrappa_api_key: Some("scrappa-test-key".to_owned()),
    };
    let apify = ApifyClient::new(&config).unwrap();
    let run = apify.get_run().await.unwrap();
    run_actor(config, &apify, &run).await.unwrap();

    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 7);
    assert!(requests[0]
        .line
        .starts_with("GET /v2/actor-runs/run-test HTTP/1.1"));
    assert!(requests[0]
        .headers
        .to_ascii_lowercase()
        .contains("authorization: bearer apify-test-token"));
    assert!(requests[1]
        .line
        .starts_with("GET /v2/key-value-stores/store-test/records/INPUT HTTP/1.1"));

    for request in [&requests[2], &requests[3]] {
        assert!(request
            .line
            .starts_with("GET /api/redfin/property?property_id=123 HTTP/1.1"));
        let headers = request.headers.to_ascii_lowercase();
        assert!(headers.contains("x-api-key: scrappa-test-key"));
        assert!(headers.contains("accept: application/json"));
        assert!(headers.contains(&format!("user-agent: {SCRAPPA_USER_AGENT}")));
    }

    assert!(requests[4]
        .line
        .starts_with("POST /v2/datasets/dataset-test/items HTTP/1.1"));
    let dataset_item: Value = serde_json::from_str(&requests[4].body).unwrap();
    assert_eq!(dataset_item["property_id"], json!(123));
    assert_eq!(dataset_item["address"], json!("1 Main St"));

    assert!(requests[5]
        .line
        .starts_with("POST /v2/actor-runs/run-test/charge HTTP/1.1"));
    assert!(requests[5]
        .headers
        .to_ascii_lowercase()
        .contains("idempotency-key: redfin-property-result-run-test-0"));
    let charged_event: Value = serde_json::from_str(&requests[5].body).unwrap();
    assert_eq!(
        charged_event,
        json!({"eventName": "property-result", "count": 1})
    );

    assert!(requests[6]
        .line
        .starts_with("PUT /v2/key-value-stores/store-test/records/OUTPUT HTTP/1.1"));
    let output: Value = serde_json::from_str(&requests[6].body).unwrap();
    assert_eq!(output["properties_requested"], json!(2));
    assert_eq!(output["results"], json!(1));
    assert_eq!(output["errors"], json!(0));
    assert_eq!(
        output["status_message"],
        json!("Charge limit reached after saving Redfin property detail result 1.")
    );
}
