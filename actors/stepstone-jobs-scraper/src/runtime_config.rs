#[cfg(test)]
pub const ACTOR_TIMEOUT_MS: u64 = 240_000;
#[cfg(test)]
pub const ACTOR_COMPLETION_RESERVE_MS: u64 = 30_000;
pub const JOBS_REQUEST_TIMEOUT_MS: u64 = 30_000;
pub const JOBS_MAX_ATTEMPTS: usize = 4;
pub const JOBS_MAX_RETRY_DELAY_MS: u64 = 20_000;

#[cfg(test)]
pub fn maximum_request_duration_ms(
    timeout_ms: u64,
    attempts: usize,
    max_retry_delay_ms: u64,
) -> u64 {
    timeout_ms * attempts as u64 + max_retry_delay_ms * attempts.saturating_sub(1) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn keeps_scrappa_retries_and_completion_within_actor_timeout() {
        let request_maximum_ms = maximum_request_duration_ms(
            JOBS_REQUEST_TIMEOUT_MS,
            JOBS_MAX_ATTEMPTS,
            JOBS_MAX_RETRY_DELAY_MS,
        );
        assert_eq!(request_maximum_ms, 180_000);
        assert_eq!(request_maximum_ms + ACTOR_COMPLETION_RESERVE_MS, 210_000);
        assert!(request_maximum_ms + ACTOR_COMPLETION_RESERVE_MS < ACTOR_TIMEOUT_MS);

        let actor_config: Value =
            serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();
        assert_eq!(
            actor_config["defaultRunOptions"]["timeoutSecs"].as_u64(),
            Some(ACTOR_TIMEOUT_MS / 1000)
        );
    }
}
