import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';
import { normalizeBusinessId } from './business-id.js';

interface GoogleMapsPhotosInput {
    business_id: string;
    use_cache?: boolean;
    maximum_cache_age?: number;
}

interface Photo {
    photo_id?: string;
    photo_url?: string;
    photo_url_large?: string;
    width?: number;
    height?: number;
    contributor_name?: string;
    contributor_url?: string;
    posted_at?: string;
    latitude?: number;
    longitude?: number;
    video_thumbnail_url?: string;
    photo_index?: number;
    source?: string;
    author?: string;
    published_at?: string;
    is_owner?: boolean;
    likes?: number;
    [key: string]: unknown;
}

type GoogleMapsPhotosResponse = Photo[] | {
    items?: Photo[];
    data?: Photo[];
    nextPage?: string | null;
    [key: string]: unknown;
};

await Actor.init();

try {
    const apiKey = process.env.SCRAPPA_API_KEY;
    if (!apiKey) {
        throw new Error('SCRAPPA_API_KEY environment variable is not set.');
    }

    const input = await Actor.getInput<GoogleMapsPhotosInput>();
    if (!input?.business_id) {
        throw new Error('Business ID is required');
    }

    let businessId: string | null = null;
    let handledInvalidInput = false;
    try {
        const normalized = normalizeBusinessId(input.business_id);
        businessId = normalized.businessId;

        if (normalized.source === 'url') {
            console.log(`Extracted Google Maps business identifier from URL: ${businessId}`);
        }
    } catch (inputError) {
        const message = inputError instanceof Error ? inputError.message : String(inputError);
        console.log(`Invalid business input: ${message}`);
        await Actor.pushData([{
            success: false,
            business_id: input.business_id,
            error: message,
        }]);
        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', { photos: [], total: 0, nextPage: null, error: message });
        handledInvalidInput = true;
    }

    if (!handledInvalidInput && businessId) {
        console.log(`Fetching photos for business: ${businessId}`);

        const client = new ScrappaClient({ apiKey });
        const params: Record<string, unknown> = {
            business_id: businessId,
        };

        if (input.use_cache !== false) {
            params.use_cache = 1;
        }
        if (input.maximum_cache_age !== undefined) {
            params.maximum_cache_age = input.maximum_cache_age;
        }

        let photos: Photo[] = [];
        let nextPage: string | null = null;
        let handledApiInputError = false;

        try {
            const response = await client.get<GoogleMapsPhotosResponse>('/maps/photos', params);

            // Handle both response types: direct array or wrapped object.
            // The Scrappa /maps/photos API currently returns { items, nextPage }.
            if (Array.isArray(response)) {
                photos = response;
            } else {
                photos = response.items ?? response.data ?? [];
                nextPage = response.nextPage ?? null;
            }
        } catch (apiError) {
            const statusCode = (apiError as any)?.statusCode;

            if (statusCode === 404 || statusCode === 422) {
                const error = statusCode === 404 ? 'Business not found' : 'Invalid input';
                console.log(`Photos request returned ${statusCode}: ${businessId}`);
                await Actor.pushData([{
                    success: false,
                    business_id: input.business_id,
                    error,
                }]);
                const store = await Actor.openKeyValueStore();
                await store.setValue('OUTPUT', { photos: [], total: 0, nextPage: null, error });
                handledApiInputError = true;
            } else {
                // Re-throw non-input errors
                throw apiError;
            }
        }

        if (!handledApiInputError) {
            if (photos && photos.length > 0) {
                await Actor.pushData(photos);
                console.log(`Found ${photos.length} photos`);
            } else {
                console.log('No photos found for the given business ID');
            }

            const store = await Actor.openKeyValueStore();
            await store.setValue('OUTPUT', { photos, total: photos.length, nextPage });
        }
    }

    console.log('Photos extraction completed');

} catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();
