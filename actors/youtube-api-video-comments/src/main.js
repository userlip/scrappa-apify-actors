import { Actor } from 'apify';
import { getScrappaApiKey } from './api-key.js';
import { fetchVideoComments, SCRAPPA_REQUEST_TIMEOUT_MS } from './comments-client.js';

function errorMessage(error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    if (rawMessage.includes('aborted')) {
        return `Scrappa API request timed out after ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s`;
    }

    return rawMessage;
}

Actor.main(async () => {
    try {
        const apiKey = getScrappaApiKey();
        const input = (await Actor.getInput()) ?? {};
        const { data } = await fetchVideoComments(input, {
            apiKey,
            onRequest: (apiUrl) => console.log(`Fetching from: ${apiUrl}`),
        });
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
