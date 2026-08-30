export const ACTOR_TIMEOUT_MS = 240000;
export const ACTOR_COMPLETION_RESERVE_MS = 30000;

export const RELATED_REQUEST_TIMEOUT_MS = 30000;
export const RELATED_MAX_ATTEMPTS = 4;
export const RELATED_MAX_RETRY_DELAY_MS = 20000;

export const AUTOCOMPLETE_REQUEST_TIMEOUT_MS = 15000;
export const AUTOCOMPLETE_MAX_ATTEMPTS = 1;

export function getMaximumRequestDurationMs(
    timeoutMs: number,
    attempts: number,
    maxRetryDelayMs: number,
): number {
    return timeoutMs * attempts + maxRetryDelayMs * Math.max(0, attempts - 1);
}
