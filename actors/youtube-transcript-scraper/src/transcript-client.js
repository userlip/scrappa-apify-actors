import { buildTranscriptUrl } from './transcript-url.js';

export const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
export const SCRAPPA_MAX_REQUEST_ATTEMPTS = 4;
export const SCRAPPA_RETRY_BASE_DELAY_MS = 1000;
export const SCRAPPA_RETRY_MAX_DELAY_MS = 10000;
export const SCRAPPA_RETRY_JITTER_RATIO = 0.2;
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

function networkRequestError(error) {
    const message = error instanceof Error ? error.message : String(error);
    return new Error(`Scrappa API request failed before receiving a response: ${message}`);
}

function retryDelayMs(attempt, baseDelayMs, maxDelayMs, jitterRatio, randomFn) {
    const delayMs = Math.min(baseDelayMs * (2 ** (attempt - 1)), maxDelayMs);
    const jitterMs = delayMs * jitterRatio * randomFn();

    return Math.min(delayMs + jitterMs, maxDelayMs);
}

function retryAfterDelayMs(response, nowMs) {
    const retryAfter = response.headers?.get?.('retry-after');
    if (!retryAfter) {
        return null;
    }

    const seconds = Number(retryAfter);
    if (Number.isFinite(seconds) && seconds >= 0) {
        return seconds * 1000;
    }

    const retryAtMs = Date.parse(retryAfter);
    if (Number.isFinite(retryAtMs)) {
        return Math.max(0, retryAtMs - nowMs);
    }

    return null;
}

function responseRetryDelayMs(response, {
    attempt,
    retryBaseDelayMs,
    retryMaxDelayMs,
    retryJitterRatio,
    randomFn,
    nowFn,
}) {
    const backoffMs = retryDelayMs(attempt, retryBaseDelayMs, retryMaxDelayMs, retryJitterRatio, randomFn);
    const headerDelayMs = retryAfterDelayMs(response, nowFn());

    return headerDelayMs === null ? backoffMs : Math.max(backoffMs, headerDelayMs);
}

function sleep(ms) {
    return new Promise((resolve) => {
        setTimeout(resolve, ms);
    });
}

async function parseJsonOrThrow(response, apiUrl) {
    try {
        return await response.json();
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new Error(`Scrappa API returned an invalid JSON response for ${apiUrl}: ${message}`);
    }
}

export async function fetchTranscript(input, {
    apiKey,
    fetchFn = fetch,
    onRequest,
    timeoutMs = SCRAPPA_REQUEST_TIMEOUT_MS,
    maxAttempts = SCRAPPA_MAX_REQUEST_ATTEMPTS,
    retryBaseDelayMs = SCRAPPA_RETRY_BASE_DELAY_MS,
    retryMaxDelayMs = SCRAPPA_RETRY_MAX_DELAY_MS,
    retryJitterRatio = SCRAPPA_RETRY_JITTER_RATIO,
    randomFn = Math.random,
    nowFn = Date.now,
    sleepFn = sleep,
} = {}) {
    let lastError;
    const attempts = Number.isFinite(maxAttempts)
        ? Math.max(1, Math.floor(maxAttempts))
        : SCRAPPA_MAX_REQUEST_ATTEMPTS;

    for (let attempt = 1; attempt <= attempts; attempt++) {
        const { apiUrl, requestOptions } = buildTranscriptRequest(input, apiKey, timeoutMs);
        onRequest?.(apiUrl, attempt);

        let response;
        try {
            response = await fetchFn(apiUrl, requestOptions);
        } catch (error) {
            lastError = networkRequestError(error);

            if (attempt >= attempts) {
                throw lastError;
            }

            await sleepFn(retryDelayMs(attempt, retryBaseDelayMs, retryMaxDelayMs, retryJitterRatio, randomFn));
            continue;
        }

        if (response.ok) {
            return {
                apiUrl,
                data: await parseJsonOrThrow(response, apiUrl),
            };
        }

        lastError = requestError(response);

        if (!SCRAPPA_RETRYABLE_STATUS_CODES.has(response.status) || attempt >= attempts) {
            throw lastError;
        }

        await sleepFn(responseRetryDelayMs(response, {
            attempt,
            retryBaseDelayMs,
            retryMaxDelayMs,
            retryJitterRatio,
            randomFn,
            nowFn,
        }));
    }

    throw lastError ?? new Error('Scrappa API request failed without a response');
}
