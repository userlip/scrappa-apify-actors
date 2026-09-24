use anyhow::{anyhow, bail, Result};
use serde_json::{Map, Number, Value};

const TRIP_TYPES: &[&str] = &["one_way", "round_trip"];
const CABIN_CLASSES: &[&str] = &["economy", "premium_economy", "business", "first"];
const MAX_STOPS_VALUES: &[&str] = &["any", "nonstop", "one_or_fewer", "two_or_fewer"];
const SORT_BY_VALUES: &[&str] = &[
    "top_flights",
    "cheapest",
    "departure_time",
    "arrival_time",
    "duration",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TripType {
    OneWay,
    RoundTrip,
}

impl TripType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OneWay => "one_way",
            Self::RoundTrip => "round_trip",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::OneWay => "one-way",
            Self::RoundTrip => "round-trip",
        }
    }
}

#[derive(Debug)]
pub struct GoogleFlightsRequest {
    pub endpoint: &'static str,
    pub trip_type: TripType,
    pub params: Map<String, Value>,
}

pub fn build_google_flights_request(
    input: &Value,
    today_days: i64,
) -> Result<GoogleFlightsRequest> {
    let object = input
        .as_object()
        .ok_or_else(|| anyhow!("Input must be an object"))?;
    let trip_type = clean_enum(
        object
            .get("trip_type")
            .or(Some(&Value::String("one_way".to_owned()))),
        "trip_type",
        TRIP_TYPES,
    )?
    .map(|value| match value.as_str() {
        "round_trip" => TripType::RoundTrip,
        _ => TripType::OneWay,
    })
    .unwrap_or(TripType::OneWay);
    let origin = clean_airport_code(object.get("origin"), "origin")?;
    let destination = clean_airport_code(object.get("destination"), "destination")?;
    if origin == destination {
        bail!("destination must be different from origin");
    }

    let departure_date = clean_date(object.get("departure_date"), "departure_date", today_days)?;
    let return_date = if trip_type == TripType::RoundTrip {
        Some(clean_date(
            object.get("return_date"),
            "return_date",
            today_days,
        )?)
    } else {
        None
    };
    if return_date
        .as_ref()
        .is_some_and(|return_date| return_date <= &departure_date)
    {
        bail!("return_date must be after departure_date");
    }

    let mut params = Map::new();
    params.insert("origin".to_owned(), Value::String(origin));
    params.insert("destination".to_owned(), Value::String(destination));
    params.insert("departure_date".to_owned(), Value::String(departure_date));
    add_if_some(&mut params, "return_date", return_date.map(Value::String));
    add_if_some(
        &mut params,
        "adults",
        clean_integer(object.get("adults"), "adults", 1, Some(9))?,
    );
    add_if_some(
        &mut params,
        "children",
        clean_integer(object.get("children"), "children", 0, Some(9))?,
    );
    add_if_some(
        &mut params,
        "infants_in_seat",
        clean_integer(object.get("infants_in_seat"), "infants_in_seat", 0, Some(9))?,
    );
    add_if_some(
        &mut params,
        "infants_on_lap",
        clean_integer(object.get("infants_on_lap"), "infants_on_lap", 0, Some(9))?,
    );
    add_if_some(
        &mut params,
        "cabin_class",
        clean_enum(object.get("cabin_class"), "cabin_class", CABIN_CLASSES)?.map(Value::String),
    );
    add_if_some(
        &mut params,
        "exclude_basic",
        clean_boolean(object.get("exclude_basic"), "exclude_basic")?,
    );
    add_if_some(
        &mut params,
        "max_stops",
        clean_enum(object.get("max_stops"), "max_stops", MAX_STOPS_VALUES)?.map(Value::String),
    );
    add_if_some(
        &mut params,
        "sort_by",
        clean_enum(object.get("sort_by"), "sort_by", SORT_BY_VALUES)?.map(Value::String),
    );
    add_if_some(
        &mut params,
        "airlines",
        clean_airlines(object.get("airlines"))?.map(Value::String),
    );
    add_if_some(
        &mut params,
        "include_baggage",
        clean_boolean(object.get("include_baggage"), "include_baggage")?,
    );
    add_if_some(
        &mut params,
        "hl",
        clean_language_code(object.get("hl"))?.map(Value::String),
    );
    add_if_some(
        &mut params,
        "gl",
        clean_country_code(object.get("gl"))?.map(Value::String),
    );
    add_if_some(
        &mut params,
        "currency",
        clean_currency(object.get("currency"))?.map(Value::String),
    );

    for (field, min, max) in [
        ("departure_time_min", 0, Some(23)),
        ("departure_time_max", 0, Some(23)),
        ("arrival_time_min", 0, Some(23)),
        ("arrival_time_max", 0, Some(23)),
        ("max_duration_minutes", 1, None),
        ("max_price", 1, None),
    ] {
        add_if_some(
            &mut params,
            field,
            clean_integer(object.get(field), field, min, max)?,
        );
    }

    for (minimum, maximum, message) in [
        (
            "departure_time_min",
            "departure_time_max",
            "departure_time_max must be greater than or equal to departure_time_min",
        ),
        (
            "arrival_time_min",
            "arrival_time_max",
            "arrival_time_max must be greater than or equal to arrival_time_min",
        ),
    ] {
        if let (Some(minimum), Some(maximum)) = (
            params.get(minimum).and_then(Value::as_i64),
            params.get(maximum).and_then(Value::as_i64),
        ) {
            if maximum < minimum {
                bail!("{message}");
            }
        }
    }

    Ok(GoogleFlightsRequest {
        endpoint: if trip_type == TripType::RoundTrip {
            "/flights/round-trip"
        } else {
            "/flights/one-way"
        },
        trip_type,
        params,
    })
}

pub fn describe_google_flights_request(request: &GoogleFlightsRequest) -> String {
    let mut parts = vec![
        format!(
            "{} to {}",
            value_string(request.params.get("origin")),
            value_string(request.params.get("destination"))
        ),
        format!(
            "departing {}",
            value_string(request.params.get("departure_date"))
        ),
    ];
    if let Some(return_date) = request.params.get("return_date") {
        parts.push(format!("returning {}", value_string(Some(return_date))));
    }
    format!(
        "{} flight search ({})",
        request.trip_type.label(),
        parts.join(", ")
    )
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
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

fn clean_enum(value: Option<&Value>, field: &str, values: &[&str]) -> Result<Option<String>> {
    let Some(value) = clean_string(value, field, 40)? else {
        return Ok(None);
    };
    let normalized = value.to_ascii_lowercase();
    if !values.contains(&normalized.as_str()) {
        bail!("{field} must be one of: {}", values.join(", "));
    }
    Ok(Some(normalized))
}

fn clean_airport_code(value: Option<&Value>, field: &str) -> Result<String> {
    let code = clean_required_string(value, field, 3)?.to_ascii_uppercase();
    if code.len() != 3 || !code.bytes().all(|byte| byte.is_ascii_uppercase()) {
        bail!("{field} must be a 3-letter IATA airport code");
    }
    Ok(code)
}

fn clean_date(value: Option<&Value>, field: &str, today_days: i64) -> Result<String> {
    let date = clean_required_string(value, field, 20)?;
    if let Some((amount, unit)) = relative_date(&date) {
        let amount = amount.ok_or_else(|| anyhow!("Invalid time value"))?;
        let amount = i64::try_from(amount).map_err(|_| anyhow!("Invalid time value"))?;
        let resolved_days = match unit {
            "day" => today_days.checked_add(amount),
            "week" => amount
                .checked_mul(7)
                .and_then(|days| today_days.checked_add(days)),
            "month" => add_months(today_days, amount),
            _ => add_years(today_days, amount),
        }
        .filter(|days| days.unsigned_abs() <= 100_000_000)
        .ok_or_else(|| anyhow!("Invalid time value"))?;
        return Ok(format_days(resolved_days));
    }
    if date.len() != 10
        || !date.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'-',
            _ => byte.is_ascii_digit(),
        })
    {
        bail!("{field} must be in YYYY-MM-DD format or a relative date such as 45 days");
    }
    let parsed = parse_absolute_date(&date)
        .ok_or_else(|| anyhow!("{field} must be a valid calendar date"))?;
    Ok(format_days(parsed))
}

fn relative_date(value: &str) -> Option<(Option<u64>, &'static str)> {
    let value = value.to_ascii_lowercase();
    for unit in ["day", "week", "month", "year"] {
        let without_plural = value.strip_suffix('s').unwrap_or(&value);
        let Some(amount) = without_plural.strip_suffix(unit) else {
            continue;
        };
        let amount = amount.trim_end();
        if amount.is_empty() || !amount.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        return Some((amount.parse().ok(), unit));
    }
    None
}

fn parse_absolute_date(value: &str) -> Option<i64> {
    if value.len() != 10
        || value.as_bytes().get(4) != Some(&b'-')
        || value.as_bytes().get(7) != Some(&b'-')
    {
        return None;
    }
    let year = value[0..4].parse::<i64>().ok()?;
    let month = value[5..7].parse::<i64>().ok()?;
    let day = value[8..10].parse::<i64>().ok()?;
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn add_months(days: i64, amount: i64) -> Option<i64> {
    let (year, month, day) = civil_from_days(days);
    let absolute_month = year
        .checked_mul(12)?
        .checked_add(month - 1)?
        .checked_add(amount)?;
    let target_year = absolute_month.div_euclid(12);
    let target_month = absolute_month.rem_euclid(12) + 1;
    Some(days_from_civil(target_year, target_month, 1).checked_add(day - 1)?)
}

fn add_years(days: i64, amount: i64) -> Option<i64> {
    let (year, month, day) = civil_from_days(days);
    let target_year = year.checked_add(amount)?;
    Some(days_from_civil(target_year, month, 1).checked_add(day - 1)?)
}

fn format_days(days: i64) -> String {
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn is_leap_year(year: i64) -> bool {
    year.rem_euclid(4) == 0 && (year.rem_euclid(100) != 0 || year.rem_euclid(400) == 0)
}

fn days_from_civil(mut year: i64, month: i64, day: i64) -> i64 {
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let adjusted_days = days + 719_468;
    let era = adjusted_days.div_euclid(146_097);
    let day_of_era = adjusted_days - era * 146_097;
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

fn clean_integer(
    value: Option<&Value>,
    field: &str,
    min: i64,
    max: Option<i64>,
) -> Result<Option<Value>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(number) = value.as_number() else {
        bail!("{field} must be an integer");
    };
    let numeric = number
        .as_i64()
        .or_else(|| {
            number
                .as_u64()
                .and_then(|number| i64::try_from(number).ok())
        })
        .or_else(|| {
            number
                .as_f64()
                .filter(|number| number.is_finite() && number.fract() == 0.0)
                .and_then(|number| i64::try_from(number as i128).ok())
        })
        .ok_or_else(|| anyhow!("{field} must be an integer"))?;
    if numeric < min || max.is_some_and(|max| numeric > max) {
        if let Some(max) = max {
            bail!("{field} must be between {min} and {max}");
        }
        bail!("{field} must be greater than or equal to {min}");
    }
    Ok(Some(Value::Number(Number::from(numeric))))
}

fn clean_boolean(value: Option<&Value>, field: &str) -> Result<Option<Value>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let Some(value) = value.as_bool() else {
        bail!("{field} must be a boolean");
    };
    Ok(Some(Value::Bool(value)))
}

fn clean_language_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(value) = clean_string(value, "hl", 10)? else {
        return Ok(None);
    };
    let normalized = value.to_ascii_lowercase();
    let mut parts = normalized.split('-');
    let language = parts.next().unwrap_or_default();
    let region = parts.next();
    if language.len() != 2
        || !language.bytes().all(|byte| byte.is_ascii_lowercase())
        || region.is_some_and(|region| {
            region.len() != 2 || !region.bytes().all(|byte| byte.is_ascii_lowercase())
        })
        || parts.next().is_some()
    {
        bail!("hl must be a valid language code such as en, de, or en-us");
    }
    Ok(Some(normalized))
}

fn clean_country_code(value: Option<&Value>) -> Result<Option<String>> {
    let Some(value) = clean_string(value, "gl", 2)? else {
        return Ok(None);
    };
    if value.len() != 2 || !value.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        bail!("gl must be a two-letter country code");
    }
    Ok(Some(value.to_ascii_lowercase()))
}

fn clean_currency(value: Option<&Value>) -> Result<Option<String>> {
    let Some(value) = clean_string(value, "currency", 3)? else {
        return Ok(None);
    };
    let normalized = value.to_ascii_uppercase();
    if normalized.len() != 3 || !normalized.bytes().all(|byte| byte.is_ascii_uppercase()) {
        bail!("currency must be a 3-letter currency code such as USD, EUR, or GBP");
    }
    Ok(Some(normalized))
}

fn clean_airlines(value: Option<&Value>) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if value.as_str() == Some("") {
        return Ok(None);
    }
    let codes: Vec<&Value> = if let Some(values) = value.as_array() {
        values.iter().collect()
    } else if let Some(value) = value.as_str() {
        let values = value
            .split(',')
            .map(|value| Value::String(value.to_owned()))
            .collect::<Vec<_>>();
        return normalize_airline_values(&values);
    } else {
        bail!("airlines must be an array of airline codes or a comma-separated string");
    };
    normalize_airline_refs(&codes)
}

fn normalize_airline_values(values: &[Value]) -> Result<Option<String>> {
    let references = values.iter().collect::<Vec<_>>();
    normalize_airline_refs(&references)
}

fn normalize_airline_refs(values: &[&Value]) -> Result<Option<String>> {
    let mut codes = Vec::with_capacity(values.len());
    for value in values {
        let Some(code) = value.as_str() else {
            bail!("airlines must contain only strings");
        };
        let normalized = code.trim().to_ascii_uppercase();
        if normalized.len() != 2
            || !normalized
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        {
            bail!("airlines must contain 2-character IATA airline codes");
        }
        codes.push(normalized);
    }
    if codes.is_empty() {
        Ok(None)
    } else {
        Ok(Some(codes.join(",")))
    }
}

fn add_if_some(params: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        params.insert(key.to_owned(), value);
    }
}

fn value_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => value.clone(),
        Some(value) => value.to_string(),
        None => "undefined".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn today() -> i64 {
        days_from_civil(2026, 9, 4)
    }

    #[test]
    fn builds_complete_round_trip_params_and_normalizes_values() {
        let request = build_google_flights_request(
            &json!({
                "trip_type": "round_trip",
                "origin": " jfk ",
                "destination": " lax ",
                "departure_date": "2026-09-15",
                "return_date": "2026-09-22",
                "adults": 2,
                "children": 1,
                "cabin_class": "BUSINESS",
                "exclude_basic": true,
                "max_stops": "one_or_fewer",
                "sort_by": "cheapest",
                "airlines": [" dl ", "AA"],
                "include_baggage": true,
                "currency": "usd",
                "hl": "EN",
                "gl": "US",
                "departure_time_min": 6,
                "departure_time_max": 18,
                "arrival_time_min": 9,
                "arrival_time_max": 22,
                "max_duration_minutes": 600,
                "max_price": 800
            }),
            today(),
        )
        .unwrap();

        assert_eq!(request.endpoint, "/flights/round-trip");
        assert_eq!(request.trip_type, TripType::RoundTrip);
        assert_eq!(request.params["origin"], "JFK");
        assert_eq!(request.params["destination"], "LAX");
        assert_eq!(request.params["airlines"], "DL,AA");
        assert_eq!(request.params["cabin_class"], "business");
        assert_eq!(request.params["currency"], "USD");
        assert_eq!(request.params["hl"], "en");
        assert_eq!(request.params["gl"], "us");
        assert_eq!(request.params["max_price"], 800);
    }

    #[test]
    fn uses_default_one_way_and_resolves_relative_dates() {
        let request = build_google_flights_request(
            &json!({
                "origin": "SFO",
                "destination": "CDG",
                "departure_date": "45 days"
            }),
            today(),
        )
        .unwrap();
        assert_eq!(request.endpoint, "/flights/one-way");
        assert_eq!(request.params["departure_date"], "2026-10-19");

        let round_trip = build_google_flights_request(
            &json!({
                "trip_type": "round_trip",
                "origin": "JFK",
                "destination": "LAX",
                "departure_date": "45 days",
                "return_date": "52 days"
            }),
            today(),
        )
        .unwrap();
        assert_eq!(round_trip.params["return_date"], "2026-10-26");
    }

    #[test]
    fn validates_required_route_date_and_filters() {
        for (input, expected) in [
            (
                json!({"origin":"JF","destination":"LAX","departure_date":"2026-09-15"}),
                "origin must be a 3-letter IATA",
            ),
            (
                json!({"origin":"JFK","destination":"JFK","departure_date":"2026-09-15"}),
                "destination must be different",
            ),
            (
                json!({"trip_type":"round_trip","origin":"JFK","destination":"LAX","departure_date":"2026-09-15"}),
                "return_date is required",
            ),
            (
                json!({"origin":"JFK","destination":"LAX","departure_date":"2026-02-30"}),
                "departure_date must be a valid calendar date",
            ),
            (
                json!({"origin":"JFK","destination":"LAX","departure_date":"next Tuesday"}),
                "departure_date must be in YYYY-MM-DD",
            ),
            (
                json!({"origin":"JFK","destination":"LAX","departure_date":"2026-09-15","adults":10}),
                "adults must be between 1 and 9",
            ),
            (
                json!({"origin":"JFK","destination":"LAX","departure_date":"2026-09-15","cabin_class":"coach"}),
                "cabin_class must be one of",
            ),
            (
                json!({"origin":"JFK","destination":"LAX","departure_date":"2026-09-15","airlines":"AAL"}),
                "airlines must contain 2-character",
            ),
            (
                json!({"origin":"JFK","destination":"LAX","departure_date":"2026-09-15","currency":"US"}),
                "currency must be a 3-letter",
            ),
            (
                json!({"origin":"JFK","destination":"LAX","departure_date":"2026-09-15","departure_time_min":20,"departure_time_max":6}),
                "departure_time_max must be greater than or equal",
            ),
        ] {
            let error = build_google_flights_request(&input, today()).unwrap_err();
            assert!(
                error.to_string().contains(expected),
                "{} did not contain {expected}",
                error
            );
        }
    }

    #[test]
    fn month_relative_date_matches_javascript_overflow() {
        let february = days_from_civil(2026, 1, 31);
        assert_eq!(format_days(add_months(february, 1).unwrap()), "2026-03-03");
    }

    #[test]
    fn describes_one_way_and_round_trip_requests() {
        let request = build_google_flights_request(
            &json!({"origin":"JFK","destination":"LAX","departure_date":"2026-09-15"}),
            today(),
        )
        .unwrap();
        assert_eq!(
            describe_google_flights_request(&request),
            "one-way flight search (JFK to LAX, departing 2026-09-15)"
        );
    }

    #[test]
    fn actor_schema_keeps_required_fields_and_input_prefills() {
        let schema: Value =
            serde_json::from_str(include_str!("../.actor/input_schema.json")).unwrap();
        assert_eq!(
            schema["required"],
            json!(["origin", "destination", "departure_date"])
        );
        assert_eq!(schema["properties"]["trip_type"]["default"], "one_way");
        assert_eq!(schema["properties"]["origin"]["prefill"], "JFK");
        assert_eq!(schema["properties"]["destination"]["prefill"], "LAX");
        assert_eq!(schema["properties"]["departure_date"]["prefill"], "45 days");
        assert_eq!(schema["properties"]["return_date"]["prefill"], "52 days");
        assert_eq!(schema["properties"]["adults"]["default"], 1);
        assert_eq!(schema["properties"]["cabin_class"]["default"], "economy");
    }
}
