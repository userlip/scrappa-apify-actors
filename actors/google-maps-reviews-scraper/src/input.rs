use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};
use url::Url;

const SORT_REQUIRED_ERROR: &str =
    "Sort parameter is required (1-4: 1=Most Relevant, 2=Newest, 3=Highest Rating, 4=Lowest Rating)";

#[derive(Debug)]
pub struct ReviewsInput {
    pub business_id: String,
    pub sort: Value,
    fields: Map<String, Value>,
}

impl ReviewsInput {
    pub fn parse(input: Value) -> Result<Self> {
        let Some(fields) = input.as_object() else {
            return Err(anyhow!("Business ID is required"));
        };

        let business_id = fields
            .get("business_id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("Business ID is required"))?
            .to_owned();
        let sort = fields
            .get("sort")
            .cloned()
            .ok_or_else(|| anyhow!(SORT_REQUIRED_ERROR))?;

        Ok(Self {
            business_id,
            sort,
            fields: fields.clone(),
        })
    }

    pub fn request_url(&self, base_url: &str) -> Result<Url> {
        let mut url = Url::parse(&format!("{base_url}/maps/reviews"))
            .map_err(|error| anyhow!("SCRAPPA_API_BASE_URL must be a valid URL: {error}"))?;
        let limit = self
            .fields
            .get("limit")
            .filter(|value| js_truthy(value))
            .cloned()
            .unwrap_or_else(|| json!(10));

        let mut pairs = Vec::new();
        append_query_value(
            &mut pairs,
            "business_id",
            &Value::String(self.business_id.clone()),
        );
        append_query_value(&mut pairs, "sort", &self.sort);
        append_query_value(&mut pairs, "limit", &limit);
        if let Some(page) = self.fields.get("page") {
            append_query_value(&mut pairs, "page", page);
        }
        if let Some(search) = self.fields.get("search") {
            append_query_value(&mut pairs, "search", search);
        }
        if let Some(debug) = self.fields.get("debug") {
            append_query_value(&mut pairs, "debug", debug);
        }
        if self.fields.get("use_cache") != Some(&Value::Bool(false)) {
            pairs.push(("use_cache".to_owned(), "1".to_owned()));
        }
        if let Some(maximum_cache_age) = self.fields.get("maximum_cache_age") {
            append_query_value(&mut pairs, "maximum_cache_age", maximum_cache_age);
        }
        {
            let mut query = url.query_pairs_mut();
            for (key, value) in pairs {
                query.append_pair(&key, &value);
            }
        }
        Ok(url)
    }

    pub fn sort_name(&self) -> Option<&'static str> {
        match js_string(&self.sort).as_str() {
            "1" => Some("Most Relevant"),
            "2" => Some("Newest"),
            "3" => Some("Highest Rating"),
            "4" => Some("Lowest Rating"),
            _ => None,
        }
    }
}

fn append_query_value(query: &mut Vec<(String, String)>, key: &str, value: &Value) {
    if value.is_null() || value == "" || value == false {
        return;
    }
    let value = match value {
        Value::Bool(true) => "1".to_owned(),
        Value::String(value) => value.clone(),
        Value::Number(value) => js_number_string(value),
        Value::Null | Value::Bool(false) => return,
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    };
    query.push((key.to_owned(), value));
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Number(value) => js_number_string(value),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

fn js_number_string(value: &serde_json::Number) -> String {
    let Some(number) = value.as_f64() else {
        return value.to_string();
    };
    if number.fract() == 0.0 && number.abs() < 1e21 {
        format!("{number:.0}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::ReviewsInput;
    use serde_json::json;

    #[test]
    fn requires_business_id_and_sort_with_actor_error_messages() {
        assert_eq!(
            ReviewsInput::parse(json!({"sort": 2}))
                .unwrap_err()
                .to_string(),
            "Business ID is required"
        );
        assert_eq!(
            ReviewsInput::parse(json!({"business_id": "place"}))
                .unwrap_err()
                .to_string(),
            "Sort parameter is required (1-4: 1=Most Relevant, 2=Newest, 3=Highest Rating, 4=Lowest Rating)"
        );
    }

    #[test]
    fn builds_upstream_query_with_the_existing_defaults_and_pagination() {
        let input = ReviewsInput::parse(json!({
            "business_id": "0x808fba02425dad8f:0x6c296c66619367e0",
            "sort": 2,
            "limit": 0,
            "page": "token with spaces",
            "search": "good service",
            "debug": false,
            "use_cache": true,
            "maximum_cache_age": 0
        }))
        .unwrap();
        let url = input.request_url("https://scrappa.co/api").unwrap();
        let query = url.query_pairs().collect::<Vec<_>>();

        assert_eq!(url.path(), "/api/maps/reviews");
        assert_eq!(
            query[0],
            (
                "business_id".into(),
                "0x808fba02425dad8f:0x6c296c66619367e0".into()
            )
        );
        assert_eq!(query[1], ("sort".into(), "2".into()));
        assert_eq!(query[2], ("limit".into(), "10".into()));
        assert_eq!(query[3], ("page".into(), "token with spaces".into()));
        assert_eq!(query[4], ("search".into(), "good service".into()));
        assert_eq!(query[5], ("use_cache".into(), "1".into()));
        assert_eq!(query[6], ("maximum_cache_age".into(), "0".into()));
        assert_eq!(input.sort_name(), Some("Newest"));
    }

    #[test]
    fn false_cache_setting_omits_the_query_parameter() {
        let input = ReviewsInput::parse(json!({
            "business_id": "place",
            "sort": 1,
            "use_cache": false
        }))
        .unwrap();
        let url = input.request_url("https://scrappa.co/api").unwrap();
        assert!(!url.query().unwrap().contains("use_cache"));
    }

    #[test]
    fn formats_integral_json_numbers_like_javascript() {
        let input = ReviewsInput::parse(json!({
            "business_id": "place",
            "sort": 2.0,
            "limit": 5.0,
            "maximum_cache_age": 3600.0
        }))
        .unwrap();
        let query = input
            .request_url("https://scrappa.co/api")
            .unwrap()
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect::<Vec<_>>();

        assert_eq!(query[1], ("sort".into(), "2".into()));
        assert_eq!(query[2], ("limit".into(), "5".into()));
        assert_eq!(query[4], ("maximum_cache_age".into(), "3600".into()));
        assert_eq!(input.sort_name(), Some("Newest"));
    }
}
