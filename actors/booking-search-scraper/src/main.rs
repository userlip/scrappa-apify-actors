use anyhow::{anyhow, bail, Context, Result};
use rand::random;
use reqwest::{header, Client, Response, StatusCode};
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fmt,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::{sleep, timeout};
use url::Url;

const APIFY_API_BASE_URL: &str = "https://api.apify.com";
const SCRAPPA_API_BASE_URL: &str = "https://scrappa.co/api";
const SCRAPPA_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const SCRAPPA_MAX_ATTEMPTS: u32 = 3;
const BOOKING_RESULT_CHARGE_EVENT: &str = "booking-result";
const DEFAULT_DATASET_ITEM_EVENT: &str = "apify-default-dataset-item";
const MAX_SEARCHES_PER_RUN: usize = 25;

struct ActorConfig {
    apify_api_base_url: Url,
    scrappa_api_base_url: Url,
    default_key_value_store_id: String,
    default_dataset_id: String,
    input_key: String,
    apify_token: String,
    actor_run_id: String,
    scrappa_api_key: String,
}

impl ActorConfig {
    fn from_env() -> Result<Self> {
        let scrappa_api_key = env::var("SCRAPPA_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow!("SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.")
            })?;

        Ok(Self {
            apify_api_base_url: base_url_from_env("APIFY_API_PUBLIC_BASE_URL", APIFY_API_BASE_URL)?,
            scrappa_api_base_url: base_url_from_env("SCRAPPA_API_BASE_URL", SCRAPPA_API_BASE_URL)?,
            default_key_value_store_id: required_env("ACTOR_DEFAULT_KEY_VALUE_STORE_ID")?,
            default_dataset_id: required_env("ACTOR_DEFAULT_DATASET_ID")?,
            input_key: env::var("ACTOR_INPUT_KEY").unwrap_or_else(|_| "INPUT".to_owned()),
            apify_token: required_env("APIFY_TOKEN")?,
            actor_run_id: required_env("ACTOR_RUN_ID")?,
            scrappa_api_key,
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    let value = env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing"))?;
    if value.trim().is_empty() {
        bail!("Required environment variable {name} is empty");
    }
    Ok(value)
}

fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).with_context(|| format!("{name} must be a valid absolute URL"))
}

fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments.iter().copied());
    Ok(url)
}

#[derive(Debug, Clone, PartialEq)]
struct BookingSearchRequest {
    params: BTreeMap<String, Value>,
    index: usize,
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

fn has_date_format(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
}

fn is_valid_date(value: &str) -> bool {
    if !has_date_format(value) {
        return false;
    }

    let year = value[0..4].parse::<u32>().ok();
    let month = value[5..7].parse::<u32>().ok();
    let day = value[8..10].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day)) = (year, month, day) else {
        return false;
    };
    if !(1..=12).contains(&month) {
        return false;
    }

    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month).contains(&day)
}

fn civil_date_from_days_since_epoch(days_since_epoch: i64) -> (i64, i64, i64) {
    let shifted_days = days_since_epoch + 719_468;
    let era = if shifted_days >= 0 {
        shifted_days
    } else {
        shifted_days - 146_096
    } / 146_097;
    let day_of_era = shifted_days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

fn today_utc() -> String {
    let days_since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        / 86_400;
    let (year, month, day) = civil_date_from_days_since_epoch(days_since_epoch);
    format!("{year:04}-{month:02}-{day:02}")
}

fn clean_date(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    let Some(date) = clean_string(value, field, 10)? else {
        return Ok(None);
    };
    if !has_date_format(&date) {
        bail!("{field} must use YYYY-MM-DD format");
    }
    if !is_valid_date(&date) {
        bail!("{field} must be a valid calendar date");
    }
    if date < today_utc() {
        bail!("{field} must be today or a future date");
    }
    Ok(Some(date))
}

fn is_integer_string(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn clean_integer(value: Option<&Value>, field: &str, min: i64, max: i64) -> Result<Option<Value>> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() || value.as_str() == Some("") {
        return Ok(None);
    }

    let number = match value {
        Value::String(value) => {
            let trimmed = value.trim();
            if !is_integer_string(trimmed) {
                bail!("{field} must be an integer");
            }
            let number = trimmed.parse::<f64>().unwrap_or(f64::INFINITY);
            if !number.is_finite() {
                bail!("{field} must be an integer");
            }
            number
        }
        Value::Number(value) => {
            let Some(number) = value.as_f64() else {
                bail!("{field} must be an integer");
            };
            if !number.is_finite() || number.fract() != 0.0 {
                bail!("{field} must be an integer");
            }
            number
        }
        _ => bail!("{field} must be an integer"),
    };

    if number < min as f64 || number > max as f64 {
        bail!("{field} must be between {min} and {max}");
    }
    Ok(Some(json!(number as i64)))
}

fn clean_language(value: Option<&Value>) -> Result<Option<String>> {
    let Some(language) = clean_string(value, "lang", 10)? else {
        return Ok(None);
    };
    let normalized = language.to_ascii_lowercase();
    let parts = normalized.split('-').collect::<Vec<_>>();
    let valid = match parts.as_slice() {
        [language] => language.len() == 2 && language.bytes().all(|byte| byte.is_ascii_lowercase()),
        [language, region] => {
            language.len() == 2
                && region.len() == 2
                && language.bytes().all(|byte| byte.is_ascii_lowercase())
                && region.bytes().all(|byte| byte.is_ascii_lowercase())
        }
        _ => false,
    };
    if !valid {
        bail!("lang must be a valid language code such as en, en-us, de, or fr");
    }
    Ok(Some(normalized))
}

fn clean_currency(value: Option<&Value>) -> Result<Option<String>> {
    let Some(currency) = clean_string(value, "currency", 20)? else {
        return Ok(None);
    };
    let normalized = currency.to_ascii_uppercase();
    if normalized.len() != 3 || !normalized.bytes().all(|byte| byte.is_ascii_uppercase()) {
        bail!("currency must be a 3-letter currency code such as USD, EUR, or GBP");
    }
    Ok(Some(normalized))
}

fn add_if_defined(params: &mut BTreeMap<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        params.insert(key.to_owned(), value);
    }
}

fn build_single_booking_search_params(
    input: &Value,
    prefix: &str,
) -> Result<BTreeMap<String, Value>> {
    let field = |name: &str| format!("{prefix}{name}");
    let mut params = BTreeMap::new();
    params.insert(
        "ss".to_owned(),
        Value::String(clean_required_string(input.get("ss"), &field("ss"), 200)?),
    );

    let checkin = clean_date(input.get("checkin"), &field("checkin"))?;
    let checkout = clean_date(input.get("checkout"), &field("checkout"))?;
    if checkin.is_some() != checkout.is_some() {
        bail!(
            "{} and {} must be provided together",
            field("checkin"),
            field("checkout")
        );
    }
    if let (Some(checkin), Some(checkout)) = (&checkin, &checkout) {
        if checkout <= checkin {
            bail!("{} must be after {}", field("checkout"), field("checkin"));
        }
    }

    add_if_defined(&mut params, "checkin", checkin.map(Value::String));
    add_if_defined(&mut params, "checkout", checkout.map(Value::String));
    add_if_defined(
        &mut params,
        "group_adults",
        clean_integer(input.get("group_adults"), &field("group_adults"), 1, 30)?,
    );
    add_if_defined(
        &mut params,
        "group_children",
        clean_integer(input.get("group_children"), &field("group_children"), 0, 20)?,
    );
    add_if_defined(
        &mut params,
        "no_rooms",
        clean_integer(input.get("no_rooms"), &field("no_rooms"), 1, 30)?,
    );
    add_if_defined(
        &mut params,
        "lang",
        clean_language(input.get("lang"))?.map(Value::String),
    );
    add_if_defined(
        &mut params,
        "currency",
        clean_currency(input.get("currency"))?.map(Value::String),
    );
    Ok(params)
}

fn build_booking_search_requests(input: &Value) -> Result<Vec<BookingSearchRequest>> {
    if let Some(searches) = input.get("searches") {
        if let Some(searches) = searches.as_array() {
            if searches.is_empty() {
                bail!("searches must include at least one search");
            }
            if searches.len() > MAX_SEARCHES_PER_RUN {
                bail!("searches cannot include more than {MAX_SEARCHES_PER_RUN} searches per run");
            }

            return searches
                .iter()
                .enumerate()
                .map(|(index, search)| {
                    if !search.is_object() {
                        bail!("searches[{index}] must be an object");
                    }
                    Ok(BookingSearchRequest {
                        params: build_single_booking_search_params(
                            search,
                            &format!("searches[{index}]."),
                        )?,
                        index,
                    })
                })
                .collect();
        }
        bail!("searches must be an array of search objects");
    }

    Ok(vec![BookingSearchRequest {
        params: build_single_booking_search_params(input, "")?,
        index: 0,
    }])
}

fn describe_booking_search_request(params: &BTreeMap<String, Value>) -> String {
    let destination = params
        .get("ss")
        .and_then(Value::as_str)
        .unwrap_or("unknown destination");
    let core = format!("\"{destination}\"");
    let dates = match (params.get("checkin"), params.get("checkout")) {
        (Some(checkin), Some(checkout)) => format!(
            " {} to {}",
            checkin.as_str().unwrap_or_default(),
            checkout.as_str().unwrap_or_default()
        ),
        _ => String::new(),
    };
    let filters = params
        .iter()
        .filter(|(field, _)| !matches!(field.as_str(), "ss" | "checkin" | "checkout"))
        .map(|(field, value)| {
            format!(
                "{field}={}",
                value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string())
            )
        })
        .collect::<Vec<_>>();

    if filters.is_empty() {
        format!("{core}{dates}")
    } else {
        format!("{core}{dates} ({})", filters.join(", "))
    }
}

fn booking_search_url(base_url: &Url, params: &BTreeMap<String, Value>) -> Result<Url> {
    let mut url = endpoint_url(base_url, &["booking", "search"])?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            let value = value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string());
            query.append_pair(key, &value);
        }
    }
    Ok(url)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrappaErrorKind {
    Timeout,
    Network,
    Api(u16),
    InvalidJson,
    Other,
}

#[derive(Debug)]
struct ScrappaError {
    message: String,
    kind: ScrappaErrorKind,
}

impl ScrappaError {
    fn timeout() -> Self {
        Self {
            message: format!(
                "Scrappa API request timed out after {}ms",
                SCRAPPA_REQUEST_TIMEOUT.as_millis()
            ),
            kind: ScrappaErrorKind::Timeout,
        }
    }

    fn network() -> Self {
        Self {
            message: "Scrappa API network request failed".to_owned(),
            kind: ScrappaErrorKind::Network,
        }
    }

    fn retryable(&self) -> bool {
        match self.kind {
            ScrappaErrorKind::Timeout | ScrappaErrorKind::Network => true,
            ScrappaErrorKind::Api(status) => matches!(status, 408 | 429 | 500 | 502 | 503 | 504),
            ScrappaErrorKind::InvalidJson | ScrappaErrorKind::Other => false,
        }
    }
}

impl fmt::Display for ScrappaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ScrappaError {}

fn retry_delay_ms(failed_attempt: u32, jitter_ms: u64) -> u64 {
    let backoff = 1_000u64.saturating_mul(2u64.saturating_pow(failed_attempt));
    backoff.saturating_add(jitter_ms).min(10_000)
}

struct ScrappaClient {
    http: Client,
    base_url: Url,
    api_key: String,
}

impl ScrappaClient {
    fn new(http: Client, base_url: Url, api_key: String) -> Self {
        Self {
            http,
            base_url,
            api_key,
        }
    }

    async fn get(
        &self,
        params: &BTreeMap<String, Value>,
    ) -> std::result::Result<Value, ScrappaError> {
        let url = booking_search_url(&self.base_url, params).map_err(|error| ScrappaError {
            message: error.to_string(),
            kind: ScrappaErrorKind::Other,
        })?;

        for attempt in 1..=SCRAPPA_MAX_ATTEMPTS {
            let result = match timeout(SCRAPPA_REQUEST_TIMEOUT, self.send_once(&url)).await {
                Ok(result) => result,
                Err(_) => Err(ScrappaError::timeout()),
            };
            match result {
                Ok(data) => return Ok(data),
                Err(error) => {
                    if attempt == SCRAPPA_MAX_ATTEMPTS || !error.retryable() {
                        return Err(error);
                    }
                    let jitter_ms = (random::<f64>() * 1_000.0) as u64;
                    let delay_ms = retry_delay_ms(attempt, jitter_ms);
                    eprintln!(
                        "Scrappa API request failed ({error}). Retrying attempt {}/{SCRAPPA_MAX_ATTEMPTS} in {delay_ms}ms.",
                        attempt + 1
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }

        unreachable!("The retry loop returns after its final attempt")
    }

    async fn send_once(&self, url: &Url) -> std::result::Result<Value, ScrappaError> {
        let response = self
            .http
            .get(url.clone())
            .header("X-API-Key", self.api_key.as_str())
            .header(header::ACCEPT, "application/json")
            .header(header::USER_AGENT, "thescrappa-booking-search-scraper/1.0")
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    ScrappaError::timeout()
                } else {
                    ScrappaError::network()
                }
            })?;

        if !response.status().is_success() {
            return Err(scrappa_api_error(response).await);
        }

        response.json().await.map_err(|_| ScrappaError {
            message: "Scrappa API response was not valid JSON".to_owned(),
            kind: ScrappaErrorKind::InvalidJson,
        })
    }
}

fn parsed_scrappa_error_message(body: &str, fallback: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    let mut message = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned();
    if let Some(errors) = value.get("errors").and_then(Value::as_object) {
        let messages = errors
            .iter()
            .filter_map(|(field, values)| {
                let values = values.as_array()?;
                let values = values.iter().filter_map(Value::as_str).collect::<Vec<_>>();
                (!values.is_empty()).then(|| format!("{field}: {}", values.join(", ")))
            })
            .collect::<Vec<_>>();
        if !messages.is_empty() {
            message.push_str(" - ");
            message.push_str(&messages.join("; "));
        }
    }
    Some(message)
}

async fn scrappa_api_error(response: Response) -> ScrappaError {
    let status = response.status();
    let fallback = status
        .canonical_reason()
        .unwrap_or("Unknown status")
        .to_owned();
    let message = match response.text().await {
        Ok(body) if !body.is_empty() => parsed_scrappa_error_message(&body, &fallback)
            .unwrap_or_else(|| {
                body.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(500)
                    .collect()
            }),
        Ok(_) => fallback,
        Err(error) if error.is_timeout() => return ScrappaError::timeout(),
        Err(_) => fallback,
    };
    ScrappaError {
        message: format!("Scrappa API error ({}): {message}", status.as_u16()),
        kind: ScrappaErrorKind::Api(status.as_u16()),
    }
}

fn get_booking_search_results(response: &Value) -> Vec<Value> {
    if let Some(results) = response.pointer("/data/results").and_then(Value::as_array) {
        return results.clone();
    }
    response
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn first_string(property: &Value, names: &[&str]) -> Value {
    names
        .iter()
        .filter_map(|name| property.get(*name).and_then(Value::as_str))
        .find(|value| !value.trim().is_empty())
        .map(|value| Value::String(value.to_owned()))
        .unwrap_or(Value::Null)
}

fn parse_javascript_number(value: &str) -> Option<f64> {
    let value = value.trim();
    if value.is_empty() {
        return Some(0.0);
    }
    let radix_value = if let Some(value) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u64::from_str_radix(value, 16)
            .ok()
            .map(|number| number as f64)
    } else if let Some(value) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        u64::from_str_radix(value, 2)
            .ok()
            .map(|number| number as f64)
    } else if let Some(value) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        u64::from_str_radix(value, 8)
            .ok()
            .map(|number| number as f64)
    } else {
        value.parse::<f64>().ok()
    }?;
    radix_value.is_finite().then_some(radix_value)
}

fn javascript_json_number(value: f64) -> Option<serde_json::Number> {
    if value.fract() == 0.0 {
        if value >= i64::MIN as f64 && value < 9_223_372_036_854_775_808.0 {
            return Some(serde_json::Number::from(value as i64));
        }
        if value >= 0.0 && value < 18_446_744_073_709_551_616.0 {
            return Some(serde_json::Number::from(value as u64));
        }
    }
    serde_json::Number::from_f64(value)
}

fn first_number(property: &Value, name: &str) -> Value {
    let Some(value) = property.get(name) else {
        return Value::Null;
    };
    let number = value.as_f64().or_else(|| {
        value
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .and_then(parse_javascript_number)
    });
    number
        .filter(|number| number.is_finite())
        .and_then(javascript_json_number)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn request_value(params: &BTreeMap<String, Value>, key: &str) -> Value {
    params.get(key).cloned().unwrap_or(Value::Null)
}

fn build_booking_dataset_item(
    property: &Value,
    params: &BTreeMap<String, Value>,
    search_index: usize,
) -> Value {
    let mut item = match property {
        Value::Object(property) => property.clone(),
        Value::Array(property) => property
            .iter()
            .enumerate()
            .map(|(index, value)| (index.to_string(), value.clone()))
            .collect::<Map<String, Value>>(),
        _ => Map::new(),
    };

    item.insert(
        "name".to_owned(),
        first_string(property, &["name", "title"]),
    );
    item.insert("url".to_owned(), first_string(property, &["url", "link"]));
    item.insert(
        "image".to_owned(),
        first_string(property, &["image", "thumbnail"]),
    );
    item.insert(
        "review_score".to_owned(),
        first_number(property, "review_score"),
    );
    item.insert(
        "review_score_word".to_owned(),
        first_string(property, &["review_score_word"]),
    );
    item.insert(
        "review_count".to_owned(),
        first_number(property, "review_count"),
    );
    item.insert(
        "location".to_owned(),
        first_string(property, &["location", "address"]),
    );
    item.insert(
        "price".to_owned(),
        first_string(property, &["price", "price_for_display"]),
    );
    let currency = first_string(property, &["currency"]);
    item.insert(
        "currency".to_owned(),
        if currency.is_null() {
            request_value(params, "currency")
        } else {
            currency
        },
    );
    item.insert("request_search_index".to_owned(), json!(search_index));
    item.insert("request_ss".to_owned(), request_value(params, "ss"));
    item.insert(
        "request_checkin".to_owned(),
        request_value(params, "checkin"),
    );
    item.insert(
        "request_checkout".to_owned(),
        request_value(params, "checkout"),
    );
    item.insert(
        "request_group_adults".to_owned(),
        request_value(params, "group_adults"),
    );
    item.insert(
        "request_group_children".to_owned(),
        request_value(params, "group_children"),
    );
    item.insert(
        "request_no_rooms".to_owned(),
        request_value(params, "no_rooms"),
    );
    item.insert("request_lang".to_owned(), request_value(params, "lang"));
    item.insert(
        "request_currency".to_owned(),
        request_value(params, "currency"),
    );
    Value::Object(item)
}

async fn response_json(response: Response, operation: &str) -> Result<Value> {
    let status = response.status();
    if !status.is_success() {
        let reason = status.canonical_reason().unwrap_or("Unknown status");
        let body = response.text().await.unwrap_or_default();
        let detail = if body.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", body.trim())
        };
        bail!(
            "{operation} failed with {} {reason}{detail}",
            status.as_u16()
        );
    }
    response
        .json()
        .await
        .with_context(|| format!("{operation} returned invalid JSON"))
}

async fn get_input(client: &Client, config: &ActorConfig) -> Result<Option<Value>> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &[
            "v2",
            "key-value-stores",
            &config.default_key_value_store_id,
            "records",
            &config.input_key,
        ],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify INPUT request failed")?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response_json(response, "Apify INPUT request")
        .await
        .map(Some)
}

async fn get_run_info(client: &Client, config: &ActorConfig) -> Result<Value> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .get(url)
        .bearer_auth(&config.apify_token)
        .send()
        .await
        .context("Apify run pricing request failed")?;
    response_json(response, "Apify run pricing request").await
}

fn run_data(run: &Value) -> &Value {
    run.get("data").unwrap_or(run)
}

fn is_pay_per_event(run: &Value) -> bool {
    run_data(run)
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        == Some("PAY_PER_EVENT")
}

#[derive(Debug, PartialEq)]
struct ChargePlan {
    charged_count: usize,
    event_charge_limit_reached: bool,
}

#[derive(Default)]
struct ChargeBudget {
    initial_charged_counts: Option<BTreeMap<String, u64>>,
    saved_dataset_items: u64,
    charged_events: BTreeMap<String, u64>,
}

fn priced_event(events: &Map<String, Value>, name: &str, required: bool) -> Result<Option<f64>> {
    let Some(event) = events.get(name) else {
        if required {
            bail!("Apify run did not provide the {name} event price");
        }
        return Ok(None);
    };
    let price = event
        .get("eventPriceUsd")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("Apify run returned an invalid {name} event price"))?;
    if !price.is_finite() || price < 0.0 {
        bail!("Apify run returned an invalid {name} event price");
    }
    Ok(Some(price))
}

fn charge_plan(
    run: &Value,
    event_name: &str,
    requested: usize,
    budget: &mut ChargeBudget,
) -> Result<ChargePlan> {
    let data = run_data(run);
    if data
        .pointer("/pricingInfo/pricingModel")
        .and_then(Value::as_str)
        != Some("PAY_PER_EVENT")
    {
        bail!("Apify run is not configured for pay-per-event pricing");
    }
    let events = data
        .pointer("/pricingInfo/pricingPerEvent/actorChargeEvents")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Apify run did not provide event prices"))?;
    let event_price = priced_event(events, event_name, true)?.unwrap_or_default();
    let dataset_item_price =
        priced_event(events, DEFAULT_DATASET_ITEM_EVENT, false)?.unwrap_or_default();
    let per_item_price = event_price + dataset_item_price;

    let reported_counts = data
        .get("chargedEventCounts")
        .and_then(Value::as_object)
        .map(|counts| {
            counts
                .iter()
                .map(|(name, count)| {
                    Ok((
                        name.clone(),
                        count.as_u64().ok_or_else(|| {
                            anyhow!("Apify run returned an invalid charged event count for {name}")
                        })?,
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let initial_counts = budget
        .initial_charged_counts
        .get_or_insert_with(|| reported_counts.clone());
    let mut counts = reported_counts;
    if events.contains_key(DEFAULT_DATASET_ITEM_EVENT) {
        let expected_dataset_items = initial_counts
            .get(DEFAULT_DATASET_ITEM_EVENT)
            .copied()
            .unwrap_or(0)
            .checked_add(budget.saved_dataset_items)
            .ok_or_else(|| anyhow!("Dataset row count overflowed"))?;
        counts
            .entry(DEFAULT_DATASET_ITEM_EVENT.to_owned())
            .and_modify(|count| *count = (*count).max(expected_dataset_items))
            .or_insert(expected_dataset_items);
    }
    for (name, locally_charged) in &budget.charged_events {
        let expected_count = initial_counts
            .get(name)
            .copied()
            .unwrap_or(0)
            .checked_add(*locally_charged)
            .ok_or_else(|| anyhow!("Charged event count overflowed for {name}"))?;
        counts
            .entry(name.clone())
            .and_modify(|count| *count = (*count).max(expected_count))
            .or_insert(expected_count);
    }
    let mut spent = 0.0;
    for (name, count) in &counts {
        if *count == 0 {
            continue;
        }
        let price = priced_event(events, name, true)?.unwrap_or_default();
        spent += price * *count as f64;
        if !spent.is_finite() {
            bail!("Apify run returned invalid charged totals");
        }
    }

    let limit_value = data.pointer("/options/maxTotalChargeUsd");
    let max_charge = match limit_value {
        None | Some(Value::Null) => None,
        Some(value) => {
            let limit = value
                .as_f64()
                .filter(|limit| limit.is_finite() && *limit >= 0.0)
                .ok_or_else(|| anyhow!("Apify run returned an invalid spending limit"))?;
            Some(limit)
        }
    };

    let Some(max_charge) = max_charge else {
        return Ok(ChargePlan {
            charged_count: requested,
            event_charge_limit_reached: false,
        });
    };
    if per_item_price == 0.0 {
        return Ok(ChargePlan {
            charged_count: requested,
            event_charge_limit_reached: false,
        });
    }

    let remaining = max_charge - spent;
    let tolerance = f64::EPSILON * max_charge.max(1.0);
    let affordable = if remaining < -tolerance {
        0
    } else {
        let count = ((remaining.max(0.0) + tolerance) / per_item_price).floor();
        if count >= usize::MAX as f64 {
            usize::MAX
        } else {
            count as usize
        }
    };
    Ok(ChargePlan {
        charged_count: requested.min(affordable),
        event_charge_limit_reached: affordable <= requested,
    })
}

async fn ensure_apify_success(response: Response, operation: &str) -> Result<()> {
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let reason = status.canonical_reason().unwrap_or("Unknown status");
    let body = response.text().await.unwrap_or_default();
    let detail = if body.trim().is_empty() {
        String::new()
    } else {
        format!(": {}", body.trim())
    };
    bail!(
        "{operation} failed with {} {reason}{detail}",
        status.as_u16()
    );
}

async fn push_dataset_items(client: &Client, config: &ActorConfig, items: &[Value]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "datasets", &config.default_dataset_id, "items"],
    )?;
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .json(items)
        .send()
        .await
        .context("Apify dataset write failed")?;
    ensure_apify_success(response, "Apify dataset write").await
}

async fn charge_event(
    client: &Client,
    config: &ActorConfig,
    search_index: usize,
    count: usize,
) -> Result<()> {
    if count == 0 {
        return Ok(());
    }
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id, "charge"],
    )?;
    let idempotency_key = format!(
        "{}-{BOOKING_RESULT_CHARGE_EVENT}-{search_index}",
        config.actor_run_id
    );
    let response = client
        .post(url)
        .bearer_auth(&config.apify_token)
        .header("idempotency-key", idempotency_key)
        .json(&json!({ "eventName": BOOKING_RESULT_CHARGE_EVENT, "count": count }))
        .send()
        .await
        .context("Apify event charge request failed")?;
    ensure_apify_success(response, "Apify event charge request").await
}

async fn set_status_message(client: &Client, config: &ActorConfig, message: &str) -> Result<()> {
    let url = endpoint_url(
        &config.apify_api_base_url,
        &["v2", "actor-runs", &config.actor_run_id],
    )?;
    let response = client
        .put(url)
        .bearer_auth(&config.apify_token)
        .json(&json!({
            "runId": config.actor_run_id,
            "statusMessage": message,
            "isStatusMessageTerminal": true
        }))
        .send()
        .await
        .context("Apify run status update failed")?;
    ensure_apify_success(response, "Apify run status update").await
}

fn timeout_failure_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if error
        .downcast_ref::<ScrappaError>()
        .is_some_and(|error| error.kind == ScrappaErrorKind::Timeout)
    {
        format!(
            "{message}. The Booking.com request exceeded the {}s Scrappa API timeout. Try a more specific destination, include check-in/check-out dates, or run the request again.",
            SCRAPPA_REQUEST_TIMEOUT.as_secs()
        )
    } else {
        message
    }
}

async fn run_actor(client: &Client, config: &ActorConfig) -> Result<()> {
    let Some(input) = get_input(client, config).await? else {
        bail!("Input is required");
    };
    if input.is_null() {
        bail!("Input is required");
    }
    let requests = build_booking_search_requests(&input)?;
    println!("Running {} Booking.com search request(s)", requests.len());

    let scrappa = ScrappaClient::new(
        client.clone(),
        config.scrappa_api_base_url.clone(),
        config.scrappa_api_key.clone(),
    );
    let mut total_results = 0;
    let mut charge_budget = ChargeBudget::default();
    for request in &requests {
        println!(
            "Searching Booking.com for {}",
            describe_booking_search_request(&request.params)
        );
        let response = scrappa
            .get(&request.params)
            .await
            .map_err(anyhow::Error::new)?;
        let properties = get_booking_search_results(&response);
        let items = properties
            .iter()
            .map(|property| build_booking_dataset_item(property, &request.params, request.index))
            .collect::<Vec<_>>();

        if !items.is_empty() {
            let run = get_run_info(client, config).await?;
            if is_pay_per_event(&run) {
                let plan = charge_plan(
                    &run,
                    BOOKING_RESULT_CHARGE_EVENT,
                    items.len(),
                    &mut charge_budget,
                )?;
                let charged_items = &items[..plan.charged_count];
                push_dataset_items(client, config, charged_items).await?;
                charge_budget.saved_dataset_items = charge_budget
                    .saved_dataset_items
                    .checked_add(charged_items.len() as u64)
                    .ok_or_else(|| anyhow!("Dataset row count overflowed"))?;
                charge_event(client, config, request.index, plan.charged_count).await?;
                let charged_count = charge_budget
                    .charged_events
                    .entry(BOOKING_RESULT_CHARGE_EVENT.to_owned())
                    .or_default();
                *charged_count = charged_count
                    .checked_add(plan.charged_count as u64)
                    .ok_or_else(|| {
                        anyhow!("Charged event count overflowed for {BOOKING_RESULT_CHARGE_EVENT}")
                    })?;

                if plan.event_charge_limit_reached {
                    let status_message = format!(
                        "Charge limit reached after saving {} of {} Booking.com results for search {}.",
                        plan.charged_count,
                        items.len(),
                        request.index + 1
                    );
                    println!(
                        "{status_message} {}",
                        json!({
                            "event": BOOKING_RESULT_CHARGE_EVENT,
                            "charged_count": plan.charged_count,
                            "requested_count": items.len(),
                            "search_index": request.index,
                        })
                    );
                    set_status_message(client, config, &status_message).await?;
                    return Ok(());
                }
            } else {
                push_dataset_items(client, config, &items).await?;
            }
        }

        total_results += items.len();
        println!(
            "Search {} returned {} Booking.com result(s)",
            request.index + 1,
            items.len()
        );
    }

    println!("Booking.com search completed successfully");
    println!(
        "Results summary: {}",
        json!({ "searches": requests.len(), "results": total_results })
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    let config = match ActorConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Actor failed: {error}");
            std::process::exit(1);
        }
    };
    let client = Client::new();
    if let Err(error) = run_actor(&client, &config).await {
        let message = timeout_failure_message(&error);
        eprintln!("Actor failed: {message}");
        if let Err(status_error) = set_status_message(&client, &config, &message).await {
            eprintln!("Could not set Actor run status message: {status_error}");
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc, Mutex,
        },
        thread,
    };

    struct MockResponse {
        status: u16,
        body: String,
    }

    struct MockServer {
        base_url: Url,
        requests: Arc<Mutex<Vec<String>>>,
        stopped: Arc<AtomicBool>,
        thread: Option<thread::JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let stopped = Arc::new(AtomicBool::new(false));
            let thread_requests = Arc::clone(&requests);
            let thread_stopped = Arc::clone(&stopped);
            let thread = thread::spawn(move || {
                let mut responses = responses.into_iter();
                while !thread_stopped.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let request = read_request(&mut stream).unwrap_or_default();
                            thread_requests.lock().unwrap().push(request);
                            let response = responses.next().unwrap_or_else(|| MockResponse {
                                status: 500,
                                body: "No mock response configured".to_owned(),
                            });
                            let reason = StatusCode::from_u16(response.status)
                                .ok()
                                .and_then(|status| status.canonical_reason())
                                .unwrap_or("Unknown status");
                            let message = format!(
                                "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n{}",
                                response.status,
                                reason,
                                response.body.len(),
                                response.body
                            );
                            let _ = stream.write_all(message.as_bytes());
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                base_url: Url::parse(&format!("http://{address}")).unwrap(),
                requests,
                stopped,
                thread: Some(thread),
            }
        }

        fn requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            self.stopped.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn mock_response(status: u16, body: &str) -> MockResponse {
        MockResponse {
            status,
            body: body.to_owned(),
        }
    }

    fn run_response(max_charge: Value, charged_counts: Value) -> String {
        json!({
            "data": {
                "pricingInfo": {
                    "pricingModel": "PAY_PER_EVENT",
                    "pricingPerEvent": { "actorChargeEvents": {
                        "booking-result": { "eventPriceUsd": 0.01 },
                        "apify-default-dataset-item": { "eventPriceUsd": 0.001 },
                        "apify-actor-start": { "eventPriceUsd": 0.00005 }
                    }}
                },
                "options": { "maxTotalChargeUsd": max_charge },
                "chargedEventCounts": charged_counts
            }
        })
        .to_string()
    }

    fn config(server: &MockServer) -> ActorConfig {
        let mut scrappa_api_base_url = server.base_url.clone();
        scrappa_api_base_url.set_path("/api");
        ActorConfig {
            apify_api_base_url: server.base_url.clone(),
            scrappa_api_base_url,
            default_key_value_store_id: "test-store".to_owned(),
            default_dataset_id: "test-dataset".to_owned(),
            input_key: "INPUT".to_owned(),
            apify_token: "test-token".to_owned(),
            actor_run_id: "test-run".to_owned(),
            scrappa_api_key: "test-key".to_owned(),
        }
    }

    fn http_client() -> Client {
        Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap()
    }

    fn request_parts(request: &str) -> (&str, &str, &str) {
        let (headers, body) = request.split_once("\r\n\r\n").unwrap_or((request, ""));
        let mut parts = headers
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace();
        (
            parts.next().unwrap_or_default(),
            parts.next().unwrap_or_default(),
            body,
        )
    }

    fn has_header(request: &str, name: &str, value: &str) -> bool {
        request
            .split_once("\r\n\r\n")
            .unwrap_or((request, ""))
            .0
            .lines()
            .skip(1)
            .any(|line| {
                line.split_once(':').is_some_and(|(header, actual)| {
                    header.eq_ignore_ascii_case(name) && actual.trim() == value
                })
            })
    }

    fn input_response(input: &Value) -> MockResponse {
        mock_response(200, &input.to_string())
    }

    #[test]
    fn preserves_schema_prefills_and_batch_limits() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(schema["properties"]["ss"]["prefill"], "Paris");
        assert!(schema["properties"]["checkin"].get("prefill").is_none());
        assert!(schema["properties"]["checkout"].get("prefill").is_none());
        let input = json!({"ss": schema["properties"]["ss"]["prefill"]});
        let searches = build_booking_search_requests(&input).unwrap();
        assert_eq!(searches.len(), 1);
        assert!(searches[0].params.get("checkin").is_none());
        assert!(searches[0].params.get("checkout").is_none());
        assert_eq!(
            schema["properties"]["searches"]["maxItems"],
            MAX_SEARCHES_PER_RUN
        );
        assert_eq!(
            schema["properties"]["searches"]["items"]["required"],
            json!(["ss"])
        );
    }

    #[test]
    fn builds_single_and_batch_searches_in_order() {
        let checkin = today_utc();
        let checkout = "2099-12-31";
        let single = build_booking_search_requests(&json!({
            "ss": " Paris ", "checkin": checkin, "checkout": checkout,
            "group_adults": "2", "group_children": 1, "no_rooms": "1",
            "lang": "EN-US", "currency": "eur"
        }))
        .unwrap();
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].params["ss"], "Paris");
        assert_eq!(single[0].params["group_adults"], 2);
        assert_eq!(single[0].params["lang"], "en-us");
        assert_eq!(single[0].params["currency"], "EUR");

        let batch = build_booking_search_requests(&json!({
            "searches": [{"ss": "Paris"}, {"ss": "Berlin"}]
        }))
        .unwrap();
        assert_eq!(
            batch
                .iter()
                .map(|request| request.index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(batch[0].params["ss"], "Paris");
        assert_eq!(batch[1].params["ss"], "Berlin");
    }

    #[test]
    fn rejects_invalid_input_with_existing_messages() {
        assert_eq!(
            build_booking_search_requests(&json!({"ss": ""}))
                .unwrap_err()
                .to_string(),
            "ss is required"
        );
        assert_eq!(
            build_booking_search_requests(&json!({"ss": "Paris", "checkin": "2099-01-01"}))
                .unwrap_err()
                .to_string(),
            "checkin and checkout must be provided together"
        );
        assert_eq!(
            build_booking_search_requests(
                &json!({"ss": "Paris", "checkin": "2099-02-30", "checkout": "2099-03-02"})
            )
            .unwrap_err()
            .to_string(),
            "checkin must be a valid calendar date"
        );
        assert_eq!(
            build_booking_search_requests(
                &json!({"ss": "Paris", "checkin": "2099-03-02", "checkout": "2099-03-01"})
            )
            .unwrap_err()
            .to_string(),
            "checkout must be after checkin"
        );
        assert_eq!(
            build_booking_search_requests(
                &json!({"ss": "Paris", "checkin": "2099-03-02", "checkout": "2099-03-02"})
            )
            .unwrap_err()
            .to_string(),
            "checkout must be after checkin"
        );
        assert_eq!(
            build_booking_search_requests(
                &json!({"ss": "Paris", "checkin": "2020-01-01", "checkout": "2020-01-02"})
            )
            .unwrap_err()
            .to_string(),
            "checkin must be today or a future date"
        );
        assert_eq!(
            build_booking_search_requests(
                &json!({"ss": "Paris", "checkin": "2099/02/01", "checkout": "2099-03-02"})
            )
            .unwrap_err()
            .to_string(),
            "checkin must use YYYY-MM-DD format"
        );
        assert!(
            build_booking_search_requests(&json!({"ss": "Paris", "group_adults": 31}))
                .unwrap_err()
                .to_string()
                .contains("group_adults must be between 1 and 30")
        );
        assert!(
            build_booking_search_requests(&json!({"ss": "Paris", "group_children": -1}))
                .unwrap_err()
                .to_string()
                .contains("group_children must be between 0 and 20")
        );
        assert!(
            build_booking_search_requests(&json!({"ss": "Paris", "no_rooms": 0}))
                .unwrap_err()
                .to_string()
                .contains("no_rooms must be between 1 and 30")
        );
        assert!(
            build_booking_search_requests(&json!({"ss": "Paris", "lang": "english"}))
                .unwrap_err()
                .to_string()
                .contains("lang must be a valid language code")
        );
        assert!(
            build_booking_search_requests(&json!({"ss": "Paris", "currency": "EURO"}))
                .unwrap_err()
                .to_string()
                .contains("currency must be a 3-letter currency code")
        );
        assert!(build_booking_search_requests(&json!({"searches": []}))
            .unwrap_err()
            .to_string()
            .contains("searches must include at least one search"));
        assert!(
            build_booking_search_requests(&json!({"ss": "Paris", "searches": "Paris"}))
                .unwrap_err()
                .to_string()
                .contains("searches must be an array of search objects")
        );
        assert!(
            build_booking_search_requests(&json!({"searches": ["Paris"]}))
                .unwrap_err()
                .to_string()
                .contains("searches[0] must be an object")
        );
        assert!(build_booking_search_requests(
            &json!({"searches": Value::Array((0..26).map(|_| json!({"ss":"Paris"})).collect())})
        )
        .unwrap_err()
        .to_string()
        .contains("more than 25 searches per run"));
    }

    #[test]
    fn descriptions_and_query_encoding_keep_request_fields() {
        let mut params = BTreeMap::new();
        params.insert("ss".to_owned(), json!("New York & Co"));
        params.insert("checkin".to_owned(), json!("2099-01-01"));
        params.insert("checkout".to_owned(), json!("2099-01-04"));
        params.insert("group_adults".to_owned(), json!(2));
        params.insert("currency".to_owned(), json!("EUR"));
        assert_eq!(
            describe_booking_search_request(&params),
            "\"New York & Co\" 2099-01-01 to 2099-01-04 (currency=EUR, group_adults=2)"
        );
        let url =
            booking_search_url(&Url::parse("https://scrappa.co/api").unwrap(), &params).unwrap();
        assert_eq!(url.path(), "/api/booking/search");
        let query = url.query_pairs().collect::<BTreeMap<_, _>>();
        assert_eq!(
            query.get("ss").map(|value| value.as_ref()),
            Some("New York & Co")
        );
        assert_eq!(
            query.get("group_adults").map(|value| value.as_ref()),
            Some("2")
        );
    }

    #[test]
    fn prefers_data_results_then_falls_back_to_top_level_and_normalizes_output() {
        let rows = get_booking_search_results(&json!({
            "data": {"results": [
                {
                    "title":"Hotel One",
                    "link":"https://booking.example/one",
                    "thumbnail":"https://example.test/one.jpg",
                    "review_score":"8.7",
                    "review_score_word":"Excellent",
                    "review_count":"1240",
                    "address":"Paris",
                    "price_for_display":"EUR 420",
                    "extra":"kept"
                },
                {"name":"Hotel Two"}
            ]},
            "results": [{"name":"ignored"}]
        }));
        assert_eq!(rows.len(), 2);
        let params = build_booking_search_requests(&json!({"ss":"Paris", "currency":"EUR"}))
            .unwrap()
            .remove(0)
            .params;
        let first = build_booking_dataset_item(&rows[0], &params, 0);
        assert_eq!(first["name"], "Hotel One");
        assert_eq!(first["url"], "https://booking.example/one");
        assert_eq!(first["image"], "https://example.test/one.jpg");
        assert_eq!(first["review_score"], 8.7);
        assert_eq!(first["review_score_word"], "Excellent");
        assert_eq!(first["review_count"], 1240);
        assert_eq!(first["location"], "Paris");
        assert_eq!(first["price"], "EUR 420");
        assert_eq!(first["currency"], "EUR");
        assert_eq!(first["extra"], "kept");
        assert_eq!(first["request_search_index"], 0);
        assert_eq!(first["request_ss"], "Paris");
        assert!(first["request_checkin"].is_null());
        assert_eq!(
            get_booking_search_results(&json!({"results":[{"name":"fallback"}]})).len(),
            1
        );
        assert!(get_booking_search_results(&json!({"data": {}})).is_empty());
    }

    #[test]
    fn ignores_non_finite_numeric_strings_in_normalized_fields() {
        let params = build_booking_search_requests(&json!({"ss":"Paris"}))
            .unwrap()
            .remove(0)
            .params;
        let item = build_booking_dataset_item(
            &json!({"name":"Hotel Example", "review_score":"Infinity", "review_count":"-Infinity"}),
            &params,
            0,
        );

        assert!(item["review_score"].is_null());
        assert!(item["review_count"].is_null());
    }

    #[test]
    fn validates_retry_policy_and_timeout() {
        assert_eq!(SCRAPPA_MAX_ATTEMPTS, 3);
        assert_eq!(SCRAPPA_REQUEST_TIMEOUT, Duration::from_secs(90));
        assert_eq!(retry_delay_ms(1, 0), 2_000);
        assert_eq!(retry_delay_ms(2, 500), 4_500);
        assert_eq!(retry_delay_ms(1, 1_000), 3_000);
        assert_eq!(retry_delay_ms(8, 1_000), 10_000);
        assert!(ScrappaError {
            message: String::new(),
            kind: ScrappaErrorKind::Api(429)
        }
        .retryable());
        assert!(ScrappaError {
            message: String::new(),
            kind: ScrappaErrorKind::Api(503)
        }
        .retryable());
        assert!(!ScrappaError {
            message: String::new(),
            kind: ScrappaErrorKind::Api(400)
        }
        .retryable());
        assert!(!ScrappaError {
            message: String::new(),
            kind: ScrappaErrorKind::InvalidJson
        }
        .retryable());
    }

    #[test]
    fn charge_plan_counts_existing_events_and_dataset_item_price() {
        let run: Value = serde_json::from_str(&run_response(
            json!(0.03105),
            json!({ "apify-actor-start": 1, "booking-result": 1, "apify-default-dataset-item": 1 }),
        ))
        .unwrap();
        let plan = charge_plan(
            &run,
            BOOKING_RESULT_CHARGE_EVENT,
            10,
            &mut ChargeBudget::default(),
        )
        .unwrap();
        assert_eq!(plan.charged_count, 1);
        assert!(plan.event_charge_limit_reached);

        let unlimited: Value = serde_json::from_str(&run_response(Value::Null, json!({}))).unwrap();
        assert_eq!(
            charge_plan(
                &unlimited,
                BOOKING_RESULT_CHARGE_EVENT,
                2,
                &mut ChargeBudget::default()
            )
            .unwrap(),
            ChargePlan {
                charged_count: 2,
                event_charge_limit_reached: false
            }
        );
    }

    #[test]
    fn charge_plan_accounts_for_successful_local_writes_when_run_metadata_lags() {
        let stale_run: Value = serde_json::from_str(&run_response(
            json!(0.02205),
            json!({ "apify-actor-start": 1 }),
        ))
        .unwrap();
        let mut budget = ChargeBudget::default();

        let first_plan =
            charge_plan(&stale_run, BOOKING_RESULT_CHARGE_EVENT, 1, &mut budget).unwrap();
        assert_eq!(first_plan.charged_count, 1);
        assert!(!first_plan.event_charge_limit_reached);
        budget.saved_dataset_items += 1;
        *budget
            .charged_events
            .entry(BOOKING_RESULT_CHARGE_EVENT.to_owned())
            .or_default() += 1;

        let second_plan =
            charge_plan(&stale_run, BOOKING_RESULT_CHARGE_EVENT, 1, &mut budget).unwrap();
        assert_eq!(second_plan.charged_count, 1);
        assert!(second_plan.event_charge_limit_reached);
        budget.saved_dataset_items += 1;
        *budget
            .charged_events
            .entry(BOOKING_RESULT_CHARGE_EVENT.to_owned())
            .or_default() += 1;

        let third_plan =
            charge_plan(&stale_run, BOOKING_RESULT_CHARGE_EVENT, 1, &mut budget).unwrap();
        assert_eq!(third_plan.charged_count, 0);
        assert!(third_plan.event_charge_limit_reached);
    }

    #[test]
    fn charge_plan_allows_dataset_writes_when_the_synthetic_item_event_is_disabled() {
        let mut run: Value = serde_json::from_str(&run_response(json!(1.0), json!({}))).unwrap();
        run["data"]["pricingInfo"]["pricingPerEvent"]["actorChargeEvents"]
            .as_object_mut()
            .unwrap()
            .remove(DEFAULT_DATASET_ITEM_EVENT);
        let mut budget = ChargeBudget::default();

        let first_plan = charge_plan(&run, BOOKING_RESULT_CHARGE_EVENT, 1, &mut budget).unwrap();
        assert_eq!(first_plan.charged_count, 1);
        budget.saved_dataset_items += 1;
        *budget
            .charged_events
            .entry(BOOKING_RESULT_CHARGE_EVENT.to_owned())
            .or_default() += 1;
        let second_plan = charge_plan(&run, BOOKING_RESULT_CHARGE_EVENT, 1, &mut budget).unwrap();

        assert_eq!(second_plan.charged_count, 1);
        assert!(!second_plan.event_charge_limit_reached);
    }

    #[test]
    fn parses_scrappa_validation_errors_using_the_original_response_shape() {
        assert_eq!(
            parsed_scrappa_error_message(
                r#"{"message":"Invalid request","errors":{"ss":["The destination is required."]}}"#,
                "Unprocessable Entity"
            ),
            Some("Invalid request - ss: The destination is required.".to_owned())
        );
        assert_eq!(
            parsed_scrappa_error_message(r#"{"error":{"message":"Nested error"}}"#, "Bad Request"),
            Some("Bad Request".to_owned())
        );
    }

    #[tokio::test]
    async fn transient_scrappa_status_retries_and_preserves_auth_headers() {
        let server = MockServer::start(vec![
            mock_response(503, r#"{"message":"busy"}"#),
            mock_response(200, r#"{"data":{"results":[]}}"#),
        ]);
        let client =
            ScrappaClient::new(http_client(), server.base_url.clone(), "secret".to_owned());
        let params = build_booking_search_requests(&json!({"ss":"Paris"}))
            .unwrap()
            .remove(0)
            .params;
        assert!(client.get(&params).await.is_ok());
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        assert!(has_header(&requests[0], "X-API-Key", "secret"));
        assert!(has_header(
            &requests[0],
            "User-Agent",
            "thescrappa-booking-search-scraper/1.0"
        ));
        assert!(request_parts(&requests[0])
            .1
            .starts_with("/booking/search?"));
    }

    #[tokio::test]
    async fn scrappa_validation_errors_keep_the_original_message_and_field_details() {
        let server = MockServer::start(vec![mock_response(
            422,
            r#"{"message":"Invalid request","errors":{"ss":["The destination is required."]}}"#,
        )]);
        let client =
            ScrappaClient::new(http_client(), server.base_url.clone(), "secret".to_owned());
        let params = build_booking_search_requests(&json!({"ss":"Paris"}))
            .unwrap()
            .remove(0)
            .params;

        let error = client.get(&params).await.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Scrappa API error (422): Invalid request - ss: The destination is required."
        );
    }

    #[tokio::test]
    async fn paid_batch_writes_results_in_order_and_charges_named_event() {
        let server = MockServer::start(vec![
            input_response(&json!({"searches":[{"ss":"Paris"},{"ss":"Berlin"}]})),
            mock_response(
                200,
                r#"{"data":{"results":[{"name":"Paris A"},{"name":"Paris B"}]}}"#,
            ),
            mock_response(
                200,
                &run_response(json!(10.0), json!({"apify-actor-start":1})),
            ),
            mock_response(201, "{}"),
            mock_response(201, "{}"),
            mock_response(200, r#"{"results":[{"name":"Berlin A"}]}"#),
            mock_response(
                200,
                &run_response(
                    json!(10.0),
                    json!({"apify-actor-start":1,"booking-result":2,"apify-default-dataset-item":2}),
                ),
            ),
            mock_response(201, "{}"),
            mock_response(201, "{}"),
        ]);
        let config = config(&server);
        run_actor(&http_client(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 9);
        assert!(has_header(
            &requests[0],
            "Authorization",
            "Bearer test-token"
        ));
        assert_eq!(request_parts(&requests[3]).0, "POST");
        let first_dataset: Value = serde_json::from_str(request_parts(&requests[3]).2).unwrap();
        assert_eq!(first_dataset[0]["name"], "Paris A");
        assert_eq!(first_dataset[1]["name"], "Paris B");
        assert_eq!(first_dataset[0]["request_search_index"], 0);
        let first_charge: Value = serde_json::from_str(request_parts(&requests[4]).2).unwrap();
        assert_eq!(
            first_charge,
            json!({"eventName":"booking-result", "count":2})
        );
        assert!(has_header(
            &requests[4],
            "idempotency-key",
            "test-run-booking-result-0"
        ));
        let second_dataset: Value = serde_json::from_str(request_parts(&requests[7]).2).unwrap();
        assert_eq!(second_dataset[0]["name"], "Berlin A");
        assert_eq!(second_dataset[0]["request_search_index"], 1);
        assert_eq!(
            request_parts(&requests[5]).1.split('?').next(),
            Some("/api/booking/search")
        );
    }

    #[tokio::test]
    async fn non_pay_per_event_runs_write_every_result_without_a_custom_charge() {
        let server = MockServer::start(vec![
            input_response(&json!({"ss":"Paris"})),
            mock_response(
                200,
                r#"{"data":{"results":[{"name":"One"},{"name":"Two"}]}}"#,
            ),
            mock_response(
                200,
                r#"{"data":{"pricingInfo":{"pricingModel":"PRICE_PER_RESULT"}}}"#,
            ),
            mock_response(201, "{}"),
        ]);
        let config = config(&server);

        run_actor(&http_client(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 4);
        let saved: Value = serde_json::from_str(request_parts(&requests[3]).2).unwrap();
        assert_eq!(saved.as_array().unwrap().len(), 2);
        assert_eq!(saved[0]["name"], "One");
        assert_eq!(saved[1]["name"], "Two");
        assert!(requests.iter().all(|request| !request.contains("/charge")));
    }

    #[tokio::test]
    async fn budget_limit_saves_only_affordable_prefix_then_exits_with_status() {
        let server = MockServer::start(vec![
            input_response(&json!({"ss":"Paris"})),
            mock_response(
                200,
                r#"{"data":{"results":[{"name":"One"},{"name":"Two"},{"name":"Three"}]}}"#,
            ),
            mock_response(
                200,
                &run_response(json!(0.02205), json!({"apify-actor-start":1})),
            ),
            mock_response(201, "{}"),
            mock_response(201, "{}"),
            mock_response(200, "{}"),
        ]);
        let config = config(&server);
        run_actor(&http_client(), &config).await.unwrap();

        let requests = server.requests();
        assert_eq!(requests.len(), 6);
        let dataset: Value = serde_json::from_str(request_parts(&requests[3]).2).unwrap();
        assert_eq!(dataset.as_array().unwrap().len(), 2);
        let charge: Value = serde_json::from_str(request_parts(&requests[4]).2).unwrap();
        assert_eq!(charge["count"], 2);
        assert_eq!(request_parts(&requests[5]).0, "PUT");
        let status: Value = serde_json::from_str(request_parts(&requests[5]).2).unwrap();
        assert_eq!(status["isStatusMessageTerminal"], true);
        assert!(status["statusMessage"].as_str().unwrap().contains("2 of 3"));
    }
}
