import { Actor } from 'apify';
import axios from 'axios';
import { buildVideoDetailsUrl } from './video-url.js';

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
        const apiUrl = buildVideoDetailsUrl(input);

        console.log(`Fetching from: ${apiUrl}`);
        const response = await axios.get(apiUrl, { timeout: SCRAPPA_REQUEST_TIMEOUT_MS });
        const data = response.data;

        await Actor.pushData(data);
        console.log(`Successfully fetched ${Array.isArray(data) ? data.length : 1} video detail result(s) for id: ${input.id}`);

        if (response.data?.continuation) {
            console.log(`Continuation token available for next page: ${response.data.continuation}`);
        }
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube video details: ${message}`);
        await Actor.fail(message);
    }
});
