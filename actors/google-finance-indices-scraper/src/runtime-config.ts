/**
 * Keep retries inside the Actor's 120-second run limit. The reserved time
 * covers dataset writes, the compact OUTPUT summary, and graceful shutdown.
 */
export const ACTOR_TIMEOUT_SECONDS = 120;
export const REQUEST_TIMEOUT_MS = 30_000;
export const REQUEST_ATTEMPTS = 3;
export const RETRY_BACKOFF_MS = 250;
export const RUN_FINALIZATION_RESERVE_MS = 20_000;

export const RETRY_TIME_BUDGET_MS = (
    REQUEST_ATTEMPTS * REQUEST_TIMEOUT_MS
) + (
    RETRY_BACKOFF_MS * ((REQUEST_ATTEMPTS * (REQUEST_ATTEMPTS - 1)) / 2)
);

export const AVAILABLE_REQUEST_TIME_MS = (
    ACTOR_TIMEOUT_SECONDS * 1_000
) - RUN_FINALIZATION_RESERVE_MS;

export function retryBudgetFitsActorTimeout(): boolean {
    return RETRY_TIME_BUDGET_MS <= AVAILABLE_REQUEST_TIME_MS;
}
