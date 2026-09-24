use serde_json::{json, Map, Number, Value};

use crate::request_params::SimilarwebTrafficRequest;

fn as_record(value: Option<&Value>) -> Map<String, Value> {
    value
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn as_array(value: Option<&Value>) -> Vec<Value> {
    value.and_then(Value::as_array).cloned().unwrap_or_default()
}

fn first_string(values: &[Option<&Value>]) -> Option<String> {
    values.iter().find_map(|value| {
        let value = value.as_ref()?.as_str()?;
        (!value.trim().is_empty()).then(|| value.to_owned())
    })
}

fn parse_number(value: &Value) -> Option<f64> {
    if let Some(number) = value.as_f64() {
        return number.is_finite().then_some(number);
    }

    let value = value.as_str()?.replace(',', "");
    let value = value.trim().to_owned();
    if value.is_empty() {
        return None;
    }

    let parsed = if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        i64::from_str_radix(hex, 16)
            .ok()
            .map(|number| number as f64)
    } else if let Some(binary) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        i64::from_str_radix(binary, 2)
            .ok()
            .map(|number| number as f64)
    } else if let Some(octal) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        i64::from_str_radix(octal, 8)
            .ok()
            .map(|number| number as f64)
    } else {
        value.parse::<f64>().ok()
    }?;

    parsed.is_finite().then_some(parsed)
}

fn first_number(values: &[Option<&Value>]) -> Option<f64> {
    values
        .iter()
        .find_map(|value| parse_number(value.as_ref()?))
}

fn number_value(value: f64) -> Value {
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value < i64::MAX as f64 {
        return Value::Number(Number::from(value as i64));
    }
    if value.fract() == 0.0 && value >= 0.0 && value < u64::MAX as f64 {
        return Value::Number(Number::from(value as u64));
    }
    Number::from_f64(value)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn rank_value(value: Option<&Value>) -> Value {
    let Some(value) = value else {
        return Value::Null;
    };
    if value.is_number() {
        return value.clone();
    }
    if let Some(rank) = value.as_str().filter(|rank| !rank.trim().is_empty()) {
        return parse_number(&Value::String(rank.to_owned()))
            .map(number_value)
            .unwrap_or_else(|| Value::String(rank.trim().to_owned()));
    }

    let record = as_record(Some(value));
    if let Some(rank) = first_number(&[record.get("Rank"), record.get("rank")]) {
        return number_value(rank);
    }
    first_string(&[record.get("Rank"), record.get("rank")])
        .map(Value::String)
        .unwrap_or(Value::Null)
}

fn latest_month(visits: &Map<String, Value>) -> Option<String> {
    visits.keys().max().cloned()
}

fn has_visit_values(visits: &Map<String, Value>) -> bool {
    visits.values().any(|value| parse_number(value).is_some())
}

pub(crate) fn has_similarweb_traffic_data(response: &Value) -> bool {
    !rank_value(response.get("global_rank")).is_null()
        || first_number(&[as_record(response.get("engagement")).get("visits")]).is_some()
        || has_visit_values(&as_record(response.get("estimated_monthly_visits")))
        || has_visit_values(&as_record(response.get("monthly_visits")))
}

pub(crate) fn build_similarweb_dataset_item(
    response: &Value,
    request: &SimilarwebTrafficRequest,
) -> Value {
    let engagement = as_record(response.get("engagement"));
    let traffic_sources = as_record(response.get("traffic_sources"));
    let monthly_visits = as_record(response.get("monthly_visits"));
    let mut estimated_monthly_visits = monthly_visits.clone();
    estimated_monthly_visits.extend(as_record(response.get("estimated_monthly_visits")));
    let latest = latest_month(&estimated_monthly_visits);
    let country_rank = response.get("country_rank");
    let country_rank_record = as_record(country_rank);

    let nullable_number = |values: &[Option<&Value>]| {
        first_number(values)
            .map(number_value)
            .unwrap_or(Value::Null)
    };
    let country_code = response
        .get("country_code")
        .filter(|value| !value.is_null())
        .cloned()
        .or_else(|| first_string(&[country_rank_record.get("CountryCode")]).map(Value::String))
        .unwrap_or(Value::Null);
    let latest_month_visits = latest
        .as_ref()
        .and_then(|month| estimated_monthly_visits.get(month))
        .and_then(|value| parse_number(value))
        .map(number_value)
        .unwrap_or(Value::Null);
    let top_countries = as_array(response.get("top_countries"));
    let top_keywords = as_array(response.get("top_keywords"));
    let top_countries_count = top_countries.len();
    let top_keywords_count = top_keywords.len();
    let monthly_visit_month_count = monthly_visits.len();
    let estimated_month_count = estimated_monthly_visits.len();
    let domain = first_string(&[response.get("domain")]).unwrap_or_else(|| request.domain.clone());

    json!({
        "success": true,
        "domain": domain,
        "site_name": response.get("site_name").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "title": response.get("title").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "description": response.get("description").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "category": response.get("category").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "global_rank_value": rank_value(response.get("global_rank")),
        "global_rank": response.get("global_rank").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "country_rank_value": rank_value(country_rank),
        "country_rank": country_rank.filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "country_code": country_code,
        "category_rank_value": rank_value(response.get("category_rank")),
        "category_rank": response.get("category_rank").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "visits": nullable_number(&[engagement.get("visits")]),
        "time_on_site": nullable_number(&[engagement.get("time_on_site")]),
        "page_per_visit": nullable_number(&[engagement.get("page_per_visit")]),
        "bounce_rate": nullable_number(&[engagement.get("bounce_rate")]),
        "engagement_month": nullable_number(&[engagement.get("month")]),
        "engagement_year": nullable_number(&[engagement.get("year")]),
        "traffic_direct": nullable_number(&[traffic_sources.get("direct")]),
        "traffic_search": nullable_number(&[traffic_sources.get("search")]),
        "traffic_social": nullable_number(&[traffic_sources.get("social")]),
        "traffic_referrals": nullable_number(&[traffic_sources.get("referrals")]),
        "traffic_mail": nullable_number(&[traffic_sources.get("mail")]),
        "traffic_paid_referrals": nullable_number(&[traffic_sources.get("paid_referrals")]),
        "top_countries": top_countries,
        "top_keywords": top_keywords,
        "monthly_visits": monthly_visits,
        "estimated_monthly_visits": estimated_monthly_visits,
        "latest_month": latest,
        "latest_month_visits": latest_month_visits,
        "screenshot": response.get("screenshot").filter(|value| !value.is_null()).cloned().unwrap_or(Value::Null),
        "request_domain": &request.domain,
        "input_domain": &request.input_domain,
        "result_counts": {
            "top_countries": top_countries_count,
            "top_keywords": top_keywords_count,
            "monthly_visit_months": monthly_visit_month_count,
            "estimated_monthly_visit_months": estimated_month_count,
        },
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn request(domain: &str, input_domain: &str) -> SimilarwebTrafficRequest {
        SimilarwebTrafficRequest {
            domain: domain.to_owned(),
            input_domain: input_domain.to_owned(),
        }
    }

    #[test]
    fn builds_dataset_item_with_normalized_metrics_and_latest_month() {
        let response = json!({
            "domain":"google.com",
            "site_name":"google.com",
            "title":"Google",
            "category":"search_engines",
            "global_rank":{"Rank":1},
            "country_rank":{"Country":840,"CountryCode":"US","Rank":"1"},
            "category_rank":{"Rank":"2","Category":"Search"},
            "engagement":{"visits":"84,172,772,881","time_on_site":"592.7","page_per_visit":"8.35","bounce_rate":"0.28","month":"12","year":"2025"},
            "traffic_sources":{"direct":0.86,"search":0.07,"social":0.01,"referrals":0.04,"mail":0.002,"paid_referrals":0.003},
            "estimated_monthly_visits":{"2025-10-01":85326206196_i64,"2025-12-01":"84172772881"},
            "monthly_visits":{"2025-09-01":"81234567890"},
            "top_countries":[{"country_code":"US","share":0.24}],
            "top_keywords":[{"keyword":"gmail","volume":114101130}],
            "screenshot":"https://example.com/screenshot.png"
        });

        let item = build_similarweb_dataset_item(
            &response,
            &request("google.com", "https://www.google.com/search"),
        );

        assert_eq!(item["success"], true);
        assert_eq!(item["domain"], "google.com");
        assert_eq!(item["global_rank_value"], 1);
        assert_eq!(item["country_rank_value"], 1);
        assert_eq!(item["category_rank_value"], 2);
        assert_eq!(item["visits"], 84172772881_f64);
        assert_eq!(item["time_on_site"], 592.7);
        assert_eq!(item["traffic_direct"], 0.86);
        assert_eq!(item["latest_month"], "2025-12-01");
        assert_eq!(item["latest_month_visits"], 84172772881_f64);
        assert_eq!(item["input_domain"], "https://www.google.com/search");
        assert_eq!(item["result_counts"]["top_countries"], 1);
        assert_eq!(item["result_counts"]["top_keywords"], 1);
        assert_eq!(item["result_counts"]["monthly_visit_months"], 1);
        assert_eq!(item["result_counts"]["estimated_monthly_visit_months"], 3);
    }

    #[test]
    fn treats_monthly_visits_as_traffic_data_and_uses_the_latest_value() {
        let response = json!({
            "domain":"startup.example",
            "monthly_visits":{"2026-01-01":"1200","2026-02-01":1500}
        });
        assert!(has_similarweb_traffic_data(&response));

        let item = build_similarweb_dataset_item(
            &response,
            &request("startup.example", "startup.example"),
        );
        assert_eq!(item["global_rank_value"], Value::Null);
        assert_eq!(item["visits"], Value::Null);
        assert_eq!(item["estimated_monthly_visits"]["2026-01-01"], "1200");
        assert_eq!(item["latest_month"], "2026-02-01");
        assert_eq!(item["latest_month_visits"], 1500);
    }

    #[test]
    fn detects_traffic_data_from_rank_engagement_or_visit_series() {
        assert!(has_similarweb_traffic_data(
            &json!({"global_rank":{"Rank":123}})
        ));
        assert!(has_similarweb_traffic_data(
            &json!({"global_rank":"unranked"})
        ));
        assert!(has_similarweb_traffic_data(
            &json!({"engagement":{"visits":"1000"}})
        ));
        assert!(has_similarweb_traffic_data(
            &json!({"estimated_monthly_visits":{"2026-01-01":"1000"}})
        ));
        assert!(has_similarweb_traffic_data(
            &json!({"monthly_visits":{"2026-01-01":"1000"}})
        ));
        assert!(!has_similarweb_traffic_data(
            &json!({"engagement":{"visits":null}})
        ));
        assert!(!has_similarweb_traffic_data(
            &json!({"monthly_visits":{"2026-01-01":"n/a"}})
        ));
    }

    #[test]
    fn parses_javascript_style_numbers_and_preserves_non_numeric_rank_strings() {
        assert_eq!(rank_value(Some(&json!(" 0x10 "))), json!(16));
        assert_eq!(rank_value(Some(&json!("not ranked"))), json!("not ranked"));
        assert_eq!(
            rank_value(Some(&json!({"Rank":"n/a","rank":"1,234"}))),
            json!(1234)
        );
    }
}
