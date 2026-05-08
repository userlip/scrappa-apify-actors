import { Actor } from 'apify';
import { buildGoogleImagesParams, describeGoogleImagesRequest } from './request-params.js';
import type { GoogleImagesInput } from './request-params.js';
import { ScrappaClient } from './shared/scrappa-client.js';

interface GoogleImageResult {
    position?: number;
    thumbnail?: string;
    source?: string;
    title?: string;
    link?: string;
    original?: string;
    original_width?: number;
    original_height?: number;
    is_product?: boolean;
    [key: string]: unknown;
}

type GoogleImagesResponse = GoogleImageResult[] | { data?: GoogleImageResult[]; [key: string]: unknown };

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

function extractImageResults(response: GoogleImagesResponse): GoogleImageResult[] {
    if (Array.isArray(response)) {
        return response;
    }

    if (Array.isArray(response.data)) {
        return response.data;
    }

    console.warn('Scrappa Google Images response did not include an image result array');
    return [];
}

function enrichResult(result: GoogleImageResult, params: Record<string, unknown>): Record<string, unknown> {
    return {
        ...result,
        position: result.position ?? null,
        image_url: result.original ?? null,
        thumbnail_url: result.thumbnail ?? null,
        source_url: result.link ?? null,
        width: result.original_width ?? null,
        height: result.original_height ?? null,
        request_q: params.q ?? null,
        request_page: params.page ?? null,
        request_hl: params.hl ?? null,
        request_gl: params.gl ?? null,
        request_imgsz: params.imgsz ?? null,
        request_imgtype: params.imgtype ?? null,
        request_imgcolor: params.imgcolor ?? null,
        request_imgar: params.imgar ?? null,
        request_tbs: params.tbs ?? null,
        request_safe: params.safe ?? null,
    };
}

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleImagesInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const params = buildGoogleImagesParams(input);
        console.log(`Fetching Google Images for ${describeGoogleImagesRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await client.get<GoogleImagesResponse>('/images', params);
        const imageResults = extractImageResults(response);

        if (imageResults.length > 0) {
            await Actor.pushData(imageResults.map((result) => enrichResult(result, params)));
            console.log(`Found ${imageResults.length} image results`);
        } else {
            console.log('No Google Images results found for this request');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const summary = {
            image_results: imageResults.length,
            products: imageResults.filter((result) => result.is_product).length,
            with_original: imageResults.filter((result) => typeof result.original === 'string' && result.original !== '').length,
            with_dimensions: imageResults.filter((result) => typeof result.original_width === 'number' && typeof result.original_height === 'number').length,
        };

        console.log('Google Images scraping completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = rawMessage.includes('timed out')
            ? `${rawMessage}. The Google Images request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a more specific query or run the request again.`
            : rawMessage;
        console.error('Actor failed: ' + message);
        await Actor.fail(message);
        return;
    }

    await Actor.exit();
}

main().catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Actor failed: ' + message);
    process.exitCode = 1;
});
