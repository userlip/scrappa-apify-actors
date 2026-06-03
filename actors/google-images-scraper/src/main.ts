import { Actor } from 'apify';
import { buildGoogleImagesParamList, describeGoogleImagesRequest } from './request-params.js';
import type { GoogleImagesInput } from './request-params.js';
import { enrichResult, extractImageResults } from './response-utils.js';
import type { GoogleImagesResponse } from './response-utils.js';
import { ScrappaClient } from './shared/scrappa-client.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;
const BATCH_CONCURRENCY = 5;

interface GoogleImagesRequestSummary {
    request: Record<string, unknown>;
    image_results: number;
    products: number;
    with_original: number;
    with_dimensions: number;
}

interface GoogleImagesRequestResult extends GoogleImagesRequestSummary {
    response: GoogleImagesResponse;
}

async function runGoogleImagesRequest(
    client: ScrappaClient,
    params: Record<string, unknown>,
): Promise<GoogleImagesRequestResult> {
    console.log(`Fetching Google Images for ${describeGoogleImagesRequest(params)}`);
    const response = await client.get<GoogleImagesResponse>('/images', params);
    const imageResults = extractImageResults(response);
    const datasetItems = imageResults.map((result) => enrichResult(result, params));

    if (datasetItems.length > 0) {
        await Actor.pushData(datasetItems);
        console.log(`Found ${imageResults.length} image results`);
    } else {
        console.log('No Google Images results found for this request');
    }

    return {
        request: params,
        image_results: imageResults.length,
        products: imageResults.filter((result) => result.is_product).length,
        with_original: imageResults.filter((result) => typeof result.original === 'string' && result.original !== '').length,
        with_dimensions: imageResults.filter((result) => typeof result.original_width === 'number' && typeof result.original_height === 'number').length,
        response,
    };
}

async function runWithConcurrency<T, R>(
    items: T[],
    concurrency: number,
    handler: (item: T) => Promise<R>,
): Promise<R[]> {
    const results = new Array<R>(items.length);
    let nextIndex = 0;

    async function worker(): Promise<void> {
        while (nextIndex < items.length) {
            const index = nextIndex;
            nextIndex += 1;
            results[index] = await handler(items[index]);
        }
    }

    await Promise.all(
        Array.from({ length: Math.min(concurrency, items.length) }, () => worker()),
    );

    return results;
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

        const paramList = buildGoogleImagesParamList(input);
        console.log(`Running ${paramList.length} Google Images request${paramList.length === 1 ? '' : 's'}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const results = await runWithConcurrency(paramList, BATCH_CONCURRENCY, (params) => runGoogleImagesRequest(client, params));
        const requestSummaries = results.map(({ response: _response, ...summary }) => summary);
        let totalImageResults = 0;
        let totalProducts = 0;
        let totalWithOriginal = 0;
        let totalWithDimensions = 0;

        for (const result of results) {
            totalImageResults += result.image_results;
            totalProducts += result.products;
            totalWithOriginal += result.with_original;
            totalWithDimensions += result.with_dimensions;
        }

        const store = await Actor.openKeyValueStore();
        if (paramList.length === 1) {
            await store.setValue('OUTPUT', results[0].response);
        } else {
            await store.setValue('OUTPUT', {
                requests: requestSummaries,
                image_results: totalImageResults,
            });
        }

        const summary = {
            requests: paramList.length,
            image_results: totalImageResults,
            products: totalProducts,
            with_original: totalWithOriginal,
            with_dimensions: totalWithDimensions,
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
