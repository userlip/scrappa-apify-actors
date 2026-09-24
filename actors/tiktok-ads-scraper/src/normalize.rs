use crate::request_params::js_trim;
use anyhow::{bail, Result};
use serde_json::{Map, Value};
use std::collections::HashSet;

pub fn extract_single_tiktok_ad_record(
    data: Option<&Value>,
    url: &str,
    warn: impl FnOnce(String),
) -> Result<Option<Value>> {
    let Some(data) = data.filter(|value| is_truthy(value)) else {
        return Ok(None);
    };
    let record = match data {
        Value::Array(records) if records.is_empty() => {
            warn(format!(
                "Scrappa returned an empty ad record array for {url}. Saving a not-found dataset item."
            ));
            return Ok(None);
        }
        Value::Array(records) if records.len() > 1 => {
            bail!(
                "Scrappa returned {} ad records for {url}. Expected exactly one ad record for a single Creative Center ad URL.",
                records.len()
            );
        }
        Value::Array(records) => &records[0],
        record => record,
    };
    Ok(is_truthy(record).then(|| record.clone()))
}

pub fn normalize_tiktok_ad_record(ad: &Value) -> Value {
    let mut normalized = object_from_value(ad);
    let advertiser = ad.get("advertiser");
    let stats = ad.get("stats");
    let video_info = ad.get("video_info");

    let video_url = first_string(&[
        ad.get("video_url"),
        ad.get("play_url"),
        ad.get("media_url"),
        video_info.and_then(|value| value.get("play")),
        video_info.and_then(|value| value.get("wmplay")),
    ]);
    let cover_url = first_string(&[
        ad.get("cover"),
        ad.get("cover_url"),
        ad.get("cover_uri"),
        ad.get("image_url"),
        video_info.and_then(|value| value.get("cover")),
    ]);
    let landing_page = first_string(&[ad.get("landing_page_url"), ad.get("destination")]);
    let media_urls = unique_strings([
        video_url.as_deref(),
        cover_url.as_deref(),
        ad.get("media_url").and_then(Value::as_str),
        ad.get("image_url").and_then(Value::as_str),
        video_info
            .and_then(|value| value.get("play"))
            .and_then(Value::as_str),
        video_info
            .and_then(|value| value.get("wmplay"))
            .and_then(Value::as_str),
        video_info
            .and_then(|value| value.get("cover"))
            .and_then(Value::as_str),
    ]);

    set_optional_string(
        &mut normalized,
        "ad_id",
        first_non_null(&[ad.get("ad_id"), ad.get("id")]).and_then(js_string),
    );
    set_optional_string(
        &mut normalized,
        "advertiser_id",
        first_non_null(&[
            ad.get("advertiser_id"),
            advertiser.and_then(|value| value.get("advertiser_id")),
            advertiser.and_then(|value| value.get("id")),
        ])
        .and_then(js_string),
    );
    set_optional_string(
        &mut normalized,
        "account_id",
        first_non_null(&[
            ad.get("account_id"),
            advertiser.and_then(|value| value.get("account_id")),
        ])
        .and_then(js_string),
    );
    set_optional_string(
        &mut normalized,
        "advertiser_name",
        first_string(&[
            ad.get("advertiser_name"),
            ad.get("brand_name"),
            advertiser.and_then(|value| value.get("brand_name")),
            advertiser.and_then(|value| value.get("name")),
        ]),
    );
    set_optional_string(
        &mut normalized,
        "account_name",
        first_string(&[
            ad.get("account_name"),
            advertiser.and_then(|value| value.get("account_name")),
            advertiser.and_then(|value| value.get("name")),
        ]),
    );
    set_optional_string(
        &mut normalized,
        "creative_text",
        first_string(&[
            ad.get("creative_text"),
            ad.get("ad_text"),
            ad.get("description"),
            ad.get("title"),
        ]),
    );
    set_optional_string(&mut normalized, "landing_page", landing_page);
    set_optional_string(&mut normalized, "video_url", video_url);
    set_optional_string(&mut normalized, "cover", cover_url);
    normalized.insert(
        "media_urls".to_owned(),
        Value::Array(media_urls.into_iter().map(Value::String).collect()),
    );
    set_optional_value(
        &mut normalized,
        "like_count",
        first_non_null(&[
            ad.get("like_count"),
            ad.get("like"),
            stats.and_then(|value| value.get("like_count")),
        ]),
    );
    set_optional_value(
        &mut normalized,
        "comment_count",
        first_non_null(&[
            ad.get("comment_count"),
            ad.get("comment"),
            stats.and_then(|value| value.get("comment_count")),
        ]),
    );
    set_optional_value(
        &mut normalized,
        "share_count",
        first_non_null(&[
            ad.get("share_count"),
            ad.get("share"),
            stats.and_then(|value| value.get("share_count")),
        ]),
    );

    Value::Object(normalized)
}

fn object_from_value(value: &Value) -> Map<String, Value> {
    match value {
        Value::Object(fields) => fields.clone(),
        Value::Array(values) => values
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect(),
        Value::String(value) => value
            .chars()
            .enumerate()
            .map(|(index, character)| (index.to_string(), Value::String(character.to_string())))
            .collect(),
        Value::Null | Value::Bool(_) | Value::Number(_) => Map::new(),
    }
}

fn first_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().find_map(|value| match value {
        Some(Value::String(value)) if !js_trim(value).is_empty() => Some(value.clone()),
        _ => None,
    })
}

fn first_non_null<'a>(values: &[Option<&'a Value>]) -> Option<&'a Value> {
    values
        .iter()
        .find_map(|value| value.filter(|value| !value.is_null()))
}

fn js_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(value) => Some(value.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => {
            let string = value.to_string();
            Some(string.strip_suffix(".0").unwrap_or(&string).to_owned())
        }
        Value::Array(values) => Some(
            values
                .iter()
                .map(|value| match value {
                    Value::Null => String::new(),
                    _ => js_string(value).unwrap_or_default(),
                })
                .collect::<Vec<_>>()
                .join(","),
        ),
        Value::Object(_) => Some("[object Object]".to_owned()),
    }
}

fn unique_strings<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .flatten()
        .filter(|value| !js_trim(value).is_empty())
        .filter(|value| seen.insert((*value).to_owned()))
        .map(str::to_owned)
        .collect()
}

fn set_optional_string(fields: &mut Map<String, Value>, key: &str, value: Option<String>) {
    match value {
        Some(value) => {
            fields.insert(key.to_owned(), Value::String(value));
        }
        None => {
            fields.remove(key);
        }
    }
}

fn set_optional_value(fields: &mut Map<String, Value>, key: &str, value: Option<&Value>) {
    match value {
        Some(value) => {
            fields.insert(key.to_owned(), value.clone());
        }
        None => {
            fields.remove(key);
        }
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_single_tiktok_ad_record, normalize_tiktok_ad_record};
    use serde_json::{json, Value};

    const URL: &str =
        "https://ads.tiktok.com/business/creativecenter/topads/7213160569871581185/pc/en";

    #[test]
    fn extracts_object_and_single_record_arrays() {
        let ad = json!({ "id": "7213160569871581185" });
        assert_eq!(
            extract_single_tiktok_ad_record(Some(&ad), URL, |_| {}).unwrap(),
            Some(ad.clone())
        );
        assert_eq!(
            extract_single_tiktok_ad_record(Some(&json!([ad.clone()])), URL, |_| {}).unwrap(),
            Some(ad)
        );
    }

    #[test]
    fn empty_array_is_not_found_and_warns() {
        let mut warning = None;
        assert_eq!(
            extract_single_tiktok_ad_record(Some(&json!([])), URL, |message| {
                warning = Some(message)
            })
            .unwrap(),
            None
        );
        assert!(warning.unwrap().contains("empty ad record array"));
    }

    #[test]
    fn multiple_records_are_an_error_for_a_single_url() {
        let error = extract_single_tiktok_ad_record(
            Some(&json!([{ "id": "one" }, { "id": "two" }])),
            URL,
            |_| {},
        )
        .unwrap_err();
        assert!(error.to_string().contains("Expected exactly one ad record"));
    }

    #[test]
    fn normalizes_nested_ad_fields_and_preserves_raw_fields() {
        let normalized = normalize_tiktok_ad_record(&json!({
            "id": "7221117041168252930",
            "brand_name": "Example brand",
            "title": "Example ad title",
            "destination": "https://example.com",
            "play_url": "https://cdn.example.com/video.mp4",
            "cover_url": "https://cdn.example.com/cover.jpg",
            "stats": { "like_count": 1200, "comment_count": 42, "share_count": 18 },
            "advertiser": { "id": 123, "account_id": 456, "name": "Example advertiser" },
            "raw_extension": { "kept": true }
        }));
        assert_eq!(normalized["ad_id"], "7221117041168252930");
        assert_eq!(normalized["advertiser_id"], "123");
        assert_eq!(normalized["account_id"], "456");
        assert_eq!(normalized["advertiser_name"], "Example brand");
        assert_eq!(normalized["account_name"], "Example advertiser");
        assert_eq!(normalized["creative_text"], "Example ad title");
        assert_eq!(normalized["landing_page"], "https://example.com");
        assert_eq!(normalized["video_url"], "https://cdn.example.com/video.mp4");
        assert_eq!(normalized["cover"], "https://cdn.example.com/cover.jpg");
        assert_eq!(normalized["like_count"], 1200);
        assert_eq!(normalized["comment_count"], 42);
        assert_eq!(normalized["share_count"], 18);
        assert_eq!(normalized["raw_extension"], json!({ "kept": true }));
        assert_eq!(
            normalized["media_urls"],
            json!([
                "https://cdn.example.com/video.mp4",
                "https://cdn.example.com/cover.jpg"
            ])
        );
    }

    #[test]
    fn keeps_existing_values_and_handles_live_response_fields() {
        let normalized = normalize_tiktok_ad_record(&json!({
            "ad_id": "existing",
            "advertiser_id": "adv_existing",
            "account_id": "acct_existing",
            "advertiser_name": "Existing advertiser",
            "account_name": "Existing account",
            "creative_text": "Existing creative",
            "landing_page_url": "https://landing.example.com",
            "video_url": "https://video.example.com/video.mp4",
            "cover": "https://video.example.com/cover.jpg",
            "like_count": 9,
            "stats": { "like_count": 1200 },
            "advertiser": { "id": 123, "name": "Nested advertiser" }
        }));
        assert_eq!(normalized["ad_id"], "existing");
        assert_eq!(normalized["advertiser_id"], "adv_existing");
        assert_eq!(normalized["account_id"], "acct_existing");
        assert_eq!(normalized["advertiser_name"], "Existing advertiser");
        assert_eq!(normalized["account_name"], "Existing account");
        assert_eq!(normalized["creative_text"], "Existing creative");
        assert_eq!(normalized["landing_page"], "https://landing.example.com");
        assert_eq!(normalized["like_count"], 9);

        let live = normalize_tiktok_ad_record(&json!({
            "id": "7543186103350427655",
            "brand_name": "TikTok",
            "title": "Promote your TikTok now!",
            "cover_uri": "https://cdn.example.com/cover.jpg",
            "like": 5215200,
            "comment": 1277551,
            "share": 1388339,
            "video_info": {
                "play": "https://cdn.example.com/play.mp4",
                "wmplay": "https://cdn.example.com/wmplay.mp4",
                "cover": "https://cdn.example.com/cover.jpg"
            }
        }));
        assert_eq!(live["ad_id"], "7543186103350427655");
        assert_eq!(live["video_url"], "https://cdn.example.com/play.mp4");
        assert_eq!(live["like_count"], 5215200);
        assert_eq!(
            live["media_urls"],
            json!([
                "https://cdn.example.com/play.mp4",
                "https://cdn.example.com/cover.jpg",
                "https://cdn.example.com/wmplay.mp4"
            ])
        );
    }

    #[test]
    fn normalizes_sparse_records_without_null_optional_fields() {
        let normalized = normalize_tiktok_ad_record(&json!({
            "media_url": "https://cdn.example.com/video.mp4",
            "video_url": "https://cdn.example.com/video.mp4",
            "ad_id": null
        }));
        assert_eq!(normalized["video_url"], "https://cdn.example.com/video.mp4");
        assert_eq!(
            normalized["media_urls"],
            json!(["https://cdn.example.com/video.mp4"])
        );
        assert!(normalized.get("ad_id").is_none());
        assert!(matches!(normalized, Value::Object(_)));
    }
}
