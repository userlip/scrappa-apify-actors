import { Actor } from 'apify';
import { buildGoogleVideosParams, describeGoogleVideosRequest } from './request-params.js';
import type { GoogleVideosInput } from './request-params.js';
import { enrichResult, extractVideoResults } from './response-utils.js';
import type { GoogleVideosResponse } from './response-utils.js';
import { ScrappaClient } from './shared/scrappa-client.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

async function main(): Promise<void> {
    await Actor.init();

    try {
        const apiKey = process.env.SCRAPPA_API_KEY;
        if (!apiKey) {
            throw new Error('SCRAPPA_API_KEY environment variable is not set. Please configure it in Actor settings.');
        }

        const input = await Actor.getInput<GoogleVideosInput>();
        if (!input) {
            throw new Error('Input is required');
        }

        const params = buildGoogleVideosParams(input);
        console.log(`Fetching Google Videos for ${describeGoogleVideosRequest(params)}`);

        const client = new ScrappaClient({ apiKey, timeoutMs: SCRAPPA_REQUEST_TIMEOUT_MS });
        const response = await client.get<GoogleVideosResponse>('/google/videos', params);
        const videoResults = extractVideoResults(response);

        if (videoResults.length > 0) {
            await Actor.pushData(videoResults.map((result) => enrichResult(result, params)));
            console.log(`Found ${videoResults.length} video results`);
        } else {
            console.log('No Google Videos results found for this request');
        }

        const store = await Actor.openKeyValueStore();
        await store.setValue('OUTPUT', response);

        const summary = {
            video_results: videoResults.length,
            found_in_videos: response.found_in_videos?.length ?? 0,
            short_videos: response.short_videos?.length ?? 0,
            related_searches: response.related_searches?.length ?? 0,
            has_pagination: !!response.pagination,
            has_scrappa_pagination: !!response.scrappa_pagination,
        };

        console.log('Google Videos scraping completed successfully');
        console.log('Results summary:', JSON.stringify(summary));
    } catch (error) {
        const rawMessage = error instanceof Error ? error.message : String(error);
        const message = rawMessage.includes('timed out')
            ? `${rawMessage}. The Google Videos request exceeded the ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s Scrappa API timeout. Try a more specific query or run the request again.`
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
