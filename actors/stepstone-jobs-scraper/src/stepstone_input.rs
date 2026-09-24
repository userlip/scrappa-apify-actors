use serde_json::{Map, Value, json};

const KNOWN_INPUT_KEYS: [&str; 10] = [
    "query",
    "location",
    "country",
    "radius",
    "sort",
    "job_type",
    "work_from_home",
    "date_posted",
    "page",
    "limit",
];

#[derive(Clone, Debug, PartialEq)]
pub struct StepstoneJobsInput {
    values: Map<String, Value>,
}

impl StepstoneJobsInput {
    pub fn normalize(input: Option<&Value>) -> Self {
        let Some(object) = input.and_then(Value::as_object) else {
            return Self::default();
        };

        let mut normalized = Map::new();
        for key in KNOWN_INPUT_KEYS {
            let Some(value) = object.get(key) else {
                continue;
            };
            if value.is_null() {
                continue;
            }

            if let Some(string) = value.as_str() {
                let trimmed = string.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let normalized_value = if key == "country" {
                    trimmed.to_lowercase()
                } else {
                    trimmed.to_owned()
                };
                normalized.insert(key.to_owned(), Value::String(normalized_value));
            } else {
                normalized.insert(key.to_owned(), value.clone());
            }
        }

        if normalized.is_empty() {
            return Self::default();
        }

        let mut values = default_values();
        values.extend(normalized);
        Self { values }
    }

    pub fn query(&self) -> &Value {
        &self.values["query"]
    }

    pub fn query_is_truthy(&self) -> bool {
        javascript_truthy(self.query())
    }

    pub fn query_for_log(&self) -> String {
        javascript_string(self.query())
    }

    pub fn request_params(&self) -> Map<String, Value> {
        self.values
            .iter()
            .filter(|(_, value)| {
                !value.is_null() && value.as_str().is_none_or(|string| !string.is_empty())
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }
}

impl Default for StepstoneJobsInput {
    fn default() -> Self {
        Self {
            values: default_values(),
        }
    }
}

fn default_values() -> Map<String, Value> {
    json!({
        "query": "software engineer",
        "location": "Berlin",
        "country": "de",
        "page": 1,
        "limit": 25
    })
    .as_object()
    .expect("default input is an object")
    .clone()
}

fn javascript_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn javascript_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                if value.is_null() {
                    String::new()
                } else {
                    javascript_string(value)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_actor_defaults_for_missing_empty_and_unknown_input() {
        let expected = StepstoneJobsInput::default();
        assert_eq!(StepstoneJobsInput::normalize(None), expected);
        assert_eq!(StepstoneJobsInput::normalize(Some(&json!({}))), expected);
        assert_eq!(
            StepstoneJobsInput::normalize(Some(&json!({"unused": "value"}))),
            expected
        );
        assert_eq!(
            StepstoneJobsInput::normalize(Some(&json!({"query": "   "}))),
            expected
        );
    }

    #[test]
    fn normalizes_country_and_preserves_false_boolean_filters() {
        let input = StepstoneJobsInput::normalize(Some(&json!({
            "query": "  Lagerist  ",
            "country": " AT ",
            "work_from_home": false,
            "unknown": true
        })));

        assert_eq!(input.query(), "Lagerist");
        assert_eq!(input.get("country"), Some(&json!("at")));
        assert_eq!(input.get("work_from_home"), Some(&json!(false)));
        assert_eq!(
            input.request_params(),
            json!({
                "query": "Lagerist",
                "location": "Berlin",
                "country": "at",
                "work_from_home": false,
                "page": 1,
                "limit": 25
            })
            .as_object()
            .unwrap()
            .clone()
        );
    }

    #[test]
    fn supplies_the_default_query_for_partial_targeting_input() {
        let input = StepstoneJobsInput::normalize(Some(&json!({
            "location": "Vienna",
            "country": "AT"
        })));

        assert_eq!(input.query(), "software engineer");
        assert_eq!(input.get("location"), Some(&json!("Vienna")));
        assert_eq!(input.get("country"), Some(&json!("at")));
    }

    #[test]
    fn serializes_boolean_query_values_as_zero_and_one() {
        assert_eq!(javascript_string(&json!(false)), "false");
        assert_eq!(javascript_string(&json!(true)), "true");
        assert!(!javascript_truthy(&json!(0)));
        assert!(!javascript_truthy(&json!(false)));
    }
}
