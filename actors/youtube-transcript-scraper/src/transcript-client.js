import { buildTranscriptUrl } from './transcript-url.js';

export const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
export const SCRAPPA_MAX_REQUEST_ATTEMPTS = 4;
export const SCRAPPA_RETRY_BASE_DELAY_MS = 1000;
export const SCRAPPA_RETRY_MAX_DELAY_MS = 10000;
export const SCRAPPA_RETRYABLE_STATUS_CODES = new Set([408, 429, 500, 502, 503, 504, 522]);

export function buildTranscriptRequest(input, apiKey, timeoutMs = SCRAPPA_REQUEST_TIMEOUT_MS) {
    const apiUrl = buildTranscriptUrl(input);
    const requestOptions = {
        headers: {
            'X-API-Key': apiKey,
            'Accept': 'application/json',
        },
        signal: AbortSignal.timeout(timeoutMs),
    };

    return { apiUrl, requestOptions };
}

function statusText(response) {
    return response.statusText || '<none>';
}

function requestError(response) {
    return new Error(`Scrappa API request failed with ${response.status} ${statusText(response)}`);
}

function retryDelayMs(attempt, baseDelayMs, maxDelayMs) {
    return Math.min(baseDelayMs * (2 ** (attempt - 1)), maxDelayMs);
}

function sleep(ms) {
    return new Promise((resolve) => {
        setTimeout(resolve, ms);
    });
}

export async function fetchTranscript(input, {
    apiKey,
    fetchFn = fetch,
    onRequest,
    timeoutMs = SCRAPPA_REQUEST_TIMEOUT_MS,
    maxAttempts = SCRAPPA_MAX_REQUEST_ATTEMPTS,
    retryBaseDelayMs = SCRAPPA_RETRY_BASE_DELAY_MS,
    retryMaxDelayMs = SCRAPPA_RETRY_MAX_DELAY_MS,
    sleepFn = sleep,
} = {}) {
    let lastError;

    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
        const { apiUrl, requestOptions } = buildTranscriptRequest(input, apiKey, timeoutMs);
        onRequest?.(apiUrl, attempt);

        let response;
        try {
            response = await fetchFn(apiUrl, requestOptions);
        } catch (error) {
            lastError = error;

            if (attempt >= maxAttempts) {
                throw lastError;
            }

            await sleepFn(retryDelayMs(attempt, retryBaseDelayMs, retryMaxDelayMs));
            continue;
        }

        if (response.ok) {
            return {
                apiUrl,
                data: await response.json(),
            };
        }

        lastError = requestError(response);

        if (!SCRAPPA_RETRYABLE_STATUS_CODES.has(response.status) || attempt >= maxAttempts) {
            throw lastError;
        }

        await sleepFn(retryDelayMs(attempt, retryBaseDelayMs, retryMaxDelayMs));
    }

    throw lastError;
}
