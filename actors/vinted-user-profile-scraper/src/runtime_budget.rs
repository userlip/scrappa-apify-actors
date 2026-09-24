#[cfg(test)]
pub const ACTOR_TIMEOUT_SECS: u64 = 600;
#[cfg(test)]
pub const ACTOR_TIMEOUT_MS: u64 = ACTOR_TIMEOUT_SECS * 1000;
pub const SCRAPPA_REQUEST_TIMEOUT_MS: u64 = 15_000;
pub const SCRAPPA_MAX_ATTEMPTS: usize = 2;
pub const PROFILE_REQUEST_CONCURRENCY: usize = 8;
pub const PROFILE_WORKFLOW_CONCURRENCY: usize = 32;
pub const APIFY_REQUEST_TIMEOUT_MS: u64 = 30_000;
pub const APIFY_CHARGE_MAX_ATTEMPTS: usize = 2;
#[cfg(test)]
pub const RUN_SAFETY_MARGIN_MS: u64 = 30_000;

pub fn retry_delay_ms(failed_attempt: usize) -> u64 {
    let exponential = 1_000_u64.saturating_mul(2_u64.saturating_pow(failed_attempt as u32));
    exponential
        .saturating_add(fastrand::u64(0..=1_000))
        .min(10_000)
}

#[cfg(test)]
pub fn worst_case_apify_profile_persistence_duration_ms() -> u64 {
    APIFY_REQUEST_TIMEOUT_MS
        + APIFY_CHARGE_MAX_ATTEMPTS as u64 * APIFY_REQUEST_TIMEOUT_MS
        + (0..APIFY_CHARGE_MAX_ATTEMPTS.saturating_sub(1))
            .map(|failed_attempt| {
                1_000_u64
                    .saturating_mul(2_u64.saturating_pow(failed_attempt as u32))
                    .saturating_add(1_000)
                    .min(10_000)
            })
            .sum::<u64>()
}

#[cfg(test)]
pub fn worst_case_scrappa_request_duration_ms(attempts: usize) -> u64 {
    let bounded_attempts = attempts.max(1) as u64;
    let retry_delay = (0..attempts.saturating_sub(1))
        .map(|failed_attempt| {
            1_000_u64
                .saturating_mul(2_u64.saturating_pow(failed_attempt as u32))
                .saturating_add(1_000)
                .min(10_000)
        })
        .sum::<u64>();

    bounded_attempts * SCRAPPA_REQUEST_TIMEOUT_MS + retry_delay
}

#[cfg(test)]
pub fn worst_case_run_duration_ms(request_count: usize) -> u64 {
    let waves = request_count.div_ceil(PROFILE_REQUEST_CONCURRENCY) as u64;
    let scrappa_duration = waves * worst_case_scrappa_request_duration_ms(SCRAPPA_MAX_ATTEMPTS);
    let profile_persistence_duration = worst_case_apify_profile_persistence_duration_ms();
    let api_setup_and_status_duration = 2 * APIFY_REQUEST_TIMEOUT_MS;

    scrappa_duration
        + profile_persistence_duration
        + api_setup_and_status_duration
        + RUN_SAFETY_MARGIN_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_maximum_batch_fits_actor_timeout() {
        assert!(
            worst_case_run_duration_ms(100) < ACTOR_TIMEOUT_MS,
            "{}ms does not fit the {}ms actor timeout",
            worst_case_run_duration_ms(100),
            ACTOR_TIMEOUT_MS
        );
        let scrappa_request_duration = worst_case_scrappa_request_duration_ms(SCRAPPA_MAX_ATTEMPTS);
        let profile_workflow_duration =
            scrappa_request_duration + worst_case_apify_profile_persistence_duration_ms();
        assert!(
            PROFILE_WORKFLOW_CONCURRENCY as u64 * scrappa_request_duration
                >= PROFILE_REQUEST_CONCURRENCY as u64 * profile_workflow_duration,
            "profile workers would bottleneck Scrappa throughput"
        );
    }

    #[test]
    fn accounts_for_request_timeout_and_worst_case_retry_delay() {
        assert_eq!(
            worst_case_scrappa_request_duration_ms(2),
            2 * SCRAPPA_REQUEST_TIMEOUT_MS + 2_000
        );
        assert!((1_000..=2_000).contains(&retry_delay_ms(0)));
    }

    #[test]
    fn includes_dataset_write_and_result_charge_deadlines_in_run_budget() {
        assert_eq!(worst_case_apify_profile_persistence_duration_ms(), 92_000);
        assert_eq!(worst_case_run_duration_ms(100), 598_000);
    }
}
