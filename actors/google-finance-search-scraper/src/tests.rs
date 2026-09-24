use anyhow::anyhow;
use serde_json::json;

use crate::{
    charging::ChargingManager,
    input::{build_search_requests, GoogleFinanceSearchRequest, MAX_QUERIES_PER_RUN},
    response::{build_dataset_items, count_search_results},
    scrappa::{
        get_retry_delay_ms, is_retryable_scrappa_error, looks_like_network_error,
        parse_scrappa_error_body, ScrappaFailure,
    },
    status::build_transient_failure_status_message,
};

#[test]
fn builds_single_and_batch_requests_with_compatibility_priority() {
    let single = build_search_requests(&json!({"q":" AAPL ","hl":"EN","gl":"US"})).unwrap();
    assert_eq!(
        single,
        vec![GoogleFinanceSearchRequest {
            q: "AAPL".to_owned(),
            hl: Some("en".to_owned()),
            gl: Some("us".to_owned()),
        }]
    );

    let batch = build_search_requests(&json!({
        "q": "ignored",
        "queries": [" Tesla ", "MSFT"],
        "hl": "en"
    }))
    .unwrap();
    assert_eq!(
        batch
            .iter()
            .map(|query| query.q.as_str())
            .collect::<Vec<_>>(),
        ["Tesla", "MSFT"]
    );
    assert!(batch.iter().all(|query| query.gl.is_none()));
}

#[test]
fn rejects_invalid_queries_and_locale_codes() {
    assert_eq!(
        build_search_requests(&json!({})).unwrap_err().to_string(),
        "q is required"
    );
    assert_eq!(
        build_search_requests(&json!({"queries":[]}))
            .unwrap_err()
            .to_string(),
        "queries must include at least one query"
    );
    assert_eq!(
        build_search_requests(&json!({"queries":["AAPL",123]}))
            .unwrap_err()
            .to_string(),
        "queries[1] must be a string"
    );
    assert_eq!(
        build_search_requests(&json!({"q":"AAPL","hl":"english"}))
            .unwrap_err()
            .to_string(),
        "hl must be a two-letter language code with an optional two-letter region"
    );
    assert_eq!(
        build_search_requests(&json!({"q":"AAPL","gl":"usa"}))
            .unwrap_err()
            .to_string(),
        "gl must be a two-letter country code"
    );
    assert!(build_search_requests(&json!({"q":"x".repeat(256)}))
        .unwrap_err()
        .to_string()
        .contains("255 characters or fewer"));
}

#[test]
fn enforces_batch_cap_and_javascript_utf16_query_length() {
    let too_many = (0..=MAX_QUERIES_PER_RUN)
        .map(|index| json!(format!("query-{index}")))
        .collect::<Vec<_>>();
    assert!(build_search_requests(&json!({"queries":too_many}))
        .unwrap_err()
        .to_string()
        .contains("at most 25"));
    assert!(build_search_requests(&json!({"q":"😀".repeat(128)}))
        .unwrap_err()
        .to_string()
        .contains("255 characters or fewer"));
}

#[test]
fn builds_finance_result_fields_and_google_url() {
    let params = GoogleFinanceSearchRequest {
        q: "AAPL".to_owned(),
        hl: Some("en".to_owned()),
        gl: Some("us".to_owned()),
    };
    let response = json!({"results":[{
        "stock":"AAPL:NASDAQ", "name":"Apple Inc", "symbol":"AAPL", "exchange":"NASDAQ",
        "type":"Stock", "currency":"USD", "price":"1,176.85",
        "price_movement":{"value":"2.34","percentage":"1.33"}
    }]});
    let items = build_dataset_items(&response, &params);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["query"], "AAPL");
    assert_eq!(items[0]["position"], 1);
    assert_eq!(items[0]["name"], "Apple Inc");
    assert_eq!(items[0]["price"], 1176.85);
    assert_eq!(items[0]["price_change"], 2.34);
    assert_eq!(items[0]["percent_change"], 1.33);
    assert_eq!(
        items[0]["google_finance_url"],
        "https://www.google.com/finance/quote/AAPL%3ANASDAQ"
    );
    assert_eq!(items[0]["raw_result"], response["results"][0]);
}

#[test]
fn finds_alternate_wrappers_and_uses_fallback_fields() {
    let params = GoogleFinanceSearchRequest {
        q: "Tesla".to_owned(),
        hl: None,
        gl: None,
    };
    let response = json!({"results":[],"data":{"search_results":[{
        "title":"Tesla Inc", "symbol":"TSLA", "exchange":"NASDAQ", "change":"4.5",
        "change_percent":"1.25", "url":"https://example.test/tesla", "region":"US"
    }]}});
    assert_eq!(count_search_results(&response), 1);
    let item = &build_dataset_items(&response, &params)[0];
    assert_eq!(item["name"], "Tesla Inc");
    assert_eq!(item["price_change"], 4.5);
    assert_eq!(item["percent_change"], 1.25);
    assert_eq!(item["link"], "https://example.test/tesla");
    assert_eq!(item["market"], "US");
    assert!(item["request_hl"].is_null());
}

#[test]
fn empty_or_unrecognized_search_payloads_produce_no_items() {
    let params = GoogleFinanceSearchRequest {
        q: "missing".to_owned(),
        hl: None,
        gl: None,
    };
    assert_eq!(count_search_results(&json!({"results":[]})), 0);
    assert!(build_dataset_items(&json!({}), &params).is_empty());
}

#[test]
fn parses_scrappa_error_messages_and_fallback_text() {
    assert_eq!(
        parse_scrappa_error_body(
            r#"{"message":"Invalid input","errors":{"q":["required","too short"]}}"#,
            "Bad Request"
        ),
        "Invalid input - q: required, too short"
    );
    assert_eq!(
        parse_scrappa_error_body(" upstream   is down ", "Service Unavailable"),
        "upstream is down"
    );
    assert_eq!(parse_scrappa_error_body("", "Not Found"), "Not Found");
}

#[test]
fn retries_only_transient_scrappa_errors_with_bounded_backoff() {
    for status in [408, 429, 500, 502, 503, 504] {
        let error = anyhow!(ScrappaFailure::Http {
            status,
            details: "failure".to_owned()
        });
        assert!(is_retryable_scrappa_error(&error));
    }
    let not_found = anyhow!(ScrappaFailure::Http {
        status: 404,
        details: "not found".to_owned()
    });
    assert!(!is_retryable_scrappa_error(&not_found));
    assert!(looks_like_network_error("read ECONNRESET"));
    assert!(!looks_like_network_error("invalid JSON response"));
    assert_eq!(get_retry_delay_ms(1, 0), 2000);
    assert_eq!(get_retry_delay_ms(20, 500), 10000);
}

#[test]
fn accounts_for_existing_and_local_pay_per_event_charges() {
    let run = json!({"data":{
        "pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{
            "finance-search-result":{"eventPriceUsd":0.05},
            "apify-default-dataset-item":{"eventPriceUsd":0.01},
            "other":{"eventPriceUsd":0.01}
        }}},
        "chargedEventCounts":{"other":2},
        "options":{"maxTotalChargeUsd":0.13}
    }});
    let mut charging = ChargingManager::from_run(&run).unwrap();
    assert_eq!(charging.max_items_within_budget(10).unwrap(), 1);
    charging.record_saved_items(1).unwrap();
    assert_eq!(charging.max_items_within_budget(10).unwrap(), 0);
}

#[test]
fn free_pricing_keeps_all_results_and_zero_price_events_are_unlimited() {
    let free = ChargingManager::from_run(&json!({"data":{"pricingInfo":{"pricingModel":"FREE"}}}))
        .unwrap();
    assert_eq!(free.max_items_within_budget(10).unwrap(), 10);
    let no_charge = ChargingManager::from_run(&json!({"data":{
        "pricingInfo":{"pricingModel":"PAY_PER_EVENT","pricingPerEvent":{"actorChargeEvents":{
            "finance-search-result":{"eventPriceUsd":0.0}
        }}},
        "chargedEventCounts":{},"options":{"maxTotalChargeUsd":0.0}
    }}))
    .unwrap();
    assert_eq!(no_charge.max_items_within_budget(10).unwrap(), 10);
}

#[test]
fn reports_partial_results_and_batch_failures_clearly() {
    assert_eq!(
        build_transient_failure_status_message("Scrappa upstream returned 503 after retries", 0, 1),
        "Scrappa upstream returned 503 after retries; no Google Finance search results were written or charged. Try the run again later."
    );
    assert!(
        build_transient_failure_status_message("failure", 7, 2).contains(
            "7 Google Finance search results were already written and may have been charged"
        )
    );
    assert!(build_transient_failure_status_message("failure", 0, 2).contains("no Google Finance search results were written or charged. Remaining batch queries were not completed"));
}
