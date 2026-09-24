use anyhow::{anyhow, bail, Result};
use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub struct InterestParams {
    pub q: String,
    pub geo: Option<String>,
    pub time_range: Option<String>,
    pub hl: Option<String>,
    pub search_type: Option<String>,
}

impl InterestParams {
    pub fn query_pairs(&self) -> Vec<(&'static str, &str)> {
        let mut pairs = vec![("q", self.q.as_str())];
        if let Some(value) = self.geo.as_deref() {
            pairs.push(("geo", value));
        }
        if let Some(value) = self.time_range.as_deref() {
            pairs.push(("time_range", value));
        }
        if let Some(value) = self.hl.as_deref() {
            pairs.push(("hl", value));
        }
        if let Some(value) = self.search_type.as_deref() {
            pairs.push(("search_type", value));
        }
        pairs
    }

    pub fn value(&self, field: &str) -> Option<&str> {
        match field {
            "q" => Some(self.q.as_str()),
            "geo" => self.geo.as_deref(),
            "time_range" => self.time_range.as_deref(),
            "hl" => self.hl.as_deref(),
            "search_type" => self.search_type.as_deref(),
            _ => None,
        }
    }

    pub fn describe(&self) -> String {
        let filters = ["geo", "time_range", "hl", "search_type"]
            .into_iter()
            .filter_map(|field| self.value(field).map(|value| format!("{field}={value}")))
            .collect::<Vec<_>>();
        let suffix = if filters.is_empty() {
            String::new()
        } else {
            format!(" ({})", filters.join(", "))
        };
        format!("\"{}\"{suffix}", self.q)
    }
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };
    if value.is_empty() {
        return Ok(None);
    }

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(trimmed.to_owned()))
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_geo(value: Option<&Value>) -> Result<Option<String>> {
    let Some(geo) = clean_string(value, "geo", 10)? else {
        return Ok(None);
    };
    if geo.eq_ignore_ascii_case("worldwide") {
        return Ok(Some("Worldwide".to_owned()));
    }
    Ok(Some(geo.to_uppercase()))
}

fn clean_language(value: Option<&Value>) -> Result<Option<String>> {
    let Some(language) = clean_string(value, "hl", 2)? else {
        return Ok(None);
    };
    if language.len() != 2 || !language.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        bail!("hl must be a two-letter language code");
    }
    Ok(Some(language.to_lowercase()))
}

fn clean_enum(value: Option<&Value>, field: &str, values: &[&str]) -> Result<Option<String>> {
    let Some(value) = clean_string(value, field, 20)? else {
        return Ok(None);
    };
    let normalized = value.to_lowercase();
    if !values.contains(&normalized.as_str()) {
        bail!("{field} must be one of: {}", values.join(", "));
    }
    Ok(Some(normalized))
}

pub fn build_interest_params(input: &Value) -> Result<InterestParams> {
    Ok(InterestParams {
        q: clean_required_string(input.get("q"), "q", 100)?,
        geo: clean_geo(input.get("geo"))?,
        time_range: clean_enum(
            input.get("time_range"),
            "time_range",
            &["1h", "4h", "1d", "7d", "30d", "90d", "1y", "5y", "all"],
        )?,
        hl: clean_language(input.get("hl"))?,
        search_type: clean_enum(
            input.get("search_type"),
            "search_type",
            &["web", "images", "news", "youtube", "shopping"],
        )?,
    })
}
