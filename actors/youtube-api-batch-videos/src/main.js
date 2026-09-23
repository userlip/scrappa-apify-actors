import { Actor } from 'apify';
import { buildBatchVideosUrl } from './videos-url.js';
import { fetchBatchVideos, REQUEST_TIMEOUT_MS } from './fetch-videos.js';

function errorMessage(error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    if (rawMessage.includes('aborted')) {
        return `Scrappa API request timed out after ${REQUEST_TIMEOUT_MS / 1000}s`;
    }

    return rawMessage;
}

Actor.main(async () => {
    try {
        const input = (await Actor.getInput()) ?? {};
        const apiUrl = buildBatchVideosUrl(input);

        console.log(`Fetching from: ${apiUrl}`);
        const videos = await fetchBatchVideos(apiUrl);

        await Actor.pushData(videos);
        console.log(`Successfully fetched ${videos.length} batch video(s) for ids: ${input.ids}`);
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube batch videos: ${message}`);
        await Actor.fail(message);
    }
});
