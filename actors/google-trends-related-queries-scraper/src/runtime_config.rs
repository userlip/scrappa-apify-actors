use std::time::Duration;

#[cfg(test)]
pub const ACTOR_TIMEOUT_MS: u64 = 375_000;
#[cfg(test)]
pub const ACTOR_COMPLETION_RESERVE_MS: u64 = 30_000;

pub const APIFY_REQUEST_TIMEOUT_MS: u64 = 30_000;
pub const RELATED_REQUEST_TIMEOUT_MS: u64 = 30_000;
pub const RELATED_MAX_ATTEMPTS: usize = 4;
pub const RELATED_MAX_RETRY_DELAY_MS: u64 = 20_000;

pub const AUTOCOMPLETE_REQUEST_TIMEOUT_MS: u64 = 15_000;
pub const AUTOCOMPLETE_MAX_ATTEMPTS: usize = 1;

#[cfg(test)]
pub fn maximum_request_duration_ms(
    timeout_ms: u64,
    attempts: usize,
    max_retry_delay_ms: u64,
) -> u64 {
    let retry_count = attempts.saturating_sub(1) as u64;
    timeout_ms
        .saturating_mul(attempts.max(1) as u64)
        .saturating_add(max_retry_delay_ms.saturating_mul(retry_count))
}

pub fn related_request_timeout() -> Duration {
    Duration::from_millis(RELATED_REQUEST_TIMEOUT_MS)
}

pub fn related_max_retry_delay() -> Duration {
    Duration::from_millis(RELATED_MAX_RETRY_DELAY_MS)
}

pub fn autocomplete_request_timeout() -> Duration {
    Duration::from_millis(AUTOCOMPLETE_REQUEST_TIMEOUT_MS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn configured_requests_fit_actor_deadline_with_completion_reserve() {
        // INPUT, run pricing, dataset write, then charge plus OUTPUT or terminal status.
        const MAX_APIFY_REQUESTS_PER_RUN: u64 = 5;

        let related = maximum_request_duration_ms(
            RELATED_REQUEST_TIMEOUT_MS,
            RELATED_MAX_ATTEMPTS,
            RELATED_MAX_RETRY_DELAY_MS,
        );
        let apify = APIFY_REQUEST_TIMEOUT_MS.saturating_mul(MAX_APIFY_REQUESTS_PER_RUN);
        let autocomplete = maximum_request_duration_ms(
            AUTOCOMPLETE_REQUEST_TIMEOUT_MS,
            AUTOCOMPLETE_MAX_ATTEMPTS,
            0,
        );

        assert_eq!(related, 180_000);
        assert_eq!(apify, 150_000);
        assert_eq!(autocomplete, 15_000);
        assert_eq!(related + apify + autocomplete, 345_000);
        assert_eq!(
            related + apify + autocomplete + ACTOR_COMPLETION_RESERVE_MS,
            ACTOR_TIMEOUT_MS
        );

        let actor: Value = serde_json::from_str(include_str!("../.actor/actor.json")).unwrap();
        assert_eq!(
            actor["defaultRunOptions"]["timeoutSecs"].as_u64().unwrap() * 1_000,
            ACTOR_TIMEOUT_MS
        );
    }
}
