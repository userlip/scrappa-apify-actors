import { Actor } from 'apify';
import { buildBatchVideosUrl } from './videos-url.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

function errorMessage(error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    if (rawMessage.includes('aborted')) {
        return `Scrappa API request timed out after ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s`;
    }

    return rawMessage;
}

Actor.main(async () => {
    try {
        const input = (await Actor.getInput()) ?? {};
        const apiUrl = buildBatchVideosUrl(input);

        console.log(`Fetching from: ${apiUrl}`);
        const response = await fetch(apiUrl, {
            signal: AbortSignal.timeout(SCRAPPA_REQUEST_TIMEOUT_MS),
        });
        if (!response.ok) {
            throw new Error(`Scrappa API request failed with ${response.status} ${response.statusText}`);
        }

        const data = await response.json();
        const videos = data?.videos ?? [];

        await Actor.pushData(videos);
        console.log(`Successfully fetched ${Array.isArray(videos) ? videos.length : 1} batch video(s) for ids: ${input.ids}`);
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube batch videos: ${message}`);
        await Actor.fail(message);
    }
});
