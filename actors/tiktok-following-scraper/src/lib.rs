mod actor;
mod apify;
mod input;
mod scrappa;
mod url_utils;
mod value;

#[cfg(test)]
mod tests;

use anyhow::Result;
use std::time::Duration;

pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
pub(crate) const PAGE_SIZE: usize = 50;

pub async fn run() -> Result<()> {
    let config = apify::ActorConfig::from_env()?;
    let client = apify::http_client()?;
    actor::run_actor(&client, &config).await
}
