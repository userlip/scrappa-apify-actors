import { Actor } from 'apify';
import axios from 'axios';
import { buildSearchUrl } from './search-url.js';

Actor.main(async () => {
    const input = await Actor.getInput();
    const apiUrl = buildSearchUrl(input);

    try {
        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl);
        const results = response.data?.results ?? [];

        await Actor.pushData(results);
        console.log(`Successfully fetched ${Array.isArray(results) ? results.length : 1} results for query: ${input.q}`);

        const continuation = response.data?.continuation ?? response.data?.pagination?.continuationToken;
        if (continuation) {
            console.log(`Continuation token available for next page: ${continuation}`);
        }
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error(`Failed to fetch YouTube search data: ${message}`);
        await Actor.fail(message);
    }
});
