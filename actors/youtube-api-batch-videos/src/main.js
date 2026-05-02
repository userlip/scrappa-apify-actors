import { Actor } from 'apify';
import axios from 'axios';
import { buildBatchVideosUrl } from './videos-url.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

function errorMessage(error) {
    if (error?.code === 'ECONNABORTED') {
        return `Scrappa API request timed out after ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s`;
    }

    return error instanceof Error ? error.message : String(error);
}

Actor.main(async () => {
    try {
        const input = (await Actor.getInput()) ?? {};
        const apiUrl = buildBatchVideosUrl(input);

        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl, { timeout: SCRAPPA_REQUEST_TIMEOUT_MS });
        const videos = response.data?.videos ?? [];

        await Actor.pushData(videos);
        console.log(`Successfully fetched ${Array.isArray(videos) ? videos.length : 1} batch video(s) for ids: ${input.ids}`);
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube batch videos: ${message}`);
        await Actor.fail(message);
    }
});
