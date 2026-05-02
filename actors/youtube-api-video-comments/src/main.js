import { Actor } from 'apify';
import axios from 'axios';
import { buildVideoCommentsUrl } from './comments-url.js';

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
        const apiUrl = buildVideoCommentsUrl(input);

        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl, { timeout: SCRAPPA_REQUEST_TIMEOUT_MS });
        const comments = response.data?.comments ?? [];

        await Actor.pushData(comments);
        console.log(`Successfully fetched ${Array.isArray(comments) ? comments.length : 1} comment(s) for video id: ${input.id}`);

        if (response.data?.continuation) {
            console.log(`Continuation token available for next page: ${response.data.continuation}`);
        }
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube video comments: ${message}`);
        await Actor.fail(message);
    }
});
