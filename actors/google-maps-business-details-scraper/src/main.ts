import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';
import { getBusinessIdRequests, type GoogleMapsBusinessDetailsInput } from './input.js';

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
    const requests = getBusinessIdRequests(input);
    if (requests.length === 0) {
        throw new Error('At least one Business ID is required. Provide business_ids or legacy business_id.');
    }

    const client = new ScrappaClient({ apiKey });
    const results: Array<{
        input_business_id: string;
        business_id: string;
        found: boolean;
        error?: string;
    }> = [];
    let firstOutput: GoogleMapsBusinessDetailsResponse | { data: []; error: string } | undefined;
    let succeeded = 0;
    let failed = 0;

    console.log(`Fetching Google Maps business details for ${requests.length} business${requests.length === 1 ? '' : 'es'}`);

    for (const request of requests) {
        console.log(`Fetching details for business ID: ${request.business_id}`);

        const params: Record<string, unknown> = {
            business_id: request.business_id,
        };

        if (input?.use_cache !== false) {
            params.use_cache = 1;
        }
        if (input?.maximum_cache_age !== undefined) {
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
                console.log(`Business not found (404): ${request.business_id}`);
                await Actor.pushData([{
                    success: false,
                    input_business_id: request.input_business_id,
                    business_id: request.business_id,
                    error: 'Business not found',
                }]);
                results.push({
                    input_business_id: request.input_business_id,
                    business_id: request.business_id,
                    found: false,
                    error: 'Business not found',
                });
                firstOutput ??= { data: [], error: 'Business not found' };
                failed += 1;
                handled404 = true;
            } else {
                // Re-throw non-404 errors
                throw apiError;
            }
        }

        if (!handled404 && response) {
            // The API returns data as an array
            if (response.data && response.data.length > 0) {
                const datasetItems = response.data.map((item) => ({
                    ...item,
                    input_business_id: request.input_business_id,
                    business_id: item.business_id ?? request.business_id,
                }));
                await Actor.pushData(datasetItems);
                console.log(`Successfully fetched: ${datasetItems[0]?.name ?? 'Business'}`);
                succeeded += 1;
                results.push({
                    input_business_id: request.input_business_id,
                    business_id: request.business_id,
                    found: true,
                });
            } else {
                console.log(`No business details found for ID: ${request.business_id}`);
                await Actor.pushData([{
                    success: false,
                    input_business_id: request.input_business_id,
                    business_id: request.business_id,
                    error: 'No business details found',
                }]);
                results.push({
                    input_business_id: request.input_business_id,
                    business_id: request.business_id,
                    found: false,
                });
                failed += 1;
            }

            firstOutput ??= response;
        }
    }

    const store = await Actor.openKeyValueStore();
    if (requests.length === 1 && firstOutput) {
        await store.setValue('OUTPUT', firstOutput);
    } else {
        await store.setValue('OUTPUT', {
            requested: requests.length,
            succeeded,
            failed,
            results,
        });
    }

    console.log('Completed successfully');

} catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();
