use serde_json::{json, Map, Value};

use crate::input::InterestParams;

fn timeline_points(value: Option<&Value>) -> Vec<&Map<String, Value>> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .collect()
}

fn get_timeline_points(response: &Value) -> Vec<&Map<String, Value>> {
    let primary = timeline_points(response.get("timeline_data"));
    if !primary.is_empty() {
        return primary;
    }
    timeline_points(
        response
            .get("interest_over_time")
            .and_then(|value| value.get("data_points")),
    )
}

pub fn build_timeline_dataset_items(response: &Value, params: &InterestParams) -> Vec<Value> {
    let points = get_timeline_points(response);
    let interest = response
        .get("interest_over_time")
        .and_then(Value::as_object);
    let search_parameters = response
        .get("search_parameters")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or(Value::Null);

    points
        .into_iter()
        .enumerate()
        .map(|(index, point)| {
            let mut item = point.clone();
            item.insert("position".to_owned(), json!(index + 1));
            item.insert(
                "timestamp".to_owned(),
                point.get("timestamp").cloned().unwrap_or(Value::Null),
            );
            item.insert(
                "date".to_owned(),
                point.get("date").cloned().unwrap_or(Value::Null),
            );
            item.insert(
                "value".to_owned(),
                point.get("value").cloned().unwrap_or(Value::Null),
            );
            for field in ["average", "max_value", "min_value"] {
                item.insert(
                    field.to_owned(),
                    interest
                        .and_then(|summary| summary.get(field))
                        .cloned()
                        .unwrap_or(Value::Null),
                );
            }
            for (field, param) in [
                ("request_q", "q"),
                ("request_geo", "geo"),
                ("request_time_range", "time_range"),
                ("request_hl", "hl"),
                ("request_search_type", "search_type"),
            ] {
                item.insert(
                    field.to_owned(),
                    params
                        .value(param)
                        .map_or(Value::Null, |value| json!(value)),
                );
            }
            item.insert(
                "response_time_ms".to_owned(),
                response
                    .get("response_time_ms")
                    .cloned()
                    .filter(|value| !value.is_null())
                    .unwrap_or(Value::Null),
            );
            item.insert("search_parameters".to_owned(), search_parameters.clone());
            Value::Object(item)
        })
        .collect()
}
