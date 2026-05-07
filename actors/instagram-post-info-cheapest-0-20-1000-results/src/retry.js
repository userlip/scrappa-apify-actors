const TRANSIENT_STATUS_CODES = new Set([408, 425, 429, 500, 502, 503, 504]);
const TRANSIENT_MESSAGE_PATTERNS = [
    /rate\s*limited/i,
    /too many requests/i,
    /\b429\b/i,
    /temporarily unavailable/i,
    /timeout/i,
];

export const DEFAULT_RETRY_DELAYS_MS = [20000, 60000];

export function sleep(ms) {
    return new Promise((resolve) => {
        setTimeout(resolve, ms);
    });
}

export function getResponseMessage(data) {
    return data?.message
        ?? data?.error
        ?? 'Unknown Scrappa API error';
}

export function isTransientScrappaError(error) {
    const status = error?.response?.status;
    if (typeof status === 'number' && TRANSIENT_STATUS_CODES.has(status)) {
        return true;
    }

    const responseMessage = getResponseMessage(error?.response?.data);
    const errorMessage = error instanceof Error ? error.message : String(error ?? '');
    const combinedMessage = `${responseMessage} ${errorMessage}`;

    return TRANSIENT_MESSAGE_PATTERNS.some((pattern) => pattern.test(combinedMessage));
}

export async function requestWithRetries(request, {
    delaysMs = DEFAULT_RETRY_DELAYS_MS,
    shouldRetry = isTransientScrappaError,
    onRetry = () => {},
    wait = sleep,
} = {}) {
    for (let attempt = 0; attempt <= delaysMs.length; attempt++) {
        try {
            return await request();
        } catch (error) {
            const isLastAttempt = attempt === delaysMs.length;
            if (isLastAttempt || !shouldRetry(error)) {
                throw error;
            }

            const delayMs = delaysMs[attempt];
            onRetry(error, attempt + 1, delayMs);
            await wait(delayMs);
        }
    }
}
