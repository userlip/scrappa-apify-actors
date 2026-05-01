import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';
import { fetchWithFallback } from './fetch-with-fallback.js';
import { addSearchResponseAliases } from './output-aliases.js';
import type { GoogleMapsSearchResponse } from './output-aliases.js';
import { buildSearchParams } from './search-params.js';
import type { GoogleMapsSearchInput } from './search-params.js';

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set.');
    }

    const input = await Actor.getInput<GoogleMapsSearchInput>();
    if (!input?.query) {
        throw new Error('Search query is required');
    }

    console.log(`Searching Google Maps for: "${input.query}"`);

    const client = new ScrappaClient({ apiKey, debug: input.debug });
    const params = buildSearchParams(input);

    const response: GoogleMapsSearchResponse = addSearchResponseAliases(
        await fetchWithFallback(client, params, input.fallback_zoom ?? 13)
    );

    if (response.items && response.items.length > 0) {
        await Actor.pushData(response.items);
        console.log(`Found ${response.items.length} results`);
    } else {
        console.log('No results found for the given search criteria');
    }

    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

    console.log('Search completed successfully');

} catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();
