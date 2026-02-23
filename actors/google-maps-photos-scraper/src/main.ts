import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';

interface GoogleMapsPhotosInput {
    business_id: string;
    use_cache?: boolean;
    maximum_cache_age?: number;
}

interface Photo {
    photo_id?: string;
    photo_url?: string;
    photo_url_large?: string;
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

type GoogleMapsPhotosResponse = Photo[] | { data?: Photo[]; [key: string]: unknown };

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

    console.log(`Fetching photos for business: ${input.business_id}`);

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

    let photos: Photo[] = [];
    let handled404 = false;

    try {
        const response = await client.get<GoogleMapsPhotosResponse>('/maps/photos', params);

        // Handle both response types: direct array or wrapped in object
        photos = Array.isArray(response)
            ? response
            : (response.data ?? []);
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
            await store.setValue('OUTPUT', { photos: [], total: 0, error: 'Business not found' });
            handled404 = true;
        } else {
            // Re-throw non-404 errors
            throw apiError;
        }
    }

    if (!handled404) {
        if (photos && photos.length > 0) {
            await Actor.pushData(photos);
            console.log(`Found ${photos.length} photos`);
        } else {
            console.log('No photos found for the given business ID');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', { photos, total: photos.length });
    }

    console.log('Photos extraction completed');

} catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();
