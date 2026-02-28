import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';

interface GoogleMapsAdvancedSearchInput {
    query: string;
    zoom: number;
    latitude?: number;
    longitude?: number;
    limit?: number;
    hl?: string;
    gl?: string;
    debug?: boolean;
}

interface BusinessResult {
    name?: string;
    business_id?: string;
    place_id?: string;
    rating?: number;
    review_count?: number;
    price_level?: string;
    website?: string;
    domain?: string;
    phone_numbers?: string[];
    full_address?: string;
    district?: string;
    latitude?: number;
    longitude?: number;
    subtypes?: string[];
    type?: string;
    short_description?: string;
    opening_hours?: Array<{
        day?: string;
        hours?: string[];
        date?: string;
    }>;
    current_status?: string;
    photos_sample?: Array<{
        photo_id?: string;
        photo_url?: string;
    }>;
    [key: string]: unknown;
}

interface GoogleMapsAdvancedSearchResponse {
    items?: BusinessResult[];
    [key: string]: unknown;
}

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set.');
    }

    const input = await Actor.getInput<GoogleMapsAdvancedSearchInput>();
    if (!input?.query || input.zoom === undefined) {
        throw new Error('Search query and zoom level are required');
    }

    // Charge for the search event (covers base compute cost)
    const chargeResult = await Actor.charge({ eventName: 'search', count: 1 });
    if (chargeResult.eventChargeLimitReached) {
        console.log('User budget limit reached, stopping.');
        await Actor.exit();
        process.exit(0);
    }

    const locationInfo = input.latitude && input.longitude
        ? `at lat ${input.latitude}, lon ${input.longitude}`
        : '(auto-resolved location)';
    console.log(`Advanced search: "${input.query}" at zoom ${input.zoom} ${locationInfo}`);

    const client = new ScrappaClient({ apiKey });
    const response = await client.get<GoogleMapsAdvancedSearchResponse>('/maps/advance-search', {
        query: input.query,
        zoom: input.zoom,
        lat: input.latitude,
        lon: input.longitude,
        limit: input.limit,
        hl: input.hl || 'en',
        gl: input.gl,
    });

    // Push results to dataset with per-result charging
    if (response.items && response.items.length > 0) {
        await Actor.pushData(response.items, 'result');
        console.log(`Found ${response.items.length} results`);
    } else {
        console.log('No results found for the given search criteria');
    }

    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

    const summary = {
        query: input.query,
        results_found: response.items?.length ?? 0,
        zoom_level: input.zoom,
        language: input.hl || 'en',
        region: input.gl || 'worldwide',
    };

    console.log('Advanced search completed:', JSON.stringify(summary));

} catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();
