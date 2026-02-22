import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';
import { fetchWithFallback } from './fetch-with-fallback.js';

interface GoogleMapsSearchInput {
    query: string;
    hl?: string;
    gl?: string;
    use_cache?: boolean;
    maximum_cache_age?: number;
    fallback_zoom?: number;
}

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

    const client = new ScrappaClient({ apiKey });
    const params: Record<string, unknown> = {
        query: input.query,
        hl: input.hl || 'en',
        gl: input.gl,
    };

    if (input.use_cache !== false) {
        params.use_cache = 1;
    }
    if (input.maximum_cache_age !== undefined) {
        params.maximum_cache_age = input.maximum_cache_age;
    }

    const response = await fetchWithFallback(client, params, input.fallback_zoom ?? 13);

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
