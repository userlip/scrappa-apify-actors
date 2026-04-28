import { Actor } from 'apify';
import { buildSearchUrl } from './search-url.js';

Actor.main(async () => {
    try {
        const input = (await Actor.getInput()) ?? {};
        const apiUrl = buildSearchUrl(input);

        console.log(`Fetching from: ${apiUrl}`);
        const response = await fetch(apiUrl);
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
        const message = error instanceof Error ? error.message : String(error);
        console.error(`Failed to fetch YouTube search data: ${message}`);
        await Actor.fail(message);
    }
});
