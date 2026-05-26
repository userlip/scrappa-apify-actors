export const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

export function getScrappaApiKey(env = process.env) {
    const apiKey = env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    return apiKey;
}

export function buildScrappaRequest(apiUrl, apiKey, timeoutMs = SCRAPPA_REQUEST_TIMEOUT_MS) {
    return {
        apiUrl,
        requestOptions: {
            headers: {
                'X-API-Key': apiKey,
                'Accept': 'application/json',
            },
            signal: AbortSignal.timeout(timeoutMs),
        },
    };
}

export async function fetchScrappaJson(apiUrl, {
    apiKey,
    fetchFn = fetch,
    timeoutMs = SCRAPPA_REQUEST_TIMEOUT_MS,
} = {}) {
    const { requestOptions } = buildScrappaRequest(apiUrl, apiKey, timeoutMs);
    const response = await fetchFn(apiUrl, requestOptions);

    if (!response.ok) {
        throw new Error(`Scrappa API request failed with ${response.status} ${response.statusText}`);
    }

    return response.json();
}
