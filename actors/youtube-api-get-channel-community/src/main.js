import { Actor } from 'apify';
import axios from 'axios';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

function errorMessage(error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    if (rawMessage.includes('timeout') || rawMessage.includes('aborted')) {
        return `Scrappa API request timed out after ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s`;
    }

    return rawMessage;
}

async function getChannelCommunity(input) {
    const { id, continuation = '' } = input;

    if (!id) {
        throw new Error('Search query "id" not provided. Please provide a value for "id" in the input.');
    }

    let apiUrl = `https://ytapi.scrappa.co/channels/community?id=${encodeURIComponent(id)}`;

    if (continuation && typeof continuation === 'string' && continuation.trim() !== '') {
        apiUrl += `&continuation=${encodeURIComponent(continuation)}`;
    }

    console.log(`Fetching from: ${apiUrl}`);
    const response = await axios.get(apiUrl, {
        timeout: SCRAPPA_REQUEST_TIMEOUT_MS,
    });
    const posts = response.data?.posts ?? [];

    await Actor.pushData(posts);
    console.log(`Successfully fetched ${posts.length} community post(s) for channel id: ${id}`);

    if (response.data?.continuation) {
        console.log(`Continuation token available for next page: ${response.data.continuation}`);
    }
}

Actor.main(async () => {
    try {
        const input = (await Actor.getInput()) ?? {};
        await getChannelCommunity(input);
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube channel community posts: ${message}`);
        await Actor.fail(message);
    }
});
