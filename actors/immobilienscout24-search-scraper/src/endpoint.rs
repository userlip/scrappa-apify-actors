use anyhow::{anyhow, Context, Result};
use url::Url;

pub(crate) fn endpoint_url(base: &str, segments: &[&str]) -> Result<Url> {
    let mut url = Url::parse(&format!("{}/", base.trim_end_matches('/')))
        .with_context(|| format!("Invalid API base URL: {base}"))?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| anyhow!("API base URL cannot contain query or fragment: {base}"))?;
        path.pop_if_empty();
        path.extend(segments.iter().copied());
    }
    Ok(url)
}
