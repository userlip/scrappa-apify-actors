use anyhow::{anyhow, Result};
use url::Url;

pub fn base_url_from_env(name: &str, default: &str) -> Result<Url> {
    let raw_url = std::env::var(name).unwrap_or_else(|_| default.to_owned());
    Url::parse(&raw_url).map_err(|error| anyhow!("{name} must be a valid absolute URL: {error}"))
}

pub fn endpoint_url(base_url: &Url, segments: &[&str]) -> Result<Url> {
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|_| anyhow!("API base URL cannot contain path segments"))?
        .pop_if_empty()
        .extend(segments);
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::{endpoint_url, Url};

    #[test]
    fn appends_api_paths_without_losing_base_path() {
        let base = Url::parse("http://127.0.0.1:8080/api").unwrap();
        let url = endpoint_url(&base, &["tiktok", "ads", "details"]).unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:8080/api/tiktok/ads/details");
    }

    #[test]
    fn encodes_ids_as_path_segments() {
        let base = Url::parse("https://api.apify.com").unwrap();
        let url = endpoint_url(&base, &["v2", "datasets", "store/with slash", "items"]).unwrap();
        assert_eq!(url.path(), "/v2/datasets/store%2Fwith%20slash/items");
    }
}
