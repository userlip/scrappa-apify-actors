import { buildVideoCommentsUrl } from './comments-url.js';

export const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

export function buildVideoCommentsRequest(input, apiKey, timeoutMs = SCRAPPA_REQUEST_TIMEOUT_MS) {
    const apiUrl = buildVideoCommentsUrl(input);
    const requestOptions = {
        headers: {
            'X-API-Key': apiKey,
            'Accept': 'application/json',
        },
        signal: AbortSignal.timeout(timeoutMs),
    };

    return { apiUrl, requestOptions };
}

export async function fetchVideoComments(input, {
    apiKey,
    fetchFn = fetch,
    onRequest,
    timeoutMs = SCRAPPA_REQUEST_TIMEOUT_MS,
} = {}) {
    const { apiUrl, requestOptions } = buildVideoCommentsRequest(input, apiKey, timeoutMs);
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
