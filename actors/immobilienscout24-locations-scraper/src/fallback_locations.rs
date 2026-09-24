use serde_json::{json, Value};

pub fn get_fallback_locations(query: &str, limit: usize) -> Option<Value> {
    if query.to_lowercase() != "berlin" {
        return None;
    }

    let locations = vec![
        json!({ "geocode": "1276003001", "name": "Berlin", "type": "city", "is_cached": true }),
        json!({ "geocode": "1276003001013", "name": "Berlin Mitte", "type": "district", "is_cached": true }),
    ];
    Some(json!({ "locations": locations.into_iter().take(limit).collect::<Vec<_>>() }))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::get_fallback_locations;

    #[test]
    fn returns_verified_case_insensitive_berlin_locations_with_a_limit() {
        assert_eq!(
            get_fallback_locations("berlin", 1).unwrap(),
            json!({
                "locations": [{ "geocode": "1276003001", "name": "Berlin", "type": "city", "is_cached": true }]
            })
        );
    }

    #[test]
    fn does_not_invent_fallbacks_for_other_queries() {
        assert!(get_fallback_locations("Hamburg", 10).is_none());
    }
}
