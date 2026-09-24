use anyhow::{anyhow, Result};
use serde_json::{Map, Value};
use url::Url;

const MAX_HOTELS_PER_RUN: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookingHotelRequest {
    pub params: Map<String, Value>,
    pub index: usize,
    pub input_type: &'static str,
}

pub fn build_booking_hotel_requests(input: &Value) -> Result<Vec<BookingHotelRequest>> {
    let mut requests = Vec::new();

    if ["url", "country", "slug"]
        .iter()
        .any(|key| input.get(key).is_some())
    {
        requests.push(build_single_request(input, requests.len(), "")?);
    }

    if let Some(urls) = input.get("urls") {
        let urls = urls
            .as_array()
            .ok_or_else(|| anyhow!("urls must be an array of Booking.com hotel URLs"))?;
        for (index, url) in urls.iter().enumerate() {
            let mut request = Map::new();
            request.insert("url".into(), url.clone());
            requests.push(build_single_request(
                &Value::Object(request),
                requests.len(),
                &format!("urls[{index}]."),
            )?);
        }
    }

    if let Some(hotels) = input.get("hotels") {
        let hotels = hotels
            .as_array()
            .ok_or_else(|| anyhow!("hotels must be an array of hotel request objects"))?;
        for (index, hotel) in hotels.iter().enumerate() {
            if !hotel.is_object() {
                return Err(anyhow!("hotels[{index}] must be an object"));
            }
            requests.push(build_single_request(
                hotel,
                requests.len(),
                &format!("hotels[{index}]."),
            )?);
        }
    }

    if requests.is_empty() {
        return Err(anyhow!(
            "Provide at least one hotel URL or country/slug pair"
        ));
    }
    if requests.len() > MAX_HOTELS_PER_RUN {
        return Err(anyhow!(
            "Hotel batches cannot include more than {MAX_HOTELS_PER_RUN} hotels per run"
        ));
    }

    Ok(requests)
}

fn build_single_request(input: &Value, index: usize, prefix: &str) -> Result<BookingHotelRequest> {
    if let Some(url) = clean_booking_url(input.get("url"), &format!("{prefix}url"))? {
        let mut params = Map::new();
        params.insert("url".into(), Value::String(url));
        return Ok(BookingHotelRequest {
            params,
            index,
            input_type: "url",
        });
    }

    let country = clean_country(input.get("country"), &format!("{prefix}country"))?;
    let slug = clean_slug(input.get("slug"), &format!("{prefix}slug"))?;
    if country.is_some() || slug.is_some() {
        let (Some(country), Some(slug)) = (country, slug) else {
            return Err(anyhow!(
                "{prefix}country and {prefix}slug must be provided together"
            ));
        };
        let mut params = Map::new();
        params.insert("country".into(), Value::String(country));
        params.insert("slug".into(), Value::String(slug));
        return Ok(BookingHotelRequest {
            params,
            index,
            input_type: "country_slug",
        });
    }

    Err(anyhow!("{prefix}url or country plus slug is required"))
}

fn clean_booking_url(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    let Some(url) = clean_string(value, field, 2048)? else {
        return Ok(None);
    };
    let parsed = Url::parse(&url).map_err(|_| anyhow!("{field} must be a valid URL"))?;
    if !is_booking_hotel_url(&parsed) {
        return Err(anyhow!("{field} must be a Booking.com hotel URL"));
    }
    Ok(Some(parsed.to_string()))
}

fn is_booking_hotel_url(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return false;
    }
    if url.port().is_some() || !url.path().starts_with("/hotel/") {
        return false;
    }

    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    let Some(subdomains) = host.strip_suffix(".booking.com") else {
        return host == "booking.com";
    };
    !subdomains.is_empty()
        && subdomains.split('.').all(|label| {
            !label.is_empty()
                && label
                    .bytes()
                    .all(|character| character.is_ascii_alphanumeric() || character == b'-')
        })
}

fn clean_country(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    let Some(country) = clean_string(value, field, 2)? else {
        return Ok(None);
    };
    if country.len() != 2
        || !country
            .bytes()
            .all(|character| character.is_ascii_alphabetic())
    {
        return Err(anyhow!("{field} must be a 2-letter country code"));
    }
    Ok(Some(country.to_ascii_lowercase()))
}

fn clean_slug(value: Option<&Value>, field: &str) -> Result<Option<String>> {
    let Some(slug) = clean_string(value, field, 255)? else {
        return Ok(None);
    };
    if slug.len() < 2 {
        return Err(anyhow!("{field} must be at least 2 characters"));
    }
    if !slug
        .bytes()
        .all(|character| character.is_ascii_alphanumeric() || b"._-".contains(&character))
    {
        return Err(anyhow!(
            "{field} must contain only letters, numbers, dots, underscores, and dashes, with optional .html"
        ));
    }
    Ok(Some(slug))
}

fn clean_string(value: Option<&Value>, field: &str, max_length: usize) -> Result<Option<String>> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Err(anyhow!("{field} must be a string"));
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.encode_utf16().count() > max_length {
        return Err(anyhow!("{field} must be {max_length} characters or fewer"));
    }
    Ok(Some(value.to_owned()))
}

pub fn describe_booking_hotel_request(request: &BookingHotelRequest) -> String {
    if let Some(url) = request.params.get("url").and_then(Value::as_str) {
        return url.to_owned();
    }
    format!(
        "{}/{}",
        request
            .params
            .get("country")
            .and_then(Value::as_str)
            .unwrap_or(""),
        request
            .params
            .get("slug")
            .and_then(Value::as_str)
            .unwrap_or("")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_single_url_and_country_slug_requests() {
        let requests = build_booking_hotel_requests(&json!({
            "url": " https://www.booking.com/hotel/fr/ritz-paris.html ",
            "country": "invalid-but-ignored",
            "slug": 17
        }))
        .unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].index, 0);
        assert_eq!(requests[0].input_type, "url");
        assert_eq!(
            requests[0].params["url"],
            "https://www.booking.com/hotel/fr/ritz-paris.html"
        );

        let requests = build_booking_hotel_requests(&json!({
            "country": "FR",
            "slug": "ritz-paris.html"
        }))
        .unwrap();
        assert_eq!(requests[0].input_type, "country_slug");
        assert_eq!(requests[0].params["country"], "fr");
        assert_eq!(requests[0].params["slug"], "ritz-paris.html");
    }

    #[test]
    fn combines_single_and_batch_inputs_in_order_with_contiguous_indexes() {
        let requests = build_booking_hotel_requests(&json!({
            "url": "https://www.booking.com/hotel/fr/ritz.html",
            "urls": ["https://de.booking.com/hotel/de/sample.html?lang=en-us"],
            "hotels": [
                {"country": "gb", "slug": "london_sample"},
                {"url": "https://www.booking.com/hotel/us/example.html", "country": "fr", "slug": "ignored"}
            ]
        }))
        .unwrap();
        assert_eq!(requests.len(), 4);
        assert_eq!(
            requests
                .iter()
                .map(|request| request.index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(
            requests[1].params["url"],
            "https://de.booking.com/hotel/de/sample.html?lang=en-us"
        );
        assert_eq!(requests[2].params["slug"], "london_sample");
        assert_eq!(
            requests[3].params["url"],
            "https://www.booking.com/hotel/us/example.html"
        );
    }

    #[test]
    fn validates_input_types_shapes_and_ten_hotel_limit() {
        assert_eq!(
            build_booking_hotel_requests(&json!({}))
                .unwrap_err()
                .to_string(),
            "Provide at least one hotel URL or country/slug pair"
        );
        assert_eq!(
            build_booking_hotel_requests(&json!({"url": "https://example.com/hotel/foo"}))
                .unwrap_err()
                .to_string(),
            "url must be a Booking.com hotel URL"
        );
        assert!(build_booking_hotel_requests(&json!({"country": "fr"}))
            .unwrap_err()
            .to_string()
            .contains("country and slug must be provided together"));
        assert!(
            build_booking_hotel_requests(&json!({"country": "fra", "slug": "hotel"}))
                .unwrap_err()
                .to_string()
                .contains("country must be 2 characters or fewer")
        );
        assert!(build_booking_hotel_requests(&json!({"urls": null}))
            .unwrap_err()
            .to_string()
            .contains("urls must be an array"));
        assert!(
            build_booking_hotel_requests(&json!({"hotels": ["not an object"]}))
                .unwrap_err()
                .to_string()
                .contains("hotels[0] must be an object")
        );
        let urls = vec!["https://www.booking.com/hotel/fr/example.html"; 11];
        assert!(build_booking_hotel_requests(&json!({"urls": urls}))
            .unwrap_err()
            .to_string()
            .contains("cannot include more than 10 hotels"));
    }

    #[test]
    fn describes_both_request_forms() {
        let requests = build_booking_hotel_requests(&json!({
            "urls": ["https://www.booking.com/hotel/fr/ritz.html"],
            "hotels": [{"country": "de", "slug": "sample"}]
        }))
        .unwrap();
        assert_eq!(
            describe_booking_hotel_request(&requests[0]),
            "https://www.booking.com/hotel/fr/ritz.html"
        );
        assert_eq!(describe_booking_hotel_request(&requests[1]), "de/sample");
    }
}
