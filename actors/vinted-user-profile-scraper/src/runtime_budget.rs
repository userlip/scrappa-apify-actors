#[cfg(test)]
pub const ACTOR_TIMEOUT_SECS: u64 = 600;
#[cfg(test)]
pub const ACTOR_TIMEOUT_MS: u64 = ACTOR_TIMEOUT_SECS * 1000;
pub const SCRAPPA_ENTRY_TIME_BUDGET_MS: u64 = 90_000;
pub const SCRAPPA_REQUEST_TIMEOUT_MS: u64 = 15_000;
pub const SCRAPPA_MAX_ATTEMPTS: usize = 1;
pub const APIFY_REQUEST_TIMEOUT_MS: u64 = 30_000;
pub const APIFY_MAX_ATTEMPTS: usize = 2;
pub const PROFILE_REQUEST_CONCURRENCY: usize = 32;
#[cfg(test)]
pub const RUN_SAFETY_MARGIN_MS: u64 = 60_000;
const MAX_RETRY_DELAY_MS: u64 = 10_000;
const MAX_RETRY_JITTER_MS: u64 = 1_000;

pub fn retry_delay_ms(failed_attempt: usize) -> u64 {
    let exponential = 1_000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt as u32));
    exponential
        .saturating_add(fastrand::u64(0..=MAX_RETRY_JITTER_MS))
        .min(MAX_RETRY_DELAY_MS)
}

#[cfg(test)]
fn max_retry_delay_ms(failed_attempt: usize) -> u64 {
    let exponential = 1_000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt as u32));
    exponential
        .saturating_add(MAX_RETRY_JITTER_MS)
        .min(MAX_RETRY_DELAY_MS)
}

#[cfg(test)]
pub fn worst_case_scrappa_request_duration_ms() -> u64 {
    SCRAPPA_ENTRY_TIME_BUDGET_MS
}

#[cfg(test)]
pub fn worst_case_apify_charge_duration_ms(attempts: usize) -> u64 {
    let bounded_attempts = attempts.max(1) as u64;
    let retry_delay = (0..attempts.saturating_sub(1))
        .map(max_retry_delay_ms)
        .sum::<u64>();

    bounded_attempts * APIFY_REQUEST_TIMEOUT_MS + retry_delay
}

#[cfg(test)]
pub fn worst_case_profile_save_duration_ms() -> u64 {
    APIFY_REQUEST_TIMEOUT_MS + worst_case_apify_charge_duration_ms(APIFY_MAX_ATTEMPTS)
}

#[cfg(test)]
pub fn worst_case_run_duration_ms(request_count: usize) -> u64 {
    let waves = request_count.div_ceil(PROFILE_REQUEST_CONCURRENCY) as u64;
    waves * (worst_case_scrappa_request_duration_ms() + worst_case_profile_save_duration_ms())
        + RUN_SAFETY_MARGIN_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefilled_single_profile_fits_actor_timeout() {
        assert_eq!(
            worst_case_scrappa_request_duration_ms(),
            SCRAPPA_ENTRY_TIME_BUDGET_MS
        );
        assert_eq!(
            worst_case_apify_charge_duration_ms(APIFY_MAX_ATTEMPTS),
            62_000
        );
        assert_eq!(worst_case_profile_save_duration_ms(), 92_000);
        assert_eq!(worst_case_run_duration_ms(1), 242_000);
        assert!(
            worst_case_run_duration_ms(1) < ACTOR_TIMEOUT_MS,
            "{}ms does not fit the {}ms actor timeout",
            worst_case_run_duration_ms(1),
            ACTOR_TIMEOUT_MS
        );
    }

    #[test]
    fn accounts_for_request_timeout_and_worst_case_retry_delay() {
        assert_eq!(worst_case_scrappa_request_duration_ms(), 90_000);
        assert!((1_000..=2_000).contains(&retry_delay_ms(0)));
    }
}
