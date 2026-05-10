import { Actor } from 'apify';
import { buildSuggestionsRequest, suggestionsToDatasetItems } from './suggestions-url.js';

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
        const request = buildSuggestionsRequest(input);

        console.log(`Fetching from: ${request.url}`);
        const response = await fetch(request.url, {
            headers: {
                'Accept': 'application/json',
            },
            signal: AbortSignal.timeout(SCRAPPA_REQUEST_TIMEOUT_MS),
        });
        if (!response.ok) {
            throw new Error(`Scrappa API request failed with ${response.status} ${response.statusText}`);
        }

        const data = await response.json();
        const suggestions = suggestionsToDatasetItems(data, request);

        await Actor.pushData(suggestions);
        console.log(`Successfully fetched ${suggestions.length} suggestion(s) for query: ${request.query}`);
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube search suggestions: ${message}`);
        await Actor.fail(message);
    }
});
