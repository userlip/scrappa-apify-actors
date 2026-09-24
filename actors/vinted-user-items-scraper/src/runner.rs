use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde_json::Value;

use crate::{
    apify::{ActorPricing, ApifyClient, DEFAULT_DATASET_ITEM_EVENT},
    input::VintedUserItemsPlan,
    response::{build_dataset_item, get_items, get_pagination},
    scrappa::{ScrappaClient, ScrappaError},
};

const USER_ITEM_RESULT_CHARGE_EVENT: &str = "user-item-result";

#[derive(Debug, PartialEq, Eq)]
pub struct RunSummary {
    pub pages_fetched: usize,
    pub saved_items: usize,
    pub status_message: Option<String>,
}

pub async fn run_actor(
    apify: &ApifyClient,
    scrappa: &ScrappaClient,
    plan: &VintedUserItemsPlan,
    pricing: &mut ActorPricing,
) -> Result<RunSummary> {
    let summary = run_user_items(apify, scrappa, plan, pricing).await?;
    if let Some(message) = summary.status_message.as_deref() {
        if let Err(error) = apify.set_status_message(message).await {
            eprintln!("Could not set terminal Apify status message: {error}");
        }
    }
    Ok(summary)
}

pub async fn run_user_items(
    apify: &ApifyClient,
    scrappa: &ScrappaClient,
    plan: &VintedUserItemsPlan,
    pricing: &mut ActorPricing,
) -> Result<RunSummary> {
    let mut pages_fetched = 0;
    let mut saved_items = 0;
    let mut status_message = None;

    'users: for user_id in &plan.user_ids {
        for offset in 0..plan.max_pages {
            let page = plan.start_page + offset;
            let params = plan.page_params(user_id, page);
            println!(
                "Fetching Vinted seller {user_id} page {page} in {}",
                plan.country
            );
            let response = scrappa
                .get("/vinted/user-items", &params)
                .await
                .map_err(anyhow::Error::new)?;
            pages_fetched += 1;

            let items = get_items(&response);
            if items.is_empty() {
                println!("No Vinted listings found for seller {user_id} on page {page}");
                break;
            }

            let dataset_items = items
                .iter()
                .map(|item| build_dataset_item(item, plan, user_id, page, &response))
                .collect::<Vec<_>>();
            let push_count =
                pricing.limit_dataset_items(items.len(), USER_ITEM_RESULT_CHARGE_EVENT);
            if push_count == 0 {
                let message = charge_limit_message(0, items.len(), user_id, page);
                println!("{message}");
                status_message = Some(message);
                break 'users;
            }

            apify
                .push_dataset_items(&dataset_items[..push_count])
                .await?;
            saved_items += push_count;

            if pricing.is_pay_per_event {
                pricing.record_dataset_items(push_count);
                if pricing.event_is_configured(USER_ITEM_RESULT_CHARGE_EVENT) {
                    apify
                        .charge_event(
                            USER_ITEM_RESULT_CHARGE_EVENT,
                            push_count,
                            &charge_idempotency_key(apify.actor_run_id(), saved_items),
                        )
                        .await?;
                }
                pricing.record_event_charge(USER_ITEM_RESULT_CHARGE_EVENT, push_count);
            }

            println!(
                "Found {} listing(s) for seller {user_id} on page {page}; saved {push_count}",
                items.len()
            );
            if pricing.is_pay_per_event
                && (pricing.event_is_exhausted(USER_ITEM_RESULT_CHARGE_EVENT)
                    || pricing.event_is_exhausted(DEFAULT_DATASET_ITEM_EVENT))
            {
                let message = charge_limit_message(push_count, items.len(), user_id, page);
                println!("{message}");
                status_message = Some(message);
                break 'users;
            }

            let pagination = get_pagination(&response);
            let has_next_page = pagination
                .and_then(|pagination| pagination.get("has_next_page"))
                .and_then(Value::as_bool);
            let total_pages = pagination
                .and_then(|pagination| pagination.get("total_pages"))
                .and_then(Value::as_f64);
            if has_next_page == Some(false)
                || total_pages.is_some_and(|total_pages| page as f64 >= total_pages)
            {
                println!(
                    "Stopping seller {user_id} after page {page}; Scrappa reported no additional Vinted pages"
                );
                break;
            }
        }
    }

    Ok(RunSummary {
        pages_fetched,
        saved_items,
        status_message,
    })
}

fn charge_limit_message(
    saved_count: usize,
    requested_count: usize,
    user_id: &str,
    page: u64,
) -> String {
    format!(
        "Charge limit reached after saving {saved_count} of {requested_count} Vinted item(s) for user {user_id} on page {page}."
    )
}

fn charge_idempotency_key(actor_run_id: &str, saved_items: usize) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{actor_run_id}-user-item-result-{saved_items}-{timestamp}")
}

pub fn timeout_status_message(error: &anyhow::Error) -> Option<String> {
    if matches!(
        error.downcast_ref::<ScrappaError>(),
        Some(ScrappaError::Timeout)
    ) {
        Some(format!(
            "{}. The Vinted user-items request exceeded the {}s Scrappa API timeout. Try fewer users, fewer pages, or run the request again.",
            error,
            crate::scrappa::REQUEST_TIMEOUT_MS / 1000
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        collections::VecDeque,
        sync::{Arc, Mutex},
    };
    use tokio::{
        io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
        net::{TcpListener, TcpStream},
        task::JoinHandle,
    };
    use url::Url;

    #[test]
    fn creates_charge_limit_message_with_saved_and_requested_counts() {
        assert_eq!(
            charge_limit_message(1, 24, "123", 2),
            "Charge limit reached after saving 1 of 24 Vinted item(s) for user 123 on page 2."
        );
    }

    #[test]
    fn formats_timeout_help_only_for_scrappa_timeouts() {
        let timeout = anyhow::Error::new(ScrappaError::Timeout);
        assert_eq!(
            timeout_status_message(&timeout).as_deref(),
            Some("Scrappa API request timed out after 90000ms. The Vinted user-items request exceeded the 90s Scrappa API timeout. Try fewer users, fewer pages, or run the request again.")
        );
        assert!(timeout_status_message(&anyhow::anyhow!("validation failed")).is_none());
    }

    #[tokio::test]
    async fn pages_through_a_seller_batch_and_charges_each_saved_listing() {
        let server = MockServer::start(vec![
            json_response(json!({
                "items": [{"id":"111-1","title":"First"}],
                "pagination": {"has_next_page":true,"total_pages":10}
            })),
            json_response(json!({"items":[]})),
            json_response(json!({
                "data": {
                    "items": [{"id":"222-1","title":"Second"}],
                    "pagination": {"has_next_page":false,"total_pages":1}
                }
            })),
        ])
        .await;
        let apify = test_apify_client(&server);
        let scrappa = ScrappaClient::new("scrappa-key", format!("{}/api", server.base_url));
        let input = serde_json::from_value(json!({
            "user_ids": ["111", "222"],
            "country": "DE",
            "per_page": 10,
            "max_pages": 3
        }))
        .unwrap();
        let plan = crate::input::build_plan(&input).unwrap();
        let mut pricing = test_pricing(10.0);

        let summary = run_actor(&apify, &scrappa, &plan, &mut pricing)
            .await
            .unwrap();

        assert_eq!(
            summary,
            RunSummary {
                pages_fetched: 3,
                saved_items: 2,
                status_message: None,
            }
        );
        let requests = server.requests.lock().unwrap();
        let scrappa_requests = requests
            .iter()
            .filter(|request| request.path == "/api/vinted/user-items")
            .collect::<Vec<_>>();
        assert_eq!(scrappa_requests.len(), 3);
        assert_eq!(scrappa_requests[0].query["user_id"], "111");
        assert_eq!(scrappa_requests[1].query["page"], "2");
        assert_eq!(scrappa_requests[2].query["user_id"], "222");

        let dataset_writes = requests
            .iter()
            .filter(|request| request.path == "/v2/datasets/test-dataset/items")
            .collect::<Vec<_>>();
        assert_eq!(dataset_writes.len(), 2);
        assert_eq!(dataset_writes[0].body[0]["input_user_id"], "111");
        assert_eq!(dataset_writes[1].body[0]["request_country"], "DE");
        let charges = requests
            .iter()
            .filter(|request| request.path == "/v2/actor-runs/test-run/charge")
            .collect::<Vec<_>>();
        assert_eq!(charges.len(), 2);
        assert!(charges.iter().all(|request| {
            request.body == json!({"eventName":"user-item-result","count":1})
                && request.headers.contains_key("idempotency-key")
        }));
        assert!(!requests.iter().any(|request| {
            request.method == "PUT" && request.path == "/v2/actor-runs/test-run"
        }));
    }

    #[tokio::test]
    async fn limits_dataset_rows_at_the_ppe_budget_and_marks_the_terminal_status() {
        let server = MockServer::start(vec![
            json_response(json!({
                "items": [{"id":"111-1"},{"id":"111-2"}],
                "pagination": {"has_next_page":true,"total_pages":10}
            })),
            json_response(json!({"items":[{"id":"222-1"}]})),
        ])
        .await;
        let apify = test_apify_client(&server);
        let scrappa = ScrappaClient::new("scrappa-key", format!("{}/api", server.base_url));
        let input = serde_json::from_value(json!({
            "user_ids": ["111", "222"],
            "country": "DE",
            "max_pages": 2
        }))
        .unwrap();
        let plan = crate::input::build_plan(&input).unwrap();
        let mut pricing = test_pricing(0.0015);

        let summary = run_actor(&apify, &scrappa, &plan, &mut pricing)
            .await
            .unwrap();

        assert_eq!(summary.pages_fetched, 1);
        assert_eq!(summary.saved_items, 1);
        assert_eq!(
            summary.status_message.as_deref(),
            Some("Charge limit reached after saving 1 of 2 Vinted item(s) for user 111 on page 1.")
        );
        let requests = server.requests.lock().unwrap();
        let scrappa_requests = requests
            .iter()
            .filter(|request| request.path == "/api/vinted/user-items")
            .collect::<Vec<_>>();
        assert_eq!(scrappa_requests.len(), 1);

        let dataset_writes = requests
            .iter()
            .filter(|request| request.path == "/v2/datasets/test-dataset/items")
            .collect::<Vec<_>>();
        assert_eq!(dataset_writes.len(), 1);
        assert_eq!(dataset_writes[0].body.as_array().unwrap().len(), 1);
        assert_eq!(dataset_writes[0].body[0]["id"], "111-1");
        assert_eq!(dataset_writes[0].body[0]["input_user_id"], "111");
        assert_eq!(dataset_writes[0].body[0]["total_pages"], 10);

        let charges = requests
            .iter()
            .filter(|request| request.path == "/v2/actor-runs/test-run/charge")
            .collect::<Vec<_>>();
        assert_eq!(charges.len(), 1);
        assert_eq!(
            charges[0].body,
            json!({"eventName":"user-item-result","count":1})
        );
        let status_updates = requests
            .iter()
            .filter(|request| request.method == "PUT" && request.path == "/v2/actor-runs/test-run")
            .collect::<Vec<_>>();
        assert_eq!(status_updates.len(), 1);
        assert_eq!(
            status_updates[0].body["statusMessage"],
            summary.status_message.unwrap()
        );
        assert_eq!(status_updates[0].body["isStatusMessageTerminal"], true);
    }

    fn test_apify_client(server: &MockServer) -> ApifyClient {
        ApifyClient::new(crate::apify::ActorConfig {
            apify_token: "apify-token".to_owned(),
            key_value_store_id: "test-store".to_owned(),
            dataset_id: "test-dataset".to_owned(),
            actor_run_id: "test-run".to_owned(),
            input_key: "INPUT".to_owned(),
            scrappa_api_key: "scrappa-key".to_owned(),
            apify_base: server.base_url.clone(),
            scrappa_base: format!("{}/api", server.base_url),
        })
        .unwrap()
    }

    fn test_pricing(max_total_charge_usd: f64) -> ActorPricing {
        ActorPricing::from_run(&json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": {
                        "actorChargeEvents": {
                            "user-item-result": {"eventPriceUsd": 0.001},
                            "apify-default-dataset-item": {"eventPriceUsd": 0.0003},
                            "apify-actor-start": {"eventPriceUsd": 0.0001}
                        }
                    }
                },
                "chargedEventCounts": {"apify-actor-start": 1},
                "options": {"maxTotalChargeUsd": max_total_charge_usd}
            }
        }))
        .unwrap()
    }

    struct MockResponse {
        status: u16,
        body: Value,
    }

    fn json_response(body: Value) -> MockResponse {
        MockResponse { status: 200, body }
    }

    #[derive(Clone, Debug)]
    struct MockRequest {
        method: String,
        path: String,
        query: std::collections::HashMap<String, String>,
        headers: std::collections::HashMap<String, String>,
        body: Value,
    }

    struct MockServer {
        base_url: String,
        requests: Arc<Mutex<Vec<MockRequest>>>,
        task: JoinHandle<()>,
    }

    impl MockServer {
        async fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let base_url = format!("http://{address}");
            let requests = Arc::new(Mutex::new(Vec::new()));
            let recorded_requests = Arc::clone(&requests);
            let queued_responses = Arc::new(Mutex::new(VecDeque::from(responses)));
            let next_response = Arc::clone(&queued_responses);

            let task = tokio::spawn(async move {
                while let Ok((stream, _)) = listener.accept().await {
                    let recorded_requests = Arc::clone(&recorded_requests);
                    let next_response = Arc::clone(&next_response);
                    tokio::spawn(async move {
                        handle_mock_request(stream, recorded_requests, next_response).await;
                    });
                }
            });
            Self {
                base_url,
                requests,
                task,
            }
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn handle_mock_request(
        stream: TcpStream,
        requests: Arc<Mutex<Vec<MockRequest>>>,
        responses: Arc<Mutex<VecDeque<MockResponse>>>,
    ) {
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).await.is_err() || request_line.is_empty() {
            return;
        }
        let mut request_parts = request_line.split_whitespace();
        let method = request_parts.next().unwrap_or_default().to_owned();
        let target = request_parts.next().unwrap_or("/");
        let mut headers = std::collections::HashMap::new();
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).await.is_err() || line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                let name = name.trim().to_ascii_lowercase();
                let value = value.trim().to_owned();
                if name == "content-length" {
                    content_length = value.parse::<usize>().unwrap_or_default();
                }
                headers.insert(name, value);
            }
        }
        let mut body = vec![0; content_length];
        if reader.read_exact(&mut body).await.is_err() {
            return;
        }
        let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
        let url = Url::parse(&format!("http://mock{target}")).unwrap();
        let request = MockRequest {
            method: method.clone(),
            path: url.path().to_owned(),
            query: url.query_pairs().into_owned().collect(),
            headers,
            body,
        };
        requests.lock().unwrap().push(request.clone());

        let response = if request.method == "GET" && request.path == "/api/vinted/user-items" {
            responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| MockResponse {
                    status: 500,
                    body: json!({"message":"No mocked Scrappa response remains"}),
                })
        } else {
            MockResponse {
                status: if request.method == "POST" || request.method == "PUT" {
                    201
                } else {
                    404
                },
                body: json!({}),
            }
        };
        let body = serde_json::to_vec(&response.body).unwrap();
        let reason = match response.status {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            404 => "Not Found",
            _ => "Internal Server Error",
        };
        let response_head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response.status,
            reason,
            body.len()
        );
        let stream = reader.get_mut();
        let _ = stream.write_all(response_head.as_bytes()).await;
        let _ = stream.write_all(&body).await;
        let _ = stream.flush().await;
    }
}
