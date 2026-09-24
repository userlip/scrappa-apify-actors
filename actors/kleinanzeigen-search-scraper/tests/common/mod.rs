use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
};

use reqwest::StatusCode;
use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};
use url::Url;

use kleinanzeigen_search_scraper::Config;

#[derive(Clone)]
pub(super) struct MockResponse {
    status: u16,
    pub(super) body: String,
}

impl MockResponse {
    pub(super) fn json(status: u16, body: Value) -> Self {
        Self {
            status,
            body: body.to_string(),
        }
    }

    pub(super) fn text(status: u16, body: &str) -> Self {
        Self {
            status,
            body: body.to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct RecordedRequest {
    pub(super) method: String,
    pub(super) target: String,
    pub(super) headers: HashMap<String, String>,
    pub(super) body: String,
}

pub(super) struct MockServer {
    pub(super) base_url: String,
    pub(super) requests: Arc<Mutex<Vec<RecordedRequest>>>,
    handle: JoinHandle<()>,
}

impl MockServer {
    pub(super) async fn start(responses: Vec<MockResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded_requests = Arc::clone(&requests);
        let handle = tokio::spawn(async move {
            for response in responses {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let Ok(Some(request)) = read_request(&mut stream).await else {
                    break;
                };
                recorded_requests.lock().unwrap().push(request);
                let reason = StatusCode::from_u16(response.status)
                    .ok()
                    .and_then(|status| status.canonical_reason())
                    .unwrap_or("Unknown");
                let response_text = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    );
                let _ = stream.write_all(response_text.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });
        Self {
            base_url: format!("http://{address}"),
            requests,
            handle,
        }
    }

    pub(super) fn finish(self) -> Vec<RecordedRequest> {
        self.handle.abort();
        let requests = self.requests.lock().unwrap().clone();
        requests
    }
}

pub(super) async fn read_request(stream: &mut TcpStream) -> io::Result<Option<RecordedRequest>> {
    let mut bytes = Vec::new();
    let mut buffer = [0; 1024];
    let header_end = loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            return Ok(None);
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break position + 4;
        }
    };
    let header_text = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = header_text.split("\r\n");
    let Some(request_line) = lines.next() else {
        return Ok(None);
    };
    let mut request_parts = request_line.split_whitespace();
    let Some(method) = request_parts.next().map(str::to_owned) else {
        return Ok(None);
    };
    let Some(target) = request_parts.next().map(str::to_owned) else {
        return Ok(None);
    };
    let mut headers = HashMap::new();
    let mut content_length = 0;
    for line in lines.filter(|line| !line.is_empty()) {
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_owned();
            if name == "content-length" {
                content_length = value.parse::<usize>().unwrap_or(0);
            }
            headers.insert(name, value);
        }
    }
    while bytes.len() < header_end + content_length {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    let body =
        String::from_utf8_lossy(&bytes[header_end..bytes.len().min(header_end + content_length)])
            .to_string();
    Ok(Some(RecordedRequest {
        method,
        target,
        headers,
        body,
    }))
}

pub(super) fn test_config(base_url: &str) -> Config {
    Config {
        apify_api_base: Url::parse(base_url).unwrap(),
        scrappa_api_base: Url::parse(&format!("{base_url}/api")).unwrap(),
        apify_token: "test-token".to_owned(),
        key_value_store_id: "test-store".to_owned(),
        dataset_id: "test-dataset".to_owned(),
        actor_run_id: "test-run".to_owned(),
        input_key: "INPUT".to_owned(),
        scrappa_api_key: Some("scrappa-test-key".to_owned()),
    }
}

pub(super) fn input_response(input: Value) -> MockResponse {
    MockResponse::json(200, input)
}

pub(super) fn flat_pricing() -> MockResponse {
    MockResponse::json(
        200,
        json!({"data": {"pricingInfo": {"pricingModel": "PRICE_PER_DATASET_ITEM"}}}),
    )
}

pub(super) fn ppe_pricing(max_charge: f64, charged: u64) -> MockResponse {
    MockResponse::json(
        200,
        json!({"data": {
            "pricingInfo": {
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {"actorChargeEvents": {
                    "listing-result": {"eventPriceUsd": 0.1},
                    "apify-default-dataset-item": {"eventPriceUsd": 0.05},
                    "apify-actor-start": {"eventPriceUsd": 0.05}
                }}
            },
            "options": {"maxTotalChargeUsd": max_charge},
            "chargedEventCounts": {
                "listing-result": charged,
                "apify-default-dataset-item": 0,
                "apify-actor-start": 0
            }
        }}),
    )
}

pub(super) fn listing_response(count: usize) -> MockResponse {
    MockResponse::json(
        200,
        json!({
            "data": (0..count).map(|index| json!({
                "id": format!("listing-{index}"),
                "title": format!("Listing {index}"),
                "image": "https://img.kleinanzeigen.de/example.jpg"
            })).collect::<Vec<_>>(),
            "meta": {"results_count": count}
        }),
    )
}

pub(super) fn requests_to<'a>(
    requests: &'a [RecordedRequest],
    target: &str,
) -> Vec<&'a RecordedRequest> {
    requests
        .iter()
        .filter(|request| request.target.starts_with(target))
        .collect()
}

pub(super) fn query_values(target: &str) -> Vec<(String, String)> {
    Url::parse(&format!("http://localhost{target}"))
        .unwrap()
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect()
}
