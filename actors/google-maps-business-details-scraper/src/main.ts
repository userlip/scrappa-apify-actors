import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';

interface GoogleMapsBusinessDetailsInput {
    business_id: string;
    use_cache?: boolean;
    maximum_cache_age?: number;
}

interface BusinessDetails {
    business_id?: string;
    name?: string;
    rating?: number;
    review_count?: number;
    phone_number?: string;
    website?: string;
    full_address?: string;
    email?: string;
    price_level?: string;
    opening_hours?: Array<{
        day?: string;
        hours?: string[];
        date?: string;
    }>;
    address_components?: Array<{
        long_name?: string;
        short_name?: string;
        types?: string[];
    }>;
    latitude?: number;
    longitude?: number;
    place_id?: string;
    type?: string;
    types?: string[];
    subtypes?: string[];
    formatted_address?: string;
    district?: string;
    international_phone_number?: string;
    url?: string;
    utc_offset?: number;
    vicinity?: string;
    formatted_phone_number?: string;
    icon?: string;
    icon_mask_base_uri?: string;
    icon_background_color?: string;
    photos?: Array<{
        height?: number;
        html_attributions?: string[];
        photo_reference?: string;
        width?: number;
    }>;
    [key: string]: unknown;
}

interface GoogleMapsBusinessDetailsResponse {
    status?: string;
    business_id?: string;
    data?: BusinessDetails[];
    [key: string]: unknown;
}

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set.');
    }

    const input = await Actor.getInput<GoogleMapsBusinessDetailsInput>();
    if (!input?.business_id) {
        throw new Error('Business ID is required');
    }

    console.log(`Fetching details for business ID: ${input.business_id}`);

    const client = new ScrappaClient({ apiKey });
    const params: Record<string, unknown> = {
        business_id: input.business_id,
    };

    if (input.use_cache !== false) {
        params.use_cache = 1;
    }
    if (input.maximum_cache_age !== undefined) {
        params.maximum_cache_age = input.maximum_cache_age;
    }

    let response: GoogleMapsBusinessDetailsResponse | null = null;
    let handled404 = false;

    try {
        response = await client.get<GoogleMapsBusinessDetailsResponse>('/maps/business-details', params);
    } catch (apiError) {
        const statusCode = (apiError as any)?.statusCode;

        // Handle 404 gracefully - push empty result instead of failing
        if (statusCode === 404) {
            console.log(`Business not found (404): ${input.business_id}`);
            await Actor.pushData([{
                success: false,
                business_id: input.business_id,
                error: 'Business not found',
            }]);
            const store = await Actor.openKeyValueStore();
            await store.setValue('OUTPUT', { data: [], error: 'Business not found' });
            handled404 = true;
        } else {
            // Re-throw non-404 errors
            throw apiError;
        }
    }

    if (!handled404 && response) {
        // The API returns data as an array
        if (response.data && response.data.length > 0) {
            await Actor.pushData(response.data);
            console.log(`Successfully fetched: ${response.data[0]?.name ?? 'Business'}`);
        } else {
            console.log('No business details found for the given ID');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);
    }

    console.log('Completed successfully');

} catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();
