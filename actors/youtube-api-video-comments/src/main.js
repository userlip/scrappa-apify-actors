import { Actor } from 'apify';
import { buildVideoCommentsUrl } from './comments-url.js';

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
        const apiUrl = buildVideoCommentsUrl(input);

        console.log(`Fetching from: ${apiUrl}`);
        const response = await fetch(apiUrl, {
            signal: AbortSignal.timeout(SCRAPPA_REQUEST_TIMEOUT_MS),
        });
        if (!response.ok) {
            throw new Error(`Scrappa API request failed with ${response.status} ${response.statusText}`);
        }

        const data = await response.json();
        const comments = data?.comments ?? [];

        await Actor.pushData(comments);
        console.log(`Successfully fetched ${comments.length} comment(s) for video id: ${input.id}`);

        if (data?.continuation) {
            console.log(`Continuation token available for next page: ${data.continuation}`);
        }
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube video comments: ${message}`);
        await Actor.fail(message);
    }
});
