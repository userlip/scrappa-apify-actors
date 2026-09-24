use anyhow::{anyhow, bail, Result};
use reqwest::Client;
use serde_json::{Map, Value};

use super::{
    apify::get_run,
    config::{ActorConfig, APIFY_DEFAULT_DATASET_ITEM_EVENT},
    input::js_truthy,
};

pub(super) fn extract_followers(data: Option<&Value>) -> Vec<Value> {
    let Some(data) = data.filter(|data| !data.is_null()) else {
        return Vec::new();
    };

    if let Value::Array(followers) = data {
        return followers.clone();
    }

    for field in ["followers", "users", "user_list"] {
        if let Some(Value::Array(followers)) = data.get(field) {
            return followers.clone();
        }
    }
    Vec::new()
}

pub(super) fn extract_pagination(data: Option<&Value>) -> (bool, Value) {
    let Some(data) = data.filter(|data| !data.is_null() && !data.is_array()) else {
        return (false, Value::Null);
    };

    let has_more = data
        .get("hasMore")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("has_more"))
        .is_some_and(js_truthy);
    let next_time = data
        .get("time")
        .filter(|value| !value.is_null())
        .or_else(|| data.get("min_time").filter(|value| !value.is_null()))
        .or_else(|| data.get("max_time").filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null);
    (has_more, next_time)
}

pub(super) fn follower_item(
    follower: &Value,
    lookup_unique_id: Option<&str>,
    lookup_user_id: &str,
) -> Value {
    let mut item = match follower {
        Value::Object(follower) => follower.clone(),
        Value::Array(follower) => follower
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        Value::String(follower) => follower
            .chars()
            .enumerate()
            .map(|(index, value)| (index.to_string(), Value::String(value.to_string())))
            .collect(),
        _ => Map::new(),
    };
    item.insert(
        "lookup_unique_id".to_owned(),
        lookup_unique_id
            .map(|value| Value::String(value.to_owned()))
            .unwrap_or(Value::Null),
    );
    item.insert(
        "lookup_user_id".to_owned(),
        Value::String(lookup_user_id.to_owned()),
    );
    Value::Object(item)
}

pub(super) fn affordable_dataset_items(run: &Value, requested: usize) -> Result<usize> {
    let data = run
        .get("data")
        .ok_or_else(|| anyhow!("Apify run pricing is missing"))?;
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        return Ok(requested);
    }

    let max_charge = match data.pointer("/options/maxTotalChargeUsd") {
        Some(Value::Number(value)) => value
            .as_f64()
            .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?,
        Some(Value::Null) | None => return Ok(requested),
        Some(_) => bail!("Apify run returned an invalid spending limit"),
    };
    if !max_charge.is_finite() || max_charge < 0.0 {
        bail!("Apify run returned invalid charging values");
    }
    if max_charge == 0.0 {
        return Ok(requested);
    }

    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let item_price = events
        .get(APIFY_DEFAULT_DATASET_ITEM_EVENT)
        .and_then(|event| event.get("eventPriceUsd"))
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run did not provide the dataset item price"))?;
    if !item_price.is_finite() || item_price < 0.0 {
        bail!("Apify run returned invalid charging values");
    }

    let counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide charged event counts"))?;
    let mut spent = 0.0;
    for (event_name, count) in counts {
        let count = count
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid charged event count for {event_name}"))?;
        if count == 0 {
            continue;
        }
        let price = events
            .get(event_name)
            .and_then(|event| event.get("eventPriceUsd"))
            .and_then(Value::as_f64)
            .ok_or_else(|| anyhow!("Missing price for charged event {event_name}"))?;
        if !price.is_finite() || price < 0.0 {
            bail!("Invalid price for charged event {event_name}");
        }
        spent += price * count as f64;
    }
    if !spent.is_finite() {
        bail!("Apify run returned invalid charged totals");
    }
    if item_price == 0.0 {
        return Ok(requested);
    }

    let tolerance = f64::EPSILON * max_charge.max(1.0);
    Ok((1..=requested)
        .take_while(|count| spent + *count as f64 * item_price <= max_charge + tolerance)
        .count())
}

pub(super) async fn run_dataset_capacity(
    client: &Client,
    config: &ActorConfig,
    requested: usize,
) -> Result<usize> {
    let run = get_run(client, config).await?;
    affordable_dataset_items(&run, requested)
}
