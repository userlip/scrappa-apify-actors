use anyhow::{anyhow, Result};
use serde_json::{Map, Value};

use crate::request_params::VintedUserProfileRequest;

pub fn get_vinted_user_profile(response: &Value) -> Result<&Map<String, Value>> {
    if response.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(anyhow!(response_failure_message(response)));
    }

    let profile = response
        .get("user")
        .and_then(profile_candidate)
        .or_else(|| response.pointer("/data/user").and_then(profile_candidate))
        .or_else(|| response.get("data").and_then(profile_candidate))
        .or_else(|| profile_candidate(response))
        .ok_or_else(|| anyhow!("Scrappa response did not include Vinted user profile details"))?;

    if profile.get("can_view_profile").and_then(Value::as_bool) == Some(false) {
        return Err(anyhow!("Vinted user profile is private or unavailable"));
    }
    if profile.get("is_account_banned").and_then(Value::as_bool) == Some(true) {
        return Err(anyhow!("Vinted user profile is banned or unavailable"));
    }
    if !is_resolved_profile(profile) {
        return Err(anyhow!(
            "Scrappa response included an incomplete Vinted user profile"
        ));
    }

    Ok(profile)
}

fn profile_candidate(value: &Value) -> Option<&Map<String, Value>> {
    let object = value.as_object()?;
    [
        "id",
        "login",
        "feedback_count",
        "profile_url",
        "can_view_profile",
    ]
    .iter()
    .any(|field| object.contains_key(*field))
    .then_some(object)
}

fn response_failure_message(response: &Value) -> String {
    let message = response
        .get("message")
        .and_then(non_empty_string)
        .or_else(|| response.pointer("/data/message").and_then(non_empty_string))
        .unwrap_or_else(|| "Scrappa response reported failure".to_owned());
    let status = response
        .get("status_code")
        .filter(|value| !value.is_null())
        .map(|value| format!(" (status_code: {value})"))
        .unwrap_or_default();

    format!("{message}{status}")
}

fn is_resolved_profile(profile: &Map<String, Value>) -> bool {
    has_valid_profile_identity(profile.get("id"))
        && profile.get("login").and_then(non_empty_string).is_some()
        && (profile
            .get("profile_url")
            .and_then(non_empty_string)
            .is_some()
            || profile.get("path").and_then(non_empty_string).is_some())
}

fn has_valid_profile_identity(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Number(number)) => {
            if let Some(integer) = number.as_u64() {
                integer > 0 && integer <= 9_007_199_254_740_991
            } else {
                number.as_f64().is_some_and(|number| {
                    number.is_finite()
                        && number > 0.0
                        && number.fract() == 0.0
                        && number <= 9_007_199_254_740_991_f64
                })
            }
        }
        Some(Value::String(value)) => {
            let value = value.trim();
            !value.is_empty()
                && value
                    .bytes()
                    .enumerate()
                    .all(|(index, byte)| byte.is_ascii_digit() && (index != 0 || byte != b'0'))
        }
        _ => false,
    }
}

fn non_empty_string(value: &Value) -> Option<String> {
    let value = value.as_str()?.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn get_or_null<'a>(object: &'a Map<String, Value>, field: &str) -> Value {
    object.get(field).cloned().unwrap_or(Value::Null)
}

fn first_non_null<'a>(object: &'a Map<String, Value>, first: &str, second: &str) -> Value {
    object
        .get(first)
        .filter(|value| !value.is_null())
        .or_else(|| object.get(second).filter(|value| !value.is_null()))
        .cloned()
        .unwrap_or(Value::Null)
}

fn verification_valid(verification: Option<&Value>) -> Value {
    verification
        .and_then(Value::as_object)
        .and_then(|verification| verification.get("valid"))
        .filter(|valid| matches!(valid, Value::Bool(_)))
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn build_vinted_user_profile_dataset_item(
    profile: &Map<String, Value>,
    request: &VintedUserProfileRequest,
    response: &Value,
) -> Value {
    let mut item = profile.clone();
    let bundle_discount = profile
        .get("bundle_discount")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or(Value::Null);
    let bundle_discount_object = bundle_discount.as_object();
    let verification = profile
        .get("verification")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    let meta = response
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let country_code = first_non_null(profile, "country_code", "country_iso_code");
    let last_activity = first_non_null(profile, "last_loged_on_ts", "last_loged_on");

    item.insert("id".into(), get_or_null(profile, "id"));
    item.insert("login".into(), get_or_null(profile, "login"));
    item.insert("country_code".into(), country_code.clone());
    item.insert(
        "country_iso_code".into(),
        first_non_null(profile, "country_iso_code", "country_code"),
    );
    item.insert(
        "country_title".into(),
        first_non_null(profile, "country_title", "country_title_local"),
    );
    item.insert("city".into(), get_or_null(profile, "city"));
    for field in [
        "feedback_count",
        "feedback_reputation",
        "positive_feedback_count",
        "neutral_feedback_count",
        "negative_feedback_count",
    ] {
        item.insert(field.into(), get_or_null(profile, field));
    }
    item.insert("bundle_discount".into(), bundle_discount.clone());
    item.insert(
        "bundle_discount_enabled".into(),
        bundle_discount_object
            .and_then(|discount| discount.get("enabled"))
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    );
    item.insert(
        "bundle_discounts".into(),
        bundle_discount_object
            .and_then(|discount| discount.get("discounts"))
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
    );
    for field in [
        "item_count",
        "total_items_count",
        "followers_count",
        "following_count",
    ] {
        item.insert(field.into(), get_or_null(profile, field));
    }
    item.insert("last_activity".into(), last_activity);
    item.insert(
        "last_activity_at".into(),
        profile
            .get("last_loged_on_ts")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    );
    item.insert(
        "last_activity_localized".into(),
        profile
            .get("last_loged_on")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null),
    );
    item.insert("verification".into(), verification.clone());
    item.insert(
        "is_email_verified".into(),
        verification_valid(verification.get("email")),
    );
    item.insert(
        "is_facebook_verified".into(),
        verification_valid(verification.get("facebook")),
    );
    item.insert(
        "is_google_verified".into(),
        verification_valid(verification.get("google")),
    );
    item.insert("business".into(), get_or_null(profile, "business"));
    item.insert(
        "business_account_id".into(),
        get_or_null(profile, "business_account_id"),
    );
    item.insert(
        "is_on_holiday".into(),
        get_or_null(profile, "is_on_holiday"),
    );
    item.insert(
        "is_account_banned".into(),
        profile
            .get("is_account_banned")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Bool(false)),
    );
    item.insert("profile_url".into(), get_or_null(profile, "profile_url"));
    item.insert(
        "share_profile_url".into(),
        first_non_null(profile, "share_profile_url", "profile_url"),
    );
    item.insert("path".into(), get_or_null(profile, "path"));
    item.insert(
        "request_user_id".into(),
        Value::String(request.user_id.clone()),
    );
    item.insert(
        "request_country".into(),
        Value::String(request.country.clone()),
    );
    item.insert("request_index".into(), Value::from(request.index));
    item.insert("request_success".into(), Value::Bool(true));
    item.insert(
        "scrappa_duration_ms".into(),
        meta.get("duration_ms").cloned().unwrap_or(Value::Null),
    );
    item.insert(
        "scrappa_scraped_at".into(),
        meta.get("scraped_at").cloned().unwrap_or(Value::Null),
    );

    Value::Object(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request_params::build_vinted_user_profile_requests;
    use serde_json::json;

    #[test]
    fn maps_wrapped_profile_and_preserves_raw_fields_and_request_metadata() {
        let response = json!({
            "success": true,
            "data": {"user": {
                "id": 255914028,
                "login": "agranier",
                "country_code": "DE",
                "city": "Wiesbaden",
                "feedback_reputation": 0.98,
                "bundle_discount": {"enabled": true, "discounts": [{"minimal_item_count": 2, "fraction": "0.05"}]},
                "last_loged_on_ts": "2026-07-13T10:12:36+02:00",
                "verification": {"email": {"valid": true}, "facebook": {"valid": false}},
                "profile_url": "https://www.vinted.de/member/255914028-agranier",
                "raw_extra": "preserved"
            }},
            "meta": {"duration_ms": 2451.07, "scraped_at": "2026-07-13T09:49:21Z"}
        });
        let profile = get_vinted_user_profile(&response).unwrap();
        let request =
            build_vinted_user_profile_requests(&json!({"user_id": "255914028", "country": "DE"}))
                .unwrap()
                .remove(0);
        let item = build_vinted_user_profile_dataset_item(profile, &request, &response);

        assert_eq!(item["login"], "agranier");
        assert_eq!(item["raw_extra"], "preserved");
        assert_eq!(item["bundle_discount_enabled"], true);
        assert_eq!(item["bundle_discounts"][0]["fraction"], "0.05");
        assert_eq!(item["last_activity"], "2026-07-13T10:12:36+02:00");
        assert_eq!(item["is_email_verified"], true);
        assert_eq!(item["is_facebook_verified"], false);
        assert_eq!(item["is_google_verified"], Value::Null);
        assert_eq!(item["request_user_id"], "255914028");
        assert_eq!(item["request_country"], "DE");
        assert_eq!(item["scrappa_duration_ms"], 2451.07);
        assert_eq!(item["request_success"], true);
    }

    #[test]
    fn accepts_direct_profiles_and_skips_failed_private_banned_or_incomplete_profiles() {
        let profile = json!({"id": "42", "login": "seller", "path": "/member/42-seller"});
        assert_eq!(
            get_vinted_user_profile(&json!({"success": true, "user": profile})).unwrap()["login"],
            "seller"
        );
        assert!(get_vinted_user_profile(
            &json!({"success": false, "message": "User not found", "status_code": 404})
        )
        .unwrap_err()
        .to_string()
        .contains("User not found"));
        assert!(get_vinted_user_profile(
            &json!({"success": true, "data": {"user": {"id": 42, "can_view_profile": false}}})
        )
        .unwrap_err()
        .to_string()
        .contains("private or unavailable"));
        assert!(get_vinted_user_profile(
            &json!({"success": true, "data": {"user": {"id": 42, "is_account_banned": true}}})
        )
        .unwrap_err()
        .to_string()
        .contains("banned"));
        assert!(
            get_vinted_user_profile(&json!({"success": true, "data": {"user": {"id": 42}}}))
                .is_err()
        );
    }
}
