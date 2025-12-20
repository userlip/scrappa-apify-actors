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
    redirect_link?: string;
    displayed_link?: string;
    snippet?: string;
    source?: string;
}

interface RelatedSearch {
    query: string;
    link: string;
    type?: string;
}

interface GoogleSearchResponse {
    search_information?: {
        query_displayed?: string;
        total_results?: number;
    };
    organic_results?: OrganicResult[];
    related_searches?: RelatedSearch[];
    people_also_search_for?: unknown[];
    related_questions?: unknown[];
    things_to_know?: unknown[];
    knowledge_graph?: unknown;
    see_results_about?: unknown;
    twitter_card?: unknown;
    inline_videos?: unknown[];
    inline_images?: unknown[];
    local_map?: unknown;
    local_results?: unknown;
    popular_products?: unknown;
    perspectives?: unknown[];
    total_results?: number;
    engine_used?: string;
    service_used?: string;
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

    // Push organic results to dataset (main output for table view)
    if (response.organic_results && response.organic_results.length > 0) {
        await Actor.pushData(response.organic_results);
        console.log(`Found ${response.organic_results.length} organic results`);
    }

    // Store full response in key-value store for complete data access
    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

    // Log summary
    console.log('Google Search completed successfully');

    const summary = {
        organic_results: response.organic_results?.length ?? 0,
        related_searches: response.related_searches?.length ?? 0,
        related_questions: response.related_questions?.length ?? 0,
        inline_videos: response.inline_videos?.length ?? 0,
        inline_images: response.inline_images?.length ?? 0,
        has_knowledge_graph: !!response.knowledge_graph,
        has_local_results: !!response.local_results,
    };

    console.log('Results summary:', JSON.stringify(summary));

} catch (error) {
    console.error(`Actor failed: ${error instanceof Error ? error.message : String(error)}`);
    throw error;
}

await Actor.exit();
