import { Actor } from 'apify';
import { buildChannelVideosUrl } from './channel-videos-url.js';

const SCRAPPA_REQUEST_TIMEOUT_MS = 60000;

function errorMessage(error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    if (rawMessage.includes('aborted')) {
        return `Scrappa API request timed out after ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s`;
    }

    return rawMessage;
}

async function getChannelVideos(input) {
    const apiUrl = buildChannelVideosUrl(input);

    console.log(`Fetching from: ${apiUrl}`);
    const response = await fetch(apiUrl, {
        signal: AbortSignal.timeout(SCRAPPA_REQUEST_TIMEOUT_MS),
    });
    if (!response.ok) {
        throw new Error(`Scrappa API request failed with ${response.status} ${response.statusText}`);
    }

    const responseData = await response.json();
    const videos = responseData?.videos ?? [];

    await Actor.pushData(videos);
    console.log(`Successfully fetched ${videos.length} videos for channel id: ${input.id}`);

    if (responseData?.continuation) {
        console.log(`Continuation token available for next page: ${responseData.continuation}`);
    }
}

Actor.main(async () => {
    try {
        const input = (await Actor.getInput()) ?? {};
        await getChannelVideos(input);
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube channel videos: ${message}`);
        await Actor.fail(message);
    }
});
