use std::time::Duration;

pub const ACTOR_TIMEOUT_SECONDS: u64 = 120;
pub const REQUEST_TIMEOUT_MS: u64 = 30_000;
pub const REQUEST_ATTEMPTS: usize = 3;
pub const RETRY_BACKOFF_MS: u64 = 250;
pub const RUN_FINALIZATION_RESERVE_MS: u64 = 20_000;

pub const RETRY_TIME_BUDGET_MS: u64 = REQUEST_ATTEMPTS as u64 * REQUEST_TIMEOUT_MS
    + RETRY_BACKOFF_MS * ((REQUEST_ATTEMPTS * (REQUEST_ATTEMPTS - 1) / 2) as u64);
pub const AVAILABLE_REQUEST_TIME_MS: u64 =
    ACTOR_TIMEOUT_SECONDS * 1_000 - RUN_FINALIZATION_RESERVE_MS;

pub fn retry_budget_fits_actor_timeout() -> bool {
    RETRY_TIME_BUDGET_MS <= AVAILABLE_REQUEST_TIME_MS
}

pub fn request_timeout() -> Duration {
    Duration::from_millis(REQUEST_TIMEOUT_MS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::fs;

    #[test]
    fn retry_policy_fits_actor_timeout_and_keeps_finalization_reserve() {
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
            published_timeout * 1_000 - 20_000
        );
        assert_eq!(RETRY_TIME_BUDGET_MS, 90_750);
        assert!(retry_budget_fits_actor_timeout());
    }
}
