import { Actor } from 'apify';
import { buildGoogleImagesParams, describeGoogleImagesRequest } from './request-params.js';
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
