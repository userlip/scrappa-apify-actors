import { Actor } from 'apify';
import { buildGoogleImagesParamList, describeGoogleImagesRequest } from './request-params.js';
import type { GoogleImagesInput } from './request-params.js';
import { enrichResult, extractImageResults } from './response-utils.js';
import type { GoogleImagesResponse } from './response-utils.js';
import { ScrappaClient } from './shared/scrappa-client.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

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
        const responses: GoogleImagesResponse[] = [];
        const requestSummaries: Array<{ request: Record<string, unknown>; image_results: number }> = [];
        let totalImageResults = 0;
        let totalProducts = 0;
        let totalWithOriginal = 0;
        let totalWithDimensions = 0;

        for (const params of paramList) {
            console.log(`Fetching Google Images for ${describeGoogleImagesRequest(params)}`);
            const response = await client.get<GoogleImagesResponse>('/images', params);
            responses.push(response);
            const imageResults = extractImageResults(response);
            totalImageResults += imageResults.length;
            totalProducts += imageResults.filter((result) => result.is_product).length;
            totalWithOriginal += imageResults.filter((result) => typeof result.original === 'string' && result.original !== '').length;
            totalWithDimensions += imageResults.filter((result) => typeof result.original_width === 'number' && typeof result.original_height === 'number').length;

            if (imageResults.length > 0) {
                await Actor.pushData(imageResults.map((result) => enrichResult(result, params)));
                console.log(`Found ${imageResults.length} image results`);
            } else {
                console.log('No Google Images results found for this request');
            }

            requestSummaries.push({ request: params, image_results: imageResults.length });
        }

        const store = await Actor.openKeyValueStore();
        if (paramList.length === 1) {
            await store.setValue('OUTPUT', responses[0]);
        } else {
            await store.setValue('OUTPUT', {
                requests: requestSummaries,
                responses,
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
