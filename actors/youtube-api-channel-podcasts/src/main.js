import { Actor } from 'apify';
import { assertContinuationMatchesBatch, buildChannelPodcastsUrl, getChannelIds } from './channel-podcasts-url.js';
import { podcastVideos } from './podcast-filter.js';
import { fetchScrappaJson, getScrappaApiKey, SCRAPPA_REQUEST_TIMEOUT_MS } from './youtube-request.js';

function errorMessage(error) {
    const rawMessage = error instanceof Error ? error.message : String(error);
    if (rawMessage.includes('timeout') || rawMessage.includes('aborted')) {
        return `Scrappa API request timed out after ${SCRAPPA_REQUEST_TIMEOUT_MS / 1000}s`;
    }

    return rawMessage;
}

async function getChannelPodcasts(input, apiKey) {
    const apiUrl = buildChannelPodcastsUrl(input);

    console.log(`Fetching from: ${apiUrl}`);
    const responseData = await fetchScrappaJson(apiUrl, { apiKey });
    const videos = podcastVideos(responseData);

    await Actor.pushData(videos);
    console.log(`Successfully fetched ${videos.length} podcast video(s) for channel id: ${input.id}`);

    const continuation = responseData?.continuation ?? responseData?.pagination?.continuationToken;
    if (continuation) {
        console.log(`Continuation token available for next page: ${continuation}`);
    }
}

Actor.main(async () => {
    try {
        const apiKey = getScrappaApiKey();
        const input = (await Actor.getInput()) ?? {};
        const ids = getChannelIds(input);

        if (ids.length === 0) {
            throw new Error('At least one YouTube channel ID must be provided in "ids" or "id".');
        }
        assertContinuationMatchesBatch(input, ids);

        for (const id of ids) {
            await getChannelPodcasts({ ...input, id }, apiKey);
        }
    } catch (error) {
        const message = errorMessage(error);
        console.error(`Failed to fetch YouTube channel podcasts: ${message}`);
        await Actor.fail(message);
    }
});
