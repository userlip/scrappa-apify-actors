use std::{env, time::Duration};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use tokio::time::Instant;

pub const ACTOR_TIMEOUT_SECONDS: u64 = 120;
pub const REQUEST_TIMEOUT_MS: u64 = 30_000;
pub const REQUEST_ATTEMPTS: usize = 3;
pub const RETRY_BACKOFF_MS: u64 = 250;
pub const APIFY_REQUEST_TIMEOUT_MS: u64 = 15_000;
pub const CHARGE_RETRY_BACKOFF_MS: u64 = 250;
pub const RUN_FINALIZATION_RESERVE_MS: u64 = 20_000;
const SAVE_SAFETY_MARGIN_MS: u64 = 1_000;

#[cfg(test)]
pub const RETRY_TIME_BUDGET_MS: u64 = REQUEST_ATTEMPTS as u64 * REQUEST_TIMEOUT_MS
    + RETRY_BACKOFF_MS * ((REQUEST_ATTEMPTS * (REQUEST_ATTEMPTS - 1) / 2) as u64);
#[cfg(test)]
pub const AVAILABLE_REQUEST_TIME_MS: u64 =
    ACTOR_TIMEOUT_SECONDS * 1_000 - RUN_FINALIZATION_RESERVE_MS;
pub const PPE_ROW_SAVE_BUDGET_MS: u64 =
    APIFY_REQUEST_TIMEOUT_MS + APIFY_REQUEST_TIMEOUT_MS + SAVE_SAFETY_MARGIN_MS;

#[derive(Clone, Copy, Debug)]
pub struct RunDeadline {
    work_until: Instant,
}

impl RunDeadline {
    pub fn from_env() -> Result<Self> {
        let actor_timeout = match env::var("ACTOR_TIMEOUT_AT") {
            Ok(value) => {
                let timeout_at = DateTime::parse_from_rfc3339(&value)
                    .with_context(|| {
                        format!("ACTOR_TIMEOUT_AT is not a valid RFC 3339 date: {value}")
                    })?
                    .with_timezone(&Utc);
                timeout_at
                    .signed_duration_since(Utc::now())
                    .to_std()
                    .unwrap_or_default()
            }
            Err(env::VarError::NotPresent) => Duration::from_secs(ACTOR_TIMEOUT_SECONDS),
            Err(error) => return Err(error).context("Could not read ACTOR_TIMEOUT_AT"),
        };

        Self::for_actor_timeout(actor_timeout)
    }

    pub fn for_actor_timeout(actor_timeout: Duration) -> Result<Self> {
        let work_window =
            actor_timeout.saturating_sub(Duration::from_millis(RUN_FINALIZATION_RESERVE_MS));
        Self::for_work_window(work_window)
    }

    pub fn for_work_window(work_window: Duration) -> Result<Self> {
        let now = Instant::now();
        let work_until = now
            .checked_add(work_window)
            .ok_or_else(|| anyhow!("Actor work deadline is outside the supported time range"))?;
        Ok(Self { work_until })
    }

    pub fn remaining(&self) -> Duration {
        self.work_until.saturating_duration_since(Instant::now())
    }

    pub fn request_timeout(&self, configured: Duration, operation: &str) -> Result<Duration> {
        let remaining = self.remaining();
        if remaining.is_zero() {
            bail!("Actor work deadline reached before {operation}");
        }
        Ok(remaining.min(configured))
    }

    pub fn ensure_remaining(&self, required: Duration, operation: &str) -> Result<()> {
        let remaining = self.remaining();
        if remaining < required {
            bail!(
                "Actor work deadline leaves {}ms, below the {}ms needed for {operation}",
                remaining.as_millis(),
                required.as_millis()
            );
        }
        Ok(())
    }
}

pub fn request_timeout() -> Duration {
    Duration::from_millis(REQUEST_TIMEOUT_MS)
}

pub fn apify_request_timeout() -> Duration {
    Duration::from_millis(APIFY_REQUEST_TIMEOUT_MS)
}

pub fn ppe_row_save_budget() -> Duration {
    Duration::from_millis(PPE_ROW_SAVE_BUDGET_MS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::fs;

    #[test]
    fn derives_the_work_deadline_with_a_finalization_reserve() {
        let deadline = RunDeadline::for_actor_timeout(Duration::from_secs(120)).unwrap();
        assert!(deadline.remaining() <= Duration::from_secs(100));
        assert!(deadline.remaining() > Duration::from_secs(99));
        assert_eq!(PPE_ROW_SAVE_BUDGET_MS, 31_000);

        let expired = RunDeadline::for_work_window(Duration::ZERO).unwrap();
        assert!(expired
            .request_timeout(Duration::from_secs(15), "test request")
            .is_err());
    }

    #[test]
    fn published_timeout_and_retry_budgets_fit_the_reserved_window() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let actor_json: Value = serde_json::from_str(
            &fs::read_to_string(format!("{manifest_dir}/.actor/actor.json")).unwrap(),
        )
        .unwrap();
        let published_timeout = actor_json["defaultRunOptions"]["timeoutSecs"]
            .as_u64()
            .unwrap();

        assert_eq!(published_timeout, ACTOR_TIMEOUT_SECONDS);
        assert_eq!(REQUEST_TIMEOUT_MS, 30_000);
        assert_eq!(REQUEST_ATTEMPTS, 3);
        assert_eq!(
            AVAILABLE_REQUEST_TIME_MS,
            published_timeout * 1_000 - RUN_FINALIZATION_RESERVE_MS
        );
        assert_eq!(RETRY_TIME_BUDGET_MS, 90_750);
        assert!(RETRY_TIME_BUDGET_MS <= AVAILABLE_REQUEST_TIME_MS);
        assert!(PPE_ROW_SAVE_BUDGET_MS <= AVAILABLE_REQUEST_TIME_MS);
    }
}
