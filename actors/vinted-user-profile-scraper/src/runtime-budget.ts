export const ACTOR_TIMEOUT_SECS = 600;
export const ACTOR_TIMEOUT_MS = ACTOR_TIMEOUT_SECS * 1000;
export const SCRAPPA_REQUEST_TIMEOUT_MS = 15_000;
export const SCRAPPA_MAX_ATTEMPTS = 2;
export const PROFILE_REQUEST_CONCURRENCY = 8;
export const RUN_SAFETY_MARGIN_MS = 60_000;

function getWorstCaseRetryDelayMs(attempts: number): number {
    let totalDelayMs = 0;

    for (let failedAttempt = 0; failedAttempt < Math.max(0, attempts - 1); failedAttempt += 1) {
        totalDelayMs += Math.min(1000 * 2 ** failedAttempt + 1000, 10_000);
    }

    return totalDelayMs;
}

export function getWorstCaseScrappaRequestDurationMs(attempts = SCRAPPA_MAX_ATTEMPTS): number {
    const boundedAttempts = Math.max(1, attempts);
    return boundedAttempts * SCRAPPA_REQUEST_TIMEOUT_MS + getWorstCaseRetryDelayMs(boundedAttempts);
}

export function getWorstCaseRunDurationMs(requestCount: number): number {
    const waves = Math.ceil(Math.max(0, requestCount) / PROFILE_REQUEST_CONCURRENCY);
    return waves * getWorstCaseScrappaRequestDurationMs() + RUN_SAFETY_MARGIN_MS;
}
