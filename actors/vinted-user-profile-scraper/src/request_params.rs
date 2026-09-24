use anyhow::{anyhow, Result};
use serde_json::Value;

pub const VALID_VINTED_COUNTRIES: [&str; 19] = [
    "FR", "DE", "ES", "IT", "NL", "BE", "AT", "PL", "CZ", "LT", "LU", "SK", "HU", "RO", "PT", "SE",
    "DK", "FI", "US",
];
pub const MAX_USER_IDS_PER_RUN: usize = 100;
const DEFAULT_COUNTRY: &str = "FR";
const MAX_USER_ID_LENGTH: usize = 32;
const MAX_BATCH_INPUT_LENGTH: usize = 4_000;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VintedUserProfileRequest {
    pub user_id: String,
    pub country: String,
    pub index: usize,
}

impl VintedUserProfileRequest {
    pub fn describe(&self) -> String {
        format!("{} in {}", self.user_id, self.country)
    }
}

fn clean_string(value: &Value, field: &str, max_length: usize) -> Result<Option<String>> {
    let raw = match value {
        Value::Null => return Ok(None),
        Value::String(value) => value.clone(),
        Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                integer.to_string()
            } else if let Some(integer) = value.as_u64() {
                integer.to_string()
            } else if let Some(number) = value.as_f64() {
                if number.is_finite() && number.fract() == 0.0 && number.abs() < 1e21 {
                    format!("{number:.0}")
                } else {
                    value.to_string()
                }
            } else {
                value.to_string()
            }
        }
        _ => return Err(anyhow!("{field} must be a string or number")),
    };

    let normalized = raw.trim();
    if normalized.is_empty() {
        return Ok(None);
    }
    if normalized.chars().count() > max_length {
        return Err(anyhow!("{field} must be {max_length} characters or fewer"));
    }

    Ok(Some(normalized.to_owned()))
}

fn clean_country(value: Option<&Value>) -> Result<String> {
    let country = match value {
        None | Some(Value::Null) => DEFAULT_COUNTRY.to_owned(),
        Some(Value::String(value)) if value.is_empty() => DEFAULT_COUNTRY.to_owned(),
        Some(value) => {
            clean_string(value, "country", 2)?.unwrap_or_else(|| DEFAULT_COUNTRY.to_owned())
        }
    }
    .to_ascii_uppercase();

    if !VALID_VINTED_COUNTRIES.contains(&country.as_str()) {
        return Err(anyhow!(
            "country must be one of: {}",
            VALID_VINTED_COUNTRIES.join(", ")
        ));
    }
    Ok(country)
}

fn clean_user_id(value: &Value, field: &str) -> Result<Option<String>> {
    if let Value::Number(number) = value {
        let id = if let Some(integer) = number.as_u64() {
            if integer == 0 || integer > MAX_SAFE_INTEGER {
                return Err(anyhow!(
                    "{field} must be a numeric Vinted user ID or safe integer"
                ));
            }
            integer.to_string()
        } else if let Some(number) = number.as_f64() {
            if !number.is_finite()
                || number <= 0.0
                || number.fract() != 0.0
                || number > MAX_SAFE_INTEGER as f64
            {
                return Err(anyhow!(
                    "{field} must be a numeric Vinted user ID or safe integer"
                ));
            }
            format!("{number:.0}")
        } else {
            return Err(anyhow!(
                "{field} must be a numeric Vinted user ID or safe integer"
            ));
        };
        return Ok(Some(id));
    }

    let Some(user_id) = clean_string(value, field, MAX_USER_ID_LENGTH)? else {
        return Ok(None);
    };
    if !user_id
        .bytes()
        .enumerate()
        .all(|(index, byte)| byte.is_ascii_digit() && (index != 0 || byte != b'0'))
    {
        return Err(anyhow!(
            "{field} must be a numeric Vinted user ID; received \"{user_id}\""
        ));
    }

    Ok(Some(user_id))
}

fn split_user_ids(value: Option<&Value>, field: &str, ids: &mut Vec<String>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };

    match value {
        Value::Null => Ok(()),
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                split_user_ids(Some(value), &format!("{field}[{index}]"), ids)?;
            }
            Ok(())
        }
        _ => {
            let Some(raw) = clean_string(value, field, MAX_BATCH_INPUT_LENGTH)? else {
                return Ok(());
            };
            for (index, raw_id) in raw.split(',').enumerate() {
                if let Some(user_id) = clean_user_id(
                    &Value::String(raw_id.to_owned()),
                    &format!("{field}[{index}]"),
                )? {
                    ids.push(user_id);
                }
            }
            Ok(())
        }
    }
}

pub fn build_vinted_user_profile_requests(input: &Value) -> Result<Vec<VintedUserProfileRequest>> {
    let country = clean_country(input.get("country"))?;
    let mut ids = Vec::new();
    split_user_ids(input.get("user_id"), "user_id", &mut ids)?;
    split_user_ids(input.get("user_ids"), "user_ids", &mut ids)?;

    let mut seen = std::collections::HashSet::new();
    ids.retain(|user_id| seen.insert(user_id.clone()));
    if ids.is_empty() {
        return Err(anyhow!(
            "Provide at least one Vinted user ID using user_id or user_ids"
        ));
    }
    if ids.len() > MAX_USER_IDS_PER_RUN {
        return Err(anyhow!(
            "user_ids supports at most {MAX_USER_IDS_PER_RUN} unique IDs per run"
        ));
    }

    Ok(ids
        .into_iter()
        .enumerate()
        .map(|(index, user_id)| VintedUserProfileRequest {
            user_id,
            country: country.clone(),
            index,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_singular_array_csv_whitespace_and_duplicate_ids_in_input_order() {
        let requests = build_vinted_user_profile_requests(&json!({
            "user_id": " 255914028 ",
            "user_ids": ["255914028", " 123 ", ["456, 123"]],
            "country": " de "
        }))
        .unwrap();

        assert_eq!(
            requests
                .iter()
                .map(|request| request.user_id.as_str())
                .collect::<Vec<_>>(),
            ["255914028", "123", "456"]
        );
        assert!(requests.iter().all(|request| request.country == "DE"));
    }

    #[test]
    fn accepts_safe_numeric_ids_and_defaults_country() {
        let requests =
            build_vinted_user_profile_requests(&json!({"user_id": 255914028.0})).unwrap();
        assert_eq!(requests[0].user_id, "255914028");
        assert_eq!(requests[0].country, "FR");
    }

    #[test]
    fn rejects_empty_malformed_unsupported_and_oversized_input() {
        assert!(build_vinted_user_profile_requests(&json!({}))
            .unwrap_err()
            .to_string()
            .contains("at least one"));
        assert!(
            build_vinted_user_profile_requests(&json!({"user_ids": "123,not-a-user"}))
                .unwrap_err()
                .to_string()
                .contains("numeric")
        );
        assert!(build_vinted_user_profile_requests(&json!({"user_id": "0"})).is_err());
        assert!(
            build_vinted_user_profile_requests(&json!({"user_id": "123", "country": "GB"}))
                .unwrap_err()
                .to_string()
                .contains("country must be one of")
        );

        let ids = (1..=101).map(|id| id.to_string()).collect::<Vec<_>>();
        assert!(
            build_vinted_user_profile_requests(&json!({"user_ids": ids}))
                .unwrap_err()
                .to_string()
                .contains("at most 100")
        );
    }
}
