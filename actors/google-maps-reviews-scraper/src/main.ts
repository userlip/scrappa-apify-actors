import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';

interface GoogleMapsReviewsInput {
    business_id: string;
    sort: number;
    limit?: number;
    page?: string;
    search?: string;
    debug?: boolean;
    use_cache?: boolean;
    maximum_cache_age?: number;
}

interface ReviewItem {
    rating?: number;
    author_name?: string;
    review_text?: string[];
    review_id?: string;
    timestamp?: number;
    author_review_count?: number;
    author_profile_photo?: string;
    author_local_guide_level?: number;
    images?: string[];
    review_likes?: number;
    review_language?: string[];
    owner_response_text?: string;
    owner_response_timestamp?: number;
    author_link?: string;
    review_link?: string;
    review_form?: Record<string, unknown>;
    [key: string]: unknown;
}

interface GoogleMapsReviewsResponse {
    items?: ReviewItem[];
    nextPage?: string;
    [key: string]: unknown;
}

await Actor.init();

try {
    // Get API key from environment variable (set as Apify secret)
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
    }

    const input = await Actor.getInput<GoogleMapsReviewsInput>();
    if (!input?.business_id) {
        throw new Error('Business ID is required');
    }
    if (input?.sort === undefined) {
        throw new Error('Sort parameter is required (1-4: 1=Most Relevant, 2=Newest, 3=Highest Rating, 4=Lowest Rating)');
    }

    const sortNames: Record<number, string> = {
        1: 'Most Relevant',
        2: 'Newest',
        3: 'Highest Rating',
        4: 'Lowest Rating',
    };

    console.log(`Fetching reviews for business ID: "${input.business_id}" sorted by: ${sortNames[input.sort]}`);

    const client = new ScrappaClient({ apiKey });

    const params: Record<string, unknown> = {
        business_id: input.business_id,
        sort: input.sort,
        limit: input.limit || 10,
        page: input.page,
        search: input.search,
        debug: input.debug,
    };

    if (input.use_cache !== false) {
        params.use_cache = 1;

        if (input.maximum_cache_age !== undefined && input.maximum_cache_age > 0) {
            params.maximum_cache_age = input.maximum_cache_age;
        }
    }

    const response = await client.get<GoogleMapsReviewsResponse>('/maps/reviews', params);

    // Push review items to dataset (main output for table view)
    if (response.items && response.items.length > 0) {
        await Actor.pushData(response.items);
        console.log(`Found ${response.items.length} reviews`);
    } else {
        console.log('No reviews found for the given search criteria');
    }

    // Store full response in key-value store for complete data access
    const store = await Actor.openKeyValueStore();
    await store.setValue('OUTPUT', response);

    // Log summary
    console.log('Google Maps Reviews extraction completed successfully');

    const summary = {
        reviews_extracted: response.items?.length ?? 0,
        business_id: input.business_id,
        sort_order: sortNames[input.sort],
        has_next_page: !!response.nextPage,
    };

    console.log('Results summary:', JSON.stringify(summary));

} catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();
