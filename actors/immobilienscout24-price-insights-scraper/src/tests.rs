use super::*;
use crate::test_support::{CapturedRequest, MockResponse, MockServer};
use serde_json::{json, Value};
use url::Url;

const API_TOKEN: &str = "test-apify-token";
const SCRAPPA_KEY: &str = "test-scrappa-key";
const RESULT_EVENT: &str = "price-insight-result";

struct Fixture {
    input: Value,
    ppe: bool,
    max_charge: f64,
    charged_counts: Value,
    failed_locations: Vec<String>,
    incomplete_locations: Vec<String>,
}

impl Fixture {
    fn new(input: Value) -> Self {
        Self {
            input,
            ppe: true,
            max_charge: 1.0,
            charged_counts: json!({}),
            failed_locations: Vec::new(),
            incomplete_locations: Vec::new(),
        }
    }

    fn run_response(&self) -> Value {
        let pricing_info = if self.ppe {
            json!({
                "pricingModel": "PAY_PER_EVENT",
                "pricingPerEvent": {
                    "actorChargeEvents": {
                        "price-insight-result": { "eventPriceUsd": 0.0005 }
                    }
                }
            })
        } else {
            json!({"pricingModel": "FREE"})
        };
        json!({
            "data": {
                "pricingInfo": pricing_info,
                "chargedEventCounts": self.charged_counts,
                "options": {"maxTotalChargeUsd": self.max_charge}
            }
        })
    }

    fn start(self) -> (MockServer, ApifyClient, ScrappaClient) {
        let input = self.input.clone();
        let run = self.run_response();
        let failures = self.failed_locations.clone();
        let incomplete = self.incomplete_locations.clone();
        let server = MockServer::start(move |request| {
            let path = request.target.split('?').next().unwrap_or_default();
            match (request.method.as_str(), path) {
                ("GET", "/v2/key-value-stores/test-store/records/INPUT") => {
                    MockResponse::json(200, input.clone())
                }
                ("GET", "/v2/actor-runs/test-run") => MockResponse::json(200, run.clone()),
                ("GET", "/api/immobilienscout24/price-insights") => {
                    let location = Url::parse(&format!("http://mock{}", request.target))
                        .unwrap()
                        .query_pairs()
                        .find(|(key, _)| key == "location")
                        .map(|(_, value)| value.into_owned())
                        .unwrap_or_default();
                    if failures.contains(&location) {
                        MockResponse::json(404, json!({"message": "Location not found"}))
                    } else if incomplete.contains(&location) {
                        MockResponse::json(200, json!({"success": true, "prices": null}))
                    } else {
                        MockResponse::json(200, price_response(&location))
                    }
                }
                ("POST", "/v2/datasets/test-dataset/items") => MockResponse::text(201, "{}"),
                ("POST", "/v2/actor-runs/test-run/charge") => MockResponse::json(200, json!({})),
                ("PUT", "/v2/actor-runs/test-run") => MockResponse::json(200, json!({})),
                _ => MockResponse::json(500, json!({"message": "unexpected request"})),
            }
        });
        let base_url = server.base_url.as_str().trim_end_matches('/').to_owned();
        let apify = ApifyClient::new(
            &base_url,
            API_TOKEN.to_owned(),
            "test-store".to_owned(),
            "test-dataset".to_owned(),
            "test-run".to_owned(),
            "INPUT".to_owned(),
        )
        .unwrap();
        let scrappa_base = Url::parse(&format!("{base_url}/api")).unwrap();
        let scrappa = ScrappaClient::new(
            SCRAPPA_KEY.to_owned(),
            scrappa_base,
            SCRAPPA_REQUEST_TIMEOUT,
        );
        (server, apify, scrappa)
    }
}

fn price_response(location: &str) -> Value {
    json!({
        "success": true,
        "location": location,
        "geocode": "1276003001",
        "currency": "EUR",
        "prices": {
            "apartment_rent_per_m2": 12.72,
            "apartment_buy_per_m2": 4189.04,
            "house_rent_per_m2": 16.51,
            "house_buy_per_m2": 4394.87
        }
    })
}

fn matching<'a>(
    requests: &'a [CapturedRequest],
    method: &str,
    path: &str,
) -> Vec<&'a CapturedRequest> {
    requests
        .iter()
        .filter(|request| {
            request.method == method && request.target.split('?').next() == Some(path)
        })
        .collect()
}

#[tokio::test]
async fn reads_the_prefilled_input_and_preserves_auth_and_dataset_output() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    assert_eq!(
        schema["properties"]["locations"]["prefill"],
        json!(["Berlin"])
    );

    let mut fixture = Fixture::new(json!({"locations": ["Berlin"]}));
    fixture.max_charge = 0.001;
    let (server, apify, scrappa) = fixture.start();
    let status = run_actor_with_clients(&apify, &scrappa).await.unwrap();
    apify.set_terminal_status(&status).await.unwrap();

    assert_eq!(
        status,
        "Saved 1 of 1 requested location snapshot(s); 0 failed."
    );
    let requests = server.requests();
    let input = matching(
        &requests,
        "GET",
        "/v2/key-value-stores/test-store/records/INPUT",
    );
    assert_eq!(input.len(), 1);
    assert_eq!(
        input[0].header("authorization"),
        Some("Bearer test-apify-token")
    );

    let scrappa_calls = matching(&requests, "GET", "/api/immobilienscout24/price-insights");
    assert_eq!(scrappa_calls.len(), 1);
    assert!(scrappa_calls[0].target.ends_with("?location=Berlin"));
    assert_eq!(scrappa_calls[0].header("x-api-key"), Some(SCRAPPA_KEY));
    assert_eq!(scrappa_calls[0].header("authorization"), None);
    assert_eq!(
        scrappa_calls[0].header("user-agent"),
        Some("thescrappa-immobilienscout24-price-insights-scraper/1.0")
    );

    let dataset_writes = matching(&requests, "POST", "/v2/datasets/test-dataset/items");
    assert_eq!(dataset_writes.len(), 1);
    assert_eq!(
        dataset_writes[0].header("authorization"),
        Some("Bearer test-apify-token")
    );
    assert_eq!(
        dataset_writes[0].json_body(),
        json!([{
            "location": "Berlin",
            "geocode": "1276003001",
            "currency": "EUR",
            "apartment_rent_per_m2": 12.72,
            "apartment_buy_per_m2": 4189.04,
            "house_rent_per_m2": 16.51,
            "house_buy_per_m2": 4394.87,
            "request_location": "Berlin",
            "request_index": 0
        }])
    );
    assert!(matching(
        &requests,
        "PUT",
        "/v2/key-value-stores/test-store/records/OUTPUT"
    )
    .is_empty());

    let charges = matching(&requests, "POST", "/v2/actor-runs/test-run/charge");
    assert_eq!(charges.len(), 1);
    assert_eq!(
        charges[0].json_body(),
        json!({"eventName": RESULT_EVENT, "count": 1})
    );
    assert_eq!(
        charges[0].header("idempotency-key"),
        Some("test-run-price-insight-result-0")
    );

    let status_updates = matching(&requests, "PUT", "/v2/actor-runs/test-run");
    assert_eq!(status_updates.len(), 1);
    assert_eq!(status_updates[0].json_body()["statusMessage"], status);
    assert_eq!(
        status_updates[0].json_body()["isStatusMessageTerminal"],
        true
    );
}

#[tokio::test]
async fn keeps_locations_ordered_and_continues_after_upstream_errors() {
    let mut fixture = Fixture::new(json!({"locations": ["Berlin", "Nowhere", "Munich"]}));
    fixture.max_charge = 0.01;
    fixture.failed_locations = vec!["Nowhere".to_owned()];
    let (server, apify, scrappa) = fixture.start();

    let status = run_actor_with_clients(&apify, &scrappa).await.unwrap();

    assert_eq!(
        status,
        "Saved 2 of 3 requested location snapshot(s); 1 failed."
    );
    let requests = server.requests();
    let dataset_writes = matching(&requests, "POST", "/v2/datasets/test-dataset/items");
    assert_eq!(dataset_writes.len(), 2);
    assert_eq!(
        dataset_writes[0].json_body()[0]["request_location"],
        "Berlin"
    );
    assert_eq!(
        dataset_writes[1].json_body()[0]["request_location"],
        "Munich"
    );
    assert_eq!(dataset_writes[0].json_body()[0]["request_index"], 0);
    assert_eq!(dataset_writes[1].json_body()[0]["request_index"], 2);
    assert_eq!(
        matching(&requests, "POST", "/v2/actor-runs/test-run/charge").len(),
        2
    );
}

#[tokio::test]
async fn stops_dataset_writes_when_the_last_charge_uses_the_run_budget() {
    let mut fixture = Fixture::new(json!({"locations": ["Berlin", "Munich"]}));
    fixture.max_charge = 0.0005;
    let (server, apify, scrappa) = fixture.start();

    let status = run_actor_with_clients(&apify, &scrappa).await.unwrap();

    assert_eq!(
        status,
        "Charge limit reached after 1 successful location snapshot(s)."
    );
    let requests = server.requests();
    assert_eq!(
        matching(&requests, "GET", "/api/immobilienscout24/price-insights").len(),
        2
    );
    assert_eq!(
        matching(&requests, "POST", "/v2/datasets/test-dataset/items").len(),
        1
    );
    assert_eq!(
        matching(&requests, "POST", "/v2/actor-runs/test-run/charge").len(),
        1
    );
}

#[tokio::test]
async fn exhausted_budget_returns_charge_limit_without_writing_or_charging() {
    let mut fixture = Fixture::new(json!({"location": "Berlin"}));
    fixture.max_charge = 0.0;
    let (server, apify, scrappa) = fixture.start();

    let status = run_actor_with_clients(&apify, &scrappa).await.unwrap();

    assert_eq!(
        status,
        "Charge limit reached after 0 successful location snapshot(s)."
    );
    let requests = server.requests();
    assert_eq!(
        matching(&requests, "GET", "/api/immobilienscout24/price-insights").len(),
        1
    );
    assert!(matching(&requests, "POST", "/v2/datasets/test-dataset/items").is_empty());
    assert!(matching(&requests, "POST", "/v2/actor-runs/test-run/charge").is_empty());
}

#[tokio::test]
async fn continues_without_custom_event_charges_on_non_ppe_runs() {
    let mut fixture = Fixture::new(json!({"locations": ["Berlin", "Munich"]}));
    fixture.ppe = false;
    let (server, apify, scrappa) = fixture.start();

    let status = run_actor_with_clients(&apify, &scrappa).await.unwrap();

    assert_eq!(
        status,
        "Saved 2 of 2 requested location snapshot(s); 0 failed."
    );
    let requests = server.requests();
    assert_eq!(
        matching(&requests, "POST", "/v2/datasets/test-dataset/items").len(),
        2
    );
    assert!(matching(&requests, "POST", "/v2/actor-runs/test-run/charge").is_empty());
}

#[tokio::test]
async fn fails_the_actor_when_no_location_resolves_to_a_complete_snapshot() {
    let mut fixture = Fixture::new(json!({"locations": ["Nowhere"]}));
    fixture.ppe = false;
    fixture.failed_locations = vec!["Nowhere".to_owned()];
    let (server, apify, scrappa) = fixture.start();

    let error = run_actor_with_clients(&apify, &scrappa).await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "No price-insights snapshots were resolved for 1 requested location(s)."
    );
    assert!(matching(
        &server.requests(),
        "POST",
        "/v2/datasets/test-dataset/items"
    )
    .is_empty());
}

#[tokio::test]
async fn maps_incomplete_scrappa_snapshots_to_location_failures() {
    let mut fixture = Fixture::new(json!({"location": "Berlin"}));
    fixture.ppe = false;
    fixture.incomplete_locations = vec!["Berlin".to_owned()];
    let (server, apify, scrappa) = fixture.start();

    let error = run_actor_with_clients(&apify, &scrappa).await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "No price-insights snapshots were resolved for 1 requested location(s)."
    );
    assert!(matching(
        &server.requests(),
        "POST",
        "/v2/datasets/test-dataset/items"
    )
    .is_empty());
}

#[test]
fn schema_keeps_singular_location_compatibility() {
    let schema: Value = serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
    assert_eq!(schema["properties"]["location"]["type"], "string");
    assert_eq!(schema["properties"]["location"]["maxLength"], 120);
}
