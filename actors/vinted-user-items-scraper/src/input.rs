use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use serde_json::Value;

pub const VALID_COUNTRIES: &[&str] = &[
    "FR", "DE", "ES", "IT", "NL", "BE", "AT", "PL", "CZ", "LT", "LU", "SK", "HU", "RO", "PT", "SE",
    "DK", "FI", "US",
];
const VALID_ORDERS: &[&str] = &[
    "newest_first",
    "price_low_to_high",
    "price_high_to_low",
    "relevance",
];
const MAX_PAGE: u64 = 999;
const DEFAULT_PER_PAGE: u64 = 24;
const DEFAULT_MAX_PAGES: u64 = 1;
const MAX_PAGES_PER_USER: u64 = 20;
const MAX_USER_IDS_PER_RUN: usize = 100;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ActorInput {
    pub user_id: Option<Value>,
    pub user_ids: Option<Value>,
    pub country: Option<Value>,
    pub page: Option<Value>,
    pub per_page: Option<Value>,
    pub max_pages: Option<Value>,
    pub order: Option<Value>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct VintedUserItemsPlan {
    pub country: String,
    pub user_ids: Vec<String>,
    pub start_page: u64,
    pub per_page: u64,
    pub max_pages: u64,
    pub order: String,
}

pub fn build_plan(input: &ActorInput) -> Result<VintedUserItemsPlan> {
    let country = clean_country(input.country.as_ref())?;
    let per_page =
        clean_integer(input.per_page.as_ref(), "per_page", 1, 100)?.unwrap_or(DEFAULT_PER_PAGE);
    let order = clean_order(input.order.as_ref())?;
    let start_page = clean_integer(input.page.as_ref(), "page", 1, MAX_PAGE)?.unwrap_or(1);
    let max_pages = clean_integer(input.max_pages.as_ref(), "max_pages", 1, MAX_PAGES_PER_USER)?
        .unwrap_or(DEFAULT_MAX_PAGES);

    if start_page + max_pages - 1 > MAX_PAGE {
        bail!("page plus max_pages cannot exceed page 999");
    }

    let user_ids = clean_user_ids(input)?;
    Ok(VintedUserItemsPlan {
        country,
        user_ids,
        start_page,
        per_page,
        max_pages,
        order,
    })
}

impl VintedUserItemsPlan {
    pub fn describe(&self) -> String {
        let seller = if self.user_ids.len() == 1 {
            format!("seller {}", self.user_ids[0])
        } else {
            format!("{} sellers", self.user_ids.len())
        };
        let pages = if self.max_pages == 1 {
            format!("page {}", self.start_page)
        } else {
            format!(
                "pages {}-{}",
                self.start_page,
                self.start_page + self.max_pages - 1
            )
        };
        format!(
            "{seller} in {} ({pages}, {}/page)",
            self.country, self.per_page
        )
    }

    pub fn page_params(&self, user_id: &str, page: u64) -> Vec<(&'static str, String)> {
        vec![
            ("country", self.country.clone()),
            ("per_page", self.per_page.to_string()),
            ("order", self.order.clone()),
            ("user_id", user_id.to_owned()),
            ("page", page.to_string()),
        ]
    }
}

fn clean_user_ids(input: &ActorInput) -> Result<Vec<String>> {
    let mut user_ids = Vec::new();
    append_user_ids(input.user_id.as_ref(), "user_id", &mut user_ids)?;
    append_user_ids(input.user_ids.as_ref(), "user_ids", &mut user_ids)?;

    let mut unique_user_ids = Vec::new();
    for user_id in user_ids {
        if !unique_user_ids.contains(&user_id) {
            unique_user_ids.push(user_id);
        }
    }

    if unique_user_ids.is_empty() {
        bail!("Provide at least one Vinted seller user_id or user_ids value");
    }
    if unique_user_ids.len() > MAX_USER_IDS_PER_RUN {
        bail!("user_ids supports at most {MAX_USER_IDS_PER_RUN} unique IDs per run");
    }
    if let Some(invalid_user_id) = unique_user_ids
        .iter()
        .find(|id| !id.bytes().all(|b| b.is_ascii_digit()))
    {
        bail!("user_ids must contain numeric Vinted user IDs; received \"{invalid_user_id}\"");
    }

    Ok(unique_user_ids)
}

fn append_user_ids(value: Option<&Value>, field: &str, user_ids: &mut Vec<String>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(());
    }
    if let Value::Array(values) = value {
        for value in values {
            append_user_ids(Some(value), field, user_ids)?;
        }
        return Ok(());
    }

    let Some(raw) = clean_string(Some(value), field, 2000)? else {
        return Ok(());
    };
    user_ids.extend(
        raw.split(',')
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_owned),
    );
    Ok(())
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }

    let raw = match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                integer.to_string()
            } else if let Some(integer) = value.as_u64() {
                integer.to_string()
            } else if let Some(number) = value.as_f64().filter(|number| number.fract() == 0.0) {
                format!("{number:.0}")
            } else {
                value.to_string()
            }
        }
        _ => bail!("{field} must be a string or number"),
    };
    let raw = raw.trim().to_owned();
    if raw.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    if raw.is_empty() {
        return Ok(None);
    }
    Ok(Some(raw))
}

fn clean_integer(value: Option<&Value>, field: &str, min: u64, max: u64) -> Result<Option<u64>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }

    let parsed = match value {
        Value::String(value) => {
            let trimmed = value.trim();
            if !trimmed.is_empty() && trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
                trimmed.parse::<u64>().ok()
            } else {
                None
            }
        }
        Value::Number(value) => {
            if value.as_i64().is_some_and(|integer| integer < 0)
                || value.as_f64().is_some_and(|number| {
                    number.is_finite() && number.fract() == 0.0 && number < 0.0
                })
            {
                bail!("{field} must be between {min} and {max}");
            }
            value
                .as_u64()
                .or_else(|| value.as_i64().map(|integer| integer as u64))
                .or_else(|| {
                    value.as_f64().and_then(|number| {
                        (number.is_finite() && number.fract() == 0.0 && number >= 0.0)
                            .then_some(number as u64)
                    })
                })
        }
        _ => None,
    }
    .ok_or_else(|| anyhow!("{field} must be an integer"))?;

    if parsed < min || parsed > max {
        bail!("{field} must be between {min} and {max}");
    }
    Ok(Some(parsed))
}

fn clean_country(value: Option<&Value>) -> Result<String> {
    let Some(country) = clean_string(value, "country", 2)? else {
        return Ok("FR".to_owned());
    };
    let country = country.to_ascii_uppercase();
    if !VALID_COUNTRIES.contains(&country.as_str()) {
        bail!("country must be one of: {}", VALID_COUNTRIES.join(", "));
    }
    Ok(country)
}

fn clean_order(value: Option<&Value>) -> Result<String> {
    let order = clean_string(value, "order", 30)?.unwrap_or_else(|| "newest_first".to_owned());
    if !VALID_ORDERS.contains(&order.as_str()) {
        bail!("order must be one of: {}", VALID_ORDERS.join(", "));
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_default_plan_from_single_seller_and_preserves_prefill_shape() {
        let input: ActorInput = serde_json::from_value(json!({ "user_id": " 12345678 " })).unwrap();
        let plan = build_plan(&input).unwrap();

        assert_eq!(
            plan,
            VintedUserItemsPlan {
                country: "FR".to_owned(),
                user_ids: vec!["12345678".to_owned()],
                start_page: 1,
                per_page: 24,
                max_pages: 1,
                order: "newest_first".to_owned(),
            }
        );
        assert_eq!(
            plan.page_params("12345678", 1),
            vec![
                ("country", "FR".to_owned()),
                ("per_page", "24".to_owned()),
                ("order", "newest_first".to_owned()),
                ("user_id", "12345678".to_owned()),
                ("page", "1".to_owned()),
            ]
        );
        assert_eq!(plan.describe(), "seller 12345678 in FR (page 1, 24/page)");
    }

    #[test]
    fn normalizes_batched_ids_country_and_pagination() {
        let input: ActorInput = serde_json::from_value(json!({
            "user_id": 12345678,
            "user_ids": ["87654321", "12345678", " 22222222,33333333 "],
            "country": "de",
            "page": "2",
            "per_page": "50",
            "max_pages": "3",
            "order": "newest_first"
        }))
        .unwrap();
        let plan = build_plan(&input).unwrap();

        assert_eq!(plan.country, "DE");
        assert_eq!(
            plan.user_ids,
            ["12345678", "87654321", "22222222", "33333333"]
        );
        assert_eq!(plan.start_page, 2);
        assert_eq!(plan.per_page, 50);
        assert_eq!(plan.max_pages, 3);
        assert_eq!(plan.describe(), "4 sellers in DE (pages 2-4, 50/page)");
    }

    #[test]
    fn validates_seller_ids_country_sort_order_and_page_bounds() {
        for (input, message) in [
            (json!({}), "Provide at least one Vinted seller user_id"),
            (
                json!({"user_ids":["abc"]}),
                "user_ids must contain numeric Vinted user IDs",
            ),
            (
                json!({"user_id":"123","country":"GB"}),
                "country must be one of",
            ),
            (
                json!({"user_id":"123","order":"oldest_first"}),
                "order must be one of",
            ),
            (
                json!({"user_id":"123","page":0}),
                "page must be between 1 and 999",
            ),
            (
                json!({"user_id":"123","page":-1}),
                "page must be between 1 and 999",
            ),
            (
                json!({"user_id":"123","max_pages":21}),
                "max_pages must be between 1 and 20",
            ),
            (
                json!({"user_id":"123","page":990,"max_pages":20}),
                "page plus max_pages",
            ),
        ] {
            let input: ActorInput = serde_json::from_value(input).unwrap();
            assert!(build_plan(&input)
                .unwrap_err()
                .to_string()
                .contains(message));
        }
    }

    #[test]
    fn deduplicates_nested_and_comma_separated_user_id_values() {
        let input: ActorInput = serde_json::from_value(json!({
            "user_ids": [["111,222"], 333, " 222,444 "]
        }))
        .unwrap();
        let plan = build_plan(&input).unwrap();
        assert_eq!(plan.user_ids, ["111", "222", "333", "444"]);
    }

    #[test]
    fn rejects_more_than_one_hundred_unique_sellers() {
        let ids = (1..=101).map(|id| id.to_string()).collect::<Vec<_>>();
        let input: ActorInput = serde_json::from_value(json!({ "user_ids": ids })).unwrap();
        assert!(build_plan(&input)
            .unwrap_err()
            .to_string()
            .contains("at most 100 unique IDs"));
    }
}
