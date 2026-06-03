import { Actor } from 'apify';
import { ScrappaClient } from './shared/index.js';
import { getBusinessIdRequests, type GoogleMapsPhotosInput } from './business-id.js';

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
    const requests = getBusinessIdRequests(input);
    if (requests.length === 0) {
        throw new Error('At least one Business ID is required. Provide business_ids or legacy business_id.');
    }

    const client = new ScrappaClient({ apiKey });
    const runResults: Array<{
        input_business_id: string;
        business_id?: string;
        total: number;
        nextPage: string | null;
        error?: string;
    }> = [];
    let succeeded = 0;
    let failed = 0;
    let totalPhotos = 0;
    let firstOutput: { photos: Photo[]; total: number; nextPage: string | null; error?: string } | undefined;

    console.log(`Fetching Google Maps photos for ${requests.length} business${requests.length === 1 ? '' : 'es'}`);

    for (const request of requests) {
        const inputBusinessId = request.input_business_id;
        const businessId = request.business_id;

        if (!businessId) {
            const error = request.validation_error ?? 'Invalid business input';
            console.log(`Invalid business input: ${error}`);
            await Actor.pushData([{
                success: false,
                input_business_id: inputBusinessId,
                business_id: inputBusinessId,
                error,
            }]);
            runResults.push({
                input_business_id: inputBusinessId,
                business_id: inputBusinessId,
                total: 0,
                nextPage: null,
                error,
            });
            firstOutput ??= { photos: [], total: 0, nextPage: null, error };
            failed += 1;
            continue;
        }

        if (request.source === 'url') {
            console.log(`Extracted Google Maps business identifier from URL: ${businessId}`);
        }

        console.log(`Fetching photos for business: ${businessId}`);

        const params: Record<string, unknown> = {
            business_id: businessId,
        };

        if (input?.use_cache !== false) {
            params.use_cache = 1;
        }
        if (input?.maximum_cache_age !== undefined) {
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
                    input_business_id: inputBusinessId,
                    business_id: businessId,
                    error,
                }]);
                runResults.push({
                    input_business_id: inputBusinessId,
                    business_id: businessId,
                    total: 0,
                    nextPage: null,
                    error,
                });
                firstOutput ??= { photos: [], total: 0, nextPage: null, error };
                failed += 1;
                handledApiInputError = true;
            } else {
                // Re-throw non-input errors
                throw apiError;
            }
        }

        if (!handledApiInputError) {
            const datasetPhotos = photos.map((photo) => ({
                ...photo,
                input_business_id: inputBusinessId,
                business_id: businessId,
            }));

            if (datasetPhotos.length > 0) {
                await Actor.pushData(datasetPhotos);
                console.log(`Found ${datasetPhotos.length} photos for ${businessId}`);
            } else {
                console.log(`No photos found for business: ${businessId}`);
            }

            runResults.push({
                input_business_id: inputBusinessId,
                business_id: businessId,
                total: datasetPhotos.length,
                nextPage,
            });
            firstOutput ??= { photos: datasetPhotos, total: datasetPhotos.length, nextPage };
            succeeded += 1;
            totalPhotos += datasetPhotos.length;
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
            total_photos: totalPhotos,
            results: runResults,
        });
    }

    console.log('Photos extraction completed');

} catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    await Actor.fail(message);
}

await Actor.exit();
