import { Actor } from 'apify';
import { buildSearchUrl } from './search-url.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

function errorMessage(error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    if (rawMessage.includes('aborted')) {
        return `Scrappa API request timed out after ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s`;
    }

    return rawMessage;
}

Actor.main(async () => {
    try {
        const input = (await Actor.getInput()) ?? {};
        const apiUrl = buildSearchUrl(input);

        console.log(`Fetching from: ${apiUrl}`);
        const response = await fetch(apiUrl, {
            signal: AbortSignal.timeout(SCRAPPA_REQUEST_TIMEOUT_MS),
        });
        if (!response.ok) {
            throw new Error(`Scrappa API request failed with ${response.status} ${response.statusText}`);
        }

        const data = await response.json();
        const results = data?.results ?? [];

        await Actor.pushData(results);
        console.log(`Successfully fetched ${Array.isArray(results) ? results.length : 1} results for query: ${input.q}`);

        const continuation = data?.continuation ?? data?.pagination?.continuationToken;
        if (continuation) {
            console.log(`Continuation token available for next page: ${continuation}`);
        }
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube search data: ${message}`);
        await Actor.fail(message);
    }
});
