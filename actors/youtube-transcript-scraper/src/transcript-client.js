import { buildTranscriptUrl } from './transcript-url.js';

export const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

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

export async function fetchTranscript(input, {
    apiKey,
    fetchFn = fetch,
    onRequest,
    timeoutMs = SCRAPPA_REQUEST_TIMEOUT_MS,
} = {}) {
    const { apiUrl, requestOptions } = buildTranscriptRequest(input, apiKey, timeoutMs);
    onRequest?.(apiUrl);

    const response = await fetchFn(apiUrl, requestOptions);

    if (!response.ok) {
        throw new Error(`Scrappa API request failed with ${response.status} ${response.statusText}`);
    }

    return {
        apiUrl,
        data: await response.json(),
    };
}
