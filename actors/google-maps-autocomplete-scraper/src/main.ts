import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';

interface GoogleMapsAutocompleteInput {
    query: string;
}

interface Suggestion {
    main_text?: string;
    main_text_highlights?: Array<{
        offset?: number;
        length?: number;
    }>;
    secondary_text?: string;
    secondary_text_highlights?: Array<{
        offset?: number;
        length?: number;
    }>;
    type?: string;
    country?: string;
    latitude?: number;
    longitude?: number;
    google_id?: string;
    place_id?: string;
    [key: string]: unknown;
}

interface GoogleMapsAutocompleteResponse {
    query?: string;
    suggestions?: Suggestion[];
    [key: string]: unknown;
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set.');
        }

        const input = await Actor.getInput<GoogleMapsAutocompleteInput>();
        if (!input?.query) {
            throw new Error('Search query is required');
        }

        console.log(`Getting autocomplete suggestions for: "${input.query}"`);

        const client = new ScrappaClient({ apiKey });
        const response = await client.get<GoogleMapsAutocompleteResponse>('/maps/autocomplete', {
            query: input.query,
        });

        if (response.suggestions && response.suggestions.length > 0) {
            await Actor.pushData(response.suggestions);
            console.log(`Found ${response.suggestions.length} suggestions`);
        } else {
            console.log('No autocomplete suggestions found for the given query');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const summary = {
            query: input.query,
            suggestions_found: response.suggestions?.length ?? 0,
        };

        console.log('Autocomplete completed:', JSON.stringify(summary));

    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error(`Failed: ${message}`);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main();
