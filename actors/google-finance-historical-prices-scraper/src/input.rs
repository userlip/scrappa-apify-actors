use anyhow::{anyhow, bail, Result};
use chrono::{NaiveDate, Utc};
use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct HistoricalPricesRequest {
    pub symbol: String,
    pub exchange: Option<String>,
    pub range: Option<u8>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub interval: Option<String>,
    pub hl: Option<String>,
    pub gl: Option<String>,
}

impl HistoricalPricesRequest {
    pub fn build(input: &Value) -> Result<Self> {
        let object = input
            .as_object()
            .ok_or_else(|| anyhow!("Input is required"))?;

        let symbol = clean_required_string(object.get("symbol"), "symbol", 20)?.to_uppercase();
        if symbol.chars().any(char::is_whitespace) {
            bail!("symbol cannot contain spaces");
        }

        let exchange = clean_string(object.get("exchange"), "exchange", 40)?
            .map(|exchange| exchange.to_uppercase());
        let range = clean_range(object.get("range"))?;
        let start_date = clean_date(object.get("start_date"), "start_date")?;
        let end_date = clean_date(object.get("end_date"), "end_date")?;
        let interval = clean_interval(object.get("interval"))?;
        let hl = clean_language_code(object.get("hl"))?;
        let gl = clean_country_code(object.get("gl"))?;

        if range.is_some() && (start_date.is_some() || end_date.is_some()) {
            bail!("Cannot use both range and start_date/end_date parameters together. Choose one approach.");
        }
        if start_date.is_some() != end_date.is_some() {
            bail!("start_date and end_date must be provided together");
        }
        if let (Some(start_date), Some(end_date)) = (&start_date, &end_date) {
            if start_date > end_date {
                bail!("end_date must be on or after start_date");
            }
        }

        Ok(Self {
            symbol,
            exchange,
            range,
            start_date,
            end_date,
            interval,
            hl,
            gl,
        })
    }

    pub fn has_custom_date_range(&self) -> bool {
        self.start_date.is_some() && self.end_date.is_some()
    }

    pub fn to_value(&self) -> Value {
        let mut params = Map::new();
        params.insert("symbol".to_owned(), Value::String(self.symbol.clone()));
        insert_optional(&mut params, "exchange", self.exchange.as_deref());
        if let Some(range) = self.range {
            params.insert("range".to_owned(), Value::from(range));
        }
        insert_optional(&mut params, "start_date", self.start_date.as_deref());
        insert_optional(&mut params, "end_date", self.end_date.as_deref());
        insert_optional(&mut params, "interval", self.interval.as_deref());
        insert_optional(&mut params, "hl", self.hl.as_deref());
        insert_optional(&mut params, "gl", self.gl.as_deref());
        Value::Object(params)
    }

    pub fn describe(&self) -> String {
        let mut filters = Vec::new();
        for (name, value) in [
            ("range", self.range.map(|value| value.to_string())),
            ("start_date", self.start_date.clone()),
            ("end_date", self.end_date.clone()),
            ("interval", self.interval.clone()),
            ("hl", self.hl.clone()),
            ("gl", self.gl.clone()),
        ] {
            if let Some(value) = value {
                filters.push(format!("{name}={value}"));
            }
        }

        let exchange = self
            .exchange
            .as_deref()
            .map(|exchange| format!(":{exchange}"))
            .unwrap_or_default();
        let filters = if filters.is_empty() {
            String::new()
        } else {
            format!(" ({})", filters.join(", "))
        };
        format!("{}{exchange}{filters}", self.symbol)
    }
}

fn insert_optional(params: &mut Map<String, Value>, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        params.insert(name.to_owned(), Value::String(value.to_owned()));
    }
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(value) = value.as_str() else {
        bail!("{field} must be a string");
    };

    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        bail!("{field} must be {max_length} characters or fewer");
    }
    Ok(Some(value.to_owned()))
}

fn clean_required_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<String> {
    clean_string(value, field, max_length)?.ok_or_else(|| anyhow!("{field} is required"))
}

fn clean_range(value: Option<&Value>) -> Result<Option<u8>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }
    if value.is_boolean() {
        bail!("range must be an integer");
    }

    let number = match value {
        Value::Number(number) => number.as_f64(),
        Value::String(value) => js_number(value),
        _ => None,
    }
    .filter(|number| number.is_finite() && number.fract() == 0.0)
    .ok_or_else(|| anyhow!("range must be an integer"))?;

    if !(1.0..=8.0).contains(&number) {
        bail!("range must be between 1 and 8");
    }
    Ok(Some(number as u8))
}

fn js_number(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Some(0.0);
    }

    let (digits, radix) = if let Some(digits) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        (digits, 16)
    } else if let Some(digits) = trimmed
        .strip_prefix("0b")
        .or_else(|| trimmed.strip_prefix("0B"))
    {
        (digits, 2)
    } else if let Some(digits) = trimmed
        .strip_prefix("0o")
        .or_else(|| trimmed.strip_prefix("0O"))
    {
        (digits, 8)
    } else {
        return trimmed.parse().ok();
    };

    u64::from_str_radix(digits, radix)
        .ok()
        .map(|value| value as f64)
}

fn clean_date(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    let Some(date) = clean_string(value, field, 10)? else {
        return Ok(None);
    };
    if date.len() != 10
        || date.as_bytes().get(4) != Some(&b'-')
        || date.as_bytes().get(7) != Some(&b'-')
        || !date
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        bail!("{field} must be in YYYY-MM-DD format");
    }

    let parsed = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|_| anyhow!("{field} must be a valid date"))?;
    if parsed.format("%Y-%m-%d").to_string() != date {
        bail!("{field} must be a valid date");
    }
    if parsed > Utc::now().date_naive() {
        bail!("{field} cannot be in the future");
    }
    Ok(Some(date))
}

fn clean_language_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(language) = clean_string(value, "hl", 10)? else {
        return Ok(None);
    };
    let language = language.to_lowercase();
    let parts: Vec<&str> = language.split('-').collect();
    let valid = matches!(parts.as_slice(), [language] if language.len() == 2 && language.bytes().all(|byte| byte.is_ascii_lowercase()))
        || matches!(parts.as_slice(), [language, country] if language.len() == 2 && country.len() == 2 && language.bytes().all(|byte| byte.is_ascii_lowercase()) && country.bytes().all(|byte| byte.is_ascii_lowercase()));
    if !valid {
        bail!("hl must be a valid language code such as en, de, or zh-cn");
    }
    Ok(Some(language))
}

fn clean_country_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(country) = clean_string(value, "gl", 2)? else {
        return Ok(None);
    };
    if country.len() != 2 || !country.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        bail!("gl must be a two-letter country code");
    }
    Ok(Some(country.to_lowercase()))
}

fn clean_interval(value: Option<&Value>) -> Result<Option<String>> {
    let Some(interval) = clean_string(value, "interval", 20)? else {
        return Ok(None);
    };
    let interval = interval.to_lowercase();
    if !["daily", "weekly", "monthly"].contains(&interval.as_str()) {
        bail!("interval must be one of: daily, weekly, monthly");
    }
    Ok(Some(interval))
}
