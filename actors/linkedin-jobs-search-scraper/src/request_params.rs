use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::{Map, Value};

pub const DEFAULT_QUERY: &str = "software engineer remote";
pub const LINKEDIN_JOBS_SITE_QUERY: &str = "site:linkedin.com/jobs/view/";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SearchInput {
    pub query: Option<String>,
    pub num: Option<u64>,
    pub page: Option<u64>,
    pub start: Option<u64>,
    pub hl: Option<String>,
    pub lr: Option<String>,
    pub gl: Option<String>,
    pub cr: Option<String>,
    pub safe: Option<String>,
    #[serde(rename = "dateRestrict")]
    pub date_restrict: Option<String>,
    pub sort: Option<String>,
    pub filter: Option<i64>,
    pub rights: Option<String>,
}

impl Default for SearchInput {
    fn default() -> Self {
        Self {
            query: Some(DEFAULT_QUERY.to_owned()),
            num: Some(10),
            page: None,
            start: None,
            hl: Some("en".to_owned()),
            lr: None,
            gl: Some("us".to_owned()),
            cr: None,
            safe: Some("off".to_owned()),
            date_restrict: None,
            sort: None,
            filter: None,
            rights: None,
        }
    }
}

pub fn normalize_input(input: Option<Value>) -> Result<SearchInput> {
    let input = match input.filter(|value| !value.is_null()) {
        Some(value) => serde_json::from_value::<SearchInput>(value)
            .map_err(|_| anyhow!("Could not parse Apify INPUT"))?,
        None => SearchInput::default(),
    };
    let defaults = SearchInput::default();

    Ok(SearchInput {
        query: trim_nonempty(input.query).or(defaults.query),
        num: input.num.or(defaults.num),
        page: input.page,
        start: input.start,
        hl: trim_nonempty(input.hl).or(defaults.hl),
        lr: trim_nonempty(input.lr),
        gl: trim_nonempty(input.gl).or(defaults.gl),
        cr: trim_nonempty(input.cr),
        safe: trim_nonempty(input.safe).or(defaults.safe),
        date_restrict: trim_nonempty(input.date_restrict),
        sort: trim_nonempty(input.sort),
        filter: input.filter,
        rights: trim_nonempty(input.rights),
    })
}

pub fn validate_input(input: &SearchInput) -> Result<()> {
    if input.page.is_some() && input.start.is_some() {
        return Err(anyhow!(
            "Use either page or start for pagination, not both."
        ));
    }
    if input.query.as_deref().is_none_or(str::is_empty) {
        return Err(anyhow!("LinkedIn jobs search query is required."));
    }
    Ok(())
}

pub fn build_search_params(input: &SearchInput) -> Result<Map<String, Value>> {
    validate_input(input)?;
    let mut params = Map::new();
    if let Some(query) = input.query.as_deref() {
        params.insert(
            "query".to_owned(),
            Value::String(format!("{LINKEDIN_JOBS_SITE_QUERY} {query}")),
        );
    }
    insert(&mut params, "num", input.num.map(Value::from));
    insert(&mut params, "page", input.page.map(Value::from));
    insert(&mut params, "start", input.start.map(Value::from));
    insert_string(&mut params, "hl", input.hl.as_deref());
    insert_string(&mut params, "lr", input.lr.as_deref());
    insert_string(&mut params, "gl", input.gl.as_deref());
    insert_string(&mut params, "cr", input.cr.as_deref());
    insert_string(&mut params, "safe", input.safe.as_deref());
    insert_string(&mut params, "dateRestrict", input.date_restrict.as_deref());
    insert_string(&mut params, "sort", input.sort.as_deref());
    insert(&mut params, "filter", input.filter.map(Value::from));
    insert_string(&mut params, "rights", input.rights.as_deref());
    Ok(params)
}

pub fn job_search_results(response: &Value) -> Vec<Value> {
    response
        .get("organic_results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn trim_nonempty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn insert(params: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        params.insert(key.to_owned(), value);
    }
}

fn insert_string(params: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        params.insert(key.to_owned(), Value::String(value.to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_search_params, job_search_results, normalize_input, validate_input, SearchInput,
        DEFAULT_QUERY, LINKEDIN_JOBS_SITE_QUERY,
    };
    use serde_json::{json, Value};

    #[test]
    fn empty_and_unknown_inputs_use_the_prefill_defaults() {
        for input in [
            None,
            Some(json!({})),
            Some(json!({"placeholder": true})),
            Some(Value::Null),
        ] {
            assert_eq!(normalize_input(input).unwrap(), SearchInput::default());
        }
    }

    #[test]
    fn partial_targeting_input_keeps_prefill_and_defaults() {
        let input = normalize_input(Some(json!({"gl": " de ", "query": "  "}))).unwrap();
        assert_eq!(input.query.as_deref(), Some(DEFAULT_QUERY));
        assert_eq!(input.num, Some(10));
        assert_eq!(input.hl.as_deref(), Some("en"));
        assert_eq!(input.gl.as_deref(), Some("de"));
        assert_eq!(input.safe.as_deref(), Some("off"));
    }

    #[test]
    fn explicit_search_options_are_trimmed_and_forwarded() {
        let input = normalize_input(Some(json!({
            "query": "  senior engineer berlin ",
            "num": 20,
            "page": 2,
            "hl": " de ",
            "lr": " lang_de ",
            "gl": " de ",
            "cr": " countryDE ",
            "safe": "active",
            "dateRestrict": " m1 ",
            "sort": " date ",
            "filter": 1,
            "rights": " cc_publicdomain "
        })))
        .unwrap();
        let params = build_search_params(&input).unwrap();
        assert_eq!(
            Value::Object(params),
            json!({
                "query": format!("{LINKEDIN_JOBS_SITE_QUERY} senior engineer berlin"),
                "num": 20,
                "page": 2,
                "hl": "de",
                "lr": "lang_de",
                "gl": "de",
                "cr": "countryDE",
                "safe": "active",
                "dateRestrict": "m1",
                "sort": "date",
                "filter": 1,
                "rights": "cc_publicdomain"
            })
        );
    }

    #[test]
    fn start_pagination_is_forwarded_without_page() {
        let input = normalize_input(Some(json!({"query": "cto", "start": 30}))).unwrap();
        assert_eq!(
            Value::Object(build_search_params(&input).unwrap()),
            json!({
                "query": format!("{LINKEDIN_JOBS_SITE_QUERY} cto"),
                "num": 10,
                "start": 30,
                "hl": "en",
                "gl": "us",
                "safe": "off"
            })
        );
    }

    #[test]
    fn page_and_start_are_rejected_together() {
        let input = normalize_input(Some(json!({"page": 1, "start": 0}))).unwrap();
        assert_eq!(
            validate_input(&input).unwrap_err().to_string(),
            "Use either page or start for pagination, not both."
        );
    }

    #[test]
    fn extracts_only_array_organic_results() {
        let result = json!({"position": 1, "title": "Software Engineer"});
        assert_eq!(
            job_search_results(&json!({"organic_results": [result.clone()]})),
            vec![result]
        );
        assert!(job_search_results(&json!({})).is_empty());
        assert!(job_search_results(&json!({ "organic_results": null })).is_empty());
        assert!(job_search_results(&json!({"organic_results": "invalid"})).is_empty());
    }
}
