import { Actor } from 'apify';
import { ScrappaClient, createActorInput, validateRequiredFields } from './shared/index.js';

interface GoogleSearchInput {
    apiKey: string;
    baseUrl?: string;
    query: string;
    location?: string;
    gl?: string;
    hl?: string;
    google_domain?: string;
    start?: number;
    amount?: number;
    safe?: string;
    tbs?: string;
    tbm?: string;
    lr?: string;
    cr?: string;
    uule?: string;
    nfpr?: number;
    filter?: number;
}

interface OrganicResult {
    position: number;
    title: string;
    link: string;
    snippet?: string;
    displayed_link?: string;
}

interface GoogleSearchResponse {
    organic_results?: OrganicResult[];
    search_information?: {
        total_results?: number;
        time_taken_displayed?: number;
    };
    ads?: unknown[];
    knowledge_graph?: unknown;
    related_searches?: unknown[];
    [key: string]: unknown;
}

await Actor.init();

try {
    const input = await createActorInput<GoogleSearchInput>();

    validateRequiredFields(input, ['apiKey', 'query']);

    console.log(`Searching Google for: "${input.query}"`);

    const client = new ScrappaClient({
        apiKey: input.apiKey,
        baseUrl: input.baseUrl,
    });

    const params: Record<string, unknown> = {
        query: input.query,
        location: input.location,
        gl: input.gl,
        hl: input.hl,
        google_domain: input.google_domain,
        start: input.start,
        amount: input.amount,
        safe: input.safe,
        tbs: input.tbs,
        tbm: input.tbm,
        lr: input.lr,
        cr: input.cr,
        uule: input.uule,
        nfpr: input.nfpr,
        filter: input.filter,
    };

    const response = await client.get<GoogleSearchResponse>('/search', params);

    // Push organic results to dataset
    if (response.organic_results && response.organic_results.length > 0) {
        await Actor.pushData(response.organic_results);
        console.log(`Pushed ${response.organic_results.length} organic results to dataset`);
    }

    // Store full response in key-value store
    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

    console.log('Google Search completed successfully');
    console.log(`Total results: ${response.search_information?.total_results ?? 'unknown'}`);

} catch (error) {
    console.error(`Actor failed: ${error instanceof Error ? error.message : String(error)}`);
    throw error;
}

await Actor.exit();
